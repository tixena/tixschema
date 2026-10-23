//! A `char` and a chrono date, time and date and time, which the Swift client holds as a
//! `String`: read as the Zod schema of their form reads them, refusing a `char` of other than one
//! code point as serde does, and sent in the form serde writes ([`TextForm`]). The read throws
//! `DecodingError.dataCorrupted` and the write `EncodingError.invalidValue`, each from a private
//! static function of the type whose code calls it.

use core::fmt::Write as _;

use crate::bound_message::swift_literal_text;
use crate::features::text_forms::TextForm;
use crate::field_type::FieldDef;

/// The form of a value typed `field` the Swift client holds as a `String`: a `DateTime<Tz>` is a
/// `Date`, and has none.
pub fn held_as_text(field: &FieldDef) -> Option<TextForm> {
    TextForm::of(field).filter(|form| match form {
        TextForm::Char => true,
        #[cfg(feature = "chrono")]
        TextForm::Date | TextForm::DateTime | TextForm::Time => true,
        #[cfg(feature = "chrono")]
        TextForm::Zoned => false,
    })
}

/// What the functions of `form` are named after.
const fn named(form: TextForm) -> &'static str {
    match form {
        TextForm::Char => "Char",
        #[cfg(feature = "chrono")]
        TextForm::Date => "Date",
        #[cfg(feature = "chrono")]
        TextForm::DateTime => "Stamp",
        #[cfg(feature = "chrono")]
        TextForm::Time => "Time",
        #[cfg(feature = "chrono")]
        TextForm::Zoned => "Zoned",
    }
}

/// The call that reads `text`, a `String` the decoder read, as `form`.
pub fn read_call(form: TextForm, text: &str, coding_path: &str) -> String {
    format!(
        "try Self.swiftSchemaRead{}({text}, codingPath: {coding_path})",
        named(form)
    )
}

/// The call that writes `text`, a `String` of `form`, as the client sends it.
pub fn sent_call(form: TextForm, text: &str) -> String {
    sent_call_at(form, text, "encoder.codingPath")
}

/// [`sent_call`], under the coding path `coding_path` names.
pub fn sent_call_at(form: TextForm, text: &str, coding_path: &str) -> String {
    format!(
        "(try Self.swiftSchemaSent{}({text}, codingPath: {coding_path}))",
        named(form)
    )
}

/// The Swift literal of the pattern that matches the whole of a text against `pattern`, each
/// digit an ASCII one: Foundation's `\d` takes a digit of any script.
fn whole(pattern: &str) -> String {
    format!(
        "\"{}\"",
        swift_literal_text(&format!("^(?:{})\\z", pattern.replace(r"\d", "[0-9]")))
    )
}

/// The condition a text `named` meets where it is one code point, or matches `pattern` whole.
fn held(pattern: Option<String>, text: &str) -> String {
    pattern.map_or_else(
        || format!("{text}.unicodeScalars.count == 1"),
        |matching| {
            format!(
                "{text}.range(of: {}, options: .regularExpression) != nil",
                whole(&matching)
            )
        },
    )
}

/// The private static functions that read and send each of `forms`, and the text a refusal quotes
/// a value in. Nothing where `forms` is empty.
pub fn helpers(forms: &[TextForm]) -> String {
    if forms.is_empty() {
        return String::new();
    }
    let mut written = String::from(
        "private static func swiftSchemaQuoted(_ text: String) -> String { \
         String(decoding: (try? JSONSerialization.data(withJSONObject: text, options: \
         [.fragmentsAllowed, .withoutEscapingSlashes])) ?? Data(), as: UTF8.self) }; ",
    );
    for &form in forms {
        let expected = form.expected();
        let mut steps = String::new();
        if form.unzoned() {
            steps.push_str("if sent.hasSuffix(\"Z\") { sent.removeLast() }; ");
        }
        // The second group of a form Swift holds as text is empty: the seconds go at the end.
        if let Some(pattern) = form.unsecond() {
            write!(
                steps,
                "if sent.range(of: {}, options: .regularExpression) != nil {{ sent += \":00\" }}; ",
                whole(&pattern)
            )
            .unwrap();
        }
        let binding = if steps.is_empty() { "let" } else { "var" };
        write!(
            written,
            "private static func swiftSchemaRead{name}(_ text: String, codingPath: [CodingKey]) \
             throws -> String {{ guard {read} else {{ throw DecodingError.dataCorrupted(\
             DecodingError.Context(codingPath: codingPath, debugDescription: \"expected \
             {expected}, found \\(swiftSchemaQuoted(text))\")) }}; return text }}; \
             private static func swiftSchemaSent{name}(_ text: String, codingPath: [CodingKey]) \
             throws -> String {{ {binding} sent = text; {steps}guard {sent} else {{ throw \
             EncodingError.invalidValue(text, EncodingError.Context(codingPath: codingPath, \
             debugDescription: \"expected {expected}, found \\(swiftSchemaQuoted(text))\")) }}; \
             return sent }}; ",
            name = named(form),
            read = held(form.read(), "text"),
            sent = held(form.sent(), "sent"),
        )
        .unwrap();
    }
    written
}
