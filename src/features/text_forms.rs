//! A text serde reads in one form alone, which a client holds as a string: a `char`, and a date,
//! time, date and time, and date and time with its zone, of chrono or of jiff. A client reads one
//! as the Zod schema of its type reads it, refusing a `char` of other than one code point as serde
//! does, and sends it in the form serde writes, which every target reads: it writes the seconds a
//! time leaves out, drops the `Z` chrono refuses on a date and time with no zone, writes the `T`
//! and the fraction jiff writes, and refuses a text no such rewrite turns into that form.
//!
//! The patterns are those of Zod 4.6.5, written with `\d` for an ASCII digit. The JSON Schema
//! holds a time and a date and time with no zone to the form serde writes through them as well.

#[cfg(all(
    feature = "chrono",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
use crate::field_type::Zone;
#[cfg(any(
    all(feature = "dart", any(feature = "chrono", feature = "jiff")),
    feature = "swift",
    feature = "kotlin"
))]
use crate::field_type::{FieldDef, FieldDefType};

/// A date as `z.iso.date()` reads it, and as serde writes it.
#[cfg(any(feature = "chrono", feature = "jiff"))]
pub const DATE: &str = concat!(
    r"(?:(?:\d\d[2468][048]|\d\d[13579][26]|\d\d0[48]|[02468][048]00|[13579][26]00)-02-29|",
    r"\d{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12]\d|3[01])|(?:0[469]|11)-(?:0[1-9]|[12]\d|30)|",
    r"(?:02)-(?:0[1-9]|1\d|2[0-8])))"
);

/// The hour and the minute of a time.
#[cfg(all(
    any(feature = "chrono", feature = "jiff"),
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub const HOUR_MINUTE: &str = r"(?:[01]\d|2[0-3]):[0-5]\d";

/// A time as `z.iso.time()` reads it: the seconds and their fraction may be left out.
#[cfg(all(
    any(feature = "chrono", feature = "jiff"),
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub const TIME: &str = r"(?:[01]\d|2[0-3]):[0-5]\d(?::[0-5]\d(?:\.\d+)?)?";

/// A time as chrono writes it, its seconds written.
#[cfg(feature = "chrono")]
pub const TIME_SENT: &str = r"(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d+)?";

/// A time as jiff writes it, its seconds written and its fraction in the fewest digits it needs.
#[cfg(feature = "jiff")]
pub const TIME_SENT: &str = r"(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d*[1-9])?";

/// A time with its seconds, its fraction in any number of digits: what a client holds of an
/// instant before it writes the instant as serde does.
#[cfg(all(
    any(feature = "chrono", feature = "jiff"),
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub const TIME_SECONDS: &str = r"(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d+)?";

/// The zone of a date and time, as `z.iso.datetime({ offset: true })` reads it.
#[cfg(all(
    any(feature = "chrono", feature = "jiff"),
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub const OFFSET: &str = r"(?:Z|[+-](?:[01]\d|2[0-3]):[0-5]\d)";

/// A text serde reads in one form alone.
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextForm {
    Char,
    #[cfg(any(feature = "chrono", feature = "jiff"))]
    Date,
    #[cfg(any(feature = "chrono", feature = "jiff"))]
    DateTime,
    /// A jiff `ZonedDateTime`, read and sent as the one text jiff writes, its zone named.
    #[cfg(feature = "jiff")]
    Named,
    /// A `DateTime<Tz>` of a zone but `Utc`, which chrono writes in the offset it holds.
    #[cfg(feature = "chrono")]
    Offset,
    #[cfg(any(feature = "chrono", feature = "jiff"))]
    Time,
    /// A `DateTime<Utc>` or a `Timestamp` a client holds as its text: read as any text, as Zod
    /// reads any text a JavaScript `Date` reads there, and sent in RFC 3339 with its zone.
    #[cfg(any(feature = "chrono", feature = "jiff"))]
    Zoned,
}

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
impl TextForm {
    /// What a refusal names the form as: `expected {expected}, found "ab"`.
    pub const fn expected(self) -> &'static str {
        match self {
            Self::Char => "a character",
            #[cfg(any(feature = "chrono", feature = "jiff"))]
            Self::Date => "a date",
            #[cfg(any(feature = "chrono", feature = "jiff"))]
            Self::DateTime => "a date and time",
            #[cfg(feature = "jiff")]
            Self::Named => "a date and time with its time zone",
            #[cfg(any(feature = "chrono", feature = "jiff"))]
            Self::Time => "a time",
            #[cfg(feature = "chrono")]
            Self::Offset => "a date and time with its zone",
            #[cfg(any(feature = "chrono", feature = "jiff"))]
            Self::Zoned => "a date and time with its zone",
        }
    }

    /// The form of a value typed `field`, at its innermost level. A `DateTime<Tz>` or a
    /// `Timestamp` under `as_number` is a number.
    #[cfg(any(feature = "chrono", feature = "jiff"))]
    pub fn of(field: &FieldDef) -> Option<Self> {
        let as_number = field
            .model_schema_prop_meta
            .as_ref()
            .is_some_and(|meta| meta.as_number);
        match &field.field_type {
            FieldDefType::Char => Some(Self::Char),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDate => Some(Self::Date),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDateTime => Some(Self::DateTime),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveTime => Some(Self::Time),
            #[cfg(feature = "chrono")]
            FieldDefType::DateTime => (!as_number).then_some(match field.zone {
                Zone::Utc => Self::Zoned,
                Zone::Other => Self::Offset,
            }),
            #[cfg(feature = "jiff")]
            FieldDefType::CivilDate => Some(Self::Date),
            #[cfg(feature = "jiff")]
            FieldDefType::CivilDateTime => Some(Self::DateTime),
            #[cfg(feature = "jiff")]
            FieldDefType::CivilTime => Some(Self::Time),
            #[cfg(feature = "jiff")]
            FieldDefType::Timestamp => (!as_number).then_some(Self::Zoned),
            #[cfg(feature = "jiff")]
            FieldDefType::ZonedDateTime => Some(Self::Named),
            #[cfg(feature = "mongodb")]
            FieldDefType::ObjectId => None,
            FieldDefType::SiblingType(_, _)
            | FieldDefType::Map(_, _)
            | FieldDefType::Tuple(_)
            | FieldDefType::Range(_, _)
            | FieldDefType::Result(_, _)
            | FieldDefType::TypeParam(_)
            | FieldDefType::Unknown
            | FieldDefType::StringLiteral(_)
            | FieldDefType::BooleanLiteral(_)
            | FieldDefType::NumberLiteral(_)
            | FieldDefType::Boolean
            | FieldDefType::String
            | FieldDefType::U8
            | FieldDefType::U16
            | FieldDefType::U32
            | FieldDefType::U64
            | FieldDefType::I8
            | FieldDefType::I16
            | FieldDefType::I32
            | FieldDefType::I64
            | FieldDefType::Isize
            | FieldDefType::Usize
            | FieldDefType::F32
            | FieldDefType::F64 => None,
        }
    }

    /// Without a date feature the `char` is the one text form.
    #[cfg(any(feature = "swift", feature = "kotlin"))]
    #[cfg(not(any(feature = "chrono", feature = "jiff")))]
    pub const fn of(field: &FieldDef) -> Option<Self> {
        match &field.field_type {
            FieldDefType::Char => Some(Self::Char),
            #[cfg(feature = "mongodb")]
            FieldDefType::ObjectId => None,
            FieldDefType::SiblingType(_, _)
            | FieldDefType::Map(_, _)
            | FieldDefType::Tuple(_)
            | FieldDefType::Range(_, _)
            | FieldDefType::Result(_, _)
            | FieldDefType::TypeParam(_)
            | FieldDefType::Unknown
            | FieldDefType::StringLiteral(_)
            | FieldDefType::BooleanLiteral(_)
            | FieldDefType::NumberLiteral(_)
            | FieldDefType::Boolean
            | FieldDefType::String
            | FieldDefType::U8
            | FieldDefType::U16
            | FieldDefType::U32
            | FieldDefType::U64
            | FieldDefType::I8
            | FieldDefType::I16
            | FieldDefType::I32
            | FieldDefType::I64
            | FieldDefType::Isize
            | FieldDefType::Usize
            | FieldDefType::F32
            | FieldDefType::F64 => None,
        }
    }

    /// The pattern a text a client reads matches whole, as the Zod schema of the form reads it:
    /// `None` for a `char`, which is one code point, and for a date and time with its zone, which
    /// is any text.
    #[cfg(any(feature = "chrono", feature = "jiff"))]
    pub fn read(self) -> Option<String> {
        match self {
            Self::Char | Self::Zoned => None,
            #[cfg(feature = "chrono")]
            Self::Offset => None,
            Self::Date => Some(DATE.to_owned()),
            #[cfg(feature = "chrono")]
            Self::DateTime => Some(format!("{DATE}T{TIME}Z?")),
            #[cfg(feature = "jiff")]
            Self::DateTime => Some(format!("{DATE}[T ]{TIME}")),
            #[cfg(feature = "jiff")]
            Self::Named => Some(zoned_text()),
            Self::Time => Some(TIME.to_owned()),
        }
    }

    /// A `char` is the one form without the date feature.
    #[cfg(not(any(feature = "chrono", feature = "jiff")))]
    pub const fn read(self) -> Option<String> {
        match self {
            Self::Char => None,
        }
    }

    /// What a client sends of `text`, in the steps every client writes in its own language: the
    /// `Z` dropped, the `T` written, the seconds written, the fraction's trailing zeros dropped,
    /// then the text held to [`Self::sent`]. `None` where it refuses the text.
    #[cfg(test)]
    pub fn sends(self, text: &str) -> Option<String> {
        let whole = |pattern: &str| regex::Regex::new(&format!("^(?:{pattern})$")).unwrap();
        let mut sent = text.to_owned();
        if self.unzoned() && sent.ends_with('Z') {
            sent.pop();
        }
        if self.spaced() {
            sent = sent.replacen(' ', "T", 1);
        }
        if let Some(captured) = self.unsecond().and_then(|pattern| {
            whole(&pattern)
                .captures(&sent)
                .map(|held| (held[1].to_owned(), held[2].to_owned()))
        }) {
            sent = format!("{}:00{}", captured.0, captured.1);
        }
        if self.trimmed() {
            sent = regex::Regex::new(r"\.(\d*?)0+$")
                .unwrap()
                .replace(&sent, |held: &regex::Captures<'_>| {
                    if held[1].is_empty() {
                        String::new()
                    } else {
                        format!(".{}", &held[1])
                    }
                })
                .into_owned();
        }
        let kept = self.sent().map_or_else(
            || sent.chars().count() == 1,
            |pattern| whole(&pattern).is_match(&sent),
        );
        kept.then_some(sent)
    }

    /// The pattern a text a client sends matches whole, the form serde writes: `None` for a
    /// `char`, which is one code point.
    #[cfg(any(feature = "chrono", feature = "jiff"))]
    pub fn sent(self) -> Option<String> {
        match self {
            Self::Char => None,
            Self::Date => Some(DATE.to_owned()),
            Self::DateTime => Some(format!("{DATE}T{TIME_SENT}")),
            #[cfg(feature = "jiff")]
            Self::Named => Some(zoned_text()),
            Self::Time => Some(TIME_SENT.to_owned()),
            #[cfg(feature = "chrono")]
            Self::Offset => Some(format!("{DATE}T{TIME_SECONDS}{OFFSET}")),
            Self::Zoned => Some(format!("{DATE}T{TIME_SECONDS}{OFFSET}")),
        }
    }

    /// A `char` is the one form without the date feature.
    #[cfg(not(any(feature = "chrono", feature = "jiff")))]
    pub const fn sent(self) -> Option<String> {
        match self {
            Self::Char => None,
        }
    }

    /// Whether a client writes the space jiff reads between a date and a time as the `T` jiff
    /// writes there.
    #[cfg(any(feature = "chrono", feature = "jiff"))]
    pub const fn spaced(self) -> bool {
        cfg!(feature = "jiff") && matches!(self, Self::DateTime)
    }

    /// A `char` is the one form without the date feature.
    #[cfg(not(any(feature = "chrono", feature = "jiff")))]
    pub const fn spaced(self) -> bool {
        match self {
            Self::Char => false,
        }
    }

    /// Whether a client drops the trailing zeros of a fraction that ends the text, and the dot
    /// of a fraction of zeros alone, as jiff writes a time.
    #[cfg(any(feature = "chrono", feature = "jiff"))]
    pub const fn trimmed(self) -> bool {
        cfg!(feature = "jiff") && matches!(self, Self::DateTime | Self::Time)
    }

    /// A `char` is the one form without the date feature.
    #[cfg(not(any(feature = "chrono", feature = "jiff")))]
    pub const fn trimmed(self) -> bool {
        match self {
            Self::Char => false,
        }
    }

    /// The pattern of a text a client sends with its seconds written as `:00`, where it leaves
    /// them out: the text up to the minute in its first group, and a zone in its second.
    #[cfg(any(feature = "chrono", feature = "jiff"))]
    pub fn unsecond(self) -> Option<String> {
        match self {
            Self::Char | Self::Date => None,
            #[cfg(feature = "jiff")]
            Self::Named => None,
            Self::DateTime => Some(format!("({DATE}T{HOUR_MINUTE})()")),
            Self::Time => Some(format!("({HOUR_MINUTE})()")),
            #[cfg(feature = "chrono")]
            Self::Offset => Some(format!("({DATE}T{HOUR_MINUTE})({OFFSET})")),
            Self::Zoned => Some(format!("({DATE}T{HOUR_MINUTE})({OFFSET})")),
        }
    }

    /// A `char` is the one form without the date feature.
    #[cfg(not(any(feature = "chrono", feature = "jiff")))]
    pub const fn unsecond(self) -> Option<String> {
        match self {
            Self::Char => None,
        }
    }

    /// Whether a client drops a `Z` that ends the text before it sends it: chrono refuses one on
    /// a date and time with no zone, and Zod reads it. jiff refuses it as well, and so does the
    /// Zod schema of a jiff date and time.
    #[cfg(any(feature = "chrono", feature = "jiff"))]
    pub const fn unzoned(self) -> bool {
        cfg!(feature = "chrono") && matches!(self, Self::DateTime)
    }

    /// A `char` is the one form without the date feature.
    #[cfg(not(any(feature = "chrono", feature = "jiff")))]
    pub const fn unzoned(self) -> bool {
        match self {
            Self::Char => false,
        }
    }
}

/// A `ZonedDateTime` as jiff writes it, whole: a date, a `T`, a time with its seconds, the offset
/// in hours and minutes, which jiff writes as `+00:00` where it is zero, and the name of its time
/// zone in brackets.
#[cfg(feature = "jiff")]
pub fn zoned_text() -> String {
    format!(r"{DATE}T{TIME_SENT}[+-](?:[01]\d|2[0-3]):[0-5]\d\[[A-Za-z0-9_/+:-]+\]")
}

#[cfg(test)]
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
mod tests;
