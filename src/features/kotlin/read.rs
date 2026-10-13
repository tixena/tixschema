//! The read of each Kotlin type the emitter writes. A type has two serializers: `{Type}Unjudged`,
//! which reads and writes as the type does, and `{Type}Serializer`, the type's own, which reads
//! through the first and throws what `validate()` answers. A holder reads each model its
//! `validate()` asks with the unjudged one, so the whole value is judged once.

use super::validate;
use super::{
    kotlin_base, kotlin_serializer_expr, kotlin_serializer_param_name, kotlin_typename,
    lookup_kotlin_name, tuple_wrapper_typename,
};
use crate::field_type::{FieldDef, FieldDefType, is_sequence_wrapper};

/// What a serializer the emitter writes binds each type parameter to: `KSerializer<T>.nullable`
/// requires it, for an `Option<T>` member.
const COMPOSED: &str = ": Any";

/// What opts a declaration into `@KeepGeneratedSerializer` and the serializer it keeps.
const OPTED_IN: &str = "@OptIn(ExperimentalSerializationApi::class)";

/// The Kotlin type of `field`, each model it holds annotated with its unjudged serializer where it
/// is written: inside each level of a list, a nullable and the values of a map. A tuple is the
/// wrapper class whose serializer reads the model of a position unjudged.
fn annotated_base(field: &FieldDef) -> String {
    let scalar = if let FieldDefType::SiblingType(name, arguments) = &field.field_type {
        if let [element] = arguments.as_slice()
            && is_sequence_wrapper(name)
        {
            return annotated_base(&field.collection_element_field(element));
        }
        let written = kotlin_base(&innermost(field));
        unjudged_of(name).map_or_else(
            || written.clone(),
            |unjudged| format!("@Serializable(with = {unjudged}::class) {written}"),
        )
    } else if let FieldDefType::Map(key, value) = &field.field_type {
        format!(
            "Map<{}, {}>",
            kotlin_typename(key),
            annotated_typename(value)
        )
    } else if let FieldDefType::Tuple(positions) = &field.field_type {
        tuple_wrapper_typename(positions, true)
    } else {
        kotlin_base(&innermost(field))
    };
    (0..field.array_depth).fold(scalar, |wrapped, level| {
        let item = if field.is_nullable_at(level) {
            format!("{wrapped}?")
        } else {
            wrapped
        };
        format!("List<{item}>")
    })
}

/// [`annotated_base`] with the `?` an optional `field` carries.
fn annotated_typename(field: &FieldDef) -> String {
    let base = annotated_base(field);
    if field.is_optional() {
        format!("{base}?")
    } else {
        base
    }
}

/// The serializers of the type parameters, as one serializer hands them to another.
fn arguments(type_parameters: &[String]) -> String {
    type_parameters
        .iter()
        .map(|parameter| kotlin_serializer_param_name(parameter))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The serializer a member that names a function is read through where the plugin reads it:
/// `read_by`, the one the member is read with, behind `correction`, the Kotlin expression of
/// `element` corrected.
pub fn corrected_member(named: &str, ty: &str, read_by: &str, correction: &str) -> String {
    format!(
        "object {named} : JsonTransformingSerializer<{ty}>({read_by}) {{ {} }}",
        transform(correction)
    )
}

/// The unjudged serializer of a type the plugin writes a serializer for: the plugin's own, under
/// the name a holder writes.
pub fn generated(export_name: &str, generic_params: &str, type_parameters: &[String]) -> String {
    if type_parameters.is_empty() {
        format!(
            "{OPTED_IN} object {export_name}Unjudged : KSerializer<{export_name}> by \
             {export_name}.generatedSerializer()"
        )
    } else {
        format!(
            "{OPTED_IN} class {export_name}Unjudged{}({}) : \
             KSerializer<{export_name}{generic_params}> by {export_name}.generatedSerializer({})",
            parameter_list(type_parameters, ""),
            handed(type_parameters, ""),
            arguments(type_parameters)
        )
    }
}

/// One constructor parameter per type parameter, its own serializer, each written after `kept`.
fn handed(type_parameters: &[String], kept: &str) -> String {
    type_parameters
        .iter()
        .map(|parameter| {
            format!(
                "{kept}{}: KSerializer<{parameter}>",
                kotlin_serializer_param_name(parameter)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// A reference to the `KSerializer` a serializer the emitter writes reads `held` with, a value the
/// member typed `member` holds: each model `validate()` asks of the member is read unjudged.
pub fn held_serializer(member: &FieldDef, held: &FieldDef, type_parameters: &[String]) -> String {
    if validate::asks_the_held(member) && holds_an_unjudged(held) {
        unjudged_serializer(held, type_parameters)
    } else {
        kotlin_serializer_expr(held, type_parameters)
    }
}

/// Whether a value typed `field` holds a model an unjudged serializer reads, where `validate()`
/// walks to one: under a nullable, each level of a list, the values of a map and the positions
/// of a tuple.
fn holds_an_unjudged(field: &FieldDef) -> bool {
    if let FieldDefType::SiblingType(name, arguments) = &field.field_type {
        if let [element] = arguments.as_slice()
            && is_sequence_wrapper(name)
        {
            return holds_an_unjudged(&field.collection_element_field(element));
        }
        unjudged_of(name).is_some()
    } else if let FieldDefType::Map(_, value) = &field.field_type {
        holds_an_unjudged(value)
    } else if let FieldDefType::Tuple(positions) = &field.field_type {
        positions.iter().any(in_a_position)
    } else {
        false
    }
}

/// Whether the position of a tuple typed `field` holds a model its wrapper reads unjudged: one
/// `validate()` asks of the position. A tuple in a position keeps its own wrapper's serializer.
fn in_a_position(field: &FieldDef) -> bool {
    !matches!(field.field_type, FieldDefType::Tuple(_))
        && validate::asks_the_held(field)
        && holds_an_unjudged(field)
}

/// `field` under its list levels and its nullable ones: the value at the innermost level.
fn innermost(field: &FieldDef) -> FieldDef {
    let mut bare = field.clone();
    bare.array_depth = 0;
    bare.nullable_levels.clear();
    bare
}

/// The serializer of `export_name`, the type's own: it reads through the unjudged one and throws
/// what `validate()` answers of the value read, joined by `; `. `composed` says the unjudged one
/// is written by the emitter, which binds each type parameter to `Any`.
pub fn judged(
    export_name: &str,
    generic_params: &str,
    type_parameters: &[String],
    composed: bool,
) -> String {
    let own = format!("{export_name}{generic_params}");
    let (opens, property, unjudged) = if type_parameters.is_empty() {
        (
            format!("object {export_name}Serializer"),
            String::new(),
            format!("{export_name}Unjudged"),
        )
    } else {
        let bound = if composed { COMPOSED } else { "" };
        (
            format!(
                "class {export_name}Serializer{}({})",
                parameter_list(type_parameters, bound),
                handed(type_parameters, "")
            ),
            format!(
                "private val kotlinSchemaUnjudged = {export_name}Unjudged({}); ",
                arguments(type_parameters)
            ),
            "kotlinSchemaUnjudged".to_owned(),
        )
    };
    format!(
        "{opens} : KSerializer<{own}> {{ {property}override val descriptor: SerialDescriptor = \
         {unjudged}.descriptor; override fun serialize(encoder: Encoder, value: {own}) = \
         {unjudged}.serialize(encoder, value); override fun deserialize(decoder: Decoder): {own} \
         {{ val read = {unjudged}.deserialize(decoder); val violations = read.validate(); if \
         (violations.isNotEmpty()) throw SerializationException(violations.joinToString(\"; \")); \
         return read }} }}"
    )
}

/// The annotations of a class the plugin writes a serializer for, which it keeps beside the
/// type's own.
pub fn kept(export_name: &str) -> String {
    format!(
        "{OPTED_IN} @Serializable(with = {export_name}Serializer::class) @KeepGeneratedSerializer"
    )
}

/// [`kept`] for the value class of a brand.
pub fn kept_inline(export_name: &str) -> String {
    format!(
        "{OPTED_IN} @JvmInline @Serializable(with = {export_name}Serializer::class) \
         @KeepGeneratedSerializer"
    )
}

/// The annotations of a plain enum that names a function: the one serializer it has is the
/// unjudged one, over the plugin's.
pub fn kept_plain(export_name: &str) -> String {
    format!(
        "{OPTED_IN} @Serializable(with = {export_name}Unjudged::class) @KeepGeneratedSerializer"
    )
}

/// A reference to the `KSerializer` a serializer the emitter writes reads the member typed
/// `field` with.
pub fn member_serializer(field: &FieldDef, type_parameters: &[String]) -> String {
    held_serializer(field, field, type_parameters)
}

/// `<T, U>`, each parameter followed by `bound`.
fn parameter_list(type_parameters: &[String], bound: &str) -> String {
    let listed = type_parameters
        .iter()
        .map(|parameter| format!("{parameter}{bound}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("<{listed}>")
}

/// A reference to the `KSerializer` the wrapper of a tuple a `validate()` walks reads the
/// position typed `field` with.
pub fn position_serializer(field: &FieldDef) -> String {
    if in_a_position(field) {
        unjudged_serializer(field, &[])
    } else {
        kotlin_serializer_expr(field, &[])
    }
}

/// How the member typed `field` is declared where the plugin reads it: the annotation that names
/// an unjudged serializer on the property, and the type, which names one inside a list and a map.
/// A model is read unjudged where `validate()` asks it for what it reports, and nowhere else.
pub fn plugin_read(field: &FieldDef) -> (String, String) {
    if !(validate::asks_the_held(field) && holds_an_unjudged(field)) {
        return (String::new(), kotlin_typename(field));
    }
    if field.array_depth == 0
        && let FieldDefType::SiblingType(name, _) = &field.field_type
        && let Some(unjudged) = unjudged_of(name)
    {
        return (
            format!("@Serializable(with = {unjudged}::class) "),
            kotlin_typename(field),
        );
    }
    (String::new(), annotated_typename(field))
}

/// The Kotlin type the member typed `field` is declared with where a serializer of its own reads
/// it, and the serializer it is read with behind that one.
pub fn read_whole(field: &FieldDef, type_parameters: &[String]) -> (String, String) {
    if matches!(field.field_type, FieldDefType::Tuple(_)) {
        // Each ask of a tuple's name queues another wrapper, so the name is asked once.
        let (_, ty) = plugin_read(field);
        let read_by = format!("serializer<{ty}>()");
        (ty, read_by)
    } else {
        (
            kotlin_typename(field),
            member_serializer(field, type_parameters),
        )
    }
}

/// What a serializer that corrects its payload overrides: `correction` is the Kotlin expression
/// of `element` corrected.
fn transform(correction: &str) -> String {
    format!("override fun transformDeserialize(element: JsonElement): JsonElement = {correction}")
}

/// [`generated`] for a type that corrects its payload: the plugin's serializer behind
/// `correction`, the Kotlin expression of `element` corrected.
pub fn transforming(
    export_name: &str,
    generic_params: &str,
    type_parameters: &[String],
    correction: &str,
) -> String {
    let body = transform(correction);
    if type_parameters.is_empty() {
        format!(
            "{OPTED_IN} object {export_name}Unjudged : \
             JsonTransformingSerializer<{export_name}>({export_name}.generatedSerializer()) {{ \
             {body} }}"
        )
    } else {
        format!(
            "{OPTED_IN} class {export_name}Unjudged{}({}) : \
             JsonTransformingSerializer<{export_name}{generic_params}>({export_name}.\
             generatedSerializer({})) {{ {body} }}",
            parameter_list(type_parameters, ""),
            handed(type_parameters, ""),
            arguments(type_parameters)
        )
    }
}

/// [`transforming`] for the value class of a brand with parameters, which a transforming
/// serializer writes unboxed: it delegates to the plugin's serializer, and reads through it from
/// `correction`, the Kotlin expression of the payload `input` decodes, corrected.
pub fn transforming_inline(
    export_name: &str,
    generic_params: &str,
    type_parameters: &[String],
    correction: &str,
) -> String {
    let own = format!("{export_name}{generic_params}");
    let read_by = format!(
        "{export_name}.generatedSerializer({})",
        arguments(type_parameters)
    );
    format!(
        "{OPTED_IN} class {export_name}Unjudged{}({}) : KSerializer<{own}> by {read_by} {{ \
         override fun deserialize(decoder: Decoder): {own} {{ val input = decoder as JsonDecoder; \
         return input.json.decodeFromJsonElement({read_by}, {correction}) }} }}",
        parameter_list(type_parameters, ""),
        handed(type_parameters, "private val ")
    )
}

/// The serializer that reads the type written `name` with no judgement, where the emitter wrote
/// that type above.
fn unjudged_of(name: &str) -> Option<String> {
    validate::reads_unjudged(name).then(|| {
        format!(
            "{}Unjudged",
            lookup_kotlin_name(name).unwrap_or_else(|| name.to_owned())
        )
    })
}

/// The `KSerializer` of a value typed `field`, each model it holds read by its unjudged one.
fn unjudged_serializer(field: &FieldDef, type_parameters: &[String]) -> String {
    let scalar = if let FieldDefType::SiblingType(name, written) = &field.field_type {
        if let [element] = written.as_slice()
            && is_sequence_wrapper(name)
        {
            return unjudged_serializer(&field.collection_element_field(element), type_parameters);
        }
        match unjudged_of(name) {
            Some(unjudged) if written.is_empty() => unjudged,
            // A serializer the emitter writes binds a parameter to `Any`, which no nullable is.
            Some(unjudged) if !written.iter().any(FieldDef::is_optional) => {
                let own = written
                    .iter()
                    .map(|argument| kotlin_serializer_expr(argument, type_parameters))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{unjudged}({own})")
            }
            Some(_) | None => kotlin_serializer_expr(&innermost(field), type_parameters),
        }
    } else if let FieldDefType::Map(key, value) = &field.field_type {
        format!(
            "MapSerializer({}, {})",
            kotlin_serializer_expr(key, type_parameters),
            unjudged_serializer(value, type_parameters)
        )
    } else {
        kotlin_serializer_expr(&innermost(field), type_parameters)
    };
    let listed = (0..field.array_depth).fold(scalar, |inner, level| {
        let item = if field.is_nullable_at(level) {
            format!("{inner}.nullable")
        } else {
            inner
        };
        format!("ListSerializer({item})")
    });
    if field.is_optional() {
        format!("{listed}.nullable")
    } else {
        listed
    }
}

/// The head of the unjudged serializer the emitter writes for a type: an `object`, or a class
/// over one serializer per type parameter, which its body composes.
pub fn written_head(export_name: &str, type_parameters: &[String]) -> String {
    if type_parameters.is_empty() {
        format!("object {export_name}Unjudged")
    } else {
        format!(
            "class {export_name}Unjudged{}({})",
            parameter_list(type_parameters, COMPOSED),
            handed(type_parameters, "private val ")
        )
    }
}
