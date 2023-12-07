use std::os::raw::c_char;
use std::os::raw::c_int;

use crate::*;

#[repr(C)]
#[derive(PartialEq)]
pub enum avifResult {
    Ok,
    UnknownError,
    InvalidFtyp,
    NoContent,
    NoYuvFormatSelected,
    ReformatFailed,
    UnsupportedDepth,
    EncodeColorFailed,
    EncodeAlphaFailed,
    BmffParseFailed,
    MissingImageItem,
    DecodeColorFailed,
    DecodeAlphaFailed,
    ColorAlphaSizeMismatch,
    IspeSizeMismatch,
    NoCodecAvailable,
    NoImagesRemaining,
    InvalidExifPayload,
    InvalidImageGrid,
    InvalidCodecSpecificOption,
    TruncatedData,
    IoNotSet,
    IoError,
    WaitingOnIo,
    InvalidArgument,
    NotImplemented,
    OutOfMemory,
    CannotChangeSetting,
    IncompatibleImage,
    EncodeGainMapFailed,
    DecodeGainMapFailed,
    InvalidToneMappedImage,
}

impl From<&AvifError> for avifResult {
    fn from(err: &AvifError) -> Self {
        match err {
            AvifError::Ok => avifResult::Ok,
            AvifError::UnknownError => avifResult::UnknownError,
            AvifError::InvalidFtyp => avifResult::InvalidFtyp,
            AvifError::NoContent => avifResult::NoContent,
            AvifError::NoYuvFormatSelected => avifResult::NoYuvFormatSelected,
            AvifError::ReformatFailed => avifResult::ReformatFailed,
            AvifError::UnsupportedDepth => avifResult::UnsupportedDepth,
            AvifError::EncodeColorFailed => avifResult::EncodeColorFailed,
            AvifError::EncodeAlphaFailed => avifResult::EncodeAlphaFailed,
            AvifError::BmffParseFailed => avifResult::BmffParseFailed,
            AvifError::MissingImageItem => avifResult::MissingImageItem,
            AvifError::DecodeColorFailed => avifResult::DecodeColorFailed,
            AvifError::DecodeAlphaFailed => avifResult::DecodeAlphaFailed,
            AvifError::ColorAlphaSizeMismatch => avifResult::ColorAlphaSizeMismatch,
            AvifError::IspeSizeMismatch => avifResult::IspeSizeMismatch,
            AvifError::NoCodecAvailable => avifResult::NoCodecAvailable,
            AvifError::NoImagesRemaining => avifResult::NoImagesRemaining,
            AvifError::InvalidExifPayload => avifResult::InvalidExifPayload,
            AvifError::InvalidImageGrid => avifResult::InvalidImageGrid,
            AvifError::InvalidCodecSpecificOption => avifResult::InvalidCodecSpecificOption,
            AvifError::TruncatedData => avifResult::TruncatedData,
            AvifError::IoNotSet => avifResult::IoNotSet,
            AvifError::IoError => avifResult::IoError,
            AvifError::WaitingOnIo => avifResult::WaitingOnIo,
            AvifError::InvalidArgument => avifResult::InvalidArgument,
            AvifError::NotImplemented => avifResult::NotImplemented,
            AvifError::OutOfMemory => avifResult::OutOfMemory,
            AvifError::CannotChangeSetting => avifResult::CannotChangeSetting,
            AvifError::IncompatibleImage => avifResult::IncompatibleImage,
            AvifError::EncodeGainMapFailed => avifResult::EncodeGainMapFailed,
            AvifError::DecodeGainMapFailed => avifResult::DecodeGainMapFailed,
            AvifError::InvalidToneMappedImage => avifResult::InvalidToneMappedImage,
        }
    }
}

pub type avifBool = c_int;
pub const AVIF_TRUE: c_int = 1;
pub const AVIF_FALSE: c_int = 0;

#[repr(C)]
#[derive(Debug)]
pub enum avifPixelFormat {
    None,
    Yuv444,
    Yuv422,
    Yuv420,
    Yuv400,
    Count,
}

impl From<PixelFormat> for avifPixelFormat {
    fn from(format: PixelFormat) -> Self {
        match format {
            PixelFormat::Yuv444 => Self::Yuv444,
            PixelFormat::Yuv422 => Self::Yuv422,
            PixelFormat::Yuv420 => Self::Yuv420,
            PixelFormat::Monochrome => Self::Yuv400,
        }
    }
}

impl avifPixelFormat {
    // TODO: these functions can be removed if avifPixelFormat can be aliased to PixelFormat (with
    // constants None and Count.
    pub fn is_monochrome(&self) -> bool {
        matches!(self, Self::Yuv400)
    }

    pub fn chroma_shift_x(&self) -> u32 {
        match self {
            Self::Yuv422 | Self::Yuv420 => 1,
            _ => 0,
        }
    }

    pub fn chroma_shift_y(&self) -> u32 {
        match self {
            Self::Yuv420 => 1,
            _ => 0,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub enum avifRange {
    Limited = 0,
    Full = 1,
}

impl From<bool> for avifRange {
    fn from(full_range: bool) -> Self {
        match full_range {
            true => Self::Full,
            false => Self::Limited,
        }
    }
}

pub const AVIF_STRICT_DISABLED: u32 = 0;
pub const AVIF_STRICT_PIXI_REQUIRED: u32 = 1 << 0;
pub const AVIF_STRICT_CLAP_VALID: u32 = 1 << 1;
pub const AVIF_STRICT_ALPHA_ISPE_REQUIRED: u32 = 1 << 2;
pub const AVIF_STRICT_ENABLED: u32 =
    AVIF_STRICT_PIXI_REQUIRED | AVIF_STRICT_CLAP_VALID | AVIF_STRICT_ALPHA_ISPE_REQUIRED;
pub type avifStrictFlags = u32;

#[repr(C)]
pub struct avifDecoderData {}

pub const AVIF_DIAGNOSTICS_ERROR_BUFFER_SIZE: usize = 256;
#[repr(C)]
pub struct avifDiagnostics {
    error: [c_char; AVIF_DIAGNOSTICS_ERROR_BUFFER_SIZE],
}

impl Default for avifDiagnostics {
    fn default() -> Self {
        Self {
            error: [0; AVIF_DIAGNOSTICS_ERROR_BUFFER_SIZE],
        }
    }
}

#[repr(C)]
pub enum avifCodecChoice {
    Auto = 0,
    Aom = 1,
    Dav1d = 2,
    Libgav1 = 3,
    Rav1e = 4,
    Svt = 5,
    Avm = 6,
}

pub fn to_avifBool(val: bool) -> avifBool {
    if val {
        AVIF_TRUE
    } else {
        AVIF_FALSE
    }
}

pub fn to_avifResult<T>(res: &AvifResult<T>) -> avifResult {
    match res {
        Ok(_) => avifResult::Ok,
        Err(err) => {
            let res: avifResult = err.into();
            res
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn avifResultToString(_res: avifResult) -> *const c_char {
    // TODO: implement this function.
    std::ptr::null()
}

// Constants and definitions from libavif that are not used in rust.

pub const AVIF_PLANE_COUNT_YUV: usize = 3;
pub const AVIF_REPETITION_COUNT_INFINITE: i32 = -1;
pub const AVIF_REPETITION_COUNT_UNKNOWN: i32 = -2;

/// cbindgen:rename-all=ScreamingSnakeCase
#[repr(C)]
pub enum avifPlanesFlag {
    AvifPlanesYuv = 1 << 0,
    AvifPlanesA = 1 << 1,
    AvifPlanesAll = 0xFF,
}
pub type avifPlanesFlags = u32;

/// cbindgen:rename-all=ScreamingSnakeCase
#[repr(C)]
pub enum avifChannelIndex {
    AvifChanY = 0,
    AvifChanU = 1,
    AvifChanV = 2,
    AvifChanA = 3,
}

/// cbindgen:rename-all=ScreamingSnakeCase
#[repr(C)]
pub enum avifHeaderFormat {
    AvifHeaderFull,
    AvifHeaderReduced,
}

#[repr(C)]
pub struct avifPixelFormatInfo {
    monochrome: avifBool,
    chromaShiftX: c_int,
    chromaShiftY: c_int,
}

#[no_mangle]
pub unsafe extern "C" fn avifGetPixelFormatInfo(
    format: avifPixelFormat,
    info: *mut avifPixelFormatInfo,
) {
    if info.is_null() {
        return;
    }
    let info = &mut (*info);
    match format {
        avifPixelFormat::Yuv444 => {
            info.chromaShiftX = 0;
            info.chromaShiftY = 0;
            info.monochrome = AVIF_FALSE;
        }
        avifPixelFormat::Yuv422 => {
            info.chromaShiftX = 1;
            info.chromaShiftY = 0;
            info.monochrome = AVIF_FALSE;
        }
        avifPixelFormat::Yuv420 => {
            info.chromaShiftX = 1;
            info.chromaShiftY = 1;
            info.monochrome = AVIF_FALSE;
        }
        avifPixelFormat::Yuv400 => {
            info.chromaShiftX = 1;
            info.chromaShiftY = 1;
            info.monochrome = AVIF_TRUE;
        }
        _ => {}
    }
}

#[no_mangle]
pub unsafe extern "C" fn avifDiagnosticsClearError(diag: *mut avifDiagnostics) {
    if diag.is_null() {
        return;
    }
    (*diag).error[0] = 0;
}

#[repr(C)]
pub enum avifCodecFlag {
    CanDecode = (1 << 0),
    CanEncode = (1 << 1),
}
pub type avifCodecFlags = u32;

// TODO: This can be moved into the rust layer and renamed.
#[repr(C)]
#[derive(Default)]
pub struct avifIOStats {
    colorOBUSize: usize,
    alphaOBUSize: usize,
}