//! A name serde reads beside the one it writes, read by the emitted Kotlin types compiled with
//! `-Werror`: a variant under its `alias`, under each tagging and in a plain enum, and a member
//! under its `alias`, in a struct the plugin reads, in one the emitter reads by hand, flattened,
//! and in a variant. Each payload is written back as serde writes it back, under the name serde
//! writes, and a member read under two of its names is refused, as serde refuses it.

#![cfg(all(feature = "kotlin", feature = "serde"))]

use core::fmt::Write as _;

use super::aliased_models::{
    CAREENING, Careening, FAIRLEADING, Fairleading, Fothering, Gybing, HAWSING, Hawsing, LULLING,
    Lulling, SCENDING, SURGING, SWELL, Scending, Surging, Swell, TIDING, Tiding, VEERING, Veering,
    careening_kotlin, pennon_kotlin, fairleading_kotlin, fothering_kotlin, gybing_kotlin,
    hawsing_kotlin, lulling_kotlin, scending_kotlin, surging_kotlin, swell_kotlin, tiding_kotlin,
    veering_kotlin, written_back,
};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read, written back, or that the read is refused with one of the
/// two exceptions the emitted reads throw.
const TOLD: &str = r#"
fun <T> row(name: String, serializer: KSerializer<T>, payload: String) {
    val told = try {
        Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))
    } catch (refused: SerializationException) {
        "refused"
    } catch (refused: IllegalStateException) {
        "refused"
    }
    println("$name $payload: $told")
}

fun unpadded(payload: JsonElement): JsonElement =
    if (payload is JsonPrimitive && payload.isString) JsonPrimitive(payload.content.trim()) else payload

fun withItsKeys(payload: JsonElement): JsonElement =
    if (payload is JsonObject) {
        JsonObject(mapOf("keys" to JsonArray(payload.keys.sorted().map { JsonPrimitive(it) })))
    } else {
        payload
    }
"#;

/// The type each payload is read as, in Kotlin, and what serde writes back of each.
fn rows() -> Vec<(&'static str, &'static str, String)> {
    let mut rows = Vec::new();
    let mut of = |read_as: &'static str, payloads: &[&'static str], told: fn(&str) -> String| {
        rows.extend(
            payloads
                .iter()
                .map(|payload| (read_as, *payload, told(payload))),
        );
    };
    of("Surging", &SURGING, written_back::<Surging>);
    of("Careening", &CAREENING, written_back::<Careening>);
    of("Scending", &SCENDING, written_back::<Scending>);
    of("Swell", &SWELL, written_back::<Swell>);
    of("Lulling", &LULLING, written_back::<Lulling>);
    of("Tiding", &[TIDING], written_back::<Tiding>);
    of("Veering", &VEERING, written_back::<Veering>);
    of("Hawsing", &HAWSING, written_back::<Hawsing>);
    of("Gybing<String>", &HAWSING, written_back::<Gybing<String>>);
    of("Fairleading", &FAIRLEADING, written_back::<Fairleading>);
    of("Fothering", &FAIRLEADING, written_back::<Fothering>);
    rows
}

/// Each payload is read under each name serde reads it under, written back under the name serde
/// writes, and refused where serde refuses it.
#[test]
fn a_name_serde_reads_beside_the_one_it_writes_is_read_in_kotlin() {
    let rows = rows();
    let mut expected = String::new();
    let mut main = String::from("fun main() {\n");
    for (read_as, payload, told) in &rows {
        let _ = writeln!(expected, "{read_as} {payload}: {told}");
        let _ = writeln!(
            main,
            "    row(\"{read_as}\", serializer<{read_as}>(), \"\"\"{payload}\"\"\")"
        );
    }
    main.push_str("}\n");
    assert_eq!(
        expected,
        "Surging {\"Berth\":{\"label\":\"n\"}}: {\"Berth\":{\"label\":\"n\"}}\n\
         Surging {\"Pier\":{\"label\":\"n\"}}: {\"Berth\":{\"label\":\"n\"}}\n\
         Surging {\"Pier\":{\"name\":\"n\"}}: {\"Berth\":{\"label\":\"n\"}}\n\
         Surging {\"Berth\":{\"label\":\"n\",\"name\":\"m\"}}: refused\n\
         Surging \"Calm\": \"Calm\"\n\
         Surging \"Still\": \"Calm\"\n\
         Surging \"Slack\": \"Calm\"\n\
         Surging {\"Slack\":null}: \"Calm\"\n\
         Surging {\"Held\":\"x\"}: {\"Held\":\"x\"}\n\
         Surging {\"Tied\":\"x\"}: {\"Held\":\"x\"}\n\
         Careening {\"kind\":\"Berth\",\"label\":\"n\"}: {\"kind\":\"Berth\",\"label\":\"n\"}\n\
         Careening {\"kind\":\"Pier\",\"label\":\"n\"}: {\"kind\":\"Berth\",\"label\":\"n\"}\n\
         Careening {\"kind\":\"Pier\",\"name\":\"n\"}: {\"kind\":\"Berth\",\"label\":\"n\"}\n\
         Careening {\"kind\":\"Berth\",\"label\":\"n\",\"name\":\"m\"}: refused\n\
         Careening {\"kind\":\"Calm\"}: {\"kind\":\"Calm\"}\n\
         Careening {\"kind\":\"Still\"}: {\"kind\":\"Calm\"}\n\
         Scending {\"kind\":\"Berth\",\"data\":{\"label\":\"n\"}}: \
         {\"kind\":\"Berth\",\"data\":{\"label\":\"n\"}}\n\
         Scending {\"kind\":\"Pier\",\"data\":{\"label\":\"n\"}}: \
         {\"kind\":\"Berth\",\"data\":{\"label\":\"n\"}}\n\
         Scending {\"kind\":\"Pier\",\"data\":{\"name\":\"n\"}}: \
         {\"kind\":\"Berth\",\"data\":{\"label\":\"n\"}}\n\
         Scending {\"kind\":\"Calm\"}: {\"kind\":\"Calm\"}\n\
         Scending {\"kind\":\"Still\"}: {\"kind\":\"Calm\"}\n\
         Scending {\"kind\":\"Held\",\"data\":\"x\"}: {\"kind\":\"Held\",\"data\":\"x\"}\n\
         Scending {\"kind\":\"Tied\",\"data\":\"x\"}: {\"kind\":\"Held\",\"data\":\"x\"}\n\
         Swell \"Flood\": \"Flood\"\n\
         Swell \"Ebb\": \"Ebb\"\n\
         Swell \"Low\": \"Ebb\"\n\
         Swell \"Neap\": \"Ebb\"\n\
         Swell \"High\": refused\n\
         Lulling \" Ebb \": \"Ebb\"\n\
         Lulling \" Low \": \"Ebb\"\n\
         Lulling \"Flood\": \"Flood\"\n\
         Tiding {\"by_swell\":{\"Neap\":1,\"Flood\":2},\"swells\":[\"Low\",\"Flood\"]}: \
         {\"by_swell\":{\"Ebb\":1,\"Flood\":2},\"swells\":[\"Ebb\",\"Flood\"]}\n\
         Veering {\"label\":\"n\"}: {\"label\":\"n\"}\n\
         Veering {\"name\":\"n\"}: {\"label\":\"n\"}\n\
         Veering {\"label\":\"n\",\"name\":\"m\"}: refused\n\
         Veering 3: 3\n\
         Hawsing {\"label\":\"n\"}: {\"label\":\"n\"}\n\
         Hawsing {\"name\":\"n\"}: {\"label\":\"n\"}\n\
         Hawsing {\"title\":\"n\",\"remark\":\"r\"}: {\"label\":\"n\",\"note\":\"r\"}\n\
         Hawsing {\"label\":\"n\",\"name\":\"m\"}: refused\n\
         Hawsing {\"name\":\"n\",\"title\":\"m\"}: refused\n\
         Gybing<String> {\"label\":\"n\"}: {\"label\":\"n\"}\n\
         Gybing<String> {\"name\":\"n\"}: {\"label\":\"n\"}\n\
         Gybing<String> {\"title\":\"n\",\"remark\":\"r\"}: {\"label\":\"n\",\"note\":\"r\"}\n\
         Gybing<String> {\"label\":\"n\",\"name\":\"m\"}: refused\n\
         Gybing<String> {\"name\":\"n\",\"title\":\"m\"}: refused\n\
         Fairleading {\"name\":\"n\",\"north\":2}: {\"label\":\"n\",\"keys\":[\"north\"]}\n\
         Fairleading {\"label\":\"n\",\"north\":2}: {\"label\":\"n\",\"keys\":[\"north\"]}\n\
         Fairleading {\"label\":\"n\",\"name\":\"m\",\"north\":2}: refused\n\
         Fothering {\"name\":\"n\",\"north\":2}: {\"label\":\"n\",\"keys\":[\"north\"]}\n\
         Fothering {\"label\":\"n\",\"north\":2}: {\"label\":\"n\",\"keys\":[\"north\"]}\n\
         Fothering {\"label\":\"n\",\"name\":\"m\",\"north\":2}: refused\n"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        pennon_kotlin::kotlin_definition(),
        surging_kotlin::kotlin_definition(),
        careening_kotlin::kotlin_definition(),
        scending_kotlin::kotlin_definition(),
        swell_kotlin::kotlin_definition(),
        lulling_kotlin::kotlin_definition(),
        tiding_kotlin::kotlin_definition(),
        veering_kotlin::kotlin_definition(),
        hawsing_kotlin::kotlin_definition(),
        gybing_kotlin::kotlin_definition(),
        fairleading_kotlin::kotlin_definition(),
        fothering_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected);
}
