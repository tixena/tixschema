//! A member serde reads and never writes: the Kotlin serializer reads it and writes no key for it,
//! in a struct of its own, beside a flattened member, and in an enum's variant under each tagging
//! that writes a variant's named members.

use super::unwritten_models::furling_kotlin;
#[cfg(feature = "serde")]
use super::unwritten_models::{
    dousing_kotlin, flemishing_kotlin, ranging_kotlin, ratcheting_kotlin, reefing_kotlin,
    shackling_kotlin, splaying_kotlin, wearing_kotlin,
};

/// A struct with a member serde never writes takes the serializer the emitter writes in place of
/// the plugin's, which writes every property: the member is read, and no key is written for it.
#[test]
fn a_member_serde_never_writes_is_read_and_has_no_put() {
    let furling = super::tests::renumbered(&furling_kotlin::kotlin_definition());
    assert!(
        furling.contains(
            "@Serializable(with = FurlingSerializer::class) data class Furling(val label: \
             String, val spare: UInt? = null)"
        ),
        "got: {furling}"
    );
    assert!(
        !furling.contains("@KeepGeneratedSerializer"),
        "got: {furling}"
    );
    assert!(
        furling.contains(
            "output.encodeJsonElement(buildJsonObject { put(\"label\", \
             output.json.encodeToJsonElement(serializer<String>(), value.label)) })"
        ),
        "got: {furling}"
    );
    assert!(
        furling.contains(
            "val spare = obj[\"spare\"]?.let { \
             input.json.decodeFromJsonElement(KotlinNumber1.nullable, it) }"
        ),
        "got: {furling}"
    );
}

/// Beside a flattened member, the member is read and no key is written for it.
#[cfg(feature = "serde")]
#[test]
fn a_member_serde_never_writes_beside_a_flattened_one_has_no_put() {
    let reefing = super::tests::renumbered(&reefing_kotlin::kotlin_definition());
    assert!(
        reefing.contains(
            "output.encodeJsonElement(buildJsonObject { \
             output.json.encodeToJsonElement(BraidingUnjudged, value.braiding).jsonObject\
             .forEach { (k, v) -> put(k, v) } })"
        ),
        "got: {reefing}"
    );
    assert!(
        reefing.contains(
            "val spare = obj[\"spare\"]?.let { \
             input.json.decodeFromJsonElement(KotlinNumber1.nullable, it) }"
        ),
        "got: {reefing}"
    );
}

/// A variant with a member serde never writes takes the serializer the emitter writes,
/// `{Enum}{Variant}Unjudged`, in place of the plugin's, under each tagging that writes a variant's
/// named members: the member is read, and no key is written for it, and a key no member reads is
/// refused, as the plugin refuses it. The variant beside it, with no such member, keeps the
/// plugin's serializer.
#[cfg(feature = "serde")]
#[test]
fn a_variant_with_a_member_serde_never_writes_names_the_serializer_the_emitter_writes() {
    for (kotlin, name) in [
        (
            super::tests::renumbered(&dousing_kotlin::kotlin_definition()),
            "Dousing",
        ),
        (
            super::tests::renumbered(&wearing_kotlin::kotlin_definition()),
            "Wearing",
        ),
        (
            super::tests::renumbered(&shackling_kotlin::kotlin_definition()),
            "Shackling",
        ),
        (
            super::tests::renumbered(&ratcheting_kotlin::kotlin_definition()),
            "Ratcheting",
        ),
    ] {
        assert!(
            kotlin.contains(&format!(
                "@Serializable(with = {name}FurledUnjudged::class) data class {name}Furled(val \
                 label: String, val spare: UInt? = null) : {name} "
            )),
            "got: {kotlin}"
        );
        assert!(
            kotlin.contains(&format!(
                "object {name}FurledUnjudged : KSerializer<{name}Furled> {{ override val \
                 descriptor: SerialDescriptor = buildClassSerialDescriptor(\"{name}Furled\"); \
                 override fun serialize(encoder: Encoder, value: {name}Furled) {{ val output = \
                 encoder as JsonEncoder; output.encodeJsonElement(buildJsonObject {{ \
                 put(\"label\", output.json.encodeToJsonElement(serializer<String>(), \
                 value.label)) }}) }}"
            )),
            "got: {kotlin}"
        );
        // Under a tag alone the variant is handed the whole object, which holds the tag too.
        let tag = if name == "Wearing" { ", \"kind\"" } else { "" };
        assert!(
            kotlin.contains(&format!(
                "val spare = obj[\"spare\"]?.let {{ \
                 input.json.decodeFromJsonElement(KotlinNumber1.nullable, it) }}; if \
                 (!input.json.configuration.ignoreUnknownKeys) (obj - setOf(\"label\", \
                 \"spare\"{tag})).keys.firstOrNull()?.let {{ throw \
                 SerializationException(\"Encountered an unknown key '\" + it + \"'\") }}; return \
                 {name}Furled(label = label, spare = spare) }} }}"
            )),
            "got: {kotlin}"
        );
        assert!(
            kotlin.contains(&format!(
                "@Serializable data class {name}Struck(val label: String) : {name} "
            )),
            "got: {kotlin}"
        );
        assert!(
            !kotlin.contains(&format!("{name}StruckUnjudged")),
            "got: {kotlin}"
        );
    }
}

/// A flattened struct and a flattened map serde never writes are read, and merge nothing into
/// what their holder writes. One that is no `Option` is read as a flattened member is, its key
/// being none to leave out.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_member_serde_never_writes_merges_nothing() {
    let flemishing = super::tests::renumbered(&flemishing_kotlin::kotlin_definition());
    let written = "output.encodeJsonElement(buildJsonObject { put(\"note\", \
                   output.json.encodeToJsonElement(serializer<String>(), value.note)) })";
    assert!(
        flemishing.contains(written),
        "no `{written}` in: {flemishing}"
    );
    let read = "val braiding = if (obj.keys.any { it in braidingKeys })";
    assert!(flemishing.contains(read), "no `{read}` in: {flemishing}");
    let splaying = super::tests::renumbered(&splaying_kotlin::kotlin_definition());
    for held in [
        "data class Splaying(val braiding: Braiding, val note: String)",
        "val braiding = input.json.decodeFromJsonElement(BraidingUnjudged, \
         JsonObject(obj.filterKeys { it in setOf(\"label\") }));",
        written,
    ] {
        assert!(splaying.contains(held), "no `{held}` in: {splaying}");
    }
    let ranging = super::tests::renumbered(&ranging_kotlin::kotlin_definition());
    for held in [
        "data class Ranging(val counts: Map<String, UByte>)",
        "output.encodeJsonElement(buildJsonObject {  })",
        "val counts = obj.mapValues { (_, v) -> \
         input.json.decodeFromJsonElement(KotlinNumber1, v) };",
    ] {
        assert!(ranging.contains(held), "no `{held}` in: {ranging}");
    }
}
