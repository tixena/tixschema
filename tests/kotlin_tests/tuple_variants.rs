//! An enum with a variant of several slots: the subclass declares its slots with one tuple
//! wrapper, and the enum's serializer writes and reads the slots by that wrapper. A tuple a writer
//! names more than once is declared and read by one wrapper per shape in the item, and a tuple
//! holding a model `validate()` walks to is declared and read by the wrapper that reads the model
//! unjudged.

#[cfg(not(feature = "jsonschema"))]
use std::collections::HashMap;

#[cfg(all(feature = "serde", not(feature = "jsonschema")))]
use super::tuple_variants_models::binning_kotlin;
use super::tuple_variants_models::{
    Banding, Bundling, Crating, Labelling, Palleting, Piling, Racking, Stacking, Stencil,
    Strapping, Wrapping, crating_kotlin, palleting_kotlin, stacking_kotlin,
};
#[cfg(not(feature = "jsonschema"))]
use super::tuple_variants_models::{Binning, Tallying, tallying_kotlin};
#[cfg(feature = "serde")]
use super::tuple_variants_models::{
    banding_kotlin, bundling_kotlin, piling_kotlin, racking_kotlin, strapping_kotlin,
    wrapping_kotlin,
};

/// The payload each type below writes and reads.
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
    #[cfg(not(feature = "jsonschema"))]
    assert_eq!(
        serde_json::to_string(&Tallying::Counts(HashMap::from([(
            "north".to_owned(),
            (1, 2)
        )])))
        .unwrap(),
        r#"{"kind":"Counts","held":{"north":[1,2]}}"#
    );
    assert_eq!(
        serde_json::from_str::<Strapping>("{}").unwrap(),
        Strapping { pair: (0, 0) }
    );
    assert_eq!(
        serde_json::to_string(&Bundling {
            labelling: Labelling {
                label: "north".to_owned()
            },
            pair: (1, 2)
        })
        .unwrap(),
        r#"{"label":"north","pair":[1,2]}"#
    );
    assert_eq!(
        serde_json::to_string(&Palleting((1, 2), 3)).unwrap(),
        "[[1,2],3]"
    );
    assert_eq!(
        serde_json::to_string(&Crating { pair: ((1, 2), 3) }).unwrap(),
        r#"{"pair":[[1,2],3]}"#
    );
    assert_eq!(
        serde_json::to_string(&Wrapping { pair: (north(), 2) }).unwrap(),
        r#"{"pair":[{"code":"north"},2]}"#
    );
    assert_eq!(
        serde_json::to_string(&Banding {
            labelling: Labelling {
                label: "north".to_owned()
            },
            pair: (north(), 2)
        })
        .unwrap(),
        r#"{"label":"north","pair":[{"code":"north"},2]}"#
    );
    #[cfg(not(feature = "jsonschema"))]
    assert_eq!(
        serde_json::to_string(&Binning::Counts(HashMap::from([(
            "north".to_owned(),
            (north(), 2)
        )])))
        .unwrap(),
        r#"{"kind":"Counts","held":{"north":[{"code":"north"},2]}}"#
    );
}

/// A model whose code holds its bound.
fn north() -> Stencil {
    Stencil {
        code: "north".to_owned(),
    }
}

/// The tuple wrappers `definition` declares, in the order declared, after asserting that each
/// one it names is one of them.
fn declared_wrappers(definition: &str) -> Vec<String> {
    let named_after = |marker: &str| -> Vec<String> {
        definition
            .match_indices(marker)
            .map(|(at, _)| {
                let index: String = definition[at + marker.len()..]
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect();
                format!("KotlinTuple{index}")
            })
            .collect()
    };
    let declared = named_after("data class KotlinTuple");
    for named in named_after("KotlinTuple") {
        assert!(
            declared.contains(&named),
            "`{named}` is not declared in: {definition}"
        );
    }
    declared
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

/// A map of tuples in a variant's one slot: the subclass declares it, and the enum's serializer
/// writes and reads it, by one wrapper.
#[cfg(not(feature = "jsonschema"))]
#[test]
fn a_map_of_tuples_in_a_variant_is_declared_and_read_by_one_wrapper() {
    let tallying = tallying_kotlin::kotlin_definition();
    let [wrapper] = declared_wrappers(&tallying).try_into().unwrap();
    assert!(
        tallying.contains(&format!(
            "data class TallyingCounts(val value: Map<String, {wrapper}>)"
        )),
        "got: {tallying}"
    );
    assert_eq!(
        tallying
            .matches(&format!("serializer<Map<String, {wrapper}>>()"))
            .count(),
        2,
        "got: {tallying}"
    );
}

/// A tuple read as its default: the property is declared, and its default read, by one wrapper.
#[cfg(feature = "serde")]
#[test]
fn a_tuple_read_as_its_default_is_declared_and_read_by_one_wrapper() {
    let strapping = strapping_kotlin::kotlin_definition();
    let [wrapper] = declared_wrappers(&strapping).try_into().unwrap();
    assert!(
        strapping.contains(&format!(
            "data class Strapping(val pair: {wrapper} = \
             Json.decodeFromString(serializer<{wrapper}>(), \"[0,0]\"))"
        )),
        "got: {strapping}"
    );
}

/// A tuple beside a flattened member: the property is declared, and the holder's serializer
/// writes and reads it, by one wrapper.
#[cfg(feature = "serde")]
#[test]
fn a_tuple_beside_a_flattened_member_is_declared_and_read_by_one_wrapper() {
    let bundling = bundling_kotlin::kotlin_definition();
    let [wrapper] = declared_wrappers(&bundling).try_into().unwrap();
    for held in [
        format!("val pair: {wrapper})"),
        format!(
            "put(\"pair\", output.json.encodeToJsonElement(serializer<{wrapper}>(), value.pair))"
        ),
        format!(
            "val pair = input.json.decodeFromJsonElement(serializer<{wrapper}>(), \
             obj.getValue(\"pair\"))"
        ),
    ] {
        assert!(bundling.contains(&held), "no `{held}` in: {bundling}");
    }
}

/// A tuple in a slot of a tuple struct: the slot is declared, and the struct's serializer writes
/// and reads it, by one wrapper.
#[test]
fn a_tuple_in_a_slot_of_a_tuple_struct_is_declared_and_read_by_one_wrapper() {
    let palleting = palleting_kotlin::kotlin_definition();
    let [wrapper] = declared_wrappers(&palleting).try_into().unwrap();
    for held in [
        format!("data class Palleting(val slot0: {wrapper}, val slot1: UByte)"),
        format!("add(output.json.encodeToJsonElement(serializer<{wrapper}>(), value.slot0))"),
        format!("input.json.decodeFromJsonElement(serializer<{wrapper}>(), array[0])"),
    ] {
        assert!(palleting.contains(&held), "no `{held}` in: {palleting}");
    }
}

/// A tuple in a position of a tuple: the outer wrapper declares the position, and writes and
/// reads it, by the one inner wrapper.
#[test]
fn a_tuple_in_a_position_of_a_tuple_is_declared_and_read_by_one_wrapper() {
    let crating = crating_kotlin::kotlin_definition();
    let [inner, outer] = declared_wrappers(&crating).try_into().unwrap();
    for held in [
        format!("data class Crating(val pair: {outer})"),
        format!("data class {outer}(val slot0: {inner}, val slot1: UByte)"),
        format!("add(output.json.encodeToJsonElement(serializer<{inner}>(), value.slot0))"),
        format!("input.json.decodeFromJsonElement(serializer<{inner}>(), array[0])"),
    ] {
        assert!(crating.contains(&held), "no `{held}` in: {crating}");
    }
}

/// The wrapper `definition` declares, after asserting it declares one, and that the wrapper reads
/// the model of its first position unjudged.
#[cfg(feature = "serde")]
fn the_walked_wrapper(definition: &str) -> String {
    let [wrapper] = declared_wrappers(definition).try_into().unwrap();
    for held in [
        format!("data class {wrapper}(val slot0: Stencil, val slot1: UByte)"),
        "add(output.json.encodeToJsonElement(StencilUnjudged, value.slot0))".to_owned(),
        "input.json.decodeFromJsonElement(StencilUnjudged, array[0])".to_owned(),
    ] {
        assert!(definition.contains(&held), "no `{held}` in: {definition}");
    }
    wrapper
}

/// A tuple holding a model, read as its default: the property is declared, and its default read,
/// by the one wrapper that reads the model unjudged.
#[cfg(feature = "serde")]
#[test]
fn a_tuple_holding_a_model_read_as_its_default_is_declared_and_read_by_one_wrapper() {
    let wrapping = wrapping_kotlin::kotlin_definition();
    let wrapper = the_walked_wrapper(&wrapping);
    let default = r#""[{\"code\":\"\"},0]""#;
    assert!(
        wrapping.contains(&format!(
            "data class Wrapping(val pair: {wrapper} = \
             Json.decodeFromString(serializer<{wrapper}>(), {default}))"
        )),
        "got: {wrapping}"
    );
}

/// A tuple holding a model beside a flattened member: the property is declared, and the holder's
/// serializer writes and reads it, by the one wrapper that reads the model unjudged.
#[cfg(feature = "serde")]
#[test]
fn a_tuple_holding_a_model_beside_a_flattened_member_is_declared_and_read_by_one_wrapper() {
    let banding = banding_kotlin::kotlin_definition();
    let wrapper = the_walked_wrapper(&banding);
    for held in [
        format!("data class Banding(val labelling: Labelling, val pair: {wrapper})"),
        format!(
            "put(\"pair\", output.json.encodeToJsonElement(serializer<{wrapper}>(), value.pair))"
        ),
        format!(
            "val pair = input.json.decodeFromJsonElement(serializer<{wrapper}>(), \
             obj.getValue(\"pair\"))"
        ),
    ] {
        assert!(banding.contains(&held), "no `{held}` in: {banding}");
    }
}

/// A map of tuples holding a model in a variant's one slot: the subclass declares it, and the
/// enum's serializer writes and reads it, by the one wrapper that reads the model unjudged.
#[cfg(all(feature = "serde", not(feature = "jsonschema")))]
#[test]
fn a_map_of_tuples_holding_a_model_in_a_variant_is_declared_and_read_by_one_wrapper() {
    let binning = binning_kotlin::kotlin_definition();
    let wrapper = the_walked_wrapper(&binning);
    assert!(
        binning.contains(&format!(
            "data class BinningCounts(val value: Map<String, {wrapper}>)"
        )),
        "got: {binning}"
    );
    assert_eq!(
        binning
            .matches(&format!(
                "MapSerializer(serializer<String>(), serializer<{wrapper}>())"
            ))
            .count(),
        2,
        "got: {binning}"
    );
}
