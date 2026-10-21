//! An enum with a variant of several slots, read and written by the emitted Kotlin types compiled
//! with `-Werror` under each tagging serde accepts for one, and the untagged enum that writes the
//! same slots from a tuple: each payload is written back as the serde read writes it back. So is
//! a tuple in each place a writer names its wrapper more than once.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::runtime::ran_kotlin;
use super::tuple_variants_models::{
    Bundling, Crating, Palleting, Piling, Racking, Stacking, Strapping, bundling_kotlin,
    crating_kotlin, labelling_kotlin, palleting_kotlin, piling_kotlin, racking_kotlin,
    stacking_kotlin, strapping_kotlin,
};
#[cfg(not(feature = "jsonschema"))]
use super::tuple_variants_models::{Tallying, tallying_kotlin};

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

/// What prints a row: the value read, written back with every member, as serde writes it.
const TOLD: &str = r#"
val everyMember = Json { encodeDefaults = true }

fun <T> row(name: String, serializer: KSerializer<T>, payload: String) {
    val read = Json.decodeFromString(serializer, payload)
    println("$name: ${everyMember.encodeToString(serializer, read)}")
}
"#;

/// A payload, the type it is read as, and what the serde read of it writes back.
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
