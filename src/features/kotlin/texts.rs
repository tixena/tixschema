//! A text serde reads in one form alone, which the Kotlin client holds as a `String`: a `char`, and
//! a chrono date, time or date and time. Each is read through a serializer of the item's own, which
//! throws `SerializationException` for a `char` of other than one code point, as serde refuses it,
//! and for a date, a time or a date and time the Zod schema of it refuses. Where serde and Zod
//! part, the read follows Zod: serde reads forms Zod refuses, and refuses a date and time under `Z`
//! or with no seconds, which Zod reads.
//!
//! The plugin reads a declared value by the serializer its annotation names, and a serializer the
//! emitter writes composes the one it reads a value with.

use core::cell::RefCell;
use std::collections::HashMap;

#[cfg(feature = "chrono")]
use super::kotlin_template_text;
use super::{KOTLIN_TUPLE_AUX, KOTLIN_TUPLE_COUNTER, validate};
use crate::field_type::{FieldDef, FieldDefType, is_sequence_wrapper};

/// A date as `z.iso.date()` reads it.
#[cfg(feature = "chrono")]
const DATE: &str = concat!(
    r"(?:(?:\d\d[2468][048]|\d\d[13579][26]|\d\d0[48]|[02468][048]00|[13579][26]00)-02-29|",
    r"\d{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12]\d|3[01])|(?:0[469]|11)-(?:0[1-9]|[12]\d|30)|",
    r"(?:02)-(?:0[1-9]|1\d|2[0-8])))"
);

/// A time as `z.iso.time()` reads it: the seconds and their fraction may be left out.
#[cfg(feature = "chrono")]
const TIME: &str = r"(?:[01]\d|2[0-3]):[0-5]\d(?::[0-5]\d(?:\.\d+)?)?";

/// The time `z.iso.datetime({ local: true })` reads under `Z`, where the seconds are required.
#[cfg(feature = "chrono")]
const TIME_UNDER_Z: &str = r"(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d+)?(?:Z)";

thread_local! {
    /// The type each alias the emitter wrote with no type parameter names, by Rust ident.
    static KOTLIN_ALIASES: RefCell<HashMap<String, FieldDef>> = RefCell::new(HashMap::new());
    /// The serializer each form the item being expanded reads is read through, so every value of
    /// one form names one. Cleared with the tuple wrappers.
    static KOTLIN_TEXTS: RefCell<Vec<(TextForm, String)>> = const { RefCell::new(Vec::new()) };
}

/// A text serde reads in one form alone.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TextForm {
    Char,
    #[cfg(feature = "chrono")]
    Date,
    #[cfg(feature = "chrono")]
    DateTime,
    #[cfg(feature = "chrono")]
    Time,
}

impl TextForm {
    /// The `object` named `named` that reads and writes a text of the form, and refuses any other.
    fn declared(self, named: &str) -> String {
        let (form, refused, expected) = match self {
            Self::Char => (
                String::new(),
                "text.codePointCount(0, text.length) != 1",
                "a character",
            ),
            #[cfg(feature = "chrono")]
            Self::Date => (matched(DATE), "!form.matches(text)", "a date"),
            #[cfg(feature = "chrono")]
            Self::DateTime => (
                matched(&format!("{DATE}T(?:{TIME_UNDER_Z}|{TIME})")),
                "!form.matches(text)",
                "a date and time",
            ),
            #[cfg(feature = "chrono")]
            Self::Time => (matched(TIME), "!form.matches(text)", "a time"),
        };
        format!(
            "object {named} : KSerializer<String> {{ {form}override val descriptor: \
             SerialDescriptor = PrimitiveSerialDescriptor(\"{named}\", PrimitiveKind.STRING); \
             override fun serialize(encoder: Encoder, value: String) = \
             encoder.encodeString(value); override fun deserialize(decoder: Decoder): String {{ \
             val text = decoder.decodeString(); if ({refused}) throw \
             SerializationException(\"expected {expected}, found \" + JsonPrimitive(text)); \
             return text }} }}"
        )
    }

    /// The form of a value typed `field`, at its innermost level.
    pub const fn of(field: &FieldDef) -> Option<Self> {
        match &field.field_type {
            FieldDefType::Char => Some(Self::Char),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDate => Some(Self::Date),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveDateTime => Some(Self::DateTime),
            #[cfg(feature = "chrono")]
            FieldDefType::NaiveTime => Some(Self::Time),
            // Zod reads any text a JavaScript `Date` reads there, which no Kotlin check can follow.
            #[cfg(feature = "chrono")]
            FieldDefType::DateTime => None,
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
}

/// Drops the serializers of the item just written.
pub fn clear() {
    KOTLIN_TEXTS.with(|texts| texts.borrow_mut().clear());
}

/// The `Regex` property a serializer matches a whole text against `pattern` through.
#[cfg(feature = "chrono")]
fn matched(pattern: &str) -> String {
    format!(
        "private val form = Regex(\"{}\"); ",
        kotlin_template_text(pattern)
    )
}

/// Whether a declaration of a value typed `field` names a text of one form where the plugin reads
/// the annotation it writes on it: itself, a key or a value of a map, a type argument, a range's
/// bound and an arm of a `Result`. A tuple's wrapper reads its positions itself, and an alias with
/// no type parameter names the serializer in its own declaration.
pub fn declared(field: &FieldDef) -> bool {
    found(field, false, is_a_text)
}

/// Whether `field`, or a value it holds where [`reaches`] or [`declared`] looks, is one `sought`
/// answers for: `written` asks for a serializer the emitter writes.
pub fn found(field: &FieldDef, written: bool, sought: fn(&FieldDef) -> bool) -> bool {
    if sought(field) {
        return true;
    }
    if let FieldDefType::SiblingType(name, arguments) = &field.field_type {
        if let [element] = arguments.as_slice()
            && is_sequence_wrapper(name)
        {
            found(element, written, sought)
        } else if arguments.is_empty() {
            written && named_by(name).is_some_and(|target| found(&target, written, sought))
        } else {
            (!written || validate::reads_unjudged(name))
                && arguments
                    .iter()
                    .any(|argument| found(argument, written, sought))
        }
    } else if let FieldDefType::Map(key, value) = &field.field_type {
        found(key, written, sought) || found(value, written, sought)
    } else if let FieldDefType::Range(_, bound) = &field.field_type {
        found(bound, written, sought)
    } else if let FieldDefType::Result(ok, err) = &field.field_type {
        found(ok, written, sought) || found(err, written, sought)
    } else {
        false
    }
}

/// Whether a value typed `field` is a text of one form at its innermost level.
const fn is_a_text(field: &FieldDef) -> bool {
    TextForm::of(field).is_some()
}

/// The type the alias written `name` names, where the emitter wrote it above with no type
/// parameter.
pub fn named_by(name: &str) -> Option<FieldDef> {
    KOTLIN_ALIASES.with(|aliases| aliases.borrow().get(name).cloned())
}

/// Notes the alias written `rust_ident`, which declares no type parameter and names `target`.
pub fn note_alias(rust_ident: &str, target: &FieldDef) {
    KOTLIN_ALIASES.with(|aliases| {
        aliases
            .borrow_mut()
            .insert(rust_ident.to_owned(), target.clone());
    });
}

/// Whether a serializer the emitter writes reads a value typed `field` through the serializer of a
/// form it holds, which it composes: the places [`declared`] names, and the type an alias written
/// above with no type parameter names. A type argument is handed to the serializer of a type the
/// emitter wrote above: the serializer of an alias with type parameters is no class's, and a type
/// declared below may be one.
pub fn reaches(field: &FieldDef) -> bool {
    found(field, true, is_a_text)
}

/// The serializer a text of `form` is read through, declared once per item.
pub fn serializer(form: TextForm) -> String {
    KOTLIN_TEXTS.with(|texts| {
        let mut written = texts.borrow_mut();
        if let Some((_, named)) = written.iter().find(|(declared, _)| *declared == form) {
            return named.clone();
        }
        let index = KOTLIN_TUPLE_COUNTER.with(|counter| {
            let next = counter.get() + 1;
            counter.set(next);
            next
        });
        let named = format!("KotlinText{index}");
        KOTLIN_TUPLE_AUX.with(|aux| aux.borrow_mut().push(form.declared(&named)));
        written.push((form, named.clone()));
        named
    })
}
