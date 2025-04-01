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

#![cfg(feature = "encoder")]

use crabby_avif::decoder::CompressionFormat;
use crabby_avif::decoder::ImageContentType;
use crabby_avif::decoder::ProgressiveState;
use crabby_avif::encoder::ScalingMode;
use crabby_avif::gainmap::*;
use crabby_avif::image::*;
use crabby_avif::utils::*;
use crabby_avif::*;

#[path = "./mod.rs"]
mod tests;

use tests::*;

fn full_to_limited_pixel(min: i32, max: i32, full: i32, v: u16) -> u16 {
    let v = v as i32;
    let v = (((v * (max - min)) + (full / 2)) / full) + min;
    if v < min {
        min as u16
    } else if v > max {
        max as u16
    } else {
        v as u16
    }
}

fn full_to_limited(v: u16, plane: Plane, depth: u8) -> u16 {
    match (plane, depth) {
        (Plane::Y, 8) => full_to_limited_pixel(16, 235, 255, v),
        (Plane::Y, 10) => full_to_limited_pixel(64, 940, 1023, v),
        (Plane::Y, 12) => full_to_limited_pixel(256, 3760, 4095, v),
        (Plane::U | Plane::V, 8) => full_to_limited_pixel(16, 240, 255, v),
        (Plane::U | Plane::V, 10) => full_to_limited_pixel(64, 960, 1023, v),
        (Plane::U | Plane::V, 12) => full_to_limited_pixel(256, 3840, 4095, v),
        _ => unreachable!(""),
    }
}

fn generate_gradient_image(
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
    for plane in ALL_PLANES {
        if !image.has_plane(plane) {
            continue;
        }
        let plane_data = image.plane_data(plane).unwrap();
        let max_xy_sum = plane_data.width + plane_data.height - 2;
        for y in 0..plane_data.height {
            if image.depth == 8 {
                let row = image.row_mut(plane, y)?;
                for x in 0..plane_data.width {
                    let value = (x + y) % (max_xy_sum + 1);
                    row[x as usize] = (value * 255 / std::cmp::max(1, max_xy_sum)) as u8;
                    if yuv_range == YuvRange::Limited && plane != Plane::A {
                        row[x as usize] =
                            full_to_limited(row[x as usize] as u16, plane, depth) as u8;
                    }
                }
            } else {
                let max_channel = image.max_channel() as u32;
                let row = image.row16_mut(plane, y)?;
                for x in 0..plane_data.width {
                    let value = (x + y) % (max_xy_sum + 1);
                    row[x as usize] = (value * max_channel / std::cmp::max(1, max_xy_sum)) as u16;
                    if yuv_range == YuvRange::Limited && plane != Plane::A {
                        row[x as usize] = full_to_limited(row[x as usize], plane, depth);
                    }
                }
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
    let input_image = generate_gradient_image(width, height, depth, yuv_format, yuv_range, alpha)?;
    let settings = encoder::Settings {
        speed: Some(10),
        mutable: encoder::MutableSettings {
            quality: 90,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut encoder = encoder::Encoder::create_with_settings(&settings)?;
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
    assert_eq!(image.pasp, input_image.pasp);
    assert_eq!(image.clli, input_image.clli);
    // TODO: test for other properties.

    if !HAS_DECODER {
        return Ok(());
    }
    assert!(decoder.next_image().is_ok());
    Ok(())
}

#[test_case::test_matrix(
    [100, 121],
    [200, 107],
    [8, 10, 12],
    [PixelFormat::Yuv420, PixelFormat::Yuv422, PixelFormat::Yuv444, PixelFormat::Yuv400],
    [YuvRange::Limited, YuvRange::Full],
    [false, true]
)]
fn encode_decode_sequence(
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
    let mut input_images = Vec::new();
    let frame_count = 10;
    for _ in 0..frame_count {
        input_images.push(generate_gradient_image(
            width, height, depth, yuv_format, yuv_range, alpha,
        )?);
    }
    let images: Vec<&Image> = input_images.iter().collect();
    let settings = encoder::Settings {
        speed: Some(6),
        timescale: 10000,
        mutable: encoder::MutableSettings {
            quality: 50,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut encoder = encoder::Encoder::create_with_settings(&settings)?;
    for image in images {
        encoder.add_image_for_sequence(image, 1000)?;
    }
    let edata = encoder.finish()?;
    assert!(!edata.is_empty());

    let mut decoder = decoder::Decoder::default();
    decoder.set_io_vec(edata);
    assert!(decoder.parse().is_ok());
    assert_eq!(decoder.compression_format(), CompressionFormat::Avif);
    assert_eq!(decoder.image_count(), 10);

    let image = decoder.image().expect("image was none");
    assert_eq!(image.alpha_present, alpha);
    assert!(image.image_sequence_track_present);
    assert_eq!(image.width, width);
    assert_eq!(image.height, height);
    assert_eq!(image.depth, depth);
    assert_eq!(image.yuv_format, yuv_format);
    assert_eq!(image.yuv_range, yuv_range);

    if !HAS_DECODER {
        return Ok(());
    }
    for _ in 0..frame_count {
        assert!(decoder.next_image().is_ok());
    }
    Ok(())
}

#[test_case::test_matrix([0, 1, 65535], [0, 1, 65535])]
fn clli(max_cll: u16, max_pall: u16) -> AvifResult<()> {
    if !HAS_ENCODER || !HAS_DECODER {
        return Ok(());
    }
    let mut image = generate_gradient_image(8, 8, 8, PixelFormat::Yuv444, YuvRange::Full, false)?;
    image.clli = Some(ContentLightLevelInformation { max_cll, max_pall });

    let settings = encoder::Settings {
        speed: Some(10),
        ..Default::default()
    };
    let mut encoder = encoder::Encoder::create_with_settings(&settings)?;
    encoder.add_image(&image)?;
    let edata = encoder.finish()?;
    assert!(!edata.is_empty());

    let mut decoder = decoder::Decoder::default();
    decoder.set_io_vec(edata);
    assert!(decoder.parse().is_ok());
    let decoded_image = decoder.image().unwrap();
    assert_eq!(decoded_image.clli, image.clli);

    Ok(())
}

fn test_progressive_decode(
    edata: Vec<u8>,
    width: u32,
    height: u32,
    extra_layer_count: u32,
) -> AvifResult<()> {
    let mut decoder = decoder::Decoder::default();
    decoder.settings.allow_progressive = true;
    decoder.set_io_vec(edata);
    assert!(decoder.parse().is_ok());
    let image = decoder.image().expect("image was none");
    assert!(matches!(image.progressive_state, ProgressiveState::Active));
    assert_eq!(decoder.image_count(), extra_layer_count + 1);
    assert_eq!(image.width, width);
    assert_eq!(image.height, height);
    if !HAS_DECODER {
        return Ok(());
    }
    for _ in 0..extra_layer_count + 1 {
        let res = decoder.next_image();
        assert!(res.is_ok());
        let image = decoder.image().expect("image was none");
        assert_eq!(image.width, width);
        assert_eq!(image.height, height);
    }
    Ok(())
}

#[test_case::test_matrix([true, false])]
fn progressive_quality_change(use_grid: bool) -> AvifResult<()> {
    if !HAS_ENCODER {
        return Ok(());
    }
    let image = generate_gradient_image(256, 256, 8, PixelFormat::Yuv444, YuvRange::Full, false)?;
    let mut settings = encoder::Settings {
        speed: Some(10),
        extra_layer_count: 1,
        mutable: encoder::MutableSettings {
            quality: 2,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut encoder = encoder::Encoder::create_with_settings(&settings)?;
    let images = [&image, &image];
    if use_grid {
        encoder.add_image_grid(2, 1, &images)?;
    } else {
        encoder.add_image(&image)?;
    }
    settings.mutable.quality = 90;
    encoder.update_settings(&settings.mutable)?;
    if use_grid {
        encoder.add_image_grid(2, 1, &images)?;
    } else {
        encoder.add_image(&image)?;
    }
    let edata = encoder.finish()?;
    assert!(!edata.is_empty());
    test_progressive_decode(
        edata,
        if use_grid { 512 } else { 256 },
        256,
        settings.extra_layer_count,
    )?;
    Ok(())
}

#[test_case::test_matrix([IFraction(1,2), IFraction(2, 6), IFraction(4, 32)], [true, false])]
fn progressive_dimension_change(scaling_fraction: IFraction, use_grid: bool) -> AvifResult<()> {
    if !HAS_ENCODER {
        return Ok(());
    }
    let image = generate_gradient_image(256, 256, 8, PixelFormat::Yuv444, YuvRange::Full, false)?;
    let mut settings = encoder::Settings {
        speed: Some(10),
        extra_layer_count: 1,
        mutable: encoder::MutableSettings {
            quality: 100,
            scaling_mode: ScalingMode {
                horizontal: scaling_fraction,
                vertical: scaling_fraction,
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let mut encoder = encoder::Encoder::create_with_settings(&settings)?;
    let images = [&image, &image];
    if use_grid {
        encoder.add_image_grid(2, 1, &images)?;
    } else {
        encoder.add_image(&image)?;
    }
    settings.mutable.scaling_mode = ScalingMode::default();
    encoder.update_settings(&settings.mutable)?;
    if use_grid {
        encoder.add_image_grid(2, 1, &images)?;
    } else {
        encoder.add_image(&image)?;
    }
    let edata = encoder.finish()?;
    assert!(!edata.is_empty());
    test_progressive_decode(
        edata,
        if use_grid { 512 } else { 256 },
        256,
        settings.extra_layer_count,
    )?;
    Ok(())
}

#[test]
fn progressive_same_layers() -> AvifResult<()> {
    if !HAS_ENCODER {
        return Ok(());
    }
    let image = generate_gradient_image(256, 256, 8, PixelFormat::Yuv444, YuvRange::Full, false)?;
    let settings = encoder::Settings {
        extra_layer_count: 3,
        speed: Some(10),
        mutable: encoder::MutableSettings {
            quality: 50,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut encoder = encoder::Encoder::create_with_settings(&settings)?;
    for _ in 0..4 {
        encoder.add_image(&image)?;
    }
    let edata = encoder.finish()?;
    assert!(!edata.is_empty());
    test_progressive_decode(edata, 256, 256, settings.extra_layer_count)?;
    Ok(())
}

#[test]
fn progressive_incorrect_number_of_layers() -> AvifResult<()> {
    if !HAS_ENCODER {
        return Ok(());
    }
    let image = generate_gradient_image(256, 256, 8, PixelFormat::Yuv444, YuvRange::Full, false)?;
    let settings = encoder::Settings {
        speed: Some(10),
        extra_layer_count: 1,
        mutable: encoder::MutableSettings {
            quality: 50,
            ..Default::default()
        },
        ..Default::default()
    };

    // Too many layers.
    let mut encoder = encoder::Encoder::create_with_settings(&settings)?;
    assert!(encoder.add_image(&image).is_ok());
    assert!(encoder.add_image(&image).is_ok());
    assert!(encoder.add_image(&image).is_err());

    // Too few layers.
    encoder = encoder::Encoder::create_with_settings(&settings)?;
    assert!(encoder.add_image(&image).is_ok());
    assert!(encoder.finish().is_err());
    Ok(())
}

fn gainmap_metadata(base_is_hdr: bool) -> GainMapMetadata {
    let mut metadata = GainMapMetadata {
        use_base_color_space: true,
        base_hdr_headroom: if base_is_hdr { UFraction(6, 2) } else { UFraction(0, 1) },
        alternate_hdr_headroom: if base_is_hdr { UFraction(0, 1) } else { UFraction(6, 2) },
        ..Default::default()
    };
    for c in 0..3u32 {
        metadata.base_offset[c as usize] = Fraction(c as i32 * 10, 1000);
        metadata.alternate_offset[c as usize] = Fraction(c as i32 * 20, 1000);
        metadata.gamma[c as usize] = UFraction(1, c + 1);
        metadata.min[c as usize] = Fraction(-1, c + 1);
        metadata.max[c as usize] = Fraction(c as i32 + 11, c + 1);
    }
    metadata
}

fn generate_gainmap_image(base_is_hdr: bool) -> AvifResult<(Image, GainMap)> {
    let mut image =
        generate_gradient_image(12, 34, 10, PixelFormat::Yuv420, YuvRange::Full, false)?;
    image.transfer_characteristics = if base_is_hdr {
        TransferCharacteristics::Pq
    } else {
        TransferCharacteristics::Srgb
    };
    let mut gainmap = GainMap {
        image: generate_gradient_image(6, 17, 8, PixelFormat::Yuv420, YuvRange::Full, false)?,
        metadata: gainmap_metadata(base_is_hdr),
        ..Default::default()
    };
    gainmap.alt_plane_count = 3;
    gainmap.alt_matrix_coefficients = MatrixCoefficients::Smpte2085;
    let clli = ContentLightLevelInformation {
        max_cll: 10,
        max_pall: 5,
    };
    if base_is_hdr {
        image.clli = Some(clli);
        gainmap.alt_plane_depth = 8;
        gainmap.alt_color_primaries = ColorPrimaries::Bt601;
        gainmap.alt_transfer_characteristics = TransferCharacteristics::Srgb;
    } else {
        gainmap.alt_clli = clli;
        gainmap.alt_plane_depth = 10;
        gainmap.alt_color_primaries = ColorPrimaries::Bt2020;
        gainmap.alt_transfer_characteristics = TransferCharacteristics::Pq;
    }
    Ok((image, gainmap))
}

#[test]
fn gainmap_base_image_sdr() -> AvifResult<()> {
    let (image, gainmap) = generate_gainmap_image(false)?;
    let settings = encoder::Settings {
        speed: Some(10),
        ..Default::default()
    };
    let mut encoder = encoder::Encoder::create_with_settings(&settings)?;
    encoder.add_image_gainmap(&image, &gainmap)?;
    let edata = encoder.finish()?;
    assert!(!edata.is_empty());

    let mut decoder = decoder::Decoder::default();
    decoder.set_io_vec(edata);
    decoder.settings.image_content_to_decode = ImageContentType::All;
    assert!(decoder.parse().is_ok());
    assert!(decoder.gainmap_present());
    let decoded_gainmap = decoder.gainmap();
    assert_eq!(
        decoded_gainmap.image.matrix_coefficients,
        gainmap.image.matrix_coefficients
    );
    assert_eq!(decoded_gainmap.alt_clli, gainmap.alt_clli);
    assert_eq!(decoded_gainmap.alt_plane_depth, 10);
    assert_eq!(decoded_gainmap.alt_plane_count, 3);
    assert_eq!(decoded_gainmap.alt_color_primaries, ColorPrimaries::Bt2020);
    assert_eq!(
        decoded_gainmap.alt_transfer_characteristics,
        TransferCharacteristics::Pq
    );
    assert_eq!(
        decoded_gainmap.alt_matrix_coefficients,
        MatrixCoefficients::Smpte2085
    );
    assert_eq!(decoded_gainmap.image.width, gainmap.image.width);
    assert_eq!(decoded_gainmap.image.height, gainmap.image.height);
    assert_eq!(decoded_gainmap.image.depth, gainmap.image.depth);
    assert_eq!(decoded_gainmap.metadata, gainmap.metadata);
    assert!(decoder.next_image().is_ok());
    Ok(())
}

#[test]
fn gainmap_base_image_hdr() -> AvifResult<()> {
    let (image, gainmap) = generate_gainmap_image(true)?;
    let settings = encoder::Settings {
        speed: Some(10),
        ..Default::default()
    };
    let mut encoder = encoder::Encoder::create_with_settings(&settings)?;
    encoder.add_image_gainmap(&image, &gainmap)?;
    let edata = encoder.finish()?;
    assert!(!edata.is_empty());

    let mut decoder = decoder::Decoder::default();
    decoder.set_io_vec(edata);
    decoder.settings.image_content_to_decode = ImageContentType::All;
    assert!(decoder.parse().is_ok());
    assert!(decoder.gainmap_present());
    let decoded_gainmap = decoder.gainmap();
    let decoded_image = decoder.image().expect("failed to get decoded image");
    assert_eq!(
        decoded_gainmap.image.matrix_coefficients,
        gainmap.image.matrix_coefficients
    );
    assert_eq!(decoded_image.clli, image.clli);
    assert_eq!(
        decoded_gainmap.alt_clli,
        ContentLightLevelInformation::default()
    );
    assert_eq!(decoded_gainmap.alt_plane_depth, 8);
    assert_eq!(decoded_gainmap.alt_plane_count, 3);
    assert_eq!(decoded_gainmap.alt_color_primaries, ColorPrimaries::Bt601);
    assert_eq!(
        decoded_gainmap.alt_transfer_characteristics,
        TransferCharacteristics::Srgb
    );
    assert_eq!(
        decoded_gainmap.alt_matrix_coefficients,
        MatrixCoefficients::Smpte2085
    );
    assert_eq!(decoded_gainmap.image.width, gainmap.image.width);
    assert_eq!(decoded_gainmap.image.height, gainmap.image.height);
    assert_eq!(decoded_gainmap.image.depth, gainmap.image.depth);
    assert_eq!(decoded_gainmap.metadata, gainmap.metadata);
    assert!(decoder.next_image().is_ok());
    Ok(())
}

#[test]
fn gainmap_oriented() -> AvifResult<()> {
    let (mut image, gainmap) = generate_gainmap_image(false)?;
    image.irot_angle = Some(1);
    image.imir_axis = Some(0);
    let settings = encoder::Settings {
        speed: Some(10),
        ..Default::default()
    };
    let mut encoder = encoder::Encoder::create_with_settings(&settings)?;
    encoder.add_image_gainmap(&image, &gainmap)?;
    let edata = encoder.finish()?;
    assert!(!edata.is_empty());

    let mut decoder = decoder::Decoder::default();
    decoder.set_io_vec(edata);
    decoder.settings.image_content_to_decode = ImageContentType::All;
    assert!(decoder.parse().is_ok());
    assert!(decoder.gainmap_present());
    let decoded_image = decoder.image().expect("failed to get decoded image");
    assert_eq!(decoded_image.irot_angle, image.irot_angle);
    assert_eq!(decoded_image.imir_axis, image.imir_axis);
    let decoded_gainmap = decoder.gainmap();
    assert!(decoded_gainmap.image.irot_angle.is_none());
    assert!(decoded_gainmap.image.imir_axis.is_none());
    Ok(())
}

#[test_case::test_matrix([0, 1, 2])]
fn gainmap_oriented_invalid(transformation_index: u8) -> AvifResult<()> {
    let (image, mut gainmap) = generate_gainmap_image(false)?;
    // Gainmap image should not have a transformative property. Expect a failure.
    match transformation_index {
        0 => gainmap.image.irot_angle = Some(1),
        1 => gainmap.image.imir_axis = Some(0),
        2 => gainmap.image.pasp = Some(PixelAspectRatio::default()),
        _ => {} // not reached.
    }
    let settings = encoder::Settings {
        speed: Some(10),
        ..Default::default()
    };
    let mut encoder = encoder::Encoder::create_with_settings(&settings)?;
    encoder.add_image_gainmap(&image, &gainmap)?;
    assert!(encoder.finish().is_err());
    Ok(())
}

#[test]
fn gainmap_all_channels_identical() -> AvifResult<()> {
    let (image, mut gainmap) = generate_gainmap_image(true)?;
    for c in 0..3 {
        gainmap.metadata.base_offset[c] = Fraction(1, 2);
        gainmap.metadata.alternate_offset[c] = Fraction(3, 4);
        gainmap.metadata.gamma[c] = UFraction(5, 6);
        gainmap.metadata.min[c] = Fraction(7, 8);
        gainmap.metadata.max[c] = Fraction(9, 10);
    }
    let settings = encoder::Settings {
        speed: Some(10),
        ..Default::default()
    };
    let mut encoder = encoder::Encoder::create_with_settings(&settings)?;
    encoder.add_image_gainmap(&image, &gainmap)?;
    let edata = encoder.finish()?;
    assert!(!edata.is_empty());

    let mut decoder = decoder::Decoder::default();
    decoder.set_io_vec(edata);
    decoder.settings.image_content_to_decode = ImageContentType::All;
    assert!(decoder.parse().is_ok());
    assert!(decoder.gainmap_present());
    let decoded_gainmap = decoder.gainmap();
    assert_eq!(decoded_gainmap.metadata, gainmap.metadata);
    Ok(())
}

#[test]
fn gainmap_grid() -> AvifResult<()> {
    let grid_columns = 2;
    let grid_rows = 2;
    let cell_width = 128;
    let cell_height = 200;
    let mut cells = Vec::new();
    for _ in 0..grid_rows * grid_columns {
        let mut image = generate_gradient_image(
            cell_width,
            cell_height,
            10,
            PixelFormat::Yuv444,
            YuvRange::Full,
            false,
        )?;
        image.transfer_characteristics = TransferCharacteristics::Pq;
        let gainmap = GainMap {
            image: generate_gradient_image(
                cell_width / 2,
                cell_height / 2,
                8,
                PixelFormat::Yuv420,
                YuvRange::Full,
                false,
            )?,
            metadata: gainmap_metadata(true),
            ..Default::default()
        };
        cells.push((image, gainmap));
    }
    let mut images = Vec::new();
    let mut gainmaps = Vec::new();
    for cell in &cells {
        images.push(&cell.0);
        gainmaps.push(&cell.1);
    }

    let settings = encoder::Settings {
        speed: Some(10),
        ..Default::default()
    };
    let mut encoder = encoder::Encoder::create_with_settings(&settings)?;
    encoder.add_image_gainmap_grid(grid_columns, grid_rows, &images, &gainmaps)?;
    let edata = encoder.finish()?;
    assert!(!edata.is_empty());

    let mut decoder = decoder::Decoder::default();
    decoder.set_io_vec(edata);
    decoder.settings.image_content_to_decode = ImageContentType::All;
    assert!(decoder.parse().is_ok());
    assert!(decoder.gainmap_present());
    assert!(decoder.next_image().is_ok());
    Ok(())
}

#[test_case::test_matrix([0, 1, 2, 3, 4])]
fn invalid_grid(test_case_index: u8) -> AvifResult<()> {
    let grid_columns = 2;
    let grid_rows = 2;
    let cell_width = 128;
    let cell_height = 200;
    let mut cells = Vec::new();
    for _ in 0..grid_rows * grid_columns {
        let mut image = generate_gradient_image(
            cell_width,
            cell_height,
            10,
            PixelFormat::Yuv444,
            YuvRange::Full,
            false,
        )?;
        image.transfer_characteristics = TransferCharacteristics::Pq;
        let gainmap = GainMap {
            image: generate_gradient_image(
                cell_width / 2,
                cell_height / 2,
                8,
                PixelFormat::Yuv420,
                YuvRange::Full,
                false,
            )?,
            metadata: gainmap_metadata(true),
            ..Default::default()
        };
        cells.push((image, gainmap));
    }

    let settings = encoder::Settings {
        speed: Some(10),
        ..Default::default()
    };
    let mut encoder = encoder::Encoder::create_with_settings(&settings)?;

    match test_case_index {
        0 => {
            // Invalid: one gainmap cell has the wrong size.
            cells[1].1.image.height = 90;
        }
        1 => {
            // Invalid: one gainmap cell has a different depth.
            cells[1].1.image.depth = 12;
        }
        2 => {
            // Invalid: one gainmap cell has different gainmap metadata.
            cells[1].1.metadata.gamma[0] = UFraction(42, 1);
        }
        3 => {
            // Invalid: one image cell has the wrong size.
            cells[1].0.height = 90;
        }
        4 => {
            // Invalid: one gainmap cell has a different depth.
            cells[1].0.depth = 12;
        }
        _ => unreachable!(),
    }
    let images: Vec<_> = cells.iter().map(|x| &x.0).collect();
    let gainmaps: Vec<_> = cells.iter().map(|x| &x.1).collect();
    assert!(encoder
        .add_image_gainmap_grid(grid_columns, grid_rows, &images, &gainmaps)
        .is_err());
    Ok(())
}
