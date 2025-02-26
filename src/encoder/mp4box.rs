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

use crate::encoder::*;

use crate::image::*;
use crate::internal_utils::stream::OStream;
use crate::internal_utils::*;
use crate::parser::mp4box::*;
use crate::parser::obu::Av1SequenceHeader;
use crate::*;

pub(crate) fn write_ftyp(stream: &mut OStream, ftyp: &FileTypeBox) -> AvifResult<()> {
    stream.start_box("ftyp")?;
    // unsigned int(32) major_brand;
    stream.write_string(&ftyp.major_brand)?;
    // unsigned int(32) minor_version;
    stream.write_u32(0)?;
    // unsigned int(32) compatible_brands[];
    for compatible_brand in &ftyp.compatible_brands {
        stream.write_string(compatible_brand)?;
    }
    stream.finish_box()?;
    Ok(())
}

pub(crate) fn write_hdlr(stream: &mut OStream, handler_type: &String) -> AvifResult<()> {
    stream.start_full_box("hdlr", (0, 0))?;
    // unsigned int(32) pre_defined = 0;
    stream.write_u32(0)?;
    // unsigned int(32) handler_type;
    stream.write_str(handler_type)?;
    // const unsigned int(32)[3] reserved = 0;
    stream.write_u32(0)?;
    stream.write_u32(0)?;
    stream.write_u32(0)?;
    // string name;
    stream.write_string_with_nul(&String::from(""))?;
    stream.finish_box()?;
    Ok(())
}

pub(crate) fn write_pitm(stream: &mut OStream, item_id: u16) -> AvifResult<()> {
    stream.start_full_box("pitm", (0, 0))?;
    //  unsigned int(16) item_ID;
    stream.write_u16(item_id)?;
    stream.finish_box()?;
    Ok(())
}

pub(crate) fn write_grid(stream: &mut OStream, grid: &Grid) -> AvifResult<()> {
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
    pub(crate) fn write_iloc(&mut self, stream: &mut OStream) -> AvifResult<()> {
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

    pub(crate) fn write_iinf(&self, stream: &mut OStream) -> AvifResult<()> {
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

    pub(crate) fn write_iref(&self, stream: &mut OStream) -> AvifResult<()> {
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

    pub(crate) fn write_iprp(&mut self, stream: &mut OStream) -> AvifResult<()> {
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

    pub(crate) fn write_mvhd(
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

    pub(crate) fn write_tracks(
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
    pub(crate) fn write_mdat(&self, stream: &mut OStream) -> AvifResult<()> {
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
