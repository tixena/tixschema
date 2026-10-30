//! A name serde reads beside the one it writes, in the Kotlin read: a variant is read under each
//! of its aliases, under each tagging and in a plain enum, a member under each of its aliases, in
//! a struct the plugin reads, in one the emitter reads by hand and in a variant, and each is
//! written under the name serde writes.

use super::aliased_models::{
    careening_kotlin, fairleading_kotlin, fothering_kotlin, gybing_kotlin, hawsing_kotlin,
    lulling_kotlin, scending_kotlin, surging_kotlin, swell_kotlin, veering_kotlin,
};

/// What reads a payload's key `name` as `label`, and refuses the two together.
const NAME_AS_LABEL: &str = "let { whole: JsonElement -> if (whole is JsonObject) \
     JsonObject(buildMap<String, JsonElement> { whole.forEach { (key, held) -> val named = when \
     (key) { \"name\" -> \"label\"; else -> key }; if (put(named, held) != null) throw \
     SerializationException(\"duplicate field `$named`\") } }) else whole }";

/// What reads a payload's keys `name` and `title` as `label`, and `remark` as `note`.
const HAWSING_NAMES: &str = "element.let { whole: JsonElement -> if (whole is JsonObject) \
     JsonObject(buildMap<String, JsonElement> { whole.forEach { (key, held) -> val named = when \
     (key) { \"name\" -> \"label\"; \"title\" -> \"label\"; \"remark\" -> \"note\"; else -> key }; \
     if (put(named, held) != null) throw SerializationException(\"duplicate field `$named`\") } \
     }) else whole }";

/// An enum reads a variant under each of its aliases, under each tagging, and writes it under the
/// name serde writes. A member of a variant is read under its alias.
#[test]
fn a_tagged_enum_reads_a_variant_under_each_alias() {
    let surging = surging_kotlin::kotlin_definition();
    for held in [
        format!(
            "\"Berth\", \"Pier\" -> input.json.decodeFromJsonElement(serializer<SurgingBerth>(), \
             data.{NAME_AS_LABEL});"
        ),
        "\"Calm\", \"Still\", \"Slack\" -> if (data is JsonNull) SurgingCalm else throw \
         SerializationException(\"Surging: \" + tag + \" takes no value\");"
            .to_owned(),
        "\"Held\", \"Tied\" -> SurgingHeld(kotlinSchemaDecoded(input.json, serializer<String>(), \
         data, \"Held\"))"
            .to_owned(),
        "is SurgingBerth -> buildJsonObject { put(\"Berth\", \
         output.json.encodeToJsonElement(serializer<SurgingBerth>(), value)) }; is SurgingCalm -> \
         JsonPrimitive(\"Calm\"); is SurgingHeld -> buildJsonObject { put(\"Held\","
            .to_owned(),
    ] {
        assert!(surging.contains(&held), "no `{held}` in: {surging}");
    }
    let careening = careening_kotlin::kotlin_definition();
    for held in [
        format!(
            "\"Berth\", \"Pier\" -> \
             input.json.decodeFromJsonElement(serializer<CareeningBerth>(), JsonObject(obj - \
             \"kind\").{NAME_AS_LABEL});"
        ),
        "\"Calm\", \"Still\" -> { if (!input.json.configuration.ignoreUnknownKeys) (obj - \
         \"kind\").keys.firstOrNull()?.let { throw SerializationException(\"Encountered an \
         unknown key '\" + it + \"'\") }; CareeningCalm } else -> throw SerializationException("
            .to_owned(),
        "is CareeningCalm -> buildJsonObject { put(\"kind\", \"Calm\") }".to_owned(),
    ] {
        assert!(careening.contains(&held), "no `{held}` in: {careening}");
    }
    let scending = scending_kotlin::kotlin_definition();
    for held in [
        format!(
            "\"Berth\", \"Pier\" -> \
             input.json.decodeFromJsonElement(serializer<ScendingBerth>(), (data ?: throw \
             SerializationException(\"missing field `data`\")).{NAME_AS_LABEL});"
        ),
        "\"Calm\", \"Still\" -> if (data == null) ScendingCalm else throw \
         SerializationException(\"Scending: \" + tag + \" takes no value\");"
            .to_owned(),
        "\"Held\", \"Tied\" -> ScendingHeld(".to_owned(),
        "is ScendingHeld -> buildJsonObject { put(\"kind\", \"Held\");".to_owned(),
    ] {
        assert!(scending.contains(&held), "no `{held}` in: {scending}");
    }
}

/// A plain enum that reads a variant under an alias is read and written by `{Type}Unjudged`, a
/// serializer written by hand, its one serializer. One that names a function is read by the
/// plugin's serializer behind the function, and the alias read as the variant's name after it.
#[test]
fn a_plain_enum_reads_a_variant_under_each_alias() {
    assert_eq!(
        swell_kotlin::kotlin_definition(),
        "@OptIn(ExperimentalSerializationApi::class) @Serializable(with = \
         SwellUnjudged::class) @KeepGeneratedSerializer enum class Swell { Ebb, Flood; fun \
         validate(): List<String> = emptyList() } object SwellUnjudged : KSerializer<Swell> { \
         override val descriptor: SerialDescriptor = Swell.generatedSerializer().descriptor; \
         override fun serialize(encoder: Encoder, value: Swell) = \
         encoder.encodeString(when (value) { Swell.Ebb -> \"Ebb\"; Swell.Flood -> \"Flood\" }); \
         override fun deserialize(decoder: Decoder): Swell = when (val name = \
         decoder.decodeString()) { \"Ebb\", \"Low\", \"Neap\" -> Swell.Ebb; \"Flood\" -> \
         Swell.Flood; else -> throw SerializationException(\"unknown variant \" + name) } }"
    );
    assert_eq!(
        lulling_kotlin::kotlin_definition(),
        "@OptIn(ExperimentalSerializationApi::class) @Serializable(with = \
         LullingUnjudged::class) @KeepGeneratedSerializer enum class Lulling { Ebb, Flood; fun \
         validate(): List<String> = emptyList() } @OptIn(ExperimentalSerializationApi::class) \
         object LullingUnjudged : JsonTransformingSerializer<Lulling>(Lulling.\
         generatedSerializer()) { override fun transformDeserialize(element: JsonElement): \
         JsonElement = unpadded(element).let { whole: JsonElement -> if (whole is JsonPrimitive \
         && whole.isString) when (whole.content) { \"Low\" -> JsonPrimitive(\"Ebb\"); else -> \
         whole } else whole } }"
    );
}

/// An untagged enum tries a variant with a member under an alias from the payload with the alias
/// read as the member's name.
#[test]
fn an_untagged_enum_reads_a_member_under_its_alias() {
    let veering = veering_kotlin::kotlin_definition();
    let held = format!(
        "recoverCatching {{ input.json.decodeFromJsonElement(serializer<VeeringBerth>(), \
         element.{NAME_AS_LABEL}) as Veering }}.recoverCatching {{ \
         input.json.decodeFromJsonElement(serializer<VeeringCount>(), element) as Veering }}"
    );
    assert!(veering.contains(&held), "no `{held}` in: {veering}");
}

/// A struct the plugin reads takes the plugin's serializer behind what reads each alias as its
/// member's name, its parameters among the serializer's.
#[test]
fn a_struct_the_plugin_reads_reads_a_member_under_each_alias() {
    let hawsing = hawsing_kotlin::kotlin_definition();
    let hawsing_read = format!(
        "@OptIn(ExperimentalSerializationApi::class) object HawsingUnjudged : \
         JsonTransformingSerializer<Hawsing>(Hawsing.generatedSerializer()) {{ override fun \
         transformDeserialize(element: JsonElement): JsonElement = {HAWSING_NAMES} }}"
    );
    assert!(
        hawsing.contains(&hawsing_read),
        "no `{hawsing_read}` in: {hawsing}"
    );
    let gybing = gybing_kotlin::kotlin_definition();
    let gybing_read = format!(
        "@OptIn(ExperimentalSerializationApi::class) class \
         GybingUnjudged<LabelType>(labelTypeSerializer: KSerializer<LabelType>) : \
         JsonTransformingSerializer<Gybing<LabelType>>(Gybing.generatedSerializer(\
         labelTypeSerializer)) {{ override fun transformDeserialize(element: JsonElement): \
         JsonElement = {HAWSING_NAMES} }}"
    );
    assert!(
        gybing.contains(&gybing_read),
        "no `{gybing_read}` in: {gybing}"
    );
}

/// A struct the emitter reads by hand reads each alias of a member of its own as the member's
/// name in the object every member is read from, and a member flattened after it is handed that
/// object without the member's key.
#[test]
fn a_struct_read_by_hand_reads_a_member_under_its_alias() {
    let fairleading = fairleading_kotlin::kotlin_definition();
    let held = format!(
        "val obj = input.decodeJsonElement().{NAME_AS_LABEL}.let {{ it as? JsonObject ?: throw \
         SerializationException(\"expected an object, found \" + it) }}; val label = \
         kotlinSchemaDecoded(input.json, serializer<String>(), (obj[\"label\"] ?: throw \
         SerializationException(\"missing field `label`\")), \"label\"); \
         val tallyKeys = setOf(\"keys\"); val tally = \
         lenient.decodeFromJsonElement(PennonUnjudged, withItsKeys(JsonObject(obj - \
         setOf(\"label\"))));"
    );
    assert!(fairleading.contains(&held), "no `{held}` in: {fairleading}");
}

/// A flattened struct with a member under an alias takes each name serde reads the member under
/// off its holder's object.
#[test]
fn a_flattened_struct_takes_each_alias_of_its_members() {
    let fothering = fothering_kotlin::kotlin_definition();
    let held = "val hawsingKeys = setOf(\"label\", \"name\", \"title\", \"note\", \"remark\"); \
                val hawsing = input.json.decodeFromJsonElement(HawsingUnjudged, \
                JsonObject(obj.filterKeys { it in setOf(\"label\", \"name\", \"title\", \"note\", \
                \"remark\") }));";
    assert!(fothering.contains(held), "no `{held}` in: {fothering}");
}
