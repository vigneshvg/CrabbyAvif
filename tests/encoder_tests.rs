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

//use crabby_avif::decoder::track::RepetitionCount;
use crabby_avif::decoder::Category;
use crabby_avif::decoder::CompressionFormat;
use crabby_avif::image::*;
//use crabby_avif::reformat::rgb;
use crabby_avif::*;

use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

#[path = "./mod.rs"]
mod tests;

use tests::*;

fn generate_random_image(
    width: u32,
    height: u32,
    depth: u8,
    yuv_format: PixelFormat,
    yuv_range: YuvRange,
    alpha: bool,
) -> AvifResult<Image> {
    let mut image = image::Image {
        width,
        height,
        depth,
        yuv_format,
        yuv_range,
        ..Default::default()
    };
    image.allocate_planes(Category::Color)?;
    if alpha {
        image.allocate_planes(Category::Alpha)?;
    }
    let mut rng: StdRng = SeedableRng::seed_from_u64(0xABCDEF);
    for plane in ALL_PLANES {
        if !image.has_plane(plane) {
            continue;
        }
        let plane_data = image.plane_data(plane).unwrap();
        for y in 0..plane_data.height {
            if image.depth == 8 {
                let row = image.row_mut(plane, y)?;
                let row_slice = &mut row[..plane_data.width as usize];
                for pixel in row_slice {
                    *pixel = rng.gen_range(0..=255);
                }
            } else {
                todo!("whoa pixel depth >8");
            }
        }
    }
    Ok(image)
}

#[test_case::test_matrix(
    [100, 121],
    [200, 107],
    [8, 10, 12],
    [PixelFormat::Yuv420, PixelFormat::Yuv422, PixelFormat::Yuv444, PixelFormat::Yuv400],
    [YuvRange::Limited, YuvRange::Full],
    [false, true]
)]
fn encode_decode(
    width: u32,
    height: u32,
    depth: u8,
    yuv_format: PixelFormat,
    yuv_range: YuvRange,
    alpha: bool,
) -> AvifResult<()> {
    if !HAS_ENCODER {
        return Ok(());
    }
    if depth > 8 || !matches!(yuv_format, PixelFormat::Yuv420) {
        return Ok(());
    }
    let input_image = generate_random_image(width, height, depth, yuv_format, yuv_range, alpha)?;
    let mut encoder: encoder::Encoder = Default::default();
    encoder.add_image(&input_image)?;
    let edata = encoder.finish()?;
    assert!(!edata.is_empty());

    let mut decoder = decoder::Decoder::default();
    decoder.set_io_vec(edata);
    assert!(decoder.parse().is_ok());
    assert_eq!(decoder.compression_format(), CompressionFormat::Avif);
    assert_eq!(decoder.image_count(), 1);

    let image = decoder.image().expect("image was none");
    assert_eq!(image.alpha_present, alpha);
    assert!(!image.image_sequence_track_present);
    assert_eq!(image.width, width);
    assert_eq!(image.height, height);
    assert_eq!(image.depth, depth);
    assert_eq!(image.yuv_format, yuv_format);
    assert_eq!(image.yuv_range, yuv_range);

    if !HAS_DECODER {
        return Ok(());
    }
    assert!(decoder.next_image().is_ok());
    Ok(())
}
