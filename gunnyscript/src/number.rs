//! Numeric values in GunnyScript.

use core::str::FromStr;

use fixed::types::I64F64;

use crate::ParseError;

/// Fixed-point number for fractional representation. This is a 128-bit number,
/// with 64 bits reserved for the whole number part and 64 bits reserved for the
/// fractional part.
pub type Fixed = I64F64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Number {
    Unsigned(u64),
    Signed(i64),
    Fixed(Fixed),
}

impl Number {
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Unsigned(u) => Some(*u),
            Self::Signed(i) => {
                if *i >= 0 {
                    Some(*i as u64)
                } else {
                    None
                }
            }
            Self::Fixed(f) => {
                if f.is_positive() && f.frac().is_zero() {
                    Some(f.to_num())
                } else {
                    None
                }
            }
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Unsigned(u) => {
                if *u < i64::MAX as u64 {
                    Some(*u as i64)
                } else {
                    None
                }
            }
            Self::Signed(i) => Some(*i),
            Self::Fixed(f) => {
                if f.frac().is_zero() {
                    Some(f.to_num())
                } else {
                    None
                }
            }
        }
    }

    pub fn as_fixed(&self) -> Fixed {
        match self {
            Self::Unsigned(u) => Fixed::from_num(*u),
            Self::Signed(i) => Fixed::from_num(*i),
            Self::Fixed(f) => *f,
        }
    }
}

impl FromStr for Number {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("0x") {
            parse_hex(s.strip_prefix("0x").unwrap())
        } else if s.contains('.') {
            parse_fixed(s)
        } else if s.starts_with('-') {
            parse_signed(s)
        } else if s.starts_with('0') && s.len() > 1 {
            parse_octal(s)
        } else {
            parse_unsigned(s)
        }
    }
}

#[inline]
fn parse_hex(s: &str) -> Result<Number, ParseError> {
    let value = u64::from_str_radix(s, 16).map_err(ParseError::InvalidHexNumber)?;
    Ok(Number::Unsigned(value))
}

#[inline]
fn parse_signed(s: &str) -> Result<Number, ParseError> {
    let value = s.parse::<i64>().map_err(ParseError::InvalidSignedNumber)?;
    Ok(Number::Signed(value))
}

#[inline]
fn parse_unsigned(s: &str) -> Result<Number, ParseError> {
    let value = s
        .parse::<u64>()
        .map_err(ParseError::InvalidUnsignedNumber)?;
    Ok(Number::Unsigned(value))
}

#[inline]
fn parse_fixed(s: &str) -> Result<Number, ParseError> {
    let value = Fixed::from_str(s).map_err(ParseError::InvalidFixedPointNumber)?;
    Ok(Number::Fixed(value))
}

#[inline]
fn parse_octal(s: &str) -> Result<Number, ParseError> {
    let value = u64::from_str_radix(s, 8).map_err(ParseError::InvalidOctalNumber)?;
    Ok(Number::Unsigned(value))
}
