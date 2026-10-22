//! A text serde reads in one form alone, which a client holds as a string: a `char`, and a chrono
//! date, time, date and time, and date and time with its zone. A client reads one as the Zod
//! schema of its type reads it, refusing a `char` of other than one code point as serde does, and
//! sends it in the form serde writes, which every target reads: it writes the seconds a time
//! leaves out, drops the `Z` serde refuses on a date and time with no zone, and refuses a text no
//! such rewrite turns into that form.
//!
//! The patterns are those of Zod 4.6.5, written with `\d` for an ASCII digit.

use crate::field_type::{FieldDef, FieldDefType};

/// A date as `z.iso.date()` reads it, and as serde writes it.
#[cfg(feature = "chrono")]
pub const DATE: &str = concat!(
    r"(?:(?:\d\d[2468][048]|\d\d[13579][26]|\d\d0[48]|[02468][048]00|[13579][26]00)-02-29|",
    r"\d{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12]\d|3[01])|(?:0[469]|11)-(?:0[1-9]|[12]\d|30)|",
    r"(?:02)-(?:0[1-9]|1\d|2[0-8])))"
);

/// The hour and the minute of a time.
#[cfg(feature = "chrono")]
pub const HOUR_MINUTE: &str = r"(?:[01]\d|2[0-3]):[0-5]\d";

/// A time as `z.iso.time()` reads it: the seconds and their fraction may be left out.
#[cfg(feature = "chrono")]
pub const TIME: &str = r"(?:[01]\d|2[0-3]):[0-5]\d(?::[0-5]\d(?:\.\d+)?)?";

/// A time as serde writes it, its seconds written.
#[cfg(feature = "chrono")]
pub const TIME_SENT: &str = r"(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d+)?";

/// The zone of a date and time, as `z.iso.datetime({ offset: true })` reads it.
#[cfg(feature = "chrono")]
pub const OFFSET: &str = r"(?:Z|[+-](?:[01]\d|2[0-3]):[0-5]\d)";

/// A text serde reads in one form alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextForm {
    Char,
    #[cfg(feature = "chrono")]
    Date,
    #[cfg(feature = "chrono")]
    DateTime,
    #[cfg(feature = "chrono")]
    Time,
    /// A `DateTime<Tz>` a client holds as its text: read as any text, as Zod reads any text a
    /// JavaScript `Date` reads there, and sent in RFC 3339 with its zone.
    #[cfg(feature = "chrono")]
    Zoned,
}

impl TextForm {
    /// What a refusal names the form as: `expected {expected}, found "ab"`.
    pub const fn expected(self) -> &'static str {
        match self {
            Self::Char => "a character",
            #[cfg(feature = "chrono")]
            Self::Date => "a date",
            #[cfg(feature = "chrono")]
            Self::DateTime => "a date and time",
            #[cfg(feature = "chrono")]
            Self::Time => "a time",
            #[cfg(feature = "chrono")]
            Self::Zoned => "a date and time with its zone",
        }
    }

    /// The form of a value typed `field`, at its innermost level. A `DateTime<Tz>` under
    /// `as_number` is a number.
    pub fn of(field: &FieldDef) -> Option<Self> {
        match &field.field_type {
            FieldDefType::Char => Some(Self::Char),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDate => Some(Self::Date),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDateTime => Some(Self::DateTime),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveTime => Some(Self::Time),
            #[cfg(feature = "chrono")]
            FieldDefType::DateTime => (!field
                .model_schema_prop_meta
                .as_ref()
                .is_some_and(|meta| meta.as_number))
            .then_some(Self::Zoned),
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
    pub fn read(self) -> Option<String> {
        match self {
            Self::Char => None,
            #[cfg(feature = "chrono")]
            Self::Date => Some(DATE.to_owned()),
            #[cfg(feature = "chrono")]
            Self::DateTime => Some(format!("{DATE}T{TIME}Z?")),
            #[cfg(feature = "chrono")]
            Self::Time => Some(TIME.to_owned()),
            #[cfg(feature = "chrono")]
            Self::Zoned => None,
        }
    }

    /// What a client sends of `text`, in the steps every client writes in its own language: the
    /// `Z` dropped, the seconds written, then the text held to [`Self::sent`]. `None` where it
    /// refuses the text.
    #[cfg(test)]
    pub fn sends(self, text: &str) -> Option<String> {
        let whole = |pattern: &str| regex::Regex::new(&format!("^(?:{pattern})$")).unwrap();
        let mut sent = text.to_owned();
        if self.unzoned() && sent.ends_with('Z') {
            sent.pop();
        }
        if let Some(captured) = self.unsecond().and_then(|pattern| {
            whole(&pattern)
                .captures(&sent)
                .map(|held| (held[1].to_owned(), held[2].to_owned()))
        }) {
            sent = format!("{}:00{}", captured.0, captured.1);
        }
        let kept = self.sent().map_or_else(
            || sent.chars().count() == 1,
            |pattern| whole(&pattern).is_match(&sent),
        );
        kept.then_some(sent)
    }

    /// The pattern a text a client sends matches whole, the form serde writes: `None` for a
    /// `char`, which is one code point.
    pub fn sent(self) -> Option<String> {
        match self {
            Self::Char => None,
            #[cfg(feature = "chrono")]
            Self::Date => Some(DATE.to_owned()),
            #[cfg(feature = "chrono")]
            Self::DateTime => Some(format!("{DATE}T{TIME_SENT}")),
            #[cfg(feature = "chrono")]
            Self::Time => Some(TIME_SENT.to_owned()),
            #[cfg(feature = "chrono")]
            Self::Zoned => Some(format!("{DATE}T{TIME_SENT}{OFFSET}")),
        }
    }

    /// The pattern of a text a client sends with its seconds written as `:00`, where it leaves
    /// them out: the text up to the minute in its first group, and a zone in its second.
    pub fn unsecond(self) -> Option<String> {
        match self {
            Self::Char => None,
            #[cfg(feature = "chrono")]
            Self::Date => None,
            #[cfg(feature = "chrono")]
            Self::DateTime => Some(format!("({DATE}T{HOUR_MINUTE})()")),
            #[cfg(feature = "chrono")]
            Self::Time => Some(format!("({HOUR_MINUTE})()")),
            #[cfg(feature = "chrono")]
            Self::Zoned => Some(format!("({DATE}T{HOUR_MINUTE})({OFFSET})")),
        }
    }

    /// Whether a client drops a `Z` that ends the text before it sends it: serde refuses one on a
    /// date and time with no zone, and Zod reads it.
    pub const fn unzoned(self) -> bool {
        match self {
            Self::Char => false,
            #[cfg(feature = "chrono")]
            Self::Date | Self::Time | Self::Zoned => false,
            #[cfg(feature = "chrono")]
            Self::DateTime => true,
        }
    }
}

#[cfg(test)]
mod tests;
