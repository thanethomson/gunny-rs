//! Gunnyscript parser in Rust.
//!
//! Only supports UTF-8 encoding at present.

use crate::{located_err, Error, Located};

const START_LINE: usize = 1;

#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    DocstringLine(&'a str),
    ObjectStart,
    ObjectEnd,
    ArrayStart,
    ArrayEnd,
    PropertyId(&'a str),
    Value(SimpleValue<'a>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SimpleValue<'a> {
    Null,
    Number(&'a str),
    Date(&'a str),
    DateTime(&'a str),
    String(&'a str),
    DedentString(&'a str),
    LiteralString(&'a str),
    DedentLiteralString(&'a str),
}

pub struct Lexer<'a> {
    src: &'a [u8],
    len: usize,  // Memoized input length
    pos: usize,  // Our current position in the input
    line: usize, // Our current line number
}

impl<'a> From<&'a str> for Lexer<'a> {
    fn from(s: &'a str) -> Self {
        let src = s.as_bytes();
        Self {
            src,
            len: src.len(),
            pos: 0,
            line: START_LINE,
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token<'a>, Located<Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.len {
            let peek = match Peek::from_slice(self.src, self.pos, 1, self.line, []) {
                Ok(p) => p,
                Err(e) => return Some(Err(e)),
            };
            // We only expect 1-byte UTF-8 characters at this point
            if peek.slice.len() != 1 {
                return Some(self.located_err(Error::UnexpectedChar));
            }
            match peek.slice[0] {
                // Whitespace
                b' ' | b'\t' | b'\r' | b'\n' => self.advance(peek),
                b'/' => match self.try_parse_comment_or_docstring() {
                    Ok(Some(docstring)) => return Some(Ok(docstring)),
                    // Skip comments that aren't docstrings
                    Ok(None) => {}
                    Err(e) => return Some(Err(e)),
                },
                b'n' => {
                    return match self.try_parse_null() {
                        Ok(Some(t)) => Some(Ok(t)),
                        Ok(None) => Some(self.parse_property_id()),
                        Err(e) => Some(Err(e)),
                    }
                }
                b't' | b'f' => {
                    return match self.try_parse_bool() {
                        Ok(Some(t)) => Some(Ok(t)),
                        Ok(None) => Some(self.parse_property_id()),
                        Err(e) => Some(Err(e)),
                    }
                }
                b'"' => return Some(self.parse_string()),
                b'#' => return Some(self.parse_string_literal()),
                b'd' => {
                    return match self.try_parse_dedent_string() {
                        Ok(Some(t)) => Some(Ok(t)),
                        Ok(None) => Some(self.parse_property_id()),
                        Err(e) => Some(Err(e)),
                    }
                }
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => return Some(self.parse_property_id()),
                b'{' => return Some(Ok(Token::ObjectStart)),
                b'}' => return Some(Ok(Token::ObjectEnd)),
                b'[' => return Some(Ok(Token::ArrayStart)),
                b']' => return Some(Ok(Token::ArrayEnd)),
                b'0'..=b'9' => {
                    return match self.try_parse_number() {
                        Ok(Some(t)) => Some(Ok(t)),
                        Ok(None) => match self.try_parse_datetime() {
                            Ok(Some(t)) => Some(Ok(t)),
                            Ok(None) => Some(self.parse_date()),
                            Err(e) => Some(Err(e)),
                        },
                        Err(e) => Some(Err(e)),
                    }
                }
                _ => return Some(self.located_err(Error::UnexpectedChar)),
            }
        }
        None
    }
}

impl<'a> Lexer<'a> {
    fn advance(&mut self, peek: Peek<'a>) {
        if peek.from != self.pos {
            panic!(
                "unexpected starting position for peek advancement: from={}, pos={}",
                peek.from, self.pos
            );
        }
        self.pos += peek.slice.len();
        self.line += peek.lines;
    }

    fn try_parse_comment_or_docstring(&mut self) -> Result<Option<Token<'a>>, Located<Error>> {
        let peek = Peek::from_slice(self.src, self.pos, 3, self.line, [])?;
        if peek.starts_with([b'/', b'/', b'/']) {
            // Skip past the "///" - we're only interested in the rest of the
            // line, including the newline.
            self.advance(peek);

            let peek = Peek::from_slice(self.src, self.pos, -1, self.line, [b'\n'])?;
            let s = core::str::from_utf8(peek.slice)
                .map_err(|e| Located::new(self.line, Error::Utf8Error(e)))?;
            self.advance(peek);
            return Ok(Some(Token::DocstringLine(s)));
        }
        let peek = if peek.starts_with([b'/', b'*']) {
            Peek::from_slice(self.src, self.pos, -1, self.line, [b'*', b'/'])?
        } else if peek.starts_with([b'/', b'/']) {
            Peek::from_slice(self.src, self.pos, -1, self.line, [b'\n'])?
        } else {
            return self.located_err(Error::UnexpectedChar);
        };
        self.advance(peek);
        // Skip comments
        Ok(None)
    }

    fn try_parse_null(&mut self) -> Result<Option<Token<'a>>, Located<Error>> {
        todo!()
    }

    fn try_parse_bool(&mut self) -> Result<Option<Token<'a>>, Located<Error>> {
        todo!()
    }

    fn parse_property_id(&mut self) -> Result<Token<'a>, Located<Error>> {
        todo!()
    }

    fn parse_string(&mut self) -> Result<Token<'a>, Located<Error>> {
        todo!()
    }

    fn parse_string_literal(&mut self) -> Result<Token<'a>, Located<Error>> {
        todo!()
    }

    fn try_parse_dedent_string(&mut self) -> Result<Option<Token<'a>>, Located<Error>> {
        todo!()
    }

    fn try_parse_number(&mut self) -> Result<Option<Token<'a>>, Located<Error>> {
        todo!()
    }

    fn try_parse_datetime(&mut self) -> Result<Option<Token<'a>>, Located<Error>> {
        todo!()
    }

    fn parse_date(&mut self) -> Result<Token<'a>, Located<Error>> {
        todo!()
    }

    #[inline]
    fn located_err<T, E>(&self, err: E) -> Result<T, Located<E>> {
        located_err(self.line, err)
    }
}

struct Peek<'a> {
    slice: &'a [u8],
    from: usize,
    lines: usize,
}

impl<'a> Peek<'a> {
    fn from_slice<const C: usize>(
        src: &'a [u8],
        from: usize,
        len: i32,
        start_line: usize,
        until: [u8; C],
    ) -> Result<Self, Located<Error>> {
        let mut pos = from;
        let mut lines = 0;
        let mut chars = 0_usize;
        let mut match_buf = [0_u8; C];
        while pos < src.len() && (len < 0 || chars < (len as usize)) {
            let b = src[pos];
            let ch_len = UTF8_CHAR_WIDTH[b as usize] as usize;
            if pos + ch_len > src.len() {
                return located_err(start_line + lines, Error::IncompleteUtf8Char);
            }
            pos += ch_len;
            chars += 1;
            if ch_len == 1 && b == b'\n' {
                lines += 1;
            }
            if C > 0 {
                slice_push(&mut match_buf, &src[pos - ch_len..pos]);
                if match_buf == until {
                    break;
                }
            }
        }
        Ok(Self {
            slice: &src[from..pos],
            from,
            lines,
        })
    }

    fn starts_with<const C: usize>(&self, p: [u8; C]) -> bool {
        if self.slice.len() < C {
            return false;
        }
        for (i, b) in p.iter().enumerate() {
            if *b != self.slice[i] {
                return false;
            }
        }
        true
    }
}

#[inline]
fn slice_rotl(s: &mut [u8], n: usize) {
    // We don't care
    if n >= s.len() {
        return;
    }
    for i in s.len() - n - 1..s.len() - 1 {
        s[i] = s[i + 1];
    }
}

#[inline]
fn slice_push(s: &mut [u8], ch: &[u8]) {
    if s.is_empty() {
        return;
    }
    slice_rotl(s, ch.len());
    let start = if ch.len() < s.len() {
        s.len() - ch.len()
    } else {
        0
    };
    for (i, c) in ch.iter().enumerate() {
        if start + i >= s.len() {
            break;
        }
        s[start + i] = *c;
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

#[cfg(test)]
mod test {
    use alloc::vec::Vec;

    use super::*;

    #[test]
    fn comment_and_whitespace_lexing() {
        const TEST_CASES: &[(&str, &[Token<'static>])] = &[
            (" ", &[]),
            ("\r", &[]),
            (" \t\r", &[]),
            ("// A comment", &[]),
            ("/*\nA multi-line comment\n*/", &[]),
            ("/// Docstring", &[Token::DocstringLine(" Docstring")]),
            (
                r#"
/*
 * A multi-line comment
 */
/// A multi-line
/// docstring

// A single-line comment
                "#,
                &[
                    Token::DocstringLine(" A multi-line\n"),
                    Token::DocstringLine(" docstring\n"),
                ],
            ),
        ];
        for (i, (tc, expected)) in TEST_CASES.iter().enumerate() {
            let lexer = Lexer::from(*tc);
            let actual = lexer
                .into_iter()
                .collect::<Result<Vec<Token>, Located<Error>>>()
                .expect(*tc);
            assert_eq!(Vec::from(*expected), actual, "test case {}", i);
        }
    }

    #[test]
    fn unexpected_char() {
        const TEST_CASES: &[&str] = &["😂", "$", "   $"];
        for tc in TEST_CASES {
            let r = Lexer::from(*tc).next().unwrap();
            assert_eq!(r, located_err(1, Error::UnexpectedChar));
        }
    }
}
