//! A map flattened as the one member of its struct, and a flattened struct beside a member of its
//! own, read and written by the emitted Kotlin types compiled with `-Werror`, and an optional
//! member beside a flattened one, beside its twin under `nullable`: each payload is written back
//! as the serde read writes it back. A member flattened after a struct, or beside a member of its
//! holder's own, is handed the keys serde hands it, and an externally tagged enum the one entry
//! serde hands it. An optional map, the one member, is read as `null` where serde reads `None`.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::flattened_models::{
    BELTING, Belting, Bending, Brailing, CLEATING, Cleating, CorrectedLocker, EmptiedLocker,
    Gasketing, HEAVING, HITCHING, Heaving, Hitching, Knotting, LOCKER, Lanyarding, Luffing,
    Netting, Nipping, PAYLOAD, ProbeLocker, Rousing, SCUDDING, STOWING, STROPPING, Scudding,
    Seizing, Slinging, Splicing, Stoppering, Stowing, Stropping, UNDER_AN_ALIAS, UNWRITTEN,
    Whipping, belting_kotlin, bending_kotlin, brailing_kotlin, bunting_kotlin, cleating_kotlin,
    corrected_locker_kotlin, emptied_locker_kotlin, gasketing_kotlin, heaving_kotlin,
    hitching_kotlin, jibing_kotlin, knotting_kotlin, lanyarding_kotlin, luffing_kotlin,
    netting_kotlin, nipping_kotlin, noting_kotlin, probe_locker_kotlin, quaying_kotlin, read_of,
    rousing_kotlin, scudding_kotlin, seizing_kotlin, sheeting_kotlin, slinging_kotlin,
    splicing_kotlin, stoppering_kotlin, stowing_kotlin, stropping_kotlin, tacking_kotlin,
    ticketed_kotlin, unwritten_kotlin, whipping_kotlin,
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

/// The functions the models name: one answers the payload as it was handed, one `null`, and one
/// the keys of the object it is handed.
const FUNCTIONS: &str = r#"
fun asGiven(payload: JsonElement): JsonElement = payload

fun asNull(payload: JsonElement): JsonElement = JsonNull

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
    handed("after a struct with an unwritten member", Rousing.serializer(), """UNWRITTEN""") {
        Json.encodeToString(Rousing.serializer(), it)
    }
    handed("a struct after one with an unwritten member", Gasketing.serializer(), """UNWRITTEN""") {
        Json.encodeToString(Gasketing.serializer(), it)
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

/// What prints a row of a holder of a flattened enum: what the read holds, as `shown` tells it, or
/// that the read is refused.
const ENTRY_READ: &str = r#"
fun <T> entry(name: String, serializer: KSerializer<T>, payload: String, shown: (T) -> String) {
    val told = try {
        shown(Json.decodeFromString(serializer, payload))
    } catch (refused: SerializationException) {
        "refused"
    }
    println("$name: $told")
}
"#;

/// A payload, the type it is read as, and what serde writes back of it.
struct Row {
    json: &'static str,
    name: &'static str,
    read_as: &'static str,
    written: fn(&str) -> String,
}

/// A holder of a flattened enum, a payload it is read from, what Kotlin prints of what it read,
/// and what serde reads of it, told the same way.
struct EntryRow {
    json: &'static str,
    read_as: &'static str,
    shown: &'static str,
    told: fn(&str) -> String,
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
/// keys the struct did not take, a struct with a member serde never writes among them, and every
/// key after a struct that takes none, and a struct that reads a key a member ahead of it, or one
/// of its holder's own, took is refused, as serde refuses it. An optional one reads no value.
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
            "after a struct with an unwritten member: {}",
            read_of::<Rousing>(UNWRITTEN)
        ),
        format!(
            "a struct after one with an unwritten member: {}",
            read_of::<Gasketing>(UNWRITTEN)
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
         after a struct with an unwritten member: \
         {\"label\":\"n\",\"keys\":[\"north\"]}\n\
         a struct after one with an unwritten member: refused\n\
         a struct after one of other keys: {\"label\":\"n\",\"note\":\"a\"}\n\
         a struct after one of the same key: refused\n\
         a struct beside a member of the same key: refused\n\
         an optional struct after one of the same key: {\"label\":\"n\"} null\n"
    );
    let main = HANDED
        .replace("PAYLOAD", PAYLOAD)
        .replace("HITCHING", HITCHING)
        .replace("STROPPING", STROPPING)
        .replace("UNWRITTEN", UNWRITTEN);
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
        unwritten_kotlin::kotlin_definition(),
        rousing_kotlin::kotlin_definition(),
        gasketing_kotlin::kotlin_definition(),
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

/// What serde reads of `json` as `T`, as `shown` tells it, or `refused` where it refuses it.
fn told_by_serde<T>(json: &str, shown: fn(&T) -> String) -> String
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json).map_or_else(|_| "refused".to_owned(), |read| shown(&read))
}

/// The JSON serde writes of `value`.
fn json<T>(value: &T) -> String
where
    T: Serialize,
{
    serde_json::to_string(value).unwrap()
}

/// Each row of a holder of a flattened externally tagged enum: what serde reads of each payload,
/// the enum written alone and each other member after it.
fn entry_rows() -> Vec<EntryRow> {
    let stowing = |json: &str| {
        told_by_serde(json, |read: &Stowing| {
            format!("{} {}", self::json(&read.tacking), read.note)
        })
    };
    let belting = |json: &str| {
        told_by_serde(json, |read: &Belting| {
            format!("{} {}", self::json(&read.tacking), read.note)
        })
    };
    let heaving = |json: &str| {
        told_by_serde(json, |read: &Heaving| {
            format!("{} {}", self::json(&read.tacking), self::json(&read.tally))
        })
    };
    let cleating = |json: &str| {
        told_by_serde(json, |read: &Cleating| {
            format!("{} {}", self::json(&read.jibing), self::json(&read.tally))
        })
    };
    let scudding = |json: &str| {
        told_by_serde(json, |read: &Scudding| {
            format!(
                "{} {} {}",
                self::json(&read.hatch),
                self::json(&read.labelling),
                self::json(&read.tally)
            )
        })
    };
    let enum_and_note = "\"${Json.encodeToString(serializer<Tacking>(), it.tacking)} ${it.note}\"";
    let optional_and_note =
        "\"${Json.encodeToString(serializer<Tacking?>(), it.tacking)} ${it.note}\"";
    let enum_and_tally = "\"${Json.encodeToString(serializer<Tacking>(), it.tacking)} \
                          ${Json.encodeToString(serializer<Bunting>(), it.tally)}\"";
    let renamed_and_tally = "\"${Json.encodeToString(serializer<Jibing>(), it.jibing)} \
                             ${Json.encodeToString(serializer<Bunting>(), it.tally)}\"";
    let enum_struct_and_tally = "\"${Json.encodeToString(serializer<Tacking>(), it.hatch)} \
                                 ${Json.encodeToString(serializer<Labelling>(), it.labelling)} \
                                 ${Json.encodeToString(serializer<Bunting>(), it.tally)}\"";
    let mut rows = Vec::new();
    rows.extend(STOWING.map(|json| EntryRow {
        json,
        read_as: "Stowing",
        shown: enum_and_note,
        told: stowing,
    }));
    rows.extend(BELTING.map(|json| EntryRow {
        json,
        read_as: "Belting",
        shown: optional_and_note,
        told: belting,
    }));
    rows.extend(HEAVING.map(|json| EntryRow {
        json,
        read_as: "Heaving",
        shown: enum_and_tally,
        told: heaving,
    }));
    rows.extend(CLEATING.map(|json| EntryRow {
        json,
        read_as: "Cleating",
        shown: renamed_and_tally,
        told: cleating,
    }));
    rows.push(EntryRow {
        json: SCUDDING,
        read_as: "Scudding",
        shown: enum_struct_and_tally,
        told: scudding,
    });
    rows
}

/// The Kotlin source of every holder of a flattened enum and what it flattens, with `main`.
fn entry_source(main: String) -> String {
    [
        KOTLIN_IMPORTS.to_owned(),
        labelling_kotlin::kotlin_definition(),
        bunting_kotlin::kotlin_definition(),
        tacking_kotlin::kotlin_definition(),
        jibing_kotlin::kotlin_definition(),
        stowing_kotlin::kotlin_definition(),
        belting_kotlin::kotlin_definition(),
        heaving_kotlin::kotlin_definition(),
        cleating_kotlin::kotlin_definition(),
        scudding_kotlin::kotlin_definition(),
        FUNCTIONS.to_owned(),
        main,
    ]
    .join("\n\n")
}

/// A flattened externally tagged enum is handed the first entry, in the object's order, under the
/// name a variant is read under, and every member after it the object without that entry, as serde
/// hands them: beside a member of its holder's own, beside a key no member reads, ahead of a
/// function and of a struct, under a `rename` and the enum's `rename_all`, and optional. A payload
/// with no such entry is refused, and an optional enum reads no value.
#[test]
fn a_flattened_enum_reads_in_kotlin_the_entry_serde_hands_it() {
    let rows = entry_rows();
    let mut expected = String::new();
    let mut main = String::from(ENTRY_READ);
    main.push_str("fun main() {\n");
    for (at, row) in rows.iter().enumerate() {
        let _ = writeln!(expected, "{at}: {}", (row.told)(row.json));
        let _ = writeln!(
            main,
            "    entry(\"{at}\", {}.serializer(), \"\"\"{}\"\"\") {{ {} }}",
            row.read_as, row.json, row.shown
        );
    }
    main.push_str("}\n");
    assert_eq!(
        expected,
        "0: {\"Bay\":{\"label\":\"n\"}} a\n\
         1: {\"Bay\":{\"label\":\"n\"}} a\n\
         2: {\"Bay\":{\"label\":\"n\"}} a\n\
         3: {\"Bay\":{\"label\":\"n\"}} a\n\
         4: \"Calm\" a\n\
         5: {\"Held\":\"x\"} a\n\
         6: refused\n\
         7: {\"Bay\":{\"label\":\"n\"}} a\n\
         8: null a\n\
         9: null a\n\
         10: {\"Bay\":{\"label\":\"n\"}} {\"keys\":[\"north\"]}\n\
         11: {\"Bay\":{\"label\":\"n\"}} {\"keys\":[\"Dock\",\"north\"]}\n\
         12: {\"Dock\":{\"label\":\"s\"}} {\"keys\":[\"Bay\",\"north\"]}\n\
         13: {\"Bay\":{\"label\":\"n\"}} {\"keys\":[\"Quay\",\"north\"]}\n\
         14: {\"pier\":{\"label\":\"n\"}} {\"keys\":[\"north\"]}\n\
         15: {\"open_bay\":{\"label\":\"n\"}} {\"keys\":[\"north\"]}\n\
         16: refused\n\
         17: {\"Bay\":{\"label\":\"n\"}} {\"label\":\"m\"} {\"keys\":[\"north\"]}\n"
    );
    let Some(printed) = ran_kotlin(&entry_source(main)) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A variant's entry under its `alias` is taken by the holder as serde takes it, and the Kotlin
/// read of the enum refuses it, where serde reads the variant.
#[test]
fn a_flattened_enum_refuses_in_kotlin_a_variant_under_its_alias() {
    let serde_read = [
        told_by_serde(UNDER_AN_ALIAS[0], |read: &Stowing| {
            format!("{} {}", json(&read.tacking), read.note)
        }),
        told_by_serde(UNDER_AN_ALIAS[1], |read: &Heaving| {
            format!("{} {}", json(&read.tacking), json(&read.tally))
        }),
    ];
    assert_eq!(
        serde_read,
        [
            "{\"Dock\":{\"label\":\"s\"}} a",
            "{\"Dock\":{\"label\":\"s\"}} {\"keys\":[\"north\"]}"
        ]
    );
    let main = format!(
        "fun <T> refusal(serializer: KSerializer<T>, payload: String) {{\n\
         try {{ Json.decodeFromString(serializer, payload); println(\"read\") }} \
         catch (refused: IllegalStateException) {{ println(refused.message) }}\n\
         }}\n\n\
         fun main() {{\n\
         refusal(Stowing.serializer(), \"\"\"{}\"\"\")\n\
         refusal(Heaving.serializer(), \"\"\"{}\"\"\")\n\
         }}\n",
        UNDER_AN_ALIAS[0], UNDER_AN_ALIAS[1]
    );
    let Some(printed) = ran_kotlin(&entry_source(main)) else {
        return;
    };
    assert_eq!(printed, "unknown tag Quay\nunknown tag Quay\n");
}

/// An enum declared below its holder is not known to take an entry: it is read from the object
/// its holder hands it, as serde reads it where that object holds its entry alone, and refused
/// where it holds any other key, which serde reads.
#[test]
fn an_enum_declared_below_its_holder_is_read_in_kotlin_from_the_object_it_is_handed() {
    let beside = STOWING[2];
    let serde_read = [STOWING[0], beside].map(|payload| {
        told_by_serde(payload, |read: &Luffing| {
            format!("{} {}", json(&read.quaying), read.note)
        })
    });
    assert_eq!(serde_read, ["{\"Bay\":{\"label\":\"n\"}} a"; 2]);
    let main = format!(
        "fun main() {{\n\
         val read = Json.decodeFromString(Luffing.serializer(), \"\"\"{}\"\"\")\n\
         println(\"${{Json.encodeToString(serializer<Quaying>(), read.quaying)}} ${{read.note}}\")\n\
         try {{ Json.decodeFromString(Luffing.serializer(), \"\"\"{beside}\"\"\"); println(\"read\") }} \
         catch (refused: IllegalArgumentException) {{ println(refused.message) }}\n\
         }}\n",
        STOWING[0]
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        quaying_kotlin::kotlin_definition(),
        luffing_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(
        printed,
        format!("{}\nCollection has more than one element.\n", serde_read[0])
    );
}

/// What serde reads of `json` as `T`: its map, then the whole read written back, or `refused`.
fn locker_told<T>(json: &str, counts: fn(&T) -> String) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json).map_or_else(
        |_| "refused".to_owned(),
        |read| format!("{} {}", counts(&read), self::json(&read)),
    )
}

/// An optional map, the one member, is read and written back as serde reads and writes it: the
/// map where every entry reads as a count, and `null` where one does not, which writes no entry.
/// Under a function it is read from what the function answers: an entry that does not read as a
/// count is refused, and `null` is read as `null`.
#[test]
fn an_optional_flattened_map_reads_and_writes_in_kotlin_as_serde_does() {
    let rows = [
        (
            "ProbeLocker",
            LOCKER.map(|json| locker_told(json, |read: &ProbeLocker| self::json(&read.counts))),
        ),
        (
            "CorrectedLocker",
            LOCKER.map(|json| locker_told(json, |read: &CorrectedLocker| self::json(&read.counts))),
        ),
        (
            "EmptiedLocker",
            LOCKER.map(|json| locker_told(json, |read: &EmptiedLocker| self::json(&read.counts))),
        ),
    ];
    let mut expected = String::new();
    let mut main = String::from(ENTRY_READ);
    main.push_str("fun main() {\n");
    for (read_as, told) in &rows {
        for (json, serde_told) in LOCKER.iter().zip(told) {
            let _ = writeln!(expected, "{read_as} {json}: {serde_told}");
            let _ = writeln!(
                main,
                "    entry(\"\"\"{read_as} {json}\"\"\", {read_as}.serializer(), \"\"\"{json}\"\"\") {{ \
                 \"${{Json.encodeToString(serializer<Map<String, UByte>?>(), it.counts)}} \
                 ${{Json.encodeToString({read_as}.serializer(), it)}}\" }}"
            );
        }
    }
    main.push_str("}\n");
    assert_eq!(
        expected,
        "ProbeLocker {}: {} {}\n\
         ProbeLocker {\"north\":2}: {\"north\":2} {\"north\":2}\n\
         ProbeLocker {\"north\":\"x\"}: null {}\n\
         ProbeLocker {\"north\":2,\"south\":\"x\"}: null {}\n\
         ProbeLocker {\"north\":null}: null {}\n\
         CorrectedLocker {}: {} {}\n\
         CorrectedLocker {\"north\":2}: {\"north\":2} {\"north\":2}\n\
         CorrectedLocker {\"north\":\"x\"}: refused\n\
         CorrectedLocker {\"north\":2,\"south\":\"x\"}: refused\n\
         CorrectedLocker {\"north\":null}: refused\n\
         EmptiedLocker {}: null {}\n\
         EmptiedLocker {\"north\":2}: null {}\n\
         EmptiedLocker {\"north\":\"x\"}: null {}\n\
         EmptiedLocker {\"north\":2,\"south\":\"x\"}: null {}\n\
         EmptiedLocker {\"north\":null}: null {}\n"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        probe_locker_kotlin::kotlin_definition(),
        corrected_locker_kotlin::kotlin_definition(),
        emptied_locker_kotlin::kotlin_definition(),
        FUNCTIONS.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected);
}
