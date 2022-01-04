//! Template-related utility methods.

use handlebars::{Context, Handlebars, Helper, HelperResult, Output, RenderContext};

/// Parses a string as a date and formats it according to a formatting rule.
///
/// Usage:
///
/// ```handlebars
/// {{ format_date "2022-01-01" "[month repr:long] [day], [year] }}
/// ```
///
/// Produces `January 1, 2022`.
///
/// The formatting rule is defined by the
/// [`time`](https://crates.io/crates/time) crate. See [the
/// docs](https://time-rs.github.io/book/api/format-description.html) for more
/// details.
pub fn format_date(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    rc: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    todo!()
}

/// Parses a string as a timestamp (with date and time) and formats it according
/// to a formatting rule.
///
/// Usage:
///
/// ```handlebars
/// {{ format_date_time "2022-01-01T14:00" "[month repr:long] [day], [year] at [hour repr:24]:[minute padding:zero]" }}
/// ```
///
/// Produces `January 1, 2022 at 14:00`.
///
/// The formatting rule is defined by the
/// [`time`](https://crates.io/crates/time) crate. See [the
/// docs](https://time-rs.github.io/book/api/format-description.html) for more
/// details.
pub fn format_date_time(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    rc: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    todo!()
}

/// Pad a string with a given character or string until it meets the specified
/// length.
///
/// Usage:
///
/// ```handlebars
/// {{ pad "2" "0" 2 }}
/// ```
///
/// Produces `02`. Parameters are in the format
/// `{{ pad string paddingCharOrString desiredMinLength }}`
pub fn pad(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    rc: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    todo!()
}
