#![allow(unused)]

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
struct Settings {
    codec_choice: u32,
    max_threads: i32,
    speed: i32,
    keyframe_interval: i32,
    timescale: u64,
    repetition_count: i32,
    // changeable.
    quality: i32,
    min_quantizer: i32,
    max_quantizer: i32,
    tile_rows_log2: i32,
    tile_columns_log2: i32,
    auto_tiling: bool,
    // end of changeable.
}

#[derive(Debug, Default)]
pub struct Sample {
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

#[derive(Clone, Copy, Debug, Default)]
struct Grid {
    columns: u32,
    rows: u32,
    width: u32,
    height: u32,
}

pub type Codec = Box<dyn crate::codecs::Encoder>;

#[derive(Default)]
struct Item {
    id: u16,
    item_type: String,
    category: Category,
    codec: Option<Codec>,
    samples: Vec<Sample>,
    codec_configuration: CodecConfiguration,
    cell_index: usize,
    hidden_image: bool,
    infe_name: String,
    infe_content_type: String,
    mdat_offset_locations: Vec<usize>,
    iref_to_id: Option<u16>, // If some, then make an iref from this id to iref_to_id.
    iref_type: Option<String>,
    grid: Option<Grid>,
    associations: Vec<(
        u8,   // 1-based property_index
        bool, // essential
    )>,
    extra_layer_count: Option<u32>,
    dimg_from_id: Option<u16>, // If some, then make an iref from dimg_from_id to this id.
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "Item: {{ id: {}, item_type: {}, has_codec: {} }}",
            self.id,
            self.item_type,
            self.codec.is_some()
        )
    }
}

impl Item {
    pub(crate) fn has_ipma(&self) -> bool {
        // TODO: also include tonemap item.
        self.grid.is_some() || self.codec.is_some()
    }

    pub(crate) fn write_ispe(
        &mut self,
        stream: &mut OStream,
        image_metadata: &Image,
    ) -> AvifResult<()> {
        stream.start_full_box("ispe", (0, 0))?;
        let width = match self.grid {
            Some(grid) => grid.width,
            None => image_metadata.width,
        };
        // unsigned int(32) image_width;
        stream.write_u32(width)?;
        let height = match self.grid {
            Some(grid) => grid.height,
            None => image_metadata.height,
        };
        // unsigned int(32) image_height;
        stream.write_u32(height)?;
        stream.finish_box()?;
        Ok(())
    }

    pub(crate) fn write_pixi(
        &mut self,
        stream: &mut OStream,
        image_metadata: &Image,
    ) -> AvifResult<()> {
        stream.start_full_box("pixi", (0, 0))?;
        let num_channels = image_metadata.yuv_format.plane_count() as u8;
        // unsigned int (8) num_channels;
        stream.write_u8(num_channels)?;
        for _ in 0..num_channels {
            // unsigned int (8) bits_per_channel;
            stream.write_u8(image_metadata.depth)?;
        }
        stream.finish_box()?;
        Ok(())
    }

    pub(crate) fn write_codec_config(&mut self, stream: &mut OStream) -> AvifResult<()> {
        match &self.codec_configuration {
            CodecConfiguration::Av1(config) => {
                stream.start_box("av1C")?;
                // unsigned int (1) marker = 1;
                stream.write_bits(1, 1)?;
                // unsigned int (7) version = 1;
                stream.write_bits(1, 7)?;
                // unsigned int(3) seq_profile;
                stream.write_bits(config.seq_profile, 3)?;
                // unsigned int(5) seq_level_idx_0;
                stream.write_bits(config.seq_level_idx0, 5)?;
                // unsigned int(1) seq_tier_0;
                stream.write_bits(config.seq_tier0, 1)?;
                // unsigned int(1) high_bitdepth;
                stream.write_bits(config.high_bitdepth as u8, 1)?;
                // unsigned int(1) twelve_bit;
                stream.write_bits(config.twelve_bit as u8, 1)?;
                // unsigned int(1) monochrome;
                stream.write_bits(config.monochrome as u8, 1)?;
                // unsigned int(1) chroma_subsampling_x;
                stream.write_bits(config.chroma_subsampling_x, 1)?;
                // unsigned int(1) chroma_subsampling_y;
                stream.write_bits(config.chroma_subsampling_y, 1)?;
                // unsigned int(2) chroma_sample_position;
                stream.write_bits(config.chroma_sample_position as u8, 2)?;
                // unsigned int (3) reserved = 0;
                // unsigned int (1) initial_presentation_delay_present;
                // unsigned int (4) reserved = 0;
                stream.write_u8(0)?;
                stream.finish_box()?;
            }
            _ => {}
        }
        Ok(())
    }
}

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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Category {
    #[default]
    Color,
    Alpha,
    Gainmap,
}

impl Category {
    pub fn infe_name(&self) -> String {
        match self {
            Self::Color => "Color",
            Self::Alpha => "Alpha",
            Self::Gainmap => "GMap",
        }
        .into()
    }
}

impl Encoder {
    fn add_items(&mut self, grid: &Grid, category: Category) -> AvifResult<u16> {
        let cell_count = usize_from_u32(grid.rows * grid.columns)?;
        println!("### cell_count: {cell_count}");
        let mut top_level_item_id = 0;
        if cell_count > 1 {
            // TODO: stuff.
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
        grid_rows: u32,
        grid_columns: u32,
        cell_images: &[&Image],
        mut duration: u32,
        flags: u32,
    ) -> AvifResult<()> {
        // TODO: validate layer count.
        let cell_count: usize = usize_from_u32(grid_rows * grid_columns)?;
        if cell_count == 0 || cell_images.len() != cell_count {
            return Err(AvifError::InvalidArgument);
        }
        // TODO: detect changes.
        if duration == 0 {
            duration = 1;
        }

        if self.items.is_empty() {
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
                // TODO: Need to copy any of these?
                // pub clli: Option<ContentLightLevelInformation>,
                // pub pasp: Option<PixelAspectRatio>,
                // pub clap: Option<CleanAperture>,
                // pub irot_angle: Option<u8>,
                // pub imir_axis: Option<u8>,
                // pub exif: Vec<u8>,
                // pub icc: Vec<u8>,
                // pub xmp: Vec<u8>,
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
            // TODO: alpha stuff.
            // TODO: gainmap.
            // TODO: exif, xmp.
        } else {
            // TODO: stuff for layered and animations.
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
                // TODO: padding or something.
            }
            item.codec.unwrap_mut().encode_image(
                image,
                item.category,
                /*tile_rows_log2=*/ 0,
                /*tile_columns_log2=*/ 0,
                /*quantizer=*/ 50,
                /*disable_lagged_output=*/ true,
                &mut item.samples,
            )?;
        }
        self.duration_in_timescales.push(duration as u64);
        Ok(())
    }

    pub fn add_image(&mut self, image: &Image, duration: u32, flags: u32) -> AvifResult<()> {
        self.add_image_impl(1, 1, &[image], duration, flags)
    }

    #[allow(unused)]
    pub fn finish(&mut self) -> AvifResult<Vec<u8>> {
        if self.items.is_empty() {
            return Err(AvifError::NoContent);
        }
        for item in &mut self.items {
            if item.codec.is_none() {
                continue;
            }
            item.codec.unwrap_mut().finish()?;
            // TODO: check if sample count == duration count.

            if !item.samples.is_empty() {
                // Harvest codec configuration from sequence header.
                let sequence_header = Av1SequenceHeader::parse_from_obus(&item.samples[0].data)?;
                item.codec_configuration = CodecConfiguration::Av1(sequence_header.config);
            }
        }
        let image_metadata = &self.image_metadata;
        let now: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut stream = OStream::default();
        let is_sequence = self.duration_in_timescales.len() > 1;
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
        write_hdlr(&mut stream)?;
        write_pitm(&mut stream, self.primary_item_id)?;
        self.write_iloc(&mut stream)?;
        self.write_iinf(&mut stream)?;
        self.write_iref(&mut stream)?;
        self.write_iprp(&mut stream)?;
        stream.finish_box()?;
        // mdat box.
        self.write_mdat(&mut stream)?;
        Ok(stream.data)
    }

    fn write_iloc(&mut self, stream: &mut OStream) -> AvifResult<()> {
        stream.start_full_box("iloc", (0, 0))?;
        // unsigned int(4) offset_size;
        // unsigned int(4) length_size;
        stream.write_u8(0x44)?;
        // unsigned int(4) base_offset_size;
        // unsigned int(4) reserved;
        stream.write_u8(0)?;
        // unsigned int(16) item_count;
        stream.write_u16(u16_from_usize(self.items.len())?)?;

        for item in &mut self.items {
            // unsigned int(16) item_ID;
            stream.write_u16(item.id)?;
            // unsigned int(16) data_reference_index;
            stream.write_u16(0)?;

            // TODO: handle layered images.

            // unsigned int(16) extent_count;
            stream.write_u16(1)?;
            item.mdat_offset_locations.push(stream.offset());
            // unsigned int(offset_size*8) extent_offset;
            stream.write_u32(0)?;
            let extent_length = if item.samples.is_empty() {
                0
            } else {
                u32_from_usize(item.samples[0].data.len())?
            };
            // unsigned int(length_size*8) extent_length;
            stream.write_u32(extent_length)?;
        }

        stream.finish_box()?;
        Ok(())
    }

    fn write_iinf(&self, stream: &mut OStream) -> AvifResult<()> {
        stream.start_full_box("iinf", (0, 0))?;

        // unsigned int(16) entry_count;
        stream.write_u16(u16_from_usize(self.items.len())?)?;

        for item in &self.items {
            let flags = if item.hidden_image { 1 } else { 0 };
            stream.start_full_box("infe", (2, flags))?;
            // unsigned int(16) item_ID;
            stream.write_u16(item.id)?;
            // unsigned int(16) item_protection_index;
            stream.write_u16(0)?;
            // unsigned int(32) item_type;
            stream.write_string(&item.item_type)?;
            // utf8string item_name;
            stream.write_string_with_nul(&item.infe_name)?;
            match item.item_type.as_str() {
                "mime" => {
                    // utf8string content_type;
                    stream.write_string_with_nul(&item.infe_content_type)?
                    // utf8string content_encoding; //optional
                }
                "uri " => {
                    // utf8string item_uri_type;
                    return Err(AvifError::NotImplemented);
                }
                _ => {}
            }
            stream.finish_box()?;
        }

        stream.finish_box()?;
        Ok(())
    }

    fn write_iref(&self, _stream: &mut OStream) -> AvifResult<()> {
        // TODO: Implement this.
        Ok(())
    }

    fn write_iprp(&mut self, stream: &mut OStream) -> AvifResult<()> {
        stream.start_box("iprp")?;
        let mut property_index = 1;
        // ipco
        {
            stream.start_box("ipco")?;
            for item in &mut self.items {
                if !item.has_ipma() {
                    continue;
                }

                item.write_ispe(stream, &self.image_metadata)?;
                item.associations.push((property_index, false));
                property_index += 1;

                item.write_pixi(stream, &self.image_metadata)?;
                item.associations.push((property_index, false));
                property_index += 1;

                if item.codec.is_some() {
                    item.write_codec_config(stream)?;
                    item.associations.push((property_index, true));
                    property_index += 1;
                }
            }
            stream.finish_box()?;
        }
        // ipma
        {
            stream.start_full_box("ipma", (0, 0))?;
            let entry_count = u32_from_usize(
                self.items
                    .iter()
                    .filter(|&item| !item.associations.is_empty())
                    .count(),
            )?;
            // unsigned int(32) entry_count;
            stream.write_u32(entry_count)?;
            for item in &self.items {
                if item.associations.is_empty() {
                    continue;
                }
                // unsigned int(16) item_ID;
                stream.write_u16(item.id)?;
                // unsigned int(8) association_count;
                stream.write_u8(u8_from_usize(item.associations.len())?)?;
                for (property_index, essential) in &item.associations {
                    // bit(1) essential;
                    stream.write_bits(*essential as u8, 1)?;
                    if *property_index >= (1 << 7) {
                        return Err(AvifError::UnknownError("".into()));
                    }
                    // unsigned int(7) property_index;
                    stream.write_bits(*property_index, 7)?;
                }
            }
            stream.finish_box()?;
        }
        stream.finish_box()?;
        Ok(())
    }

    fn write_mdat(&self, stream: &mut OStream) -> AvifResult<()> {
        stream.start_box("mdat")?;
        let mdat_start_offset = stream.offset();
        for item in &self.items {
            // TODO: alpha, gainmap, dedupe, etc.
            if !item.samples.is_empty() {
                for sample in &item.samples {
                    stream.write_slice(&sample.data);
                }
            } else {
                // TODO: write metadata.
            }
            for mdat_offset_location in &item.mdat_offset_locations {
                stream.write_u32_at_offset(
                    u32_from_usize(mdat_start_offset)?,
                    *mdat_offset_location,
                )?;
            }
        }
        stream.finish_box()?;
        Ok(())
    }
}
