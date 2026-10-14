//! A key under `#[serde(default)]` the payload leaves out, read by the emitted Kotlin types
//! compiled with `-Werror`: each payload of the shared table prints what the serde read prints
//! of it.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::runtime::ran_kotlin;
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::serde_default_models::report;
use super::serde_default_models::{
    Basin, Bollard, CASES, Jetty, Marina, PREPROCESSED, PREPROCESSED_REPORT, REPORT, basin_kotlin,
    bollard_kotlin, gangway_kotlin, jetty_kotlin, marina_kotlin, pontoon_kotlin,
    preprocessed_values, read_values, unwritten_defaults, wharf_kotlin,
};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// The function `Bollard` names, as a consumer writes it in Kotlin.
const TRIMMED_DOCK: &str = "fun trimmedDock(payload: JsonElement): JsonElement =
    if (payload is JsonPrimitive && payload.isString) JsonPrimitive(payload.content.trim()) else payload";

/// What prints a row: the value read, written back with every member, or each message of what
/// the read threw.
const TOLD: &str = r#"
val everyMember = Json { encodeDefaults = true }

fun told(name: String, read: () -> String) {
    try {
        println("$name: read ${read()}")
    } catch (refused: SerializationException) {
        println("$name: refused")
        (refused.message ?: "").split("; ").forEach { println("    $it") }
    }
}
"#;

/// What the default `Json` writes of a value whose members equal their defaults: a member with a
/// default is written only where its value is another, as `encodeDefaults = false` writes it.
const WRITTEN: &str = "\
the container's default: {}
one key held: {\"dock\":\"west\"}
";

/// `text` as a Kotlin string writes it: a backslash, a quote and a `$` each behind a backslash.
fn kotlin_string(text: &str) -> String {
    format!(
        "\"{}\"",
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
    )
}

/// The emitted types of the table, what prints a row, and `main`, whose statements are `body`.
fn module(body: &[String]) -> String {
    let mut main = String::from("fun main() {\n");
    for statement in body {
        let _ = writeln!(main, "    {statement}");
    }
    main.push_str("}\n");
    [
        KOTLIN_IMPORTS.to_owned(),
        gangway_kotlin::kotlin_definition(),
        wharf_kotlin::kotlin_definition(),
        jetty_kotlin::kotlin_definition(),
        pontoon_kotlin::kotlin_definition(),
        bollard_kotlin::kotlin_definition(),
        TRIMMED_DOCK.to_owned(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n")
}

/// Every row of the table: a key left out is read as the default serde reads, a key held as it
/// was, and a default that breaks its bound is refused with its member named.
#[test]
fn each_payload_reads_in_kotlin_as_the_serde_read_reads_it() {
    for ((_, payload), read) in PREPROCESSED.iter().zip(preprocessed_values()) {
        assert_eq!(serde_json::from_str::<Bollard>(payload).unwrap(), read);
    }
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(report(), REPORT, "what the serde read answers");
    let (every_default, _) = read_values();
    assert!(REPORT.contains(&format!(
        "every default at once: read {}\n",
        serde_json::to_string(&every_default).unwrap()
    )));
    let bollards = PREPROCESSED
        .iter()
        .map(|&(name, payload)| (name, "Bollard", payload));
    let rows: Vec<String> = CASES
        .iter()
        .copied()
        .chain(bollards)
        .map(|(name, read_as, payload)| {
            format!(
                "told({}) {{ everyMember.encodeToString(serializer<{read_as}>(), \
                 Json.decodeFromString(serializer<{read_as}>(), {})) }}",
                kotlin_string(name),
                kotlin_string(payload)
            )
        })
        .collect();
    let Some(printed) = ran_kotlin(&module(&rows)) else {
        return;
    };
    assert_eq!(printed, format!("{REPORT}{PREPROCESSED_REPORT}"));
}

/// The plugin writes a member with a default only where its value is another: the default `Json`
/// leaves out a key whose value is its default, which serde reads back as the same value.
#[test]
fn the_default_json_leaves_out_a_member_whose_value_is_its_default() {
    let held = serde_json::to_value(serde_json::from_str::<Jetty>(r#"{"dock":"west"}"#).unwrap());
    assert_eq!(
        serde_json::from_str::<Jetty>("{}").unwrap(),
        Jetty::default(),
        "serde reads the key left out as the default"
    );
    assert_eq!(
        held.unwrap(),
        serde_json::json!({ "berths": 4_u32, "dock": "west", "moorings": ["a", "b"] })
    );
    let rows = [
        "println(\"the container's default: ${Json.encodeToString(serializer<Jetty>(), Jetty())}\")"
            .to_owned(),
        "println(\"one key held: ${Json.encodeToString(serializer<Jetty>(), Jetty(dock = \"west\"))}\")"
            .to_owned(),
    ];
    let Some(printed) = ran_kotlin(&module(&rows)) else {
        return;
    };
    assert_eq!(printed, WRITTEN);
}

/// A default with no JSON keeps its key required: `Marina` derives no `Serialize`, and the
/// `moored` of `Basin` names its parameter. The `dock` of `Basin` names none, and is read as its
/// default.
#[test]
fn a_default_with_no_json_keeps_its_key_required() {
    let (read_marina, read_basin) = unwritten_defaults();
    assert_eq!(
        serde_json::from_str::<Marina>(r#"{"label":"m"}"#).unwrap(),
        read_marina
    );
    assert_eq!(
        serde_json::from_str::<Basin<String>>(r#"{"moored":[]}"#).unwrap(),
        read_basin
    );
    let marina = marina_kotlin::kotlin_definition();
    assert!(
        marina.contains("data class Marina(val dock: String, val label: String)"),
        "{marina}"
    );
    let basin = basin_kotlin::kotlin_definition();
    assert!(
        basin.contains(
            "data class Basin<Moored>(val dock: String = Json.decodeFromString(serializer<String>(), \
             \"\\\"\\\"\"), val moored: List<Moored>)"
        ),
        "{basin}"
    );
}
