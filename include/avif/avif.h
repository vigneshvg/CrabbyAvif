#ifndef AVIF_H
#define AVIF_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#define CRABBYAVIF_AVIF_DEFAULT_IMAGE_SIZE_LIMIT (16384 * 16384)

#define CRABBYAVIF_AVIF_DEFAULT_IMAGE_DIMENSION_LIMIT 32768

#define CRABBYAVIF_AVIF_DEFAULT_IMAGE_COUNT_LIMIT ((12 * 3600) * 60)

#define CRABBYAVIF_AVIF_MAX_AV1_LAYER_COUNT 4

#define CRABBYAVIF_AVIF_TRUE 1

#define CRABBYAVIF_AVIF_FALSE 0

#define CRABBYAVIF_AVIF_STRICT_DISABLED 0

#define CRABBYAVIF_AVIF_STRICT_PIXI_REQUIRED (1 << 0)

#define CRABBYAVIF_AVIF_STRICT_CLAP_VALID (1 << 1)

#define CRABBYAVIF_AVIF_STRICT_ALPHA_ISPE_REQUIRED (1 << 2)

#define CRABBYAVIF_AVIF_STRICT_ENABLED ((CRABBYAVIF_AVIF_STRICT_PIXI_REQUIRED | CRABBYAVIF_AVIF_STRICT_CLAP_VALID) | CRABBYAVIF_AVIF_STRICT_ALPHA_ISPE_REQUIRED)

#define CRABBYAVIF_AVIF_DIAGNOSTICS_ERROR_BUFFER_SIZE 256

#define CRABBYAVIF_AVIF_PLANE_COUNT_YUV 3

#define CRABBYAVIF_AVIF_REPETITION_COUNT_INFINITE -1

#define CRABBYAVIF_AVIF_REPETITION_COUNT_UNKNOWN -2

#define CRABBYAVIF_AVIF_TRANSFORM_NONE 0

#define CRABBYAVIF_AVIF_TRANSFORM_PASP (1 << 0)

#define CRABBYAVIF_AVIF_TRANSFORM_CLAP (1 << 1)

#define CRABBYAVIF_AVIF_TRANSFORM_IROT (1 << 2)

#define CRABBYAVIF_AVIF_TRANSFORM_IMIR (1 << 3)

#define CRABBYAVIF_AVIF_COLOR_PRIMARIES_BT709 1

#define CRABBYAVIF_AVIF_COLOR_PRIMARIES_IEC61966_2_4 1

#define CRABBYAVIF_AVIF_COLOR_PRIMARIES_BT2100 9

#define CRABBYAVIF_AVIF_COLOR_PRIMARIES_DCI_P3 12

#define CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_SMPTE2084 16

typedef enum CRABBYAVIF_avifChromaDownsampling {
    CRABBYAVIF_AVIF_CHROMA_DOWNSAMPLING_AUTOMATIC,
    CRABBYAVIF_AVIF_CHROMA_DOWNSAMPLING_FASTEST,
    CRABBYAVIF_AVIF_CHROMA_DOWNSAMPLING_BEST_QUALITY,
    CRABBYAVIF_AVIF_CHROMA_DOWNSAMPLING_AVERAGE,
    CRABBYAVIF_AVIF_CHROMA_DOWNSAMPLING_SHARP_YUV,
} CRABBYAVIF_avifChromaDownsampling;

typedef enum CRABBYAVIF_avifChromaSamplePosition {
    CRABBYAVIF_AVIF_CHROMA_SAMPLE_POSITION_UNKNOWN = 0,
    CRABBYAVIF_AVIF_CHROMA_SAMPLE_POSITION_VERTICAL = 1,
    CRABBYAVIF_AVIF_CHROMA_SAMPLE_POSITION_COLOCATED = 2,
} CRABBYAVIF_avifChromaSamplePosition;

typedef enum CRABBYAVIF_avifChromaUpsampling {
    CRABBYAVIF_AVIF_CHROMA_UPSAMPLING_AUTOMATIC,
    CRABBYAVIF_AVIF_CHROMA_UPSAMPLING_FASTEST,
    CRABBYAVIF_AVIF_CHROMA_UPSAMPLING_BEST_QUALITY,
    CRABBYAVIF_AVIF_CHROMA_UPSAMPLING_NEAREST,
    CRABBYAVIF_AVIF_CHROMA_UPSAMPLING_BILINEAR,
} CRABBYAVIF_avifChromaUpsampling;

enum CRABBYAVIF_avifColorPrimaries
#ifdef __cplusplus
  : uint16_t
#endif // __cplusplus
 {
    CRABBYAVIF_AVIF_COLOR_PRIMARIES_UNKNOWN = 0,
    CRABBYAVIF_AVIF_COLOR_PRIMARIES_SRGB = 1,
    CRABBYAVIF_AVIF_COLOR_PRIMARIES_UNSPECIFIED = 2,
    CRABBYAVIF_AVIF_COLOR_PRIMARIES_BT470M = 4,
    CRABBYAVIF_AVIF_COLOR_PRIMARIES_BT470BG = 5,
    CRABBYAVIF_AVIF_COLOR_PRIMARIES_BT601 = 6,
    CRABBYAVIF_AVIF_COLOR_PRIMARIES_SMPTE240 = 7,
    CRABBYAVIF_AVIF_COLOR_PRIMARIES_GENERIC_FILM = 8,
    CRABBYAVIF_AVIF_COLOR_PRIMARIES_BT2020 = 9,
    CRABBYAVIF_AVIF_COLOR_PRIMARIES_XYZ = 10,
    CRABBYAVIF_AVIF_COLOR_PRIMARIES_SMPTE431 = 11,
    CRABBYAVIF_AVIF_COLOR_PRIMARIES_SMPTE432 = 12,
    CRABBYAVIF_AVIF_COLOR_PRIMARIES_EBU3213 = 22,
};
#ifndef __cplusplus
typedef uint16_t CRABBYAVIF_avifColorPrimaries;
#endif // __cplusplus

typedef enum CRABBYAVIF_avifRGBFormat {
    CRABBYAVIF_AVIF_RGB_FORMAT_RGB,
    CRABBYAVIF_AVIF_RGB_FORMAT_RGBA,
    CRABBYAVIF_AVIF_RGB_FORMAT_ARGB,
    CRABBYAVIF_AVIF_RGB_FORMAT_BGR,
    CRABBYAVIF_AVIF_RGB_FORMAT_BGRA,
    CRABBYAVIF_AVIF_RGB_FORMAT_ABGR,
    CRABBYAVIF_AVIF_RGB_FORMAT_RGB565,
} CRABBYAVIF_avifRGBFormat;

enum CRABBYAVIF_avifMatrixCoefficients
#ifdef __cplusplus
  : uint16_t
#endif // __cplusplus
 {
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_IDENTITY = 0,
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_BT709 = 1,
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_UNSPECIFIED = 2,
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_FCC = 4,
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_BT470BG = 5,
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_BT601 = 6,
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_SMPTE240 = 7,
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_YCGCO = 8,
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_BT2020_NCL = 9,
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_BT2020_CL = 10,
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_SMPTE2085 = 11,
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_CHROMA_DERIVED_NCL = 12,
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_CHROMA_DERIVED_CL = 13,
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_ICTCP = 14,
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_YCGCO_RE = 15,
    CRABBYAVIF_AVIF_MATRIX_COEFFICIENTS_YCGCO_RO = 16,
};
#ifndef __cplusplus
typedef uint16_t CRABBYAVIF_avifMatrixCoefficients;
#endif // __cplusplus

typedef enum CRABBYAVIF_avifProgressiveState {
    CRABBYAVIF_AVIF_PROGRESSIVE_STATE_UNAVAILABLE = 0,
    CRABBYAVIF_AVIF_PROGRESSIVE_STATE_AVAILABLE = 1,
    CRABBYAVIF_AVIF_PROGRESSIVE_STATE_ACTIVE = 2,
} CRABBYAVIF_avifProgressiveState;

typedef enum CRABBYAVIF_avifDecoderSource {
    CRABBYAVIF_AVIF_DECODER_SOURCE_AUTO = 0,
    CRABBYAVIF_AVIF_DECODER_SOURCE_PRIMARY_ITEM = 1,
    CRABBYAVIF_AVIF_DECODER_SOURCE_TRACKS = 2,
} CRABBYAVIF_avifDecoderSource;

enum CRABBYAVIF_avifTransferCharacteristics
#ifdef __cplusplus
  : uint16_t
#endif // __cplusplus
 {
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_UNKNOWN = 0,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_BT709 = 1,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_UNSPECIFIED = 2,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_BT470M = 4,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_BT470BG = 5,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_BT601 = 6,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_SMPTE240 = 7,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_LINEAR = 8,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_LOG100 = 9,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_LOG100_SQRT10 = 10,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_IEC61966 = 11,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_BT1361 = 12,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_SRGB = 13,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_BT2020_10BIT = 14,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_BT2020_12BIT = 15,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_PQ = 16,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_SMPTE428 = 17,
    CRABBYAVIF_AVIF_TRANSFER_CHARACTERISTICS_HLG = 18,
};
#ifndef __cplusplus
typedef uint16_t CRABBYAVIF_avifTransferCharacteristics;
#endif // __cplusplus

typedef enum CRABBYAVIF_avifChannelIndex {
    AVIF_CHAN_Y = 0,
    AVIF_CHAN_U = 1,
    AVIF_CHAN_V = 2,
    AVIF_CHAN_A = 3,
} CRABBYAVIF_avifChannelIndex;

typedef enum CRABBYAVIF_avifCodecChoice {
    CRABBYAVIF_AVIF_CODEC_CHOICE_AUTO = 0,
    CRABBYAVIF_AVIF_CODEC_CHOICE_AOM = 1,
    CRABBYAVIF_AVIF_CODEC_CHOICE_DAV1D = 2,
    CRABBYAVIF_AVIF_CODEC_CHOICE_LIBGAV1 = 3,
    CRABBYAVIF_AVIF_CODEC_CHOICE_RAV1E = 4,
    CRABBYAVIF_AVIF_CODEC_CHOICE_SVT = 5,
    CRABBYAVIF_AVIF_CODEC_CHOICE_AVM = 6,
} CRABBYAVIF_avifCodecChoice;

typedef enum CRABBYAVIF_avifCodecFlag {
    CRABBYAVIF_AVIF_CODEC_FLAG_CAN_DECODE = (1 << 0),
    CRABBYAVIF_AVIF_CODEC_FLAG_CAN_ENCODE = (1 << 1),
} CRABBYAVIF_avifCodecFlag;

typedef enum CRABBYAVIF_avifHeaderFormat {
    AVIF_HEADER_FULL,
    AVIF_HEADER_REDUCED,
} CRABBYAVIF_avifHeaderFormat;

typedef enum CRABBYAVIF_avifPixelFormat {
    CRABBYAVIF_AVIF_PIXEL_FORMAT_NONE,
    CRABBYAVIF_AVIF_PIXEL_FORMAT_YUV444,
    CRABBYAVIF_AVIF_PIXEL_FORMAT_YUV422,
    CRABBYAVIF_AVIF_PIXEL_FORMAT_YUV420,
    CRABBYAVIF_AVIF_PIXEL_FORMAT_YUV400,
    CRABBYAVIF_AVIF_PIXEL_FORMAT_COUNT,
} CRABBYAVIF_avifPixelFormat;

typedef enum CRABBYAVIF_avifPlanesFlag {
    AVIF_PLANES_YUV = (1 << 0),
    AVIF_PLANES_A = (1 << 1),
    AVIF_PLANES_ALL = 255,
} CRABBYAVIF_avifPlanesFlag;

typedef enum CRABBYAVIF_avifRange {
    CRABBYAVIF_AVIF_RANGE_LIMITED = 0,
    CRABBYAVIF_AVIF_RANGE_FULL = 1,
} CRABBYAVIF_avifRange;

typedef enum CRABBYAVIF_avifResult {
    CRABBYAVIF_AVIF_RESULT_OK = 0,
    CRABBYAVIF_AVIF_RESULT_UNKNOWN_ERROR = 1,
    CRABBYAVIF_AVIF_RESULT_INVALID_FTYP = 2,
    CRABBYAVIF_AVIF_RESULT_NO_CONTENT = 3,
    CRABBYAVIF_AVIF_RESULT_NO_YUV_FORMAT_SELECTED = 4,
    CRABBYAVIF_AVIF_RESULT_REFORMAT_FAILED = 5,
    CRABBYAVIF_AVIF_RESULT_UNSUPPORTED_DEPTH = 6,
    CRABBYAVIF_AVIF_RESULT_ENCODE_COLOR_FAILED = 7,
    CRABBYAVIF_AVIF_RESULT_ENCODE_ALPHA_FAILED = 8,
    CRABBYAVIF_AVIF_RESULT_BMFF_PARSE_FAILED = 9,
    CRABBYAVIF_AVIF_RESULT_MISSING_IMAGE_ITEM = 10,
    CRABBYAVIF_AVIF_RESULT_DECODE_COLOR_FAILED = 11,
    CRABBYAVIF_AVIF_RESULT_DECODE_ALPHA_FAILED = 12,
    CRABBYAVIF_AVIF_RESULT_COLOR_ALPHA_SIZE_MISMATCH = 13,
    CRABBYAVIF_AVIF_RESULT_ISPE_SIZE_MISMATCH = 14,
    CRABBYAVIF_AVIF_RESULT_NO_CODEC_AVAILABLE = 15,
    CRABBYAVIF_AVIF_RESULT_NO_IMAGES_REMAINING = 16,
    CRABBYAVIF_AVIF_RESULT_INVALID_EXIF_PAYLOAD = 17,
    CRABBYAVIF_AVIF_RESULT_INVALID_IMAGE_GRID = 18,
    CRABBYAVIF_AVIF_RESULT_INVALID_CODEC_SPECIFIC_OPTION = 19,
    CRABBYAVIF_AVIF_RESULT_TRUNCATED_DATA = 20,
    CRABBYAVIF_AVIF_RESULT_IO_NOT_SET = 21,
    CRABBYAVIF_AVIF_RESULT_IO_ERROR = 22,
    CRABBYAVIF_AVIF_RESULT_WAITING_ON_IO = 23,
    CRABBYAVIF_AVIF_RESULT_INVALID_ARGUMENT = 24,
    CRABBYAVIF_AVIF_RESULT_NOT_IMPLEMENTED = 25,
    CRABBYAVIF_AVIF_RESULT_OUT_OF_MEMORY = 26,
    CRABBYAVIF_AVIF_RESULT_CANNOT_CHANGE_SETTING = 27,
    CRABBYAVIF_AVIF_RESULT_INCOMPATIBLE_IMAGE = 28,
    CRABBYAVIF_AVIF_RESULT_ENCODE_GAIN_MAP_FAILED = 29,
    CRABBYAVIF_AVIF_RESULT_DECODE_GAIN_MAP_FAILED = 30,
    CRABBYAVIF_AVIF_RESULT_INVALID_TONE_MAPPED_IMAGE = 31,
} CRABBYAVIF_avifResult;

typedef struct CRABBYAVIF_Decoder CRABBYAVIF_Decoder;

typedef int CRABBYAVIF_avifBool;

typedef uint32_t CRABBYAVIF_avifStrictFlags;

typedef struct CRABBYAVIF_avifRWData {
    uint8_t *data;
    size_t size;
} CRABBYAVIF_avifRWData;

typedef struct CRABBYAVIF_ContentLightLevelInformation {
    uint16_t maxCLL;
    uint16_t maxPALL;
} CRABBYAVIF_ContentLightLevelInformation;

typedef struct CRABBYAVIF_ContentLightLevelInformation CRABBYAVIF_avifContentLightLevelInformationBox;

typedef uint32_t CRABBYAVIF_avifTransformFlags;

typedef struct CRABBYAVIF_PixelAspectRatio {
    uint32_t hSpacing;
    uint32_t vSpacing;
} CRABBYAVIF_PixelAspectRatio;

typedef struct CRABBYAVIF_PixelAspectRatio CRABBYAVIF_avifPixelAspectRatioBox;

typedef struct CRABBYAVIF_avifCleanApertureBox {
    uint32_t widthN;
    uint32_t widthD;
    uint32_t heightN;
    uint32_t heightD;
    uint32_t horizOffN;
    uint32_t horizOffD;
    uint32_t vertOffN;
    uint32_t vertOffD;
} CRABBYAVIF_avifCleanApertureBox;

typedef struct CRABBYAVIF_avifImageRotation {
    uint8_t angle;
} CRABBYAVIF_avifImageRotation;

typedef struct CRABBYAVIF_avifImageMirror {
    uint8_t axis;
} CRABBYAVIF_avifImageMirror;

typedef struct CRABBYAVIF_avifGainMapMetadata {
    int32_t gainMapMinN[3];
    uint32_t gainMapMinD[3];
    int32_t gainMapMaxN[3];
    uint32_t gainMapMaxD[3];
    uint32_t gainMapGammaN[3];
    uint32_t gainMapGammaD[3];
    int32_t baseOffsetN[3];
    uint32_t baseOffsetD[3];
    int32_t alternateOffsetN[3];
    uint32_t alternateOffsetD[3];
    uint32_t baseHdrHeadroomN;
    uint32_t baseHdrHeadroomD;
    uint32_t alternateHdrHeadroomN;
    uint32_t alternateHdrHeadroomD;
    CRABBYAVIF_avifBool backwardDirection;
    CRABBYAVIF_avifBool useBaseColorSpace;
} CRABBYAVIF_avifGainMapMetadata;

typedef struct CRABBYAVIF_avifGainMap {
    struct CRABBYAVIF_avifImage *image;
    struct CRABBYAVIF_avifGainMapMetadata metadata;
    struct CRABBYAVIF_avifRWData altICC;
    CRABBYAVIF_avifColorPrimaries altColorPrimaries;
    CRABBYAVIF_avifTransferCharacteristics altTransferCharacteristics;
    CRABBYAVIF_avifMatrixCoefficients altMatrixCoefficients;
    enum CRABBYAVIF_avifRange altYUVRange;
    uint32_t altDepth;
    uint32_t altPlaneCount;
    CRABBYAVIF_avifContentLightLevelInformationBox altCLLI;
} CRABBYAVIF_avifGainMap;

typedef struct CRABBYAVIF_avifImage {
    uint32_t width;
    uint32_t height;
    uint32_t depth;
    enum CRABBYAVIF_avifPixelFormat yuvFormat;
    enum CRABBYAVIF_avifRange yuvRange;
    enum CRABBYAVIF_avifChromaSamplePosition yuvChromaSamplePosition;
    uint8_t *yuvPlanes[CRABBYAVIF_AVIF_PLANE_COUNT_YUV];
    uint32_t yuvRowBytes[CRABBYAVIF_AVIF_PLANE_COUNT_YUV];
    CRABBYAVIF_avifBool imageOwnsYUVPlanes;
    uint8_t *alphaPlane;
    uint32_t alphaRowBytes;
    CRABBYAVIF_avifBool imageOwnsAlphaPlane;
    CRABBYAVIF_avifBool alphaPremultiplied;
    struct CRABBYAVIF_avifRWData icc;
    CRABBYAVIF_avifColorPrimaries colorPrimaries;
    CRABBYAVIF_avifTransferCharacteristics transferCharacteristics;
    CRABBYAVIF_avifMatrixCoefficients matrixCoefficients;
    CRABBYAVIF_avifContentLightLevelInformationBox clli;
    CRABBYAVIF_avifTransformFlags transformFlags;
    CRABBYAVIF_avifPixelAspectRatioBox pasp;
    struct CRABBYAVIF_avifCleanApertureBox clap;
    struct CRABBYAVIF_avifImageRotation irot;
    struct CRABBYAVIF_avifImageMirror imir;
    struct CRABBYAVIF_avifRWData exif;
    struct CRABBYAVIF_avifRWData xmp;
    struct CRABBYAVIF_avifGainMap *gainMap;
} CRABBYAVIF_avifImage;

typedef struct CRABBYAVIF_avifImageTiming {
    uint64_t timescale;
    double pts;
    uint64_t ptsInTimescales;
    double duration;
    uint64_t durationInTimescales;
} CRABBYAVIF_avifImageTiming;

typedef struct CRABBYAVIF_avifIOStats {
    size_t colorOBUSize;
    size_t alphaOBUSize;
} CRABBYAVIF_avifIOStats;

typedef struct CRABBYAVIF_avifDiagnostics {
    char error[CRABBYAVIF_AVIF_DIAGNOSTICS_ERROR_BUFFER_SIZE];
} CRABBYAVIF_avifDiagnostics;

typedef struct CRABBYAVIF_avifDecoderData {

} CRABBYAVIF_avifDecoderData;

typedef struct CRABBYAVIF_avifDecoder {
    enum CRABBYAVIF_avifCodecChoice codecChoice;
    int32_t maxThreads;
    enum CRABBYAVIF_avifDecoderSource requestedSource;
    CRABBYAVIF_avifBool allowProgressive;
    CRABBYAVIF_avifBool allowIncremental;
    CRABBYAVIF_avifBool ignoreExif;
    CRABBYAVIF_avifBool ignoreXMP;
    uint32_t imageSizeLimit;
    uint32_t imageDimensionLimit;
    uint32_t imageCountLimit;
    CRABBYAVIF_avifStrictFlags strictFlags;
    struct CRABBYAVIF_avifImage *image;
    int32_t imageIndex;
    int32_t imageCount;
    enum CRABBYAVIF_avifProgressiveState progressiveState;
    struct CRABBYAVIF_avifImageTiming imageTiming;
    uint64_t timescale;
    double duration;
    uint64_t durationInTimescales;
    int32_t repetitionCount;
    CRABBYAVIF_avifBool alphaPresent;
    struct CRABBYAVIF_avifIOStats ioStats;
    struct CRABBYAVIF_avifDiagnostics diag;
    struct CRABBYAVIF_avifDecoderData *data;
    CRABBYAVIF_avifBool gainMapPresent;
    CRABBYAVIF_avifBool enableDecodingGainMap;
    CRABBYAVIF_avifBool enableParsingGainMapMetadata;
    CRABBYAVIF_avifBool imageSequenceTrackPresent;
    struct CRABBYAVIF_Decoder *rust_decoder;
    struct CRABBYAVIF_avifImage image_object;
    struct CRABBYAVIF_avifGainMap gainmap_object;
    struct CRABBYAVIF_avifImage gainmap_image_object;
} CRABBYAVIF_avifDecoder;

typedef void (*CRABBYAVIF_avifIODestroyFunc)(struct CRABBYAVIF_avifIO *io);

typedef struct CRABBYAVIF_avifROData {
    const uint8_t *data;
    size_t size;
} CRABBYAVIF_avifROData;

typedef enum CRABBYAVIF_avifResult (*CRABBYAVIF_avifIOReadFunc)(struct CRABBYAVIF_avifIO *io,
                                                                uint32_t readFlags,
                                                                uint64_t offset,
                                                                size_t size,
                                                                struct CRABBYAVIF_avifROData *out);

typedef enum CRABBYAVIF_avifResult (*CRABBYAVIF_avifIOWriteFunc)(struct CRABBYAVIF_avifIO *io,
                                                                 uint32_t writeFlags,
                                                                 uint64_t offset,
                                                                 const uint8_t *data,
                                                                 size_t size);

typedef struct CRABBYAVIF_avifIO {
    CRABBYAVIF_avifIODestroyFunc destroy;
    CRABBYAVIF_avifIOReadFunc read;
    CRABBYAVIF_avifIOWriteFunc write;
    uint64_t sizeHint;
    CRABBYAVIF_avifBool persistent;
    void *data;
} CRABBYAVIF_avifIO;

typedef struct CRABBYAVIF_Extent {
    uint64_t offset;
    size_t size;
} CRABBYAVIF_Extent;

typedef struct CRABBYAVIF_Extent CRABBYAVIF_avifExtent;

typedef uint32_t CRABBYAVIF_avifPlanesFlags;

typedef struct CRABBYAVIF_CropRect {
    uint32_t x;
    uint32_t y;
    uint32_t width;
    uint32_t height;
} CRABBYAVIF_CropRect;

typedef struct CRABBYAVIF_CropRect CRABBYAVIF_avifCropRect;

typedef struct CRABBYAVIF_avifRGBImage {
    uint32_t width;
    uint32_t height;
    uint32_t depth;
    enum CRABBYAVIF_avifRGBFormat format;
    enum CRABBYAVIF_avifChromaUpsampling chromaUpsampling;
    enum CRABBYAVIF_avifChromaDownsampling chromaDownsampling;
    bool ignoreAlpha;
    bool alphaPremultiplied;
    bool isFloat;
    int32_t maxThreads;
    uint8_t *pixels;
    uint32_t rowBytes;
} CRABBYAVIF_avifRGBImage;

typedef struct CRABBYAVIF_avifPixelFormatInfo {
    CRABBYAVIF_avifBool monochrome;
    int chromaShiftX;
    int chromaShiftY;
} CRABBYAVIF_avifPixelFormatInfo;

typedef uint32_t CRABBYAVIF_avifCodecFlags;











#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

struct CRABBYAVIF_avifDecoder *CRABBYAVIF_avifDecoderCreate(void);

void CRABBYAVIF_avifDecoderSetIO(struct CRABBYAVIF_avifDecoder *decoder,
                                 struct CRABBYAVIF_avifIO *io);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifDecoderSetIOFile(struct CRABBYAVIF_avifDecoder *decoder,
                                                           const char *filename);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifDecoderSetIOMemory(struct CRABBYAVIF_avifDecoder *decoder,
                                                             const uint8_t *data,
                                                             size_t size);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifDecoderSetSource(struct CRABBYAVIF_avifDecoder *decoder,
                                                           enum CRABBYAVIF_avifDecoderSource source);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifDecoderParse(struct CRABBYAVIF_avifDecoder *decoder);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifDecoderNextImage(struct CRABBYAVIF_avifDecoder *decoder);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifDecoderNthImage(struct CRABBYAVIF_avifDecoder *decoder,
                                                          uint32_t frameIndex);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifDecoderNthImageTiming(const struct CRABBYAVIF_avifDecoder *decoder,
                                                                uint32_t frameIndex,
                                                                struct CRABBYAVIF_avifImageTiming *outTiming);

void CRABBYAVIF_avifDecoderDestroy(struct CRABBYAVIF_avifDecoder *decoder);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifDecoderRead(struct CRABBYAVIF_avifDecoder *decoder,
                                                      struct CRABBYAVIF_avifImage *image);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifDecoderReadMemory(struct CRABBYAVIF_avifDecoder *decoder,
                                                            struct CRABBYAVIF_avifImage *image,
                                                            const uint8_t *data,
                                                            size_t size);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifDecoderReadFile(struct CRABBYAVIF_avifDecoder *decoder,
                                                          struct CRABBYAVIF_avifImage *image,
                                                          const char *filename);

CRABBYAVIF_avifBool CRABBYAVIF_avifDecoderIsKeyframe(const struct CRABBYAVIF_avifDecoder *decoder,
                                                     uint32_t frameIndex);

uint32_t CRABBYAVIF_avifDecoderNearestKeyframe(const struct CRABBYAVIF_avifDecoder *decoder,
                                               uint32_t frameIndex);

uint32_t CRABBYAVIF_avifDecoderDecodedRowCount(const struct CRABBYAVIF_avifDecoder *decoder);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifDecoderNthImageMaxExtent(const struct CRABBYAVIF_avifDecoder *decoder,
                                                                   uint32_t frameIndex,
                                                                   CRABBYAVIF_avifExtent *outExtent);

CRABBYAVIF_avifBool CRABBYAVIF_avifPeekCompatibleFileType(const struct CRABBYAVIF_avifROData *input);

struct CRABBYAVIF_avifImage *CRABBYAVIF_avifImageCreateEmpty(void);

struct CRABBYAVIF_avifImage *CRABBYAVIF_avifImageCreate(uint32_t width,
                                                        uint32_t height,
                                                        uint32_t depth,
                                                        enum CRABBYAVIF_avifPixelFormat yuvFormat);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifImageAllocatePlanes(struct CRABBYAVIF_avifImage *image,
                                                              CRABBYAVIF_avifPlanesFlags planes);

void CRABBYAVIF_avifImageFreePlanes(struct CRABBYAVIF_avifImage *image,
                                    CRABBYAVIF_avifPlanesFlags planes);

void CRABBYAVIF_avifImageDestroy(struct CRABBYAVIF_avifImage *image);

CRABBYAVIF_avifBool CRABBYAVIF_avifImageUsesU16(const struct CRABBYAVIF_avifImage *image);

CRABBYAVIF_avifBool CRABBYAVIF_avifImageIsOpaque(const struct CRABBYAVIF_avifImage *image);

uint8_t *CRABBYAVIF_avifImagePlane(const struct CRABBYAVIF_avifImage *image, int channel);

uint32_t CRABBYAVIF_avifImagePlaneRowBytes(const struct CRABBYAVIF_avifImage *image, int channel);

uint32_t CRABBYAVIF_avifImagePlaneWidth(const struct CRABBYAVIF_avifImage *image, int channel);

uint32_t CRABBYAVIF_avifImagePlaneHeight(const struct CRABBYAVIF_avifImage *image, int channel);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifImageSetViewRect(struct CRABBYAVIF_avifImage *dstImage,
                                                           const struct CRABBYAVIF_avifImage *srcImage,
                                                           const CRABBYAVIF_avifCropRect *rect);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifRWDataRealloc(struct CRABBYAVIF_avifRWData *raw,
                                                        size_t newSize);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifRWDataSet(struct CRABBYAVIF_avifRWData *raw,
                                                    const uint8_t *data,
                                                    size_t size);

void avifRWDataFree(struct CRABBYAVIF_avifRWData *raw);

void cioDestroy(struct CRABBYAVIF_avifIO *_io);

enum CRABBYAVIF_avifResult cioRead(struct CRABBYAVIF_avifIO *io,
                                   uint32_t _readFlags,
                                   uint64_t offset,
                                   size_t size,
                                   struct CRABBYAVIF_avifROData *out);

enum CRABBYAVIF_avifResult cioWrite(struct CRABBYAVIF_avifIO *_io,
                                    uint32_t _writeFlags,
                                    uint64_t _offset,
                                    const uint8_t *_data,
                                    size_t _size);

struct CRABBYAVIF_avifIO *CRABBYAVIF_avifIOCreateMemoryReader(const uint8_t *data, size_t size);

struct CRABBYAVIF_avifIO *CRABBYAVIF_avifIOCreateFileReader(const char *filename);

void CRABBYAVIF_avifIODestroy(struct CRABBYAVIF_avifIO *io);

void CRABBYAVIF_avifRGBImageSetDefaults(struct CRABBYAVIF_avifRGBImage *rgb,
                                        const struct CRABBYAVIF_avifImage *image);

enum CRABBYAVIF_avifResult CRABBYAVIF_avifImageYUVToRGB(const struct CRABBYAVIF_avifImage *image,
                                                        struct CRABBYAVIF_avifRGBImage *rgb);

const char *CRABBYAVIF_avifResultToString(enum CRABBYAVIF_avifResult _res);

CRABBYAVIF_avifBool CRABBYAVIF_avifCropRectConvertCleanApertureBox(CRABBYAVIF_avifCropRect *cropRect,
                                                                   const struct CRABBYAVIF_avifCleanApertureBox *clap,
                                                                   uint32_t imageW,
                                                                   uint32_t imageH,
                                                                   enum CRABBYAVIF_avifPixelFormat yuvFormat,
                                                                   struct CRABBYAVIF_avifDiagnostics *_diag);

void CRABBYAVIF_avifGetPixelFormatInfo(enum CRABBYAVIF_avifPixelFormat format,
                                       struct CRABBYAVIF_avifPixelFormatInfo *info);

void CRABBYAVIF_avifDiagnosticsClearError(struct CRABBYAVIF_avifDiagnostics *diag);

void *CRABBYAVIF_avifAlloc(size_t size);

void CRABBYAVIF_avifFree(void *p);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* AVIF_H */
