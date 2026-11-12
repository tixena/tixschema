//! The two serializers of each Kotlin type the emitter writes: `{Type}Unjudged`, the read alone,
//! and `{Type}Serializer`, the type's own, which reads through the first and judges the value
//! with `validate()`. The text is the same in a build with a schema surface and in one with none.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[cfg(feature = "serde")]
use super::fixed_arrays_models::{
    bitter_kotlin, bunt_kotlin, buntline_kotlin, cringling_kotlin, fishing_kotlin, reefband_kotlin,
    sheaving_kotlin, shroud_kotlin, shrouding_kotlin,
};
use super::kotlin_pathed::PATHED;

/// The private functions a serializer holds ahead of its `deserialize` where it reads an element
/// itself, each part from its text, placed under its step.
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

/// What a type with no rule answers.
const NO_RULE: &str = "fun validate(): List<String> = emptyList()";

/// What holds a pallet's label to one character.
const LABELLED: &str = "fun validate(): List<String> { val violations = mutableListOf<String>(); \
     if (label.codePointCount(0, label.length) < 1) violations.add(\"'label': too short: minimum \
     length is 1, got ${label.codePointCount(0, label.length)}\"); return violations }";

/// The annotation a holder reads a pallet unjudged with.
const UNJUDGED: &str = "@Serializable(with = PalletUnjudged::class)";

/// A model with one bound.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pallet {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A model held in a field, a list, a nullable and a map, and a type declared below.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bay {
    pub first: Pallet,
    pub listed: Vec<Pallet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<Pallet>,
    pub stored: HashMap<String, Pallet>,
    pub trailer: Trailer,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Trailer {
    pub plate: String,
}

/// A type with a parameter.
#[model_schema(default_types(Held = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cask<Held> {
    pub held: Vec<Held>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A type with a parameter held bare and in a list, a model behind the parameter of the first.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cellar {
    pub cask: Cask<Pallet>,
    pub casks: Vec<Cask<u32>>,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Seal(String);

/// An enum under its variants' names, one of which holds a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Load {
    Boxed(Pallet),
    Empty,
}

/// The serializer that reads `named` with no judgement, where the plugin writes it.
fn generated(named: &str) -> String {
    format!(
        "@OptIn(ExperimentalSerializationApi::class) object {named}Unjudged : \
         KSerializer<{named}> by {named}.generatedSerializer()"
    )
}

/// The serializer of `named`, a type with no parameter: its unjudged read, then `validate()`.
pub fn judged(named: &str) -> String {
    format!(
        "object {named}Serializer : KSerializer<{named}>, () -> KSerializer<{named}> {{ \
         override fun invoke(): KSerializer<{named}> = {named}Unjudged; override val \
         descriptor: SerialDescriptor = {named}Unjudged.descriptor; override fun \
         serialize(encoder: Encoder, value: {named}) = {named}Unjudged.serialize(encoder, \
         value); override fun deserialize(decoder: Decoder): {named} {{ val read = \
         {named}Unjudged.deserialize(decoder); val violations = read.validate(); if \
         (violations.isNotEmpty()) throw SerializationException(violations.joinToString(\"; \"), \
         IllegalArgumentException(\"validate()\")); return read }} }}"
    )
}

/// The serializer of `named`, a type with the one parameter `parameter`, which `bound` follows.
pub fn judged_over(named: &str, parameter: &str, bound: &str, handed: &str) -> String {
    format!(
        "class {named}Serializer<{parameter}{bound}>({handed}: \
         KSerializer<{parameter}>) : KSerializer<{named}<{parameter}>>, () -> \
         KSerializer<{named}<{parameter}>> {{ private val kotlinSchemaUnjudged = \
         {named}Unjudged({handed}); override fun invoke(): \
         KSerializer<{named}<{parameter}>> = kotlinSchemaUnjudged; override val descriptor: \
         SerialDescriptor = kotlinSchemaUnjudged.descriptor; override fun serialize(encoder: \
         Encoder, value: {named}<{parameter}>) = kotlinSchemaUnjudged.serialize(encoder, \
         value); override fun deserialize(decoder: Decoder): {named}<{parameter}> {{ val read \
         = kotlinSchemaUnjudged.deserialize(decoder); val violations = read.validate(); if \
         (violations.isNotEmpty()) throw SerializationException(violations.joinToString(\"; \"), \
         IllegalArgumentException(\"validate()\")); return read }} }}"
    )
}

/// The annotations of a type the plugin reads, which keeps the plugin's serializer.
pub fn kept(named: &str) -> String {
    format!(
        "@OptIn(ExperimentalSerializationApi::class) @Serializable(with = \
         {named}Serializer::class) @KeepGeneratedSerializer"
    )
}

/// The annotations of a brand's value class, which keeps the plugin's serializer.
#[cfg(feature = "serde")]
pub fn kept_inline(named: &str) -> String {
    format!(
        "@OptIn(ExperimentalSerializationApi::class) @JvmInline @Serializable(with = \
         {named}Serializer::class) @KeepGeneratedSerializer"
    )
}

/// The two serializers of `named`, a type with no parameter the plugin reads.
pub fn serializers(named: &str) -> String {
    format!("{} {}", generated(named), judged(named))
}

/// The serializer of a map keyed by the brand `named`, a brand over a text: each key read
/// unjudged from its text at the map's path, then judged, each message under the key's text, and
/// each value from its text under the key's text.
pub fn mapped(named: &str) -> String {
    mapped_reading(
        named,
        &format!(
            "kotlinSchemaDecoded(input.json, {named}Unjudged, JsonPrimitive(entry.key), \
             entry.key, \"\")"
        ),
    )
}

/// [`mapped`] of a brand over a brand over a text, which throws the refusal of the brand it holds
/// under the key's text as well.
pub fn mapped_over_a_brand(named: &str) -> String {
    mapped_reading(
        named,
        &format!(
            "try {{ kotlinSchemaDecoded(input.json, {named}Unjudged, JsonPrimitive(entry.key), \
             entry.key, \"\") }} catch (refused: SerializationException) {{ if \
             (refused.cause?.message != \"validate()\") throw refused; throw \
             SerializationException(\"'\" + entry.key + \"': \" + refused.message.orEmpty()) }}"
        ),
    )
}

/// The serializer of a map keyed by the brand `named`, which reads each key with `key` and then
/// judges it.
fn mapped_reading(named: &str, key: &str) -> String {
    format!(
        "class {named}MapSerializer<KotlinSchemaValue>(private val keySerializer: \
         KSerializer<{named}>, private val valueSerializer: KSerializer<KotlinSchemaValue>) : \
         KSerializer<Map<{named}, KotlinSchemaValue>> {{ private val whole = \
         MapSerializer(keySerializer, valueSerializer); override val descriptor: SerialDescriptor \
         = whole.descriptor; override fun serialize(encoder: Encoder, value: Map<{named}, \
         KotlinSchemaValue>) = whole.serialize(encoder, value); {DECODED}{PATHED}override fun \
         deserialize(decoder: Decoder): Map<{named}, KotlinSchemaValue> {{ val input = decoder \
         as JsonDecoder; val element = input.decodeJsonElement(); return kotlinSchemaAt(input) \
         {{ (element as? JsonObject ?: throw SerializationException(\"expected an object, found \
         \" + element)).entries.associate {{ entry -> val key = {key}; val violations = \
         key.validate(); if (violations.isNotEmpty()) throw \
         SerializationException(violations.joinToString(\"; \") {{ \"'\" + entry.key + \"': \" + \
         it }}); key to kotlinSchemaDecoded(input.json, valueSerializer, entry.value, entry.key, \
         \".\" + entry.key) }} }} }} }}"
    )
}

fn pallet(label: &str) -> Pallet {
    PalletDraft {
        label: label.to_owned(),
    }
    .validate()
    .unwrap()
}

#[test]
fn every_fixture_is_written_as_serde_writes_it() {
    let bay = BayDraft {
        first: pallet("a"),
        listed: vec![pallet("b")],
        spare: None,
        stored: HashMap::from([("north".to_owned(), pallet("c"))]),
        trailer: TrailerDraft {
            plate: "p".to_owned(),
        }
        .validate()
        .unwrap(),
    };
    assert_eq!(
        serde_json::to_value(bay).unwrap(),
        serde_json::json!({
            "first": { "label": "a" },
            "listed": [{ "label": "b" }],
            "stored": { "north": { "label": "c" } },
            "trailer": { "plate": "p" }
        })
    );
    let cellar = CellarDraft {
        cask: CaskDraft {
            held: vec![pallet("d")],
            label: "e".to_owned(),
        }
        .validate()
        .unwrap(),
        casks: vec![
            CaskDraft {
                held: vec![7],
                label: "f".to_owned(),
            }
            .validate()
            .unwrap(),
        ],
    };
    assert_eq!(
        serde_json::to_value(cellar).unwrap(),
        serde_json::json!({
            "cask": { "held": [{ "label": "d" }], "label": "e" },
            "casks": [{ "held": [7_u32], "label": "f" }]
        })
    );
    assert_eq!(
        serde_json::to_value((
            Seal::new("s".to_owned()).unwrap(),
            LoadDraft::Boxed(pallet("g")),
            LoadDraft::Empty
        ))
        .unwrap(),
        serde_json::json!(["s", { "Boxed": { "label": "g" } }, "Empty"])
    );
}

/// A class keeps the plugin's serializer under `{Type}Unjudged`, and its own serializer reads
/// through it and throws what `validate()` answers.
#[test]
fn a_class_is_read_by_the_plugins_serializer_and_judged_by_its_own() {
    assert_eq!(
        super::tests::renumbered(&pallet_kotlin::kotlin_definition()),
        format!(
            "{} data class Pallet(val label: String) {{ {LABELLED} }} {}",
            kept("Pallet"),
            serializers("Pallet")
        )
    );
}

/// A holder names the unjudged serializer of each model it holds where `validate()` walks it: on
/// the property, and on the type inside a list and a map. A type declared below the holder is
/// read by its own serializer.
#[test]
fn a_holder_reads_each_model_it_walks_with_no_judgement() {
    let bay = super::tests::renumbered(&bay_kotlin::kotlin_definition());
    assert!(
        bay.starts_with(&format!(
            "{} data class Bay({UNJUDGED} val first: Pallet, val listed: List<{UNJUDGED} \
             Pallet>, {UNJUDGED} val spare: Pallet? = null, val stored: Map<String, {UNJUDGED} \
             Pallet>, val trailer: Trailer) {{ fun validate(): List<String> {{",
            kept("Bay")
        )),
        "got: {bay}"
    );
    assert!(bay.ends_with(&serializers("Bay")), "got: {bay}");
    assert_eq!(
        super::tests::renumbered(&trailer_kotlin::kotlin_definition()),
        format!(
            "{} data class Trailer(val plate: String) {{ {NO_RULE} }} {}",
            kept("Trailer"),
            serializers("Trailer")
        )
    );
}

/// A type with a parameter writes both serializers as classes over one serializer per parameter.
/// A holder names the unjudged one, and a model behind the parameter is read by its own.
#[test]
fn a_type_with_a_parameter_takes_one_serializer_per_parameter() {
    assert_eq!(
        super::tests::renumbered(&cask_kotlin::kotlin_definition()),
        format!(
            "{} data class Cask<Held>(val held: List<Held>, val label: String) {{ {LABELLED} }} \
             @OptIn(ExperimentalSerializationApi::class) class CaskUnjudged<Held>(heldSerializer: \
             KSerializer<Held>) : KSerializer<Cask<Held>> by \
             Cask.generatedSerializer(heldSerializer) {}",
            kept("Cask"),
            judged_over("Cask", "Held", "", "heldSerializer")
        )
    );
    let cellar = super::tests::renumbered(&cellar_kotlin::kotlin_definition());
    assert!(
        cellar.contains(
            "data class Cellar(@Serializable(with = CaskUnjudged::class) val cask: Cask<Pallet>, \
             val casks: List<@Serializable(with = CaskUnjudged::class) Cask<@Serializable(with = KotlinNumber1::class) UInt>>) {"
        ),
        "got: {cellar}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_brand_is_read_by_the_plugins_serializer_and_judged_by_its_own() {
    assert_eq!(
        super::tests::renumbered(&seal_kotlin::kotlin_definition()),
        format!(
            "{} value class Seal(val value: String) {{ fun kotlinSchemaText(): String = value; \
             fun kotlinSchemaLength(): Int = value.codePointCount(0, value.length); \
             {NO_RULE} }} {} {}",
            kept_inline("Seal"),
            serializers("Seal"),
            mapped("Seal")
        )
    );
}

/// The serializer the emitter writes for an enum is its unjudged read, which reads the model a
/// variant holds unjudged, and the enum's own serializer judges the variant it read.
#[test]
fn an_enum_reads_what_a_variant_holds_unjudged_and_judges_the_variant() {
    let load = super::tests::renumbered(&load_kotlin::kotlin_definition());
    for held in [
        format!(
            "@Serializable(with = LoadSerializer::class) sealed interface Load {{ fun validate(): \
             List<String> }} @Serializable data class LoadBoxed({UNJUDGED} val value: Pallet) : \
             Load {{"
        ),
        "} object LoadUnjudged : KSerializer<Load> { override val descriptor: SerialDescriptor = \
         buildClassSerialDescriptor(\"Load\");"
            .to_owned(),
        "is LoadBoxed -> buildJsonObject { put(\"Boxed\", \
         output.json.encodeToJsonElement(PalletUnjudged, value.value)) }"
            .to_owned(),
        "\"Boxed\" -> kotlinSchemaUnder(\".\" + tag) { LoadBoxed(kotlinSchemaDecoded(input.json, \
         PalletUnjudged, data, \"Boxed\", \"\"))"
            .to_owned(),
    ] {
        assert!(load.contains(&held), "no `{held}` in: {load}");
    }
    assert!(load.ends_with(&judged("Load")), "got: {load}");
}

/// The class `kotlin` declares that reads a list of `length` items, and refuses one of any other.
#[cfg(feature = "serde")]
fn array_class(kotlin: &str, length: usize) -> String {
    let declared = |named: &str| {
        format!(
            "class {named}<Item>(item: KSerializer<Item>) : KSerializer<List<Item>> {{ private \
             val listed = ListSerializer(item); override val descriptor: SerialDescriptor = \
             listed.descriptor; override fun serialize(encoder: Encoder, value: List<Item>) = \
             listed.serialize(encoder, value); {PATHED}override fun deserialize(decoder: \
             Decoder): List<Item> {{ val read = listed.deserialize(decoder); return \
             kotlinSchemaAt(decoder) {{ if (read.size != {length}) throw \
             SerializationException(\"invalid length \" + read.size + \", expected {length}\"); \
             read }} }} }}"
        )
    };
    let named = kotlin
        .match_indices("class KotlinArray")
        .map(|(at, _)| {
            let digits: String = kotlin[at + "class KotlinArray".len()..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            format!("KotlinArray{digits}")
        })
        .find(|named| kotlin.contains(&declared(named)));
    assert!(named.is_some(), "no class of {length} items in {kotlin}");
    named.unwrap()
}

/// A level written `[T; N]` is read through a class the item declares, which refuses an array of
/// another length, wherever a read reads it: named where the plugin reads it, on the property, in
/// the type an alias names and inside a map's value and a type argument, and composed where the
/// emitter writes the read, beside a flattened member, in a tuple's position and in a variant's
/// slot. A list of any length is read as it was.
#[cfg(feature = "serde")]
#[test]
fn a_fixed_array_is_read_through_the_class_of_its_length() {
    let bunt = super::tests::renumbered(&bunt_kotlin::kotlin_definition());
    assert!(
        bunt.contains("data class Bunt(val held: List<@Serializable(with = KotlinNumber1::class) UByte>, val label: String)")
            && !bunt.contains("KotlinArray"),
        "got: {bunt}"
    );
    let composed = |kotlin: &str| format!("{}(KotlinNumber1)", array_class(kotlin, 2));
    let annotated =
        |kotlin: &str| format!("@Serializable(with = {}::class)", array_class(kotlin, 2));
    let reefband = super::tests::renumbered(&reefband_kotlin::kotlin_definition());
    let buntline = super::tests::renumbered(&buntline_kotlin::kotlin_definition());
    let shroud = super::tests::renumbered(&shroud_kotlin::kotlin_definition());
    let shrouding = super::tests::renumbered(&shrouding_kotlin::kotlin_definition());
    let fishing = super::tests::renumbered(&fishing_kotlin::kotlin_definition());
    let sheaving = super::tests::renumbered(&sheaving_kotlin::kotlin_definition());
    let cringling = super::tests::renumbered(&cringling_kotlin::kotlin_definition());
    let bitter = super::tests::renumbered(&bitter_kotlin::kotlin_definition());
    for (kotlin, held) in [
        (
            &bitter,
            format!(
                "typealias BitterType = {} List<UByte>; typealias Bitter = BitterType;",
                annotated(&bitter)
            ),
        ),
        (
            &reefband,
            "data class Reefband(val bunt: Bunt, val pair: List<UByte>) {".to_owned(),
        ),
        (
            &reefband,
            format!(
                "val pair = kotlinSchemaDecoded(input.json, {}, (obj[\"pair\"] ?: throw \
                 SerializationException(\"missing field `pair`\")), \"pair\", \".pair\");",
                composed(&reefband)
            ),
        ),
        (
            &buntline,
            format!(
                "kotlinSchemaDecoded(input.json, {}, array[1], \"1\", \"[1]\")",
                composed(&buntline)
            ),
        ),
        (
            &shroud,
            format!(
                "data class ShroudPaired(val value: {} List<UByte>) : Shroud",
                annotated(&shroud)
            ),
        ),
        (
            &shroud,
            format!(
                "\"Paired\" -> kotlinSchemaUnder(\".\" + tag) {{ \
                 ShroudPaired(kotlinSchemaDecoded(input.json, {}, data, \"Paired\", \"\")) }}",
                composed(&shroud)
            ),
        ),
        (
            &shrouding,
            format!(
                "data class Shrouding(val byDock: Map<String, {} List<UByte>>)",
                annotated(&shrouding)
            ),
        ),
        (
            &fishing,
            format!(
                "data class Fishing(val maybe: {} List<UByte>? = null)",
                annotated(&fishing)
            ),
        ),
        (
            &sheaving,
            format!("val catted: Catting<{} List<UByte>>", annotated(&sheaving)),
        ),
        (
            &cringling,
            format!(
                "data class Cringling<T>(val pair: @Serializable(with = {}::class) List<T>)",
                array_class(&cringling, 2)
            ),
        ),
    ] {
        let plain = super::tests::without_number_readers(kotlin);
        assert!(plain.contains(&held), "no `{held}` in: {kotlin}");
    }
}
