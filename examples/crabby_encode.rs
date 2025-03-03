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

use clap::value_parser;
use clap::Parser;

use crabby_avif::encoder::*;
use crabby_avif::image::*;
use crabby_avif::utils::clap::CleanAperture;
use crabby_avif::utils::UFraction;
use crabby_avif::*;

mod writer;

use writer::y4m::Y4MReader;

use std::fs::File;
use std::io::Write;

macro_rules! split_and_check_count {
    ($parameter: literal, $input:ident, $delimiter:literal, $count:literal, $type:ty) => {{
        let values: Result<Vec<_>, _> = $input
            .split($delimiter)
            .map(|x| x.parse::<$type>())
            .collect();
        if values.is_err() {
            return Err(format!("Invalid {} string", $parameter));
        }
        let values = values.unwrap();
        if values.len() != $count {
            return Err(format!(
                "Invalid {} string. Expecting exactly {} values separated with a \"{}\"",
                $parameter, $count, $delimiter
            ));
        }
        values
    }};
}

fn clap_parser(s: &str) -> Result<CleanAperture, String> {
    let values = split_and_check_count!("clap", s, ",", 8, i32);
    let values: Vec<_> = values.into_iter().map(|x| x as u32).collect();
    Ok(CleanAperture {
        width: UFraction(values[0], values[1]),
        height: UFraction(values[2], values[3]),
        horiz_off: UFraction(values[4], values[5]),
        vert_off: UFraction(values[6], values[7]),
    })
}

fn clli_parser(s: &str) -> Result<ContentLightLevelInformation, String> {
    let values = split_and_check_count!("clli", s, ",", 2, u16);
    Ok(ContentLightLevelInformation {
        max_cll: values[0],
        max_pall: values[1],
    })
}

fn pasp_parser(s: &str) -> Result<PixelAspectRatio, String> {
    let values = split_and_check_count!("pasp", s, ",", 2, u32);
    Ok(PixelAspectRatio {
        h_spacing: values[0],
        v_spacing: values[1],
    })
}

fn cicp_parser(s: &str) -> Result<Nclx, String> {
    let values = split_and_check_count!("cicp", s, "/", 3, u16);
    Ok(Nclx {
        color_primaries: values[0].into(),
        transfer_characteristics: values[1].into(),
        matrix_coefficients: values[2].into(),
        ..Default::default()
    })
}

#[derive(Parser)]
struct CommandLineArgs {
    /// Disable strict decoding, which disables strict validation checks and errors
    #[arg(long, default_value = "false")]
    no_strict: bool,

    /// Decode all frames and display all image information instead of saving to disk
    #[arg(short = 'i', long, default_value = "false")]
    info: bool,

    #[arg(long, short = 'j')]
    jobs: Option<u32>,

    /// Quality for color in %d..%d where %d is lossless
    #[arg(long = "qcolor", short = 'q', value_parser = value_parser!(u8).range(0..=100))]
    quality: Option<u8>,

    /// Quality for color in %d..%d where %d is lossless
    #[arg(long, short = 's', value_parser = value_parser!(u8).range(0..=10), default_value = "6")]
    speed: u8,

    /// Add irot property (rotation), in 0..3. Makes (90 * ANGLE) degree rotation anti-clockwise
    #[arg(long = "irot", value_parser = value_parser!(u8).range(0..=3))]
    irot_angle: Option<u8>,

    /// Add imir property (mirroring). 0=top-to-bottom, 1=left-to-right
    #[arg(long = "imir", value_parser = value_parser!(u8).range(0..=1))]
    imir_axis: Option<u8>,

    /// Add clap property (clean aperture). Width, Height, HOffset, VOffset (in
    /// numerator/denominator pairs)
    #[arg(long, value_parser = clap_parser)]
    clap: Option<CleanAperture>,

    /// Add pasp property (aspect ratio). Horizontal spacing, Vertical spacing
    #[arg(long, value_parser = pasp_parser)]
    pasp: Option<PixelAspectRatio>,

    /// Add clli property (content light level information). MaxCLL, MaxPALL
    #[arg(long, value_parser = clli_parser)]
    clli: Option<ContentLightLevelInformation>,

    /// Set CICP values (nclx colr box) (P/T/M 3 raw numbers, use -r to set range flag)
    #[arg(long, value_parser = cicp_parser)]
    cicp: Option<Nclx>,

    /// Auto set parameters to encode a simple layered image supporting progressive rendering from
    /// a single input frame.
    #[arg(long, default_value = "false")]
    progressive: bool,

    /// Input file (y4m)
    #[arg(allow_hyphen_values = false)]
    input_file: String,

    /// Output AVIF file
    #[arg(allow_hyphen_values = false)]
    output_file: Option<String>,
}

/*
fn print_data_as_columns(rows: &[(usize, &str, String)]) {
    let rows: Vec<_> = rows
        .iter()
        .filter(|x| !x.1.is_empty())
        .map(|x| (format!("{} * {}", " ".repeat(x.0 * 4), x.1), x.2.as_str()))
        .collect();

    // Calculate the maximum width for the first column.
    let mut max_col1_width = 0;
    for (col1, _) in &rows {
        max_col1_width = max_col1_width.max(col1.len());
    }

    for (col1, col2) in &rows {
        println!("{col1:<max_col1_width$} : {col2}");
    }
}

fn print_vec(data: &[u8]) -> String {
    if data.is_empty() {
        format!("Absent")
    } else {
        format!("Present ({} bytes)", data.len())
    }
}

fn print_image_info(decoder: &Decoder) {
    let image = decoder.image().unwrap();
    let mut image_data = vec![
        (
            0,
            "File Format",
            format!("{:#?}", decoder.compression_format()),
        ),
        (0, "Resolution", format!("{}x{}", image.width, image.height)),
        (0, "Bit Depth", format!("{}", image.depth)),
        (0, "Format", format!("{:#?}", image.yuv_format)),
        if image.yuv_format == PixelFormat::Yuv420 {
            (
                0,
                "Chroma Sample Position",
                format!("{:#?}", image.chroma_sample_position),
            )
        } else {
            (0, "", "".into())
        },
        (
            0,
            "Alpha",
            format!(
                "{}",
                match (image.alpha_present, image.alpha_premultiplied) {
                    (true, true) => "Premultiplied",
                    (true, false) => "Not premultiplied",
                    (false, _) => "Absent",
                }
            ),
        ),
        (0, "Range", format!("{:#?}", image.yuv_range)),
        (
            0,
            "Color Primaries",
            format!("{:#?}", image.color_primaries),
        ),
        (
            0,
            "Transfer Characteristics",
            format!("{:#?}", image.transfer_characteristics),
        ),
        (
            0,
            "Matrix Coefficients",
            format!("{:#?}", image.matrix_coefficients),
        ),
        (0, "ICC Profile", print_vec(&image.icc)),
        (0, "XMP Metadata", print_vec(&image.xmp)),
        (0, "Exif Metadata", print_vec(&image.exif)),
    ];
    if image.pasp.is_none()
        && image.clap.is_none()
        && image.irot_angle.is_none()
        && image.imir_axis.is_none()
    {
        image_data.push((0, "Transformations", format!("None")));
    } else {
        image_data.push((0, "Transformations", format!("")));
        if let Some(pasp) = image.pasp {
            image_data.push((
                1,
                "pasp (Aspect Ratio)",
                format!("{}/{}", pasp.h_spacing, pasp.v_spacing),
            ));
        }
        if let Some(clap) = image.clap {
            image_data.push((1, "clap (Clean Aperture)", format!("")));
            image_data.push((2, "W", format!("{}/{}", clap.width.0, clap.width.1)));
            image_data.push((2, "H", format!("{}/{}", clap.height.0, clap.height.1)));
            image_data.push((
                2,
                "hOff",
                format!("{}/{}", clap.horiz_off.0, clap.horiz_off.1),
            ));
            image_data.push((
                2,
                "vOff",
                format!("{}/{}", clap.vert_off.0, clap.vert_off.1),
            ));
            match CropRect::create_from(&clap, image.width, image.height, image.yuv_format) {
                Ok(rect) => image_data.extend_from_slice(&[
                    (2, "Valid, derived crop rect", format!("")),
                    (3, "X", format!("{}", rect.x)),
                    (3, "Y", format!("{}", rect.y)),
                    (3, "W", format!("{}", rect.width)),
                    (3, "H", format!("{}", rect.height)),
                ]),
                Err(_) => image_data.push((2, "Invalid", format!(""))),
            }
        }
        if let Some(angle) = image.irot_angle {
            image_data.push((1, "irot (Rotation)", format!("{angle}")));
        }
        if let Some(axis) = image.imir_axis {
            image_data.push((1, "imir (Mirror)", format!("{axis}")));
        }
    }
    image_data.push((0, "Progressive", format!("{:#?}", image.progressive_state)));
    if let Some(clli) = image.clli {
        image_data.push((0, "CLLI", format!("{}, {}", clli.max_cll, clli.max_pall)));
    }
    if decoder.gainmap_present() {
        let gainmap = decoder.gainmap();
        let gainmap_image = &gainmap.image;
        image_data.extend_from_slice(&[
            (
                0,
                "Gainmap",
                format!(
                "{}x{} pixels, {} bit, {:#?}, {:#?} Range, Matrix Coeffs. {:#?}, Base Image is {}",
                gainmap_image.width,
                gainmap_image.height,
                gainmap_image.depth,
                gainmap_image.yuv_format,
                gainmap_image.yuv_range,
                gainmap_image.matrix_coefficients,
                if gainmap.metadata.base_hdr_headroom.0 == 0 { "SDR" } else { "HDR" },
            ),
            ),
            (0, "Alternate image", format!("")),
            (
                1,
                "Color Primaries",
                format!("{:#?}", gainmap.alt_color_primaries),
            ),
            (
                1,
                "Transfer Characteristics",
                format!("{:#?}", gainmap.alt_transfer_characteristics),
            ),
            (
                1,
                "Matrix Coefficients",
                format!("{:#?}", gainmap.alt_matrix_coefficients),
            ),
            (1, "ICC Profile", print_vec(&gainmap.alt_icc)),
            (1, "Bit Depth", format!("{}", gainmap.alt_plane_depth)),
            (1, "Planes", format!("{}", gainmap.alt_plane_count)),
            if let Some(clli) = gainmap_image.clli {
                (1, "CLLI", format!("{}, {}", clli.max_cll, clli.max_pall))
            } else {
                (1, "", "".into())
            },
        ])
    } else {
        // TODO: b/394162563 - check if we need to report the present but ignored case.
        image_data.push((0, "Gainmap", format!("Absent")));
    }
    if image.image_sequence_track_present {
        image_data.push((
            0,
            "Repeat Count",
            match decoder.repetition_count() {
                RepetitionCount::Finite(x) => format!("{x}"),
                RepetitionCount::Infinite => format!("Infinite"),
                RepetitionCount::Unknown => format!("Unknown"),
            },
        ));
    }
    print_data_as_columns(&image_data);
}
*/

fn max_threads(jobs: &Option<u32>) -> u32 {
    match jobs {
        Some(x) => {
            if *x == 0 {
                match std::thread::available_parallelism() {
                    Ok(value) => value.get() as u32,
                    Err(_) => 1,
                }
            } else {
                *x
            }
        }
        None => 1,
    }
}

/*
#[allow(unused)]
fn read_yuv420p(filepath: &Path, width: u32, height: u32) -> AvifResult<image::Image> {
    let mut reader =
        BufReader::new(File::open(filepath).or(Err(AvifError::UnknownError("".into())))?);
    let y_size = width * height;
    let uv_size = ((width + 1) / 2) * ((height + 1) / 2);
    let mut image = image::Image {
        width,
        height,
        depth: 8,
        yuv_format: PixelFormat::Yuv420,
        yuv_range: YuvRange::Limited,
        ..Default::default()
    };
    let category = Category::Color;
    image.allocate_planes(category)?;
    for plane in category.planes() {
        let plane_slice = image.slice_mut(*plane)?;
        reader
            .read_exact(plane_slice)
            .or(Err(AvifError::UnknownError("".into())))?;
    }
    if false {
        let category = Category::Alpha;
        image.allocate_planes(category)?;
        for y in 0..image.height {
            let alpha_row = image.row_mut(Plane::A, y)?;
            for pixel in alpha_row {
                *pixel = std::cmp::min(y, 255) as u8;
            }
        }
    }
    Ok(image)
}
*/

#[allow(unused)]
fn main() {
    let args = CommandLineArgs::parse();
    let mut y4m = Y4MReader::create(&args.input_file).expect("failed to create y4m reader");
    let mut image = y4m.read_frame().expect("failed to read y4m frame");

    image.irot_angle = args.irot_angle;
    image.imir_axis = args.imir_axis;
    image.clap = args.clap;
    image.pasp = args.pasp;
    image.clli = args.clli;
    if let Some(nclx) = args.cicp {
        image.color_primaries = nclx.color_primaries;
        image.transfer_characteristics = nclx.transfer_characteristics;
        image.matrix_coefficients = nclx.matrix_coefficients;
    }

    let settings = Settings {
        extra_layer_count: if args.progressive { 1 } else { 0 },
        ..Default::default()
    };
    let mut encoder = Encoder::create_with_settings(&settings).expect("failed to create encoder");
    if y4m.has_more_frames() {
        if args.progressive {
            println!("Automatic progressive encoding can only have one input image.");
            return;
        }
        let mut frame_count = 0;
        loop {
            encoder
                .add_image_for_sequence(&image, 1000)
                .expect("add image failed");
            frame_count += 1;
            if !y4m.has_more_frames() {
                break;
            }
            image = y4m.read_frame().expect("failed to read y4m frame");
            if frame_count == 4 {
                break;
            }
        }
        println!("added {frame_count} frames");
    } else {
        if args.progressive {
            // Encode first layer.
            encoder
                .add_layered_image(&image)
                .expect("add image failed for first layer");
            // Encode second layer.
            encoder
                .add_layered_image(&image)
                .expect("add image failed for second layer");
        } else {
            encoder.add_image(&image).expect("add image failed");
        }
    }

    let edata = encoder.finish().expect("finish failed");
    println!("### encoded data final size: {}", edata.len());
    match args.output_file {
        Some(ref filepath) => {
            let mut file = File::create(&filepath).expect("file creation failed");
            file.write_all(&edata);
            println!("### write output to {filepath}");
        }
        None => println!("### no output file provided"),
    }
    println!("### has more frames: {}", y4m.has_more_frames());
    println!("### all done :)");
    /*
    {
        let width = 960;
        let height = 540;
        println!(
            "### encoding raw yuv({width}x{height}): {}",
            args.input_file
        );
        let mut encoder: encoder::Encoder = Default::default();
        let image_count = 20;
        if image_count > 1 {
            let mut images: Vec<Image> = Vec::new();
            for _ in 0..image_count {
                images.push(
                    read_yuv420p(Path::new(&args.input_file), width, height)
                        .expect("yuv reading failed"),
                );
                encoder
                    .add_image_for_sequence(images.last().unwrap(), 1000)
                    .expect("add image failed");
            }
        } else {
            let grid_rows = 1;
            let grid_columns = 1;
            if grid_rows * grid_columns > 1 {
                let mut images: Vec<Image> = Vec::new();
                for _ in 0..grid_rows * grid_columns {
                    images.push(
                        read_yuv420p(Path::new(&args.input_file), width, height)
                            .expect("yuv reading failed"),
                    );
                }
                let image_refs: Vec<&Image> = images.iter().collect();
                encoder
                    .add_image_grid(grid_columns, grid_rows, &image_refs)
                    .expect("add image failed");
            } else {
                let image = read_yuv420p(Path::new(&args.input_file), width, height)
                    .expect("yuv reading failed");
                encoder.add_image(&image).expect("add image failed");
            }
        }
        let edata = encoder.finish().expect("finish failed");
        println!("### encoded data final size: {}", edata.len());
        match args.output_file {
            Some(filepath) => {
                let mut file = File::create(&filepath).expect("file creation failed");
                file.write_all(&edata);
                println!("### write output to {filepath}");
            }
            None => println!("### no output file provided"),
        }
    }
    */
}
