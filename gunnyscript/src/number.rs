//! Numeric values in GunnyScript.

use fixed::types::I64F64;

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
