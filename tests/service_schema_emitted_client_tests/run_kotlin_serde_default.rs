//! A key under `#[serde(default)]` the payload leaves out, read by the emitted Kotlin types
//! compiled with `-Werror`: each payload of the shared table prints what the serde read prints
//! of it.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::runtime::ran_kotlin;
use super::serde_default_models::{
    Bollard, CASES, Harbour, Jetty, Marina, PREPROCESSED, PREPROCESSED_REPORT, REPORT, REQUIRED,
    REQUIRED_REPORT, VARIANT_CASES, VARIANT_REPORT, adjacent_berthing_kotlin, basin_kotlin,
    berthing_kotlin, bollard_kotlin, config_kotlin, external_berthing_kotlin, filled_cases,
    filled_expected, filled_report, fleet_kotlin, gangway_kotlin, harbour_kotlin, jetty_kotlin,
    link_kotlin, logbook_kotlin, manifest_kotlin, marina_kotlin, pontoon_kotlin,
    preprocessed_values, quay_kotlin, read_values, untagged_berthing_kotlin, unwritten_defaults,
    wharf_kotlin,
};
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::serde_default_models::{report, variant_report};
#[cfg(feature = "indexmap")]
use super::serde_default_models::{indexed_fleet_kotlin, indexed_logbook_kotlin};

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

/// What prints a row of a key the type requires: whether the read answered or threw.
const REQUIRES: &str = r#"
fun requires(name: String, read: () -> Any) {
    try {
        read()
        println("$name: read")
    } catch (refused: SerializationException) {
        println("$name: refused")
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

/// A default with no JSON keeps its key required: `Marina` derives no `Serialize`, and each
/// member of `Harbour` names its parameter outside a list read as empty.
#[test]
fn a_default_with_no_json_keeps_its_key_required() {
    let (read_marina, _) = unwritten_defaults();
    assert_eq!(
        serde_json::from_str::<Marina>(r#"{"label":"m"}"#).unwrap(),
        read_marina
    );
    let marina = marina_kotlin::kotlin_definition();
    assert!(
        marina.contains("data class Marina(val dock: String, val label: String)"),
        "{marina}"
    );
    let harbour = harbour_kotlin::kotlin_definition();
    assert!(
        harbour.contains(
            "data class Harbour<Moored>(val berths: List<Moored>, val flagship: Moored, val \
             slips: List<Moored>)"
        ),
        "{harbour}"
    );
}

/// A list, a set or a map whose type names a parameter of the class is the empty collection
/// where the key is left out: a property's initializer has no serializer of the parameter to read
/// `[]` with. A member whose type names none reads the JSON of its default as before.
#[test]
fn an_empty_collection_of_a_parameter_is_the_default_of_its_property() {
    let basin = basin_kotlin::kotlin_definition();
    assert!(
        basin.contains(
            "data class Basin<Moored>(val dock: String = Json.decodeFromString(serializer<String>(), \
             \"\\\"\\\"\"), val moored: List<Moored> = emptyList())"
        ),
        "{basin}"
    );
    let config = config_kotlin::kotlin_definition();
    assert!(
        config.contains(
            "data class Config<IdType>(val dataLinks: List<@Serializable(with = \
             LinkUnjudged::class) Link<IdType>> = emptyList(), val sequences: List<String> = \
             Json.decodeFromString(serializer<List<String>>(), \"[]\"))"
        ),
        "{config}"
    );
    let fleet = fleet_kotlin::kotlin_definition();
    assert!(
        fleet.contains(
            "val vecDeque: List<@Serializable(with = LinkUnjudged::class) Link<IdType>> = \
             emptyList()"
        ),
        "{fleet}"
    );
    let logbook = logbook_kotlin::kotlin_definition();
    assert!(
        logbook.contains(
            "val btreeMap: Map<String, @Serializable(with = LinkUnjudged::class) Link<IdType>> = \
             emptyMap()"
        ),
        "{logbook}"
    );
}

/// Each payload of a struct that declares a parameter reads at `String` as serde reads it, and a
/// payload that leaves out a key `Harbour` requires is refused.
#[test]
fn each_payload_of_a_struct_that_declares_a_parameter_reads_in_kotlin_as_serde_reads_it() {
    assert_eq!(
        filled_report(),
        filled_expected(),
        "what the serde read answers"
    );
    let (_, harbours) = unwritten_defaults();
    for ((_, payload), harbour) in REQUIRED.iter().zip(harbours) {
        assert_eq!(
            serde_json::from_str::<Harbour<String>>(payload).unwrap(),
            harbour
        );
    }
    let filled = filled_cases().into_iter().map(|(name, read_as, payload)| {
        format!(
            "told({}) {{ everyMember.encodeToString(serializer<{read_as}<String>>(), \
             Json.decodeFromString(serializer<{read_as}<String>>(), {})) }}",
            kotlin_string(name),
            kotlin_string(payload)
        )
    });
    let required = REQUIRED.iter().map(|&(name, payload)| {
        format!(
            "requires({}) {{ Json.decodeFromString(serializer<Harbour<String>>(), {}) }}",
            kotlin_string(name),
            kotlin_string(payload)
        )
    });
    let mut main = String::from("fun main() {\n");
    for statement in filled.chain(required) {
        let _ = writeln!(main, "    {statement}");
    }
    main.push_str("}\n");
    let definitions = [
        link_kotlin::kotlin_definition(),
        basin_kotlin::kotlin_definition(),
        config_kotlin::kotlin_definition(),
        fleet_kotlin::kotlin_definition(),
        logbook_kotlin::kotlin_definition(),
        harbour_kotlin::kotlin_definition(),
        manifest_kotlin::kotlin_definition(),
        #[cfg(feature = "indexmap")]
        indexed_fleet_kotlin::kotlin_definition(),
        #[cfg(feature = "indexmap")]
        indexed_logbook_kotlin::kotlin_definition(),
    ];
    let source = [
        &[KOTLIN_IMPORTS.to_owned()],
        definitions.as_slice(),
        &[TOLD.to_owned(), REQUIRES.to_owned(), main],
    ]
    .concat()
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    assert_eq!(printed, format!("{}{REQUIRED_REPORT}", filled_expected()));
}

/// Every row of the variant table, under each tagging: a member of a variant left out is read as
/// the default serde reads, a member held as it was, and a default that breaks its bound is
/// refused with its member named.
#[test]
fn each_variant_payload_reads_in_kotlin_as_the_serde_read_reads_it() {
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(variant_report(), VARIANT_REPORT, "what the serde read answers");
    let mut main = String::from("fun main() {\n");
    for &(name, read_as, payload) in &VARIANT_CASES {
        let _ = writeln!(
            main,
            "    told({}) {{ everyMember.encodeToString(serializer<{read_as}>(), \
             Json.decodeFromString(serializer<{read_as}>(), {})) }}",
            kotlin_string(name),
            kotlin_string(payload)
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        berthing_kotlin::kotlin_definition(),
        external_berthing_kotlin::kotlin_definition(),
        adjacent_berthing_kotlin::kotlin_definition(),
        untagged_berthing_kotlin::kotlin_definition(),
        quay_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    assert_eq!(printed, VARIANT_REPORT);
}
