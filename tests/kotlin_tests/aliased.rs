//! A name serde reads beside the one it writes, in the Kotlin read: a variant is read under each
//! of its aliases, under each tagging and in a plain enum, a member under each of its aliases, in
//! a struct the plugin reads, in one the emitter reads by hand and in a variant, and each is
//! written under the name serde writes.

use super::aliased_models::{
    CAREENING, Careening, FAIRLEADING, Fairleading, Fothering, Gybing, HAWSING, Hawsing, LULLING,
    Lulling, SCENDING, SURGING, SWELL, Scending, Surging, Swell, TIDING, Tiding, VEERING, Veering,
    careening_kotlin, fairleading_kotlin, fothering_kotlin, gybing_kotlin, hawsing_kotlin,
    lulling_kotlin, scending_kotlin, surging_kotlin, swell_kotlin, veering_kotlin, written_back,
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

/// What serde reads of each payload: a name it reads beside the one it writes is read as that
/// variant or member, written back under the name serde writes, and a member under two of its
/// names is refused.
#[test]
fn serde_reads_a_variant_and_a_member_under_each_alias() {
    let surging = SURGING.map(written_back::<Surging>);
    assert_eq!(
        surging,
        [
            r#"{"Berth":{"label":"n"}}"#,
            r#"{"Berth":{"label":"n"}}"#,
            r#"{"Berth":{"label":"n"}}"#,
            "refused",
            r#""Calm""#,
            r#""Calm""#,
            r#""Calm""#,
            r#""Calm""#,
            r#"{"Held":"x"}"#,
            r#"{"Held":"x"}"#,
        ]
    );
    let careening = CAREENING.map(written_back::<Careening>);
    assert_eq!(
        careening,
        [
            r#"{"kind":"Berth","label":"n"}"#,
            r#"{"kind":"Berth","label":"n"}"#,
            r#"{"kind":"Berth","label":"n"}"#,
            "refused",
            r#"{"kind":"Calm"}"#,
            r#"{"kind":"Calm"}"#,
        ]
    );
    let scending = SCENDING.map(written_back::<Scending>);
    assert_eq!(
        scending,
        [
            r#"{"kind":"Berth","data":{"label":"n"}}"#,
            r#"{"kind":"Berth","data":{"label":"n"}}"#,
            r#"{"kind":"Berth","data":{"label":"n"}}"#,
            r#"{"kind":"Calm"}"#,
            r#"{"kind":"Calm"}"#,
            r#"{"kind":"Held","data":"x"}"#,
            r#"{"kind":"Held","data":"x"}"#,
        ]
    );
    let swell = SWELL.map(written_back::<Swell>);
    assert_eq!(
        swell,
        [r#""Flood""#, r#""Ebb""#, r#""Ebb""#, r#""Ebb""#, "refused"]
    );
    assert_eq!(
        LULLING.map(written_back::<Lulling>),
        [r#""Ebb""#, r#""Ebb""#, r#""Flood""#]
    );
    assert_eq!(
        written_back::<Tiding>(TIDING),
        r#"{"by_swell":{"Ebb":1,"Flood":2},"swells":["Ebb","Flood"]}"#
    );
    let veering = VEERING.map(written_back::<Veering>);
    assert_eq!(
        veering,
        [r#"{"label":"n"}"#, r#"{"label":"n"}"#, "refused", "3"]
    );
    let hawsing = HAWSING.map(written_back::<Hawsing>);
    let held = [
        r#"{"label":"n"}"#,
        r#"{"label":"n"}"#,
        r#"{"label":"n","note":"r"}"#,
        "refused",
        "refused",
    ];
    assert_eq!(hawsing, held);
    assert_eq!(HAWSING.map(written_back::<Gybing<String>>), held);
    assert_eq!(
        serde_json::from_str::<Hawsing>(HAWSING[3])
            .unwrap_err()
            .to_string(),
        "duplicate field `label` at line 1 column 19"
    );
    let tallied = [
        r#"{"label":"n","keys":["north"]}"#,
        r#"{"label":"n","keys":["north"]}"#,
        "refused",
    ];
    assert_eq!(FAIRLEADING.map(written_back::<Fairleading>), tallied);
    assert_eq!(FAIRLEADING.map(written_back::<Fothering>), tallied);
}

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
        "\"Calm\", \"Still\", \"Slack\" -> SurgingCalm;".to_owned(),
        "\"Held\", \"Tied\" -> SurgingHeld(input.json.decodeFromJsonElement(serializer<String>(), \
         data))"
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
        "\"Calm\", \"Still\" -> CareeningCalm else -> throw SerializationException(".to_owned(),
        "is CareeningCalm -> buildJsonObject { put(\"kind\", \"Calm\") }".to_owned(),
    ] {
        assert!(careening.contains(&held), "no `{held}` in: {careening}");
    }
    let scending = scending_kotlin::kotlin_definition();
    for held in [
        format!(
            "\"Berth\", \"Pier\" -> \
             input.json.decodeFromJsonElement(serializer<ScendingBerth>(), \
             data!!.{NAME_AS_LABEL});"
        ),
        "\"Calm\", \"Still\" -> ScendingCalm;".to_owned(),
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
        "@Serializable(with = SwellUnjudged::class) enum class Swell { Ebb, Flood; fun \
         validate(): List<String> = emptyList() } object SwellUnjudged : KSerializer<Swell> { \
         override val descriptor: SerialDescriptor = PrimitiveSerialDescriptor(\"Swell\", \
         PrimitiveKind.STRING); override fun serialize(encoder: Encoder, value: Swell) = \
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
        "val obj = input.decodeJsonElement().{NAME_AS_LABEL}.jsonObject; val label = \
         input.json.decodeFromJsonElement(serializer<String>(), obj.getValue(\"label\")); \
         val tallyKeys = setOf(\"keys\"); val tally = \
         lenient.decodeFromJsonElement(BuntingUnjudged, withItsKeys(JsonObject(obj - \
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
                val hawsing = lenient.decodeFromJsonElement(HawsingUnjudged, obj);";
    assert!(fothering.contains(held), "no `{held}` in: {fothering}");
}
