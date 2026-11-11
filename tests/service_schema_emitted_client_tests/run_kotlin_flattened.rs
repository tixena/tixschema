//! A map flattened as the one member of its struct, and a flattened struct beside a member of its
//! own, read and written by the emitted Kotlin types compiled with `-Werror`, and an optional
//! member beside a flattened one, beside its twin under `nullable`: each payload is written back as
//! the serde read writes it back. A member flattened after a struct, or beside a member of its
//! holder's own, is handed the keys serde hands it, and an externally tagged enum the one entry
//! serde hands it, and is written as serde writes it, a unit variant as its name over `null`, the
//! one value its entry is read over, alone and flattened. An optional map, the one member, is read as `null` where serde reads `None`, and a map keyed by a
//! plain enum under the names serde writes its variants under. An optional struct with a flattened
//! member of its own is read wherever the struct reads, and as `null` wherever serde reads `None`,
//! and one that takes a key is `null` where the key is absent and refuses a key that does not read.
//! An optional type the emitter recorded no keys for is read as the first where its serializer
//! declares no property, and as the second where it declares some. An optional enum under a tag key
//! is `null` where neither its tag nor its content key is there, and is read where one is, so one
//! that does not read refuses the payload, as TypeScript and Zod refuse it. A variant reads and
//! writes a flattened member at its own level, under every tagging. An optional member that names a
//! function is read from what the function answers, as the Zod schema of its holder reads it. A
//! struct under its own tag is written with the tag ahead of its members, alone and flattened, as
//! serde writes it, and refuses a payload with no tag or another value under it, as TypeScript and
//! Zod refuse it, where serde reads it. A key no member reads is refused, as Zod refuses it, where
//! serde reads past it.

#![cfg(feature = "kotlin")]

use core::fmt::Debug;
use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::corrected_options_models::{
    CORRECTED, OPEN_CORRECTED, frapped_kotlin, marled_kotlin, moused_kotlin, parcelled_kotlin,
    reeved_kotlin, seized_kotlin, served_kotlin, snaked_kotlin, tholepin_kotlin, wormed_kotlin,
};
use super::flattened_models::{
    BELTING, BOWLINING, BROACHING, Belting, Bending, Bowlining, Brailing, Broaching, CHAFING,
    CHOCKED, CLEATING, Cathead, Chafing, Cleating, CorrectedLocker, DRESSING, DRESSING_TAGS,
    Dipping, Dressing, ENSIGN, EmptiedLocker, FLAG, FLAG_TAGS, FRAYING, Flag, Fraying, GAFFING,
    GONFALON, Gaffing, Gammoning, Gonfalon, HEAVING, HITCHING, Heaving, Hitching, Knotting, LOCKER,
    Lanyarding, Luffing, MARLINE, Netting, PAYLOAD, PENNANT_TAGS, Pennant, Rousing, SCARFING,
    SCUDDING, SEMAPHORE, SNUBBED, STOWING, STROPPING, STRUCK, SWAYING, Scarfing, Scudding, Seizing,
    Semaphore, Snubbing, Splicing, Stowing, Striking, Stropping, Swaying, Tacking, UNDER_AN_ALIAS,
    UNDER_TWO_NAMES, UNWRITTEN, WHELPING, Whelping, Whipping, Yawing, belting_kotlin,
    bending_kotlin, bowlining_kotlin, brailing_kotlin, broaching_kotlin, bunting_kotlin,
    cathead_kotlin, chafing_kotlin, chock_kotlin, cleating_kotlin, corrected_locker_kotlin,
    dipping_kotlin, dressing_kotlin, emptied_locker_kotlin, ensign_kotlin, flag_kotlin,
    fraying_kotlin, gaffing_kotlin, gammoning_kotlin, gonfalon_kotlin, hawser_kotlin,
    heaving_kotlin, hitching_kotlin, jibing_kotlin, knotting_kotlin, lanyarding_kotlin,
    luffing_kotlin, marline_kotlin, netting_kotlin, noting_kotlin, pennant_kotlin, quaying_kotlin,
    read_of, rousing_kotlin, scarfing_kotlin, scudding_kotlin, seizing_kotlin, semaphore_kotlin,
    sheeting_kotlin, snubber_kotlin, snubbing_kotlin, splicing_kotlin, stowing_kotlin,
    striking_kotlin, stropping_kotlin, swaying_kotlin, tacking_kotlin, ticketed_kotlin,
    unwritten_kotlin, whelping_kotlin, whipping_kotlin, yawing_kotlin,
};
use super::runtime::ran_kotlin;
#[cfg(all(feature = "zod", feature = "typescript"))]
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
use super::swift_flattened_options_models::{
    Crowning, Easing, FLAKING, FLAKING_ABSENT, FLAKING_MISTYPED, Fidding, Flaking, Gasketing,
    HEELING, Heeling, LAPPING, LAPPING_ABSENT, LAPPING_MISTYPED, LOWERING, Lowering, PITCHING,
    Pitching, Ratline, SQUARING, Squaring, Thimble, becket_kotlin, crowning_kotlin, easing_kotlin,
    fidding_kotlin, flaking_kotlin, gasketing_kotlin, grommet_kotlin, heeling_kotlin,
    lowering_kotlin, nippering_kotlin, pitching_kotlin, ratline_kotlin, shackle_kotlin,
    squaring_kotlin, tabling_kotlin, thimble_kotlin, toggle_kotlin,
};
use super::tuple_variants_models::labelling_kotlin;
use super::unrecorded_options_models::{
    CLAPPING, CLAPPING_READ_BACK, Clapping, GROMMETING, GROMMETING_READ_BACK, Grommeting,
    READ_BACK, UNRECORDED, bight_kotlin, bridled_kotlin, bridling_kotlin, clapped_kotlin,
    clapping_kotlin, crowned_kotlin, earing_kotlin, fiddling_kotlin, gromet_kotlin,
    grommeting_kotlin, hitched_kotlin, slipped_kotlin, slipping_kotlin, snagging_kotlin,
};
#[cfg(all(feature = "zod", feature = "typescript"))]
use super::unrecorded_options_models::{Earing, Gromet};

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

fun nullWhereNoted(payload: JsonElement): JsonElement =
    if (payload is JsonObject && "note" in payload) JsonNull else payload

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
    handed("a struct after one of other keys", Stropping.serializer(), """STROPPING""") {
        Json.encodeToString(Stropping.serializer(), it)
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

/// What prints a row of a holder of a flattened `Option` of an enum: the holder read and written
/// back, or the class and the first line of what its read throws.
const TOLD_OR_REFUSED: &str = r#"
fun <T> told(name: String, serializer: KSerializer<T>, payload: String) {
    val told = try {
        Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))
    } catch (refused: Exception) {
        "refused: ${refused::class.simpleName}: ${(refused.message ?: "").lines().first()}"
    }
    println("$name: $told")
}
"#;

/// What an externally tagged enum, then a holder of a flattened `Option` of it, is read from: a
/// unit variant's entry over a number and over `null`, its bare name and a variant's entry, then
/// the unit variant's entry over a number and over `null` beside the holder's own member, and no
/// entry.
const UNIT_ENTRIES: [(&str, &str); 7] = [
    ("Thimble", r#"{"Calm":1}"#),
    ("Thimble", r#"{"Calm":null}"#),
    ("Thimble", r#""Calm""#),
    ("Thimble", r#"{"Round":{"turns":2}}"#),
    ("Easing", r#"{"Calm":1,"note":"n"}"#),
    ("Easing", r#"{"Calm":null,"note":"n"}"#),
    ("Easing", r#"{"note":"n"}"#),
];

/// What each payload of a [`TagRows`] holds under the tag.
const TAGS: [&str; 4] = ["its tag", "no tag", "another tag", "a tag of another kind"];

/// What prints a [`Stowing`] read from each payload and written back, or the refusal of its read.
const UNDER_TWO_NAMES_READ: &str = r#"
fun stowing(at: Int, payload: String) {
    val told = try {
        Json.encodeToString(Stowing.serializer(), Json.decodeFromString(Stowing.serializer(), payload))
    } catch (refused: SerializationException) {
        "refused: " + refused.message
    }
    println("$at: $told")
}
"#;

/// What prints each holder of an enum under a tag and a content key read from each payload: the
/// keys its function was handed or the struct after it, and the holder written back, or the
/// refusal of its read.
const TAG_AND_CONTENT_READ: &str = r#"
fun <T> held(at: Int, serializer: KSerializer<T>, payload: String, shown: (T) -> String) {
    val told = try {
        val read = Json.decodeFromString(serializer, payload)
        "${shown(read)} ${Json.encodeToString(serializer, read)}"
    } catch (refused: SerializationException) {
        "refused: " + refused.message
    }
    println("$at: $told")
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

/// A holder of a flattened `Option` of an enum, a payload, and what serde writes back of it.
struct OptionRow {
    holder: &'static str,
    json: &'static str,
    serde: String,
}

/// A struct under its own tag, which writes `named` under it, read from the payloads of
/// [`FLAG_TAGS`] written for it, by `serializer` in Kotlin and by `written` in serde, which writes
/// `back` of every one.
struct TagRows {
    back: &'static str,
    name: &'static str,
    named: &'static str,
    payloads: [String; 4],
    serializer: &'static str,
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
/// keys the struct did not take, a struct with a member serde never writes among them, and every
/// key after a struct that takes none.
#[test]
fn a_member_flattened_after_a_struct_reads_in_kotlin_the_keys_serde_hands_it() {
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
            "a struct after one of other keys: {}",
            read_of::<Stropping>(STROPPING)
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
         a struct after one of other keys: {\"label\":\"n\",\"note\":\"a\"}\n"
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
        stropping_kotlin::kotlin_definition(),
        FUNCTIONS.to_owned(),
        HANDED_ROW.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A struct under its own tag, alone, with a type parameter and with a member under its alias,
/// and flattened ahead of a member of its holder's own, each read from a payload with the tag
/// after the members, is written back with the tag ahead of them, as serde writes it.
#[test]
fn a_struct_under_its_own_tag_is_written_in_kotlin_with_the_tag_ahead_as_serde_writes_it() {
    let rows = [
        (
            "alone",
            "Flag.serializer()",
            FLAG,
            written_back::<Flag>(FLAG),
        ),
        (
            "with a type parameter",
            "GonfalonSerializer(serializer<String>())",
            GONFALON,
            written_back::<Gonfalon<String>>(GONFALON),
        ),
        (
            "with a member under its alias",
            "Semaphore.serializer()",
            SEMAPHORE,
            written_back::<Semaphore>(SEMAPHORE),
        ),
        (
            "flattened ahead of a member",
            "Dressing.serializer()",
            DRESSING,
            written_back::<Dressing>(DRESSING),
        ),
    ];
    let mut expected = String::new();
    let mut main = String::from("fun main() {\n");
    for (name, serializer, payload, serde) in &rows {
        let _ = writeln!(expected, "{name}: {serde}");
        let _ = writeln!(
            main,
            "    row(\"{name}\", {serializer}, \"\"\"{payload}\"\"\")"
        );
    }
    main.push_str("}\n");
    assert_eq!(
        expected,
        "alone: {\"kind\":\"Flag\",\"label\":\"n\"}\n\
         with a type parameter: {\"kind\":\"Gonfalon\",\"label\":\"n\"}\n\
         with a member under its alias: {\"kind\":\"Semaphore\",\"label\":\"n\"}\n\
         flattened ahead of a member: {\"kind\":\"Ensign\",\"label\":\"n\",\"north\":2}\n"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        flag_kotlin::kotlin_definition(),
        gonfalon_kotlin::kotlin_definition(),
        semaphore_kotlin::kotlin_definition(),
        ensign_kotlin::kotlin_definition(),
        dressing_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// A struct under its own tag alone, with a type parameter, with a member under its alias, with a
/// flattened member of its own and flattened ahead of a member of its holder's own.
fn tag_rows() -> [TagRows; 5] {
    let renamed =
        |named: &str| FLAG_TAGS.map(|row| row.replace("\"Flag\"", &format!("\"{named}\"")));
    [
        TagRows {
            back: r#"{"kind":"Flag","label":"n"}"#,
            name: "alone",
            named: "Flag",
            payloads: FLAG_TAGS.map(str::to_owned),
            serializer: "Flag.serializer()",
            written: written_back::<Flag>,
        },
        TagRows {
            back: r#"{"kind":"Gonfalon","label":"n"}"#,
            name: "with a type parameter",
            named: "Gonfalon",
            payloads: renamed("Gonfalon"),
            serializer: "GonfalonSerializer(serializer<String>())",
            written: written_back::<Gonfalon<String>>,
        },
        TagRows {
            back: r#"{"kind":"Semaphore","label":"n"}"#,
            name: "with a member under its alias",
            named: "Semaphore",
            payloads: renamed("Semaphore"),
            serializer: "Semaphore.serializer()",
            written: written_back::<Semaphore>,
        },
        TagRows {
            back: r#"{"kind":"Pennant","label":"n","note":"a"}"#,
            name: "with a flattened member",
            named: "Pennant",
            payloads: PENNANT_TAGS.map(str::to_owned),
            serializer: "Pennant.serializer()",
            written: written_back::<Pennant>,
        },
        TagRows {
            back: r#"{"kind":"Ensign","label":"n","north":2}"#,
            name: "flattened ahead of a member",
            named: "Ensign",
            payloads: DRESSING_TAGS.map(str::to_owned),
            serializer: "Dressing.serializer()",
            written: written_back::<Dressing>,
        },
    ]
}

/// Each struct of [`tag_rows`] reads a payload under its own tag as serde reads it, and refuses one
/// with no tag, or with another value under it, as TypeScript and Zod refuse it
/// (`kind: z.literal("Flag")`), where serde reads each and writes the struct's name back.
#[test]
fn a_struct_under_its_own_tag_refuses_in_kotlin_every_tag_zod_refuses() {
    let mut expected = String::new();
    let mut main = String::from(TOLD_OR_REFUSED);
    main.push_str("fun main() {\n");
    for rows in tag_rows() {
        let name = rows.name;
        assert_eq!(
            rows.payloads
                .each_ref()
                .map(|payload| (rows.written)(payload)),
            [rows.back; 4],
            "serde reads every payload {name} and writes the struct's name back"
        );
        for (tag, payload) in TAGS.iter().zip(&rows.payloads) {
            let _ = writeln!(
                main,
                "    told(\"{name}, {tag}\", {}, \"\"\"{payload}\"\"\")",
                rows.serializer
            );
        }
        let named = rows.named;
        let _ = write!(
            expected,
            "{name}, {}: {}\n\
             {name}, {}: refused: SerializationException: missing field `kind` at path: $\n\
             {name}, {}: refused: SerializationException: expected tag \"{named}\", found \
             \"Other\" at path: $\n\
             {name}, {}: refused: SerializationException: expected tag \"{named}\", found 5 at \
             path: $\n",
            TAGS[0], rows.back, TAGS[1], TAGS[2], TAGS[3]
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        flag_kotlin::kotlin_definition(),
        gonfalon_kotlin::kotlin_definition(),
        semaphore_kotlin::kotlin_definition(),
        labelling_kotlin::kotlin_definition(),
        pennant_kotlin::kotlin_definition(),
        ensign_kotlin::kotlin_definition(),
        dressing_kotlin::kotlin_definition(),
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

/// A flattened externally tagged enum is handed the one entry under the name a variant is read
/// under, and every member after it the object without that entry, as serde hands them: beside a
/// member of its holder's own, ahead of a function and of a struct, under a `rename`, the enum's
/// `rename_all` and an `alias`, and optional. A payload with no such entry is refused, and an
/// optional enum reads no value. A key no member reads is refused, as Zod and serde refuse it, and
/// so are entries under two names, as TypeScript and Zod refuse them, where serde reads the first
/// beside a member that reads every key.
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
         2: refused\n\
         3: refused\n\
         4: \"Calm\" a\n\
         5: {\"Held\":\"x\"} a\n\
         6: refused\n\
         7: {\"Bay\":{\"label\":\"n\"}} a\n\
         8: refused\n\
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
    let under_two_names = [HEAVING[1], HEAVING[2], HEAVING[3]];
    let mut kotlin = String::new();
    for (at, row) in rows.iter().enumerate() {
        let told = if under_two_names.contains(&row.json) {
            "refused".to_owned()
        } else {
            (row.told)(row.json)
        };
        let _ = writeln!(kotlin, "{at}: {told}");
    }
    let Some(printed) = ran_kotlin(&entry_source(main)) else {
        return;
    };
    assert_eq!(printed, kotlin);
}

/// A flattened externally tagged enum with entries under two of its names is refused, as
/// TypeScript, Zod and serde refuse it, under two variants' names in each order and under a
/// variant's name beside its alias, naming each variant once, in the order the enum declares
/// them. One entry, a unit variant's over `null`, an entry under an alias and no
/// entry are read and refused as serde reads and refuses them.
#[test]
fn a_flattened_enum_with_entries_under_two_names_is_refused_in_kotlin() {
    let refused = |variants: &str| {
        format!(
            "refused: more than one variant of enum Tacking found in flattened data: {variants}"
        )
    };
    let mut rows: Vec<(&str, String)> = UNDER_TWO_NAMES
        .iter()
        .zip([refused("Bay, Dock"), refused("Bay, Dock"), refused("Dock")])
        .map(|(&json, told)| {
            assert!(
                serde_json::from_str::<Stowing>(json).is_err(),
                "serde reads an entry of {json}"
            );
            (json, told)
        })
        .collect();
    for json in [STOWING[1], STOWING[4], UNDER_AN_ALIAS[0], STOWING[6]] {
        let told = serde_json::from_str::<Stowing>(json).map_or_else(
            |refusal| {
                let said = refusal.to_string();
                let (unplaced, _position) = said.split_once(" at line").unwrap_or((&said, ""));
                format!("refused: {unplaced}")
            },
            |read| serde_json::to_string(&read).unwrap(),
        );
        rows.push((json, told));
    }
    let mut expected = String::new();
    let mut main = format!("{UNDER_TWO_NAMES_READ}\nfun main() {{\n");
    for (at, (json, told)) in rows.iter().enumerate() {
        let _ = writeln!(expected, "{at}: {told}");
        let _ = writeln!(main, "    stowing({at}, \"\"\"{json}\"\"\")");
    }
    main.push_str("}\n");
    assert_eq!(
        expected,
        "0: refused: more than one variant of enum Tacking found in flattened data: Bay, Dock\n\
         1: refused: more than one variant of enum Tacking found in flattened data: Bay, Dock\n\
         2: refused: more than one variant of enum Tacking found in flattened data: Dock\n\
         3: {\"note\":\"a\",\"Bay\":{\"label\":\"n\"}}\n\
         4: {\"note\":\"a\",\"Calm\":null}\n\
         5: {\"note\":\"a\",\"Dock\":{\"label\":\"s\"}}\n\
         6: refused: no variant of enum Tacking found in flattened data\n"
    );
    let Some(printed) = ran_kotlin(&entry_source(main)) else {
        return;
    };
    // The Kotlin read follows serde's words with the path of the holder it refuses.
    let placed: String = expected
        .lines()
        .map(|row| {
            if row.contains(": refused: ") {
                format!("{row} at path: $\n")
            } else {
                format!("{row}\n")
            }
        })
        .collect();
    assert_eq!(printed, placed);
}

/// An enum under a tag and a content key flattened into its holder takes both keys, as serde reads
/// it as a struct of the two: a member that names a function flattened after it is handed every
/// other key, and a struct flattened after it reads the whole object. Each holder is written back
/// as serde writes it back, and a key no member reads is refused, as Zod and serde refuse it.
#[test]
fn an_enum_under_a_tag_and_a_content_key_takes_both_keys_in_kotlin() {
    let chafing: Chafing = serde_json::from_str(CHAFING).unwrap();
    let fraying: Fraying = serde_json::from_str(FRAYING[0]).unwrap();
    assert!(
        serde_json::from_str::<Fraying>(FRAYING[1]).is_err(),
        "serde reads past a key no member reads"
    );
    let expected = format!(
        "0: {} {}\n1: {} {}\n2: refused: Encountered an unknown key 'north' at path: $\n",
        serde_json::to_string(&chafing.tally).unwrap(),
        serde_json::to_string(&chafing).unwrap(),
        serde_json::to_string(&fraying.labelling).unwrap(),
        serde_json::to_string(&fraying).unwrap(),
    );
    assert_eq!(
        expected,
        "0: {\"keys\":[\"north\"]} {\"kind\":\"Rope\",\"held\":{\"label\":\"n\"},\"keys\":[\"north\"]}\n\
         1: {\"label\":\"m\"} {\"kind\":\"Rope\",\"held\":{\"label\":\"n\"},\"label\":\"m\"}\n\
         2: refused: Encountered an unknown key 'north' at path: $\n"
    );
    let main = format!(
        "{TAG_AND_CONTENT_READ}\nfun main() {{\n    held(0, Chafing.serializer(), \"\"\"{CHAFING}\"\"\") \
         {{ Json.encodeToString(Bunting.serializer(), it.tally) }}\n    held(1, \
         Fraying.serializer(), \"\"\"{}\"\"\") {{ Json.encodeToString(Labelling.serializer(), \
         it.labelling) }}\n    held(2, Fraying.serializer(), \"\"\"{}\"\"\") {{ \
         Json.encodeToString(Labelling.serializer(), it.labelling) }}\n}}\n",
        FRAYING[0], FRAYING[1]
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        labelling_kotlin::kotlin_definition(),
        bunting_kotlin::kotlin_definition(),
        snubber_kotlin::kotlin_definition(),
        chafing_kotlin::kotlin_definition(),
        fraying_kotlin::kotlin_definition(),
        FUNCTIONS.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
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
/// where it holds any other key, as serde refuses it: the enum takes its own entry, so the holder
/// refuses a key neither declares.
#[test]
fn an_enum_declared_below_its_holder_is_read_in_kotlin_from_the_object_it_is_handed() {
    let beside = STOWING[2];
    let serde_read = [STOWING[0], beside].map(|payload| {
        told_by_serde(payload, |read: &Luffing| {
            format!("{} {}", json(&read.quaying), read.note)
        })
    });
    assert_eq!(serde_read, ["{\"Bay\":{\"label\":\"n\"}} a", "refused"]);
    let main = format!(
        "fun main() {{\n\
         val read = Json.decodeFromString(Luffing.serializer(), \"\"\"{}\"\"\")\n\
         println(\"${{Json.encodeToString(serializer<Quaying>(), read.quaying)}} ${{read.note}}\")\n\
         try {{ Json.decodeFromString(Luffing.serializer(), \"\"\"{beside}\"\"\"); println(\"read\") }} \
         catch (refused: SerializationException) {{ println(refused.message) }}\n\
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
        format!("{}\nexpected value at path: $\n", serde_read[0])
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

/// An optional map, the one member, under a function is read and written back as serde reads and
/// writes it: from what the function answers, an entry that does not read as a count refused, and
/// `null` read as `null`.
#[test]
fn an_optional_flattened_map_under_a_function_reads_and_writes_in_kotlin_as_serde_does() {
    let rows = [
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
        "CorrectedLocker {}: {} {}\n\
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
/// `rename_all` among them, and a key no variant is written under refused.
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
         Snubbing {\"bow\":2}: {\"bow\":2} {\"bow\":2}\n\
         Snubbing {\"open_water\":2}: {\"open_water\":2} {\"open_water\":2}\n\
         Snubbing {\"OpenWater\":2}: refused\n"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        chock_kotlin::kotlin_definition(),
        hawser_kotlin::kotlin_definition(),
        cathead_kotlin::kotlin_definition(),
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

/// Beside the `nonstrict` `Gasketing`, an optional struct that takes a key is read where the key is
/// there and reads, and is `null` where it is absent and where it does not read, as serde and Zod
/// read it. The holder is written back as serde writes it back.
#[test]
fn an_optional_struct_beside_a_nonstrict_holder_reads_in_kotlin_as_serde_does() {
    let rows = [LAPPING, LAPPING_ABSENT, LAPPING_MISTYPED];
    let serde_told = rows.map(|json| {
        told_by_serde(json, |read: &Gasketing| {
            format!("{} {}", self::json(&read.tabling), self::json(read))
        })
    });
    assert_eq!(
        serde_told,
        [
            "{\"label\":\"north\"} {\"note\":\"n\",\"label\":\"north\"}",
            "null {\"note\":\"n\"}",
            "null {\"note\":\"n\"}",
        ],
        "serde no longer reads each payload as it did"
    );
    let mut expected = String::new();
    let mut main = String::from(ENTRY_READ);
    main.push_str("fun main() {\n");
    for (at, (json, told)) in rows.iter().zip(&serde_told).enumerate() {
        let _ = writeln!(expected, "{at}: {told}");
        let _ = writeln!(
            main,
            "    entry(\"{at}\", Gasketing.serializer(), \"\"\"{json}\"\"\") {{ \
             \"${{Json.encodeToString(serializer<Tabling?>(), it.tabling)}} \
             ${{Json.encodeToString(Gasketing.serializer(), it)}}\" }}"
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        tabling_kotlin::kotlin_definition(),
        gasketing_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not serde's");
}

/// An optional member whose type the emitter recorded no keys for and whose serializer is written
/// by hand, a struct declared below its holder with a flattened member of its own, an untagged
/// enum and an enum under a tag alone that names a function on itself, is read wherever its read
/// succeeds and is `null` wherever it throws, as serde reads it, and is written back as serde
/// writes it back. An enum under a tag key the emitter wrote is `null` where neither its tag nor
/// its content key is there, and its tag beside no member refuses the payload beside a strict
/// holder, as TypeScript and Zod refuse it, where serde reads `None`, and reads `null` beside a
/// `nonstrict` one, as serde and Zod read it. A struct declared below
/// its holder whose serializer the plugin writes is looked for by the keys its descriptor names:
/// `null` where none is there, and a key that does not read refuses the payload, as TypeScript and
/// Zod refuse it, where serde reads `None`.
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
    let clapping_told =
        CLAPPING.map(|json| told_by_serde(json, |read: &Clapping| self::json(read)));
    assert_eq!(
        clapping_told, CLAPPING_READ_BACK,
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
        .chain(looked_for.map(|json| ("Ratline", json)))
        .chain(CLAPPING.map(|json| ("Clapping", json)));
    let mut in_kotlin: Vec<&str> = READ_BACK
        .into_iter()
        .chain([ratline_told[0].as_str(), ratline_told[1].as_str()])
        .chain(["refused"])
        .chain(CLAPPING_READ_BACK)
        .collect();
    // TypeScript and Zod refuse each: the tag of `Bridled` beside no member, and the key
    // `Nippering` reads held as a number. The tag of `Hitched` beside no member reads `null` beside
    // the `nonstrict` `Snagging`, as serde and Zod read it.
    for (at, refused) in [
        (
            10,
            "refused: SerializationException: missing field `held` at path: $",
        ),
        (
            16,
            "refused: SerializationException: Expected quotation mark '\"', but had '5' instead at \
             path: $",
        ),
    ] {
        in_kotlin[at] = refused;
    }
    let mut expected = String::new();
    let mut main = String::from(TOLD_OR_REFUSED);
    main.push_str("fun main() {\n");
    for (at, ((holder, json), told)) in rows.zip(in_kotlin).enumerate() {
        let _ = writeln!(expected, "{at}: {told}");
        let _ = writeln!(
            main,
            "    told(\"{at}\", {holder}.serializer(), \"\"\"{json}\"\"\")"
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
        clapped_kotlin::kotlin_definition(),
        clapping_kotlin::kotlin_definition(),
        FUNCTIONS.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not the read above");
}

/// An optional member whose type is declared below its holder, with a flattened member of its own,
/// is refused where its type reads with no judgement and breaks a rule, with the rule's message, as
/// serde refuses it, and is read where it keeps it. It is `null` where no key of it is there and
/// where a key of it does not read, as serde reads `None`.
#[test]
fn an_optional_type_declared_below_that_breaks_a_rule_is_refused_in_kotlin_as_serde_refuses_it() {
    let serde_told = GROMMETING.map(|json| {
        serde_json::from_str::<Grommeting>(json)
            .map_or_else(|refused| refused.to_string(), |read| self::json(&read))
    });
    assert_eq!(
        serde_told, GROMMETING_READ_BACK,
        "serde no longer reads each payload as it did"
    );
    let mut expected = String::new();
    let mut main = String::from(TOLD_OR_REFUSED);
    main.push_str("fun main() {\n");
    for (at, (json, serde)) in GROMMETING.iter().zip(GROMMETING_READ_BACK).enumerate() {
        let in_kotlin = if serde.starts_with('{') {
            serde.to_owned()
        } else {
            format!("refused: SerializationException: {serde}")
        };
        let _ = writeln!(expected, "{at}: {in_kotlin}");
        let _ = writeln!(
            main,
            "    told(\"{at}\", Grommeting.serializer(), \"\"\"{json}\"\"\")"
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        earing_kotlin::kotlin_definition(),
        grommeting_kotlin::kotlin_definition(),
        gromet_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not the read above");
}

/// The README's sentence on a broken rule held below its holder declares the struct the run reads,
/// and shows payloads and the message the run measures.
#[test]
fn the_readme_shows_the_broken_rule_held_below_the_run_measures() {
    let readme = include_str!("../../docs/guide/kotlin.md");
    for shown in [
        "`struct Gromet { #[serde(flatten)] earing: Earing, #[model_schema_prop(minLength = 2)] \
         note: String }`"
            .to_owned(),
        format!(
            "`{}` is refused with `SerializationException: {}`",
            GROMMETING[0], GROMMETING_READ_BACK[0]
        ),
        format!("`{}` reads `null`", GROMMETING[3]),
    ] {
        assert!(readme.contains(&shown), "not in the README: {shown}");
    }
}

/// The generated Zod schema of [`Grommeting`], `nonstrict` as the holder of a type declared below
/// it that reads every key must be, reads every payload: the branch of the absent member strips
/// that member's keys, so it reads the payload whose [`Gromet`] breaks its rule, which serde and the
/// Kotlin read refuse.
#[cfg(all(feature = "zod", feature = "typescript"))]
#[test]
fn zod_reads_every_payload_of_a_nonstrict_holder_of_an_option_declared_below() {
    const REQUIRED: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED) else {
        stand_down_modules(
            REQUIRED,
            "the generated Zod schema of a broken rule held below",
        );
        return;
    };
    let mut module = [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        Earing::ts_definition(),
        Earing::zod_schema(),
        Grommeting::ts_definition(),
        Grommeting::zod_schema(),
        Gromet::ts_definition(),
        Gromet::zod_schema(),
    ]
    .join("\n\n");
    module.push_str("\n\nconst told = [];\n");
    for json in GROMMETING {
        let _ = writeln!(
            module,
            "told.push(Grommeting$Schema.safeParse(JSON.parse({})).success ? \"read\" : \
             \"refused\");",
            serde_json::to_string(json).unwrap()
        );
    }
    module.push_str("console.log(JSON.stringify(told));\n");
    let Some(printed) = ran_with_modules(
        "grommeting",
        "TIXSCHEMA_NODE",
        "node",
        "grommeting.mts",
        &module,
        &modules,
    ) else {
        return;
    };
    let told: Vec<String> = serde_json::from_str(printed.trim()).unwrap();
    assert_eq!(
        told,
        ["read", "read", "read", "read"],
        "the Zod schema no longer answers each payload as it did"
    );
}

/// Beside a `nonstrict` holder, an optional member that names a function is `null` exactly where
/// the function answers `null`, and is read from any other answer: each payload of [`CORRECTED`]
/// and [`OPEN_CORRECTED`] reads as serde and the Zod schema of its holder read it, written back
/// with its keys in order.
#[test]
fn an_optional_type_that_names_a_function_reads_in_kotlin_as_zod_does() {
    let mut expected = String::new();
    let mut main = String::from(ENTRY_READ);
    main.push_str("fun main() {\n");
    for (at, row) in CORRECTED.iter().chain(&OPEN_CORRECTED).enumerate() {
        let _ = writeln!(expected, "{at}: {}", row.zod);
        let _ = writeln!(
            main,
            "    entry(\"{at}\", {holder}.serializer(), \"\"\"{}\"\"\") {{ \
             Json.encodeToString(JsonObject(Json.encodeToJsonElement({holder}.serializer(), \
             it).jsonObject.toSortedMap())) }}",
            row.json,
            holder = row.holder
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        FUNCTIONS.to_owned(),
        tholepin_kotlin::kotlin_definition(),
        reeved_kotlin::kotlin_definition(),
        wormed_kotlin::kotlin_definition(),
        frapped_kotlin::kotlin_definition(),
        moused_kotlin::kotlin_definition(),
        parcelled_kotlin::kotlin_definition(),
        served_kotlin::kotlin_definition(),
        marled_kotlin::kotlin_definition(),
        snaked_kotlin::kotlin_definition(),
        seized_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not the Zod read");
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
/// tagging that builds one: a map alone, a struct beside a member of the variant's own, that member under its alias too, a member that names a function after a struct,
/// a struct beside the tag, and a map of the enum's own parameter. The enums whose variant flattens
/// a map are `nonstrict`, as the build asks of them, so a key no member of a variant reads is read
/// past, as serde and Zod read it.
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
         {\"kind\":\"Labelled\",\"held\":{\"note\":\"a\",\"label\":\"n\"}}: BowliningLabelled \
         {\"kind\":\"Labelled\",\"held\":{\"label\":\"n\",\"note\":\"a\"}}\n\
         {\"kind\":\"Labelled\",\"held\":{\"note\":\"a\"}}: refused\n\
         {\"held\":{\"label\":\"n\",\"north\":2},\"kind\":\"Tallied\"}: BowliningTallied \
         {\"kind\":\"Tallied\",\"held\":{\"label\":\"n\",\"keys\":[\"north\"]}}\n\
         {\"kind\":\"Tallied\",\"held\":{\"north\":2}}: refused\n\
         {\"note\":\"a\",\"label\":\"n\"}: SwayingAnnotated {\"label\":\"n\",\"note\":\"a\"}\n\
         {\"north\":2}: SwayingCounted {\"north\":2}\n\
         {\"north\":\"x\"}: refused\n\
         {}: SwayingCounted {}\n\
         {\"label\":\"n\"}: refused\n\
         {\"label\":\"n\",\"note\":\"a\",\"north\":2}: SwayingAnnotated \
         {\"label\":\"n\",\"note\":\"a\"}\n\
         {\"kind\":\"Labelled\",\"label\":\"n\",\"note\":\"a\"}: WhelpingLabelled \
         {\"kind\":\"Labelled\",\"label\":\"n\",\"note\":\"a\"}\n\
         {\"note\":\"a\",\"label\":\"n\",\"kind\":\"Labelled\"}: WhelpingLabelled \
         {\"kind\":\"Labelled\",\"label\":\"n\",\"note\":\"a\"}\n\
         {\"kind\":\"Hoisted\",\"label\":\"n\"}: WhelpingHoisted \
         {\"kind\":\"Hoisted\",\"label\":\"n\"}\n\
         {\"label\":\"n\",\"north\":2,\"kind\":\"Hoisted\"}: refused\n\
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

/// Each payload of each holder of a flattened `Option` of an enum, beside what serde writes back
/// of it: an enum under a tag alone, one under a tag and a content key, an untagged one and an
/// externally tagged one declared below its holder.
fn option_rows() -> Vec<OptionRow> {
    fn rows<T, const N: usize>(holder: &'static str, payloads: [&'static str; N]) -> Vec<OptionRow>
    where
        T: DeserializeOwned + Serialize,
    {
        payloads
            .into_iter()
            .map(|json| OptionRow {
                holder,
                json,
                serde: written_back::<T>(json),
            })
            .collect()
    }
    [
        rows::<Lowering, 6>("Lowering", LOWERING),
        rows::<Squaring, 6>("Squaring", SQUARING),
        rows::<Heeling, 3>("Heeling", HEELING),
        rows::<Pitching, 3>("Pitching", PITCHING),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// A flattened `Option` of an enum under a tag key the emitter wrote is looked for by its tag, then
/// its content key, in the whole object its holder reads, as TypeScript and Zod read it: `null`
/// where neither is there, and the enum read where one is, so a read that throws refuses the
/// payload beside the strict `Squaring`, and is `null` beside the `nonstrict` `Lowering`, as serde
/// and Zod read it.
#[test]
fn a_flattened_option_of_an_enum_under_a_tag_key_is_looked_for_by_its_tag_in_kotlin() {
    let looked_for = |kotlin: &str, prop: &str, read: &str, keys: &str| {
        let held = format!(
            "val {prop} = kotlinSchemaFlattened {{ if (obj.keys.any {{ it in setOf({keys}) }}) \
             {read} else null }};"
        );
        assert!(kotlin.contains(&held), "no `{held}` in: {kotlin}");
    };
    looked_for(
        &lowering_kotlin::kotlin_definition(),
        "grommet",
        "runCatching { lenient.decodeFromString(GrommetUnjudged, obj.toString()) }.getOrNull()",
        "\"kind\"",
    );
    looked_for(
        &squaring_kotlin::kotlin_definition(),
        "becket",
        "lenient.decodeFromString(BecketUnjudged, obj.toString())",
        "\"kind\", \"body\"",
    );
}

/// Each payload of each holder of a flattened `Option` of an enum reads in Kotlin as serde reads
/// it, but where the object holds the tag or the content key of an enum under a tag key and the
/// enum does not read it beside the strict `Squaring`: TypeScript and Zod refuse each of those, and
/// so does the Kotlin read, with what the enum's read throws, where serde reads `None`. Beside the
/// `nonstrict` `Lowering` each reads `null`, as serde and Zod read it. A variant's member with no tag
/// reads `null`, as serde reads it. An untagged enum and one declared below its holder, which have
/// no key to be looked for by, are `null` wherever their read throws, as serde reads `None`.
#[test]
fn a_flattened_option_of_an_enum_reads_in_kotlin_as_zod_does_where_its_keys_are_there() {
    let rows = option_rows();
    let serde_told: Vec<String> = rows
        .iter()
        .map(|row| format!("{}: {}", row.json, row.serde))
        .collect();
    assert_eq!(
        serde_told,
        [
            r#"{"note":"n"}: {"note":"n"}"#,
            r#"{"kind":"Round","note":"n","turns":2}: {"kind":"Round","turns":2,"note":"n"}"#,
            r#"{"kind":"Calm","note":"n"}: {"kind":"Calm","note":"n"}"#,
            r#"{"kind":"Round","note":"n","turns":"x"}: {"note":"n"}"#,
            r#"{"kind":"Gale","note":"n"}: {"note":"n"}"#,
            r#"{"note":"n","turns":2}: {"note":"n"}"#,
            r#"{"note":"n"}: {"note":"n"}"#,
            r#"{"body":{"turns":2},"kind":"Round","note":"n"}: {"kind":"Round","body":{"turns":2},"note":"n"}"#,
            r#"{"kind":"Calm","note":"n"}: {"kind":"Calm","note":"n"}"#,
            r#"{"body":{"turns":"x"},"kind":"Round","note":"n"}: {"note":"n"}"#,
            r#"{"kind":"Gale","note":"n"}: {"note":"n"}"#,
            r#"{"body":{"turns":2},"note":"n"}: {"note":"n"}"#,
            r#"{"note":"n"}: {"note":"n"}"#,
            r#"{"note":"n","turns":2}: {"note":"n","turns":2}"#,
            r#"{"note":"n","turns":"x"}: {"note":"n"}"#,
            r#"{"note":"n"}: {"note":"n"}"#,
            r#"{"Round":{"turns":2},"note":"n"}: {"note":"n","Round":{"turns":2}}"#,
            r#"{"Round":{"turns":"x"},"note":"n"}: {"note":"n"}"#,
        ],
        "serde no longer reads each payload as it did"
    );
    let mut in_kotlin: Vec<String> = rows.iter().map(|row| row.serde.clone()).collect();
    let mistyped = "SerializationException: invalid type: string \"x\", expected u32 at path:";
    let unnamed = "SerializationException: unknown tag Gale at path: $";
    for (at, refused) in [
        (9, format!("{mistyped} $")),
        (10, unnamed.to_owned()),
        (
            11,
            "SerializationException: missing field `kind` at path: $".to_owned(),
        ),
    ] {
        in_kotlin[at] = format!("refused: {refused}");
    }
    let mut expected = String::new();
    let mut main = String::from(TOLD_OR_REFUSED);
    main.push_str("fun main() {\n");
    for (at, (row, told)) in rows.iter().zip(&in_kotlin).enumerate() {
        let _ = writeln!(expected, "{at}: {told}");
        let _ = writeln!(
            main,
            "    told(\"{at}\", {}.serializer(), \"\"\"{}\"\"\")",
            row.holder, row.json
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        grommet_kotlin::kotlin_definition(),
        lowering_kotlin::kotlin_definition(),
        becket_kotlin::kotlin_definition(),
        squaring_kotlin::kotlin_definition(),
        toggle_kotlin::kotlin_definition(),
        heeling_kotlin::kotlin_definition(),
        pitching_kotlin::kotlin_definition(),
        shackle_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not the read above");
}

/// An externally tagged enum reads a unit variant's entry over `null` alone, as serde reads the
/// enum alone: the entry over any other value is refused, read alone and flattened, an optional
/// holder refusing the payload, as TypeScript, Zod, Dart and Swift refuse it, where serde reads
/// `None`. A bare name, an entry over `null` and a variant's entry read as they did.
#[test]
fn a_unit_variants_entry_reads_in_kotlin_over_null_alone() {
    let serde_told: Vec<String> = UNIT_ENTRIES
        .iter()
        .map(|&(holder, payload)| {
            if holder == "Thimble" {
                told_by_serde(payload, json::<Thimble>)
            } else {
                told_by_serde(payload, json::<Easing>)
            }
        })
        .collect();
    assert_eq!(
        serde_told,
        [
            "refused",
            r#""Calm""#,
            r#""Calm""#,
            r#"{"Round":{"turns":2}}"#,
            r#"{"note":"n"}"#,
            r#"{"note":"n","Calm":null}"#,
            r#"{"note":"n"}"#,
        ],
        "serde no longer reads each payload as it did"
    );
    let unit_over_a_number = serde_json::from_str::<Thimble>(UNIT_ENTRIES[0].1).unwrap_err();
    assert_eq!(
        unit_over_a_number.to_string(),
        "invalid type: integer `1`, expected unit at line 1 column 9",
        "serde no longer refuses a unit variant's entry over a number as it did"
    );
    let refused = "refused: SerializationException: Thimble: Calm takes no value at path:";
    let mut in_kotlin = serde_told;
    in_kotlin[0] = format!("{refused} $.Calm");
    // Flattened, the enum's refusal is placed at its holder, as serde places it.
    in_kotlin[4] = format!("{refused} $");
    let mut expected = String::new();
    let mut main = String::from(TOLD_OR_REFUSED);
    main.push_str("fun main() {\n");
    for (at, (&(holder, payload), told)) in UNIT_ENTRIES.iter().zip(&in_kotlin).enumerate() {
        let _ = writeln!(expected, "{at}: {told}");
        let _ = writeln!(
            main,
            "    told(\"{at}\", serializer<{holder}>(), \"\"\"{payload}\"\"\")"
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        thimble_kotlin::kotlin_definition(),
        easing_kotlin::kotlin_definition(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not the read above");
}
