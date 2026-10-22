//! A map flattened as the one member of its struct, and a flattened struct beside a member of its
//! own, read and written by the emitted Kotlin types compiled with `-Werror`, and an optional
//! member beside a flattened one, beside its twin under `nullable`: each payload is written back
//! as the serde read writes it back. A member flattened after a struct, or beside a member of its
//! holder's own, is handed the keys serde hands it.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::flattened_models::{
    Bending, Brailing, HITCHING, Hitching, Knotting, Lanyarding, Netting, Nipping, PAYLOAD,
    STROPPING, Seizing, Slinging, Splicing, Stoppering, Stropping, Whipping, bending_kotlin,
    brailing_kotlin, bunting_kotlin, hitching_kotlin, knotting_kotlin, lanyarding_kotlin,
    netting_kotlin, nipping_kotlin, noting_kotlin, read_of, seizing_kotlin, sheeting_kotlin,
    slinging_kotlin, splicing_kotlin, stoppering_kotlin, stropping_kotlin, ticketed_kotlin,
    whipping_kotlin,
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

/// Each struct's payload, the second's keys in an order serde does not write them in.
const ROWS: [Row; 2] = [
    Row {
        json: r#"{"north":2}"#,
        name: "a map, the one member",
        read_as: "Netting",
        written: written_back::<Netting>,
    },
    Row {
        json: r#"{"note":"a","label":"n"}"#,
        name: "a member of its own beside a flattened struct",
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

/// The functions the models name: one answers the payload as it was handed, the other the keys of
/// the object it is handed.
const FUNCTIONS: &str = r#"
fun asGiven(payload: JsonElement): JsonElement = payload

fun withItsKeys(payload: JsonElement): JsonElement =
    if (payload is JsonObject) {
        JsonObject(mapOf("keys" to JsonArray(payload.keys.sorted().map { JsonPrimitive(it) })))
    } else {
        payload
    }
"#;

/// What prints a row of a holder of a member flattened after or beside another: the value read,
/// as `shown` tells it, or that the read is refused.
const HANDED: &str = r#"
fun <T> handed(name: String, serializer: KSerializer<T>, payload: String, shown: (T) -> String) {
    val told = try {
        shown(Json.decodeFromString(serializer, payload))
    } catch (refused: SerializationException) {
        "refused"
    }
    println("$name: $told")
}

fun main() {
    handed("after a struct", Whipping.serializer(), """PAYLOAD""") {
        Json.encodeToString(Whipping.serializer(), it)
    }
    handed("after a struct and beside a member of its own", Hitching.serializer(), """HITCHING""") {
        Json.encodeToString(Hitching.serializer(), it)
    }
    handed("ahead of a struct", Bending.serializer(), """PAYLOAD""") {
        Json.encodeToString(Bending.serializer(), it)
    }
    handed("after a struct that names a function", Lanyarding.serializer(), """PAYLOAD""") {
        Json.encodeToString(Lanyarding.serializer(), it)
    }
    handed("after a struct with a flattened member", Brailing.serializer(), """PAYLOAD""") {
        Json.encodeToString(Brailing.serializer(), it)
    }
    handed("a struct after one of other keys", Stropping.serializer(), """STROPPING""") {
        Json.encodeToString(Stropping.serializer(), it)
    }
    handed("a struct after one of the same key", Stoppering.serializer(), """PAYLOAD""") {
        Json.encodeToString(Stoppering.serializer(), it)
    }
    handed("a struct beside a member of the same key", Nipping.serializer(), """PAYLOAD""") {
        Json.encodeToString(Nipping.serializer(), it)
    }
    handed("an optional struct after one of the same key", Slinging.serializer(), """PAYLOAD""") {
        val spare = Json.encodeToString(serializer<Labelling?>(), it.spare)
        "${Json.encodeToString(Slinging.serializer(), it)} $spare"
    }
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
    assert_eq!(
        expected,
        "a map, the one member: {\"north\":2}\n\
         a member of its own beside a flattened struct: {\"label\":\"n\",\"note\":\"a\"}\n"
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

/// A member flattened after a struct, or beside a member of its holder's own, is handed the
/// object without the keys they took, as serde hands it: a function after a struct is handed the
/// keys the struct did not take, and every key after a struct that takes none, and a struct that
/// reads a key a member ahead of it, or one of its holder's own, took is refused, as serde refuses
/// it. An optional one reads no value.
#[test]
fn a_member_flattened_after_a_struct_reads_in_kotlin_the_keys_serde_hands_it() {
    let slinging: Slinging = serde_json::from_str(PAYLOAD).unwrap();
    let expected = [
        format!("after a struct: {}", read_of::<Whipping>(PAYLOAD)),
        format!(
            "after a struct and beside a member of its own: {}",
            read_of::<Hitching>(HITCHING)
        ),
        format!("ahead of a struct: {}", read_of::<Bending>(PAYLOAD)),
        format!(
            "after a struct that names a function: {}",
            read_of::<Lanyarding>(PAYLOAD)
        ),
        format!(
            "after a struct with a flattened member: {}",
            read_of::<Brailing>(PAYLOAD)
        ),
        format!(
            "a struct after one of other keys: {}",
            read_of::<Stropping>(STROPPING)
        ),
        format!(
            "a struct after one of the same key: {}",
            read_of::<Stoppering>(PAYLOAD)
        ),
        format!(
            "a struct beside a member of the same key: {}",
            read_of::<Nipping>(PAYLOAD)
        ),
        format!(
            "an optional struct after one of the same key: {} {}",
            serde_json::to_string(&slinging).unwrap(),
            serde_json::to_string(&slinging.spare).unwrap()
        ),
    ]
    .map(|line| format!("{line}\n"))
    .concat();
    assert_eq!(
        expected,
        "after a struct: {\"label\":\"n\",\"keys\":[\"north\"]}\n\
         after a struct and beside a member of its own: \
         {\"label\":\"n\",\"note\":\"a\",\"keys\":[\"north\"]}\n\
         ahead of a struct: {\"keys\":[\"label\",\"north\"],\"label\":\"n\"}\n\
         after a struct that names a function: \
         {\"label\":\"n\",\"keys\":[\"label\",\"north\"]}\n\
         after a struct with a flattened member: \
         {\"label\":\"n\",\"keys\":[\"label\",\"north\"]}\n\
         a struct after one of other keys: {\"label\":\"n\",\"note\":\"a\"}\n\
         a struct after one of the same key: refused\n\
         a struct beside a member of the same key: refused\n\
         an optional struct after one of the same key: {\"label\":\"n\"} null\n"
    );
    let main = HANDED
        .replace("PAYLOAD", PAYLOAD)
        .replace("HITCHING", HITCHING)
        .replace("STROPPING", STROPPING);
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        labelling_kotlin::kotlin_definition(),
        bunting_kotlin::kotlin_definition(),
        noting_kotlin::kotlin_definition(),
        whipping_kotlin::kotlin_definition(),
        hitching_kotlin::kotlin_definition(),
        bending_kotlin::kotlin_definition(),
        ticketed_kotlin::kotlin_definition(),
        lanyarding_kotlin::kotlin_definition(),
        sheeting_kotlin::kotlin_definition(),
        brailing_kotlin::kotlin_definition(),
        stropping_kotlin::kotlin_definition(),
        stoppering_kotlin::kotlin_definition(),
        nipping_kotlin::kotlin_definition(),
        slinging_kotlin::kotlin_definition(),
        FUNCTIONS.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected);
}
