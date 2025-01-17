use std::fmt::Write;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use skyline::hooks::{getRegionAddress, Region};

use std::arch::aarch64::{uint8x16_t, vaddv_u8, vandq_u8, vceqq_u8, vdupq_n_u8, vget_high_u8, vget_low_u8, vld1q_s8, vld1q_u8, vshlq_u8};

const NEON_REGISTER_LENGTH: usize = 16;

static OFFSETS: Lazy<Offsets> = Lazy::new(|| {
    let path = crate::utils::paths::cache().join("offsets.toml");
    let offsets = match std::fs::read_to_string(&path) {
        Ok(string) => match toml::de::from_str(string.as_str()) {
            Ok(offsets) => Some(offsets),
            Err(err) => {
                error!("Unable to parse 'offsets.toml'. Reason: {:?}", err);
                Offsets::new()
            },
        },
        Err(err) => {
            error!("Unable to read 'offsets.toml'. Reason: {:?}", err);
            Offsets::new()
        },
    }
    .expect("unable to find subsequence");

    match toml::ser::to_string_pretty(&offsets) {
        Ok(string) => {
            if std::fs::write(path, string.as_bytes()).is_err() {
                error!("Unable to write 'offsets.toml'.")
            }
        },
        Err(_) => error!("Failed to serialize offsets."),
    }

    offsets
});

// Search Code: Tuple(ByteArray, Offset)

static FILESYSTEM_INFO_ADRP_SEARCH_CODE: (&[u8], isize) = (&[0xf3, 0x03, 0x00, 0xaa, 0x1f, 0x01, 0x09, 0x6b, 0xe0, 0x04, 0x00, 0x54], 12);

static RES_SERVICE_ADRP_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x04, 0x01, 0x49, 0xfa, 0x21, 0x05, 0x00, 0x54, 0x5f, 0x00, 0x00, 0xf9, 0x7f, 0x00, 0x00, 0xf9,
    ],
    0x10,
);

static LOOKUP_STREAM_HASH_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x29, 0x58, 0x40, 0xf9, 0x28, 0x60, 0x40, 0xf9, 0x2a, 0x05, 0x40, 0xb9, 0x09, 0x0d, 0x0a, 0x8b, 0xaa, 0x01, 0x00, 0x34, 0x5f, 0x01, 0x00,
        0xf1,
    ],
    0x0,
);

static TITLE_SCREEN_VERSION_SEARCH_CODE: (&[u8], isize) = (
    &[
        0xfc, 0x0f, 0x1d, 0xf8, 0xf4, 0x4f, 0x01, 0xa9, 0xfd, 0x7b, 0x02, 0xa9, 0xfd, 0x83, 0x00, 0x91, 0xff, 0x07, 0x40, 0xd1, 0xf4, 0x03, 0x01,
        0xaa, 0xf3, 0x03, 0x00, 0xaa,
    ],
    0x0,
);

static ESHOPMANAGER_SHOW_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x08, 0xe1, 0x43, 0xf9, 0x14, 0x05, 0x40, 0xf9, 0x88, 0x22, 0x44, 0x39, 0x08, 0x04, 0x00, 0x35,
    ],
    -0x10,
);

static INFLATE_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x4b, 0x00, 0x1b, 0x0b, 0x00, 0x01, 0x1f, 0xd6, 0x68, 0x6a, 0x40, 0xf9, 0x09, 0x3d, 0x40, 0xf9, 0x2c, 0x01, 0x40, 0xf9,
    ],
    0x0,
);

static MEMCPY_1_SEARCH_CODE: (&[u8], isize) = (
    &[
        0xf5, 0x1f, 0x40, 0xb9, 0xa7, 0x00, 0x00, 0x14, 0xe2, 0xa3, 0x00, 0x91, 0xe4, 0xc3, 0x00, 0x91,
    ],
    -0x4,
);

static MEMCPY_2_SEARCH_CODE: (&[u8], isize) = (
    &[
        0xf8, 0x1b, 0x40, 0xf9, 0x1f, 0x03, 0x15, 0xeb, 0xa2, 0x2a, 0x00, 0x54, 0x96, 0x03, 0x18, 0x8b, 0x68, 0x1a, 0x40, 0xf9,
    ],
    -0x4,
);

static MEMCPY_3_SEARCH_CODE: (&[u8], isize) = (
    &[
        0xe8, 0x03, 0x18, 0xaa, 0xf8, 0x1b, 0x40, 0xf9, 0xd6, 0x02, 0x18, 0x8b, 0xbf, 0x02, 0x18, 0xeb, 0x88, 0xfb, 0xff, 0x54,
    ],
    -0x4,
);

static INFLATE_DIR_FILE_SEARCH_CODE: (&[u8], isize) = (
    &[
        0xfc, 0x6f, 0xba, 0xa9, 0xfa, 0x67, 0x01, 0xa9, 0xf8, 0x5f, 0x02, 0xa9, 0xf6, 0x57, 0x03, 0xa9, 0xf4, 0x4f, 0x04, 0xa9, 0xfd, 0x7b, 0x05,
        0xa9, 0xfd, 0x43, 0x01, 0x91, 0xff, 0x03, 0x07, 0xd1, 0x4c, 0xb4, 0x40, 0xa9,
    ],
    0x0,
);

static INITIAL_LOADING_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x08, 0x3f, 0x40, 0xf9, 0x08, 0x01, 0x40, 0xf9, 0x08, 0x21, 0x40, 0xf9, 0x08, 0x3d, 0x40, 0xb9, 0x08, 0x5d, 0x00, 0x12,
    ],
    0x0,
);

static RES_LOAD_LOOP_START_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x2a, 0x05, 0x09, 0x8b, 0x6e, 0x62, 0x01, 0x91, 0xdf, 0x01, 0x1b, 0xeb, 0x4d, 0xf1, 0x7d, 0xd3, 0xca, 0x01, 0x0d, 0x8b, 0x6d, 0x03, 0x0d,
        0x8b,
    ],
    0x0,
);

static RES_LOAD_LOOP_REFRESH_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x68, 0x32, 0x40, 0xf9, 0xee, 0x1b, 0x40, 0xf9, 0xdf, 0x01, 0x08, 0xeb, 0xec, 0x3f, 0x40, 0xf9, 0xed, 0x37, 0x40, 0xf9,
    ],
    0x0,
);

static PACKET_SEND_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x28, 0x4c, 0x43, 0xb9, 0x08, 0x4c, 0x03, 0xb9, 0xc0, 0x03, 0x5f, 0xd6, 0x00, 0x00, 0x00, 0x00,
    ],
    0x10,
);

static LUA_MAGIC_CHECK_SEARCH_CODE: (&[u8], isize) = (
    &[
        0xfd, 0x7b, 0x04, 0xa9, 0xfd, 0x03, 0x01, 0x91, 0x08, 0x04, 0x40, 0xf9, 0x93, 0x00, 0x80, 0x52, 0x13, 0x00, 0xa8, 0x72,
    ],
    0xB0,
);

static INKLING_PATCH_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x08, 0x95, 0x3e, 0x91, 0xe0, 0x1b, 0x80, 0x3d, 0x00, 0x04, 0xc0, 0x3d, 0xe9, 0x0b, 0x1e, 0x32, 0xa1, 0xe3, 0x02, 0xd1, 0xe0, 0x17, 0x80,
        0x3d, 0x00, 0x08, 0xc0, 0x3d, 0xe0, 0x13, 0x80, 0x3d, 0x00, 0x0c, 0xc0, 0x3d, 0xa9, 0x83, 0x14, 0x38, 0x09, 0x61, 0x40, 0xf8, 0x08, 0x01,
        0x40, 0xf9, 0xe0, 0x03, 0x15, 0xaa, 0xbf, 0x83, 0x15, 0xf8, 0x49, 0x73, 0x00, 0xf8, 0x48, 0x13, 0x00, 0xf8, 0xbf, 0x73, 0x15, 0x38, 0xe0,
        0x0f, 0x80, 0x3d,
    ],
    0x74,
);

static CLEAR_INK_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x08, 0xed, 0x19, 0x91, 0xe0, 0x17, 0x80, 0x3d, 0x00, 0x04, 0xc0, 0x3d, 0xe9, 0x0b, 0x1e, 0x32, 0xa1, 0xe3, 0x02, 0xd1, 0xe0, 0x13, 0x80,
        0x3d, 0x00, 0x08, 0xc0, 0x3d, 0xe0, 0x0f, 0x80, 0x3d, 0x00, 0x0c, 0xc0, 0x3d, 0xa9, 0x83, 0x14, 0x38, 0x09, 0x61, 0x40, 0xf8, 0x08, 0x01,
        0x40, 0xf9, 0xe0, 0x03, 0x15, 0xaa, 0xbf, 0x83, 0x15, 0xf8, 0xe9, 0x72, 0x00, 0xf8, 0xe8, 0x12, 0x00, 0xf8, 0xbf, 0x73, 0x15, 0x38, 0xe0,
        0x0b, 0x80, 0x3d,
    ],
    0x74,
);

static SET_GLOBAL_COLOR_FOR_CLASSIC_MODE_SEARCH_CODE: (&[u8], isize) =
    (&[0xA9, 0x1A, 0x00, 0xB9, 0x01, 0x8D, 0x43, 0x79, 0x80, 0xEE, 0x40, 0xF9], 0x0);

static LOAD_CHARA_1_FOR_ALL_COSTUMES_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x88, 0xea, 0x40, 0xb9, 0x08, 0x01, 0x1e, 0x32, 0x88, 0xea, 0x00, 0xb9, 0x88, 0x52, 0x40, 0xf9, 0xe9, 0x03, 0x00, 0x32, 0x89, 0xc6, 0x03,
        0x39, 0x9f, 0xd6, 0x03, 0x39, 0x89, 0xd2, 0x43, 0x39, 0x08, 0x41, 0x40, 0xf9, 0x09, 0x31, 0x07, 0x39, 0x08, 0x00, 0x80, 0x12, 0x09, 0xe0,
        0xdf, 0xd2, 0xe9, 0x1f, 0xe1, 0xf2, 0x88, 0xee, 0x00, 0xb9, 0xe8, 0x0b, 0x00, 0x32, 0xe0, 0x03, 0x13, 0xaa, 0x89, 0x7e, 0x00, 0xf9, 0x88,
        0x02, 0x01, 0xb9,
    ],
    -0xA94,
);

static LOAD_UI_FILE_SEARCH_CODE: (&[u8], isize) = (
    &[
        0xda, 0x2a, 0x00, 0xb9, 0xd3, 0x1a, 0x00, 0xf9, 0xfd, 0x7b, 0x46, 0xa9, 0xf4, 0x4f, 0x45, 0xa9, 0xf6, 0x57, 0x44, 0xa9, 0xf8, 0x5f, 0x43,
        0xa9, 0xfa, 0x67, 0x42, 0xa9, 0xfc, 0x6f, 0x41, 0xa9, 0xff, 0xc3, 0x01, 0x91, 0xc0, 0x03, 0x5f, 0xd6,
    ],
    0x28,
);

static GET_UI_CHARA_PATH_FROM_HASH_SEARCH: (&[u8], isize) = (
    &[
        0xff, 0xc3, 0x06, 0xd1, 0xfc, 0x67, 0x16, 0xa9, 0xf8, 0x5f, 0x17, 0xa9, 0xf6, 0x57, 0x18, 0xa9, 0xf4, 0x4f, 0x19, 0xa9, 0xfd, 0x7b, 0x1a,
        0xa9, 0xfd, 0x83, 0x06, 0x91, 0xf4, 0x03, 0x00, 0xaa, 0x18, 0x20, 0xf8, 0xd2, 0x9f, 0x9e, 0x40, 0xf2, 0x8a, 0x1e, 0x48, 0x92, 0xe8, 0x07,
        0x9f, 0x1a, 0x5f, 0x01, 0x18, 0xeb, 0xe0, 0x03, 0x1f, 0xaa, 0xe9, 0x17, 0x9f, 0x1a,
    ],
    0x0,
);

static GET_COLOR_NUM_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x68, 0x26, 0x40, 0xf9, 0x6e, 0x0e, 0x40, 0xf9, 0x0c, 0x9d, 0x40, 0x92, 0x28, 0x51, 0x80, 0xb8, 0xcd, 0x39, 0x42, 0xa9, 0xeb, 0x03, 0x1f,
        0x2a, 0xce, 0x01, 0x08, 0x8b, 0xe8, 0xa6, 0x00, 0xd0, 0x08, 0x2d, 0x3c, 0x91, 0x4f, 0x01, 0x0b, 0x0b, 0xff, 0x01, 0x00, 0x71, 0xef, 0xa5,
        0x8f, 0x1a, 0xef, 0x7d, 0x01, 0x13, 0xd0, 0xcd, 0x2f, 0x8b, 0x11, 0x02, 0x40, 0xb9, 0xb1, 0x79, 0x71, 0xf8, 0x3f, 0x02, 0x0c, 0xeb,
    ],
    0x104,
);

static GET_ECHO_FROM_HASH_SEARCH_CODE: (&[u8], isize) = (
    &[
        0xf6, 0x03, 0x00, 0x2a, 0x82, 0xef, 0x81, 0xd2, 0x02, 0x97, 0xaf, 0xf2, 0x82, 0x01, 0xc0, 0xf2, 0xe0, 0x03, 0x13, 0xaa, 0xe1, 0x03, 0x16,
        0x2a,
    ],
    -0x124,
);

static LOAD_STOCK_ICON_FOR_PORTRAIT_MENU_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x1c, 0x15, 0x40, 0xf9, 0x9c, 0x01, 0x00, 0xb4, 0x88, 0x03, 0x40, 0xf9, 0x08, 0xfd, 0x40, 0xf9, 0xe1, 0x03, 0x00, 0x32, 0xe0, 0x03, 0x1c,
        0xaa, 0x00, 0x01, 0x3f, 0xd6, 0x88, 0x03, 0x40, 0xf9, 0x08, 0xc5, 0x41, 0xf9, 0xe0, 0x03, 0x1c, 0xaa, 0xe1, 0x03, 0x1f, 0x2a, 0xe2, 0x03,
        0x1f, 0x2a, 0x00, 0x01, 0x3f, 0xd6, 0xf8, 0x7f, 0x01, 0xa9,
    ],
    -0x60,
);

static CSS_SET_SELECTED_CHARACTER_UI_SEARCH_CODE: (&[u8], isize) = (
    &[
        0xfc, 0x6f, 0xba, 0xa9, 0xfa, 0x67, 0x01, 0xa9, 0xf8, 0x5f, 0x02, 0xa9, 0xf6, 0x57, 0x03, 0xa9, 0xf4, 0x4f, 0x04, 0xa9, 0xfd, 0x7b, 0x05,
        0xa9, 0xfd, 0x43, 0x01, 0x91, 0xff, 0x83, 0x07, 0xd1, 0x08, 0x14, 0x41, 0xf9, 0x1c, 0x20, 0xf8, 0xd2, 0x0a, 0x1d, 0x48, 0x92, 0x09, 0x9d,
        0x40, 0x92, 0x36, 0x9c, 0x40, 0x92, 0x5f, 0x01, 0x1c, 0xeb, 0xf4, 0x03, 0x04, 0x2a, 0xf3, 0x03, 0x00, 0xaa, 0xf7, 0x03, 0x01, 0xaa, 0x24,
        0x09, 0x40, 0xfa,
    ],
    0x0,
);

static CHARA_SELECT_SCENE_DESTRUCTOR_SEARCH_CODE: (&[u8], isize) = (
    &[
        0xf5, 0x0f, 0x1d, 0xf8, 0xf4, 0x4f, 0x01, 0xa9, 0xfd, 0x7b, 0x02, 0xa9, 0xfd, 0x83, 0x00, 0x91, 0x48, 0x00, 0x40, 0xf9, 0x08, 0x11, 0x40,
        0xf9, 0xf5, 0x03, 0x00, 0xaa, 0xe0, 0x03, 0x02, 0xaa, 0xf3, 0x03, 0x02, 0xaa, 0xf4, 0x03, 0x01, 0xaa, 0x00, 0x01, 0x3f, 0xd6, 0xa8, 0x02,
        0x40, 0xf9, 0x03, 0x39, 0x40, 0xf9, 0xe1, 0x03, 0x14, 0xaa, 0xe2, 0x03, 0x13, 0xaa, 0xfd, 0x7b, 0x42, 0xa9, 0xf4, 0x4f, 0x41, 0xa9, 0xe0,
        0x03, 0x15, 0xaa, 0xf5, 0x07, 0x43, 0xf8, 0x60, 0x00, 0x1f, 0xd6,
    ],
    0xA80,
);

static MSBT_TEXT_SEARCH_CODE: (&[u8], isize) = (
    &[
        0xaa, 0x43, 0x00, 0x91, 0xea, 0x5f, 0x00, 0xf9, 0xea, 0x03, 0x00, 0x91, 0xe9, 0x05, 0x80, 0x92, 0x09, 0xf0, 0xdf, 0xf2, 0xe9, 0x9f, 0x00,
        0xf9, 0x4a, 0x01, 0x02, 0x91, 0xea, 0x67, 0x00, 0xf9, 0xea, 0x23, 0x02, 0x91, 0xe9, 0x7f, 0x0d, 0xa9, 0xe9, 0x67, 0x40, 0xf9, 0x4a, 0xc1,
        0x00, 0x91, 0xea, 0x63, 0x00, 0xf9, 0xe9, 0x9b, 0x00, 0xf9, 0xe9, 0x03, 0x0a, 0xaa, 0xe9, 0x97, 0x00, 0xf9, 0xe9, 0x5f, 0x40, 0xf9, 0xf6,
        0x63, 0x03, 0x91, 0xe8, 0x03, 0x01, 0xaa, 0xd4, 0x22, 0x00, 0x91, 0xf3, 0x03, 0x00, 0xaa, 0xe1, 0x17, 0x00, 0x32, 0xe3, 0x83, 0x04, 0x91,
        0xe0, 0x03, 0x14, 0xaa, 0xe2, 0x03, 0x08, 0xaa, 0xb5, 0xb8, 0x93, 0x52, 0x95, 0x23, 0xb0, 0x72, 0xff, 0x83, 0x03, 0x39, 0xe9, 0x93, 0x00,
        0xf9,
    ],
    0xC0,
);

static SKIP_OPENING_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x08, 0x40, 0x40, 0xf9, 0x08, 0x75, 0x40, 0xf9, 0x08, 0x01, 0x40, 0xf9, 0x08, 0x01, 0x40, 0xf9, 0x08, 0x01, 0x43, 0xf9, 0x00, 0x8d, 0x44,
        0xb9, 0xc0, 0x03, 0x5f, 0xd6,
    ],
    -0x9F8,
);

static TITLE_SCREEN_OPENING_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x68, 0x0a, 0x08, 0x8b, 0xe9, 0x03, 0x1f, 0x32, 0x09, 0x79, 0x00, 0xb9, 0xfd, 0x7b, 0x41, 0xa9, 0xf4, 0x4f, 0xc2, 0xa8, 0xc0, 0x03, 0x5f,
        0xd6,
    ],
    0x8,
);

static TITLE_SCENE_SHOW_HOW_TO_PLAY_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x68, 0x0a, 0x08, 0x8b, 0xe9, 0x03, 0x1f, 0x32, 0x09, 0x79, 0x00, 0xb9, 0xfd, 0x7b, 0x41, 0xa9, 0xf4, 0x4f, 0xc2, 0xa8, 0xc0, 0x03, 0x5f,
        0xd6,
    ],
    0xC8,
);

static PARAMETERS_CACHE_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x08, 0x11, 0x82, 0x52, 0x68, 0x6b, 0x68, 0x38, 0x69, 0x3f, 0x48, 0xf9, 0x6a, 0x43, 0x48, 0xf9, 0xea, 0x1b, 0x01, 0xf9, 0xe8, 0xe3, 0x08,
        0x39, 0xe9, 0x17, 0x01, 0xf9, 0xeb, 0x0b, 0x40, 0xf9, 0x7b, 0x22, 0x55, 0xa9, 0x08, 0x05, 0xc0, 0x39, 0xe9, 0x2b, 0x40, 0xa9, 0xec, 0x33,
        0x40, 0x79, 0x1f, 0x05, 0x00, 0x71, 0xe8, 0xd7, 0x9f, 0x1a, 0xe8, 0x07, 0x02, 0x39, 0x6c, 0xd3, 0x1c, 0x79, 0x6b, 0x33, 0x07, 0xf9, 0x6a,
        0x2f, 0x07, 0xf9, 0x69, 0x2b, 0x07, 0xf9, 0xea, 0xc3, 0x41, 0xf8, 0xeb, 0x4b, 0x40, 0x79, 0x68, 0x43, 0x39, 0x91, 0x69, 0xb3, 0x39, 0x91,
        0x1f, 0x01, 0x19, 0xeb,
    ],
    -0x158,
);

static IS_ONLINE_SEARCH_CODE: (&[u8], isize) = (
    &[
        0x29, 0xa1, 0x17, 0x91, 0xea, 0x03, 0x17, 0xaa, 0xe8, 0x02, 0x00, 0xf9, 0xe8, 0x03, 0x14, 0x32, 0xff, 0xfe, 0x00, 0xa9, 0x49, 0x8d, 0x01,
        0xf8, 0xe8, 0x22, 0x00, 0xb9, 0xe8, 0x03, 0x17, 0xaa, 0xf6, 0x03, 0x17, 0xaa, 0x1f, 0x8d, 0x02, 0xf8, 0xe8, 0x17, 0x00, 0xf9, 0xe8, 0x03,
        0x17, 0xaa, 0xff, 0x2a, 0x00, 0xf9, 0xff, 0x7e, 0x03, 0xa9, 0xfc, 0x03, 0x17, 0xaa, 0xf5, 0x03, 0x17, 0xaa, 0xf3, 0xe3, 0x06, 0x91, 0x1a,
        0x48, 0x88, 0x52, 0xfa, 0x01, 0xa0, 0x72, 0x1f, 0x8d, 0x04, 0xf8, 0xe8, 0x22, 0x00, 0xf9, 0xe8, 0x03, 0x17, 0xaa, 0x1f, 0x8d, 0x05, 0xf8,
        0xea, 0x23, 0x06, 0xa9, 0xe8, 0x03, 0x17, 0xaa, 0x1f, 0x0d, 0x09, 0xf8, 0xe8, 0x1f, 0x00, 0xf9, 0xe8, 0x03, 0x17, 0xaa, 0xff, 0xe2, 0x0e,
        0xf8, 0x1f, 0x0d, 0x0c, 0xf8, 0xe8, 0x23, 0x00, 0xf9, 0xe8, 0x03, 0x17, 0xaa, 0x1f, 0x8d, 0x0d, 0xf8, 0xdf, 0x0e, 0x06, 0xf8, 0x9f, 0x8f,
        0x07, 0xf8, 0xbf, 0x8e, 0x0a, 0xf8, 0xe8, 0x1b, 0x00, 0xf9,
    ],
    -0xAf4,
);

static CHANGE_COLOR_R_CODE: (&[u8], isize) = (
    &[
        0xa2, 0x06, 0x41, 0xf9, 0xa4, 0x0a, 0x5b, 0x39, 0x03, 0x1d, 0x00, 0x12, 0xa0, 0xf6, 0x42, 0xf9, 0xe1, 0x03, 0x18, 0xaa, 0xe5, 0x03, 0x1f,
        0x2a, 0xa8, 0x42, 0x08, 0x39,
    ],
    0x18,
);

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn get_offset(haystack: &[u8], search_code: (&[u8], isize)) -> usize {
    (find_subsequence(&haystack, search_code.0).unwrap() as isize + search_code.1) as usize
}

#[allow(clippy::inconsistent_digit_grouping)]
fn offset_from_adrp(adrp_offset: usize) -> usize {
    unsafe {
        let adrp = *(offset_to_addr(adrp_offset) as *const u32);
        let immhi = (adrp & 0b0000_0000_1111_1111_1111_1111_1110_0000) >> 3;
        let immlo = (adrp & 0b0110_0000_0000_0000_0000_0000_0000_0000) >> 29;
        let imm = ((immhi | immlo) << 12) as i32 as usize;
        let base = adrp_offset & 0xFFFF_FFFF_FFFF_F000;
        base + imm
    }
}

#[allow(clippy::inconsistent_digit_grouping)]
fn offset_from_ldr(ldr_offset: usize) -> usize {
    unsafe {
        let ldr = *(offset_to_addr(ldr_offset) as *const u32);
        let size = (ldr & 0b1100_0000_0000_0000_0000_0000_0000_0000) >> 30;
        let imm = (ldr & 0b0000_0000_0011_1111_1111_1100_0000_0000) >> 10;
        (imm as usize) << size
    }
}

#[allow(clippy::inconsistent_digit_grouping)]
fn offset_from_strb_unsigned_immediate(strb_offset: usize) -> usize {
    unsafe {
        let strb = *(offset_to_addr(strb_offset) as *const u32);
        ((strb & 0b00000_000_00_111111111111_00000_00000) >> 10) as usize
    }
}

pub fn offset_to_addr(offset: usize) -> *const () {
    unsafe { (getRegionAddress(Region::Text) as *const u8).add(offset) as _ }
}

fn get_text() -> &'static [u8] {
    unsafe {
        let ptr = getRegionAddress(Region::Text) as *const u8;
        let size = (getRegionAddress(Region::Rodata) as usize) - (ptr as usize);
        std::slice::from_raw_parts(ptr, size)
    }
}

macro_rules! generate_members {
    (struct $name:ident {
        $($field_name:ident: $field_type:ty,)*
    }) => {

        #[derive(Serialize, Deserialize)]
        struct $name {
            pub $($field_name: $field_type,)*
        }

        $(pub fn $field_name() -> usize { OFFSETS.$field_name })*
    }
}

generate_members! {
    struct Offsets {
        lookup_stream_hash: usize,
        inflate: usize,
        memcpy_1: usize,
        memcpy_2: usize,
        memcpy_3: usize,
        inflate_dir_file: usize,
        initial_loading: usize,
        res_load_loop_start: usize,
        res_load_loop_refresh: usize,
        title_screen_version: usize,
        eshop_button: usize,
        msbt_text: usize,
        skip_opening: usize,
        title_scene_play_opening: usize,
        title_scene_how_to_play: usize,
        filesystem_info: usize,
        res_service: usize,
        packet_send: usize,
        lua_magic_check: usize,
        inkling_patch: usize,
        clear_ink_patch: usize,
        set_global_color_for_classic_mode: usize,
        load_chara_1_for_all_costumes: usize,
        load_ui_file: usize,
        get_ui_chara_path_from_hash: usize,
        get_color_num_from_hash: usize,
        get_echo_from_hash: usize,
        load_stock_icon_for_portrait_menu: usize,
        css_set_selected_character_ui: usize,
        chara_select_scene_destructor: usize,
        parameters_cache: usize,
        is_online: usize,
        change_color_r: usize,
        change_color_l: usize,
    }
}

impl Offsets {
    pub fn new() -> Option<Self> {
        let text = get_text();
        let lookup_stream_hash = get_offset_neon(text, LOOKUP_STREAM_HASH_SEARCH_CODE);
        let inflate = get_offset_neon(text, INFLATE_SEARCH_CODE);
        let memcpy_1 = get_offset_neon(text, MEMCPY_1_SEARCH_CODE);
        let memcpy_2 = get_offset_neon(text, MEMCPY_2_SEARCH_CODE);
        let memcpy_3 = get_offset_neon(text, MEMCPY_3_SEARCH_CODE);
        let inflate_dir_file = get_offset_neon(text, INFLATE_DIR_FILE_SEARCH_CODE);
        let initial_loading = get_offset_neon(text, INITIAL_LOADING_SEARCH_CODE);
        let res_load_loop_start = get_offset_neon(text, RES_LOAD_LOOP_START_SEARCH_CODE);
        let res_load_loop_refresh = get_offset_neon(text, RES_LOAD_LOOP_REFRESH_SEARCH_CODE);
        let title_screen_version = get_offset_neon(text, TITLE_SCREEN_VERSION_SEARCH_CODE);
        let eshop_button = get_offset_neon(text, ESHOPMANAGER_SHOW_SEARCH_CODE);
        let msbt_text = get_offset_neon(text, MSBT_TEXT_SEARCH_CODE);
        let skip_opening = get_offset_neon(text, SKIP_OPENING_SEARCH_CODE);
        let title_scene_play_opening = get_offset_neon(text, TITLE_SCREEN_OPENING_SEARCH_CODE);
        let title_scene_how_to_play = get_offset_neon(text, TITLE_SCENE_SHOW_HOW_TO_PLAY_SEARCH_CODE);
        let packet_send = get_offset_neon(text, PACKET_SEND_SEARCH_CODE);
        let lua_magic_check = get_offset_neon(text, LUA_MAGIC_CHECK_SEARCH_CODE);
        let inkling_patch = get_offset_neon(text, INKLING_PATCH_SEARCH_CODE);
        let clear_ink_patch = get_offset_neon(text, CLEAR_INK_SEARCH_CODE);
        let set_global_color_for_classic_mode = get_offset_neon(text, SET_GLOBAL_COLOR_FOR_CLASSIC_MODE_SEARCH_CODE);
        let load_chara_1_for_all_costumes = get_offset_neon(text, LOAD_CHARA_1_FOR_ALL_COSTUMES_SEARCH_CODE);
        let load_ui_file = get_offset_neon(text, LOAD_UI_FILE_SEARCH_CODE);
        let get_ui_chara_path_from_hash = get_offset_neon(text, GET_UI_CHARA_PATH_FROM_HASH_SEARCH);
        let get_color_num_from_hash = get_offset_neon(text, GET_COLOR_NUM_SEARCH_CODE);
        let get_echo_from_hash = get_offset_neon(text, GET_ECHO_FROM_HASH_SEARCH_CODE);
        let load_stock_icon_for_portrait_menu = get_offset_neon(text, LOAD_STOCK_ICON_FOR_PORTRAIT_MENU_SEARCH_CODE);
        let css_set_selected_character_ui = get_offset_neon(text, CSS_SET_SELECTED_CHARACTER_UI_SEARCH_CODE);
        let chara_select_scene_destructor = get_offset_neon(text, CHARA_SELECT_SCENE_DESTRUCTOR_SEARCH_CODE);
        let change_color_r = get_offset_neon(text, CHANGE_COLOR_R_CODE);
        let change_color_l = change_color_r + 0x298;
        let filesystem_info = {
            let adrp = get_offset_neon(text, FILESYSTEM_INFO_ADRP_SEARCH_CODE);
            let adrp_offset = offset_from_adrp(adrp);
            let ldr_offset = offset_from_ldr(adrp + 4);
            adrp_offset + ldr_offset
        };
        let res_service = {
            let adrp = get_offset_neon(text, RES_SERVICE_ADRP_SEARCH_CODE);
            let adrp_offset = offset_from_adrp(adrp);
            let ldr_offset = offset_from_ldr(adrp + 4);
            adrp_offset + ldr_offset
        };
        let parameters_cache = {
            let adrp = get_offset_neon(text, PARAMETERS_CACHE_SEARCH_CODE);
            let adrp_offset = offset_from_adrp(adrp);
            let ldr_offset = offset_from_ldr(adrp + 4);
            adrp_offset + ldr_offset
        };
        let is_online = {
            let adrp = get_offset_neon(text, IS_ONLINE_SEARCH_CODE);
            let adrp_offset = offset_from_adrp(adrp);
            let strb_offset = offset_from_strb_unsigned_immediate(adrp + 4);
            adrp_offset + strb_offset
        };

        Some(Self {
            lookup_stream_hash,
            inflate,
            memcpy_1,
            memcpy_2,
            memcpy_3,
            inflate_dir_file,
            initial_loading,
            res_load_loop_start,
            res_load_loop_refresh,
            title_screen_version,
            eshop_button,
            msbt_text,
            skip_opening,
            title_scene_play_opening,
            title_scene_how_to_play,
            packet_send,
            filesystem_info,
            res_service,
            lua_magic_check,
            inkling_patch,
            clear_ink_patch,
            set_global_color_for_classic_mode,
            load_chara_1_for_all_costumes,
            load_ui_file,
            get_ui_chara_path_from_hash,
            get_color_num_from_hash,
            get_echo_from_hash,
            load_stock_icon_for_portrait_menu,
            css_set_selected_character_ui,
            chara_select_scene_destructor,
            parameters_cache,
            is_online,
            change_color_r,
            change_color_l,
        })
    }
}

// Don't go and steal that stuff, it's definitely not finished
pub fn get_offset_neon(data: &[u8], pattern: (&'static [u8], isize)) -> usize {
    let mut s = String::new();

    for byte in pattern.0 {
        write!(&mut s, "{:X} ", byte).expect("lmao");
    }

    write!(&mut s, "??").expect("lmao");

    ((find_pattern_neon(data.as_ptr(), data.len(), s) as isize) + pattern.1) as usize
}

pub fn find_pattern_neon<S: AsRef<str>>(data: *const u8, data_len: usize, pattern: S) -> usize {
    let pattern = SimdPatternScanData::new(&pattern);

    let match_table = build_match_indexes(&pattern);
    let pattern_vecs = pattern_to_vec(&pattern);
    let match_table_len = match_table.len();

    // Fills a register with the first byte of the pattern
    let first_byte_vec = unsafe { vdupq_n_u8(pattern.bytes[pattern.leading_ignore_count]) };

    // Compute the size of the array minus what's the biggest size between the pattern or a Simd register
    let search_length = data_len - std::cmp::max(pattern.bytes.len(), NEON_REGISTER_LENGTH);

    let leading_ignore_count = pattern.leading_ignore_count;
    let mut data_ptr = data as usize;
    let data_ptr_max = data_ptr + search_length;

    'data: while data_ptr < data_ptr_max {
        // Fills a register with bytes
        let rhs = unsafe { vld1q_u8(data_ptr as _) };

        // Compare the register filled with the first byte with the 16 next bytes and return a vector where matching bytes are represented by 0xFF and the rest by 0x0
        let equal = unsafe { vceqq_u8(first_byte_vec, rhs) };

        // Converts vceqq's output to a u32 bitfield equivalent where matching bytes are represented by a bit being set
        let find_first_byte = _mm_movemask_aarch64(equal);

        // If the value is 0, it means no bit was set, and therefore the first byte of the signature is missing.
        // Abort early and move on to the next 16 bytes
        if find_first_byte == 0 {
            data_ptr += NEON_REGISTER_LENGTH - 1;
            continue;
        }

        // Advance the pointer by the amount of non-matching bytes in the current window
        let test = (find_first_byte.trailing_zeros() as i32).wrapping_sub(leading_ignore_count as i32);
        data_ptr = data_ptr.wrapping_add_signed(test as isize);

        let mut match_table_index = 0;

        // For each array of pattern
        for (i, cur_pattern_vec) in pattern_vecs.iter().enumerate() {
            let register_byte_offs = i * NEON_REGISTER_LENGTH;

            let next_byte = data_ptr + register_byte_offs + 1;

            let rhs_2 = unsafe { vld1q_u8(next_byte as _) };

            let compare_result = unsafe { _mm_movemask_aarch64(vceqq_u8(*cur_pattern_vec, rhs_2)) };

            // TODO
            'match_table: while match_table_index < match_table_len {
                let match_index = std::num::Wrapping(match_table[match_table_index] as usize) - std::num::Wrapping(register_byte_offs as usize);

                if match_index.0 < NEON_REGISTER_LENGTH {
                    if ((compare_result >> match_index.0) & 1) != 1 {
                        // TODO: Improve this. Moves by one
                        data_ptr += 1;
                        continue 'data;
                    } else {
                        match_table_index += 1;
                        continue;
                    }
                }

                break;
            }
        }

        return data_ptr - data as usize;
    }

    // We are past the point where we can still look for the signature without risking an overflow, so tread carefully
    let position = data_ptr - data as usize;

    // TODO: Do a simpler search in the remaining bytes here
    data_ptr - data as usize
}

pub fn pattern_to_vec(cb_pattern: &SimdPatternScanData) -> Vec<uint8x16_t> {
    let mut pattern_len = cb_pattern.mask.len();
    let vector_count = (pattern_len - 1).div_ceil(NEON_REGISTER_LENGTH);
    let mut pattern_vecs: Vec<uint8x16_t> = Vec::with_capacity(vector_count);

    let pattern = unsafe { cb_pattern.bytes.as_slice().get_unchecked(1) } as *const u8;

    pattern_len -= 1;

    for i in 0..vector_count {
        if i < vector_count - 1 {
            pattern_vecs.push(unsafe { vld1q_u8(pattern.add(i * NEON_REGISTER_LENGTH)) })
        } else {
            let o = i * NEON_REGISTER_LENGTH;
            let neon: &mut [u8; NEON_REGISTER_LENGTH] = &mut [0; NEON_REGISTER_LENGTH];

            unsafe {
                neon[0] = *pattern.add(o);
                neon[1] = if o + 1 < pattern_len { *pattern.add(o + 1) } else { 0 };
                neon[2] = if o + 2 < pattern_len { *pattern.add(o + 2) } else { 0 };
                neon[3] = if o + 3 < pattern_len { *pattern.add(o + 3) } else { 0 };
                neon[4] = if o + 4 < pattern_len { *pattern.add(o + 4) } else { 0 };
                neon[5] = if o + 5 < pattern_len { *pattern.add(o + 5) } else { 0 };
                neon[6] = if o + 6 < pattern_len { *pattern.add(o + 6) } else { 0 };
                neon[7] = if o + 7 < pattern_len { *pattern.add(o + 7) } else { 0 };
                neon[8] = if o + 8 < pattern_len { *pattern.add(o + 8) } else { 0 };
                neon[9] = if o + 9 < pattern_len { *pattern.add(o + 9) } else { 0 };
                neon[10] = if o + 10 < pattern_len { *pattern.add(o + 10) } else { 0 };
                neon[11] = if o + 11 < pattern_len { *pattern.add(o + 11) } else { 0 };
                neon[12] = if o + 12 < pattern_len { *pattern.add(o + 12) } else { 0 };
                neon[13] = if o + 13 < pattern_len { *pattern.add(o + 13) } else { 0 };
                neon[14] = if o + 14 < pattern_len { *pattern.add(o + 14) } else { 0 };
                neon[15] = if o + 15 < pattern_len { *pattern.add(o + 15) } else { 0 };
            }

            unsafe { pattern_vecs.push(vld1q_u8(neon.as_ptr())) };
        }
    }

    pattern_vecs
}

pub fn build_match_indexes(scan_pattern: &SimdPatternScanData) -> Vec<u16> {
    let mask_length = scan_pattern.mask.len();
    let mut full_match_table: Vec<u16> = vec![0; mask_length];
    let mut match_count = 0;

    for i in 1..mask_length {
        // If this byte is masked, we continue
        if scan_pattern.mask[i] != 1 {
            continue;
        }

        // Add the index of the byte that wasn't in the vector
        full_match_table[match_count] = i as u16 - 1;
        match_count += 1;
    }

    full_match_table
}

/// Equivalent to MoveMask in x86
#[inline]
fn _mm_movemask_aarch64(input: uint8x16_t) -> u32 {
    const UC_SHIFT: [i8; 16] = [-7, -6, -5, -4, -3, -2, -1, 0, -7, -6, -5, -4, -3, -2, -1, 0];
    // Fills a vector with UC_SHIFT
    let vshift = unsafe { vld1q_s8(UC_SHIFT.as_ptr()) };
    // Fills a vector with 0x80 and performs AND on the input vector
    let vmask = unsafe { vandq_u8(input, vdupq_n_u8(0x80)) };
    // Shift-left vmask using UC_SHIFT
    let vmask = unsafe { vshlq_u8(vmask, vshift) };

    // Takes the lower 64 bits of vmask and add all bytes together
    let mut out: u32 = unsafe { vaddv_u8(vget_low_u8(vmask)) }.into();
    // Takes the higher 64 bits of vmask, add all bytes together then shift left by 8 and add the result to out
    out += unsafe { (vaddv_u8(vget_high_u8(vmask)) as u32) << 8 };

    out
}

pub struct SimdPatternScanData {
    pub bytes: Vec<u8>,
    pub mask: Vec<u8>,
    pub leading_ignore_count: usize,
}

impl SimdPatternScanData {
    pub fn new<S: AsRef<str>>(pattern: S) -> Self {
        let pattern = pattern.as_ref();
        let mut leading_ignore_count = 0;
        let mut bytes = vec![];
        let mut mask = vec![];
        let mut found_non_ignore = false;
        let iter = pattern.split(' ').map(|value| value.trim_start_matches("0x"));

        for curr in iter {
            if curr == "??" {
                mask.push(0);
                bytes.push(0);

                if !found_non_ignore {
                    leading_ignore_count += 1;
                }
            } else {
                bytes.push(u8::from_str_radix(curr, 16).unwrap());
                mask.push(1);
                found_non_ignore = true;
            }
        }

        Self {
            bytes,
            mask,
            leading_ignore_count,
        }
    }
}
