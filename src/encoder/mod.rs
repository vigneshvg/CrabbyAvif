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
    metadata_payload: Vec<u8>,
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
        let num_channels = if self.category == Category::Alpha {
            1
        } else {
            image_metadata.yuv_format.plane_count() as u8
        };
        // unsigned int (8) num_channels;
        stream.write_u8(num_channels)?;
        for _ in 0..num_channels {
            // unsigned int (8) bits_per_channel;
            stream.write_u8(image_metadata.depth)?;
        }
        stream.finish_box()?;
        Ok(())
    }

    pub(crate) fn write_codec_config(&self, stream: &mut OStream) -> AvifResult<()> {
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

    #[allow(non_snake_case)]
    pub(crate) fn write_auxC(&mut self, stream: &mut OStream) -> AvifResult<()> {
        stream.start_full_box("auxC", (0, 0))?;
        stream
            .write_string_with_nul(&String::from("urn:mpeg:mpegB:cicp:systems:auxiliary:alpha"))?;
        stream.finish_box()?;
        Ok(())
    }

    pub(crate) fn get_property_streams(
        &mut self,
        image_metadata: &Image,
        streams: &mut Vec<OStream>,
    ) -> AvifResult<()> {
        if !self.has_ipma() {
            return Ok(());
        }

        streams.push(OStream::default());
        self.write_ispe(streams.last_mut().unwrap(), image_metadata)?;
        self.associations
            .push((u8_from_usize(streams.len())?, false));

        streams.push(OStream::default());
        self.write_pixi(streams.last_mut().unwrap(), image_metadata)?;
        self.associations
            .push((u8_from_usize(streams.len())?, false));

        if self.codec.is_some() {
            streams.push(OStream::default());
            self.write_codec_config(streams.last_mut().unwrap())?;
            self.associations
                .push((u8_from_usize(streams.len())?, true));
        }

        match self.category {
            Category::Color => {
                // TODO: write color and hdr properties.
            }
            Category::Alpha => {
                streams.push(OStream::default());
                self.write_auxC(streams.last_mut().unwrap())?;
                self.associations
                    .push((u8_from_usize(streams.len())?, false));
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn write_tkhd(
        &self,
        stream: &mut OStream,
        image_metadata: &Image,
        duration: u64,
        timestamp: u64,
    ) -> AvifResult<()> {
        stream.start_full_box("tkhd", (1, 1))?;
        // unsigned int(64) creation_time;
        stream.write_u64(timestamp)?;
        // unsigned int(64) modification_time;
        stream.write_u64(timestamp)?;
        // unsigned int(32) track_ID;
        stream.write_u32(self.id as u32)?;
        // const unsigned int(32) reserved = 0;
        stream.write_u32(0)?;
        // unsigned int(64) duration;
        stream.write_u64(duration)?;
        // const unsigned int(32)[2] reserved = 0;
        stream.write_u32(0)?;
        stream.write_u32(0)?;
        // template int(16) layer = 0;
        stream.write_u16(0)?;
        // template int(16) alternate_group = 0;
        stream.write_u16(0)?;
        // template int(16) volume = {if track_is_audio 0x0100 else 0};
        stream.write_u16(0)?;
        // const unsigned int(16) reserved = 0;
        stream.write_u16(0)?;
        // template int(32)[9] matrix
        stream.write_slice(&UNITY_MATRIX)?;
        // unsigned int(32) width;
        stream.write_u32(image_metadata.width << 16)?;
        // unsigned int(32) height;
        stream.write_u32(image_metadata.height << 16)?;
        stream.finish_box()
    }

    pub(crate) fn write_vmhd(&self, stream: &mut OStream) -> AvifResult<()> {
        stream.start_full_box("vmhd", (0, 1))?;
        // template unsigned int(16) graphicsmode = 0; (copy over the existing image)
        stream.write_u16(0)?;
        // template unsigned int(16)[3] opcolor = {0, 0, 0};
        stream.write_u16(0)?;
        stream.write_u16(0)?;
        stream.write_u16(0)?;
        stream.finish_box()
    }

    pub(crate) fn write_dinf(&self, stream: &mut OStream) -> AvifResult<()> {
        stream.start_box("dinf")?;
        {
            stream.start_full_box("dref", (0, 0))?;
            // unsigned int(32) entry_count
            stream.write_u32(1)?;
            {
                // flags:1 means data is in this file
                stream.start_full_box("url ", (0, 1))?;
                stream.finish_box()?;
            }
            stream.finish_box()?;
        }
        stream.finish_box()
    }

    pub(crate) fn write_ccst(&self, stream: &mut OStream) -> AvifResult<()> {
        stream.start_full_box("ccst", (0, 0))?;
        // unsigned int(1) all_ref_pics_intra;
        stream.write_bits(0, 1)?;
        // unsigned int(1) intra_pred_used;
        stream.write_bits(1, 1)?;
        // unsigned int(4) max_ref_per_pic;
        stream.write_bits(15, 4)?;
        // unsigned int(26) reserved;
        stream.write_bits(0, 2)?;
        stream.write_u8(0)?;
        stream.write_u8(0)?;
        stream.write_u8(0)?;
        stream.finish_box()
    }

    pub(crate) fn write_stsd(
        &self,
        stream: &mut OStream,
        image_metadata: &Image,
    ) -> AvifResult<()> {
        stream.start_full_box("stsd", (0, 0))?;
        // unsigned int(32) entry_count;
        stream.write_u32(1)?;
        {
            stream.start_box("av01")?;
            // const unsigned int(8)[6] reserved = 0;
            for _ in 0..6 {
                stream.write_u8(0)?;
            }
            // unsigned int(16) data_reference_index;
            stream.write_u16(1)?;
            // unsigned int(16) pre_defined = 0;
            stream.write_u16(0)?;
            // const unsigned int(16) reserved = 0;
            stream.write_u16(0)?;
            // unsigned int(32)[3] pre_defined = 0;
            stream.write_u32(0)?;
            stream.write_u32(0)?;
            stream.write_u32(0)?;
            // unsigned int(16) width;
            stream.write_u16(u16_from_u32(image_metadata.width)?)?;
            // unsigned int(16) height;
            stream.write_u16(u16_from_u32(image_metadata.height)?)?;
            // template unsigned int(32) horizresolution
            stream.write_u32(0x00480000)?;
            // template unsigned int(32) vertresolution
            stream.write_u32(0x00480000)?;
            // const unsigned int(32) reserved = 0;
            stream.write_u32(0)?;
            // template unsigned int(16) frame_count = 1;
            stream.write_u16(1)?;
            // string[32] compressorname;
            const COMPRESSOR_NAME: &str = "AOM Coding                      ";
            assert_eq!(COMPRESSOR_NAME.len(), 32);
            stream.write_str(COMPRESSOR_NAME)?;
            // template unsigned int(16) depth = 0x0018;
            stream.write_u16(0x0018)?;
            // int(16) pre_defined = -1
            stream.write_u16(0xffff)?;

            self.write_codec_config(stream)?;
            if self.category == Category::Color {
                // TODO: write color, HDR and transformative properties.
            }
            self.write_ccst(stream)?;

            stream.finish_box()?;
        }
        stream.finish_box()
    }

    pub(crate) fn write_stts(
        &self,
        stream: &mut OStream,
        duration_in_timescales: &Vec<u64>,
    ) -> AvifResult<()> {
        let mut stts: Vec<(u64, u32)> = Vec::new();
        let mut current_value = None;
        let mut current_count = 0;
        for duration in duration_in_timescales {
            if let Some(current) = current_value {
                if *duration == current {
                    current_count += 1;
                } else {
                    stts.push((current, current_count));
                    current_value = Some(*duration);
                    current_count = 1;
                }
            } else {
                current_value = Some(*duration);
                current_count = 1;
            }
        }
        if let Some(current) = current_value {
            stts.push((current, current_count));
        }

        stream.start_full_box("stts", (0, 0))?;
        // unsigned int(32) entry_count;
        stream.write_u32(u32_from_usize(stts.len())?)?;
        for (sample_delta, sample_count) in stts {
            // unsigned int(32) sample_count;
            stream.write_u32(sample_count)?;
            // unsigned int(32) sample_delta;
            stream.write_u32(u32_from_u64(sample_delta)?)?;
        }
        stream.finish_box()
    }

    pub(crate) fn write_stsc(&self, stream: &mut OStream) -> AvifResult<()> {
        stream.start_full_box("stsc", (0, 0))?;
        // unsigned int(32) entry_count;
        stream.write_u32(1)?;
        // unsigned int(32) first_chunk;
        stream.write_u32(1)?;
        // unsigned int(32) samples_per_chunk;
        stream.write_u32(u32_from_usize(self.samples.len())?)?;
        // unsigned int(32) sample_description_index;
        stream.write_u32(1)?;
        stream.finish_box()
    }

    pub(crate) fn write_stsz(&self, stream: &mut OStream) -> AvifResult<()> {
        stream.start_full_box("stsz", (0, 0))?;
        // unsigned int(32) sample_size;
        stream.write_u32(0)?;
        // unsigned int(32) sample_count;
        stream.write_u32(u32_from_usize(self.samples.len())?)?;
        for sample in &self.samples {
            // unsigned int(32) entry_size;
            stream.write_u32(u32_from_usize(sample.data.len())?)?;
        }
        stream.finish_box()
    }

    pub(crate) fn write_stco(&mut self, stream: &mut OStream) -> AvifResult<()> {
        stream.start_full_box("stco", (0, 0))?;
        // unsigned int(32) entry_count;
        stream.write_u32(1)?;
        // unsigned int(32) chunk_offset;
        self.mdat_offset_locations.push(stream.offset());
        stream.write_u32(0)?;
        stream.finish_box()
    }

    pub(crate) fn write_stss(&mut self, stream: &mut OStream) -> AvifResult<()> {
        let sync_samples_count = self.samples.iter().filter(|x| x.sync).count();
        if sync_samples_count == self.samples.len() {
            // ISO/IEC 14496-12, Section 8.6.2.1:
            //   If the SyncSampleBox is not present, every sample is a sync sample.
            return Ok(());
        }
        stream.start_full_box("stss", (0, 0))?;
        // unsigned int(32) entry_count;
        stream.write_u32(u32_from_usize(sync_samples_count)?)?;
        for (index, sample) in self.samples.iter().enumerate() {
            if !sample.sync {
                continue;
            }
            // unsigned int(32) sample_number;
            stream.write_u32(u32_from_usize(index + 1)?)?;
        }
        stream.finish_box()
    }

    pub(crate) fn write_stbl(
        &mut self,
        stream: &mut OStream,
        image_metadata: &Image,
        duration_in_timescales: &Vec<u64>,
    ) -> AvifResult<()> {
        stream.start_box("stbl")?;
        self.write_stsd(stream, image_metadata)?;
        self.write_stts(stream, duration_in_timescales)?;
        self.write_stsc(stream)?;
        self.write_stsz(stream)?;
        self.write_stco(stream)?;
        self.write_stss(stream)?;
        stream.finish_box()
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

fn write_grid(stream: &mut OStream, grid: &Grid) -> AvifResult<()> {
    // ISO/IEC 23008-12 6.6.2.3.2
    // aligned(8) class ImageGrid {
    //     unsigned int(8) version = 0;
    //     unsigned int(8) flags;
    //     FieldLength = ((flags & 1) + 1) * 16;
    //     unsigned int(8) rows_minus_one;
    //     unsigned int(8) columns_minus_one;
    //     unsigned int(FieldLength) output_width;
    //     unsigned int(FieldLength) output_height;
    // }
    let flags = if grid.width > 65535 || grid.height > 65535 { 1 } else { 0 };
    // unsigned int(8) version = 0;
    stream.write_u8(0)?;
    // unsigned int(8) flags;
    stream.write_u8(flags)?;
    // unsigned int(8) rows_minus_one;
    stream.write_u8(grid.rows as u8 - 1)?;
    // unsigned int(8) columns_minus_one;
    stream.write_u8(grid.columns as u8 - 1)?;
    // unsigned int(FieldLength) output_width;
    // unsigned int(FieldLength) output_height;
    if flags == 1 {
        stream.write_u32(grid.width)?;
        stream.write_u32(grid.height)?;
    } else {
        stream.write_u16(grid.width as u16)?;
        stream.write_u16(grid.height as u16)?;
    }
    Ok(())
}

impl Encoder {
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
            // TODO: validate image against self.image_metadata.
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
            item.codec.unwrap_mut().encode_image(
                image,
                item.category,
                /*tile_rows_log2=*/ 0,
                /*tile_columns_log2=*/ 0,
                /*quantizer=*/ 50,
                /*disable_lagged_output=*/ true,
                is_single_image,
                &mut item.samples,
            )?;
        }
        self.duration_in_timescales.push(duration as u64);
        Ok(())
    }

    pub fn add_image(&mut self, image: &Image) -> AvifResult<()> {
        self.add_image_impl(1, 1, &[image], 0, true)
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
                u32_from_usize(item.metadata_payload.len())?
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

    fn write_iref(&self, stream: &mut OStream) -> AvifResult<()> {
        let mut box_started = false;
        for item in &self.items {
            let dimg_item_ids: Vec<_> = self
                .items
                .iter()
                .filter(|dimg_item| dimg_item.dimg_from_id.unwrap_or_default() == item.id)
                .map(|dimg_item| dimg_item.id)
                .collect();
            if !dimg_item_ids.is_empty() {
                if !box_started {
                    stream.start_full_box("iref", (0, 0))?;
                    box_started = true;
                }
                stream.start_box("dimg")?;
                // unsigned int(16) from_item_ID;
                stream.write_u16(item.id)?;
                // unsigned int(16) reference_count;
                stream.write_u16(u16_from_usize(dimg_item_ids.len())?)?;
                for dimg_item_id in dimg_item_ids {
                    // unsigned int(16) to_item_ID;
                    stream.write_u16(dimg_item_id)?;
                }
                stream.finish_box()?;
            }
            if let Some(iref_to_id) = item.iref_to_id {
                if !box_started {
                    stream.start_full_box("iref", (0, 0))?;
                    box_started = true;
                }
                stream.start_box(item.iref_type.as_ref().unwrap().as_str())?;
                // unsigned int(16) from_item_ID;
                stream.write_u16(item.id)?;
                // unsigned int(16) reference_count;
                stream.write_u16(1)?;
                // unsigned int(16) to_item_ID;
                stream.write_u16(iref_to_id)?;
                stream.finish_box()?;
            }
        }
        if box_started {
            stream.finish_box()?;
        }
        Ok(())
    }

    fn write_iprp(&mut self, stream: &mut OStream) -> AvifResult<()> {
        stream.start_box("iprp")?;
        // ipco
        stream.start_box("ipco")?;
        let mut property_streams = Vec::new();
        for item in &mut self.items {
            item.get_property_streams(&self.image_metadata, &mut property_streams)?;
        }
        // Deduplicate the property streams.
        let mut property_index_map = Vec::new();
        let mut last_written_property_index = 0u8;
        for i in 0..property_streams.len() {
            let current_data = &property_streams[i].data;
            match property_streams[0..i]
                .iter()
                .position(|x| x.data == *current_data)
            {
                Some(property_index) => {
                    // A duplicate stream was already written. Simply store the index of that
                    // stream.
                    property_index_map.push(property_index_map[property_index as usize]);
                }
                None => {
                    // No duplicate streams were found. Write this stream and store its index.
                    stream.write_slice(current_data)?;
                    last_written_property_index += 1;
                    property_index_map.push(last_written_property_index);
                }
            }
        }
        stream.finish_box()?;
        // end of ipco

        // ipma
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
                // property_index_map is 0-indexed whereas the index stored in item.associations is
                // 1-indexed.
                let index = property_index_map[*property_index as usize - 1];
                if index >= (1 << 7) {
                    return Err(AvifError::UnknownError("".into()));
                }
                // unsigned int(7) property_index;
                stream.write_bits(index, 7)?;
            }
        }
        stream.finish_box()?;
        // end of ipma

        stream.finish_box()?;
        Ok(())
    }

    fn write_mvhd(
        &mut self,
        stream: &mut OStream,
        duration: u64,
        timestamp: u64,
    ) -> AvifResult<()> {
        stream.start_full_box("mvhd", (1, 0))?;
        // unsigned int(64) creation_time;
        stream.write_u64(timestamp)?;
        // unsigned int(64) modification_time;
        stream.write_u64(timestamp)?;
        // unsigned int(32) timescale;
        stream.write_u32(u32_from_u64(self.settings.timescale)?)?;
        // unsigned int(64) duration;
        stream.write_u64(duration)?;
        // template int(32) rate = 0x00010000; // typically 1.0
        stream.write_u32(0x00010000)?;
        // template int(16) volume = 0x0100; // typically, full volume
        stream.write_u16(0x0100)?;
        // const bit(16) reserved = 0;
        stream.write_u16(0)?;
        // const unsigned int(32)[2] reserved = 0;
        stream.write_u32(0)?;
        stream.write_u32(0)?;
        // template int(32)[9] matrix
        stream.write_slice(&UNITY_MATRIX)?;
        // bit(32)[6] pre_defined = 0;
        for _ in 0..6 {
            stream.write_u32(0)?;
        }
        println!("### ITEMS LEN: {}", self.items.len());
        // unsigned int(32) next_track_ID;
        stream.write_u32(u32_from_usize(self.items.len())?)?;
        stream.finish_box()
    }

    fn write_tracks(
        &mut self,
        stream: &mut OStream,
        duration: u64,
        timestamp: u64,
    ) -> AvifResult<()> {
        for item in &mut self.items {
            if item.samples.is_empty() {
                continue;
            }
            stream.start_box("trak")?;
            item.write_tkhd(stream, &self.image_metadata, duration, timestamp)?;
            if let Some(iref_to_id) = item.iref_to_id {
                todo!("write tref box");
            }
            // TODO: write edts box.
            if item.category == Category::Color {
                // TODO: write track meta box.
            }
            // mdia
            {
                stream.start_box("mdia")?;
                // mdhd
                {
                    stream.start_full_box("mdhd", (1, 0))?;
                    // unsigned int(64) creation_time;
                    stream.write_u64(timestamp)?;
                    // unsigned int(64) modification_time;
                    stream.write_u64(timestamp)?;
                    // unsigned int(32) timescale;
                    stream.write_u32(u32_from_u64(self.settings.timescale)?)?;
                    // unsigned int(64) duration;
                    stream.write_u64(duration)?;
                    // bit(1) pad = 0; unsigned int(5)[3] language; ("und")
                    stream.write_u16(21956)?;
                    // unsigned int(16) pre_defined = 0;
                    stream.write_u16(0)?;
                    stream.finish_box()?;
                }
                write_hdlr(
                    stream,
                    &String::from(if item.category == Category::Alpha { "auxv" } else { "pict" }),
                )?;
                // minf
                {
                    stream.start_box("minf")?;
                    item.write_vmhd(stream)?;
                    item.write_dinf(stream)?;
                    item.write_stbl(stream, &self.image_metadata, &self.duration_in_timescales)?;
                    stream.finish_box()?;
                }
                stream.finish_box()?;
            }
            stream.finish_box()?;
        }
        Ok(())
    }

    #[allow(unused)]
    fn write_mdat(&self, stream: &mut OStream) -> AvifResult<()> {
        stream.start_box("mdat")?;
        let mdat_start_offset = stream.offset();
        // Use multiple passes to pack the items in the following order:
        //   * Pass 0: metadata (Exif/XMP/gain map metadata)
        //   * Pass 1: alpha, gain map image (AV1)
        //   * Pass 2: all other item data (AV1 color)
        //
        // See here for the discussion on alpha coming before color:
        // https://github.com/AOMediaCodec/libavif/issues/287
        //
        // Exif and XMP are packed first as they're required to be fully available by
        // Decoder::parse() before it returns AVIF_RESULT_OK, unless ignore_xmp and ignore_exif are
        // enabled.
        for pass in 0..=2 {
            for item in &self.items {
                if pass == 0
                    && item.item_type != "mime"
                    && item.item_type != "Exif"
                    && item.item_type != "tmap"
                {
                    continue;
                }
                if pass == 1 && !matches!(item.category, Category::Alpha | Category::Gainmap) {
                    continue;
                }
                if pass == 2 && item.category != Category::Color {
                    continue;
                }
                let chunk_offset = stream.offset();
                // TODO: alpha, gainmap, dedupe, etc.
                if !item.samples.is_empty() {
                    for sample in &item.samples {
                        stream.write_slice(&sample.data);
                    }
                } else if !item.metadata_payload.is_empty() {
                    println!("### writing metadata payload");
                    stream.write_slice(&item.metadata_payload);
                } else {
                    panic!("### empty item");
                    // TODO: empty item, ignore or error?
                }
                for mdat_offset_location in &item.mdat_offset_locations {
                    stream.write_u32_at_offset(
                        u32_from_usize(chunk_offset)?,
                        *mdat_offset_location,
                    )?;
                }
            }
        }
        stream.finish_box()?;
        Ok(())
    }
}
