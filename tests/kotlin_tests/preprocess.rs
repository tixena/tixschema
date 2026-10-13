//! `preprocess` as the macro's own path hands it to the Kotlin emitter: the functions a type names
//! are called by its unjudged serializer, and those a member names by a serializer of the member's
//! own. The text is the same in a build with a schema surface and in one with none.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use super::read::{judged, judged_over, kept};

/// The parcel of the documents' sample, under a name of this file's own.
#[model_schema(preprocess = ["withoutFragile"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Packet {
    pub fragile_item_codes: Vec<String>,
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1, preprocess = ["trimmed"])]
    pub label: String,
}

/// The type that holds the packet under a key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lading {
    pub packet: Packet,
}

/// An enum of unit variants.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Tier {
    Express,
    Standard,
}

/// A type with a parameter that names two functions, and a member that reaches the parameter.
#[model_schema(default_types(CodeType = String), preprocess = ["unwrapped", "withoutFragile"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hamper<CodeType> {
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub codes: Vec<CodeType>,
}

/// What a serializer that corrects its payload overrides, handing `correction` on.
fn transform(correction: &str) -> String {
    format!(
        "{{ override fun transformDeserialize(element: JsonElement): JsonElement = {correction} }}"
    )
}

fn trimmed(payload: Value) -> Value {
    if let Value::String(text) = payload {
        Value::String(text.trim().to_owned())
    } else {
        payload
    }
}

fn unwrapped(payload: Value) -> Value {
    if let Value::Object(mut entries) = payload {
        entries
            .remove("payload")
            .unwrap_or_else(|| Value::Object(entries))
    } else {
        payload
    }
}

fn without_fragile(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object
            .entry("fragileItemCodes")
            .or_insert_with(|| Value::Array(Vec::new()));
    }
    payload
}

#[test]
fn every_fixture_is_read_through_its_functions_and_written_as_serde_writes_it() {
    let packet: Packet =
        serde_json::from_value(json!({ "label": " A ", "itemCodeList": ["x"] })).unwrap();
    let lading = Lading { packet };
    assert_eq!(
        serde_json::to_value(&lading).unwrap(),
        json!({ "packet": { "fragileItemCodes": [], "itemCodeList": ["x"], "label": "A" } })
    );
    let tier: Tier = serde_json::from_value(json!({ "payload": "Express" })).unwrap();
    assert_eq!(serde_json::to_value(tier).unwrap(), json!("Express"));
    let hamper: Hamper<u32> =
        serde_json::from_value(json!({ "payload": { "codes": { "payload": [7_u32] } } })).unwrap();
    assert_eq!(
        serde_json::to_value(hamper).unwrap(),
        json!({ "codes": [7_u32] })
    );
    assert_eq!(
        serde_json::to_value(Tier::Standard).unwrap(),
        json!("Standard")
    );
}

/// The type's function is called by `PacketUnjudged`, over the plugin's serializer, and the
/// label's by `PacketLabelPreprocess`, which the property names and which is written after the
/// type's two serializers.
#[test]
fn a_type_and_a_member_that_name_a_function_are_read_through_it() {
    assert_eq!(
        packet_kotlin::kotlin_definition(),
        format!(
            "{} data class Packet(val fragileItemCodes: List<String>, val itemCodeList: \
             List<String>, @Serializable(with = PacketLabelPreprocess::class) val label: String) \
             {{ fun validate(): List<String> {{ val violations = mutableListOf<String>(); if \
             (label.codePointCount(0, label.length) < 1) violations.add(\"'label': too short: \
             minimum length is 1, got ${{label.codePointCount(0, label.length)}}\"); return \
             violations }} }} @OptIn(ExperimentalSerializationApi::class) object PacketUnjudged : \
             JsonTransformingSerializer<Packet>(Packet.generatedSerializer()) {} {} object \
             PacketLabelPreprocess : JsonTransformingSerializer<String>(serializer<String>()) {}",
            kept("Packet"),
            transform("withoutFragile(element)"),
            judged("Packet"),
            transform("trimmed(element)")
        )
    );
}

/// A holder names the unjudged serializer of the type it holds as it does without the key, and
/// that serializer is the one that calls the type's functions.
#[test]
fn a_holder_reads_the_type_through_the_serializer_that_corrects_it() {
    let lading = lading_kotlin::kotlin_definition();
    assert!(
        lading.contains(
            "data class Lading(@Serializable(with = PacketUnjudged::class) val packet: Packet) {"
        ),
        "got: {lading}"
    );
    assert!(!lading.contains("Preprocess"), "got: {lading}");
}

/// A plain enum that names a function is read by `TierUnjudged`, the one serializer it has.
#[test]
fn a_plain_enum_that_names_a_function_has_one_serializer_that_calls_it() {
    assert_eq!(
        tier_kotlin::kotlin_definition(),
        format!(
            "@OptIn(ExperimentalSerializationApi::class) @Serializable(with = \
             TierUnjudged::class) @KeepGeneratedSerializer enum class Tier {{ Express, Standard; \
             fun validate(): List<String> = emptyList() }} \
             @OptIn(ExperimentalSerializationApi::class) object TierUnjudged : \
             JsonTransformingSerializer<Tier>(Tier.generatedSerializer()) {}",
            transform("unwrapped(element)")
        )
    );
}

/// A type with a parameter calls its two functions in the order written, then the function of
/// the member that reaches the parameter, at the member's key.
#[test]
fn a_type_with_a_parameter_corrects_the_whole_payload_and_then_the_member() {
    let corrected = "withoutFragile(unwrapped(element)).let { whole: JsonElement -> if (whole \
                     is JsonObject) JsonObject(whole.mapValues { (key, held) -> when (key) { \
                     \"codes\" -> unwrapped(held); else -> held } }) else whole }";
    assert_eq!(
        hamper_kotlin::kotlin_definition(),
        format!(
            "{} data class Hamper<CodeType>(val codes: List<CodeType>) {{ fun validate(): \
             List<String> = emptyList() }} @OptIn(ExperimentalSerializationApi::class) class \
             HamperUnjudged<CodeType>(codeTypeSerializer: KSerializer<CodeType>) : \
             JsonTransformingSerializer<Hamper<CodeType>>(Hamper.\
             generatedSerializer(codeTypeSerializer)) {} {}",
            kept("Hamper"),
            transform(corrected),
            judged_over("Hamper", "CodeType", "", "codeTypeSerializer")
        )
    );
}
