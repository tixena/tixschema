//! A text serde reads in one form alone, which the Kotlin client holds as a `String`: a `char`, and
//! a chrono date, time, date and time, and date and time with its zone. Each is read and written
//! through a serializer of the item's own. Its read throws `SerializationException` for a `char`
//! of other than one code point, as serde refuses it, and for a date, a time or a date and time
//! the Zod schema of it refuses: where serde and Zod part, the read follows Zod. Its write sends
//! the form serde writes ([`TextForm`]), and throws for a text no rewrite turns into it.
//!
//! A number and a `bool` are read through a serializer of the item's own as well: the JSON decoder
//! of `kotlinx.serialization` reads a number or a `bool` written between quotes, which serde and
//! the Zod schema refuse. A map's key is the one place a quoted number is the form, and is read
//! as it was.
//!
//! The plugin reads a declared value by the serializer its annotation names, and a serializer the
//! emitter writes composes the one it reads a value with.

use core::cell::{Cell, RefCell};
use core::fmt::Write as _;
use std::collections::{HashMap, HashSet};

use super::{KOTLIN_TUPLE_AUX, KOTLIN_TUPLE_COUNTER, kotlin_template_text, validate};
use crate::features::text_forms::TextForm;
use crate::field_type::{FieldDef, FieldDefType, is_sequence_wrapper};

thread_local! {
    /// The type each alias the emitter wrote with no type parameter names, by Rust ident.
    static KOTLIN_ALIASES: RefCell<HashMap<String, FieldDef>> = RefCell::new(HashMap::new());
    /// The brands, by Rust ident, that write a serializer for a map key of their own.
    static KOTLIN_KEYED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    /// The serializer each form the item being expanded reads is read through, so every value of
    /// one form names one. Cleared with the tuple wrappers.
    static KOTLIN_TEXTS: RefCell<Vec<(TextForm, String)>> = const { RefCell::new(Vec::new()) };
    /// The same for each number type and `bool` the item reads.
    static KOTLIN_NUMBERS: RefCell<Vec<(Primitive, String)>> = const { RefCell::new(Vec::new()) };
    /// How many number serializers any item has declared: the types of one file share their
    /// names, so it is not cleared.
    static KOTLIN_NUMBER_COUNTER: Cell<usize> = const { Cell::new(0) };
}

/// A number type or `bool` the Kotlin client reads from an unquoted JSON value alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Primitive {
    Bool,
    /// A `bool` that is a map's key: the text `true` or `false`, in that case alone.
    BoolKey,
    F32,
    F64,
    I16,
    I32,
    I64,
    I8,
    Isize,
    /// A `DateTime<Tz>` under `as_number`: the epoch milliseconds, held as a `Long`.
    #[cfg(feature = "chrono")]
    Millis,
    U16,
    U32,
    U8,
}

/// What a map key reaches once its aliases are peeled, where it is read from the text serde writes.
pub enum KeyLeaf {
    /// A brand, by Rust ident, that writes a serializer for a map key of its own.
    Brand(String),
    Primitive(Primitive),
}

impl Primitive {
    /// The Kotlin type the primitive is held as.
    pub const fn kotlin_type(self) -> &'static str {
        self.words().0
    }

    /// The primitive a value typed `field` is at its innermost level. A `u64` and a `usize` are
    /// refused under `kotlin` before they are read.
    pub fn of(field: &FieldDef) -> Option<Self> {
        match &field.field_type {
            FieldDefType::Boolean => Some(Self::Bool),
            FieldDefType::I8 => Some(Self::I8),
            FieldDefType::I16 => Some(Self::I16),
            FieldDefType::I32 => Some(Self::I32),
            FieldDefType::I64 => Some(Self::I64),
            FieldDefType::Isize => Some(Self::Isize),
            FieldDefType::U8 => Some(Self::U8),
            FieldDefType::U16 => Some(Self::U16),
            FieldDefType::U32 => Some(Self::U32),
            FieldDefType::F32 => Some(Self::F32),
            FieldDefType::F64 => Some(Self::F64),
            #[cfg(feature = "chrono")]
            FieldDefType::DateTime
                if field
                    .model_schema_prop_meta
                    .as_ref()
                    .is_some_and(|meta| meta.as_number) =>
            {
                Some(Self::Millis)
            }
            _ => None,
        }
    }

    /// The Kotlin type, and the name serde words the refusal with.
    const fn words(self) -> (&'static str, &'static str) {
        match self {
            Self::Bool | Self::BoolKey => ("Boolean", "a boolean"),
            Self::I8 => ("Byte", "i8"),
            Self::I16 => ("Short", "i16"),
            Self::I32 => ("Int", "i32"),
            Self::I64 => ("Long", "i64"),
            Self::Isize => ("Long", "isize"),
            Self::U8 => ("UByte", "u8"),
            Self::U16 => ("UShort", "u16"),
            Self::U32 => ("UInt", "u32"),
            Self::F32 => ("Float", "f32"),
            Self::F64 => ("Double", "f64"),
            #[cfg(feature = "chrono")]
            Self::Millis => ("Long", "a unix timestamp in milliseconds"),
        }
    }
}

/// The `object` named `named` that reads a number or a `bool` as the library does, but for one
/// written as a string, which it refuses in serde's words.
fn number_declared(primitive: Primitive, named: &str) -> String {
    let (kotlin, expected) = primitive.words();
    if primitive == Primitive::BoolKey {
        return format!(
            "object {named} : KSerializer<{kotlin}> {{ private val held = serializer<{kotlin}>(); \
             override val descriptor: SerialDescriptor = held.descriptor; override fun \
             serialize(encoder: Encoder, value: {kotlin}) = held.serialize(encoder, value); \
             override fun deserialize(decoder: Decoder): {kotlin} {{ val json = decoder as? \
             JsonDecoder ?: return held.deserialize(decoder); val element = \
             json.decodeJsonElement(); return when ((element as? JsonPrimitive)?.takeIf {{ \
             it.isString }}?.content) {{ \"true\" -> true; \"false\" -> false; else -> throw \
             SerializationException(\"invalid type: \" + element + \", expected {expected}\") }} }} }}"
        );
    }
    format!(
        "object {named} : KSerializer<{kotlin}> {{ private val held = serializer<{kotlin}>(); \
         override val descriptor: SerialDescriptor = held.descriptor; override fun \
         serialize(encoder: Encoder, value: {kotlin}) = held.serialize(encoder, value); override \
         fun deserialize(decoder: Decoder): {kotlin} {{ val json = decoder as? JsonDecoder ?: \
         return held.deserialize(decoder); val element = json.decodeJsonElement(); if (element is \
         JsonPrimitive && element.isString) throw SerializationException(\"invalid type: string \" \
         + element + \", expected {expected}\"); return json.json.decodeFromJsonElement(held, \
         element) }} }}"
    )
}

/// The `object` named `named` that reads a text of `form`, refusing one Zod refuses, and writes
/// it in the form serde writes, refusing one no rewrite turns into that form.
fn object_declared(form: TextForm, named: &str) -> String {
    let expected = form.expected();
    let refused = |text: &str| {
        format!(
            "throw SerializationException(\"expected {expected}, found \" + JsonPrimitive({text}))"
        )
    };
    let read = form.read();
    let sent = form.sent();
    let mut members = String::new();
    if let Some(pattern) = &read {
        members.push_str(&matched("form", pattern));
    }
    let sending = match (&read, &sent) {
        (Some(reading), Some(sending)) if reading == sending => Some("form"),
        (_, Some(pattern)) => {
            members.push_str(&matched("sending", pattern));
            Some("sending")
        }
        (_, None) => None,
    };
    let (mut steps, zoneless) = if form.unzoned() {
        (
            "val unzoned = value.removeSuffix(\"Z\"); ".to_owned(),
            "unzoned",
        )
    } else {
        (String::new(), "value")
    };
    let current = form.unsecond().map_or(zoneless, |pattern| {
        members.push_str(&matched("unsecond", &pattern));
        let _ = write!(
            steps,
            "val sent = unsecond.matchEntire({zoneless})?.let {{ it.groupValues[1] + \":00\" + \
             it.groupValues[2] }} ?: {zoneless}; "
        );
        "sent"
    });
    let held = sending.map_or_else(
        || format!("{current}.codePointCount(0, {current}.length) != 1"),
        |pattern| format!("!{pattern}.matches({current})"),
    );
    let checked_read = read.map_or_else(
        || {
            if form == TextForm::Char {
                format!(
                    "if (text.codePointCount(0, text.length) != 1) {}; ",
                    refused("text")
                )
            } else {
                String::new()
            }
        },
        |_| format!("if (!form.matches(text)) {}; ", refused("text")),
    );
    format!(
        "object {named} : KSerializer<String> {{ {members}override val descriptor: \
         SerialDescriptor = PrimitiveSerialDescriptor(\"{named}\", PrimitiveKind.STRING); \
         override fun serialize(encoder: Encoder, value: String) {{ {steps}if ({held}) {}; \
         encoder.encodeString({current}) }} override fun deserialize(decoder: Decoder): String {{ \
         val text = decoder.decodeString(); {checked_read}return text }} }}",
        refused("value")
    )
}

/// Drops the serializers of the item just written.
pub fn clear() {
    KOTLIN_TEXTS.with(|texts| texts.borrow_mut().clear());
    KOTLIN_NUMBERS.with(|numbers| numbers.borrow_mut().clear());
}

/// The `Regex` property `named` a serializer matches a whole text against `pattern` through.
fn matched(named: &str, pattern: &str) -> String {
    format!(
        "private val {named} = Regex(\"{}\"); ",
        kotlin_template_text(pattern)
    )
}

/// Whether a declaration of a value typed `field` names a text of one form where the plugin reads
/// the annotation it writes on it: itself, a key or a value of a map, a type argument, a range's
/// bound and an arm of a `Result`. A tuple's wrapper reads its positions itself, and an alias with
/// no type parameter names the serializer in its own declaration.
pub fn declared(field: &FieldDef) -> bool {
    found(field, false, is_a_form)
}

/// Whether `field`, or a value it holds where [`reaches`] or [`declared`] looks, is one `sought`
/// answers for: `written` asks for a serializer the emitter writes. `sought` is told whether the
/// value is a map's key.
pub fn found(field: &FieldDef, written: bool, sought: fn(&FieldDef, bool) -> bool) -> bool {
    walk(field, written, sought, false)
}

fn walk(field: &FieldDef, written: bool, sought: fn(&FieldDef, bool) -> bool, key: bool) -> bool {
    let found = |held: &FieldDef| walk(held, written, sought, key);
    if sought(field, key) {
        return true;
    }
    if let FieldDefType::SiblingType(name, arguments) = &field.field_type {
        if let [element] = arguments.as_slice()
            && is_sequence_wrapper(name)
        {
            found(element)
        } else if arguments.is_empty() {
            written && named_by(name).is_some_and(|target| found(&target))
        } else {
            (!written || validate::reads_unjudged(name))
                && arguments
                    .iter()
                    .any(|argument| walk(argument, written, sought, false))
        }
    } else if let FieldDefType::Map(map_key, value) = &field.field_type {
        walk(map_key, written, sought, true) || walk(value, written, sought, false)
    } else if let FieldDefType::Range(_, bound) = &field.field_type {
        found(bound)
    } else if let FieldDefType::Result(ok, err) = &field.field_type {
        found(ok) || found(err)
    } else {
        false
    }
}

/// Whether a value typed `field` is a text of one form at its innermost level, or a number or a
/// `bool`; a map's key is only a `bool`, which is read as `true` or `false` alone, or a brand or
/// an alias that reaches a number or a `bool`.
fn is_a_form(field: &FieldDef, key: bool) -> bool {
    TextForm::of(field).is_some()
        || Primitive::of(field).is_some_and(|primitive| !key || primitive == Primitive::Bool)
        || (key
            && matches!(field.field_type, FieldDefType::SiblingType(_, _))
            && key_leaf(field).is_some())
}

/// What `key` reaches, or `None` for a key that reaches neither a brand with a key serializer nor
/// a number or a `bool`.
pub fn key_leaf(key: &FieldDef) -> Option<KeyLeaf> {
    let mut held = key.clone();
    loop {
        if held.array_depth > 0 || held.is_optional() {
            return None;
        }
        let FieldDefType::SiblingType(name, arguments) = &held.field_type else {
            return Primitive::of(&held)
                .filter(|&primitive| !is_epoch(primitive))
                .map(KeyLeaf::Primitive);
        };
        if !arguments.is_empty() {
            return None;
        }
        if KOTLIN_KEYED.with(|keyed| keyed.borrow().contains(name)) {
            return Some(KeyLeaf::Brand(name.clone()));
        }
        held = named_by(name)?;
    }
}

#[cfg(feature = "chrono")]
fn is_epoch(primitive: Primitive) -> bool {
    primitive == Primitive::Millis
}

#[cfg(not(feature = "chrono"))]
fn is_epoch(_: Primitive) -> bool {
    false
}

/// Notes the brand written `rust_ident`, which writes a serializer for a map key of its own.
pub fn note_keyed(rust_ident: &str) {
    KOTLIN_KEYED.with(|keyed| {
        keyed.borrow_mut().insert(rust_ident.to_owned());
    });
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
    found(field, true, is_a_form)
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
        KOTLIN_TUPLE_AUX.with(|aux| aux.borrow_mut().push(object_declared(form, &named)));
        written.push((form, named.clone()));
        named
    })
}

/// The serializer a number or a `bool` is read through, declared once per item.
pub fn number_serializer(primitive: Primitive) -> String {
    KOTLIN_NUMBERS.with(|numbers| {
        let mut written = numbers.borrow_mut();
        if let Some((_, named)) = written.iter().find(|(declared, _)| *declared == primitive) {
            return named.clone();
        }
        let index = KOTLIN_NUMBER_COUNTER.with(|counter| {
            let next = counter.get() + 1;
            counter.set(next);
            next
        });
        let named = format!("KotlinNumber{index}");
        KOTLIN_TUPLE_AUX.with(|aux| aux.borrow_mut().push(number_declared(primitive, &named)));
        written.push((primitive, named.clone()));
        named
    })
}
