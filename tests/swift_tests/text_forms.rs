//! What the Swift emitter writes for a text serde reads in one form alone: the read of a `char`
//! and a date, time and date and time, which refuses a text Zod refuses, and their write, which
//! writes the form serde writes, each a private static function of the type that calls it, over
//! the models of `kotlin_tests/text_forms_models.rs`.

use super::text_forms_models::{chronometer_swift, instant_log_swift, stamp_log_swift};

/// A date as `z.iso.date()` reads it, as Swift source writes it, each digit an ASCII one.
const DATE: &str = concat!(
    "(?:(?:[0-9][0-9][2468][048]|[0-9][0-9][13579][26]|[0-9][0-9]0[48]|[02468][048]00|",
    "[13579][26]00)-02-29|[0-9]{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12][0-9]|3[01])|",
    "(?:0[469]|11)-(?:0[1-9]|[12][0-9]|30)|(?:02)-(?:0[1-9]|1[0-9]|2[0-8])))"
);

/// The hour and the minute of a time, as Swift source writes it.
const HOUR_MINUTE: &str = "(?:[01][0-9]|2[0-3]):[0-5][0-9]";

/// A time as serde writes it, as Swift source writes it.
const TIME_SENT: &str = r"(?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9](?:\\.[0-9]+)?";

/// The test a text `held` passes where it matches `pattern` whole.
fn matching(held: &str, pattern: &str) -> String {
    format!("{held}.range(of: \"^(?:{pattern})\\\\z\", options: .regularExpression) != nil")
}

/// The function that reads a text of the form `named`, held to `test`.
fn reader(named: &str, test: &str, expected: &str) -> String {
    format!(
        "private static func swiftSchemaRead{named}(_ text: String, codingPath: [CodingKey]) \
         throws -> String {{ guard {test} else {{ throw DecodingError.dataCorrupted(\
         DecodingError.Context(codingPath: codingPath, debugDescription: \"expected {expected}, \
         found \\(swiftSchemaQuoted(text))\")) }}; return text }}; "
    )
}

/// The function that writes a text of the form `named`: its `steps`, then `sent` held to `test`.
fn writer(named: &str, steps: &str, test: &str, expected: &str) -> String {
    format!(
        "private static func swiftSchemaSent{named}(_ text: String, codingPath: [CodingKey]) \
         throws -> String {{ {steps}guard {test} else {{ throw EncodingError.invalidValue(text, \
         EncodingError.Context(codingPath: codingPath, debugDescription: \"expected {expected}, \
         found \\(swiftSchemaQuoted(text))\")) }}; return sent }}; "
    )
}

/// Each of `written` in `swift`.
fn holds(swift: &str, written: &[String]) {
    for text in written {
        assert!(swift.contains(text.as_str()), "{text}\nin: {swift}");
    }
}

/// A `char` and a date are read and written through the functions of their form, a map's key
/// among them, which the type declares for itself.
#[test]
fn a_char_and_a_date_are_read_and_written_through_the_functions_of_their_form() {
    let letter = "text.unicodeScalars.count == 1";
    holds(
        &chronometer_swift::swift_definition(),
        &[
            "self.day = try Self.swiftSchemaReadDate(wireDay, codingPath: decoder.codingPath); "
                .to_owned(),
            "let wireLetter = (try Self.swiftSchemaSentChar(self.letter, \
             codingPath: encoder.codingPath)); "
                .to_owned(),
            "let key = try Self.swiftSchemaReadChar(wireKey, \
             codingPath: container.codingPath + [codingKey]); "
                .to_owned(),
            "extension Chronometer { private static func swiftSchemaQuoted(".to_owned(),
            reader("Date", &matching("text", DATE), "a date"),
            writer(
                "Date",
                "let sent = text; ",
                &matching("sent", DATE),
                "a date",
            ),
            reader("Char", letter, "a character"),
            writer(
                "Char",
                "let sent = text; ",
                "sent.unicodeScalars.count == 1",
                "a character",
            ),
        ],
    );
}

/// A date and time is written without the `Z` serde refuses, with the seconds a text leaves out,
/// then held to the form serde writes.
#[test]
fn a_date_and_time_is_written_without_its_z_and_with_its_seconds() {
    holds(
        &stamp_log_swift::swift_definition(),
        &[writer(
            "Stamp",
            &format!(
                "var sent = text; if sent.hasSuffix(\"Z\") {{ sent.removeLast() }}; if {} {{ sent \
                 += \":00\" }}; ",
                matching("sent", &format!("({DATE}T{HOUR_MINUTE})()"))
            ),
            &matching("sent", &format!("{DATE}T{TIME_SENT}")),
            "a date and time",
        )],
    );
}

/// A date and time with its zone is a `Date`, read by either formatter and written by the one with
/// a fraction of a second.
#[test]
fn a_date_and_time_with_its_zone_is_read_with_or_without_a_fraction() {
    holds(
        &instant_log_swift::swift_definition(),
        &[
            "self.instant = try Self.swiftSchemaParseIso8601Date(wireInstant, \
             codingPath: decoder.codingPath); "
                .to_owned(),
            "let wireInstant = Self.swiftSchemaFormatIso8601Date(self.instant); ".to_owned(),
            "guard let date = swiftSchemaIso8601Formatter.date(from: text) ?? \
             swiftSchemaWholeIso8601Formatter.date(from: text) else { "
                .to_owned(),
        ],
    );
}
