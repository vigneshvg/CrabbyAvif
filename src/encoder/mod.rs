// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(unused)]

pub mod item;
pub mod mp4box;

use crate::encoder::item::*;
use crate::encoder::mp4box::*;

use crate::codecs::EncoderConfig;
use crate::image::*;
use crate::internal_utils::stream::OStream;
use crate::internal_utils::*;
use crate::parser::mp4box::*;
use crate::parser::obu::Av1SequenceHeader;
use crate::*;

#[cfg(feature = "aom")]
use crate::codecs::aom::Aom;

use std::fmt;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

#[derive(Clone, Copy, Debug, Default)]
pub struct Settings {
    pub max_threads: i32,
    pub speed: i32,
    pub keyframe_interval: i32,
    pub timescale: u64,
    pub repetition_count: i32,
    pub extra_layer_count: u32,
    // changeable.
    pub quality: i32,
    pub tile_rows_log2: i32,
    pub tile_columns_log2: i32,
    pub auto_tiling: bool,
    // end of changeable.
}

impl Settings {
    pub(crate) fn quantizer(&self) -> i32 {
        // TODO: account for category here.
        ((100 - self.quality) * 63 + 50) / 100
    }
}

#[derive(Debug, Default)]
pub(crate) struct Sample {
    pub data: Vec<u8>,
    pub sync: bool,
}

impl Sample {
    pub fn create_from(data: &[u8], sync: bool) -> AvifResult<Self> {
        let mut copied_data: Vec<u8> = create_vec_exact(data.len())?;
        copied_data.extend_from_slice(data);
        Ok(Sample {
            data: copied_data,
            sync,
        })
    }
}

pub(crate) type Codec = Box<dyn crate::codecs::Encoder>;

#[derive(Default)]
pub struct Encoder {
    settings: Settings,
    items: Vec<Item>,
    image_metadata: Image,
    quantizer: i32,
    tile_rows_log2: i32,
    tile_columns_log2: i32,
    primary_item_id: u16,
    // alternative_item_ids.
    single_image: bool,
    alpha_present: bool,
    image_item_type: String,
    config_property_name: String,
    duration_in_timescales: Vec<u64>,
}

const UNITY_MATRIX: [u8; 9 * 4] = [
    0x00, 0x01, 0x00, 0x00, //
    0x00, 0x00, 0x00, 0x00, //
    0x00, 0x00, 0x00, 0x00, //
    0x00, 0x00, 0x00, 0x00, //
    0x00, 0x01, 0x00, 0x00, //
    0x00, 0x00, 0x00, 0x00, //
    0x00, 0x00, 0x00, 0x00, //
    0x00, 0x00, 0x00, 0x00, //
    0x40, 0x00, 0x00, 0x00, //
];

impl Encoder {
    pub fn create_with_settings(settings: &Settings) -> AvifResult<Self> {
        if settings.extra_layer_count >= MAX_AV1_LAYER_COUNT as u32 {
            return Err(AvifError::InvalidArgument);
        }
        Ok(Self {
            settings: *settings,
            ..Default::default()
        })
    }

    pub fn update_settings(&mut self, settings: &Settings) -> AvifResult<()> {
        // TODO: validate that only fields that are allowed update are updated.
        self.settings = *settings;
        Ok(())
    }

    fn add_items(&mut self, grid: &Grid, category: Category) -> AvifResult<u16> {
        let cell_count = usize_from_u32(grid.rows * grid.columns)?;
        println!("### cell_count: {cell_count}");
        let mut top_level_item_id = 0;
        if cell_count > 1 {
            let mut stream = OStream::default();
            write_grid(&mut stream, grid)?;
            let mut item = Item {
                id: u16_from_usize(self.items.len() + 1)?,
                item_type: "grid".into(),
                infe_name: category.infe_name(),
                category,
                grid: Some(*grid),
                metadata_payload: stream.data,
                ..Default::default()
            };
            top_level_item_id = item.id;
            self.items.push(item);
        }
        // TODO: create vec exact.
        for cell_index in 0..cell_count {
            let mut item = Item {
                id: u16_from_usize(self.items.len() + 1)?,
                item_type: "av01".into(),
                infe_name: category.infe_name(),
                cell_index,
                category,
                dimg_from_id: if cell_count > 1 { Some(top_level_item_id) } else { None },
                hidden_image: cell_count > 1,
                extra_layer_count: self.settings.extra_layer_count,
                #[cfg(feature = "aom")]
                codec: Some(Box::<Aom>::default()),
                ..Default::default()
            };
            if cell_count == 1 {
                top_level_item_id = item.id;
            }
            self.items.push(item);
        }
        Ok(top_level_item_id)
    }

    fn add_image_impl(
        &mut self,
        grid_columns: u32,
        grid_rows: u32,
        cell_images: &[&Image],
        mut duration: u32,
        is_single_image: bool,
    ) -> AvifResult<()> {
        let cell_count: usize = usize_from_u32(grid_rows * grid_columns)?;
        if cell_count == 0 || cell_images.len() != cell_count {
            return Err(AvifError::InvalidArgument);
        }
        // TODO: detect changes.
        if duration == 0 {
            duration = 1;
        }

        if self.items.is_empty() {
            // TODO: validate clap.
            self.image_metadata = Image {
                width: cell_images[0].width,
                height: cell_images[0].height,
                depth: cell_images[0].depth,
                yuv_format: cell_images[0].yuv_format,
                yuv_range: cell_images[0].yuv_range,
                chroma_sample_position: cell_images[0].chroma_sample_position,
                alpha_present: cell_images[0].alpha_present,
                alpha_premultiplied: cell_images[0].alpha_premultiplied,
                color_primaries: cell_images[0].color_primaries,
                transfer_characteristics: cell_images[0].transfer_characteristics,
                matrix_coefficients: cell_images[0].matrix_coefficients,
                clli: cell_images[0].clli,
                pasp: cell_images[0].pasp,
                clap: cell_images[0].clap,
                irot_angle: cell_images[0].irot_angle,
                imir_axis: cell_images[0].imir_axis,
                exif: cell_images[0].exif.clone(),
                icc: cell_images[0].icc.clone(),
                xmp: cell_images[0].xmp.clone(),
                ..Default::default()
            };

            let first_image = cell_images[0];
            let last_image = cell_images.last().unwrap();
            let grid = Grid {
                rows: grid_rows,
                columns: grid_columns,
                width: (grid_columns - 1) * first_image.width + last_image.width,
                height: (grid_rows - 1) * first_image.height + last_image.height,
            };
            let color_item_id = self.add_items(&grid, Category::Color)?;
            self.primary_item_id = color_item_id;
            self.alpha_present = first_image.has_plane(Plane::A);

            if self.alpha_present && self.single_image {
                // TODO: Handle opaque alpha.
            }

            if self.alpha_present {
                let alpha_item_id = self.add_items(&grid, Category::Alpha)?;
                let alpha_item = &mut self.items[alpha_item_id as usize - 1];
                alpha_item.iref_type = Some(String::from("auxl"));
                alpha_item.iref_to_id = Some(color_item_id);
                if self.image_metadata.alpha_premultiplied {
                    let color_item = &mut self.items[color_item_id as usize - 1];
                    color_item.iref_type = Some(String::from("prem"));
                    color_item.iref_to_id = Some(alpha_item_id);
                }
            }
            // TODO: gainmap.
            // TODO: exif, xmp.
        } else {
            // Another frame in an image sequence, or layer in a layered image.
            let first_image = cell_images[0];
            if !first_image.has_same_cicp(&self.image_metadata)
                || first_image.alpha_premultiplied != self.image_metadata.alpha_premultiplied
                || first_image.alpha_present != self.image_metadata.alpha_present
            {
                return Err(AvifError::InvalidArgument);
            }
        }

        println!("### items: {:#?}", self.items);

        // Encode the AV1 OBUs.
        for item in &mut self.items {
            if !item.codec.is_some() {
                println!("### no codec");
                continue;
            }
            let image = cell_images[item.cell_index];
            let first_image = cell_images[0];
            if image.width != first_image.width || image.height != first_image.height {
                // TODO: pad the image so that the dimensions of all cells are equal.
            }
            let encoder_config = EncoderConfig {
                tile_rows_log2: self.settings.tile_rows_log2,
                tile_columns_log2: self.settings.tile_columns_log2,
                quantizer: self.settings.quantizer(),
                disable_lagged_output: true,
                is_single_image,
            };
            item.codec.unwrap_mut().encode_image(
                image,
                item.category,
                &encoder_config,
                &mut item.samples,
            )?;
        }
        self.duration_in_timescales.push(duration as u64);
        Ok(())
    }

    pub fn add_image(&mut self, image: &Image) -> AvifResult<()> {
        self.add_image_impl(1, 1, &[image], 0, self.settings.extra_layer_count == 0)
    }

    pub fn add_image_for_sequence(&mut self, image: &Image, duration: u32) -> AvifResult<()> {
        // TODO: this and add_image cannot be used on the same instance.
        self.add_image_impl(1, 1, &[image], duration, false)
    }

    pub fn add_image_grid(
        &mut self,
        grid_columns: u32,
        grid_rows: u32,
        images: &[&Image],
    ) -> AvifResult<()> {
        if grid_columns == 0 || grid_columns > 256 || grid_rows == 0 || grid_rows > 256 {
            return Err(AvifError::InvalidImageGrid("".into()));
        }
        // TODO: if layer count is zero, set single image flag here.
        self.add_image_impl(grid_columns, grid_rows, images, 0, true)
    }

    #[allow(unused)]
    pub fn finish(&mut self) -> AvifResult<Vec<u8>> {
        if self.items.is_empty() {
            return Err(AvifError::NoContent);
        }
        self.settings.timescale = 10000;
        for item in &mut self.items {
            if item.codec.is_none() {
                continue;
            }
            item.codec.unwrap_mut().finish()?;
            // TODO: check if sample count == duration count.

            if !item.samples.is_empty() {
                // Harvest codec configuration from sequence header.
                let sequence_header = Av1SequenceHeader::parse_from_obus(&item.samples[0].data)?;
                //println!("### seq_header: {:#?}", sequence_header);
                item.codec_configuration = CodecConfiguration::Av1(sequence_header.config);
            }
        }
        let image_metadata = &self.image_metadata;
        let mut stream = OStream::default();
        let is_sequence =
            self.settings.extra_layer_count == 0 && self.duration_in_timescales.len() > 1;
        let mut ftyp = FileTypeBox {
            major_brand: String::from(if is_sequence { "avis" } else { "avif" }),
            // TODO: check if avio brand is necessary.
            compatible_brands: vec![
                String::from("avif"),
                String::from("mif1"),
                String::from("miaf"),
            ],
        };
        if is_sequence {
            ftyp.compatible_brands.extend_from_slice(&[
                String::from("avis"),
                String::from("msf1"),
                String::from("iso8"),
            ]);
        }
        match image_metadata.depth {
            8 | 10 => match image_metadata.yuv_format {
                PixelFormat::Yuv420 => ftyp.compatible_brands.push(String::from("MA1B")),
                PixelFormat::Yuv444 => ftyp.compatible_brands.push(String::from("MA1A")),
                _ => {}
            },
            _ => {}
        }
        // TODO: Write tmap brand if necessary.

        // ftyp box.
        write_ftyp(&mut stream, &ftyp)?;
        // meta box.
        stream.start_full_box("meta", (0, 0))?;
        write_hdlr(&mut stream, &String::from("pict"))?;
        write_pitm(&mut stream, self.primary_item_id)?;
        self.write_iloc(&mut stream)?;
        self.write_iinf(&mut stream)?;
        self.write_iref(&mut stream)?;
        self.write_iprp(&mut stream)?;
        stream.finish_box()?;
        // moov box.
        if is_sequence {
            let frames_duration_in_timescales = self
                .duration_in_timescales
                .iter()
                .try_fold(0u64, |acc, &x| acc.checked_add(x))
                .ok_or(AvifError::UnknownError("".into()))?;
            let timestamp: u64 = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            // TODO: duration_in_timescales should account for loop count.
            stream.start_box("moov")?;
            self.write_mvhd(&mut stream, frames_duration_in_timescales, timestamp)?;
            self.write_tracks(&mut stream, frames_duration_in_timescales, timestamp)?;
            stream.finish_box()?;
        }
        // mdat box.
        self.write_mdat(&mut stream)?;
        Ok(stream.data)
    }
}
