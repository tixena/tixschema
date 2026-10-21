//! A member serde reads and never writes: the Kotlin serializer reads it and writes no key for it,
//! in a struct of its own and beside a flattened member.

use super::unwritten_models::furling_kotlin;
#[cfg(feature = "serde")]
use super::unwritten_models::reefing_kotlin;

/// A struct with a member serde never writes takes the serializer the emitter writes in place of
/// the plugin's, which writes every property: the member is read, and no key is written for it.
#[test]
fn a_member_serde_never_writes_is_read_and_has_no_put() {
    let furling = furling_kotlin::kotlin_definition();
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
             input.json.decodeFromJsonElement(serializer<UInt?>(), it) }"
        ),
        "got: {furling}"
    );
}

/// Beside a flattened member, the member is read and no key is written for it.
#[cfg(feature = "serde")]
#[test]
fn a_member_serde_never_writes_beside_a_flattened_one_has_no_put() {
    let reefing = reefing_kotlin::kotlin_definition();
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
             input.json.decodeFromJsonElement(serializer<UInt?>(), it) }"
        ),
        "got: {reefing}"
    );
}
