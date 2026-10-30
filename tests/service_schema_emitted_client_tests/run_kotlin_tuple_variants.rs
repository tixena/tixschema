//! An enum with a variant of several slots, read and written by the emitted Kotlin types compiled
//! with `-Werror` under each tagging serde accepts for one, and the untagged enum that writes the
//! same slots from a tuple: each payload is written back as the serde read writes it back. So is
//! a tuple in each place a writer names its wrapper more than once, and a tuple holding a model
//! `validate()` walks to, which a payload that breaks the model's bound is refused for at the
//! holder's path.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use std::collections::HashMap;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::runtime::ran_kotlin;
use super::tuple_variants_models::{
    Banding, Bundling, Crating, Grading, Lashing, Palleting, Piling, Racking, Ranking, Slotting,
    Spooling, Stacking, Strapping, Wrapping, banding_kotlin, bundling_kotlin, crating_kotlin,
    grading_kotlin, labelling_kotlin, lashing_kotlin, palleting_kotlin, piling_kotlin,
    racking_kotlin, ranking_kotlin, slotting_kotlin, spooling_kotlin, stacking_kotlin,
    stencil_kotlin, strapping_kotlin, wrapping_kotlin,
};
#[cfg(not(feature = "jsonschema"))]
use super::tuple_variants_models::{
    Binning, Coiling, Heaping, Tallying, binning_kotlin, coiling_kotlin, heaping_kotlin,
    tallying_kotlin,
};
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::tuple_variants_models::{Labelling, Stencil};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

const ROWS: [Row; 6] = [
    Row {
        json: r#"{"kind":"Split","held":[[1,2],3]}"#,
        name: "slots under a tag and a content key",
        read_as: "Racking",
        written: written_back::<Racking>,
    },
    Row {
        json: r#"{"kind":"Loose","held":{"members":["a"]}}"#,
        name: "members under a tag and a content key",
        read_as: "Racking",
        written: written_back::<Racking>,
    },
    Row {
        json: r#"{"Split":[[1,2],3]}"#,
        name: "slots under the variant's name",
        read_as: "Stacking",
        written: written_back::<Stacking>,
    },
    Row {
        json: r#"{"Loose":{"members":["a"]}}"#,
        name: "members under the variant's name",
        read_as: "Stacking",
        written: written_back::<Stacking>,
    },
    Row {
        json: "[[1,2],3]",
        name: "slots untagged",
        read_as: "Piling",
        written: written_back::<Piling>,
    },
    Row {
        json: r#"{"members":["a"]}"#,
        name: "members untagged",
        read_as: "Piling",
        written: written_back::<Piling>,
    },
];

/// A tuple in each place a writer names its wrapper more than once.
const NAMED_TWICE: [Row; 5] = [
    Row {
        json: r#"{"pair":[1,2]}"#,
        name: "a tuple with a default, its key held",
        read_as: "Strapping",
        written: written_back::<Strapping>,
    },
    Row {
        json: "{}",
        name: "a tuple with a default, its key left out",
        read_as: "Strapping",
        written: written_back::<Strapping>,
    },
    Row {
        json: r#"{"label":"north","pair":[1,2]}"#,
        name: "a tuple beside a flattened member",
        read_as: "Bundling",
        written: written_back::<Bundling>,
    },
    Row {
        json: "[[1,2],3]",
        name: "a tuple in a slot of a tuple struct",
        read_as: "Palleting",
        written: written_back::<Palleting>,
    },
    Row {
        json: r#"{"pair":[[1,2],3]}"#,
        name: "a tuple in a position of a tuple",
        read_as: "Crating",
        written: written_back::<Crating>,
    },
];

/// A tuple holding a model `validate()` walks to, in each place a writer reads the tuple by a
/// wrapper of its own: a payload that holds the model whole, and one that breaks its bound.
const HOLDING_A_MODEL: [Row; 5] = [
    Row {
        json: r#"{"pair":[{"code":"north"},2]}"#,
        name: "a tuple holding a model with a default, its key held",
        read_as: "Wrapping",
        written: written_back::<Wrapping>,
    },
    Row {
        json: r#"{"pair":[{"code":""},2]}"#,
        name: "a tuple holding a broken model with a default",
        read_as: "Wrapping",
        written: refused_at_the_pair,
    },
    Row {
        json: "{}",
        name: "a tuple holding a model with a default, its key left out",
        read_as: "Wrapping",
        written: refused_at_the_pair,
    },
    Row {
        json: r#"{"label":"north","pair":[{"code":"north"},2]}"#,
        name: "a tuple holding a model beside a flattened member",
        read_as: "Banding",
        written: written_back::<Banding>,
    },
    Row {
        json: r#"{"label":"north","pair":[{"code":""},2]}"#,
        name: "a tuple holding a broken model beside a flattened member",
        read_as: "Banding",
        written: refused_at_the_pair,
    },
];

/// What the holder's `validate()` answers of a pair whose model breaks its bound, as the model's
/// default does, its code being empty.
const PAIR_REFUSED: &str = "'pair.0.code': too short: minimum length is 1, got 0";

/// What the holder's `validate()` answers of a map of pairs, in a variant's one slot, whose model
/// breaks its bound under the key `north`.
#[cfg(not(feature = "jsonschema"))]
const COUNT_REFUSED: &str = "'held.north.0.code': too short: minimum length is 1, got 0";

/// What a tuple struct's `validate()` answers of a pair in its first slot whose model breaks its
/// bound.
const SLOT_REFUSED: &str = "'0.0.code': too short: minimum length is 1, got 0";

/// What the holder's `validate()` answers of a pair in the `Ok` arm of a `Result`, in the first
/// position of a pair, whose model breaks its bound.
const ARM_REFUSED: &str = "'pair.0.Ok.0.code': too short: minimum length is 1, got 0";

/// What the holder's `validate()` answers of a pair in an `Option` whose model breaks its bound.
const OPTION_REFUSED: &str = "'maybe.0.code': too short: minimum length is 1, got 0";

/// What the holder's `validate()` answers of the first pair of a list whose model breaks its
/// bound.
const LIST_REFUSED: &str = "'pairs.0.0.code': too short: minimum length is 1, got 0";

/// What the holder's `validate()` answers of a map of pairs, in the first position of a pair,
/// whose model breaks its bound under the key `north`.
#[cfg(not(feature = "jsonschema"))]
const KEYED_PAIR_REFUSED: &str = "'pair.0.north.0.code': too short: minimum length is 1, got 0";

/// What the holder's `validate()` answers of a map of pairs under a key of its own whose model
/// breaks its bound under the key `north`.
const OWN_KEY_REFUSED: &str = "'extra.north.0.code': too short: minimum length is 1, got 0";

/// What the holder's `validate()` answers of a map of pairs, read through a function a
/// `preprocess` names, whose model breaks its bound under the key `north`.
#[cfg(not(feature = "jsonschema"))]
const PREPROCESSED_REFUSED: &str = "'counts.north.0.code': too short: minimum length is 1, got 0";

/// The function a `preprocess` names on [`Heaping`], in Kotlin: the payload as it was handed.
#[cfg(not(feature = "jsonschema"))]
const UNCHANGED: &str = "fun unchanged(payload: JsonElement): JsonElement = payload";

/// What prints a row: the value read, written back with every member, as serde writes it, or
/// what the type's own serializer refused it with.
const TOLD: &str = r#"
val everyMember = Json { encodeDefaults = true }

fun <T> row(name: String, serializer: KSerializer<T>, payload: String) {
    try {
        val read = Json.decodeFromString(serializer, payload)
        println("$name: ${everyMember.encodeToString(serializer, read)}")
    } catch (refused: SerializationException) {
        println("$name: refused: ${refused.message}")
    }
}
"#;

/// A payload, the type it is read as, and what the read of it prints: what serde writes back of
/// it, or the refusal of it.
struct Row {
    json: &'static str,
    name: &'static str,
    read_as: &'static str,
    written: fn(&str) -> String,
}

/// What `rows` print: each name beside what serde writes back of its payload.
fn expected(rows: &[Row]) -> String {
    let mut expected = String::new();
    for row in rows {
        let _ = writeln!(expected, "{}: {}", row.name, (row.written)(row.json));
    }
    expected
}

/// What the emitted Kotlin of `definitions` prints of `rows`, or `None` where no Kotlin toolchain
/// is reachable.
fn printed(rows: &[Row], definitions: Vec<String>) -> Option<String> {
    let mut main = String::from("fun main() {\n");
    for row in rows {
        let _ = writeln!(
            main,
            "    row(\"{}\", {}.serializer(), \"\"\"{}\"\"\")",
            row.name, row.read_as, row.json
        );
    }
    main.push_str("}\n");
    let mut source = vec![KOTLIN_IMPORTS.to_owned()];
    source.extend(definitions);
    source.extend([TOLD.to_owned(), main]);
    ran_kotlin(&source.join("\n\n"))
}

/// What serde writes back of `json`, read as `T`.
fn written_back<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    let read: T = serde_json::from_str(json).unwrap();
    serde_json::to_string(&read).unwrap()
}

/// What a row refused with [`PAIR_REFUSED`] prints.
fn refused_at_the_pair(_json: &str) -> String {
    format!("refused: {PAIR_REFUSED}")
}

/// What a row refused with [`COUNT_REFUSED`] prints.
#[cfg(not(feature = "jsonschema"))]
fn refused_at_the_count(_json: &str) -> String {
    format!("refused: {COUNT_REFUSED}")
}

/// What a row refused with [`SLOT_REFUSED`] prints.
fn refused_at_the_slot(_json: &str) -> String {
    format!("refused: {SLOT_REFUSED}")
}

/// What a row refused with [`ARM_REFUSED`] prints.
fn refused_at_the_arm(_json: &str) -> String {
    format!("refused: {ARM_REFUSED}")
}

/// What a row refused with [`OPTION_REFUSED`] and [`LIST_REFUSED`] prints.
fn refused_in_the_option_and_the_list(_json: &str) -> String {
    format!("refused: {OPTION_REFUSED}; {LIST_REFUSED}")
}

/// What a row refused with [`KEYED_PAIR_REFUSED`] prints.
#[cfg(not(feature = "jsonschema"))]
fn refused_at_the_keyed_pair(_json: &str) -> String {
    format!("refused: {KEYED_PAIR_REFUSED}")
}

/// What a row refused with [`OWN_KEY_REFUSED`] prints.
fn refused_at_the_own_key(_json: &str) -> String {
    format!("refused: {OWN_KEY_REFUSED}")
}

/// What a row refused with [`PREPROCESSED_REFUSED`] prints.
#[cfg(not(feature = "jsonschema"))]
fn refused_at_the_preprocessed_count(_json: &str) -> String {
    format!("refused: {PREPROCESSED_REFUSED}")
}

/// Each variant of several slots, and the variant beside it, read from the payload serde writes
/// for it and written back as serde writes it back.
#[test]
fn a_variant_of_several_slots_reads_and_writes_in_kotlin_as_serde_does() {
    let expected = expected(&ROWS);
    assert!(
        expected
            .contains(r#"slots under a tag and a content key: {"kind":"Split","held":[[1,2],3]}"#),
        "{expected}"
    );
    let definitions = vec![
        racking_kotlin::kotlin_definition(),
        stacking_kotlin::kotlin_definition(),
        piling_kotlin::kotlin_definition(),
    ];
    let Some(printed) = printed(&ROWS, definitions) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A tuple whose writer names its wrapper twice, read from the payload serde writes for it and
/// written back as serde writes it back.
#[test]
fn a_tuple_named_twice_by_one_writer_reads_and_writes_in_kotlin_as_serde_does() {
    let expected = expected(&NAMED_TWICE);
    assert!(
        expected.contains(r#"a tuple with a default, its key left out: {"pair":[0,0]}"#),
        "{expected}"
    );
    let definitions = vec![
        strapping_kotlin::kotlin_definition(),
        labelling_kotlin::kotlin_definition(),
        bundling_kotlin::kotlin_definition(),
        palleting_kotlin::kotlin_definition(),
        crating_kotlin::kotlin_definition(),
    ];
    let Some(printed) = printed(&NAMED_TWICE, definitions) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A map of tuples in a variant's one slot, whose writer names the tuple's wrapper twice, read
/// from the payload serde writes for it and written back as serde writes it back.
#[cfg(not(feature = "jsonschema"))]
#[test]
fn a_map_of_tuples_in_a_variant_reads_and_writes_in_kotlin_as_serde_does() {
    let rows = [Row {
        json: r#"{"kind":"Counts","held":{"north":[1,2]}}"#,
        name: "a map of tuples under a tag and a content key",
        read_as: "Tallying",
        written: written_back::<Tallying>,
    }];
    let expected = expected(&rows);
    assert!(
        expected.contains(r#"tag and a content key: {"kind":"Counts","held":{"north":[1,2]}}"#),
        "{expected}"
    );
    let Some(printed) = printed(&rows, vec![tallying_kotlin::kotlin_definition()]) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A `Result` holding a tuple in a variant's one slot, read from the payload serde writes for each
/// arm and written back as serde writes it back.
#[test]
fn a_result_holding_a_tuple_in_a_variant_reads_and_writes_in_kotlin_as_serde_does() {
    let rows = [
        Row {
            json: r#"{"kind":"Places","held":{"Ok":[1,2]}}"#,
            name: "a tuple in the Ok arm of a Result under a tag and a content key",
            read_as: "Ranking",
            written: written_back::<Ranking>,
        },
        Row {
            json: r#"{"kind":"Places","held":{"Err":"none"}}"#,
            name: "the Err arm of a Result holding a tuple under a tag and a content key",
            read_as: "Ranking",
            written: written_back::<Ranking>,
        },
    ];
    let expected = expected(&rows);
    assert!(
        expected.contains(r#"tag and a content key: {"kind":"Places","held":{"Ok":[1,2]}}"#),
        "{expected}"
    );
    let Some(printed) = printed(&rows, vec![ranking_kotlin::kotlin_definition()]) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A tuple holding a model `validate()` walks to: the wrapper reads the model unjudged, so a
/// payload that holds the model whole is written back as serde writes it back, and one that
/// breaks its bound is refused by the holder's own serializer at the holder's path.
#[test]
fn a_tuple_holding_a_model_is_refused_at_the_holders_path_in_kotlin() {
    let expected = expected(&HOLDING_A_MODEL);
    assert!(
        expected.contains(
            "a tuple holding a broken model beside a flattened member: refused: \
             'pair.0.code': too short: minimum length is 1, got 0"
        ),
        "{expected}"
    );
    let definitions = vec![
        stencil_kotlin::kotlin_definition(),
        labelling_kotlin::kotlin_definition(),
        wrapping_kotlin::kotlin_definition(),
        banding_kotlin::kotlin_definition(),
    ];
    let Some(printed) = printed(&HOLDING_A_MODEL, definitions) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A map of tuples holding a model in a variant's one slot, read and written back as serde does,
/// and refused at the variant's path where the model breaks its bound.
#[cfg(not(feature = "jsonschema"))]
#[test]
fn a_map_of_tuples_holding_a_model_in_a_variant_is_refused_at_the_variants_path_in_kotlin() {
    let rows = [
        Row {
            json: r#"{"kind":"Counts","held":{"north":[{"code":"north"},2]}}"#,
            name: "a map of tuples holding a model under a tag and a content key",
            read_as: "Binning",
            written: written_back::<Binning>,
        },
        Row {
            json: r#"{"kind":"Counts","held":{"north":[{"code":""},2]}}"#,
            name: "a map of tuples holding a broken model under a tag and a content key",
            read_as: "Binning",
            written: refused_at_the_count,
        },
    ];
    let expected = expected(&rows);
    assert!(
        expected
            .contains(r#"a content key: {"kind":"Counts","held":{"north":[{"code":"north"},2]}}"#),
        "{expected}"
    );
    let definitions = vec![
        stencil_kotlin::kotlin_definition(),
        binning_kotlin::kotlin_definition(),
    ];
    let Some(printed) = printed(&rows, definitions) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A tuple holding a model in a slot of a tuple struct, read and written back as serde does, and
/// refused at the slot's path where the model breaks its bound.
#[test]
fn a_tuple_holding_a_model_in_a_slot_of_a_tuple_struct_is_refused_at_the_slots_path_in_kotlin() {
    let rows = [
        Row {
            json: r#"[[{"code":"north"},2],3]"#,
            name: "a tuple holding a model in a slot of a tuple struct",
            read_as: "Slotting",
            written: written_back::<Slotting>,
        },
        Row {
            json: r#"[[{"code":""},2],3]"#,
            name: "a tuple holding a broken model in a slot of a tuple struct",
            read_as: "Slotting",
            written: refused_at_the_slot,
        },
    ];
    let expected = expected(&rows);
    assert!(
        expected.contains(
            "a broken model in a slot of a tuple struct: refused: '0.0.code': too short: minimum \
             length is 1, got 0"
        ),
        "{expected}"
    );
    let definitions = vec![
        stencil_kotlin::kotlin_definition(),
        slotting_kotlin::kotlin_definition(),
    ];
    let Some(printed) = printed(&rows, definitions) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A tuple holding a model in the `Ok` arm of a `Result` in a position of a tuple, read and
/// written back as serde does under each arm, and refused at the arm's path where the model breaks
/// its bound.
#[test]
fn a_tuple_holding_a_model_in_a_result_in_a_tuple_is_refused_at_the_arms_path_in_kotlin() {
    let rows = [
        Row {
            json: r#"{"pair":[{"Ok":[{"code":"north"},2]},3]}"#,
            name: "a tuple holding a model in the Ok arm of a Result in a tuple",
            read_as: "Grading",
            written: written_back::<Grading>,
        },
        Row {
            json: r#"{"pair":[{"Err":4},3]}"#,
            name: "the Err arm of a Result holding a tuple in a tuple",
            read_as: "Grading",
            written: written_back::<Grading>,
        },
        Row {
            json: r#"{"pair":[{"Ok":[{"code":""},2]},3]}"#,
            name: "a tuple holding a broken model in the Ok arm of a Result in a tuple",
            read_as: "Grading",
            written: refused_at_the_arm,
        },
    ];
    let expected = expected(&rows);
    assert!(
        expected.contains(
            "Ok arm of a Result in a tuple: refused: 'pair.0.Ok.0.code': too short: minimum \
             length is 1, got 0"
        ),
        "{expected}"
    );
    let definitions = vec![
        stencil_kotlin::kotlin_definition(),
        grading_kotlin::kotlin_definition(),
    ];
    let Some(printed) = printed(&rows, definitions) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// Tuples holding a model in an `Option` and in a list beside a flattened member, read and written
/// back as serde does, and each refused at its own path where its model breaks its bound.
#[test]
fn tuples_holding_a_model_in_an_option_and_a_list_are_refused_at_their_paths_in_kotlin() {
    let rows = [
        Row {
            json: r#"{"label":"north","maybe":[{"code":"north"},3],"pairs":[[{"code":"north"},2]]}"#,
            name: "tuples holding a model in an Option and a list beside a flattened member",
            read_as: "Spooling",
            written: written_back::<Spooling>,
        },
        Row {
            json: r#"{"label":"north","maybe":[{"code":""},3],"pairs":[[{"code":""},2]]}"#,
            name: "tuples holding a broken model in an Option and a list beside a flattened member",
            read_as: "Spooling",
            written: refused_in_the_option_and_the_list,
        },
    ];
    let expected = expected(&rows);
    assert!(
        expected.contains(
            "beside a flattened member: refused: 'maybe.0.code': too short: minimum length is 1, \
             got 0; 'pairs.0.0.code': too short: minimum length is 1, got 0"
        ),
        "{expected}"
    );
    let definitions = vec![
        stencil_kotlin::kotlin_definition(),
        labelling_kotlin::kotlin_definition(),
        spooling_kotlin::kotlin_definition(),
    ];
    let Some(printed) = printed(&rows, definitions) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A map of tuples holding a model in a position of a tuple, read and written back as serde does,
/// and refused at the entry's path where the model breaks its bound.
#[cfg(not(feature = "jsonschema"))]
#[test]
fn a_map_of_tuples_holding_a_model_in_a_tuple_is_refused_at_the_entrys_path_in_kotlin() {
    let rows = [
        Row {
            json: r#"{"pair":[{"north":[{"code":"north"},2]},3]}"#,
            name: "a map of tuples holding a model in a tuple",
            read_as: "Coiling",
            written: written_back::<Coiling>,
        },
        Row {
            json: r#"{"pair":[{"north":[{"code":""},2]},3]}"#,
            name: "a map of tuples holding a broken model in a tuple",
            read_as: "Coiling",
            written: refused_at_the_keyed_pair,
        },
    ];
    let expected = expected(&rows);
    assert!(
        expected.contains(r#"a model in a tuple: {"pair":[{"north":[{"code":"north"},2]},3]}"#),
        "{expected}"
    );
    let definitions = vec![
        stencil_kotlin::kotlin_definition(),
        coiling_kotlin::kotlin_definition(),
    ];
    let Some(printed) = printed(&rows, definitions) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A map of tuples holding a model under a key of its own beside a member of the holder's own, read
/// and written back as serde does, and refused at the key's path where the model breaks its bound.
#[test]
fn a_map_of_tuples_holding_a_model_under_its_own_key_is_refused_at_the_keys_path_in_kotlin() {
    let rows = [
        Row {
            json: r#"{"dock":"south","extra":{"north":[{"code":"north"},2]}}"#,
            name: "a map of tuples holding a model under its own key",
            read_as: "Lashing",
            written: written_back::<Lashing>,
        },
        Row {
            json: r#"{"dock":"south","extra":{"north":[{"code":""},2]}}"#,
            name: "a map of tuples holding a broken model under its own key",
            read_as: "Lashing",
            written: refused_at_the_own_key,
        },
    ];
    let expected = expected(&rows);
    assert!(
        expected.contains(
            r#"holding a model under its own key: {"dock":"south","extra":{"north":[{"code":"north"},2]}}"#
        ),
        "{expected}"
    );
    let definitions = vec![
        stencil_kotlin::kotlin_definition(),
        lashing_kotlin::kotlin_definition(),
    ];
    let Some(printed) = printed(&rows, definitions) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A map of tuples holding a model, read through a function a `preprocess` names, read and written
/// back as serde does, and refused at the entry's path where the model breaks its bound.
#[cfg(not(feature = "jsonschema"))]
#[test]
fn a_preprocessed_map_of_tuples_holding_a_model_is_refused_at_the_entrys_path_in_kotlin() {
    let rows = [
        Row {
            json: r#"{"counts":{"north":[{"code":"north"},2]}}"#,
            name: "a preprocessed map of tuples holding a model",
            read_as: "Heaping",
            written: written_back::<Heaping>,
        },
        Row {
            json: r#"{"counts":{"north":[{"code":""},2]}}"#,
            name: "a preprocessed map of tuples holding a broken model",
            read_as: "Heaping",
            written: refused_at_the_preprocessed_count,
        },
    ];
    let expected = expected(&rows);
    assert!(
        expected.contains(r#"holding a model: {"counts":{"north":[{"code":"north"},2]}}"#),
        "{expected}"
    );
    let definitions = vec![
        stencil_kotlin::kotlin_definition(),
        heaping_kotlin::kotlin_definition(),
        UNCHANGED.to_owned(),
    ];
    let Some(printed) = printed(&rows, definitions) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// The refusals the Kotlin rows above are held to are what the holder's `validate()` answers in
/// Rust, the read of a payload that leaves the key out included.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn the_refusals_held_here_are_what_the_holders_validate_answers() {
    let broken = || (Stencil::default(), 2);
    assert_eq!(
        Wrapping { pair: broken() }.validate(),
        Err(vec![PAIR_REFUSED.to_owned()])
    );
    assert_eq!(
        Banding {
            labelling: Labelling {
                label: "north".to_owned()
            },
            pair: broken()
        }
        .validate(),
        Err(vec![PAIR_REFUSED.to_owned()])
    );
    assert_eq!(
        serde_json::from_str::<Wrapping>("{}")
            .unwrap_err()
            .to_string(),
        PAIR_REFUSED
    );
    #[cfg(not(feature = "jsonschema"))]
    assert_eq!(
        Binning::Counts(HashMap::from([("north".to_owned(), broken())])).validate(),
        Err(vec![COUNT_REFUSED.to_owned()])
    );
    assert_eq!(
        Slotting(broken(), 3).validate(),
        Err(vec![SLOT_REFUSED.to_owned()])
    );
    assert_eq!(
        Grading {
            pair: (Ok(broken()), 3)
        }
        .validate(),
        Err(vec![ARM_REFUSED.to_owned()])
    );
    assert_eq!(
        Spooling {
            labelling: Labelling {
                label: "north".to_owned()
            },
            maybe: Some(broken()),
            pairs: vec![broken()]
        }
        .validate(),
        Err(vec![OPTION_REFUSED.to_owned(), LIST_REFUSED.to_owned()])
    );
    #[cfg(not(feature = "jsonschema"))]
    assert_eq!(
        Coiling {
            pair: (HashMap::from([("north".to_owned(), broken())]), 3)
        }
        .validate(),
        Err(vec![KEYED_PAIR_REFUSED.to_owned()])
    );
    assert_eq!(
        Lashing {
            dock: "south".to_owned(),
            extra: HashMap::from([("north".to_owned(), broken())])
        }
        .validate(),
        Err(vec![OWN_KEY_REFUSED.to_owned()])
    );
    #[cfg(not(feature = "jsonschema"))]
    assert_eq!(
        Heaping {
            counts: HashMap::from([("north".to_owned(), broken())])
        }
        .validate(),
        Err(vec![PREPROCESSED_REFUSED.to_owned()])
    );
}
