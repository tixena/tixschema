//! A map flattened as the one member of its struct, and a flattened struct beside a member of its
//! own, read and written by the emitted Kotlin types compiled with `-Werror`, and an optional
//! member beside a flattened one, beside its twin under `nullable`: each payload is written back
//! as the serde read writes it back. A member flattened after a struct, or beside a member of its
//! holder's own, is handed the keys serde hands it, and an externally tagged enum the one entry
//! serde hands it, and is written as serde writes it, a unit variant as its name over `null`. An
//! optional map, the one member, is read as `null` where serde reads `None`, and a map keyed by a
//! plain enum under the names serde writes its variants under. An optional struct with a flattened
//! member of its own is read wherever the struct reads, and as `null` wherever serde reads `None`,
//! and one that takes a key is `null` where the key is absent and refuses a key that does not read.
//! An optional type the emitter recorded no keys for is read as the first where its serializer
//! declares no property, and as the second where it declares some. A variant reads and writes a
//! flattened member at its own level, under every tagging.

#![cfg(feature = "kotlin")]

use core::fmt::Debug;
use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::flattened_models::{
    BELTING, BOWLINING, BROACHING, Belting, Bending, Bowlining, Brailing, Broaching, CHOCKED,
    CLEATING, Cathead, Cleating, CorrectedLocker, Dipping, ENSIGN, EmptiedLocker, GAFFING, Gaffing,
    Gammoning, Gasketing, HEAVING, HITCHING, Hawsepipe, Heaving, Hitching, Knotting, LOCKER,
    Lanyarding, Luffing, MARLINE, Netting, Nipping, PAYLOAD, ProbeLocker, Rousing, SCARFING,
    SCUDDING, SNUBBED, STOWING, STROPPING, STRUCK, SWAYING, Scarfing, Scudding, Seizing, Slinging,
    Snubbing, Splicing, Stoppering, Stowing, Striking, Stropping, Swaying, Tacking, UNDER_AN_ALIAS,
    UNWRITTEN, WHELPING, Whelping, Whipping, Yawing, belting_kotlin, bending_kotlin,
    bowlining_kotlin, brailing_kotlin, broaching_kotlin, bunting_kotlin, cathead_kotlin,
    chock_kotlin, cleating_kotlin, corrected_locker_kotlin, dipping_kotlin, emptied_locker_kotlin,
    ensign_kotlin, gaffing_kotlin, gammoning_kotlin, gasketing_kotlin, hawsepipe_kotlin,
    hawser_kotlin, heaving_kotlin, hitching_kotlin, jibing_kotlin, knotting_kotlin,
    lanyarding_kotlin, luffing_kotlin, marline_kotlin, netting_kotlin, nipping_kotlin,
    noting_kotlin, probe_locker_kotlin, quaying_kotlin, read_of, rousing_kotlin, scarfing_kotlin,
    scudding_kotlin, seizing_kotlin, sheeting_kotlin, slinging_kotlin, snubbing_kotlin,
    splicing_kotlin, stoppering_kotlin, stowing_kotlin, striking_kotlin, stropping_kotlin,
    swaying_kotlin, tacking_kotlin, ticketed_kotlin, unwritten_kotlin, whelping_kotlin,
    whipping_kotlin, yawing_kotlin,
};
use super::runtime::ran_kotlin;
use super::swift_flattened_options_models::{
    Crowning, FLAKING, FLAKING_ABSENT, FLAKING_MISTYPED, Fidding, Flaking, LAPPING, LAPPING_ABSENT,
    LAPPING_MISTYPED, Ratline, crowning_kotlin, fidding_kotlin, flaking_kotlin, nippering_kotlin,
    ratline_kotlin, tabling_kotlin,
};
use super::tuple_variants_models::labelling_kotlin;
use super::unrecorded_options_models::{
    READ_BACK, UNRECORDED, bight_kotlin, bridled_kotlin, bridling_kotlin, crowned_kotlin,
    fiddling_kotlin, hitched_kotlin, slipped_kotlin, slipping_kotlin, snagging_kotlin,
};

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
const HANDED_ROW: &str = r#"
fun <T> handed(name: String, serializer: KSerializer<T>, payload: String, shown: (T) -> String) {
    val told = try {
        shown(Json.decodeFromString(serializer, payload))
    } catch (refused: SerializationException) {
        "refused"
    }
    println("$name: $told")
}
"#;

/// The rows of each holder of a member flattened after or beside another.
const HANDED: &str = r#"
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

/// The rows of each holder of a member flattened after a struct whose own tag, or whose member
/// serde never reads, is in the object.
const HANDED_ON: &str = r#"
fun main() {
    handed("after a struct under its own tag", Dipping.serializer(), """ENSIGN""") {
        "${it.ensign.label} ${Json.encodeToString(Bunting.serializer(), it.tally)}"
    }
    handed("after a struct with a member it never reads", Gammoning.serializer(), """MARLINE""") {
        "${it.marline.label} ${Json.encodeToString(Bunting.serializer(), it.tally)}"
    }
    handed("an optional struct under its own tag, its tag alone", Striking.serializer(), """STRUCK""") {
        Json.encodeToString(Striking.serializer(), it)
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

/// What prints a row of a written holder of a flattened enum: the value built in Kotlin written,
/// and the payload serde wrote read and written back.
const WRITTEN: &str = r#"
fun <T> written(name: String, serializer: KSerializer<T>, value: T, payload: String) {
    val back = Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))
    println("$name: ${Json.encodeToString(serializer, value)} $back")
}
"#;

/// What prints a row of an enum with a flattened member in a variant: the subclass read and the
/// enum written back, or that the read is refused.
const VARIANT_READ: &str = r#"
fun <T : Any> variant(serializer: KSerializer<T>, payload: String) {
    val told = try {
        val read = Json.decodeFromString(serializer, payload)
        "${read::class.simpleName} ${Json.encodeToString(serializer, read)}"
    } catch (refused: SerializationException) {
        "refused"
    }
    println("$payload: $told")
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
        HANDED_ROW.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A member flattened after a struct is handed the struct's own tag and the key of a member the
/// struct never reads, as serde hands them, and an optional struct whose tag alone is in the
/// object reads no value, as serde reads none.
#[test]
fn a_member_flattened_after_a_struct_reads_in_kotlin_the_tag_and_the_keys_serde_hands_on() {
    let dipping: Dipping = serde_json::from_str(ENSIGN).unwrap();
    let gammoning: Gammoning = serde_json::from_str(MARLINE).unwrap();
    let expected = [
        format!(
            "after a struct under its own tag: {} {}",
            dipping.ensign.label,
            serde_json::to_string(&dipping.tally).unwrap()
        ),
        format!(
            "after a struct with a member it never reads: {} {}",
            gammoning.marline.label,
            serde_json::to_string(&gammoning.tally).unwrap()
        ),
        format!(
            "an optional struct under its own tag, its tag alone: {}",
            read_of::<Striking>(STRUCK)
        ),
    ]
    .map(|line| format!("{line}\n"))
    .concat();
    assert_eq!(
        expected,
        "after a struct under its own tag: n {\"keys\":[\"kind\",\"north\"]}\n\
         after a struct with a member it never reads: n {\"keys\":[\"north\",\"spare\"]}\n\
         an optional struct under its own tag, its tag alone: {\"note\":\"a\"}\n"
    );
    let main = HANDED_ON
        .replace("ENSIGN", ENSIGN)
        .replace("MARLINE", MARLINE)
        .replace("STRUCK", STRUCK);
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        bunting_kotlin::kotlin_definition(),
        ensign_kotlin::kotlin_definition(),
        dipping_kotlin::kotlin_definition(),
        marline_kotlin::kotlin_definition(),
        gammoning_kotlin::kotlin_definition(),
        striking_kotlin::kotlin_definition(),
        FUNCTIONS.to_owned(),
        HANDED_ROW.to_owned(),
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
    rows.push(EntryRow {
        json: UNDER_AN_ALIAS[0],
        read_as: "Stowing",
        shown: enum_and_note,
        told: stowing,
    });
    rows.push(EntryRow {
        json: UNDER_AN_ALIAS[1],
        read_as: "Heaving",
        shown: enum_and_tally,
        told: heaving,
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
/// function and of a struct, under a `rename`, the enum's `rename_all` and an `alias`, and
/// optional. A payload with no such entry is refused, and an optional enum reads no value.
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
         17: {\"Bay\":{\"label\":\"n\"}} {\"label\":\"m\"} {\"keys\":[\"north\"]}\n\
         18: {\"Dock\":{\"label\":\"s\"}} a\n\
         19: {\"Dock\":{\"label\":\"s\"}} {\"keys\":[\"north\"]}\n"
    );
    let Some(printed) = ran_kotlin(&entry_source(main)) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A flattened externally tagged enum is written as serde writes it, beside a member of its
/// holder's own, optional and under a function: a variant that holds a value as its entry, and a
/// unit variant as its name over `null`. What serde writes is read and written back as it was.
#[test]
fn a_flattened_enum_writes_in_kotlin_as_serde_writes_it() {
    let tackings = [
        (
            Tacking::Bay {
                label: "n".to_owned(),
            },
            "TackingBay(label = \"n\")",
        ),
        (Tacking::Calm, "TackingCalm"),
        (
            Tacking::Dock {
                label: "s".to_owned(),
            },
            "TackingDock(label = \"s\")",
        ),
        (Tacking::Held("x".to_owned()), "TackingHeld(value = \"x\")"),
    ];
    let note = || "a".to_owned();
    let mut rows = Vec::new();
    for (tacking, built) in tackings {
        let stowing = Stowing {
            note: note(),
            tacking: tacking.clone(),
        };
        let belting = Belting {
            note: note(),
            tacking: Some(tacking.clone()),
        };
        let yawing = Yawing {
            note: note(),
            tacking,
        };
        let written = [
            ("Stowing", json(&stowing)),
            ("Belting", json(&belting)),
            ("Yawing", json(&yawing)),
        ];
        rows.extend(
            written.map(|(holder, wrote)| (holder, wrote, format!("{holder}(\"a\", {built})"))),
        );
    }
    let unheld = Belting {
        note: note(),
        tacking: None,
    };
    rows.push(("Belting", json(&unheld), "Belting(\"a\", null)".to_owned()));
    let mut expected = String::new();
    let mut main = String::from(WRITTEN);
    main.push_str("fun main() {\n");
    for (holder, serde_wrote, built) in &rows {
        let _ = writeln!(expected, "{holder}: {serde_wrote} {serde_wrote}");
        let _ = writeln!(
            main,
            "    written(\"{holder}\", {holder}.serializer(), {built}, \"\"\"{serde_wrote}\"\"\")"
        );
    }
    main.push_str("}\n");
    assert_eq!(
        expected,
        "Stowing: {\"note\":\"a\",\"Bay\":{\"label\":\"n\"}} \
         {\"note\":\"a\",\"Bay\":{\"label\":\"n\"}}\n\
         Belting: {\"note\":\"a\",\"Bay\":{\"label\":\"n\"}} \
         {\"note\":\"a\",\"Bay\":{\"label\":\"n\"}}\n\
         Yawing: {\"note\":\"a\",\"Bay\":{\"label\":\"n\"}} \
         {\"note\":\"a\",\"Bay\":{\"label\":\"n\"}}\n\
         Stowing: {\"note\":\"a\",\"Calm\":null} {\"note\":\"a\",\"Calm\":null}\n\
         Belting: {\"note\":\"a\",\"Calm\":null} {\"note\":\"a\",\"Calm\":null}\n\
         Yawing: {\"note\":\"a\",\"Calm\":null} {\"note\":\"a\",\"Calm\":null}\n\
         Stowing: {\"note\":\"a\",\"Dock\":{\"label\":\"s\"}} \
         {\"note\":\"a\",\"Dock\":{\"label\":\"s\"}}\n\
         Belting: {\"note\":\"a\",\"Dock\":{\"label\":\"s\"}} \
         {\"note\":\"a\",\"Dock\":{\"label\":\"s\"}}\n\
         Yawing: {\"note\":\"a\",\"Dock\":{\"label\":\"s\"}} \
         {\"note\":\"a\",\"Dock\":{\"label\":\"s\"}}\n\
         Stowing: {\"note\":\"a\",\"Held\":\"x\"} {\"note\":\"a\",\"Held\":\"x\"}\n\
         Belting: {\"note\":\"a\",\"Held\":\"x\"} {\"note\":\"a\",\"Held\":\"x\"}\n\
         Yawing: {\"note\":\"a\",\"Held\":\"x\"} {\"note\":\"a\",\"Held\":\"x\"}\n\
         Belting: {\"note\":\"a\"} {\"note\":\"a\"}\n"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        tacking_kotlin::kotlin_definition(),
        stowing_kotlin::kotlin_definition(),
        belting_kotlin::kotlin_definition(),
        yawing_kotlin::kotlin_definition(),
        FUNCTIONS.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected);
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

/// A map keyed by a plain enum, the one member, is read and written back as serde reads and
/// writes it: each key as the name its variant is written under, its `rename` and the enum's
/// `rename_all` among them, and a key no variant is written under refused, or read as `null`
/// where the map is optional, which writes no entry.
#[test]
fn a_flattened_map_keyed_by_an_enum_reads_and_writes_in_kotlin_as_serde_does() {
    let rows = [
        (
            "Cathead",
            "Map<Chock, UByte>",
            CHOCKED
                .map(|json| locker_told(json, |read: &Cathead| self::json(&read.lines)))
                .to_vec(),
            CHOCKED.as_slice(),
        ),
        (
            "Hawsepipe",
            "Map<Chock, UByte>?",
            CHOCKED
                .map(|json| locker_told(json, |read: &Hawsepipe| self::json(&read.lines)))
                .to_vec(),
            CHOCKED.as_slice(),
        ),
        (
            "Snubbing",
            "Map<Hawser, UByte>",
            SNUBBED
                .map(|json| locker_told(json, |read: &Snubbing| self::json(&read.lines)))
                .to_vec(),
            SNUBBED.as_slice(),
        ),
    ];
    let mut expected = String::new();
    let mut main = String::from(ENTRY_READ);
    main.push_str("fun main() {\n");
    for (read_as, lines, told, payloads) in &rows {
        for (json, serde_told) in payloads.iter().zip(told) {
            let _ = writeln!(expected, "{read_as} {json}: {serde_told}");
            let _ = writeln!(
                main,
                "    entry(\"\"\"{read_as} {json}\"\"\", {read_as}.serializer(), \"\"\"{json}\"\"\") {{ \
                 \"${{Json.encodeToString(serializer<{lines}>(), it.lines)}} \
                 ${{Json.encodeToString({read_as}.serializer(), it)}}\" }}"
            );
        }
    }
    main.push_str("}\n");
    assert_eq!(
        expected,
        "Cathead {}: {} {}\n\
         Cathead {\"Port\":2}: {\"Port\":2} {\"Port\":2}\n\
         Cathead {\"Port\":2,\"Starboard\":3}: {\"Port\":2,\"Starboard\":3} \
         {\"Port\":2,\"Starboard\":3}\n\
         Cathead {\"Bow\":2}: refused\n\
         Cathead {\"Port\":\"x\"}: refused\n\
         Hawsepipe {}: {} {}\n\
         Hawsepipe {\"Port\":2}: {\"Port\":2} {\"Port\":2}\n\
         Hawsepipe {\"Port\":2,\"Starboard\":3}: {\"Port\":2,\"Starboard\":3} \
         {\"Port\":2,\"Starboard\":3}\n\
         Hawsepipe {\"Bow\":2}: null {}\n\
         Hawsepipe {\"Port\":\"x\"}: null {}\n\
         Snubbing {\"bow\":2}: {\"bow\":2} {\"bow\":2}\n\
         Snubbing {\"open_water\":2}: {\"open_water\":2} {\"open_water\":2}\n\
         Snubbing {\"OpenWater\":2}: refused\n"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        chock_kotlin::kotlin_definition(),
        hawser_kotlin::kotlin_definition(),
        cathead_kotlin::kotlin_definition(),
        hawsepipe_kotlin::kotlin_definition(),
        snubbing_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// An optional struct with a flattened member of its own, which takes no key, is read wherever
/// the struct reads from the object and as `null` wherever it does not, as serde reads `None`: no
/// key, a key the struct reads absent, and a key held as a type it does not read. An optional
/// struct that takes a key is `null` where the key is absent and is read where it is there, so a
/// key that does not read refuses the payload, as TypeScript and Zod refuse it, where serde reads
/// `None`. Each holder is written back as serde writes it back, and the struct with a flattened
/// member of its own is read and written as serde does.
#[test]
fn an_optional_struct_that_takes_no_key_reads_and_writes_in_kotlin_as_serde_does() {
    let crowning = |json: &str| told_by_serde(json, |read: &Crowning| self::json(read));
    let fidding = |json: &str| {
        told_by_serde(json, |read: &Fidding| {
            format!("{} {}", self::json(&read.crowning), self::json(read))
        })
    };
    let scarfing = |json: &str| {
        told_by_serde(json, |read: &Scarfing| {
            format!("{} {}", self::json(&read.sheeting), self::json(read))
        })
    };
    let flaking = |json: &str| {
        told_by_serde(json, |read: &Flaking| {
            format!("{} {}", self::json(&read.tabling), self::json(read))
        })
    };
    let fidding_shown = "\"${Json.encodeToString(serializer<Crowning?>(), it.crowning)} \
                         ${Json.encodeToString(Fidding.serializer(), it)}\"";
    let scarfing_shown = "\"${Json.encodeToString(serializer<Sheeting?>(), it.sheeting)} \
                          ${Json.encodeToString(Scarfing.serializer(), it)}\"";
    let flaking_shown = "\"${Json.encodeToString(serializer<Tabling?>(), it.tabling)} \
                         ${Json.encodeToString(Flaking.serializer(), it)}\"";
    let mut rows = vec![EntryRow {
        json: LAPPING,
        read_as: "Crowning",
        shown: "Json.encodeToString(Crowning.serializer(), it)",
        told: crowning,
    }];
    rows.extend(
        [LAPPING, FLAKING_ABSENT, LAPPING_ABSENT, LAPPING_MISTYPED].map(|json| EntryRow {
            json,
            read_as: "Fidding",
            shown: fidding_shown,
            told: fidding,
        }),
    );
    rows.extend(SCARFING.map(|json| EntryRow {
        json,
        read_as: "Scarfing",
        shown: scarfing_shown,
        told: scarfing,
    }));
    rows.extend(
        [FLAKING, FLAKING_ABSENT, FLAKING_MISTYPED].map(|json| EntryRow {
            json,
            read_as: "Flaking",
            shown: flaking_shown,
            told: flaking,
        }),
    );
    let serde_told: Vec<String> = rows.iter().map(|row| (row.told)(row.json)).collect();
    assert_eq!(
        serde_told,
        [
            "{\"note\":\"n\",\"label\":\"north\"}",
            "{\"note\":\"n\",\"label\":\"north\"} {\"note\":\"n\",\"label\":\"north\"}",
            "null {}",
            "null {}",
            "null {}",
            "{\"label\":\"n\"} {\"label\":\"n\"}",
            "null {}",
            "null {}",
            "{\"label\":\"north\"} {\"label\":\"north\"}",
            "null {}",
            "null {}",
        ],
        "serde no longer reads each payload as it did"
    );
    let mut in_kotlin = serde_told;
    // The key `Tabling` reads, held as a number: TypeScript and Zod refuse it.
    in_kotlin[10] = "refused".to_owned();
    let mut expected = String::new();
    let mut main = String::from(ENTRY_READ);
    main.push_str("fun main() {\n");
    for (at, (row, told)) in rows.iter().zip(&in_kotlin).enumerate() {
        let _ = writeln!(expected, "{at}: {told}");
        let _ = writeln!(
            main,
            "    entry(\"{at}\", {}.serializer(), \"\"\"{}\"\"\") {{ {} }}",
            row.read_as, row.json, row.shown
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        tabling_kotlin::kotlin_definition(),
        crowning_kotlin::kotlin_definition(),
        fidding_kotlin::kotlin_definition(),
        flaking_kotlin::kotlin_definition(),
        labelling_kotlin::kotlin_definition(),
        sheeting_kotlin::kotlin_definition(),
        scarfing_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not the read above");
}

/// An optional member whose type the emitter recorded no keys for and whose serializer is written
/// by hand, a struct declared below its holder with a flattened member of its own and an enum
/// under each tagging but serde's own, is read wherever its read succeeds and is `null` wherever
/// it throws, as serde reads it, and is written back as serde writes it back. A struct declared
/// below its holder whose serializer the plugin writes is looked for by the keys its descriptor
/// names: `null` where none is there, and a key that does not read refuses the payload, as
/// TypeScript and Zod refuse it, where serde reads `None`.
#[test]
fn an_optional_type_of_no_recorded_key_reads_and_writes_in_kotlin_as_serde_does() {
    let serde_told: Vec<String> = UNRECORDED
        .iter()
        .map(|row| (row.read_back)(row.json))
        .collect();
    assert_eq!(
        serde_told, READ_BACK,
        "serde no longer reads each payload as it did"
    );
    let looked_for = [FLAKING, FLAKING_ABSENT, FLAKING_MISTYPED];
    let ratline_told =
        looked_for.map(|json| told_by_serde(json, |read: &Ratline| self::json(read)));
    assert_eq!(
        ratline_told,
        ["{\"label\":\"north\"}", "{}", "{}"],
        "serde no longer reads each payload as it did"
    );
    let rows = UNRECORDED
        .iter()
        .map(|row| (row.holder, row.json))
        .chain(looked_for.map(|json| ("Ratline", json)));
    // The key `Nippering` reads, held as a number: TypeScript and Zod refuse it.
    let in_kotlin = READ_BACK.into_iter().chain([
        ratline_told[0].as_str(),
        ratline_told[1].as_str(),
        "refused",
    ]);
    let mut expected = String::new();
    let mut main = String::from(ENTRY_READ);
    main.push_str("fun main() {\n");
    for (at, ((holder, json), told)) in rows.zip(in_kotlin).enumerate() {
        let _ = writeln!(expected, "{at}: {told}");
        let _ = writeln!(
            main,
            "    entry(\"{at}\", {holder}.serializer(), \"\"\"{json}\"\"\") {{ \
             Json.encodeToString({holder}.serializer(), it) }}"
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        bight_kotlin::kotlin_definition(),
        fiddling_kotlin::kotlin_definition(),
        crowned_kotlin::kotlin_definition(),
        hitched_kotlin::kotlin_definition(),
        snagging_kotlin::kotlin_definition(),
        bridled_kotlin::kotlin_definition(),
        bridling_kotlin::kotlin_definition(),
        slipped_kotlin::kotlin_definition(),
        slipping_kotlin::kotlin_definition(),
        ratline_kotlin::kotlin_definition(),
        nippering_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not the read above");
}

/// What serde reads of `json` as the enum `T`, published as `enum_name`: the variant read, named
/// as its Kotlin subclass is, and the enum written back, or `refused`.
fn variant_read<T>(enum_name: &str, json: &str) -> String
where
    T: Debug + DeserializeOwned + Serialize,
{
    let told = told_by_serde(json, |read: &T| {
        let debugged = format!("{read:?}");
        let variant = debugged.split(' ').next().unwrap().to_owned();
        format!("{variant} {}", self::json(read))
    });
    if told == "refused" {
        told
    } else {
        format!("{enum_name}{told}")
    }
}

/// A variant reads and writes a flattened member at its own level, as serde does, under every
/// tagging that builds one: a map alone, an optional map alone, a struct beside a member of the
/// variant's own, that member under its alias too, a member that names a function after a struct,
/// a struct beside the tag, and a map of the enum's own parameter.
#[test]
fn a_variant_reads_and_writes_a_flattened_member_in_kotlin_as_serde_does() {
    let mut rows: Vec<(&str, &str, String)> = Vec::new();
    rows.extend(GAFFING.map(|json| {
        let told = variant_read::<Gaffing>("Gaffing", json);
        (json, "GaffingSerializer", told)
    }));
    rows.extend(BOWLINING.map(|json| {
        let told = variant_read::<Bowlining>("Bowlining", json);
        (json, "BowliningSerializer", told)
    }));
    rows.extend(SWAYING.map(|json| {
        let told = variant_read::<Swaying>("Swaying", json);
        (json, "SwayingSerializer", told)
    }));
    rows.extend(WHELPING.map(|json| {
        let told = variant_read::<Whelping>("Whelping", json);
        (json, "WhelpingSerializer", told)
    }));
    rows.extend(BROACHING.map(|json| {
        let told = variant_read::<Broaching<u8>>("Broaching", json);
        (json, "BroachingSerializer(serializer<UByte>())", told)
    }));
    let mut expected = String::new();
    let mut main = String::from(VARIANT_READ);
    main.push_str("fun main() {\n");
    for (json, serializer, told) in &rows {
        let _ = writeln!(expected, "{json}: {told}");
        let _ = writeln!(main, "    variant({serializer}, \"\"\"{json}\"\"\")");
    }
    main.push_str("}\n");
    assert_eq!(
        expected,
        "{\"Counted\":{}}: GaffingCounted {\"Counted\":{}}\n\
         {\"Counted\":{\"north\":2}}: GaffingCounted {\"Counted\":{\"north\":2}}\n\
         {\"Counted\":{\"north\":\"x\"}}: refused\n\
         {\"Spare\":{}}: GaffingSpare {\"Spare\":{}}\n\
         {\"Spare\":{\"north\":2}}: GaffingSpare {\"Spare\":{\"north\":2}}\n\
         {\"Spare\":{\"north\":\"x\"}}: GaffingSpare {\"Spare\":{}}\n\
         {\"Labelled\":{\"label\":\"n\",\"note\":\"a\"}}: GaffingLabelled \
         {\"Labelled\":{\"label\":\"n\",\"note\":\"a\"}}\n\
         {\"Labelled\":{\"note\":\"a\",\"label\":\"n\",\"north\":2}}: GaffingLabelled \
         {\"Labelled\":{\"label\":\"n\",\"note\":\"a\"}}\n\
         {\"Labelled\":{\"label\":\"n\",\"remark\":\"a\"}}: GaffingLabelled \
         {\"Labelled\":{\"label\":\"n\",\"note\":\"a\"}}\n\
         {\"Labelled\":{\"label\":\"n\",\"note\":\"a\",\"remark\":\"b\"}}: refused\n\
         {\"Labelled\":{\"note\":\"a\"}}: refused\n\
         {\"kind\":\"Counted\",\"held\":{}}: BowliningCounted {\"kind\":\"Counted\",\"held\":{}}\n\
         {\"kind\":\"Counted\",\"held\":{\"north\":2}}: BowliningCounted \
         {\"kind\":\"Counted\",\"held\":{\"north\":2}}\n\
         {\"kind\":\"Counted\",\"held\":{\"north\":\"x\"}}: refused\n\
         {\"kind\":\"Spare\",\"held\":{\"north\":2}}: BowliningSpare \
         {\"kind\":\"Spare\",\"held\":{\"north\":2}}\n\
         {\"kind\":\"Spare\",\"held\":{\"north\":\"x\"}}: BowliningSpare \
         {\"kind\":\"Spare\",\"held\":{}}\n\
         {\"kind\":\"Labelled\",\"held\":{\"note\":\"a\",\"label\":\"n\"}}: BowliningLabelled \
         {\"kind\":\"Labelled\",\"held\":{\"label\":\"n\",\"note\":\"a\"}}\n\
         {\"kind\":\"Labelled\",\"held\":{\"note\":\"a\"}}: refused\n\
         {\"held\":{\"label\":\"n\",\"north\":2},\"kind\":\"Tallied\"}: BowliningTallied \
         {\"kind\":\"Tallied\",\"held\":{\"label\":\"n\",\"keys\":[\"north\"]}}\n\
         {\"kind\":\"Tallied\",\"held\":{\"north\":2}}: refused\n\
         {\"note\":\"a\",\"label\":\"n\"}: SwayingAnnotated {\"label\":\"n\",\"note\":\"a\"}\n\
         {\"north\":2}: SwayingCounted {\"north\":2}\n\
         {\"north\":\"x\"}: SwayingSpare {}\n\
         {}: SwayingCounted {}\n\
         {\"label\":\"n\"}: SwayingSpare {}\n\
         {\"label\":\"n\",\"note\":\"a\",\"north\":2}: SwayingAnnotated \
         {\"label\":\"n\",\"note\":\"a\"}\n\
         {\"kind\":\"Labelled\",\"label\":\"n\",\"note\":\"a\"}: WhelpingLabelled \
         {\"kind\":\"Labelled\",\"label\":\"n\",\"note\":\"a\"}\n\
         {\"note\":\"a\",\"label\":\"n\",\"kind\":\"Labelled\"}: WhelpingLabelled \
         {\"kind\":\"Labelled\",\"label\":\"n\",\"note\":\"a\"}\n\
         {\"kind\":\"Hoisted\",\"label\":\"n\"}: WhelpingHoisted \
         {\"kind\":\"Hoisted\",\"label\":\"n\"}\n\
         {\"label\":\"n\",\"north\":2,\"kind\":\"Hoisted\"}: WhelpingHoisted \
         {\"kind\":\"Hoisted\",\"label\":\"n\"}\n\
         {\"kind\":\"Hoisted\"}: refused\n\
         {\"Counted\":{}}: BroachingCounted {\"Counted\":{}}\n\
         {\"Counted\":{\"north\":2}}: BroachingCounted {\"Counted\":{\"north\":2}}\n\
         {\"Counted\":{\"north\":\"x\"}}: refused\n",
        "serde no longer reads each payload as it did"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        labelling_kotlin::kotlin_definition(),
        bunting_kotlin::kotlin_definition(),
        gaffing_kotlin::kotlin_definition(),
        bowlining_kotlin::kotlin_definition(),
        swaying_kotlin::kotlin_definition(),
        whelping_kotlin::kotlin_definition(),
        broaching_kotlin::kotlin_definition(),
        FUNCTIONS.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not the read above");
}
