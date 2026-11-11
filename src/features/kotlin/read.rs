//! The read of each Kotlin type the emitter writes. A type has two serializers: `{Type}Unjudged`,
//! which reads and writes as the type does, and `{Type}Serializer`, the type's own, which reads
//! through the first and throws what `validate()` answers. A holder reads each model its
//! `validate()` asks with the unjudged one, so the whole value is judged once.

use super::texts;
use super::{arrays, validate};
use super::{
    field_reaches_type_parameter, innermost, key_object_name, key_serializer_expr, kotlin_base,
    kotlin_quoted, kotlin_serializer_expr, kotlin_serializer_param_name, kotlin_typename,
    listed_serializer, lookup_kotlin_name, map_object_name, map_serializer, marked_nullable_at,
    range_serializer, range_typename, result_serializer, result_typename, sibling_typename,
    tuple_wrapper_serializer, tuple_wrapper_typename,
};
use crate::features::text_forms::TextForm;
use crate::field_type::{FieldDef, FieldDefType, is_sequence_wrapper};
use crate::utils::is_nonstrict;

/// The private functions a serializer holds ahead of its `deserialize` where it reads an element
/// itself: a part is read from its text, where kotlinx states the path of what it refuses, and the
/// refusal is rebased onto the part's step.
pub const DECODED: &str = "private fun kotlinSchemaKinded(serializer: DeserializationStrategy<*>, \
     element: JsonElement, at: String): JsonElement { var held = serializer.descriptor; \
     while (held.isInline && held.elementsCount == 1) held = held.getElementDescriptor(0); \
     val kind = held.kind; if ((kind is PrimitiveKind && element !is JsonPrimitive) || (kind == StructureKind.LIST && \
     element !is JsonArray) || (kind == StructureKind.MAP && element !is JsonObject)) throw \
     SerializationException(\"'\" + at + \"': \" + kind + \" cannot be read from \" + when (element) { \
     is JsonArray -> \"an array\"; is JsonObject -> \"an object\"; else -> \"a primitive\" }); \
     return element }; private fun <KotlinSchemaRead> kotlinSchemaDecoded(json: Json, serializer: \
     DeserializationStrategy<KotlinSchemaRead>, element: JsonElement, at: String, step: String): \
     KotlinSchemaRead = kotlinSchemaUnder(step) { json.decodeFromString(serializer, \
     kotlinSchemaKinded(serializer, element, at).toString()) }; ";

/// What a serializer throws for the violations `validate()` answers: its cause marks a message that
/// names each place from the value judged, which [`PATHED`] leaves as it is.
pub const JUDGED: &str = "SerializationException(violations.joinToString(\"; \"), \
     IllegalArgumentException(\"validate()\"))";

/// The private functions a serializer holds where it refuses a value it took off the decoder: kotlinx
/// states its path only in an exception, so the refusal is rebased onto the path a second read of
/// the decoder throws with, and the refusal of a part read apart onto that part's step.
pub const PATHED: &str = "private fun kotlinSchemaRebased(refused: SerializationException, at: \
     String): SerializationException { if (refused.cause?.message == \"validate()\") return \
     refused; val said = \
     refused.message.orEmpty().replaceFirst(Regex(\"^Unexpected JSON token at offset \\\\d+: \"), \
     \"\"); return \
     SerializationException(if (\" at path: \\$\" in said) said.replaceFirst(\" at path: \\$\", \" at \
     path: \" + at) else said + \" at path: \" + at, refused) }; @OptIn(ExperimentalSerializationApi::class) \
     private inline fun <KotlinSchemaRead> kotlinSchemaAt(input: Decoder, read: () -> \
     KotlinSchemaRead): KotlinSchemaRead = try { read() } catch (refused: SerializationException) \
     { val at = try { (input as? JsonDecoder)?.decodeJsonElement(); null } catch (probe: \
     JsonDecodingException) { probe.path } ?: throw refused; throw kotlinSchemaRebased(refused, \
     at) }; private inline fun \
     <KotlinSchemaRead> kotlinSchemaUnder(step: String, read: () -> KotlinSchemaRead): \
     KotlinSchemaRead = try { read() } catch (refused: SerializationException) { throw \
     kotlinSchemaRebased(refused, \"\\$\" + step) }; ";

/// What a serializer reads a flattened member through. serde reads such a member from the keys its
/// holder buffers and places a refusal inside it at the holder, so the refusal states the root as
/// its path, which the holder's `kotlinSchemaAt` rebases, drops the place it states, and keeps the
/// key it names as unknown or missing.
const FLATTENED: &str = "private inline fun <KotlinSchemaRead> kotlinSchemaFlattened(read: () -> \
     KotlinSchemaRead): KotlinSchemaRead = try { read() } catch (refused: SerializationException) \
     { if (refused.cause?.message == \"validate()\") throw refused; throw \
     SerializationException(refused.message.orEmpty().replaceFirst(Regex(\"^Unexpected JSON token \
     at offset \\\\d+: \"), \"\").replaceFirst(Regex(\"^'[^']*': \"), \"\").replace(Regex(\" at \
     path: \\\\$[^\\\\n]*\"), \" at path: \\\\$\"), refused) }; ";

/// What a serializer reads a part of the element it took apart with: from its text, where
/// kotlinx states the path of a value it refuses, rebased onto `step`.
pub const INNER: &str = "private fun <KotlinSchemaRead> kotlinSchemaInner(json: Json, step: String, \
     serializer: DeserializationStrategy<KotlinSchemaRead>, element: JsonElement): \
     KotlinSchemaRead = kotlinSchemaUnder(step) { json.decodeFromString(serializer, \
     element.toString()) }; ";

/// The private function a serializer holds where a flattened member's type is not one the emitter
/// wrote above it. A plain enum has no members to read from an object: serde writes its variant as
/// the name of an entry over `null`, and the function hands the enum's serializer that name.
const FLATTENED_NAME: &str = "private fun kotlinSchemaFlat(serializer: DeserializationStrategy<*>, \
     element: JsonElement): JsonElement { val kind = serializer.descriptor.kind; \
     if (kind != SerialKind.ENUM || element !is JsonObject) return element; val named = element.filterKeys { key -> runCatching { \
     Json.decodeFromJsonElement(serializer, JsonPrimitive(key)) }.isSuccess }; \
     if (named.isEmpty()) throw SerializationException(\"no variant of enum \" + \
     serializer.descriptor.serialName + \" found in flattened data\"); \
     if (named.size > 1) throw SerializationException(\"more than one variant of enum \" + \
     serializer.descriptor.serialName + \" found in flattened data: \" + \
     named.keys.joinToString(\", \")); if (named.values.single() != JsonNull) throw \
     SerializationException(\"variant \" + named.keys.single() + \" of enum \" + \
     serializer.descriptor.serialName + \" is read over null\"); \
     return JsonPrimitive(named.keys.single()) }; ";

/// What a serializer the emitter writes binds each type parameter to: `KSerializer<T>.nullable`
/// requires it, for an `Option<T>` member.
const COMPOSED: &str = ": Any";

/// What opts a declaration into `@KeepGeneratedSerializer` and the serializer it keeps.
const OPTED_IN: &str = "@OptIn(ExperimentalSerializationApi::class)";

/// A declaration the emitter's own serializer reads, which names no serializer, of a member whose
/// models `validate()` asks.
const WRITTEN: Annotated = Annotated {
    models: false,
    texts: false,
    numbers: Numbers::Plain,
    unjudged: true,
};

/// How a declaration reads a number and a `bool`: a map's key is written between quotes, and is
/// read as the library reads it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Numbers {
    /// A map's key under a declaration that refuses a number written between quotes: a `bool` key
    /// is read only as `true` or `false`, and a number key as serde reads the text of one.
    Key,
    Plain,
    Refusing,
}

/// How a declaration names the serializer each value it holds is read by.
#[derive(Clone, Copy)]
struct Annotated {
    /// Each model `validate()` asks names its unjudged serializer, for the plugin to read it by.
    models: bool,
    /// Whether a number and a `bool` name the serializer that refuses one written between quotes.
    numbers: Numbers,
    /// Each text of one form names the serializer that refuses a text of any other, and each
    /// array of one length the class that refuses an array of any other.
    texts: bool,
    /// The declaration holds models `validate()` asks, which are read unjudged: a tuple is the
    /// wrapper that reads the model of a position unjudged.
    unjudged: bool,
}

/// The Kotlin type of `field`, which names a serializer inside each level of a list, a nullable, a
/// map and a type argument where `annotated` says so, for the plugin to read the value there by. A
/// type argument is read by the serializer of its own the plugin finds, a model's judged one.
fn annotated_base(field: &FieldDef, annotated: Annotated) -> String {
    let in_arguments = Annotated {
        models: false,
        texts: annotated.texts,
        numbers: annotated.numbers,
        unjudged: false,
    };
    let in_keys = Annotated {
        numbers: if annotated.numbers == Numbers::Refusing {
            Numbers::Key
        } else {
            Numbers::Plain
        },
        ..in_arguments
    };
    let scalar = if let FieldDefType::SiblingType(name, arguments) = &field.field_type {
        if let [element] = arguments.as_slice()
            && is_sequence_wrapper(name)
        {
            return annotated_base(&field.collection_element_field(element), annotated);
        }
        let written = sibling_typename(name, arguments, |argument| {
            annotated_typename(argument, in_arguments)
        });
        let key_leaf = (annotated.numbers == Numbers::Key)
            .then(|| texts::key_leaf(field))
            .flatten();
        if let Some(leaf) = key_leaf {
            return match leaf {
                texts::KeyLeaf::Brand(brand) => format!(
                    "@Serializable(with = {}::class) {written}",
                    key_object_name(&brand)
                ),
                texts::KeyLeaf::Primitive(primitive) => texts::key_object(primitive).map_or_else(
                    || primitive.kotlin_type().to_owned(),
                    |named| {
                        format!(
                            "@Serializable(with = {named}::class) {}",
                            primitive.kotlin_type()
                        )
                    },
                ),
            };
        }
        unjudged_of(name).filter(|_| annotated.models).map_or_else(
            || written.clone(),
            |unjudged| format!("@Serializable(with = {unjudged}::class) {written}"),
        )
    } else if let FieldDefType::Map(key, value) = &field.field_type {
        annotated_map((key, value), in_keys, annotated)
    } else if let FieldDefType::Tuple(positions) = &field.field_type {
        tuple_wrapper_typename(positions, annotated.unjudged)
    } else if let FieldDefType::Range(kind, bound) = &field.field_type {
        range_typename(*kind, &annotated_typename(bound, annotated))
    } else if let FieldDefType::Result(ok, err) = &field.field_type {
        result_typename(ok, err, |arm| annotated_typename(arm, annotated))
    } else {
        let written = kotlin_base(&innermost(field));
        let serializer = TextForm::of(field)
            .filter(|_| annotated.texts)
            .map(texts::serializer)
            .or_else(|| {
                let primitive = texts::Primitive::of(field)?;
                match annotated.numbers {
                    Numbers::Plain => None,
                    Numbers::Refusing => Some(texts::number_serializer(primitive)),
                    Numbers::Key => texts::key_object(primitive),
                }
            });
        serializer.map_or_else(
            || written.clone(),
            |named| format!("@Serializable(with = {named}::class) {written}"),
        )
    };
    (0..field.array_depth).fold(scalar, |wrapped, level| {
        let item = if marked_nullable_at(field, level) {
            format!("{wrapped}?")
        } else {
            wrapped
        };
        field
            .fixed_length_at(level)
            .filter(|_| annotated.texts)
            .map_or_else(
                || format!("List<{item}>"),
                |length| {
                    format!(
                        "@Serializable(with = {}::class) List<{item}>",
                        arrays::serializer(length)
                    )
                },
            )
    })
}

/// [`annotated_base`] of a map of `key` to `value`, its keys declared as `in_keys` says: one keyed
/// by a brand names the brand's map serializer where the plugin reads it.
fn annotated_map(
    (key, value): (&FieldDef, &FieldDef),
    in_keys: Annotated,
    annotated: Annotated,
) -> String {
    let map = format!(
        "Map<{}, {}>",
        annotated_typename(key, in_keys),
        annotated_typename(value, annotated)
    );
    match (in_keys.numbers == Numbers::Key)
        .then(|| texts::mapped_by(key))
        .flatten()
    {
        Some(brand) => format!(
            "@Serializable(with = {}::class) {map}",
            map_object_name(&brand)
        ),
        None => map,
    }
}

/// [`annotated_base`] with the `?` an optional `field` carries.
fn annotated_typename(field: &FieldDef, annotated: Annotated) -> String {
    let base = annotated_base(field, annotated);
    if marked_nullable_at(field, field.array_depth) {
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
pub fn corrected_member(
    named: &str,
    ty: &str,
    read_by: &str,
    correction: &str,
    key: &str,
) -> String {
    let kinded = kinded(read_by, correction, &kotlin_quoted(key));
    format!(
        "object {named} : KSerializer<{ty}> {{ {} }}",
        transformed(ty, read_by, &transform(&kinded))
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

/// A reference to the `KSerializer` a serializer the emitter writes reads `held` with: each model
/// `validate()` asks of the member is read unjudged.
pub fn held_serializer(held: &FieldDef, type_parameters: &[String]) -> String {
    if holds_an_unjudged(held) {
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
    } else if let FieldDefType::Range(_, bound) = &field.field_type {
        holds_an_unjudged(bound)
    } else if let FieldDefType::Result(ok, err) = &field.field_type {
        [ok, err]
            .into_iter()
            .any(|arm| !arm.is_unit() && holds_an_unjudged(arm))
    } else {
        false
    }
}

/// Whether the position of a tuple typed `field` holds a model its wrapper reads unjudged: one
/// `validate()` asks of the position. A tuple in a position keeps its own wrapper's serializer.
fn in_a_position(field: &FieldDef) -> bool {
    !matches!(field.field_type, FieldDefType::Tuple(_)) && holds_an_unjudged(field)
}

/// The serializer of `export_name`, the type's own: it reads through the unjudged one and throws
/// what `validate()` answers of the value read, joined by `; `. Called, it answers the unjudged
/// one, which a holder that cannot name it reads by. `composed` says the unjudged one is written
/// by the emitter, which binds each type parameter to `Any`. The serializer of each parameter in
/// `keyed` is kept and handed to `validate()`, which names a map's key by it; where `variants`
/// lists the subclasses that take them, only those are handed, as a sealed interface declares no
/// overload.
pub fn judged(
    export_name: &str,
    generic_params: &str,
    type_parameters: &[String],
    composed: bool,
    keyed_by: (&[String], &[String]),
) -> String {
    judged_counting(
        export_name,
        generic_params,
        type_parameters,
        composed,
        keyed_by,
        None,
    )
}

/// [`judged`] for a brand that counts a list default, which hands `kotlinSchemaAtFilling()` the
/// declared type `filled` names where the serializer of the parameter it names is that type's.
pub fn judged_counting(
    export_name: &str,
    generic_params: &str,
    type_parameters: &[String],
    composed: bool,
    (keyed, variants): (&[String], &[String]),
    filled: Option<(&str, &str)>,
) -> String {
    let own = format!("{export_name}{generic_params}");
    let kept: Vec<String> = keyed
        .iter()
        .cloned()
        .chain(filled.map(|(parameter, _)| parameter.to_owned()))
        .collect();
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
                kept_parameters(type_parameters, &kept)
            ),
            format!(
                "private val kotlinSchemaUnjudged = {export_name}Unjudged({}); ",
                arguments(type_parameters)
            ),
            "kotlinSchemaUnjudged".to_owned(),
        )
    };
    let handing = keyed
        .iter()
        .map(|parameter| kotlin_serializer_param_name(parameter))
        .collect::<Vec<_>>()
        .join(", ");
    let counted = filled.map_or_else(String::new, |(parameter, declared)| {
        format!(
            " + read.kotlinSchemaAtFilling(if ({}.descriptor == serializer<{declared}>().descriptor) \
             {} else \"\")",
            kotlin_serializer_param_name(parameter),
            kotlin_quoted(declared),
        )
    });
    let validated = if variants.is_empty() {
        format!("read.validate({handing}){counted}")
    } else {
        let arms = variants
            .iter()
            .map(|variant| format!("is {variant} -> read.validate({handing})"))
            .collect::<Vec<_>>()
            .join("; ");
        format!("when (read) {{ {arms}; else -> read.validate() }}")
    };
    format!(
        "{opens} : KSerializer<{own}>, () -> KSerializer<{own}> {{ {property}override fun \
         invoke(): KSerializer<{own}> = {unjudged}; override val descriptor: SerialDescriptor = \
         {unjudged}.descriptor; override fun serialize(encoder: Encoder, value: {own}) = \
         {unjudged}.serialize(encoder, value); override fun deserialize(decoder: Decoder): {own} \
         {{ val read = {unjudged}.deserialize(decoder); val violations = {validated}; if \
         (violations.isNotEmpty()) throw {JUDGED}; return read }} }}"
    )
}

/// [`handed`] with the serializer of each parameter in `kept` held as a property.
fn kept_parameters(type_parameters: &[String], kept: &[String]) -> String {
    type_parameters
        .iter()
        .map(|parameter| {
            let held = if kept.contains(parameter) {
                "private val "
            } else {
                ""
            };
            format!(
                "{held}{}: KSerializer<{parameter}>",
                kotlin_serializer_param_name(parameter)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The annotations of a class the plugin writes a serializer for, which it keeps beside the
/// type's own.
pub fn kept(export_name: &str) -> String {
    format!(
        "{OPTED_IN} @Serializable(with = {export_name}Serializer::class) @KeepGeneratedSerializer"
    )
}

/// The annotation of a class the plugin reads that reads past a key no member reads, which is the
/// class of a `nonstrict` type, and nothing for any other. A class that [`kept`] annotates is
/// opted in already.
pub fn ignoring_unknown_keys() -> &'static str {
    if is_nonstrict() {
        " @JsonIgnoreUnknownKeys"
    } else {
        ""
    }
}

/// [`ignoring_unknown_keys`] for a class that is opted into nothing yet.
pub fn opted_in_ignoring_unknown_keys() -> &'static str {
    if is_nonstrict() {
        "@OptIn(ExperimentalSerializationApi::class) @JsonIgnoreUnknownKeys "
    } else {
        ""
    }
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

/// [`member_serializer`] of a member already declared `declared`: a tuple is read by the wrapper
/// that declaration names, a walked and an unwalked wrapper of one tuple being two classes, and a
/// wrapper that declares a type parameter is handed the serializer of each.
pub fn declared_serializer(field: &FieldDef, declared: &str, type_parameters: &[String]) -> String {
    let FieldDefType::Tuple(positions) = &field.field_type else {
        return member_serializer(field, type_parameters);
    };
    if field_reaches_type_parameter(field, type_parameters) {
        listed_serializer(
            field,
            tuple_wrapper_serializer(positions, member_read_unjudged(field)),
        )
    } else {
        format!("serializer<{declared}>()")
    }
}

/// The Kotlin type the member typed `field` is declared with where a serializer the emitter writes
/// reads it by [`member_serializer`]: each tuple that serializer reads by its walked wrapper is
/// named by it.
pub fn declared_typename(field: &FieldDef) -> String {
    if holds_an_unjudged(field) {
        annotated_typename(field, WRITTEN)
    } else {
        kotlin_typename(field)
    }
}

/// A reference to the `KSerializer` a serializer the emitter writes reads the member typed
/// `field` with.
pub fn member_serializer(field: &FieldDef, type_parameters: &[String]) -> String {
    held_serializer(field, type_parameters)
}

/// Whether [`member_serializer`] reads the member typed `field` with an unjudged serializer.
pub fn member_read_unjudged(field: &FieldDef) -> bool {
    holds_an_unjudged(field)
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
/// position typed `field` with, composed from the serializers of `type_parameters`, the
/// wrapper's own.
pub fn position_serializer(field: &FieldDef, type_parameters: &[String]) -> String {
    if in_a_position(field) {
        unjudged_serializer(field, type_parameters)
    } else {
        kotlin_serializer_expr(field, type_parameters)
    }
}

/// The Kotlin type the wrapper of a tuple a `validate()` walks declares the position typed `field`
/// with: the type [`position_serializer`] reads.
pub fn position_typename(field: &FieldDef) -> String {
    if in_a_position(field) {
        annotated_typename(field, WRITTEN)
    } else {
        kotlin_typename(field)
    }
}

/// How the member typed `field` is declared where the plugin reads it: the annotation that names
/// an unjudged serializer on the property, and the type, which names one inside a list and a map,
/// the serializer of each text of one form and the class of each array of one length where it is
/// written. A model is read unjudged where `validate()` asks it for what it reports, and nowhere
/// else.
pub fn plugin_read(field: &FieldDef) -> (String, String) {
    let models = holds_an_unjudged(field);
    if !models && !texts::declared(field) && !arrays::declared(field) {
        return (String::new(), kotlin_typename(field));
    }
    if models
        && field.array_depth == 0
        && let FieldDefType::SiblingType(name, _) = &field.field_type
        && let Some(unjudged) = unjudged_of(name)
    {
        return (
            format!("@Serializable(with = {unjudged}::class) "),
            with_text_forms(field),
        );
    }
    (
        String::new(),
        annotated_typename(
            field,
            Annotated {
                models,
                texts: true,
                numbers: Numbers::Refusing,
                unjudged: models,
            },
        ),
    )
}

/// The Kotlin type the member typed `field` is declared with where a serializer of its own reads
/// it, and the serializer it is read with behind that one.
pub fn read_whole(field: &FieldDef, type_parameters: &[String]) -> (String, String) {
    let ty = declared_typename(field);
    let read_by = declared_serializer(field, &ty, type_parameters);
    (ty, read_by)
}

/// The read of `element` by `serializer` under `json`, which refuses an element of another kind
/// than the serializer reads with a `SerializationException` naming `at`, a Kotlin expression of
/// the member's key or position, and places a refusal under `step`, the Kotlin expression of the
/// path from the element the serializer took apart. Needs [`with_decoded`] or [`helpers_for`] on
/// the serializer that holds it.
pub fn decoded(json: &str, serializer: &str, element: &str, at: &str, step: &str) -> String {
    format!("kotlinSchemaDecoded({json}, {serializer}, {element}, {at}, {step})")
}

/// The Kotlin expression of the step to the entry under `key`, the Kotlin expression of a key, as
/// kotlinx writes a path.
pub fn keyed_step(key: &str) -> String {
    key.strip_prefix('"')
        .map_or_else(|| format!("\".\" + {key}"), |quoted| format!("\".{quoted}"))
}

/// The Kotlin expression of the step to the position `slot` of an array, as kotlinx writes a path.
pub fn slot_step(slot: usize) -> String {
    format!("\"[{slot}]\"")
}

/// The statements of a `deserialize` that reads `element`, a Kotlin expression of what it takes off
/// `decoder`, and answers `read`, statements over `element` that end in what is read: a refusal is
/// placed at the path where `decoder` read the element.
pub fn read_at(decoder: &str, element: &str, read: &str) -> String {
    format!("val element = {element}; return kotlinSchemaAt({decoder}) {{ {read} }}")
}

/// `element`, the Kotlin expression of a payload, refused as [`decoded`] refuses it when it is of
/// another kind than `serializer` reads.
pub fn kinded(serializer: &str, element: &str, at: &str) -> String {
    format!("kotlinSchemaKinded({serializer}, {element}, {at})")
}

/// The private functions the reads in `declarations`, the members of a serializer, name.
pub fn helpers_for(declarations: &str) -> String {
    let decoded = declarations.contains("kotlinSchemaDecoded(")
        || declarations.contains("kotlinSchemaKinded(");
    let flat = declarations.contains("kotlinSchemaFlat(");
    let inner = declarations.contains("kotlinSchemaInner(");
    let pathed = inner
        || declarations.contains("kotlinSchemaDecoded(")
        || declarations.contains("kotlinSchemaAt(")
        || declarations.contains("kotlinSchemaAt<")
        || declarations.contains("kotlinSchemaUnder(");
    let flattened = declarations.contains("kotlinSchemaFlattened {");
    format!(
        "{}{}{}{}{}",
        if decoded { DECODED } else { "" },
        if flat { FLATTENED_NAME } else { "" },
        if pathed { PATHED } else { "" },
        if inner { INNER } else { "" },
        if flattened { FLATTENED } else { "" }
    )
}

/// `read`, the read of a flattened member, through [`FLATTENED`]: a refusal inside the member
/// names its holder, as serde names it.
pub fn flattened(read: &str) -> String {
    format!("kotlinSchemaFlattened {{ {read} }}")
}

/// `declarations`, the members of a serializer, behind the private functions its reads name.
pub fn with_decoded(declarations: &str) -> String {
    format!("{}{declarations}", helpers_for(declarations))
}

/// `element`, the Kotlin expression of the object a flattened member is handed, as the member's
/// serializer reads it: a plain enum is handed the name of the one entry it is written as.
pub fn flat(serializer: &str, element: &str) -> String {
    format!("kotlinSchemaFlat({serializer}, {element})")
}

/// What a serializer that corrects its payload, or refuses a struct's own tag of another value,
/// reads its element through: `correction` is the Kotlin expression of `element` corrected and
/// checked.
pub fn transform(correction: &str) -> String {
    format!("private fun transformDeserialize(element: JsonElement): JsonElement = {correction}")
}

/// What a serializer that writes a struct's own tag, under `key`, ahead of the struct's members
/// writes its element through: the plugin writes the property that holds the tag after the
/// constructor's.
pub fn tag_first(key: &str) -> String {
    let written = kotlin_quoted(key);
    format!(
        "private fun transformSerialize(element: JsonElement): JsonElement = \
         JsonObject(mapOf({written} to element.jsonObject.getValue({written})) + \
         element.jsonObject)"
    )
}

/// The members of a serializer of `ty` that reads and writes through `held`, behind `body`, a
/// [`transform`] and maybe a [`tag_first`]: it reads `held` from the text of the element
/// `transformDeserialize` answers, so a refusal is placed at the path of the element it took.
fn transformed(ty: &str, held: &str, body: &str) -> String {
    let serialize = if body.contains("fun transformSerialize(") {
        format!(
            "override fun serialize(encoder: Encoder, value: {ty}) {{ val output = encoder as \
             JsonEncoder; output.encodeJsonElement(transformSerialize(output.json.\
             encodeToJsonElement(held, value))) }}"
        )
    } else {
        format!(
            "override fun serialize(encoder: Encoder, value: {ty}) = held.serialize(encoder, value)"
        )
    };
    let read = read_at(
        "input",
        "input.decodeJsonElement()",
        "input.json.decodeFromString(held, transformDeserialize(element).toString())",
    );
    let members = format!(
        "override fun deserialize(decoder: Decoder): {ty} {{ val input = decoder as JsonDecoder; \
         {read} }}; {body}"
    );
    format!(
        "private val held = {held}; override val descriptor: SerialDescriptor = held.descriptor; \
         {serialize}; {}{members}",
        helpers_for(&members)
    )
}

/// [`generated`] for a type that corrects its payload or holds its own tag: the plugin's serializer
/// behind `body`, the [`transform`] and the [`tag_first`] it reads and writes through.
pub fn transforming(
    export_name: &str,
    generic_params: &str,
    type_parameters: &[String],
    body: &str,
) -> String {
    let own = format!("{export_name}{generic_params}");
    let held = format!(
        "{export_name}.generatedSerializer({})",
        arguments(type_parameters)
    );
    if type_parameters.is_empty() {
        format!(
            "{OPTED_IN} object {export_name}Unjudged : KSerializer<{own}> {{ {} }}",
            transformed(&own, &held, body)
        )
    } else {
        format!(
            "{OPTED_IN} class {export_name}Unjudged{}({}) : KSerializer<{own}> {{ {} }}",
            parameter_list(type_parameters, ""),
            handed(type_parameters, ""),
            transformed(&own, &held, body)
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
    let read = read_at(
        "input",
        correction,
        &format!(
            "input.json.decodeFromString({read_by}, {}.toString())",
            kinded(&read_by, "element", &kotlin_quoted(export_name))
        ),
    );
    format!(
        "{OPTED_IN} class {export_name}Unjudged{}({}) : KSerializer<{own}> by {read_by} {{ \
         {}override fun deserialize(decoder: Decoder): {own} {{ val input = decoder as JsonDecoder; \
         {read} }} }}",
        parameter_list(type_parameters, ""),
        handed(type_parameters, "private val "),
        helpers_for(&read),
    )
}

/// The Kotlin type of `field` where the plugin reads it, which names the serializer of each text of
/// one form and the class of each array of one length where it is written: the value of a brand,
/// and the type an alias names.
pub fn with_text_forms(field: &FieldDef) -> String {
    annotated_typename(
        field,
        Annotated {
            models: false,
            texts: true,
            numbers: Numbers::Refusing,
            unjudged: false,
        },
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
pub(super) fn unjudged_serializer(field: &FieldDef, type_parameters: &[String]) -> String {
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
        map_serializer(
            key,
            &key_serializer_expr(key, type_parameters),
            &unjudged_serializer(value, type_parameters),
        )
    } else if let FieldDefType::Tuple(positions) = &field.field_type {
        tuple_wrapper_serializer(positions, true)
    } else if let FieldDefType::Range(kind, bound) = &field.field_type {
        range_serializer(*kind, &unjudged_serializer(bound, type_parameters))
    } else if let FieldDefType::Result(ok, err) = &field.field_type {
        result_serializer(ok, err, |arm| unjudged_serializer(arm, type_parameters))
    } else {
        kotlin_serializer_expr(&innermost(field), type_parameters)
    };
    listed_serializer(field, scalar)
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
