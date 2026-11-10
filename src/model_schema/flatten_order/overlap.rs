//! Whether a variant of an untagged enum reads every value serde writes for a variant declared
//! after it. serde reads an untagged enum's variants in the order declared and keeps the first that
//! reads, so such a later variant never reads back.
//!
//! What each type writes and reads is recorded when `#[model_schema]` expands it. A type the
//! expansion has not met, a generic type, and a member read through a function of its author's are
//! not looked into. A read a bound, a pattern or a check narrows reads only some of the values of
//! its type, so it reads every value of no variant.

use core::cell::RefCell;
use std::collections::HashMap;

use proc_macro2::TokenStream;
use syn::ext::IdentExt as _;
use syn::{Field, Fields, Item, ItemEnum, ItemStruct, Variant};

use super::super::without_its_option;
use super::{
    Answer, DEPTH, ModelSchemaArgs, absent_reads, def_of, newtype_slot, parameter_names,
    prefixed_guard_message, read_by_a_function, strict, variant_rename_all, written_as_its_slot,
};
use crate::features::model_schema_prop::{
    LiteralValue, ModelSchemaPropMeta, parse_model_schema_prop_attributes,
};
use crate::features::serde::{
    Tagging, derive_reads_a_default, parse_read_renames, parse_serde_field_attributes,
    parse_serde_key_omission, parse_written_renames, struct_tag, variant_on_the_wire,
};
use crate::field_type::{FieldDef, FieldDefType, is_sequence_wrapper};
use crate::rename_rule::resolve_rename_rule;

/// What serde writes of a value, and what it reads as one.
#[derive(Clone)]
enum Wire {
    /// An array of items, of the length it is held to where it is held to one.
    Array(Option<usize>, Box<Self>),
    /// A `bool`, or the one it is held to.
    Bool(Option<bool>),
    Float,
    /// An integer, between the least and the greatest its type holds.
    Integer(i128, i128),
    /// A value whose read a bound, a pattern or a check narrows.
    Narrowed(Box<Self>),
    Null,
    Object(Box<Object>),
    /// Any of these: an `Option`, or the variants of an enum.
    OneOf(Vec<Self>),
    /// Text, or one of the texts it is held to.
    Text(Option<Vec<String>>),
    /// What the expansion cannot see.
    Unseen,
}

/// An object serde writes or reads.
#[derive(Clone, Default)]
struct Object {
    /// Whether it refuses a key it does not declare.
    closed: bool,
    keys: Vec<Key>,
    /// What every key it does not declare holds, where it writes or reads such keys: a flattened
    /// map.
    rest: Option<Box<Wire>>,
}

/// One key of an object.
#[derive(Clone)]
struct Key {
    /// The names serde reads it under, none where serde does not read it.
    read: Vec<String>,
    /// Whether serde reads the object without it.
    read_optional: bool,
    value: Wire,
    /// The name serde writes it under, and whether serde writes it into every value.
    written: Option<(String, bool)>,
}

impl Key {
    /// A key every value holds, at the one text `name`.
    fn tag(key: String, name: String) -> Self {
        Self {
            read: vec![key.clone()],
            read_optional: false,
            value: Wire::Text(Some(vec![name])),
            written: Some((key, true)),
        }
    }
}

thread_local! {
    /// What each type `#[model_schema]` has expanded so far writes and reads.
    static WIRES: RefCell<HashMap<String, Wire>> = RefCell::new(HashMap::new());
}

const fn answer(yes: bool) -> Answer {
    if yes { Answer::Yes } else { Answer::No }
}

/// Whether `reader` reads every value serde writes as `writer`.
fn covers(reader: &Wire, writer: &Wire, depth: usize) -> Answer {
    if depth > DEPTH {
        return Answer::Unknown;
    }
    match (reader, writer) {
        (Wire::Unseen, _) | (_, Wire::Unseen) => Answer::Unknown,
        (Wire::Narrowed(_), _) => Answer::No,
        (_, Wire::Narrowed(written)) => covers(reader, written, depth),
        (_, Wire::OneOf(written)) => written
            .iter()
            .fold(Answer::Yes, |all, one| all.and(covers(reader, one, depth))),
        (Wire::OneOf(read), _) => read
            .iter()
            .fold(Answer::No, |any, one| any.or(covers(one, writer, depth))),
        (Wire::Array(read_length, read), Wire::Array(written_length, written)) => {
            let lengths = read_length.is_none() || read_length == written_length;
            answer(lengths).and(covers(read, written, depth + 1))
        }
        (Wire::Bool(read), Wire::Bool(written)) => answer(read.is_none() || read == written),
        (Wire::Integer(least, greatest), Wire::Integer(lowest, highest)) => {
            answer(least <= lowest && highest <= greatest)
        }
        (Wire::Text(read), Wire::Text(written)) => answer(match (read, written) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(names), Some(texts)) => texts.iter().all(|text| names.contains(text)),
        }),
        (Wire::Object(read), Wire::Object(written)) => object_covers(read, written, depth + 1),
        (Wire::Float, Wire::Float | Wire::Integer(..)) | (Wire::Null, Wire::Null) => Answer::Yes,
        (
            Wire::Array(..)
            | Wire::Bool(_)
            | Wire::Float
            | Wire::Integer(..)
            | Wire::Null
            | Wire::Object(_)
            | Wire::Text(_),
            _,
        ) => Answer::No,
    }
}

/// Whether `reader` reads every object serde writes as `writer`: it reads each key `writer` may
/// write, holding each value `writer` writes there, and requires only keys `writer` always writes.
fn object_covers(reader: &Object, writer: &Object, depth: usize) -> Answer {
    let mut found = Answer::Yes;
    for key in &writer.keys {
        let Some((name, _)) = &key.written else {
            continue;
        };
        let declared = reader.keys.iter().find(|read| read.read.contains(name));
        found = found.and(declared.map_or_else(
            || {
                reader.rest.as_ref().map_or_else(
                    || answer(!reader.closed),
                    |rest| covers(rest, &key.value, depth),
                )
            },
            |read| covers(&read.value, &key.value, depth),
        ));
    }
    if let Some(rest) = &writer.rest {
        // A key the map writes may be any name, one the reader declares among them.
        found = found.and(answer(reader.keys.is_empty())).and(
            reader
                .rest
                .as_ref()
                .map_or_else(|| answer(!reader.closed), |read| covers(read, rest, depth)),
        );
    }
    for key in reader.keys.iter().filter(|key| !key.read_optional) {
        found = found.and(answer(writer.keys.iter().any(|written| {
            written
                .written
                .as_ref()
                .is_some_and(|(name, always)| *always && key.read.contains(name))
        })));
    }
    found
}

/// What serde writes and reads of a value `def` describes.
fn wire_of(def: &FieldDef, depth: usize) -> Wire {
    if depth > DEPTH {
        return Wire::Unseen;
    }
    if def.is_optional() {
        return Wire::OneOf(vec![
            Wire::Null,
            wire_of(&without_its_option(def), depth + 1),
        ]);
    }
    if def.array_depth > 0 {
        let outermost = def.array_depth - 1;
        let mut item = def.clone();
        item.array_depth = outermost;
        return Wire::Array(
            def.array_lengths
                .iter()
                .find(|(level, _)| *level == outermost)
                .map(|(_, length)| *length),
            Box::new(wire_of(&item, depth + 1)),
        );
    }
    match &def.field_type {
        FieldDefType::I8 => Wire::Integer(i128::from(i8::MIN), i128::from(i8::MAX)),
        FieldDefType::I16 => Wire::Integer(i128::from(i16::MIN), i128::from(i16::MAX)),
        FieldDefType::I32 => Wire::Integer(i128::from(i32::MIN), i128::from(i32::MAX)),
        FieldDefType::I64 | FieldDefType::Isize => {
            Wire::Integer(i128::from(i64::MIN), i128::from(i64::MAX))
        }
        FieldDefType::U8 => Wire::Integer(0, i128::from(u8::MAX)),
        FieldDefType::U16 => Wire::Integer(0, i128::from(u16::MAX)),
        FieldDefType::U32 => Wire::Integer(0, i128::from(u32::MAX)),
        FieldDefType::U64 | FieldDefType::Usize => Wire::Integer(0, i128::from(u64::MAX)),
        FieldDefType::Boolean => Wire::Bool(None),
        FieldDefType::BooleanLiteral(value) => Wire::Bool(Some(*value)),
        FieldDefType::Char => Wire::Narrowed(Box::new(Wire::Text(None))),
        FieldDefType::String => Wire::Text(None),
        FieldDefType::StringLiteral(text) => Wire::Text(Some(vec![text.clone()])),
        FieldDefType::F32 | FieldDefType::F64 => Wire::Float,
        FieldDefType::Map(key, value) => {
            let map = Wire::Object(Box::new(Object {
                closed: false,
                keys: Vec::new(),
                rest: Some(Box::new(wire_of(value, depth + 1))),
            }));
            // A key of another type than text is read from some texts alone.
            if matches!(key.field_type, FieldDefType::String) && key.array_depth == 0 {
                map
            } else {
                Wire::Narrowed(Box::new(map))
            }
        }
        FieldDefType::Tuple(elements) => {
            if elements.is_empty() {
                Wire::Null
            } else {
                Wire::Unseen
            }
        }
        FieldDefType::SiblingType(name, arguments) => {
            if is_sequence_wrapper(name) || !arguments.is_empty() {
                Wire::Unseen
            } else {
                WIRES.with(|wires| wires.borrow().get(name).cloned().unwrap_or(Wire::Unseen))
            }
        }
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime
        | FieldDefType::NaiveDate
        | FieldDefType::NaiveDateTime
        | FieldDefType::NaiveTime => Wire::Narrowed(Box::new(Wire::Unseen)),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => Wire::Narrowed(Box::new(Wire::Unseen)),
        FieldDefType::NumberLiteral(_)
        | FieldDefType::Range(..)
        | FieldDefType::Result(..)
        | FieldDefType::TypeParam(_)
        | FieldDefType::Unknown => Wire::Unseen,
    }
}

/// Whether `meta` narrows what serde reads of the member it is written on.
const fn narrows(meta: &ModelSchemaPropMeta) -> bool {
    !meta.check.is_empty()
        || meta.enum_pattern.is_some()
        || meta.max_items.is_some()
        || meta.max_length.is_some()
        || meta.maximum.is_some()
        || meta.min_items.is_some()
        || meta.min_length.is_some()
        || meta.minimum.is_some()
        || meta.pattern.is_some()
}

/// What serde writes and reads of the member or the slot `field`.
fn field_wire(field: &Field, parameters: &[String]) -> Wire {
    let meta = parse_model_schema_prop_attributes(&field.attrs);
    if read_by_a_function(field) || meta.as_number || meta.as_type.is_some() {
        return Wire::Unseen;
    }
    if let Some(literal) = &meta.literal {
        return match literal {
            LiteralValue::Bool(value) => Wire::Bool(Some(*value)),
            LiteralValue::Str(text) => Wire::Text(Some(vec![text.clone()])),
            LiteralValue::Integer(_) | LiteralValue::Number(_) => Wire::Unseen,
        };
    }
    let wire = wire_of(&def_of(&field.ty, parameters), 0);
    if narrows(&meta) {
        Wire::Narrowed(Box::new(wire))
    } else {
        wire
    }
}

/// The object serde writes and reads for the members `fields` declare, `closed` or not, every
/// member defaulted where `defaulted` says the container is.
fn level_wire(
    fields: &Fields,
    rename_all: Option<&str>,
    closed: bool,
    defaulted: bool,
    parameters: &[String],
) -> Wire {
    let rule = resolve_rename_rule(rename_all);
    let mut object = Object {
        closed,
        ..Object::default()
    };
    for field in fields {
        let omission = parse_serde_key_omission(&field.attrs);
        if omission.absent_from_wire() {
            continue;
        }
        if parse_serde_field_attributes(&field.attrs).flatten {
            if !flatten_into(&mut object, field, parameters) {
                return Wire::Unseen;
            }
            continue;
        }
        let Some(ident) = &field.ident else {
            return Wire::Unseen;
        };
        let name = ident.unraw().to_string();
        let mut read = Vec::new();
        if !omission.skips_deserializing {
            read.push(
                parse_read_renames(&field.attrs)
                    .rename
                    .unwrap_or_else(|| rule.apply_to_field(&name)),
            );
            read.extend(parse_serde_field_attributes(&field.attrs).aliases);
        }
        let written = parse_written_renames(&field.attrs)
            .rename
            .unwrap_or_else(|| rule.apply_to_field(&name));
        object.keys.push(Key {
            read,
            read_optional: absent_reads(field, defaulted, parameters),
            value: field_wire(field, parameters),
            written: Some((written, !omission.omits_key)),
        });
    }
    Wire::Object(Box::new(object))
}

/// Adds the keys of the flattened member `field` to `object`, and whether the expansion sees them.
fn flatten_into(object: &mut Object, field: &Field, parameters: &[String]) -> bool {
    if read_by_a_function(field) {
        return false;
    }
    let def = def_of(&field.ty, parameters);
    let optional = def.is_optional();
    let Wire::Object(member) = wire_of(&without_its_option(&def), 0) else {
        return false;
    };
    for mut key in member.keys {
        if optional {
            key.read_optional = true;
            if let Some((_, always)) = &mut key.written {
                *always = false;
            }
        }
        object.keys.push(key);
    }
    if let Some(rest) = member.rest {
        if object.rest.is_some() {
            return false;
        }
        object.rest = Some(rest);
    }
    true
}

/// What serde writes and reads of the data `variant` holds, with no tag: `null` for a unit
/// variant.
fn payload_wire(
    item_enum: &ItemEnum,
    variant: &Variant,
    closed: bool,
    defaulted: bool,
    parameters: &[String],
) -> Wire {
    if let Some(slot) = newtype_slot(variant) {
        return field_wire(slot, parameters);
    }
    match &variant.fields {
        Fields::Named(_) => level_wire(
            &variant.fields,
            variant_rename_all(item_enum, variant).as_deref(),
            closed,
            defaulted,
            parameters,
        ),
        Fields::Unnamed(_) => Wire::Unseen,
        Fields::Unit => Wire::Null,
    }
}

/// `payload`, the data of a variant serde writes under `named`, as `tagging` writes it.
fn tagged(tagging: &Tagging, named: String, payload: Wire, closed: bool) -> Wire {
    match tagging {
        Tagging::Untagged => payload,
        Tagging::External | Tagging::Plain => {
            if matches!(payload, Wire::Null) {
                Wire::Text(Some(vec![named]))
            } else {
                Wire::Object(Box::new(Object {
                    closed: true,
                    keys: vec![Key {
                        read: vec![named.clone()],
                        read_optional: false,
                        value: payload,
                        written: Some((named, true)),
                    }],
                    rest: None,
                }))
            }
        }
        Tagging::Internal { tag } => match payload {
            Wire::Null => Wire::Object(Box::new(Object {
                closed,
                keys: vec![Key::tag(tag.clone(), named)],
                rest: None,
            })),
            Wire::Object(mut object) => {
                object.keys.push(Key::tag(tag.clone(), named));
                Wire::Object(object)
            }
            Wire::Array(..)
            | Wire::Bool(_)
            | Wire::Float
            | Wire::Integer(..)
            | Wire::Narrowed(_)
            | Wire::OneOf(_)
            | Wire::Text(_)
            | Wire::Unseen => Wire::Unseen,
        },
        Tagging::Adjacent { content, tag } => {
            let mut keys = vec![Key::tag(tag.clone(), named)];
            if !matches!(payload, Wire::Null) {
                keys.push(Key {
                    read: vec![content.clone()],
                    read_optional: false,
                    value: payload,
                    written: Some((content.clone(), true)),
                });
            }
            Wire::Object(Box::new(Object {
                closed,
                keys,
                rest: None,
            }))
        }
    }
}

/// What serde writes and reads of each variant of `item_enum` serde writes or reads, beside it.
fn variant_wires<'item>(
    item_enum: &'item ItemEnum,
    args: &ModelSchemaArgs,
) -> Vec<(&'item Variant, Wire)> {
    let parameters = parameter_names(&item_enum.generics);
    let closed = strict::closed_by_hand(&item_enum.attrs) || strict::closes_enum(item_enum, args);
    let defaulted = derive_reads_a_default(&item_enum.attrs);
    let tagging = Tagging::of(item_enum);
    let rule = resolve_rename_rule(parse_read_renames(&item_enum.attrs).rename_all.as_deref());
    item_enum
        .variants
        .iter()
        .filter(|variant| variant_on_the_wire(variant))
        .map(|variant| {
            let named = parse_read_renames(&variant.attrs)
                .rename
                .unwrap_or_else(|| rule.apply_to_variant(&variant.ident.unraw().to_string()));
            let payload = payload_wire(item_enum, variant, closed, defaulted, &parameters);
            (variant, tagged(&tagging, named, payload, closed))
        })
        .collect()
}

/// What serde writes and reads of `item_struct`.
fn struct_wire(item_struct: &ItemStruct, args: &ModelSchemaArgs) -> Wire {
    let parameters = parameter_names(&item_struct.generics);
    if let Some(slot) = written_as_its_slot(item_struct) {
        return field_wire(slot, &parameters);
    }
    let Fields::Named(_) = &item_struct.fields else {
        return Wire::Unseen;
    };
    let closed =
        strict::closed_by_hand(&item_struct.attrs) || strict::closes_struct(item_struct, args);
    let mut wire = level_wire(
        &item_struct.fields,
        parse_read_renames(&item_struct.attrs).rename_all.as_deref(),
        closed,
        derive_reads_a_default(&item_struct.attrs),
        &parameters,
    );
    if let (Some((key, named)), Wire::Object(object)) = (struct_tag(item_struct), &mut wire) {
        object.keys.push(Key::tag(key, named));
    }
    wire
}

/// What serde writes and reads of `item`, a type the expansion looks into.
fn item_wire(item: &Item, args: &ModelSchemaArgs) -> Wire {
    let wire = if let Item::Struct(item_struct) = item {
        struct_wire(item_struct, args)
    } else if let Item::Enum(item_enum) = item {
        Wire::OneOf(
            variant_wires(item_enum, args)
                .into_iter()
                .map(|(_, wire)| wire)
                .collect(),
        )
    } else if let Item::Type(alias) = item {
        wire_of(&def_of(&alias.ty, &[]), 0)
    } else {
        return Wire::Unseen;
    };
    let checked = !args.check.is_empty()
        || args.max_length.is_some()
        || args.min_length.is_some()
        || args.pattern.is_some();
    if !args.preprocess.is_empty() {
        Wire::Unseen
    } else if checked {
        Wire::Narrowed(Box::new(wire))
    } else {
        wire
    }
}

/// Records what `item` writes and reads, for the untagged enums expanded after it that hold it.
pub(super) fn record(item: &Item, args: &ModelSchemaArgs) {
    let (ident, generics) = if let Item::Struct(item_struct) = item {
        (&item_struct.ident, &item_struct.generics)
    } else if let Item::Enum(item_enum) = item {
        (&item_enum.ident, &item_enum.generics)
    } else if let Item::Type(alias) = item {
        (&alias.ident, &alias.generics)
    } else {
        return;
    };
    let wire = if generics.type_params().next().is_some() {
        Wire::Unseen
    } else {
        item_wire(item, args)
    };
    let name = ident.to_string();
    WIRES.with(|wires| wires.borrow_mut().insert(name, wire));
}

/// The refusal of each variant of the untagged `item_enum` every value of which a variant declared
/// before it reads.
pub(super) fn errors(item_enum: &ItemEnum, args: &ModelSchemaArgs) -> Vec<TokenStream> {
    if !matches!(Tagging::of(item_enum), Tagging::Untagged) || !args.preprocess.is_empty() {
        return Vec::new();
    }
    let wires = variant_wires(item_enum, args);
    let enum_name = item_enum.ident.unraw();
    let mut refused = Vec::new();
    // A unit variant of an untagged enum is refused by a rule of its own.
    let held = |variant: &Variant| !matches!(variant.fields, Fields::Unit);
    for (later, (variant, written)) in wires.iter().enumerate() {
        let Some((earlier, read)) = wires[..later]
            .iter()
            .filter(|(earlier, _)| held(variant) && held(earlier))
            .find(|(_, read)| covers(read, written, 0) == Answer::Yes)
        else {
            continue;
        };
        let reading = earlier.ident.unraw();
        let shadowed = variant.ident.unraw();
        let fix = if covers(written, read, 0) == Answer::Yes {
            format!(
                "`{shadowed}` reads every value of `{reading}` as well, so give one of them a \
                 member serde requires that the other never writes"
            )
        } else {
            format!("Declare `{shadowed}` before `{reading}`")
        };
        let message = format!(
            "type `{enum_name}`: variant `{shadowed}` of the untagged `{enum_name}` never reads \
             back: serde reads the variants in the order declared, and `{reading}`, declared \
             before it, reads every value serde writes for `{shadowed}`. {fix}."
        );
        refused.push(
            syn::Error::new_spanned(variant, prefixed_guard_message(&message)).to_compile_error(),
        );
    }
    refused
}
