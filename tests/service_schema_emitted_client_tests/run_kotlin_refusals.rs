//! A payload serde refuses, read by a serializer the Kotlin emitter writes, compiled with
//! `-Werror`: each is refused with `SerializationException`, the exception the plugin's own
//! serializers and the type's judged one throw, so a caller that catches it catches each.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use serde::de::DeserializeOwned;

use super::flattened_models::{
    Bowlining, Stowing, Swaying, Tacking, Whelping, bowlining_kotlin, bunting_kotlin,
    stowing_kotlin, swaying_kotlin, tacking_kotlin, whelping_kotlin,
};
use super::runtime::ran_kotlin;
use super::tuple_variants_models::{Palleting, labelling_kotlin, palleting_kotlin};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// The function a variant of `Bowlining` names, and what prints a row: `read` where the payload is
/// read, and otherwise the full name of the class of what the read throws, and its message.
const THROWN: &str = r#"
fun withItsKeys(payload: JsonElement): JsonElement =
    if (payload is JsonObject) {
        JsonObject(mapOf("keys" to JsonArray(payload.keys.sorted().map { JsonPrimitive(it) })))
    } else {
        payload
    }

fun <T> thrown(at: String, serializer: KSerializer<T>, payload: String) {
    val told = try {
        Json.decodeFromString(serializer, payload)
        "read"
    } catch (refusal: Exception) {
        "${refusal::class.qualifiedName}: ${refusal.message}"
    }
    println("$at: $told")
}
"#;

/// Every payload: a variant under a tag and a content key with no content, with `null` for one,
/// with no tag, under a tag no variant has and under a tag that is no string; an externally tagged
/// enum read from an object of no key, of two, from an array and under a name no variant has; a
/// struct with a flattened member read from an array and from an object without its own member;
/// a tuple struct whose slot is a tuple, read from arrays of another length, from an object, and
/// with its slot's tuple of another length; an enum under a tag alone read from an array; and
/// an untagged enum no variant of which reads the payload.
const ROWS: [Row; 18] = [
    Row {
        json: r#"{"kind":"Counted"}"#,
        read_as: "Bowlining",
        serde: serde_told::<Bowlining>,
        thrown: "missing field `held`",
    },
    Row {
        json: r#"{"kind":"Counted","held":null}"#,
        read_as: "Bowlining",
        serde: serde_told::<Bowlining>,
        thrown: "expected an object, found null",
    },
    Row {
        json: r#"{"held":{}}"#,
        read_as: "Bowlining",
        serde: serde_told::<Bowlining>,
        thrown: "missing field `kind`",
    },
    Row {
        json: r#"{"kind":"Gale","held":{}}"#,
        read_as: "Bowlining",
        serde: serde_told::<Bowlining>,
        thrown: "unknown tag Gale",
    },
    Row {
        json: r#"{"kind":[],"held":{}}"#,
        read_as: "Bowlining",
        serde: serde_told::<Bowlining>,
        thrown: "expected a string, found []",
    },
    Row {
        json: "{}",
        read_as: "Tacking",
        serde: serde_told::<Tacking>,
        thrown: "expected an object of one key, found {}",
    },
    Row {
        json: r#"{"Bay":{"label":"n"},"Dock":{"label":"s"}}"#,
        read_as: "Tacking",
        serde: serde_told::<Tacking>,
        thrown: concat!(
            "expected an object of one key, found ",
            r#"{"Bay":{"label":"n"},"Dock":{"label":"s"}}"#
        ),
    },
    Row {
        json: "[1]",
        read_as: "Tacking",
        serde: serde_told::<Tacking>,
        thrown: "expected an object, found [1]",
    },
    Row {
        json: r#"{"Gale":{}}"#,
        read_as: "Tacking",
        serde: serde_told::<Tacking>,
        thrown: "unknown tag Gale",
    },
    Row {
        json: "[]",
        read_as: "Stowing",
        serde: serde_told::<Stowing>,
        thrown: "expected an object, found []",
    },
    Row {
        json: "{}",
        read_as: "Stowing",
        serde: serde_told::<Stowing>,
        thrown: "missing field `note`",
    },
    Row {
        json: "[1]",
        read_as: "Palleting",
        serde: serde_told::<Palleting>,
        thrown: "invalid length 1, expected 2",
    },
    Row {
        json: "[[1,2]]",
        read_as: "Palleting",
        serde: serde_told::<Palleting>,
        thrown: "invalid length 1, expected 2",
    },
    Row {
        json: "{}",
        read_as: "Palleting",
        serde: serde_told::<Palleting>,
        thrown: "expected an array, found {}",
    },
    Row {
        json: "[[1],2]",
        read_as: "Palleting",
        serde: serde_told::<Palleting>,
        thrown: "invalid length 1, expected 2",
    },
    Row {
        json: "[[1,2],3,4]",
        read_as: "Palleting",
        serde: serde_told::<Palleting>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        json: "[]",
        read_as: "Whelping",
        serde: serde_told::<Whelping>,
        thrown: "expected an object, found []",
    },
    Row {
        json: "[1]",
        read_as: "Swaying",
        serde: serde_told::<Swaying>,
        thrown: "No variant of Swaying matched",
    },
];

/// One payload, the type it is read as and what serde answers of it.
struct Row {
    json: &'static str,
    /// The Kotlin type the payload is read as, whose serializer is the type's own.
    read_as: &'static str,
    /// What serde answers of the payload: its refusal, or `read`.
    serde: fn(&str) -> String,
    /// The message of the `SerializationException` the Kotlin read throws.
    thrown: &'static str,
}

/// serde's refusal of `json` as `T`, or `read` where it reads it.
fn serde_told<T>(json: &str) -> String
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json)
        .map_or_else(|refusal| refusal.to_string(), |_| "read".to_owned())
}

/// Each payload serde refuses is refused by the Kotlin read with `SerializationException` and the
/// message the serializer tixschema writes throws it with.
#[test]
fn a_serializer_tixschema_writes_refuses_with_a_serialization_exception() {
    let serde_refused: Vec<String> = ROWS.iter().map(|row| (row.serde)(row.json)).collect();
    assert_eq!(
        serde_refused,
        [
            "missing field `held` at line 1 column 18",
            "invalid type: null, expected struct variant Bowlining::Counted at line 1 column 29",
            "missing field `kind` at line 1 column 11",
            "unknown variant `Gale`, expected one of `Counted`, `Labelled`, `Spare`, `Tallied` at \
             line 1 column 14",
            "expected value at line 1 column 9",
            "expected value at line 1 column 2",
            "expected value at line 1 column 20",
            "expected value at line 1 column 1",
            "unknown variant `Gale`, expected one of `Bay`, `Calm`, `Dock`, `Quay`, `Held` at line \
             1 column 7",
            "invalid type: sequence, expected struct Stowing at line 1 column 0",
            "missing field `note` at line 1 column 2",
            "invalid type: integer `1`, expected a tuple of size 2 at line 1 column 2",
            "invalid length 1, expected tuple struct Palleting with 2 elements at line 1 column 7",
            "invalid type: map, expected tuple struct Palleting at line 1 column 0",
            "invalid length 1, expected a tuple of size 2 at line 1 column 4",
            "trailing characters at line 1 column 10",
            "missing field `kind` at line 1 column 2",
            "data did not match any variant of untagged enum Swaying",
        ],
        "serde no longer refuses each payload as it did"
    );
    let mut expected = String::new();
    let mut main = String::from(THROWN);
    main.push_str("fun main() {\n");
    for (at, row) in ROWS.iter().enumerate() {
        let _ = writeln!(
            expected,
            "{at}: kotlinx.serialization.SerializationException: {}",
            row.thrown
        );
        let _ = writeln!(
            main,
            "    thrown(\"{at}\", {}.serializer(), \"\"\"{}\"\"\")",
            row.read_as, row.json
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        labelling_kotlin::kotlin_definition(),
        bunting_kotlin::kotlin_definition(),
        bowlining_kotlin::kotlin_definition(),
        tacking_kotlin::kotlin_definition(),
        stowing_kotlin::kotlin_definition(),
        palleting_kotlin::kotlin_definition(),
        whelping_kotlin::kotlin_definition(),
        swaying_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(
        printed, expected,
        "the Kotlin read is not the refusal above"
    );
}

/// The README's sentence on the refusals of the Kotlin read shows messages the run above measures.
#[test]
fn the_readme_shows_refusals_the_run_measures() {
    let readme = include_str!("../../README.md");
    for shown in [
        "``missing field `held` ``",
        "`expected an object, found [1]`",
        "`expected an object of one key, found {}`",
        "`unknown tag Gale`",
        "`invalid length 3, expected 2`",
    ] {
        let message = shown.trim_matches('`').trim();
        assert!(
            ROWS.iter().any(|row| row.thrown == message),
            "no row throws `{message}`"
        );
        assert!(readme.contains(shown), "not in the README: {shown}");
    }
}
