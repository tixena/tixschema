//! The sentence one violated bound is reported in, built once and handed to both emitters.

#[cfg(feature = "zod")]
use crate::utils::escape_js_double_quoted;
#[cfg(feature = "serde")]
use quote::quote;

/// Every stem a violated bound's sentence can begin with. A refusal that reaches a reader as text
/// is this crate's own only if it opens with one of these.
#[cfg(feature = "serde")]
pub const VIOLATION_STEMS: [&str; 5] = [
    "does not match pattern '",
    "too large: maximum is ",
    "too long: maximum length is ",
    "too short: minimum length is ",
    "too small: minimum is ",
];

/// A bound a declaration can carry, at the one value it was written with.
#[cfg(any(feature = "serde", feature = "zod"))]
#[derive(Clone, Copy)]
pub enum Bound<'pattern> {
    MaxLength(usize),
    Maximum(f64),
    MinLength(usize),
    Minimum(f64),
    Pattern(&'pattern str),
}

/// How a sentence measures the value that broke the bound, for the bounds that quote one back.
#[cfg(any(feature = "serde", feature = "zod"))]
#[derive(Clone, Copy)]
enum Observed {
    Length,
    Value,
}

#[cfg(any(feature = "serde", feature = "zod"))]
impl Bound<'_> {
    /// What the sentence quotes back, or `None` for a bound whose report names only itself.
    const fn observed(self) -> Option<Observed> {
        match self {
            Self::MaxLength(_) | Self::MinLength(_) => Some(Observed::Length),
            Self::Maximum(_) | Self::Minimum(_) => Some(Observed::Value),
            Self::Pattern(_) => None,
        }
    }

    /// The bound in words, without the value that broke it. Every number here is rendered at
    /// expansion time, by the same `Display` that writes the bound into the Zod check beside it,
    /// so the two spell it identically without either having to know how the other did.
    fn stated(self) -> String {
        let stem = self.stem();
        match self {
            Self::MaxLength(len) | Self::MinLength(len) => format!("{stem}{len}"),
            Self::Maximum(bound) | Self::Minimum(bound) => format!("{stem}{bound}"),
            Self::Pattern(pattern) => format!("{stem}{pattern}'"),
        }
    }

    /// The words this bound opens with, before the value it was written at. Spelled here rather
    /// than inside [`Self::stated`] so that [`VIOLATION_STEMS`] and the sentence itself cannot
    /// drift: a reader tells one of these sentences from a deserializer's own by that list alone.
    const fn stem(self) -> &'static str {
        match self {
            Self::MaxLength(_) => "too long: maximum length is ",
            Self::Maximum(_) => "too large: maximum is ",
            Self::MinLength(_) => "too short: minimum length is ",
            Self::Minimum(_) => "too small: minimum is ",
            Self::Pattern(_) => "does not match pattern '",
        }
    }
}

/// The `format!` a generated validator pushes onto its report for `bound`.
///
/// `field` is the member the bound was declared on, or `None` where the checked value is not a
/// member of anything — a brand, whose name is written by whatever holds it. `observed` is the
/// expression the sentence quotes back, read in the validator's own scope: the length of the value
/// it measured, or the value it compared.
#[cfg(feature = "serde")]
pub fn rust_violation(
    bound: Bound<'_>,
    field: Option<&str>,
    observed: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let stated = bound.stated();
    match (field, bound.observed()) {
        (Some(named), Some(_)) => quote! {
            format!("'{}': {}, got {}", #named, #stated, #observed)
        },
        (Some(named), None) => quote! {
            format!("'{}': {}", #named, #stated)
        },
        (None, Some(_)) => quote! {
            format!("{}, got {}", #stated, #observed)
        },
        (None, None) => quote! {
            #stated.to_owned()
        },
    }
}

/// The parameter object a generated Zod check carries for `bound`, which is what makes it report
/// the sentence [`rust_violation`] writes instead of zod's own.
///
/// A bound that quotes the value back takes a function, that being the only form zod hands the
/// value to. The value arrives as `unknown` and goes through `String` rather than straight into the
/// template, so the expression type-checks under `--strict` whatever the schema was handed.
#[cfg(feature = "zod")]
pub fn zod_error_arg(bound: Bound<'_>) -> String {
    let stated = bound.stated();
    // Only a `pattern` carries text somebody else wrote, and it is the one rendered into a quoted
    // string; the rest are built here out of a fixed vocabulary and digits.
    match bound.observed() {
        Some(Observed::Length) => {
            format!("{{ error: (issue) => `{stated}, got ${{String(issue.input).length}}` }}")
        }
        Some(Observed::Value) => {
            format!("{{ error: (issue) => `{stated}, got ${{String(issue.input)}}` }}")
        }
        None => format!("{{ error: \"{}\" }}", escape_js_double_quoted(&stated)),
    }
}

#[cfg(test)]
#[cfg(any(feature = "serde", feature = "zod"))]
mod tests;
