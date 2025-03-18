// Copyright 2024 Google LLC
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

#[cfg(feature = "dav1d")]
pub mod dav1d;

#[cfg(feature = "libgav1")]
pub mod libgav1;

#[cfg(feature = "android_mediacodec")]
pub mod android_mediacodec;

use crate::decoder::GridImageHelper;
use crate::image::Image;
use crate::parser::mp4box::CodecConfiguration;
use crate::AndroidMediaCodecOutputColorFormat;
use crate::AvifResult;
use crate::Category;

use std::num::NonZero;

#[derive(Clone, Default)]
pub struct DecoderConfig {
    pub operating_point: u8,
    pub all_layers: bool,
    pub width: u32,
    pub height: u32,
    pub depth: u8,
    pub max_threads: u32,
    pub image_size_limit: Option<NonZero<u32>>,
    pub max_input_size: usize,
    pub codec_config: CodecConfiguration,
    pub category: Category,
    pub android_mediacodec_output_color_format: AndroidMediaCodecOutputColorFormat,
}

pub trait Decoder {
    fn initialize(&mut self, config: &DecoderConfig) -> AvifResult<()>;
    // Decode a single image and write the output into |image|.
    fn get_next_image(
        &mut self,
        av1_payload: &[u8],
        spatial_id: u8,
        image: &mut Image,
        category: Category,
    ) -> AvifResult<()>;
    // Decode a list of input images and outputs them into the |grid_image_helper|.
    fn get_next_image_grid(
        &mut self,
        payloads: &[Vec<u8>],
        spatial_id: u8,
        grid_image_helper: &mut GridImageHelper,
    ) -> AvifResult<()>;
    // Destruction must be implemented using Drop.
}
