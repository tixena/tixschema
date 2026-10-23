//! `validate()` on every Kotlin type the emitter writes: the text of each shape, which is the
//! same in a build with a schema surface and in one with none. A test whose fixture writes a
//! serde attribute the emitter reads is compiled with the `serde` feature alone.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[cfg(feature = "serde")]
use super::read::judged;
use super::read::{judged_over, kept, kept_inline, serializers};
#[cfg(feature = "serde")]
use super::tests::{
    audit_kotlin, date_value_kotlin, dynamic_value_kotlin, external_tagged_kotlin,
    simple_choice_kotlin, status_kotlin, window_error_kotlin,
};
use super::tests::{
    correlation_id_kotlin, generic_pair_kotlin, holds_tuple_kotlin, ping_kotlin, widget_kotlin,
    wrapper_kotlin,
};

/// What a type with no rule answers.
const NO_RULE: &str = "fun validate(): List<String> = emptyList()";

/// What a holder writes beside a `validate()` it calls on a name the emitter has not written a
/// type for.
const FALLBACK: &str = "private fun Any?.validate(): List<String> = emptyList()";

/// What a string answers where a declared type is asked for its text.
const TEXT: &str = "private fun String.kotlinSchemaText(): String = this";

/// What a type that names a place writes a message under it with.
const UNDER: &str = "private fun kotlinSchemaUnder(place: String, violation: String): String { \
     val end = violation.indexOf('\\'', 1); \
     return if (violation.startsWith(\"'\") && end > 0) { \
     \"'$place.${violation.substring(1, end)}'${violation.substring(end + 1)}\" \
     } else { \"'$place': $violation\" } }";

/// The parcel of the shared table: a bound, a field's check and a type's check.
#[model_schema(check = ["withinParcel"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub fragile_item_codes: Vec<String>,
    #[model_schema_prop(check = ["withoutDuplicates"])]
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub parcel: Parcel,
}

/// A holder of a type declared below it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Shelf {
    pub later: Later,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Later {
    pub label: String,
}

/// A brand over a string, which answers its text.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Slug(pub String);

/// A brand over a brand: it answers through the brand it holds.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Badge(pub Slug);

#[model_schema()]
pub type Title = String;

/// A bound on a brand declared below, on one declared above and on an alias of a string.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Entry {
    #[model_schema_prop(minLength = 3)]
    pub late: LateSlug,
    #[model_schema_prop(minLength = 3)]
    pub slug: Slug,
    #[model_schema_prop(minLength = 2)]
    pub titled: Title,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LateSlug(pub String);

/// A variant of several slots under serde's own tagging.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Freight {
    Held,
    Split(#[model_schema_prop(minLength = 1)] String, Parcel),
}

/// A variant of several slots under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Haul {
    Held,
    Split(#[model_schema_prop(minLength = 1)] String, Parcel),
}

fn without_duplicates(value: &[String]) -> Result<(), Vec<String>> {
    for (index, code) in value.iter().enumerate() {
        if value[..index].contains(code) {
            return Err(vec![format!("duplicate item code {code}")]);
        }
    }
    Ok(())
}

fn within_parcel(value: &Parcel) -> Result<(), Vec<String>> {
    let outside: Vec<String> = value
        .fragile_item_codes
        .iter()
        .enumerate()
        .filter(|(_, code)| !value.item_code_list.contains(code))
        .map(|(index, code)| {
            format!("'fragileItemCodes.{index}': {code} is outside of itemCodeList")
        })
        .collect();
    if outside.is_empty() {
        Ok(())
    } else {
        Err(outside)
    }
}

fn parcel() -> Parcel {
    Parcel {
        fragile_item_codes: vec!["y".to_owned()],
        item_code_list: vec!["x".to_owned(), "x".to_owned()],
        label: String::new(),
    }
}

/// The statement that holds the text the declared type `read` answers to `least` characters,
/// under `place`.
fn asked_statement(read: &str, place: &str, least: u32) -> String {
    let counted =
        format!("{read}.kotlinSchemaText().codePointCount(0, {read}.kotlinSchemaText().length)");
    format!(
        "if ({counted} < {least}) violations.add(\"{place}too short: minimum length is {least}, \
         got ${{{counted}}}\")"
    )
}

/// The statements a variant of several slots runs, each slot under `place` and its position.
fn split_statements(place: &str) -> String {
    format!(
        "{{ override fun validate(): List<String> {{ val violations = mutableListOf<String>(); \
         if (value.slot0.codePointCount(0, value.slot0.length) < 1) violations.add(\"'{place}.0': \
         too short: minimum length is 1, got ${{value.slot0.codePointCount(0, \
         value.slot0.length)}}\"); violations.addAll(value.slot1.validate().map {{ \
         kotlinSchemaUnder(\"{place}.1\", it) }}); return violations }}; {UNDER} }}"
    )
}

#[test]
fn every_fixture_is_written_as_serde_writes_it() {
    let held = parcel();
    assert_eq!(
        serde_json::to_value(Manifest {
            parcel: held.clone()
        })
        .unwrap(),
        serde_json::json!({
            "parcel": { "fragileItemCodes": ["y"], "itemCodeList": ["x", "x"], "label": "" }
        })
    );
    assert_eq!(
        serde_json::to_value(Shelf {
            later: Later {
                label: "a".to_owned()
            }
        })
        .unwrap(),
        serde_json::json!({ "later": { "label": "a" } })
    );
    assert_eq!(
        serde_json::to_value([Freight::Held, Freight::Split("a".to_owned(), held.clone())])
            .unwrap()[0],
        serde_json::json!("Held")
    );
    assert_eq!(
        serde_json::to_value([Haul::Held, Haul::Split("a".to_owned(), held)]).unwrap()[0],
        serde_json::json!({ "kind": "Held" })
    );
    let titled: Title = "t".to_owned();
    assert_eq!(
        serde_json::to_value((
            Badge(Slug("b".to_owned())),
            Entry {
                late: LateSlug("l".to_owned()),
                slug: Slug("s".to_owned()),
                titled,
            }
        ))
        .unwrap(),
        serde_json::json!(["b", { "late": "l", "slug": "s", "titled": "t" }])
    );
}

/// The model of the shared table: each member in the order declared, its bound, what it holds and
/// its checks, then the type's check.
#[cfg(feature = "serde")]
#[test]
fn a_class_runs_each_members_rules_and_then_its_own_check() {
    assert_eq!(
        super::tests::renumbered(&parcel_kotlin::kotlin_definition()),
        format!(
            "{} data class Parcel(val fragileItemCodes: List<String>, val itemCodeList: \
             List<String>, val label: String) {{ fun validate(): List<String> {{ val violations = \
             mutableListOf<String>(); violations.addAll(withoutDuplicates(itemCodeList).map {{ \
             kotlinSchemaUnder(\"itemCodeList\", it) }}); if (label.codePointCount(0, \
             label.length) < 1) violations.add(\"'label': too short: minimum length is 1, got \
             ${{label.codePointCount(0, label.length)}}\"); \
             violations.addAll(withinParcel(this)); return violations }}; {UNDER} }} {}",
            kept("Parcel"),
            serializers("Parcel")
        )
    );
    assert_eq!(
        super::tests::renumbered(&manifest_kotlin::kotlin_definition()),
        format!(
            "{} data class Manifest(@Serializable(with = ParcelUnjudged::class) val parcel: \
             Parcel) {{ fun validate(): List<String> {{ val violations = mutableListOf<String>(); \
             violations.addAll(parcel.validate().map {{ kotlinSchemaUnder(\"parcel\", it) }}); \
             return violations }}; {UNDER} }} {}",
            kept("Manifest"),
            serializers("Manifest")
        )
    );
}

#[test]
fn a_holder_of_a_type_declared_below_writes_what_a_type_with_no_validate_answers() {
    assert_eq!(
        super::tests::renumbered(&shelf_kotlin::kotlin_definition()),
        format!(
            "{} data class Shelf(val later: Later) {{ fun validate(): List<String> {{ val \
             violations = mutableListOf<String>(); violations.addAll(later.validate().map {{ \
             kotlinSchemaUnder(\"later\", it) }}); return violations }}; {FALLBACK}; {UNDER} }} {}",
            kept("Shelf"),
            serializers("Shelf")
        )
    );
    assert_eq!(
        super::tests::renumbered(&later_kotlin::kotlin_definition()),
        format!(
            "{} data class Later(val label: String) {{ {NO_RULE} }} {}",
            kept("Later"),
            serializers("Later")
        )
    );
}

/// A brand over a string answers its text with a member, and a brand over a brand through the
/// brand it holds.
#[test]
fn a_brand_answers_its_text() {
    assert_eq!(
        super::tests::renumbered(&slug_kotlin::kotlin_definition()),
        format!(
            "{} value class Slug(val value: String) {{ fun kotlinSchemaText(): String = value; \
             {NO_RULE} }} {}",
            kept_inline("Slug"),
            serializers("Slug")
        )
    );
    assert_eq!(
        super::tests::renumbered(&badge_kotlin::kotlin_definition()),
        format!(
            "{} value class Badge(val value: Slug) {{ fun kotlinSchemaText(): String = \
             value.kotlinSchemaText(); {NO_RULE}; {TEXT} }} {}",
            kept_inline("Badge"),
            serializers("Badge")
        )
    );
}

/// A holder asks a declared type for its text and answers for a string itself, so a brand
/// declared above, one declared below and an alias of a string are each measured.
#[test]
fn a_bound_on_a_declared_type_measures_the_text_the_type_answers() {
    assert_eq!(
        super::tests::renumbered(&entry_kotlin::kotlin_definition()),
        format!(
            "{} data class Entry(val late: LateSlug, val slug: Slug, val titled: TitleType) {{ \
             fun validate(): List<String> {{ val violations = mutableListOf<String>(); {}; {}; \
             {}; return violations }}; {TEXT} }} {}",
            kept("Entry"),
            asked_statement("late", "'late': ", 3),
            asked_statement("slug", "'slug': ", 3),
            asked_statement("titled", "'titled': ", 2),
            serializers("Entry")
        )
    );
}

/// A slot of a variant of several is named by the step serde writes the variant under and by its
/// position: the variant's name where no tag is written.
#[test]
fn a_slot_of_a_variant_of_several_is_named_by_the_variant_and_its_position() {
    let freight = super::tests::renumbered(&freight_kotlin::kotlin_definition());
    assert!(
        freight.contains(&format!(") : Freight {}", split_statements("Split"))),
        "got: {freight}"
    );
}

/// Under a tag and a content key, the same slot is named by the content key and its position.
#[cfg(feature = "serde")]
#[test]
fn a_slot_of_a_variant_of_several_is_named_by_the_content_key_and_its_position() {
    let haul = super::tests::renumbered(&haul_kotlin::kotlin_definition());
    assert!(
        haul.contains(&format!(") : Haul {}", split_statements("held"))),
        "got: {haul}"
    );
}

#[test]
fn every_shape_carries_validate() {
    for (definition, written) in [
        (
            super::tests::renumbered(&ping_kotlin::kotlin_definition()),
            format!(
                "{} object Ping {{ {NO_RULE} }} {}",
                kept("Ping"),
                serializers("Ping")
            ),
        ),
        (
            super::tests::renumbered(&correlation_id_kotlin::kotlin_definition()),
            format!(
                "{} value class CorrelationId(val value: String) {{ fun kotlinSchemaText(): \
                 String = value; {NO_RULE} }} {}",
                kept_inline("CorrelationId"),
                serializers("CorrelationId")
            ),
        ),
        (
            super::tests::renumbered(&wrapper_kotlin::kotlin_definition()),
            format!(
                "{} data class Wrapper<T>(val value: T) {{ {NO_RULE} }} \
                 @OptIn(ExperimentalSerializationApi::class) class WrapperUnjudged<T>(tSerializer: \
                 KSerializer<T>) : KSerializer<Wrapper<T>> by \
                 Wrapper.generatedSerializer(tSerializer) {}",
                kept("Wrapper"),
                judged_over("Wrapper", "T", "", "tSerializer")
            ),
        ),
        (
            super::tests::renumbered(&widget_kotlin::kotlin_definition()),
            format!(
                "{} data class RenamedWidget(val label: String) {{ {NO_RULE} }} {} typealias \
                 Widget = RenamedWidget;",
                kept("RenamedWidget"),
                serializers("RenamedWidget")
            ),
        ),
    ] {
        assert_eq!(definition, written);
    }
    let pair = super::tests::renumbered(&generic_pair_kotlin::kotlin_definition());
    assert!(
        pair.contains(&format!(
            "data class GenericPair<T>(val slot0: T, val slot1: UInt) {{ {NO_RULE} }} class \
             GenericPairUnjudged<T: Any>("
        )),
        "got: {pair}"
    );
    assert!(
        pair.ends_with(&judged_over("GenericPair", "T", ": Any", "tSerializer")),
        "got: {pair}"
    );
    let holder = super::tests::renumbered(&holds_tuple_kotlin::kotlin_definition());
    assert!(
        holder.contains(&format!(
            "{} data class HoldsTuple(val coords: KotlinTuple",
            kept("HoldsTuple")
        )) && holder.ends_with(&format!(") {{ {NO_RULE} }} {}", serializers("HoldsTuple"))),
        "got: {holder}"
    );
}

/// A plain enum carries the method after its members.
#[cfg(feature = "serde")]
#[test]
fn a_plain_enum_carries_validate() {
    assert_eq!(
        super::tests::renumbered(&status_kotlin::kotlin_definition()),
        format!(
            "@Serializable enum class Status {{ @SerialName(\"active\") Active, \
             @SerialName(\"inactive\") Inactive, @SerialName(\"pending\") Pending; {NO_RULE} }}"
        )
    );
}

/// A sealed interface declares the method, and each variant overrides it, under every tagging.
#[cfg(feature = "serde")]
#[test]
fn a_sealed_interface_declares_validate_and_each_variant_overrides_it() {
    let declared = "{ fun validate(): List<String> }";
    let overridden = format!("{{ override {NO_RULE} }}");
    for (definition, interface, variants) in [
        (
            super::tests::renumbered(&window_error_kotlin::kotlin_definition()),
            "WindowError",
            vec![
                "data object WindowErrorNotFound : WindowError",
                "data class WindowErrorRateLimited(@SerialName(\"retry_after_ms\") val \
                 retryAfterMs: Long) : WindowError",
            ],
        ),
        (
            super::tests::renumbered(&dynamic_value_kotlin::kotlin_definition()),
            "DynamicValue",
            vec![
                "data class DynamicValueFlag(val value: Boolean) : DynamicValue",
                "data object DynamicValueNothing : DynamicValue",
                "data class DynamicValueNumber(val value: Long) : DynamicValue",
            ],
        ),
        (
            super::tests::renumbered(&external_tagged_kotlin::kotlin_definition()),
            "ExternalTagged",
            vec![
                "data class ExternalTaggedPing(val nonce: Int) : ExternalTagged",
                "data class ExternalTaggedPong(val nonce: Int) : ExternalTagged",
            ],
        ),
        (
            super::tests::renumbered(&date_value_kotlin::kotlin_definition()),
            "DateValue",
            vec![
                "value class DateValueEpoch(val value: Long) : DateValue",
                "value class DateValueIso(val value: String) : DateValue",
            ],
        ),
    ] {
        let types = super::tests::without_number_readers(&definition);
        assert!(
            types.contains(&format!("sealed interface {interface} {declared} ")),
            "got: {definition}"
        );
        for variant in variants {
            assert!(
                types.contains(&format!("{variant} {overridden}")),
                "no `{variant}` with its override in: {definition}"
            );
        }
    }
}

/// A flattened member writes no key, so what it reports is written under no name.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_member_is_reported_under_no_name() {
    let audit = super::tests::renumbered(&audit_kotlin::kotlin_definition());
    assert!(
        audit.contains(
            "data class Audit(val id: String, val stamp: Stamp) { fun validate(): List<String> { \
             val violations = mutableListOf<String>(); violations.addAll(stamp.validate()); \
             return violations } } object AuditUnjudged"
        ),
        "got: {audit}"
    );
}

/// The serializer the emitter writes for a struct with a flattened member and for a tuple struct
/// is its unjudged one, a model it holds read unjudged, and the type's own serializer follows it.
/// The struct refuses a key neither its own member nor the struct it flattens reads.
#[cfg(feature = "serde")]
#[test]
fn the_serializer_the_emitter_writes_for_each_struct_shape_is_its_unjudged_one() {
    for (definition, unjudged, own) in [
        (
            super::tests::renumbered(&audit_kotlin::kotlin_definition()),
            "object AuditUnjudged : KSerializer<Audit> { override val descriptor: \
             SerialDescriptor = buildClassSerialDescriptor(\"Audit\"); override fun \
             serialize(encoder: Encoder, value: Audit) { val output = encoder as JsonEncoder; \
             output.encodeJsonElement(buildJsonObject { \
             put(\"id\", output.json.encodeToJsonElement(serializer<String>(), value.id)); \
             output.json.encodeToJsonElement(StampUnjudged, value.stamp).jsonObject.forEach { (k, \
             v) -> put(k, v) } }) }; override fun deserialize(decoder: Decoder): Audit { val \
             input = decoder as JsonDecoder; val obj = input.decodeJsonElement().let { it as? \
             JsonObject ?: throw SerializationException(\"expected an object, found \" + it) }; \
             val id = input.json.decodeFromJsonElement(serializer<String>(), (obj[\"id\"] ?: \
             throw SerializationException(\"missing field `id`\"))); \
             val stampKeys = setOf(\"created_at\", \"updated_at\"); val stamp = \
             input.json.decodeFromJsonElement(StampUnjudged, JsonObject(obj.filterKeys { it in \
             setOf(\"created_at\", \"updated_at\") })); \
             if (!input.json.configuration.ignoreUnknownKeys) (obj - setOf(\"id\") - \
             setOf(\"created_at\", \"updated_at\")).keys.firstOrNull()?.let { throw \
             SerializationException(\"Encountered an unknown key '\" + it + \"'\") }; return \
             Audit(id = id, stamp = stamp) } }",
            judged("Audit"),
        ),
        (
            super::tests::renumbered(&generic_pair_kotlin::kotlin_definition()),
            "class GenericPairUnjudged<T: Any>(private val tSerializer: KSerializer<T>) : \
             KSerializer<GenericPair<T>> { override val descriptor: SerialDescriptor = \
             buildClassSerialDescriptor(\"GenericPair\"); override fun serialize(encoder: \
             Encoder, value: GenericPair<T>) { val output = encoder as JsonEncoder; \
             output.encodeJsonElement(buildJsonArray { \
             add(output.json.encodeToJsonElement(tSerializer, value.slot0)); \
             add(output.json.encodeToJsonElement(KotlinNumber1, value.slot1)) }) }; override \
             fun deserialize(decoder: Decoder): GenericPair<T> { val input = decoder as \
             JsonDecoder; val array = input.decodeJsonElement().let { it as? JsonArray ?: throw \
             SerializationException(\"expected an array, found \" + it) }; if (array.size != 2) \
             throw SerializationException(\"invalid length \" + array.size + \", expected 2\"); \
             return GenericPair(input.json.decodeFromJsonElement(tSerializer, array[0]), \
             input.json.decodeFromJsonElement(KotlinNumber1, array[1])) } }",
            judged_over("GenericPair", "T", ": Any", "tSerializer"),
        ),
    ] {
        assert!(
            definition.ends_with(&format!("{unjudged} {own}")),
            "the serializers moved in: {definition}"
        );
    }
}

/// The serializer the emitter writes for an enum under each tagging is its unjudged one, and the
/// type's own serializer follows it.
#[cfg(feature = "serde")]
#[test]
fn the_serializer_the_emitter_writes_for_each_enum_shape_is_its_unjudged_one() {
    for (definition, unjudged, own) in [
        (
            super::tests::renumbered(&simple_choice_kotlin::kotlin_definition()),
            "object SimpleChoiceUnjudged : KSerializer<SimpleChoice> { override val \
             descriptor: SerialDescriptor = buildClassSerialDescriptor(\"SimpleChoice\"); \
             override fun serialize(encoder: Encoder, value: SimpleChoice) { val output = encoder \
             as JsonEncoder; output.encodeJsonElement(when (value) { is SimpleChoiceNone -> \
             JsonPrimitive(\"None\"); is SimpleChoiceValue -> buildJsonObject { put(\"Value\", \
             output.json.encodeToJsonElement(serializer<String>(), value.value)) } }) }; override \
             fun deserialize(decoder: Decoder): SimpleChoice { val input = decoder as \
             JsonDecoder; val element = input.decodeJsonElement(); val (tag, data) = if (element \
             is JsonPrimitive) element.content to JsonNull else (element as? JsonObject ?: throw \
             SerializationException(\"expected an object, found \" + \
             element)).entries.singleOrNull()?.let { it.key to it.value } ?: throw \
             SerializationException(\"expected an object of one key, found \" + element); return \
             when (tag) { \
             \"None\" -> if (data is JsonNull) SimpleChoiceNone else throw \
             SerializationException(\"SimpleChoice: \" + tag + \" takes no value\"); \"Value\" -> \
             SimpleChoiceValue(input.json.decodeFromJsonElement(serializer<String>(), data)) else \
             -> throw SerializationException(\"unknown tag \" + tag) } } }",
            judged("SimpleChoice"),
        ),
        (
            super::tests::renumbered(&dynamic_value_kotlin::kotlin_definition()),
            "object DynamicValueUnjudged : KSerializer<DynamicValue> { override val \
             descriptor: SerialDescriptor = buildClassSerialDescriptor(\"DynamicValue\"); \
             override fun serialize(encoder: Encoder, value: DynamicValue) { val output = encoder \
             as JsonEncoder; output.encodeJsonElement(when (value) { is DynamicValueFlag -> \
             buildJsonObject { put(\"type\", \"Flag\"); put(\"value\", \
             output.json.encodeToJsonElement(KotlinNumber1, value.value)) }; is \
             DynamicValueNothing -> buildJsonObject { put(\"type\", \"Nothing\") }; is \
             DynamicValueNumber -> buildJsonObject { put(\"type\", \"Number\"); put(\"value\", \
             output.json.encodeToJsonElement(KotlinNumber2, value.value)) } }) }; override \
             fun deserialize(decoder: Decoder): DynamicValue { val input = decoder as \
             JsonDecoder; val obj = input.decodeJsonElement().let { it as? JsonObject ?: throw \
             SerializationException(\"expected an object, found \" + it) }; val tag = \
             (obj[\"type\"] ?: throw SerializationException(\"missing field `type`\")).let { it \
             as? JsonPrimitive ?: throw SerializationException(\"expected a string, found \" + \
             it) }.content; val data = obj[\"value\"]; if \
             (!input.json.configuration.ignoreUnknownKeys) (obj - \"type\" - \
             \"value\").keys.firstOrNull()?.let { throw SerializationException(\"Encountered an \
             unknown key '\" + it + \"'\") }; return when (tag) { \"Flag\" -> \
             DynamicValueFlag(input.json.decodeFromJsonElement(KotlinNumber1, (data ?: \
             throw SerializationException(\"missing field `value`\")))); \"Nothing\" -> if \
             (data == null) DynamicValueNothing else throw \
             SerializationException(\"DynamicValue: \" + tag + \" takes no value\"); \"Number\" \
             -> \
             DynamicValueNumber(input.json.decodeFromJsonElement(KotlinNumber2, (data ?: \
             throw SerializationException(\"missing field `value`\")))) else -> throw \
             SerializationException(\"unknown tag \" + tag) } } }",
            judged("DynamicValue"),
        ),
        (
            super::tests::renumbered(&date_value_kotlin::kotlin_definition()),
            "object DateValueUnjudged : KSerializer<DateValue> { override val descriptor: \
             SerialDescriptor = buildClassSerialDescriptor(\"DateValue\"); override fun \
             serialize(encoder: Encoder, value: DateValue) { val output = encoder as JsonEncoder; \
             output.encodeJsonElement(when (value) { is DateValueEpoch -> \
             output.json.encodeToJsonElement(serializer<DateValueEpoch>(), value); is \
             DateValueIso -> output.json.encodeToJsonElement(serializer<DateValueIso>(), value) \
             }) }; override fun deserialize(decoder: Decoder): DateValue { val input = decoder as \
             JsonDecoder; val element = input.decodeJsonElement(); return runCatching { \
             error(\"unreachable\") as Nothing }.recoverCatching { \
             input.json.decodeFromJsonElement(serializer<DateValueEpoch>(), element) as DateValue \
             }.recoverCatching { input.json.decodeFromJsonElement(serializer<DateValueIso>(), \
             element) as DateValue }.getOrElse { throw SerializationException(\"No variant of \
             DateValue matched\") } } }",
            judged("DateValue"),
        ),
    ] {
        assert!(
            definition.ends_with(&format!("{unjudged} {own}")),
            "the serializers moved in: {definition}"
        );
    }
}
