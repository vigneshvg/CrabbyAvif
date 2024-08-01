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

use crate::codecs::Decoder;
use crate::decoder::Category;
use crate::image::Image;
use crate::image::YuvRange;
use crate::internal_utils::pixels::*;
use crate::internal_utils::*;
use crate::*;

use ndk_sys::bindings::*;

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use log::LevelFilter;
use android_logger::Config;

#[derive(Debug, Default)]
pub struct MediaCodec {
    codec: Option<*mut AMediaCodec>,
    format: Option<*mut AMediaFormat>,
    output_buffer_index: Option<usize>,
}

macro_rules! c_str {
    ($var: ident, $var_tmp:ident, $str:expr) => {
        let $var_tmp = CString::new($str).unwrap();
        let $var = $var_tmp.as_ptr();
    };
}

fn get_i32(format: *mut AMediaFormat, key: *const c_char) -> Option<i32> {
    let mut value: i32 = 0;
    match unsafe { AMediaFormat_getInt32(format, key, &mut value as *mut _) } {
        true => Some(value),
        false => None,
    }
}

fn get_i32_from_str(format: *mut AMediaFormat, key: &str) -> Option<i32> {
    c_str!(key_str, key_str_tmp, key);
    let mut value: i32 = 0;
    match unsafe { AMediaFormat_getInt32(format, key_str, &mut value as *mut _) } {
        true => Some(value),
        false => None,
    }
}

impl Decoder for MediaCodec {
    fn initialize(
        &mut self,
        _operating_point: u8,
        _all_layers: bool,
        width: u32,
        height: u32,
        csd: Option<&Vec<u8>>,
    ) -> AvifResult<()> {
        android_logger::init_once(
            Config::default().with_max_level(LevelFilter::Trace),
        );
        if csd.is_some() {
            println!("### csd len: {}", csd.unwrap().len());
        } else {
            println!("### no csd");
        }
        // Does not support operating point and all layers.
        if self.codec.is_some() {
            return Ok(()); // Already initialized.
        }
        //c_str!(codec_mime_type, codec_mime_type_tmp, "video/hevc");
        //let codec = unsafe { AMediaCodec_createDecoderByType(codec_mime_type) };
        //c_str!(codec_name, codec_name_tmp, "c2.android.av1.decoder");
        c_str!(codec_name, codec_name_tmp, "c2.android.hevc.decoder");
        let codec = unsafe { AMediaCodec_createCodecByName(codec_name) };
        if codec.is_null() {
            return Err(AvifError::NoCodecAvailable);
        }
        let format = unsafe { AMediaFormat_new() };
        if format.is_null() {
            return Err(AvifError::UnknownError("".into()));
        }
        unsafe {
            //c_str!(mime_type, mime_type_tmp, "video/av01");
            c_str!(mime_type, mime_type_tmp, "image/vnd.android.heic");
            AMediaFormat_setString(format, AMEDIAFORMAT_KEY_MIME, mime_type);
            AMediaFormat_setInt32(format, AMEDIAFORMAT_KEY_WIDTH, i32_from_u32(width)?);
            AMediaFormat_setInt32(format, AMEDIAFORMAT_KEY_HEIGHT, i32_from_u32(height)?);

            // https://developer.android.com/reference/android/media/MediaCodecInfo.CodecCapabilities#COLOR_FormatYUV420Flexible
            //AMediaFormat_setInt32(format, AMEDIAFORMAT_KEY_COLOR_FORMAT, 2135033992);
            //AMediaFormat_setInt32(format, AMEDIAFORMAT_KEY_COLOR_FORMAT, 19);

            if let Some(csd) = csd  {
                println!("### setting csd");
                AMediaFormat_setBuffer(
                    format,
                    AMEDIAFORMAT_KEY_CSD_0,
                    csd.as_ptr() as *const _,
                    csd.len(),
                );
            }
            c_str!(num_inp_buf, num_inp_buf_tmp, "android._num-input-buffers");
            AMediaFormat_setInt32(format, num_inp_buf, 1);
            c_str!(num_op_buf, num_op_buf_tmp, "android._num-output-buffers");
            AMediaFormat_setInt32(format, num_op_buf, 1);
            c_str!(level_buf, level_buf_tmp, "level");
            AMediaFormat_setInt32(format, level_buf, 256);
            c_str!(max_inp_size, max_inp_size_buf, "max-input-size");
            AMediaFormat_setInt32(format, max_inp_size, 1444800);

            // TODO: for 10-bit need to set format to 54 in order to get 10-bit
            // output. Or maybe it is possible to get RGB 1010102 itself?
            // int32_t COLOR_FormatYUVP010 = 54;
            // rgb 1010102 = 2130750114

            if AMediaCodec_configure(codec, format, ptr::null_mut(), ptr::null_mut(), 0)
                != media_status_t_AMEDIA_OK
            {
                return Err(AvifError::NoCodecAvailable);
            }
            if AMediaCodec_start(codec) != media_status_t_AMEDIA_OK {
                return Err(AvifError::NoCodecAvailable);
            }
            error!("### configure complete");
            AMediaFormat_delete(format);
        }
        self.codec = Some(codec);
        Ok(())
    }

    fn get_next_image(
        &mut self,
        av1_payload: &[u8],
        _spatial_id: u8,
        image: &mut Image,
        category: Category,
    ) -> AvifResult<()> {
        if self.codec.is_none() {
            self.initialize(0, true, 0, 0, None)?;
        }
        let codec = self.codec.unwrap();
        if self.output_buffer_index.is_some() {
            // Release any existing output buffer.
            unsafe {
                AMediaCodec_releaseOutputBuffer(codec, self.output_buffer_index.unwrap(), false);
            }
        }
        unsafe {
            let input_index = AMediaCodec_dequeueInputBuffer(codec, 0);
            if input_index >= 0 {
                let mut input_buffer_size: usize = 0;
                let input_buffer = AMediaCodec_getInputBuffer(
                    codec,
                    input_index as usize,
                    &mut input_buffer_size as *mut _,
                );
                if input_buffer.is_null() {
                    return Err(AvifError::UnknownError(format!(
                        "input buffer at index {input_index} was null"
                    )));
                }
                println!("### input buffer size: {input_buffer_size}");
                // TODO: Alternative is to create a slice from raw parts and use copy_from_slice.
                ptr::copy_nonoverlapping(av1_payload.as_ptr(), input_buffer, av1_payload.len());
                let sl = std::slice::from_raw_parts_mut(input_buffer, 4);
                sl[0] = 0;
                sl[1] = 0;
                sl[2] = 0;
                sl[3] = 1;
                let dd = std::slice::from_raw_parts(input_buffer, 10);
                //println!("### first {} bytes: {:#?}", dd.len(), dd);
                println!("### first {} bytes:", dd.len());
                for (i, v) in dd.iter().enumerate() {
                    println!("### {i}: {v}");
                }
                if AMediaCodec_queueInputBuffer(
                    codec,
                    usize_from_isize(input_index)?,
                    /*offset=*/ 0,
                    av1_payload.len(),
                    /*pts=*/ 0,
                    /*flags=*/ 0,
                ) != media_status_t_AMEDIA_OK
                {
                    return Err(AvifError::UnknownError("".into()));
                }
                println!("### queued input buffer of len: {}", av1_payload.len());
                error!("### queued input buffer of len: {}", av1_payload.len());
                // queue eos.
                let input_eos_index = AMediaCodec_dequeueInputBuffer(codec, 0);
                println!("### Input_index: {input_index} input_eos_index: {input_eos_index}");
                if true {
                    if input_eos_index >= 0{
                        if AMediaCodec_queueInputBuffer(
                            codec,
                            usize_from_isize(input_eos_index)?,
                            /*offset=*/ 0,
                            /*payload_length=*/0,
                            /*pts=*/ 0,
                            AMEDIACODEC_BUFFER_FLAG_END_OF_STREAM as u32,
                        ) != media_status_t_AMEDIA_OK
                        {
                            return Err(AvifError::UnknownError("".into()));
                        }
                        println!("### queued input eos");
                        error!("### queued input eos");
                    } else {
                        return Err(AvifError::UnknownError(format!(
                            "got input eos index < 0: {input_eos_index}"
                        )));
                    }
                }
            } else {
                return Err(AvifError::UnknownError(format!(
                    "got input index < 0: {input_index}"
                )));
            }
        }
        let mut buffer: Option<*mut u8> = None;
        let mut buffer_size: usize = 0;
        let mut retry_count = 0;
        let mut buffer_info = AMediaCodecBufferInfo::default();
        while retry_count < 100 {
            retry_count += 1;
            unsafe {
                let output_index =
                    AMediaCodec_dequeueOutputBuffer(codec, &mut buffer_info as *mut _, 10000);
                if output_index >= 0 {
                    let output_buffer = AMediaCodec_getOutputBuffer(
                        codec,
                        usize_from_isize(output_index)?,
                        &mut buffer_size as *mut _,
                    );
                    if output_buffer.is_null() {
                        return Err(AvifError::UnknownError("output buffer is null".into()));
                    }
                    buffer = Some(output_buffer);
                    self.output_buffer_index = Some(usize_from_isize(output_index)?);
                    break;
                } else if output_index == AMEDIACODEC_INFO_OUTPUT_BUFFERS_CHANGED as isize {
                    // TODO: what to do?
                    continue;
                } else if output_index == AMEDIACODEC_INFO_OUTPUT_FORMAT_CHANGED as isize {
                    let format = AMediaCodec_getOutputFormat(codec);
                    if format.is_null() {
                        return Err(AvifError::UnknownError("output format was null".into()));
                    }
                    self.format = Some(format);
                    continue;
                } else if output_index == AMEDIACODEC_INFO_TRY_AGAIN_LATER as isize {
                    continue;
                } else {
                    return Err(AvifError::UnknownError(format!(
                        "mediacodec dequeue_output_buffer failed: {output_index}"
                    )));
                }
            }
        }
        if buffer.is_none() {
            return Err(AvifError::UnknownError(
                "did not get buffer from mediacodec".into(),
            ));
        }
        if self.format.is_none() {
            return Err(AvifError::UnknownError("format is none".into()));
        }
        let buffer = buffer.unwrap();
        let format = self.format.unwrap();
        let width = get_i32(format, unsafe { AMEDIAFORMAT_KEY_WIDTH })
            .ok_or(AvifError::UnknownError("".into()))?;
        let height = get_i32(format, unsafe { AMEDIAFORMAT_KEY_HEIGHT })
            .ok_or(AvifError::UnknownError("".into()))?;
        let stride = get_i32(format, unsafe { AMEDIAFORMAT_KEY_STRIDE })
            .ok_or(AvifError::UnknownError("".into()))?;
        let color_format = get_i32(format, unsafe { AMEDIAFORMAT_KEY_COLOR_FORMAT })
            .ok_or(AvifError::UnknownError("".into()))?;
        // color-range is documented but the key variable is not exposed in the NDK:
        // https://developer.android.com/reference/android/media/MediaFormat#KEY_COLOR_RANGE
        let color_range = get_i32_from_str(format, "color-range").unwrap_or(2);
        match category {
            Category::Alpha => {
                // TODO: make sure alpha plane matches previous alpha plane.
                image.width = width as u32;
                image.height = height as u32;
                image.depth = 8; // TODO: 10?
                image.yuv_range = if color_range == 0 { YuvRange::Limited } else { YuvRange::Full };
                image.planes[3] = Some(Pixels::Pointer(buffer));
                image.row_bytes[3] = stride as u32;
            }
            _ => {
                image.width = width as u32;
                image.height = height as u32;
                image.depth = 8; // TODO: 10?
                let mut reverse_uv = true;
                image.yuv_format = match color_format {
                    // Android maps all AV1 8-bit images into yuv 420.
                    2135033992 => PixelFormat::Yuv420,
                    19 | 35 => {
                        reverse_uv = false;
                        PixelFormat::Yuv420
                    }
                    _ => {
                        return Err(AvifError::UnknownError(format!(
                            "unknown color format: {color_format}"
                        )));
                    }
                };
                image.yuv_range = if color_range == 0 { YuvRange::Limited } else { YuvRange::Full };
                image.chroma_sample_position = ChromaSamplePosition::Unknown;

                image.color_primaries = ColorPrimaries::Unspecified;
                image.transfer_characteristics = TransferCharacteristics::Unspecified;
                image.matrix_coefficients = MatrixCoefficients::Unspecified;

                image.planes[0] = Some(Pixels::Pointer(buffer));
                // TODO: u and v order must be inverted for color format 19.
                let u_plane_offset = isize_from_i32(stride * height)?;
                let u_index = if reverse_uv { 2 } else { 1 };
                image.planes[u_index] =
                    Some(Pixels::Pointer(unsafe { buffer.offset(u_plane_offset) }));
                let u_plane_size = isize_from_i32(((width + 1) / 2) * ((height + 1) / 2))?;
                let v_plane_offset = u_plane_offset + u_plane_size;
                let v_index = if reverse_uv { 1 } else { 2 };
                image.planes[v_index] =
                    Some(Pixels::Pointer(unsafe { buffer.offset(v_plane_offset) }));

                image.row_bytes[0] = stride as u32;
                image.row_bytes[1] = ((stride + 1) / 2) as u32;
                image.row_bytes[2] = ((stride + 1) / 2) as u32;
            }
        }
        // TODO: gainmap category.
        Ok(())
    }
}

impl Drop for MediaCodec {
    fn drop(&mut self) {
        if self.codec.is_some() {
            if self.output_buffer_index.is_some() {
                unsafe {
                    AMediaCodec_releaseOutputBuffer(
                        self.codec.unwrap(),
                        self.output_buffer_index.unwrap(),
                        false,
                    );
                }
                self.output_buffer_index = None;
            }
            unsafe {
                AMediaCodec_stop(self.codec.unwrap());
                AMediaCodec_delete(self.codec.unwrap());
            }
            self.codec = None;
        }
        if self.format.is_some() {
            unsafe { AMediaFormat_delete(self.format.unwrap()) };
            self.format = None;
        }
    }
}
