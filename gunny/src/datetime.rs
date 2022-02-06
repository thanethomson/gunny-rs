use std::str::FromStr;

use time::format_description::well_known::Rfc3339;

use crate::Error;

/// A simple date object, encapsulating a year, month and day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(time::Date);

impl FromStr for Date {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0.format(&Rfc3339).map_err(|_| std::fmt::Error)?
        )
    }
}

impl From<Date> for time::Date {
    fn from(d: Date) -> Self {
        d.0
    }
}

impl From<time::Date> for Date {
    fn from(d: time::Date) -> Self {
        Self(d)
    }
}

/// Representation of a date and time with time zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime(time::OffsetDateTime);

impl FromStr for DateTime {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl std::fmt::Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0.format(&Rfc3339).map_err(|_| std::fmt::Error)?
        )
    }
}

impl From<DateTime> for time::OffsetDateTime {
    fn from(dt: DateTime) -> Self {
        dt.0
    }
}

impl From<time::OffsetDateTime> for DateTime {
    fn from(dt: time::OffsetDateTime) -> Self {
        Self(dt)
    }
}
