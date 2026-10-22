//! A payload serde or Zod refuses, read by a serializer the Kotlin emitter writes, compiled with
//! `-Werror`: each is refused with `SerializationException`, the exception the plugin's own
//! serializers and the type's judged one throw, so a caller that catches it catches each.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::fixed_arrays_models::{
    FIXED, becketing_kotlin, bitter_kotlin, bunt_kotlin, buntline_kotlin, catting_kotlin,
    cringling_kotlin, earring_kotlin, fishing_kotlin, garland_kotlin, reefband_kotlin,
    sheaving_kotlin, shroud_kotlin, shrouding_kotlin,
};
use super::flattened_models::{
    Bowlining, Stowing, Swaying, Tacking, Whelping, bowlining_kotlin, bunting_kotlin,
    stowing_kotlin, swaying_kotlin, tacking_kotlin, whelping_kotlin,
};
#[cfg(feature = "dart")]
use super::run_dart_refusals::{
    Clasping, Clenching, Grasping, clasping_kotlin, clenching_kotlin, grasping_kotlin,
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

/// What prints a fixed array's row: the payload read by the type's own serializer, then by its
/// unjudged one, each written back as serde writes it, or the full name of the class of what the
/// read throws and its message.
const FIXED_TOLD: &str = r#"
fun <T> fixed(at: String, judged: KSerializer<T>, unjudged: KSerializer<T>, payload: String) {
    val told = listOf(judged, unjudged).joinToString(" | ") { serializer ->
        try {
            Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))
        } catch (refusal: Exception) {
            "${refusal::class.qualifiedName}: ${refusal.message}"
        }
    }
    println("$at: $told")
}
"#;

/// [`Clenching`] and [`Clasping`], a fixed array at one level and at two, read from an array of
/// another length at each level and of the length written, and [`Grasping`], a list, read at any
/// length: each payload and what the Kotlin read throws, or `None` where it reads it.
#[cfg(feature = "dart")]
const CLENCHED: [(&str, &str, Option<&str>); 9] = [
    (
        "Clenching",
        r#"{"pair":[1,2,3]}"#,
        Some("invalid length 3, expected 2"),
    ),
    (
        "Clenching",
        r#"{"pair":[1]}"#,
        Some("invalid length 1, expected 2"),
    ),
    (
        "Clasping",
        r#"{"grid":[[1,2],[3,4,5],[6,7]]}"#,
        Some("invalid length 3, expected 2"),
    ),
    (
        "Clasping",
        r#"{"grid":[[1,2],[3,4]]}"#,
        Some("invalid length 2, expected 3"),
    ),
    ("Clenching", r#"{"pair":[1,2]}"#, None),
    ("Clasping", r#"{"grid":[[1,2],[3,4],[5,6]]}"#, None),
    ("Grasping", r#"{"held":[1,2,3]}"#, None),
    ("Grasping", r#"{"held":[]}"#, None),
    ("Grasping", r#"{"held":[1]}"#, None),
];

/// [`Becalming`]'s unit variant with no content, with `null` for one and with a value for one:
/// each payload, what serde answers of it, and what the Kotlin read answers of it.
const UNIT_ROWS: [(&str, &str, &str); 3] = [
    (r#"{"kind":"Lull"}"#, "read", "read"),
    (
        r#"{"kind":"Lull","held":null}"#,
        "read",
        "kotlinx.serialization.SerializationException: Becalming: Lull takes no value",
    ),
    (
        r#"{"kind":"Lull","held":5}"#,
        "invalid type: integer `5`, expected unit variant Becalming::Lull at line 1 column 23",
        "kotlinx.serialization.SerializationException: Becalming: Lull takes no value",
    ),
];

/// A unit variant under a tag and a content key, beside a variant that holds a value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Becalming {
    Breeze(bool),
    Lull,
}

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

/// A unit variant under a tag and a content key is read where the object holds no content key, and
/// an object that holds one is refused, `null` among its values, as Zod refuses it: its schema of
/// the variant is a strict object of the tag alone. serde reads the variant over `null`.
#[test]
fn a_unit_variant_under_a_content_key_is_read_where_the_object_holds_no_content() {
    let serde_told: Vec<String> = UNIT_ROWS
        .iter()
        .map(|&(json, _, _)| serde_told::<Becalming>(json))
        .collect();
    let serde_pinned: Vec<&str> = UNIT_ROWS.iter().map(|&(_, serde, _)| serde).collect();
    assert_eq!(
        serde_told, serde_pinned,
        "serde no longer answers each payload as it did"
    );
    let zod = Becalming::zod_schema();
    assert!(
        zod.contains("z.strictObject({\n  kind: z.literal(\"Lull\"),\n})"),
        "Zod no longer reads the unit variant as an object of its tag alone: {zod}"
    );
    let mut expected = String::new();
    let mut main = String::from(THROWN);
    main.push_str("fun main() {\n");
    for (at, &(json, _, kotlin)) in UNIT_ROWS.iter().enumerate() {
        let _ = writeln!(expected, "{at}: {kotlin}");
        let _ = writeln!(
            main,
            "    thrown(\"{at}\", Becalming.serializer(), \"\"\"{json}\"\"\")"
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        becalming_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not the answer above");
}

/// The Kotlin rows of the fixed arrays `rows` names, read by the type's own serializer and by its
/// unjudged one, each held to `thrown` where the read refuses the payload and to what serde writes
/// back of it where it reads it, and the source that prints them.
fn fixed_rows<'rows>(
    rows: impl Iterator<Item = (&'rows str, &'rows str, Option<&'rows str>, String)>,
) -> (String, String) {
    let mut expected = String::new();
    let mut main = String::from(FIXED_TOLD);
    main.push_str("fun main() {\n");
    for (at, (read_as, json, thrown, serde)) in rows.enumerate() {
        let told = thrown.map_or_else(
            || serde.clone(),
            |message| format!("kotlinx.serialization.SerializationException: {message}"),
        );
        let _ = writeln!(expected, "{at}: {told} | {told}");
        let _ = writeln!(
            main,
            "    fixed(\"{at}\", {read_as}.serializer(), {read_as}Unjudged, \"\"\"{json}\"\"\")"
        );
    }
    main.push_str("}\n");
    (expected, main)
}

/// A fixed array of another length is refused, beside a flattened member, as an item of a list,
/// named by an alias, in a tuple's position, in a variant's slot, as a map's value, under an
/// `Option`, filling a type parameter and of a type parameter's filling, with the
/// `SerializationException` the read of a tuple throws for an array of another length, by the
/// type's own serializer and by its unjudged one, as serde refuses it. One of the length written
/// is read and written back as serde writes it, and a list is read at any length.
#[test]
fn a_fixed_array_of_another_length_is_refused_wherever_it_is_held() {
    for row in &FIXED {
        let serde = (row.serde)(row.json);
        assert_eq!(
            serde == "refused",
            row.thrown.is_some(),
            "serde and the Kotlin read part on `{}` as {}",
            row.json,
            row.read_as
        );
    }
    let (expected, main) = fixed_rows(
        FIXED
            .iter()
            .map(|row| (row.read_as, row.json, row.thrown, (row.serde)(row.json))),
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        bunt_kotlin::kotlin_definition(),
        reefband_kotlin::kotlin_definition(),
        earring_kotlin::kotlin_definition(),
        bitter_kotlin::kotlin_definition(),
        becketing_kotlin::kotlin_definition(),
        buntline_kotlin::kotlin_definition(),
        shroud_kotlin::kotlin_definition(),
        shrouding_kotlin::kotlin_definition(),
        fishing_kotlin::kotlin_definition(),
        catting_kotlin::kotlin_definition(),
        sheaving_kotlin::kotlin_definition(),
        cringling_kotlin::kotlin_definition(),
        garland_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not the read above");
}

/// What serde writes back of `json` read as `T`, or its refusal.
#[cfg(feature = "dart")]
fn serde_written<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json).map_or_else(
        |refusal| refusal.to_string(),
        |read| serde_json::to_string(&read).unwrap(),
    )
}

/// The fixed arrays of the Dart run, [`Clenching`] and [`Clasping`], are refused at each level
/// where the array is of another length, as serde refuses them, and read where it is of the length
/// written, as [`Grasping`]'s list is read at any length, each written back as serde writes it.
#[cfg(feature = "dart")]
#[test]
fn a_fixed_array_is_read_from_an_array_of_its_length_alone_at_each_level() {
    let told = |&(read_as, json, _): &(&str, &str, Option<&str>)| match read_as {
        "Clenching" => serde_written::<Clenching>(json),
        "Clasping" => serde_written::<Clasping>(json),
        _ => serde_written::<Grasping>(json),
    };
    let serde_told: Vec<String> = CLENCHED.iter().map(told).collect();
    assert_eq!(
        serde_told,
        [
            "trailing characters at line 1 column 14",
            "invalid length 1, expected an array of length 2 at line 1 column 11",
            "trailing characters at line 1 column 21",
            "invalid length 2, expected an array of length 3 at line 1 column 21",
            r#"{"pair":[1,2]}"#,
            r#"{"grid":[[1,2],[3,4],[5,6]]}"#,
            r#"{"held":[1,2,3]}"#,
            r#"{"held":[]}"#,
            r#"{"held":[1]}"#,
        ],
        "serde no longer answers each payload as it did"
    );
    let (expected, main) = fixed_rows(
        CLENCHED
            .iter()
            .zip(serde_told)
            .map(|(&(read_as, json, thrown), serde)| (read_as, json, thrown, serde)),
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        clenching_kotlin::kotlin_definition(),
        clasping_kotlin::kotlin_definition(),
        grasping_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not the read above");
}

/// The README's sentences on the refusals of the Kotlin read show payloads and messages the runs
/// above measure.
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
    for &(json, _, kotlin) in &UNIT_ROWS {
        let shown = format!("`{json}`");
        assert!(readme.contains(&shown), "not in the README: {shown}");
        let refusal = kotlin.trim_start_matches("kotlinx.serialization.");
        assert!(
            kotlin == "read" || readme.contains(&format!("`{refusal}`")),
            "not in the README: {refusal}"
        );
    }
}

/// The README's sentence on a fixed array declares the struct the run reads, and shows payloads
/// and messages the run measures.
#[cfg(feature = "dart")]
#[test]
fn the_readme_shows_the_fixed_array_refusals_the_run_measures() {
    let readme = include_str!("../../README.md");
    assert!(
        readme
            .contains("with `struct Clenching { pair: [u8; 2] }`, `{\"pair\":[1,2,3]}` is refused"),
        "the README no longer declares the struct the run reads"
    );
    for &(read_as, json, thrown) in &CLENCHED[..2] {
        let message = thrown.unwrap();
        let shown = if json.ends_with("3]}") {
            format!("`{json}` is refused with `SerializationException: {message}`")
        } else {
            format!("`{json}` with `{message}`")
        };
        assert_eq!(read_as, "Clenching", "the README shows a row of Clenching");
        assert!(readme.contains(&shown), "not in the README: {shown}");
    }
}
