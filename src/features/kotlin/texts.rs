//! A text serde reads in one form alone, which the Kotlin client holds as a `String`: a `char`, and
//! a chrono date, time, date and time, and date and time with its zone. Each is read and written
//! through a serializer of the item's own. Its read throws `SerializationException` for a `char`
//! of other than one code point, as serde refuses it, and for a date, a time or a date and time
//! the Zod schema of it refuses: where serde and Zod part, the read follows Zod. Its write sends
//! the form serde writes ([`TextForm`]), and throws for a text no rewrite turns into it.
//!
//! An `i64` a member writes as the string of its decimal digits is held as a `Long`, read from
//! and written as that string through a serializer of the item's own as well, which throws for a
//! text the Zod schema refuses.
//!
//! The plugin reads a declared value by the serializer its annotation names, and a serializer the
//! emitter writes composes the one it reads a value with.

use core::cell::RefCell;
use core::fmt::Write as _;
use std::collections::HashMap;

use super::{KOTLIN_TUPLE_AUX, KOTLIN_TUPLE_COUNTER, kotlin_template_text, validate};
use crate::features::text_forms::TextForm;
use crate::field_type::{FieldDef, FieldDefType, is_sequence_wrapper};

thread_local! {
    /// The type each alias the emitter wrote with no type parameter names, by Rust ident.
    static KOTLIN_ALIASES: RefCell<HashMap<String, FieldDef>> = RefCell::new(HashMap::new());
    /// The serializer each form the item being expanded reads is read through, so every value of
    /// one form names one. Cleared with the tuple wrappers.
    static KOTLIN_TEXTS: RefCell<Vec<(TextForm, String)>> = const { RefCell::new(Vec::new()) };
    /// The serializer the digits of an `i64` the item being expanded reads are read through.
    /// Cleared with the tuple wrappers.
    static KOTLIN_DIGITS: RefCell<Option<String>> = const { RefCell::new(None) };
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
    KOTLIN_DIGITS.with(|digits| digits.borrow_mut().take());
}

/// The `object` named `named` that reads the digits of an `i64` as the `Long` they hold, refusing
/// a text the Zod schema refuses, and writes a `Long` as its digits.
fn digits_declared(named: &str) -> String {
    let refused = "throw SerializationException(\"expected an i64 as a string of its decimal \
                   digits, found \" + JsonPrimitive(text))";
    format!(
        "object {named} : KSerializer<Long> {{ {}override val descriptor: SerialDescriptor = \
         PrimitiveSerialDescriptor(\"{named}\", PrimitiveKind.STRING); override fun \
         serialize(encoder: Encoder, value: Long) {{ encoder.encodeString(value.toString()) }} \
         override fun deserialize(decoder: Decoder): Long {{ val text = decoder.decodeString(); \
         if (!form.matches(text)) {refused}; return text.toLongOrNull() ?: {refused} }} }}",
        matched("form", "^-?(0|[1-9][0-9]*)$")
    )
}

/// The serializer the digits of an `i64` are read through, declared once per item.
pub fn digits_serializer() -> String {
    KOTLIN_DIGITS.with(|digits| {
        let mut written = digits.borrow_mut();
        if let Some(named) = written.as_ref() {
            return named.clone();
        }
        let index = KOTLIN_TUPLE_COUNTER.with(|counter| {
            let next = counter.get() + 1;
            counter.set(next);
            next
        });
        let named = format!("KotlinDigits{index}");
        KOTLIN_TUPLE_AUX.with(|aux| aux.borrow_mut().push(digits_declared(&named)));
        *written = Some(named.clone());
        named
    })
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

/// Whether a value typed `field` is a text of one form at its innermost level, the digits of an
/// integer among them.
fn is_a_text(field: &FieldDef) -> bool {
    TextForm::of(field).is_some() || field.field_type.digit_range().is_some()
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
        KOTLIN_TUPLE_AUX.with(|aux| aux.borrow_mut().push(object_declared(form, &named)));
        written.push((form, named.clone()));
        named
    })
}
