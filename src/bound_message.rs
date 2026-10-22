//! The sentence one violated bound is reported in, built once and handed to every emitter.

#[cfg(feature = "swift")]
use core::fmt::Write as _;

#[cfg(feature = "dart")]
use crate::features::dart::dart_escaped;
#[cfg(feature = "kotlin")]
use crate::features::kotlin::kotlin_template_text;
#[cfg(any(
    feature = "serde",
    feature = "zod",
    feature = "dart",
    feature = "kotlin",
    feature = "swift"
))]
use crate::features::model_schema_prop::LiteralValue;
#[cfg(any(
    feature = "serde",
    feature = "zod",
    feature = "dart",
    feature = "kotlin",
    feature = "swift"
))]
use crate::field_type::format_number_literal;
#[cfg(any(
    feature = "serde",
    feature = "zod",
    feature = "dart",
    feature = "kotlin",
    feature = "swift"
))]
use crate::utils::escape_js_double_quoted;
#[cfg(feature = "serde")]
use quote::quote;

/// What every surface but Rust answers for a set whose payload repeats an item: Rust's read keeps
/// one item of it.
#[cfg(any(
    feature = "zod",
    feature = "dart",
    feature = "kotlin",
    feature = "swift"
))]
pub const REPEATED_ITEM: &str = "repeats an item: a set holds each item once";

/// Every stem a violated bound's sentence can begin with. A refusal that reaches a reader as text
/// is this crate's own only if it opens with one of these.
#[cfg(feature = "serde")]
pub const VIOLATION_STEMS: [&str; 6] = [
    "does not match pattern '",
    "not the literal ",
    "too large: maximum is ",
    "too long: maximum length is ",
    "too short: minimum length is ",
    "too small: minimum is ",
];

/// A bound a declaration can carry, at the one value it was written with. `MaxItems` and
/// `MinItems` are `maxLength` and `minLength` written on a list, whose items they count, and
/// `Literal` is the one value a `literal` admits.
#[cfg(any(
    feature = "serde",
    feature = "zod",
    feature = "dart",
    feature = "kotlin",
    feature = "swift"
))]
#[derive(Clone, Copy)]
pub enum Bound<'written> {
    Literal(&'written LiteralValue),
    MaxItems(usize),
    MaxLength(usize),
    Maximum(f64),
    MinItems(usize),
    MinLength(usize),
    Minimum(f64),
    Pattern(&'written str),
}

/// How a sentence measures the value that broke the bound, for the bounds that quote one back.
#[cfg(any(
    feature = "serde",
    feature = "zod",
    feature = "dart",
    feature = "kotlin",
    feature = "swift"
))]
#[derive(Clone, Copy)]
enum Observed {
    Count,
    Length,
    Value,
}

#[cfg(any(
    feature = "serde",
    feature = "zod",
    feature = "dart",
    feature = "kotlin",
    feature = "swift"
))]
impl Bound<'_> {
    /// What the sentence quotes back, or `None` for a bound whose report names only itself.
    const fn observed(self) -> Option<Observed> {
        match self {
            Self::MaxItems(_) | Self::MinItems(_) => Some(Observed::Count),
            Self::MaxLength(_) | Self::MinLength(_) => Some(Observed::Length),
            Self::Literal(_) | Self::Maximum(_) | Self::Minimum(_) => Some(Observed::Value),
            Self::Pattern(_) => None,
        }
    }

    /// The bound in words, without the value that broke it. Every number here is rendered at
    /// expansion time, by the same `Display` that writes the bound into the Zod check beside it,
    /// so the two spell it identically without either having to know how the other did. A literal
    /// is written as the Zod schema writes it: a string between double quotes.
    fn stated(self) -> String {
        let stem = self.stem();
        match self {
            Self::Literal(LiteralValue::Bool(value)) => format!("{stem}{value}"),
            Self::Literal(LiteralValue::Number(value)) => {
                format!("{stem}{}", format_number_literal(*value))
            }
            Self::Literal(LiteralValue::Str(text)) => {
                format!("{stem}\"{}\"", escape_js_double_quoted(text))
            }
            Self::MaxItems(len)
            | Self::MaxLength(len)
            | Self::MinItems(len)
            | Self::MinLength(len) => format!("{stem}{len}"),
            Self::Maximum(bound) | Self::Minimum(bound) => format!("{stem}{bound}"),
            Self::Pattern(pattern) => format!("{stem}{pattern}'"),
        }
    }

    /// The words this bound opens with, before the value it was written at. Spelled here rather
    /// than inside [`Self::stated`] so that [`VIOLATION_STEMS`] and the sentence itself cannot
    /// drift: a reader tells one of these sentences from a deserializer's own by that list alone.
    const fn stem(self) -> &'static str {
        match self {
            Self::Literal(_) => "not the literal ",
            Self::MaxItems(_) | Self::MaxLength(_) => "too long: maximum length is ",
            Self::Maximum(_) => "too large: maximum is ",
            Self::MinItems(_) | Self::MinLength(_) => "too short: minimum length is ",
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

/// The Kotlin string a generated `validate()` reports `bound` with: the sentence
/// [`rust_violation`] writes. `place` is the name the value is reported under, already written as
/// the text of a Kotlin string, and `observed` the Kotlin expression the sentence quotes back.
#[cfg(feature = "kotlin")]
pub fn kotlin_violation(bound: Bound<'_>, place: Option<&str>, observed: &str) -> String {
    let stated = kotlin_template_text(&bound.stated());
    let lead = place.map_or_else(String::new, |named| format!("'{named}': "));
    if bound.observed().is_some() {
        format!("\"{lead}{stated}, got ${{{observed}}}\"")
    } else {
        format!("\"{lead}{stated}\"")
    }
}

/// The Kotlin string a generated `validate()` reports a set that repeats an item with, under
/// `place`, already written as the text of a Kotlin string.
#[cfg(feature = "kotlin")]
pub fn kotlin_repeated(place: Option<&str>) -> String {
    let lead = place.map_or_else(String::new, |named| format!("'{named}': "));
    format!("\"{lead}{}\"", kotlin_template_text(REPEATED_ITEM))
}

/// The parameter object a generated Zod check carries for `bound`, which is what makes it report
/// the sentence [`rust_violation`] writes instead of zod's own.
///
/// A bound that quotes the value back takes a function, that being the only form zod hands the
/// value to. The value arrives as `unknown` and goes through `String` rather than straight into the
/// template, so the expression type-checks under `--strict` whatever the schema was handed. A list
/// is counted behind `Array.isArray`, which is what narrows it.
#[cfg(feature = "zod")]
pub fn zod_error_arg(bound: Bound<'_>) -> String {
    let stated = bound.stated();
    // A `pattern` and a string `literal` carry text somebody else wrote, so the sentence is
    // escaped for the string or the template it is written into.
    match bound.observed() {
        Some(Observed::Count) => format!(
            "{{ error: (issue) => `{}, got ${{Array.isArray(issue.input) ? issue.input.length : 0}}` }}",
            js_template_text(&stated)
        ),
        Some(Observed::Length) => format!(
            "{{ error: (issue) => `{}, got ${{String(issue.input).length}}` }}",
            js_template_text(&stated)
        ),
        Some(Observed::Value) => format!(
            "{{ error: (issue) => `{}, got ${{String(issue.input)}}` }}",
            js_template_text(&stated)
        ),
        None => format!("{{ error: \"{}\" }}", escape_js_double_quoted(&stated)),
    }
}

/// `text` as it is written between the backticks of a JavaScript template literal, which reads it
/// back exactly.
#[cfg(feature = "zod")]
fn js_template_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('$', "\\$")
}

/// The check a set's Zod array is written with: an array whose parsed items write one JSON text
/// twice, each `bigint` written as its digits as a client sends it, is refused with
/// [`REPEATED_ITEM`]. The issue carries `continue: true`, as each issue a written check copies
/// does.
#[cfg(feature = "zod")]
pub fn zod_repeat_check() -> String {
    format!(
        ".check((ctx) => {{\n  const seen = new Set<string>();\n  for (const item of ctx.value) \
         {{\n    const text = JSON.stringify(item, (_, held) => (typeof held === \"bigint\" ? \
         held.toString() : held));\n    if (seen.has(text)) {{\n      \
         ctx.issues.push({{ code: \"custom\", message: \"{}\", input: ctx.value, continue: true \
         }});\n      return;\n    }}\n    seen.add(text);\n  }}\n}})",
        escape_js_double_quoted(REPEATED_ITEM)
    )
}

/// The Dart string a generated `validate()` lists for a set that repeats an item, under `place`
/// as a Dart string holds it between two `"`.
#[cfg(feature = "dart")]
pub fn dart_repeated(place: Option<&str>) -> String {
    let led = place.map_or_else(String::new, |named| format!("'{named}': "));
    format!("\"{led}{}\"", dart_escaped(REPEATED_ITEM, '"'))
}

/// The Dart string a generated `validate()` lists for `bound`: the sentence [`rust_violation`]
/// writes.
///
/// `place` is what the message opens with between its quotes, as a Dart string holds it, or
/// `None` for a value reported at no place. `observed` is the Dart expression the sentence quotes
/// back, read where the string is written.
#[cfg(feature = "dart")]
pub fn dart_violation(bound: Bound<'_>, place: Option<&str>, observed: &str) -> String {
    let stated = dart_escaped(&bound.stated(), '"');
    let led = place.map_or_else(String::new, |named| format!("'{named}': "));
    match bound.observed() {
        Some(_) => format!("\"{led}{stated}, got ${{{observed}}}\""),
        None => format!("\"{led}{stated}\""),
    }
}

/// `text` as it is written between the `"` delimiters of a Swift string literal.
#[cfg(feature = "swift")]
pub fn swift_literal_text(text: &str) -> String {
    let mut written = String::with_capacity(text.len());
    for letter in text.chars() {
        match letter {
            '\\' => written.push_str("\\\\"),
            '"' => written.push_str("\\\""),
            '\n' => written.push_str("\\n"),
            '\r' => written.push_str("\\r"),
            '\t' => written.push_str("\\t"),
            other if other.is_control() => {
                write!(written, "\\u{{{:X}}}", u32::from(other)).unwrap();
            }
            other => written.push(other),
        }
    }
    written
}

/// The Swift string literal a generated `validate()` appends to its report for a set that repeats
/// an item. `place` is already the text of a Swift string literal, an index or a key interpolated
/// into it.
#[cfg(feature = "swift")]
pub fn swift_repeated(place: Option<&str>) -> String {
    let stated = swift_literal_text(REPEATED_ITEM);
    place.map_or_else(
        || format!("\"{stated}\""),
        |under| format!("\"'{under}': {stated}\""),
    )
}

/// The Swift string literal a generated `validate()` appends to its report for `bound`: the
/// sentence `rust_violation` writes.
///
/// `place` is the name the value is reported under, or `None` for a value reported under none.
/// `observed` is the Swift expression the sentence quotes back.
#[cfg(feature = "swift")]
pub fn swift_violation(bound: Bound<'_>, place: Option<&str>, observed: &str) -> String {
    let stated = swift_literal_text(&bound.stated());
    let named = place.map(swift_literal_text);
    match (named, bound.observed()) {
        (Some(under), Some(_)) => format!("\"'{under}': {stated}, got \\({observed})\""),
        (Some(under), None) => format!("\"'{under}': {stated}\""),
        (None, Some(_)) => format!("\"{stated}, got \\({observed})\""),
        (None, None) => format!("\"{stated}\""),
    }
}

#[cfg(test)]
#[cfg(any(feature = "serde", feature = "zod", feature = "dart"))]
mod tests;
