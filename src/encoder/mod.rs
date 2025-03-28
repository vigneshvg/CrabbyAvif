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
use crate::gainmap::GainMap;
use crate::image::*;
use crate::internal_utils::stream::OStream;
use crate::internal_utils::*;
use crate::parser::mp4box::*;
use crate::parser::obu::Av1SequenceHeader;
use crate::utils::IFraction;
use crate::*;

#[cfg(feature = "aom")]
use crate::codecs::aom::Aom;

use std::fmt;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScalingMode {
    pub horizontal: IFraction,
    pub vertical: IFraction,
}

impl Default for ScalingMode {
    fn default() -> Self {
        Self {
            horizontal: IFraction(1, 1),
            vertical: IFraction(1, 1),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MutableSettings {
    pub quality: i32,
    pub tile_rows_log2: i32,
    pub tile_columns_log2: i32,
    pub auto_tiling: bool,
    pub scaling_mode: ScalingMode,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Settings {
    pub threads: u32,
    pub speed: Option<u32>,
    pub keyframe_interval: i32,
    pub timescale: u64,
    pub repetition_count: i32,
    pub extra_layer_count: u32,
    pub mutable: MutableSettings,
}

impl Settings {
    pub(crate) fn quantizer(&self) -> i32 {
        // TODO: account for category here.
        ((100 - self.mutable.quality) * 63 + 50) / 100
    }
}

#[derive(Debug, Default)]
pub(crate) struct Sample {
    pub data: Vec<u8>,
    pub sync: bool,
}

impl Sample {
    pub(crate) fn create_from(data: &[u8], sync: bool) -> AvifResult<Self> {
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
    gainmap_image_metadata: Image,
    alt_image_metadata: Image,
    quantizer: i32,
    tile_rows_log2: i32,
    tile_columns_log2: i32,
    primary_item_id: u16,
    alternative_item_ids: Vec<u16>,
    single_image: bool,
    alpha_present: bool,
    image_item_type: String,
    config_property_name: String,
    duration_in_timescales: Vec<u64>,
}

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

    pub fn update_settings(&mut self, mutable: &MutableSettings) -> AvifResult<()> {
        self.settings.mutable = *mutable;
        Ok(())
    }

    pub(crate) fn is_sequence(&self) -> bool {
        self.settings.extra_layer_count == 0 && self.duration_in_timescales.len() > 1
    }

    fn add_tmap_item(&mut self, gainmap: &GainMap) -> AvifResult<u16> {
        let item = Item {
            id: u16_from_usize(self.items.len() + 1)?,
            item_type: "tmap".into(),
            infe_name: Category::Gainmap.infe_name(),
            category: Category::Color,
            metadata_payload: write_tmap(&gainmap.metadata)?,
            ..Default::default()
        };
        let item_id = item.id;
        self.items.push(item);
        Ok(item_id)
    }

    fn add_items(&mut self, grid: &Grid, category: Category) -> AvifResult<u16> {
        let cell_count = usize_from_u32(grid.rows * grid.columns)?;
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
                hidden_image: category == Category::Gainmap,
                ..Default::default()
            };
            top_level_item_id = item.id;
            self.items.push(item);
        }
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

    fn add_exif_item(&mut self) -> AvifResult<()> {
        if self.image_metadata.exif.is_empty() {
            return Ok(());
        }
        // TODO: find the TIFF header and include the offset in the payload.
        self.items.push(Item {
            id: u16_from_usize(self.items.len() + 1)?,
            item_type: "Exif".into(),
            infe_name: "Exif".into(),
            iref_to_id: Some(self.primary_item_id),
            iref_type: Some("cdsc".into()),
            metadata_payload: self.image_metadata.exif.clone(),
            ..Default::default()
        });
        Ok(())
    }

    fn add_xmp_item(&mut self) -> AvifResult<()> {
        if self.image_metadata.xmp.is_empty() {
            return Ok(());
        }
        self.items.push(Item {
            id: u16_from_usize(self.items.len() + 1)?,
            item_type: "mime".into(),
            infe_name: "XMP".into(),
            infe_content_type: "application/rdf+xml".into(),
            iref_to_id: Some(self.primary_item_id),
            iref_type: Some("cdsc".into()),
            metadata_payload: self.image_metadata.xmp.clone(),
            ..Default::default()
        });
        Ok(())
    }

    fn copy_alt_image_metadata(&mut self, gainmap: &GainMap) {
        self.alt_image_metadata.width = self.image_metadata.width;
        self.alt_image_metadata.height = self.image_metadata.height;
        self.alt_image_metadata.icc = gainmap.alt_icc.clone();
        self.alt_image_metadata.color_primaries = gainmap.alt_color_primaries;
        self.alt_image_metadata.transfer_characteristics = gainmap.alt_transfer_characteristics;
        self.alt_image_metadata.matrix_coefficients = gainmap.alt_matrix_coefficients;
        self.alt_image_metadata.yuv_range = gainmap.alt_yuv_range;
        self.alt_image_metadata.depth = if gainmap.alt_plane_depth > 0 {
            gainmap.alt_plane_depth
        } else {
            std::cmp::max(self.image_metadata.depth, gainmap.image.depth)
        };
        self.alt_image_metadata.yuv_format = if gainmap.alt_plane_count == 1 {
            PixelFormat::Yuv400
        } else {
            PixelFormat::Yuv444
        };
        self.alt_image_metadata.clli = Some(gainmap.alt_clli);
    }

    fn validate_image_grid(grid: &Grid, images: &[&Image]) -> AvifResult<()> {
        let first_image = images[0];
        let last_image = images.last().unwrap();
        for (index, image) in images.iter().enumerate() {
            if image.depth != 8 && image.depth != 10 && image.depth != 12 {
                return Err(AvifError::InvalidArgument);
            }
            let expected_width = if grid.is_last_column(index as u32) {
                first_image.width
            } else {
                last_image.width
            };
            let expected_height = if grid.is_last_row(index as u32) {
                first_image.height
            } else {
                last_image.height
            };
            if image.width != expected_width
                || image.height != expected_height
                || !image.has_same_cicp(first_image)
                || image.has_alpha() != first_image.has_alpha()
                || image.alpha_premultiplied != first_image.alpha_premultiplied
            {
                return Err(AvifError::InvalidImageGrid(
                    "all cells do not have the same properties".into(),
                ));
            }
            if image.matrix_coefficients == MatrixCoefficients::Identity
                && image.yuv_format != PixelFormat::Yuv444
            {
                return Err(AvifError::InvalidArgument);
            }
            if !image.has_plane(Plane::Y) {
                return Err(AvifError::NoContent);
            }
        }
        if last_image.width > first_image.width || last_image.height > first_image.height {
            return Err(AvifError::InvalidImageGrid(
                "last cell was larger than the first cell".into(),
            ));
        }
        if images.len() > 1 {
            validate_grid_image_dimensions(first_image, grid)?;
        }
        Ok(())
    }

    fn validate_gainmap_grid(grid: &Grid, gainmaps: &[&GainMap]) -> AvifResult<()> {
        for gainmap in &gainmaps[1..] {
            if gainmaps[0] != *gainmap {
                return Err(AvifError::InvalidImageGrid(
                    "all cells should have the same gain map metadata".into(),
                ));
            }
        }
        if gainmaps[0].image.color_primaries != ColorPrimaries::Unspecified
            || gainmaps[0].image.transfer_characteristics != TransferCharacteristics::Unspecified
        {
            return Err(AvifError::InvalidArgument);
        }
        let gainmap_images: Vec<_> = gainmaps.iter().map(|x| &x.image).collect();
        Self::validate_image_grid(grid, &gainmap_images)?;
        Ok(())
    }

    fn add_image_impl(
        &mut self,
        grid_columns: u32,
        grid_rows: u32,
        cell_images: &[&Image],
        mut duration: u32,
        is_single_image: bool,
        gainmaps: Option<&[&GainMap]>,
    ) -> AvifResult<()> {
        let cell_count: usize = usize_from_u32(grid_rows * grid_columns)?;
        if cell_count == 0 || cell_images.len() != cell_count {
            return Err(AvifError::InvalidArgument);
        }
        if duration == 0 {
            duration = 1;
        }
        if self.items.is_empty() {
            // TODO: validate clap.
            let first_image = cell_images[0];
            let last_image = cell_images.last().unwrap();
            let grid = Grid {
                rows: grid_rows,
                columns: grid_columns,
                width: (grid_columns - 1) * first_image.width + last_image.width,
                height: (grid_rows - 1) * first_image.height + last_image.height,
            };
            Self::validate_image_grid(&grid, cell_images)?;
            self.image_metadata = first_image.shallow_clone();
            if gainmaps.is_some() {
                self.gainmap_image_metadata = gainmaps.unwrap()[0].image.shallow_clone();
                self.copy_alt_image_metadata(gainmaps.unwrap()[0]);
            }
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
            if let Some(gainmaps) = gainmaps {
                if gainmaps.len() != cell_images.len() {
                    return Err(AvifError::InvalidImageGrid(
                        "invalid number of gainmap images".into(),
                    ));
                }
                let first_gainmap_image = &gainmaps[0].image;
                let last_gainmap_image = &gainmaps.last().unwrap().image;
                let gainmap_grid = Grid {
                    rows: grid_rows,
                    columns: grid_columns,
                    width: (grid_columns - 1) * first_gainmap_image.width
                        + last_gainmap_image.width,
                    height: (grid_rows - 1) * first_gainmap_image.height
                        + last_gainmap_image.height,
                };
                Self::validate_gainmap_grid(&gainmap_grid, gainmaps)?;
                let tonemap_item_id = self.add_tmap_item(gainmaps[0])?;
                if !self.alternative_item_ids.is_empty() {
                    return Err(AvifError::UnknownError("".into()));
                }
                self.alternative_item_ids.push(tonemap_item_id);
                self.alternative_item_ids.push(color_item_id);
                let gainmap_item_id = self.add_items(&gainmap_grid, Category::Gainmap)?;
                for item_id in [color_item_id, gainmap_item_id] {
                    self.items[item_id as usize - 1].dimg_from_id = Some(tonemap_item_id);
                }
            }
            self.add_exif_item()?;
            self.add_xmp_item()?;
        } else {
            if gainmaps.is_some() {
                return Err(AvifError::NotImplemented);
            }
            // Another frame in an image sequence, or layer in a layered image.
            let first_image = cell_images[0];
            if !first_image.has_same_cicp(&self.image_metadata)
                || first_image.alpha_premultiplied != self.image_metadata.alpha_premultiplied
                || first_image.alpha_present != self.image_metadata.alpha_present
            {
                return Err(AvifError::InvalidArgument);
            }
        }

        // Encode the AV1 OBUs.
        for item in &mut self.items {
            if item.codec.is_none() {
                continue;
            }
            let image = match item.category {
                Category::Gainmap => &gainmaps.unwrap()[item.cell_index].image,
                _ => cell_images[item.cell_index],
            };
            let first_image = match item.category {
                Category::Gainmap => &gainmaps.unwrap()[0].image,
                _ => cell_images[0],
            };
            if image.width != first_image.width || image.height != first_image.height {
                // TODO: pad the image so that the dimensions of all cells are equal.
            }
            let encoder_config = EncoderConfig {
                tile_rows_log2: self.settings.mutable.tile_rows_log2,
                tile_columns_log2: self.settings.mutable.tile_columns_log2,
                quantizer: self.settings.quantizer(),
                disable_lagged_output: self.alpha_present,
                is_single_image,
                speed: self.settings.speed,
                extra_layer_count: self.settings.extra_layer_count,
                threads: self.settings.threads,
                scaling_mode: self.settings.mutable.scaling_mode,
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
        self.add_image_impl(
            1,
            1,
            &[image],
            0,
            self.settings.extra_layer_count == 0,
            None,
        )
    }

    pub fn add_image_for_sequence(&mut self, image: &Image, duration: u32) -> AvifResult<()> {
        // TODO: this and add_image cannot be used on the same instance.
        self.add_image_impl(1, 1, &[image], duration, false, None)
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
        self.add_image_impl(
            grid_columns,
            grid_rows,
            images,
            0,
            self.settings.extra_layer_count == 0,
            None,
        )
    }

    pub fn add_image_gainmap(&mut self, image: &Image, gainmap: &GainMap) -> AvifResult<()> {
        if self.settings.extra_layer_count != 0 {
            return Err(AvifError::NotImplemented);
        }
        self.add_image_impl(1, 1, &[image], 0, true, Some(&[gainmap]))
    }

    pub fn add_image_gainmap_grid(
        &mut self,
        grid_columns: u32,
        grid_rows: u32,
        images: &[&Image],
        gainmaps: &[&GainMap],
    ) -> AvifResult<()> {
        if grid_columns == 0 || grid_columns > 256 || grid_rows == 0 || grid_rows > 256 {
            return Err(AvifError::InvalidImageGrid("".into()));
        }
        if self.settings.extra_layer_count != 0 {
            return Err(AvifError::NotImplemented);
        }
        self.add_image_impl(grid_columns, grid_rows, images, 0, true, Some(gainmaps))
    }

    pub fn finish(&mut self) -> AvifResult<Vec<u8>> {
        if self.items.is_empty() {
            return Err(AvifError::NoContent);
        }
        self.settings.timescale = 10000;
        for item in &mut self.items {
            if item.codec.is_none() {
                continue;
            }
            item.codec.unwrap_mut().finish(&mut item.samples)?;
            if item.extra_layer_count > 0
                && item.samples.len() != 1 + item.extra_layer_count as usize
            {
                return Err(AvifError::InvalidArgument);
            }
            // TODO: check if sample count == duration count.

            if !item.samples.is_empty() {
                // Harvest codec configuration from sequence header.
                let sequence_header = Av1SequenceHeader::parse_from_obus(&item.samples[0].data)?;
                item.codec_configuration = CodecConfiguration::Av1(sequence_header.config);
            }
        }
        let mut stream = OStream::default();
        self.write_ftyp(&mut stream)?;
        self.write_meta(&mut stream)?;
        self.write_moov(&mut stream)?;
        self.write_mdat(&mut stream)?;
        Ok(stream.data)
    }
}
