//! The Dart expressions that hold a value the Dart client keeps in a wider type than the Rust one
//! to what serde reads: a `char` and a chrono date or time, kept as a `String`, read as the Zod
//! schema of their form reads them and sent in the form serde writes ([`TextForm`]); an integer
//! narrower than a Dart `int`, and an `f32`, kept as a `double`, read and sent within the range
//! of the Rust type. Each refuses with a `FormatException`, written inline, as every refusal of the
//! Dart client is.

use super::declared;
use crate::features::text_forms::TextForm;
use crate::field_type::{F32_READ_BOUND, FieldDefType};

/// The Dart pattern that matches the whole of a text against `pattern`.
fn whole(pattern: &str) -> String {
    format!("RegExp(r'^(?:{pattern})$')")
}

/// The condition a text `named` meets where it is one code point, or matches `pattern` whole.
fn held(pattern: Option<String>, named: &str) -> String {
    pattern.map_or_else(
        || format!("{named}.runes.length == 1"),
        |matching| format!("{}.hasMatch({named})", whole(&matching)),
    )
}

/// `text`, a `String` the Dart client read as `form`: itself where the Zod schema of the form
/// reads it, and refused otherwise.
pub fn text_read(form: TextForm, text: &str) -> String {
    let named = declared("text");
    format!(
        "(switch ({text}) {{ final {named} when {} => {named}, final {named} => throw \
         FormatException('expected {}, found ${{jsonEncode({named})}}') }})",
        held(form.read(), &named),
        form.expected()
    )
}

/// What the Dart client sends of `value`, a `String` it holds as `form`: the form serde writes,
/// its `Z` dropped and its seconds written where the form says, and a refusal of a text no such
/// rewrite turns into it.
pub fn text_sent(form: TextForm, value: &str) -> String {
    let zoneless = if form.unzoned() {
        format!("({value}.endsWith('Z') ? {value}.substring(0, {value}.length - 1) : {value})")
    } else {
        value.to_owned()
    };
    let current = form.unsecond().map_or_else(
        || zoneless.clone(),
        |pattern| {
            let unzoned = declared("unzoned");
            let found = declared("found");
            format!(
                "switch ({zoneless}) {{ final {unzoned} => switch ({}.firstMatch({unzoned})) {{ \
                 final {found}? => '${{{found}[1]}}:00${{{found}[2] ?? ''}}', _ => {unzoned} }} }}",
                whole(&pattern)
            )
        },
    );
    let sent = declared("sent");
    format!(
        "(switch ({current}) {{ final {sent} when {} => {sent}, _ => throw \
         FormatException('expected {}, found ${{jsonEncode({value})}}') }})",
        held(form.sent(), &sent),
        form.expected()
    )
}

/// `number`, a Dart `int` or `double` of `field_type`, held to what serde reads of the Rust type:
/// an integer within its range, an `f32` below its bound. `None` for a type a Dart number holds
/// whole.
pub fn number_held(field_type: &FieldDefType, number: &str) -> Option<String> {
    let named = declared("number");
    let (condition, expected) = if matches!(field_type, FieldDefType::F32) {
        (
            format!("{named}.abs() < {F32_READ_BOUND}"),
            "a 32-bit float".to_owned(),
        )
    } else {
        match field_type.integer_range() {
            (Some(least), Some(most)) => (
                format!("{named} >= {least} && {named} <= {most}"),
                format!("an integer from {least} to {most}"),
            ),
            (Some(least), None) => (
                format!("{named} >= {least}"),
                format!("an integer of at least {least}"),
            ),
            (None, Some(most)) => (
                format!("{named} <= {most}"),
                format!("an integer of at most {most}"),
            ),
            (None, None) => return None,
        }
    };
    Some(format!(
        "(switch ({number}) {{ final {named} when {condition} => {named}, final {named} => throw \
         FormatException('expected {expected}, found ${named}') }})"
    ))
}
