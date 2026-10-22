//! What a client reads and sends of each text of one form.

use super::TextForm;

/// Whether a client reads `text` as `form`, by the pattern [`TextForm::read`] answers.
fn reads(form: TextForm, text: &str) -> bool {
    form.read().map_or_else(
        || {
            matches!(form, TextForm::Char) && text.chars().count() == 1
                || !matches!(form, TextForm::Char)
        },
        |pattern| {
            regex::Regex::new(&format!("^(?:{pattern})$"))
                .unwrap()
                .is_match(text)
        },
    )
}

#[test]
fn a_char_is_one_code_point_read_and_sent_as_it_is() {
    for (text, held) in [("a", true), ("\u{1f600}", true), ("ab", false), ("", false)] {
        assert_eq!(reads(TextForm::Char, text), held, "{text}");
        assert_eq!(
            TextForm::Char.sends(text),
            held.then(|| text.to_owned()),
            "{text}"
        );
    }
}

#[cfg(feature = "chrono")]
#[test]
fn each_chrono_text_is_read_as_zod_reads_it_and_sent_as_serde_writes_it() {
    for (form, text, read, sent) in [
        (TextForm::Date, "2024-01-05", true, Some("2024-01-05")),
        (TextForm::Date, "2024-1-5", false, None),
        (TextForm::Date, "2023-02-29", false, None),
        (TextForm::Time, "10:20:30", true, Some("10:20:30")),
        (TextForm::Time, "10:20:30.123", true, Some("10:20:30.123")),
        (TextForm::Time, "10:20", true, Some("10:20:00")),
        (TextForm::Time, "10:20Z", false, None),
        (TextForm::Time, "hello", false, None),
        (
            TextForm::DateTime,
            "2024-01-05T10:20:30",
            true,
            Some("2024-01-05T10:20:30"),
        ),
        (
            TextForm::DateTime,
            "2024-01-05T10:20:30.000Z",
            true,
            Some("2024-01-05T10:20:30.000"),
        ),
        (
            TextForm::DateTime,
            "2024-01-05T10:20",
            true,
            Some("2024-01-05T10:20:00"),
        ),
        (
            TextForm::DateTime,
            "2024-01-05T10:20Z",
            true,
            Some("2024-01-05T10:20:00"),
        ),
        (TextForm::DateTime, "2024-01-05T10:20:30ZZ", false, None),
        (TextForm::DateTime, "2024-01-05 10:20:30", false, None),
        (
            TextForm::Zoned,
            "2024-01-05T10:20:30Z",
            true,
            Some("2024-01-05T10:20:30Z"),
        ),
        (
            TextForm::Zoned,
            "2024-01-05T10:20:30+02:00",
            true,
            Some("2024-01-05T10:20:30+02:00"),
        ),
        (
            TextForm::Zoned,
            "2024-01-05T10:20Z",
            true,
            Some("2024-01-05T10:20:00Z"),
        ),
        (
            TextForm::Zoned,
            "2024-01-05T10:20-05:30",
            true,
            Some("2024-01-05T10:20:00-05:30"),
        ),
        (TextForm::Zoned, "2026-10-22T13:07:11.032917", true, None),
        (TextForm::Zoned, "hello", true, None),
    ] {
        assert_eq!(reads(form, text), read, "{form:?} reads {text}");
        assert_eq!(form.sends(text).as_deref(), sent, "{form:?} sends {text}");
    }
}
