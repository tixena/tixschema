//! An enum with a variant of several slots, read and written by the emitted Kotlin types compiled
//! with `-Werror` under each tagging serde accepts for one, and the untagged enum that writes the
//! same slots from a tuple: each payload is written back as the serde read writes it back. So is
//! a tuple in each place a writer names its wrapper more than once, and a tuple holding a model
//! `validate()` walks to, which a payload that breaks the model's bound is refused for at the
//! holder's path.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;
#[cfg(all(
    not(feature = "jsonschema"),
    any(feature = "typescript", feature = "zod")
))]
use std::collections::HashMap;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::runtime::ran_kotlin;
use super::tuple_variants_models::{
    Banding, Bundling, Crating, Palleting, Piling, Racking, Ranking, Stacking, Strapping, Wrapping,
    banding_kotlin, bundling_kotlin, crating_kotlin, labelling_kotlin, palleting_kotlin,
    piling_kotlin, racking_kotlin, ranking_kotlin, stacking_kotlin, stencil_kotlin,
    strapping_kotlin, wrapping_kotlin,
};
#[cfg(not(feature = "jsonschema"))]
use super::tuple_variants_models::{Binning, Tallying, binning_kotlin, tallying_kotlin};
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
}
