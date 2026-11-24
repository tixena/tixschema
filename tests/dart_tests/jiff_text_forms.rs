//! What the Dart emitter writes for a text serde reads in one form alone: the read of a `char` and
//! a date, time and date and time, which refuses a text Zod refuses, and their write, which writes
//! the form serde writes, over the models of `kotlin_tests/jiff_text_forms_models.rs`.

use super::text_forms_models::{chronometer_dart, instant_log_dart, stamp_log_dart, time_log_dart};

/// A date as `z.iso.date()` reads it, as Dart source writes it.
const DATE: &str = concat!(
    r"(?:(?:\d\d[2468][048]|\d\d[13579][26]|\d\d0[48]|[02468][048]00|[13579][26]00)-02-29|",
    r"\d{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12]\d|3[01])|(?:0[469]|11)-(?:0[1-9]|[12]\d|30)|",
    r"(?:02)-(?:0[1-9]|1\d|2[0-8])))"
);

/// The hour and the minute of a time, as Dart source writes it.
const HOUR_MINUTE: &str = r"(?:[01]\d|2[0-3]):[0-5]\d";

/// The end of the UTC text Dart writes of a `DateTime`, its fraction in the fewest digits.
const UTC_TRIMMED: &str = concat!(
    r"replaceFirstMapped(RegExp(r'\.(\d*?)0+Z$'), ",
    r"(found) => found[1]!.isEmpty ? 'Z' : '.${found[1]}Z')"
);

/// A time as `z.iso.time()` reads it, as Dart source writes it.
const TIME: &str = r"(?:[01]\d|2[0-3]):[0-5]\d(?::[0-5]\d(?:\.\d+)?)?";

/// A time as serde writes it, its fraction in the fewest digits, as Dart source writes it.
const TIME_SENT: &str = r"(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d*[1-9])?";

/// The test a text `held` passes where it matches `pattern` whole.
fn matching(pattern: &str, held: &str) -> String {
    format!("RegExp(r'^(?:{pattern})$').hasMatch({held})")
}

/// The read of the text `read`, held to `test` of `text`, refused as not `expected`.
fn read(read: &str, test: &str, expected: &str) -> String {
    format!(
        "(switch ({read}) {{ final text when {test} => text, final text => throw \
         FormatException('expected {expected}, found ${{jsonEncode(text)}}') }})"
    )
}

/// The write of `value` as `current`, held to `test` of `sent`, refused as not `expected`.
fn sent(value: &str, current: &str, test: &str, expected: &str) -> String {
    format!(
        "(switch ({current}) {{ final sent when {test} => sent, _ => throw \
         FormatException('expected {expected}, found ${{jsonEncode({value})}}') }})"
    )
}

/// The text `value` with the seconds written that the pattern `unsecond` finds left out.
fn unseconded(value: &str, unsecond: &str) -> String {
    format!(
        "switch ({value}) {{ final unzoned => switch (RegExp(r'^(?:{unsecond})$')\
         .firstMatch(unzoned)) {{ final found? => '${{found[1]}}:00${{found[2] ?? \
         ''}}', _ => unzoned }} }}"
    )
}

/// The text `value` with the trailing zeros of its fraction dropped.
fn trimmed(value: &str) -> String {
    format!(
        "({value}).replaceFirstMapped(RegExp(r'\\.(\\d*?)0+$'), (found) => found[1]!.isEmpty ? '' \
         : '.${{found[1]}}')"
    )
}

/// Each of `written` in `dart`.
fn holds(dart: &str, written: &[String]) {
    for text in written {
        assert!(dart.contains(text.as_str()), "{text}\nin: {dart}");
    }
}

/// A `char` and a date are read where Zod reads them, and written as they are where serde writes
/// them, a map's key among them, which is a `String` already and so is not cast.
#[test]
fn a_char_and_a_date_are_read_and_written_in_their_form() {
    let letter = "text.runes.length == 1";
    holds(
        &chronometer_dart::dart_definition(),
        &[
            format!(
                "day: _readAt(json, 'day', () => {}),",
                read("json['day'] as String", &matching(DATE, "text"), "a date")
            ),
            format!(
                "letter: _readAt(json, 'letter', () => {}),",
                read("json['letter'] as String", letter, "a character")
            ),
            format!("MapEntry({}, ", read("e.key", letter, "a character")),
            format!(
                "MapEntry({}, ",
                read("e.key", &matching(DATE, "text"), "a date")
            ),
            format!(
                "MapEntry({}, ",
                sent("e.key", "e.key", "sent.runes.length == 1", "a character")
            ),
            format!(
                "MapEntry({}, ",
                sent("e.key", "e.key", &matching(DATE, "sent"), "a date")
            ),
            format!(
                "'day': {},",
                sent("day", "day", &matching(DATE, "sent"), "a date")
            ),
            format!(
                "'letter': {},",
                sent("letter", "letter", "sent.runes.length == 1", "a character")
            ),
        ],
    );
}

/// A time is written with the seconds a text leaves out and its fraction in the fewest digits, and
/// a date and time with the `T` jiff writes where it reads a space too, then held to the form
/// serde writes.
#[test]
fn a_time_and_a_date_and_time_are_written_with_their_seconds() {
    holds(
        &time_log_dart::dart_definition(),
        &[sent(
            "e",
            &trimmed(&unseconded("e", &format!("({HOUR_MINUTE})()"))),
            &matching(TIME_SENT, "sent"),
            "a time",
        )],
    );
    let stamp = stamp_log_dart::dart_definition();
    holds(
        &stamp,
        &[
            format!(
                "stamp: _readAt(json, 'stamp', () => {}),",
                read(
                    "json['stamp'] as String",
                    &matching(&format!("{DATE}[T ]{TIME}"), "text"),
                    "a date and time"
                )
            ),
            format!(
                "'stamp': {},",
                sent(
                    "stamp",
                    &trimmed(&unseconded(
                        "(stamp).replaceFirst(' ', 'T')",
                        &format!("({DATE}T{HOUR_MINUTE})()")
                    )),
                    &matching(&format!("{DATE}T{TIME_SENT}"), "sent"),
                    "a date and time"
                )
            ),
        ],
    );
}

/// A date and time with its zone is a `DateTime`, read by `DateTime.parse` and written in UTC, as a
/// value and as a map's key.
#[test]
fn a_date_and_time_with_its_zone_is_written_in_utc() {
    holds(
        &instant_log_dart::dart_definition(),
        &[
            "instant: _readAt(json, 'instant', () => DateTime.parse(json['instant'] as String)),"
                .to_owned(),
            format!("'instant': (instant).toUtc().toIso8601String().{UTC_TRIMMED},"),
            format!("MapEntry(e.key.toUtc().toIso8601String().{UTC_TRIMMED}, "),
            format!(
                "'instants': instants.map((e) => (e).toUtc().toIso8601String().{UTC_TRIMMED}).toList(),"
            ),
        ],
    );
}
