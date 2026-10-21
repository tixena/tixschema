//! An enum with a variant of several slots: the subclass declares its slots with one tuple
//! wrapper, and the enum's serializer writes and reads the slots by that wrapper.

use super::tuple_variants_models::{Piling, Racking, Stacking, stacking_kotlin};
#[cfg(feature = "serde")]
use super::tuple_variants_models::{piling_kotlin, racking_kotlin};

/// The payload each enum's arms below write and read.
#[test]
fn every_fixture_is_written_as_serde_writes_it() {
    assert_eq!(
        serde_json::to_string(&Racking::Split(vec![1, 2], 3)).unwrap(),
        r#"{"kind":"Split","held":[[1,2],3]}"#
    );
    assert_eq!(
        serde_json::to_string(&Stacking::Split(vec![1, 2], 3)).unwrap(),
        r#"{"Split":[[1,2],3]}"#
    );
    assert_eq!(
        serde_json::to_string(&Piling::Split((vec![1, 2], 3))).unwrap(),
        "[[1,2],3]"
    );
}

/// The wrapper `definition` declares, after asserting it declares one: the type of the value
/// `subclass` holds.
fn the_one_wrapper(definition: &str, subclass: &str) -> String {
    assert_eq!(
        definition.matches("data class KotlinTuple").count(),
        1,
        "got: {definition}"
    );
    let declared = format!("{subclass}(val value: KotlinTuple");
    let at = definition.find(&declared).unwrap() + declared.len();
    let index: String = definition[at..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    format!("KotlinTuple{index}")
}

/// Under a tag and a content key, the slots are written and read under the content key by the
/// wrapper the subclass declares.
#[cfg(feature = "serde")]
#[test]
fn the_slots_under_a_content_key_are_written_and_read_by_the_wrapper_declared() {
    let racking = racking_kotlin::kotlin_definition();
    let wrapper = the_one_wrapper(&racking, "RackingSplit");
    assert!(
        racking.contains(&format!(
            "@Serializable data class RackingSplit(val value: {wrapper}) : Racking"
        )),
        "got: {racking}"
    );
    assert!(
        racking.contains(&format!(
            "is RackingSplit -> buildJsonObject {{ put(\"kind\", \"Split\"); put(\"held\", \
             output.json.encodeToJsonElement(serializer<{wrapper}>(), value.value)) }}"
        )),
        "got: {racking}"
    );
    assert!(
        racking.contains(&format!(
            "\"Split\" -> RackingSplit(input.json.decodeFromJsonElement(serializer<{wrapper}>(), \
             data!!))"
        )),
        "got: {racking}"
    );
}

/// Under the variant's name, the slots are written and read under that name by the wrapper the
/// subclass declares.
#[test]
fn the_slots_under_the_variants_name_are_written_and_read_by_the_wrapper_declared() {
    let stacking = stacking_kotlin::kotlin_definition();
    let wrapper = the_one_wrapper(&stacking, "StackingSplit");
    assert!(
        stacking.contains(&format!(
            "@Serializable data class StackingSplit(val value: {wrapper}) : Stacking"
        )),
        "got: {stacking}"
    );
    assert!(
        stacking.contains(&format!(
            "is StackingSplit -> buildJsonObject {{ put(\"Split\", \
             output.json.encodeToJsonElement(serializer<{wrapper}>(), value.value)) }}"
        )),
        "got: {stacking}"
    );
    assert!(
        stacking.contains(&format!(
            "\"Split\" -> StackingSplit(input.json.decodeFromJsonElement(serializer<{wrapper}>(), \
             data))"
        )),
        "got: {stacking}"
    );
}

/// Untagged, the subclass declares the tuple of its one slot with the one wrapper, and the enum's
/// serializer reads and writes the subclass by the subclass's own.
#[cfg(feature = "serde")]
#[test]
fn an_untagged_slot_holding_the_slots_declares_one_wrapper() {
    let piling = piling_kotlin::kotlin_definition();
    let wrapper = the_one_wrapper(&piling, "PilingSplit");
    assert!(
        piling.contains(&format!(
            "@JvmInline @Serializable value class PilingSplit(val value: {wrapper}) : Piling"
        )),
        "got: {piling}"
    );
    assert!(
        piling.contains(
            "is PilingSplit -> output.json.encodeToJsonElement(serializer<PilingSplit>(), value)"
        ),
        "got: {piling}"
    );
}
