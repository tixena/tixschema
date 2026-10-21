//! A struct whose members are all flattened, one of them a map, read and written by the emitted
//! Kotlin types compiled with `-Werror`, beside its twin with a member of its own, and an optional
//! member beside a flattened one, beside its twin under `nullable`: each payload is written back
//! as the serde read writes it back. A map flattened ahead of a flattened struct, and two
//! flattened maps, each member read as serde reads it.

#![cfg(feature = "kotlin")]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::flattened_models::{
    Hitching, Knotting, Netting, Seizing, Splicing, Whipping, hitching_kotlin, knotting_kotlin,
    netting_kotlin, seizing_kotlin, splicing_kotlin, whipping_kotlin,
};
use super::runtime::ran_kotlin;
use super::tuple_variants_models::labelling_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read, written back as serde writes it.
const TOLD: &str = r#"
fun <T> row(name: String, serializer: KSerializer<T>, payload: String) {
    val read = Json.decodeFromString(serializer, payload)
    println("$name: ${Json.encodeToString(serializer, read)}")
}
"#;

/// Each struct's payload, its keys in an order serde does not write them in for the twin.
const ROWS: [Row; 2] = [
    Row {
        json: r#"{"label":"n","north":2}"#,
        name: "every member flattened, one of them a map",
        read_as: "Netting",
        written: written_back::<Netting>,
    },
    Row {
        json: r#"{"note":"a","label":"n","north":2}"#,
        name: "a member of its own beside the flattened ones",
        read_as: "Knotting",
        written: written_back::<Knotting>,
    },
];

/// An optional member beside a flattened one, absent and present, and its twin under `nullable`
/// with the key absent.
const OPTIONAL_ROWS: [Row; 3] = [
    Row {
        json: r#"{"label":"north"}"#,
        name: "an absent optional member beside a flattened one",
        read_as: "Splicing",
        written: written_back::<Splicing>,
    },
    Row {
        json: r#"{"label":"north","spare":3}"#,
        name: "a present optional member beside a flattened one",
        read_as: "Splicing",
        written: written_back::<Splicing>,
    },
    Row {
        json: r#"{"label":"north"}"#,
        name: "an absent nullable member beside a flattened one",
        read_as: "Seizing",
        written: written_back::<Seizing>,
    },
];

/// The payload of a map of text flattened ahead of a flattened struct.
const HITCHING: &str = r#"{"label":"n","north":"2"}"#;

/// What prints what [`HITCHING`] reads as: each member, and the object written back.
const HITCHING_MAIN: &str = r#"
fun main() {
    val read = Json.decodeFromString(Hitching.serializer(), """PAYLOAD""")
    println("extra: " + Json.encodeToString(MapSerializer(String.serializer(), String.serializer()), read.extra))
    println("labelling: " + Json.encodeToString(Labelling.serializer(), read.labelling))
    println("written: " + Json.encodeToString(Hitching.serializer(), read))
}
"#;

/// The payload of two flattened maps.
const WHIPPING: &str = r#"{"north":2,"south":3}"#;

/// What prints what [`WHIPPING`] reads as, and what a value whose two maps hold one key apart is
/// written as and read back as.
const WHIPPING_MAIN: &str = r#"
val counts = MapSerializer(String.serializer(), UByte.serializer())

fun main() {
    val read = Json.decodeFromString(Whipping.serializer(), """PAYLOAD""")
    println("first: " + Json.encodeToString(counts, read.first))
    println("second: " + Json.encodeToString(counts, read.second))
    println("written: " + Json.encodeToString(Whipping.serializer(), read))
    val apart = Whipping(first = mapOf("north" to 1.toUByte()), second = mapOf("north" to 2.toUByte(), "south" to 3.toUByte()))
    val written = Json.encodeToString(Whipping.serializer(), apart)
    println("apart written: " + written)
    val back = Json.decodeFromString(Whipping.serializer(), written)
    println("apart read back: " + Json.encodeToString(counts, back.first) + " " + Json.encodeToString(counts, back.second))
}
"#;

/// A payload, the type it is read as, and what serde writes back of it.
struct Row {
    json: &'static str,
    name: &'static str,
    read_as: &'static str,
    written: fn(&str) -> String,
}

/// What serde writes back of `json`, read as `T`.
fn written_back<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    let read: T = serde_json::from_str(json).unwrap();
    serde_json::to_string(&read).unwrap()
}

/// The object `text` reads as, each key once with its last value: serde writes a key two members
/// hold twice, and a Kotlin `JsonObject` holds it once.
fn each_key_once(text: &str) -> String {
    serde_json::from_str::<serde_json::Value>(text)
        .unwrap()
        .to_string()
}

/// What serde writes back of each row, one line each, and the `main` that prints the same of each
/// in Kotlin.
fn expected_and_main(rows: &[Row]) -> (String, String) {
    let mut expected = String::new();
    let mut main = String::from("fun main() {\n");
    for row in rows {
        let _ = writeln!(expected, "{}: {}", row.name, (row.written)(row.json));
        let _ = writeln!(
            main,
            "    row(\"{}\", {}.serializer(), \"\"\"{}\"\"\")",
            row.name, row.read_as, row.json
        );
    }
    main.push_str("}\n");
    (expected, main)
}

/// Each struct read from its payload and written back as serde writes it back.
#[test]
fn a_struct_of_flattened_members_reads_and_writes_in_kotlin_as_serde_does() {
    let (expected, main) = expected_and_main(&ROWS);
    assert!(
        expected.contains(r#"every member flattened, one of them a map: {"label":"n","north":2}"#),
        "{expected}"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        labelling_kotlin::kotlin_definition(),
        netting_kotlin::kotlin_definition(),
        knotting_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// An optional member beside a flattened one is left out where it holds no value, as serde leaves
/// it out, and one under `nullable` is written as `null`.
#[test]
fn an_optional_member_beside_a_flattened_one_is_written_in_kotlin_as_serde_writes_it() {
    let (expected, main) = expected_and_main(&OPTIONAL_ROWS);
    assert_eq!(
        expected,
        "an absent optional member beside a flattened one: {\"label\":\"north\"}\n\
         a present optional member beside a flattened one: {\"label\":\"north\",\"spare\":3}\n\
         an absent nullable member beside a flattened one: {\"label\":\"north\",\"spare\":null}\n"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        labelling_kotlin::kotlin_definition(),
        splicing_kotlin::kotlin_definition(),
        seizing_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A map flattened ahead of a flattened struct is handed every key, the struct's among them, as
/// serde hands it. serde writes the struct's key twice, and refuses the text it wrote: Kotlin
/// writes the key once, the payload serde read.
#[test]
fn a_map_flattened_ahead_of_a_struct_reads_in_kotlin_the_keys_serde_hands_it() {
    let read: Hitching = serde_json::from_str(HITCHING).unwrap();
    let written = serde_json::to_string(&read).unwrap();
    assert_eq!(written, r#"{"label":"n","north":"2","label":"n"}"#);
    assert!(
        serde_json::from_str::<Hitching>(&written)
            .unwrap_err()
            .to_string()
            .starts_with("duplicate field `label`"),
        "{written}"
    );
    let expected = format!(
        "extra: {}\nlabelling: {}\nwritten: {}\n",
        serde_json::to_string(&read.extra).unwrap(),
        serde_json::to_string(&read.labelling).unwrap(),
        each_key_once(&written),
    );
    assert_eq!(
        expected,
        "extra: {\"label\":\"n\",\"north\":\"2\"}\nlabelling: {\"label\":\"n\"}\n\
         written: {\"label\":\"n\",\"north\":\"2\"}\n"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        labelling_kotlin::kotlin_definition(),
        hitching_kotlin::kotlin_definition(),
        HITCHING_MAIN.replace("PAYLOAD", HITCHING),
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// Two flattened maps are each handed the same keys, as serde hands them, and written in the
/// order declared. serde writes a key both hold twice, and reads that text back with the last
/// value in both maps: Kotlin writes the key once, with that value.
#[test]
fn two_flattened_maps_read_and_write_in_kotlin_as_serde_does() {
    let read: Whipping = serde_json::from_str(WHIPPING).unwrap();
    let written = serde_json::to_string(&read).unwrap();
    assert_eq!(written, r#"{"north":2,"south":3,"north":2,"south":3}"#);
    let apart = Whipping {
        first: BTreeMap::from([("north".to_owned(), 1)]),
        second: BTreeMap::from([("north".to_owned(), 2), ("south".to_owned(), 3)]),
    };
    let apart_written = serde_json::to_string(&apart).unwrap();
    assert_eq!(apart_written, r#"{"north":1,"north":2,"south":3}"#);
    let back: Whipping = serde_json::from_str(&apart_written).unwrap();
    let expected = format!(
        "first: {}\nsecond: {}\nwritten: {}\napart written: {}\napart read back: {} {}\n",
        serde_json::to_string(&read.first).unwrap(),
        serde_json::to_string(&read.second).unwrap(),
        each_key_once(&written),
        each_key_once(&apart_written),
        serde_json::to_string(&back.first).unwrap(),
        serde_json::to_string(&back.second).unwrap(),
    );
    assert_eq!(
        expected,
        "first: {\"north\":2,\"south\":3}\nsecond: {\"north\":2,\"south\":3}\n\
         written: {\"north\":2,\"south\":3}\napart written: {\"north\":2,\"south\":3}\n\
         apart read back: {\"north\":2,\"south\":3} {\"north\":2,\"south\":3}\n"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        whipping_kotlin::kotlin_definition(),
        WHIPPING_MAIN.replace("PAYLOAD", WHIPPING),
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected);
}
