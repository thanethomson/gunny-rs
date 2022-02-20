//! Functionality relating to UTF-8 decoding.

use bytes::{Buf, Bytes};

use crate::EncodingError;

/// Decodes characters from an in-memory buffer of bytes.
pub trait Decoder {
    /// Attempt to decode a single character from the given buffer.
    fn decode_char(buf: &mut Bytes) -> Result<Option<char>, EncodingError>;
}

// Fast lookup table taken from core::str::validation
// https://tools.ietf.org/html/rfc3629
const UTF8_CHAR_WIDTH: &[u8; 256] = &[
    // 1  2  3  4  5  6  7  8  9  A  B  C  D  E  F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 1
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 2
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 3
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 4
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 5
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 6
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 7
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 8
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 9
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // A
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // B
    0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // C
    2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // D
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // E
    4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // F
];

#[derive(Default)]
pub struct Utf8Decoder;

impl Decoder for Utf8Decoder {
    fn decode_char(buf: &mut Bytes) -> Result<Option<char>, EncodingError> {
        // Not enough data
        if !buf.has_remaining() {
            return Ok(None);
        }
        let a = buf.get_u8();
        let ch_len = UTF8_CHAR_WIDTH[a as usize] as usize;
        if ch_len == 0 {
            return Err(EncodingError::InvalidUtf8);
        }
        // Not enough data
        if buf.remaining() < ch_len - 1 {
            return Ok(None);
        }
        let a = a as u32;
        let ch = char::try_from(match ch_len {
            1 => a,
            2 => {
                let b = buf.get_u8() as u32;
                (a & 0x1F) << 6 | b
            }
            3 => {
                let b = buf.get_u8() as u32;
                let c = buf.get_u8() as u32;
                (a & 0x0F) << 12 | b << 6 | c
            }
            4 => {
                let b = buf.get_u8() as u32;
                let c = buf.get_u8() as u32;
                let d = buf.get_u8() as u32;
                (a & 0x07) << 18 | b << 12 | c << 6 | d
            }
            _ => return Err(EncodingError::InvalidUtf8),
        })
        .map_err(|_| EncodingError::InvalidUtf8)?;
        Ok(Some(ch))
    }
}
