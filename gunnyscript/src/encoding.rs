//! String encoding/decoding functionality for Gunnyscript.
//!
//! At the moment, only UTF-8 encoding is supported.

use crate::{located_err, Error, Located};

pub const START_LINE: usize = 1;

/// A decoder groups bytes together to be interpreted by an encoding scheme. For
/// example, for UTF-8, one would group up to 4 bytes together.
pub trait Decoder<'a>: From<&'a str> + Iterator<Item = Result<&'a [u8], Located<Error>>> {
    /// Peeks ahead one character (group of bytes). If the stream has ended,
    /// returns `None`.
    fn peek(&self) -> Option<Result<&'a [u8], Located<Error>>>;

    /// Peeks ahead `len` characters. If the stream has ended, returns `None`.
    fn peek_slice(&self, len: usize) -> Option<Result<&'a [u8], Located<Error>>>;

    /// Extracts a slice of bytes from the input stream from the given start
    /// position (inclusive) up to the given end position (exclusive). If the
    /// end of the range extends past the end of the stream, this returns
    /// `None`.
    fn slice(&self, start: usize, end: usize) -> Option<&'a [u8]>;

    /// Returns the current byte position in the input stream.
    fn pos(&self) -> usize;

    /// Returns the current line being processed in the input stream.
    fn line(&self) -> usize;

    /// Returns whether or not we have hit the end of the stream.
    fn eof(&self) -> bool;
}

pub struct Utf8Decoder<'a> {
    src: &'a [u8],
    pos: usize,
    len: usize,
    line: usize,
}

impl<'a> From<&'a str> for Utf8Decoder<'a> {
    fn from(s: &'a str) -> Self {
        let src = s.as_bytes();
        Self {
            src,
            pos: 0,
            len: src.len(),
            line: START_LINE,
        }
    }
}

impl<'a> Iterator for Utf8Decoder<'a> {
    type Item = Result<&'a [u8], Located<Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eof() {
            return None;
        }
        let pos = self.pos;
        let b = self.src[pos];
        let ch_len = UTF8_CHAR_WIDTH[b as usize] as usize;
        if pos + ch_len >= self.len {
            return Some(located_err(self.line, Error::UnexpectedEof));
        }
        self.pos += ch_len;
        // Exclusively count newline characters as newlines
        if ch_len == 1 && b == 0x0A {
            self.line += 1;
        }
        Some(Ok(&self.src[pos..self.pos]))
    }
}

impl<'a> Decoder<'a> for Utf8Decoder<'a> {
    #[inline]
    fn peek(&self) -> Option<Result<&'a [u8], Located<Error>>> {
        if self.pos >= self.len {
            return None;
        }
        let b = self.src[self.pos];
        let ch_len = UTF8_CHAR_WIDTH[b as usize] as usize;
        if self.pos + ch_len >= self.len {
            return Some(located_err(self.line, Error::IncompleteUtf8Char));
        }
        Some(Ok(&self.src[self.pos..self.pos + ch_len]))
    }

    #[inline]
    fn peek_slice(&self, len: usize) -> Option<Result<&'a [u8], Located<Error>>> {
        if self.pos >= self.len {
            return None;
        }
        let mut bytes_len = 0;
        for i in 0..len {
            let pos = (self.pos + bytes_len) as usize;
            let ch_len = UTF8_CHAR_WIDTH[self.src[pos] as usize] as usize;
            if pos + ch_len >= self.len {
                return Some(located_err(self.line, Error::IncompleteUtf8Char));
            }
            bytes_len += ch_len;
        }
        Some(Ok(&self.src[self.pos..self.pos + bytes_len]))
    }

    #[inline]
    fn slice(&self, start: usize, end: usize) -> Option<&'a [u8]> {
        if end >= self.len {
            None
        } else {
            Some(&self.src[start..end])
        }
    }

    #[inline]
    fn pos(&self) -> usize {
        self.pos
    }

    #[inline]
    fn line(&self) -> usize {
        self.line
    }

    #[inline]
    fn eof(&self) -> bool {
        self.pos >= self.len
    }
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
