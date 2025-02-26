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

#![allow(unused)]

use crate::codecs::Encoder;
use crate::encoder::Sample;
use crate::image::Image;
use crate::image::YuvRange;
use crate::internal_utils::pixels::*;
use crate::*;

use aom_sys::bindings::*;

use std::mem::MaybeUninit;

#[derive(Default)]
pub struct Aom {
    encoder: Option<aom_codec_ctx_t>,
    config: Option<aom_codec_enc_cfg>,
}

const AOM_CODEC_OK: u32 = 0;

impl Encoder for Aom {
    fn encode_image(
        &mut self,
        image: &Image,
        category: Category,
        tile_rows_log2: i32,
        tile_columns_log2: i32,
        quantizer: i32,
        disable_lagged_output: bool,
        is_single_image: bool,
        output_samples: &mut Vec<Sample>,
    ) -> AvifResult<()> {
        println!("### here 1");
        if self.encoder.is_none() {
            let encoder_iface = unsafe { aom_codec_av1_cx() };
            let aom_usage = AOM_USAGE_REALTIME;
            let mut cfg_uninit: MaybeUninit<aom_codec_enc_cfg> = MaybeUninit::uninit();
            let err = unsafe {
                aom_codec_enc_config_default(encoder_iface, cfg_uninit.as_mut_ptr(), aom_usage)
            };
            if err != aom_codec_err_t_AOM_CODEC_OK {
                return Err(AvifError::UnknownError("".into()));
            }
            self.config = Some(unsafe { cfg_uninit.assume_init() });
            let config = self.config.unwrap_mut();
            config.rc_end_usage = aom_rc_mode_AOM_CBR;
            // TODO: handle 444 here.
            config.g_profile = 0;
            config.g_bit_depth = image.depth as u32;
            config.g_w = image.width;
            config.g_h = image.height;

            if is_single_image {
                config.g_limit = 1;
                config.g_lag_in_frames = 0;
                config.kf_mode = aom_kf_mode_AOM_KF_DISABLED;
                config.kf_max_dist = 0;
            }
            config.rc_min_quantizer = quantizer as u32;
            config.rc_max_quantizer = quantizer as u32;
            config.monochrome = match category {
                Category::Color => 0,
                Category::Alpha => 1,
                _ => return Err(AvifError::NotImplemented),
            };

            let mut encoder_uninit: MaybeUninit<aom_codec_ctx_t> = MaybeUninit::uninit();
            let err = unsafe {
                aom_codec_enc_init_ver(
                    encoder_uninit.as_mut_ptr(),
                    encoder_iface,
                    self.config.unwrap_ref() as *const aom_codec_enc_cfg,
                    0,
                    AOM_ENCODER_ABI_VERSION as i32,
                )
            };
            if err != aom_codec_err_t_AOM_CODEC_OK {
                return Err(AvifError::UnknownError("".into()));
            }
            self.encoder = Some(unsafe { encoder_uninit.assume_init() });

            match category {
                Category::Alpha => unsafe {
                    aom_codec_control(
                        self.encoder.unwrap_mut() as *mut _,
                        aome_enc_control_id_AV1E_SET_COLOR_RANGE as _,
                        aom_color_range_AOM_CR_FULL_RANGE,
                    );
                },
                Category::Color => unsafe {
                    aom_codec_control(
                        self.encoder.unwrap_mut() as *mut _,
                        aome_enc_control_id_AV1E_SET_COLOR_PRIMARIES as _,
                        image.color_primaries,
                    );
                    aom_codec_control(
                        self.encoder.unwrap_mut() as *mut _,
                        aome_enc_control_id_AV1E_SET_TRANSFER_CHARACTERISTICS as _,
                        image.transfer_characteristics,
                    );
                    aom_codec_control(
                        self.encoder.unwrap_mut() as *mut _,
                        aome_enc_control_id_AV1E_SET_MATRIX_COEFFICIENTS as _,
                        image.matrix_coefficients,
                    );
                    aom_codec_control(
                        self.encoder.unwrap_mut() as *mut _,
                        aome_enc_control_id_AV1E_SET_COLOR_RANGE as _,
                        if image.yuv_range == YuvRange::Limited {
                            aom_color_range_AOM_CR_STUDIO_RANGE
                        } else {
                            aom_color_range_AOM_CR_FULL_RANGE
                        },
                    );
                },
                _ => todo!("not implemented"),
            }
        }
        println!("### here 2");
        let mut aom_image: aom_image_t = unsafe { std::mem::zeroed() };
        aom_image.fmt = match image.yuv_format {
            PixelFormat::Yuv444 => aom_img_fmt_AOM_IMG_FMT_I444,
            _ => aom_img_fmt_AOM_IMG_FMT_I420,
        };
        aom_image.bit_depth = if image.depth > 8 { 16 } else { 8 };
        aom_image.w = image.width;
        aom_image.h = image.height;
        aom_image.d_w = image.width;
        aom_image.d_h = image.height;
        aom_image.bps = 12;
        aom_image.x_chroma_shift = image.yuv_format.chroma_shift_x().0;
        aom_image.y_chroma_shift = image.yuv_format.chroma_shift_y();
        match category {
            Category::Color => {
                aom_image.range = image.yuv_range as u32;
                aom_image.monochrome = 0;
                for i in 0..3 {
                    aom_image.planes[i] = image.planes[i].unwrap_ref().ptr() as *mut u8;
                    aom_image.stride[i] = image.row_bytes[i] as i32;
                }
            }
            Category::Alpha => {
                aom_image.range = aom_color_range_AOM_CR_FULL_RANGE;
                aom_image.monochrome = 1;
                aom_image.x_chroma_shift = 1;
                aom_image.y_chroma_shift = 1;
                aom_image.planes[0] = image.planes[3].unwrap_ref().ptr() as *mut u8;
                aom_image.stride[0] = image.row_bytes[3] as i32;
            }
            _ => return Err(AvifError::NotImplemented),
        }
        println!("## aom range: {}", aom_image.range);
        aom_image.cp = image.color_primaries as u32;
        aom_image.tc = image.transfer_characteristics as u32;
        aom_image.mc = image.matrix_coefficients as u32;
        //let encode_flags = AOM_EFLAG_FORCE_KF as i64;
        let encode_flags = 0;
        let err = unsafe {
            aom_codec_encode(
                self.encoder.unwrap_mut() as *mut _,
                &aom_image as *const _,
                0,
                1,
                encode_flags,
            )
        };
        if err != aom_codec_err_t_AOM_CODEC_OK {
            return Err(AvifError::UnknownError(format!("err: {err}")));
        }
        println!("### im here 3");
        let mut iter: aom_codec_iter_t = std::ptr::null_mut();
        loop {
            let pkt = unsafe {
                aom_codec_get_cx_data(self.encoder.unwrap_mut() as *mut _, &mut iter as *mut _)
            };
            if pkt.is_null() {
                break;
            }
            let pkt = unsafe { *pkt };
            println!("### pkt.kind: {:#?}", pkt.kind);
            if pkt.kind == aom_codec_cx_pkt_kind_AOM_CODEC_CX_FRAME_PKT {
                unsafe {
                    let encoded_data = std::slice::from_raw_parts(
                        pkt.data.frame.buf as *const u8,
                        pkt.data.frame.sz,
                    );
                    let sync = (pkt.data.frame.flags & AOM_FRAME_IS_KEY) != 0;
                    println!("### pkt size: {} is_key: {sync}", encoded_data.len());
                    output_samples.push(Sample::create_from(encoded_data, sync)?);
                }
            }
        }
        if is_single_image {
            self.finish()?;
            unsafe {
                aom_codec_destroy(self.encoder.unwrap_mut() as *mut _);
            }
            self.encoder = None;
            println!("### destroyed");
        }
        Ok(())
    }

    fn finish(&mut self) -> AvifResult<()> {
        if self.encoder.is_none() {
            return Ok(());
        }
        // TODO: flush in a loop until gotPacket.
        let err = unsafe {
            aom_codec_encode(
                self.encoder.unwrap_mut() as *mut _,
                std::ptr::null(),
                0,
                1,
                0,
            )
        };
        if err != aom_codec_err_t_AOM_CODEC_OK {
            return Err(AvifError::UnknownError("".into()));
        }
        let mut iter: aom_codec_iter_t = std::ptr::null_mut();
        loop {
            let pkt = unsafe {
                aom_codec_get_cx_data(self.encoder.unwrap_mut() as *mut _, &mut iter as *mut _)
            };
            if pkt.is_null() {
                break;
            }
            let pkt = unsafe { *pkt };
            println!("### pkt.kind in flush: {:#?}", pkt.kind);
            if pkt.kind == aom_codec_cx_pkt_kind_AOM_CODEC_CX_FRAME_PKT {
                // TODO: Add sample to output.
                unsafe {
                    println!(
                        "### pkt size: {} is_key: {}",
                        pkt.data.frame.sz,
                        (pkt.data.frame.flags & AOM_FRAME_IS_KEY) != 0
                    );
                }
            }
        }
        Ok(())
    }
}

impl Drop for Aom {
    fn drop(&mut self) {
        println!("### in drop");
        if self.encoder.is_some() {
            println!("### destroying in drop");
            unsafe {
                aom_codec_destroy(self.encoder.unwrap_mut() as *mut _);
            }
        }
    }
}
