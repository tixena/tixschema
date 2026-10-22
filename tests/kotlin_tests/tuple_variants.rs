//! An enum with a variant of several slots: the subclass declares its slots with one tuple
//! wrapper, and the enum's serializer writes and reads the slots by that wrapper. A tuple a writer
//! names more than once is declared and read by one wrapper per shape in the item, and a tuple
//! holding a model `validate()` walks to is declared and read by the wrapper that reads the model
//! unjudged.

#[cfg(not(feature = "jsonschema"))]
use std::collections::HashMap;

use super::tuple_variants_models::{
    Banding, Bundling, Crating, Grading, Labelling, Palleting, Piling, Racking, Ranking, Slotting,
    Spooling, Stacking, Stencil, Strapping, Wrapping, crating_kotlin, grading_kotlin,
    palleting_kotlin, ranking_kotlin, slotting_kotlin, stacking_kotlin,
};
#[cfg(not(feature = "jsonschema"))]
use super::tuple_variants_models::{
    Binning, Coiling, Heaping, Lashing, Tallying, coiling_kotlin, heaping_kotlin, tallying_kotlin,
};
#[cfg(feature = "serde")]
use super::tuple_variants_models::{
    banding_kotlin, bundling_kotlin, piling_kotlin, racking_kotlin, spooling_kotlin,
    strapping_kotlin, wrapping_kotlin,
};
#[cfg(all(feature = "serde", not(feature = "jsonschema")))]
use super::tuple_variants_models::{binning_kotlin, lashing_kotlin};

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
        serde_json::to_string(&Ranking::Places(Ok((1, 2)))).unwrap(),
        r#"{"kind":"Places","held":{"Ok":[1,2]}}"#
    );
    assert_eq!(
        serde_json::to_string(&Ranking::Places(Err("none".to_owned()))).unwrap(),
        r#"{"kind":"Places","held":{"Err":"none"}}"#
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

/// The payload each holder of a tuple holding a model further in writes and reads: in a slot of a
/// tuple struct, in a `Result` and in a map in a tuple, in an `Option` and a list, and in a map
/// read flattened or through a function.
#[test]
fn every_fixture_holding_a_tuple_further_in_is_written_as_serde_writes_it() {
    assert_eq!(
        serde_json::to_string(&Slotting((north(), 2), 3)).unwrap(),
        r#"[[{"code":"north"},2],3]"#
    );
    assert_eq!(
        serde_json::to_string(&Grading {
            pair: (Ok((north(), 2)), 3)
        })
        .unwrap(),
        r#"{"pair":[{"Ok":[{"code":"north"},2]},3]}"#
    );
    assert_eq!(
        serde_json::to_string(&Spooling {
            labelling: Labelling {
                label: "north".to_owned()
            },
            maybe: Some((north(), 3)),
            pairs: vec![(north(), 2)]
        })
        .unwrap(),
        r#"{"label":"north","maybe":[{"code":"north"},3],"pairs":[[{"code":"north"},2]]}"#
    );
    #[cfg(not(feature = "jsonschema"))]
    assert_eq!(
        serde_json::to_string(&Coiling {
            pair: (HashMap::from([("north".to_owned(), (north(), 2))]), 3)
        })
        .unwrap(),
        r#"{"pair":[{"north":[{"code":"north"},2]},3]}"#
    );
    #[cfg(not(feature = "jsonschema"))]
    assert_eq!(
        serde_json::to_string(&Lashing {
            dock: "south".to_owned(),
            extra: HashMap::from([("north".to_owned(), (north(), 2))])
        })
        .unwrap(),
        r#"{"dock":"south","north":[{"code":"north"},2]}"#
    );
    #[cfg(not(feature = "jsonschema"))]
    assert_eq!(
        serde_json::to_string(&Heaping {
            counts: HashMap::from([("north".to_owned(), (north(), 2))])
        })
        .unwrap(),
        r#"{"counts":{"north":[{"code":"north"},2]}}"#
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
             (data ?: throw SerializationException(\"missing field `held`\"))))"
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

/// A `Result` holding a tuple in a variant's one slot: the subclass declares it, and the enum's
/// serializer writes and reads it, by one wrapper.
#[test]
fn a_result_holding_a_tuple_in_a_variant_is_declared_and_read_by_one_wrapper() {
    let ranking = ranking_kotlin::kotlin_definition();
    let [wrapper] = declared_wrappers(&ranking).try_into().unwrap();
    let opens = "data class RankingPlaces(val value: ";
    let at = ranking.find(opens).unwrap() + opens.len();
    let declared = &ranking[at..at + ranking[at..].find(')').unwrap()];
    assert!(
        declared.starts_with("KotlinResult") && declared.ends_with(&format!("<{wrapper}, String>")),
        "got: {ranking}"
    );
    assert_eq!(
        ranking
            .matches(&format!("serializer<{declared}>()"))
            .count(),
        2,
        "got: {ranking}"
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
             (obj[\"pair\"] ?: throw SerializationException(\"missing field `pair`\")))"
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

/// Asserts that `wrapper`, declared in `definition`, reads the model of its first position
/// unjudged.
fn reads_the_model_unjudged(definition: &str, wrapper: &str) {
    for held in [
        format!("data class {wrapper}(val slot0: Stencil, val slot1: UByte)"),
        "add(output.json.encodeToJsonElement(StencilUnjudged, value.slot0))".to_owned(),
        "input.json.decodeFromJsonElement(StencilUnjudged, array[0])".to_owned(),
    ] {
        assert!(definition.contains(&held), "no `{held}` in: {definition}");
    }
}

/// The wrapper `definition` declares, after asserting it declares one, and that the wrapper reads
/// the model of its first position unjudged.
fn the_walked_wrapper(definition: &str) -> String {
    let [wrapper] = declared_wrappers(definition).try_into().unwrap();
    reads_the_model_unjudged(definition, &wrapper);
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
             (obj[\"pair\"] ?: throw SerializationException(\"missing field `pair`\")))"
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

/// A tuple holding a model in a slot of a tuple struct: the slot is declared, and the struct's
/// serializer writes and reads it, by the one wrapper that reads the model unjudged.
#[test]
fn a_tuple_holding_a_model_in_a_slot_of_a_tuple_struct_is_declared_and_read_by_one_wrapper() {
    let slotting = slotting_kotlin::kotlin_definition();
    let wrapper = the_walked_wrapper(&slotting);
    for held in [
        format!("data class Slotting(val slot0: {wrapper}, val slot1: UByte)"),
        format!("add(output.json.encodeToJsonElement(serializer<{wrapper}>(), value.slot0))"),
        format!("input.json.decodeFromJsonElement(serializer<{wrapper}>(), array[0])"),
    ] {
        assert!(slotting.contains(&held), "no `{held}` in: {slotting}");
    }
}

/// A tuple holding a model in the `Ok` arm of a `Result` in a position of a tuple: the outer
/// wrapper declares the arm, and writes and reads it, by the one wrapper that reads the model
/// unjudged.
#[test]
fn a_tuple_holding_a_model_in_a_result_in_a_tuple_is_declared_and_read_by_one_wrapper() {
    let grading = grading_kotlin::kotlin_definition();
    let [inner, outer] = declared_wrappers(&grading).try_into().unwrap();
    reads_the_model_unjudged(&grading, &inner);
    assert!(
        grading.contains(&format!("data class Grading(val pair: {outer})")),
        "got: {grading}"
    );
    let opens = format!("data class {outer}(val slot0: ");
    let at = grading.find(&opens).unwrap() + opens.len();
    let declared = &grading[at..at + grading[at..].find(", val slot1: UByte)").unwrap()];
    let arguments = format!("<{inner}, UByte>");
    assert!(
        declared.starts_with("KotlinResult") && declared.ends_with(&arguments),
        "got: {grading}"
    );
    let result = declared.strip_suffix(&arguments).unwrap();
    assert_eq!(
        grading
            .matches(&format!(
                "{result}Serializer(serializer<{inner}>(), serializer<UByte>())"
            ))
            .count(),
        2,
        "got: {grading}"
    );
}

/// Tuples holding a model in an `Option` and in a list, beside a flattened member: both
/// properties are declared, and the holder's serializer writes and reads both, by the one wrapper
/// that reads the model unjudged.
#[cfg(feature = "serde")]
#[test]
fn tuples_holding_a_model_in_an_option_and_a_list_are_declared_and_read_by_one_wrapper() {
    let spooling = spooling_kotlin::kotlin_definition();
    let wrapper = the_walked_wrapper(&spooling);
    for held in [
        format!(
            "data class Spooling(val labelling: Labelling, val maybe: {wrapper}? = null, \
             val pairs: List<{wrapper}>)"
        ),
        format!(
            "put(\"maybe\", output.json.encodeToJsonElement(serializer<{wrapper}>().nullable, \
             value.maybe))"
        ),
        format!(
            "put(\"pairs\", \
             output.json.encodeToJsonElement(ListSerializer(serializer<{wrapper}>()), \
             value.pairs))"
        ),
        format!(
            "val maybe = obj[\"maybe\"]?.let {{ \
             input.json.decodeFromJsonElement(serializer<{wrapper}>().nullable, it) }}"
        ),
        format!(
            "val pairs = \
             input.json.decodeFromJsonElement(ListSerializer(serializer<{wrapper}>()), \
             (obj[\"pairs\"] ?: throw SerializationException(\"missing field `pairs`\")))"
        ),
    ] {
        assert!(spooling.contains(&held), "no `{held}` in: {spooling}");
    }
}

/// A map of tuples holding a model in a position of a tuple: the outer wrapper declares the map,
/// and writes and reads it, by the one wrapper that reads the model unjudged.
#[cfg(not(feature = "jsonschema"))]
#[test]
fn a_map_of_tuples_holding_a_model_in_a_tuple_is_declared_and_read_by_one_wrapper() {
    let coiling = coiling_kotlin::kotlin_definition();
    let [inner, outer] = declared_wrappers(&coiling).try_into().unwrap();
    reads_the_model_unjudged(&coiling, &inner);
    for held in [
        format!("data class Coiling(val pair: {outer})"),
        format!("data class {outer}(val slot0: Map<String, {inner}>, val slot1: UByte)"),
    ] {
        assert!(coiling.contains(&held), "no `{held}` in: {coiling}");
    }
    assert_eq!(
        coiling
            .matches(&format!(
                "MapSerializer(serializer<String>(), serializer<{inner}>())"
            ))
            .count(),
        2,
        "got: {coiling}"
    );
}

/// A flattened map of tuples holding a model, beside a member of the holder's own: the property is
/// declared, and the holder's serializer writes and reads each entry, by the one wrapper that
/// reads the model unjudged.
#[cfg(all(feature = "serde", not(feature = "jsonschema")))]
#[test]
fn a_flattened_map_of_tuples_holding_a_model_is_declared_and_read_by_one_wrapper() {
    let lashing = lashing_kotlin::kotlin_definition();
    let wrapper = the_walked_wrapper(&lashing);
    for held in [
        format!("data class Lashing(val dock: String, val extra: Map<String, {wrapper}>)"),
        format!("put(k, output.json.encodeToJsonElement(serializer<{wrapper}>(), v))"),
        format!("input.json.decodeFromJsonElement(serializer<{wrapper}>(), v)"),
    ] {
        assert!(lashing.contains(&held), "no `{held}` in: {lashing}");
    }
}

/// A map of tuples holding a model, read through a function a `preprocess` names: the property is
/// declared, and the serializer that corrects it reads it, by the one wrapper that reads the
/// model unjudged.
#[cfg(not(feature = "jsonschema"))]
#[test]
fn a_preprocessed_map_of_tuples_holding_a_model_is_declared_and_read_by_one_wrapper() {
    let heaping = heaping_kotlin::kotlin_definition();
    let wrapper = the_walked_wrapper(&heaping);
    for held in [
        format!(
            "data class Heaping(@Serializable(with = HeapingCountsPreprocess::class) val counts: \
             Map<String, {wrapper}>)"
        ),
        format!(
            "object HeapingCountsPreprocess : JsonTransformingSerializer<Map<String, {wrapper}>>(\
             MapSerializer(serializer<String>(), serializer<{wrapper}>()))"
        ),
    ] {
        assert!(heaping.contains(&held), "no `{held}` in: {heaping}");
    }
}
