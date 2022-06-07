//! Gunnyscript parser in Rust.
//!
//! Only supports UTF-8 encoding at present.

use crate::{located_err, Error, Located};

const START_LINE: usize = 1;
const MATCH_BUF_SIZE: usize = 10;
const STRING_LITERAL_DELIM: &[u8] = b"\"####################";

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
    Bool(bool),
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
            let peek = match self.peek_char() {
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
                    return match self.try_parse_bool(peek.slice[0]) {
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
        let peek = self.peek_to_len(3)?;
        if peek.starts_with(b"///") {
            // Skip past the "///" - we're only interested in the rest of the
            // line, including the newline.
            self.advance(peek);

            let peek = self.peek_until_match(&[b"\n"], &[], true, false)?;
            let s = core::str::from_utf8(peek.slice)
                .map_err(|e| Located::new(self.line, Error::Utf8Error(e)))?;
            self.advance(peek);
            return Ok(Some(Token::DocstringLine(s)));
        }
        let peek = if peek.starts_with(b"/*") {
            self.peek_until_match(&[b"*/"], &[], true, true)?
        } else if peek.starts_with(b"//") {
            self.peek_until_match(&[b"\n"], &[], true, false)?
        } else {
            return self.located_err(Error::UnexpectedChar);
        };
        self.advance(peek);
        // Skip comments
        Ok(None)
    }

    fn try_parse_null(&mut self) -> Result<Option<Token<'a>>, Located<Error>> {
        let peek = self.peek_to_len(4)?;
        if peek.slice == b"null" {
            self.advance(peek);
            Ok(Some(Token::Value(SimpleValue::Null)))
        } else {
            Ok(None)
        }
    }

    fn try_parse_bool(&mut self, first: u8) -> Result<Option<Token<'a>>, Located<Error>> {
        if first == b't' {
            let peek = self.peek_to_len(4)?;
            if peek.slice == b"true" {
                self.advance(peek);
                return Ok(Some(Token::Value(SimpleValue::Bool(true))));
            }
        } else {
            let peek = self.peek_to_len(5)?;
            if peek.slice == b"false" {
                self.advance(peek);
                return Ok(Some(Token::Value(SimpleValue::Bool(false))));
            }
        };
        Ok(None)
    }

    fn parse_property_id(&mut self) -> Result<Token<'a>, Located<Error>> {
        let peek = self.peek_until_match(&[b" ", b"\n", b"\t", b"\r"], &[], false, false)?;
        if !is_valid_property_id(peek.slice) {
            return self.located_err(Error::InvalidPropertyId);
        }
        let s = core::str::from_utf8(peek.slice)
            .map_err(|e| Located::new(self.line, Error::Utf8Error(e)))?;
        self.advance(peek);
        Ok(Token::PropertyId(s))
    }

    fn parse_string(&mut self) -> Result<Token<'a>, Located<Error>> {
        let s = self._parse_string(1)?;
        Ok(Token::Value(SimpleValue::String(s)))
    }

    fn _parse_string(&mut self, skip_initial: usize) -> Result<&'a str, Located<Error>> {
        // Skip over the initial '"' character
        self.pos += skip_initial;
        let peek = self.peek_until_match(&[b"\""], &[b"\\\""], false, true)?;
        let s = core::str::from_utf8(peek.slice)
            .map_err(|e| Located::new(self.line, Error::Utf8Error(e)))?;
        self.advance(peek);
        // Skip over the terminating '"' character
        self.pos += 1;
        Ok(s)
    }

    fn parse_string_literal(&mut self) -> Result<Token<'a>, Located<Error>> {
        let s = self._parse_string_literal(0)?;
        Ok(Token::Value(SimpleValue::LiteralString(s)))
    }

    fn _parse_string_literal(&mut self, skip_initial: usize) -> Result<&'a str, Located<Error>> {
        self.pos += skip_initial;
        let peek = self.peek_until_not_match(&[b"#"])?;
        let hash_count = peek.slice.len();
        if hash_count > STRING_LITERAL_DELIM.len() - 1 {
            return self.located_err(Error::StringLiteralDelimTooLong {
                len: hash_count,
                max_len: STRING_LITERAL_DELIM.len() - 1,
            });
        }
        self.advance(peek);

        // We expect a '"' character after '#'*
        let peek = self.peek_char()?;
        if peek.slice != b"\"" {
            return self.located_err(Error::UnexpectedChar);
        }
        self.advance(peek);

        let peek =
            self.peek_until_match(&[&STRING_LITERAL_DELIM[..hash_count + 1]], &[], false, true)?;
        let s = core::str::from_utf8(peek.slice)
            .map_err(|e| Located::new(self.line, Error::Utf8Error(e)))?;
        self.advance(peek);

        // Advance past the end delimiter
        self.pos += hash_count + 1;
        Ok(s)
    }

    fn try_parse_dedent_string(&mut self) -> Result<Option<Token<'a>>, Located<Error>> {
        let peek = self.peek_chars(2)?;
        if peek.slice.len() != 2 {
            return Ok(None);
        }
        match peek.slice[1] {
            b'"' => Ok(Some(Token::Value(SimpleValue::DedentString(
                self._parse_string(2)?,
            )))),
            b'#' => Ok(Some(Token::Value(SimpleValue::DedentLiteralString(
                self._parse_string_literal(1)?,
            )))),
            _ => Ok(None),
        }
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

    // Peeks up to `len` characters.
    fn peek_to_len(&self, len: usize) -> Result<Peek<'a>, Located<Error>> {
        let mut pos = self.pos;
        let mut lines = 0;
        let mut chars = 0;
        while pos < self.src.len() && chars < len {
            let peek = self.peek_char()?;
            pos += peek.slice.len();
            chars += 1;
            lines += peek.lines;
        }
        Ok(Peek {
            slice: &self.src[self.pos..pos],
            from: self.pos,
            lines,
        })
    }

    // Peeks until we match any of the given byte strings. Includes the matching
    // slice at the end of the match. Skips over any matching byte strings in
    // `skip`.
    fn peek_until_match(
        &self,
        opts: &[&[u8]],
        skip: &[&[u8]],
        include_match: bool,
        must_match: bool,
    ) -> Result<Peek<'a>, Located<Error>> {
        let mut pos = self.pos;
        let mut match_end = pos;
        let mut lines = 0;
        let mut buf = [0_u8; MATCH_BUF_SIZE];
        let mut matched = false;
        'outer: while pos < self.src.len() {
            let peek = self.peek_char_at(pos)?;
            pos += peek.slice.len();
            match_end = pos;
            lines += peek.lines;

            // Rotate the buffer left by enough elements to inject the new slice
            // at the end of the buffer
            for i in 0..MATCH_BUF_SIZE - peek.slice.len() {
                buf[i] = buf[i + peek.slice.len()];
            }
            // Inject the new slice at the end of the buffer
            for (i, b) in peek.slice.iter().enumerate() {
                buf[MATCH_BUF_SIZE - peek.slice.len() + i] = *b;
            }
            for sk in skip {
                if &buf[MATCH_BUF_SIZE - sk.len()..] == *sk {
                    continue 'outer;
                }
            }
            for opt in opts {
                if &buf[MATCH_BUF_SIZE - opt.len()..] == *opt {
                    if !include_match {
                        match_end -= opt.len();
                    }
                    matched = true;
                    break 'outer;
                }
            }
        }
        if must_match && !matched {
            return self.located_err(Error::MissingTerminator);
        }
        Ok(Peek {
            slice: &self.src[self.pos..match_end],
            from: self.pos,
            lines,
        })
    }

    // Peeks until we don't match any of the given byte strings.
    fn peek_until_not_match(&self, opts: &[&[u8]]) -> Result<Peek<'a>, Located<Error>> {
        let mut pos = self.pos;
        let mut match_end = pos;
        let mut lines = 0;
        let mut buf = [0_u8; MATCH_BUF_SIZE];
        let mut matched = false;
        'outer: while pos < self.src.len() {
            let peek = self.peek_char_at(pos)?;
            // We match to this position to exclude the first non-matching
            // character
            match_end = pos;
            pos += peek.slice.len();
            lines += peek.lines;

            // Rotate the buffer left by enough elements to inject the new slice
            // at the end of the buffer
            for i in 0..MATCH_BUF_SIZE - peek.slice.len() {
                buf[i] = buf[i + peek.slice.len()];
            }
            // Inject the new slice at the end of the buffer
            for (i, b) in peek.slice.iter().enumerate() {
                buf[MATCH_BUF_SIZE - peek.slice.len() + i] = *b;
            }
            for opt in opts {
                if &buf[MATCH_BUF_SIZE - opt.len()..] == *opt {
                    continue 'outer;
                }
            }
            matched = true;
            break;
        }
        if !matched {
            return self.located_err(Error::MissingTerminator);
        }
        Ok(Peek {
            slice: &self.src[self.pos..match_end],
            from: self.pos,
            lines,
        })
    }

    #[inline]
    fn peek_char(&self) -> Result<Peek<'a>, Located<Error>> {
        self.peek_char_at(self.pos)
    }

    fn peek_chars(&self, n: usize) -> Result<Peek<'a>, Located<Error>> {
        let mut pos = self.pos;
        let mut chars = 0;
        let mut lines = 0;
        while pos < self.len && chars < n {
            let peek = self.peek_char_at(pos)?;
            pos += peek.slice.len();
            lines += peek.lines;
            chars += 1;
        }
        Ok(Peek {
            slice: &self.src[self.pos..pos],
            from: self.pos,
            lines,
        })
    }

    fn peek_char_at(&self, pos: usize) -> Result<Peek<'a>, Located<Error>> {
        let b = self.src[pos];
        let ch_len = UTF8_CHAR_WIDTH[b as usize] as usize;
        if self.pos + ch_len > self.src.len() {
            self.located_err(Error::IncompleteUtf8Char)
        } else {
            Ok(Peek {
                slice: &self.src[pos..pos + ch_len],
                from: pos,
                lines: if ch_len == 1 && b == b'\n' { 1 } else { 0 },
            })
        }
    }
}

struct Peek<'a> {
    slice: &'a [u8],
    from: usize,
    lines: usize,
}

impl<'a> Peek<'a> {
    // Returns whether or not the slice we've peeked starts with the given
    // prefix.
    #[inline]
    fn starts_with(&self, p: &[u8]) -> bool {
        self.slice.len() >= p.len() && p[..] == self.slice[..p.len()]
    }
}

#[inline]
fn is_valid_property_id(s: &[u8]) -> bool {
    if s.is_empty() {
        return false;
    }
    if !matches!(s[0], b'a'..=b'z' | b'A'..=b'Z' | b'_') {
        return false;
    }
    for b in &s[1..] {
        if !matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_') {
            return false;
        }
    }
    true
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
    fn peek_to_len() {
        let mut lexer = Lexer::from("a test string");
        let peek = lexer.peek_to_len(6).unwrap();
        assert_eq!(peek.slice, b"a test");
        assert_eq!(peek.from, 0);
        assert_eq!(peek.lines, 0);

        lexer.advance(peek);
        let peek = lexer.peek_to_len(4).unwrap();
        assert_eq!(peek.slice, b" str");
        assert_eq!(peek.from, 6);
        assert_eq!(peek.lines, 0);
    }

    #[test]
    fn peek_until_match() {
        const TEST_CASES: &[(&str, &str, &str)] = &[
            ("a test string", "test", "a test"),
            ("a test string", "str", "a test str"),
        ];
        for (tc, opt, expected) in TEST_CASES {
            let lexer = Lexer::from(*tc);
            let peek = lexer
                .peek_until_match(&[opt.as_bytes()], &[], true, true)
                .unwrap();
            assert_eq!(peek.slice, expected.as_bytes());
        }
    }

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

    #[test]
    fn null_lexing() {
        const TEST_CASES: &[&str] = &["null", "   null", "null   ", "\n\nnull\n\n"];
        for tc in TEST_CASES {
            let t = Lexer::from(*tc).next().unwrap().unwrap();
            assert_eq!(t, Token::Value(SimpleValue::Null));
        }
    }

    #[test]
    fn bool_lexing() {
        const TEST_CASES: &[(&str, bool)] = &[
            ("true", true),
            ("false", false),
            ("   true", true),
            ("\n\ntrue", true),
        ];
        for (tc, expected) in TEST_CASES {
            let t = Lexer::from(*tc).next().unwrap().unwrap();
            assert_eq!(t, Token::Value(SimpleValue::Bool(*expected)));
        }
    }

    #[test]
    fn property_lexing() {
        const TEST_CASES: &[(&str, &[Token])] = &[
            (
                "empty null",
                &[Token::PropertyId("empty"), Token::Value(SimpleValue::Null)],
            ),
            (
                "some-bool true\nanother-bool false",
                &[
                    Token::PropertyId("some-bool"),
                    Token::Value(SimpleValue::Bool(true)),
                    Token::PropertyId("another-bool"),
                    Token::Value(SimpleValue::Bool(false)),
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
    fn string_lexing() {
        const TEST_CASES: &[(&str, &[Token])] = &[
            ("\"string\"", &[Token::Value(SimpleValue::String("string"))]),
            (
                "a-string \"string\"",
                &[
                    Token::PropertyId("a-string"),
                    Token::Value(SimpleValue::String("string")),
                ],
            ),
            (
                "literal #\"a literal string\"#",
                &[
                    Token::PropertyId("literal"),
                    Token::Value(SimpleValue::LiteralString("a literal string")),
                ],
            ),
            (
                "literal ##\"a #\"literal\"# string\"##",
                &[
                    Token::PropertyId("literal"),
                    Token::Value(SimpleValue::LiteralString("a #\"literal\"# string")),
                ],
            ),
            (
                "lit #\"literal\"#\nstr \"string\"",
                &[
                    Token::PropertyId("lit"),
                    Token::Value(SimpleValue::LiteralString("literal")),
                    Token::PropertyId("str"),
                    Token::Value(SimpleValue::String("string")),
                ],
            ),
            (
                "d\"dedent\"",
                &[Token::Value(SimpleValue::DedentString("dedent"))],
            ),
            (
                "dedent d\"dedent\"",
                &[
                    Token::PropertyId("dedent"),
                    Token::Value(SimpleValue::DedentString("dedent")),
                ],
            ),
            (
                "d#\"dedent literal\"#",
                &[Token::Value(SimpleValue::DedentLiteralString(
                    "dedent literal",
                ))],
            ),
            (
                "dedent d##\"dedent #\"literal\"#\"##",
                &[
                    Token::PropertyId("dedent"),
                    Token::Value(SimpleValue::DedentLiteralString("dedent #\"literal\"#")),
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
}
