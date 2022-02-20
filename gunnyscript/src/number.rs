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
