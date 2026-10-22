//! The lone slot of a newtype variant under a tag alone, read and written back by the emitted
//! Kotlin types compiled with `-Werror`, under the default `Json` and under `Json {
//! ignoreUnknownKeys = true }`, beside what serde reads and what the generated Zod schema answers
//! of the same payload. Kotlin reads the slot from the object without the tag and writes its
//! members beside the tag, as serde does, and refuses the slot under a `value` key, as serde and
//! Zod refuse it.

#![cfg(all(feature = "kotlin", feature = "serde"))]

use core::fmt::Write as _;

use super::runtime::ran_kotlin;
#[cfg(all(feature = "zod", feature = "typescript"))]
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
#[cfg(all(feature = "zod", feature = "typescript"))]
use super::tagged_slot_models::{
    Ballast, Bunkered, Demurrage, Docketed, Dray, Freighted, Lighter, Lightering, Palleted,
    Stevedore, Tonnage, Trussed, Waybilled, Wharfage, drayage_schema, lashing_schema,
};
use super::tagged_slot_models::{
    ENTRIES_REFUSED, WRITTEN, ballast_kotlin, bunkered_kotlin, demurrage_kotlin, docketed_kotlin,
    dray_kotlin, drayage_kotlin, freighted_kotlin, lashing_kotlin, lighter_kotlin,
    lightering_kotlin, palleted_kotlin, serde_told, stevedore_kotlin, tonnage_kotlin,
    trussed_kotlin, waybilled_kotlin, wharfage_kotlin,
};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// The function `Docketed::Trimmed` and `Docketed::Pared` name, and what prints a row: the read
/// under the default `Json`, then under one that ignores unknown keys, each the value written back
/// or the first line of the message of the `SerializationException` the read throws.
const READ: &str = r#"
fun lightened(payload: JsonElement): JsonElement =
    if (payload is JsonObject) JsonObject(payload - "spare") else payload

val ignoring = Json { ignoreUnknownKeys = true }

fun <T> told(at: Int, serializer: KSerializer<T>, payload: String) {
    val told = listOf(Json, ignoring).joinToString(" | ") { json ->
        try {
            "read " + json.encodeToString(serializer, json.decodeFromString(serializer, payload))
        } catch (refusal: SerializationException) {
            "refused: " + (refusal.message ?: "").lines().first()
        }
    }
    println("$at: $told")
}
"#;

/// The function `Docketed::Trimmed` and `Docketed::Pared` name, for the Zod schema.
#[cfg(all(feature = "zod", feature = "typescript"))]
const LIGHTENED: &str = "function lightened(payload: unknown): unknown {\n  \
     if (typeof payload !== \"object\" || payload === null || Array.isArray(payload)) return \
     payload;\n  const { spare: _, ...rest } = payload as Record<string, unknown>;\n  return \
     rest;\n}";

/// What the default `Json` throws for the key `other`, in the words of the plugin's serializer.
const UNKNOWN_AT: &str = "refused: Encountered an unknown key 'other' at path: $";

/// What a payload no row of [`WRITTEN`] or of [`ENTRIES_REFUSED`] holds is read as by each side:
/// the slot under a `value` key, which serde and Zod refuse; a key the slot does not read, which
/// serde reads past and Zod refuses; a key the function of the slot drops; and a struct under its
/// own tag with no tag.
const ROWS: [Row; 9] = [
    Row {
        ignoring: "refused: Field 'cost' is required for type with serial name 'Tonnage', but \
                   it was missing at path: $",
        json: r#"{"kind":"Weighed","value":{"cost":"c"}}"#,
        kotlin: "refused: Encountered an unknown key 'value' at path: $",
        read_as: "Docketed",
        serde: "refused",
        zod: "refused",
    },
    Row {
        ignoring: "refused: Field 'cost' is required for type with serial name 'Tonnage', but \
                   it was missing at path: $",
        json: r#"{"kind":"Held","value":{"cost":"c"}}"#,
        kotlin: "refused: Encountered an unknown key 'value' at path: $",
        read_as: "Waybilled<Tonnage>",
        serde: "refused",
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Weighed\",\"cost\":\"c\"}",
        json: r#"{"kind":"Weighed","cost":"c","other":1}"#,
        kotlin: UNKNOWN_AT,
        read_as: "Docketed",
        serde: "read {\"kind\":\"Weighed\",\"cost\":\"c\"}",
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Hired\"}",
        json: r#"{"kind":"Hired","other":1}"#,
        kotlin: UNKNOWN_AT,
        read_as: "Docketed",
        serde: "read {\"kind\":\"Hired\"}",
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Merged\",\"note\":\"n\",\"cost\":\"c\"}",
        json: r#"{"kind":"Merged","note":"n","cost":"c","other":1}"#,
        kotlin: "refused: Encountered an unknown key 'other'",
        read_as: "Docketed",
        serde: "read {\"kind\":\"Merged\",\"note\":\"n\",\"cost\":\"c\"}",
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Below\",\"barge\":\"g\"}",
        json: r#"{"kind":"Below","barge":"g","other":1}"#,
        kotlin: UNKNOWN_AT,
        read_as: "Docketed",
        serde: "read {\"kind\":\"Below\",\"barge\":\"g\"}",
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Held\",\"cost\":\"c\"}",
        json: r#"{"kind":"Held","cost":"c","other":1}"#,
        kotlin: UNKNOWN_AT,
        read_as: "Waybilled<Tonnage>",
        serde: "read {\"kind\":\"Held\",\"cost\":\"c\"}",
        zod: "refused",
    },
    Row {
        ignoring: "read {\"kind\":\"Trimmed\",\"cost\":\"c\"}",
        json: r#"{"kind":"Trimmed","cost":"c","spare":1}"#,
        kotlin: "read {\"kind\":\"Trimmed\",\"cost\":\"c\"}",
        read_as: "Docketed",
        serde: "read {\"kind\":\"Trimmed\",\"cost\":\"c\"}",
        zod: "read {\"kind\":\"Trimmed\",\"cost\":\"c\"}",
    },
    Row {
        ignoring: "refused: missing field `sort`",
        json: r#"{"kind":"Sorted","weight":3}"#,
        kotlin: "refused: missing field `sort`",
        read_as: "Docketed",
        serde: "read {\"kind\":\"Sorted\",\"sort\":\"Ballast\",\"weight\":3}",
        zod: "refused",
    },
];

/// One payload, the type it is read as, and what each read answers of it.
struct Row {
    /// What the Kotlin read under `Json { ignoreUnknownKeys = true }` answers.
    ignoring: &'static str,
    json: &'static str,
    /// What the Kotlin read under the default `Json` answers.
    kotlin: &'static str,
    /// The type the payload is read as, as Kotlin names it.
    read_as: &'static str,
    /// What serde answers of the payload.
    serde: &'static str,
    /// What the generated Zod schema answers of the payload: what it reads, or `refused`.
    zod: &'static str,
}

/// A row with what each read answers of it as text of its own.
struct Told {
    ignoring: String,
    json: &'static str,
    kotlin: String,
    read_as: &'static str,
    serde: String,
    zod: String,
}

/// Every payload, beside what serde, Zod and both Kotlin reads answer of it: each payload serde
/// writes, read and written back as serde writes it, its unit entry among them, then [`ROWS`], and
/// then each payload of [`ENTRIES_REFUSED`], which all of them refuse.
fn every_row() -> Vec<Told> {
    let written = WRITTEN.iter().map(|&(read_as, json)| {
        let read = format!("read {json}");
        Told {
            ignoring: read.clone(),
            json,
            kotlin: read.clone(),
            read_as,
            serde: read.clone(),
            zod: read,
        }
    });
    let pinned = ROWS.iter().map(|row| Told {
        ignoring: row.ignoring.to_owned(),
        json: row.json,
        kotlin: row.kotlin.to_owned(),
        read_as: row.read_as,
        serde: row.serde.to_owned(),
        zod: row.zod.to_owned(),
    });
    let refused = ENTRIES_REFUSED.iter().map(|row| Told {
        ignoring: row.kotlin.to_owned(),
        json: row.json,
        kotlin: row.kotlin.to_owned(),
        read_as: "Docketed",
        serde: "refused".to_owned(),
        zod: "refused".to_owned(),
    });
    written.chain(pinned).chain(refused).collect()
}

/// serde reads each payload it writes and writes it back, and answers each row as pinned.
#[test]
fn serde_answers_each_payload_as_pinned() {
    for row in every_row() {
        assert_eq!(
            serde_told(row.read_as, row.json),
            row.serde,
            "serde no longer answers {} as it did",
            row.json
        );
    }
}

/// Kotlin reads the slot of a newtype variant under a tag alone from the object without the tag
/// and writes its members beside the tag, as serde does: every payload serde writes is read and
/// written back as serde writes it. A slot under a `value` key is refused, as serde and Zod refuse
/// it, and a key the slot does not read is refused under the default `Json`, as Zod refuses it.
#[test]
fn a_slot_beside_the_tag_is_read_and_written_by_kotlin_as_serde_and_zod_read_it() {
    let mut expected = String::new();
    let mut main = String::from("fun main() {\n");
    for (at, row) in every_row().iter().enumerate() {
        let _ = writeln!(expected, "{at}: {} | {}", row.kotlin, row.ignoring);
        let _ = writeln!(
            main,
            "    told({at}, serializer<{}>(), \"\"\"{}\"\"\")",
            row.read_as, row.json
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        tonnage_kotlin::kotlin_definition(),
        ballast_kotlin::kotlin_definition(),
        wharfage_kotlin::kotlin_definition(),
        drayage_kotlin::kotlin_definition(),
        lighter_kotlin::kotlin_definition(),
        demurrage_kotlin::kotlin_definition(),
        bunkered_kotlin::kotlin_definition(),
        palleted_kotlin::kotlin_definition(),
        lashing_kotlin::kotlin_definition(),
        trussed_kotlin::kotlin_definition(),
        dray_kotlin::kotlin_definition(),
        stevedore_kotlin::kotlin_definition(),
        freighted_kotlin::kotlin_definition(),
        docketed_kotlin::kotlin_definition(),
        lightering_kotlin::kotlin_definition(),
        waybilled_kotlin::kotlin_definition(),
        READ.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not the read above");
}

/// The emitted Kotlin reads a lone slot under a tag alone through the slot's own serializer and
/// writes it beside the tag, with no `value` key: the variant's class keeps its `value` property.
#[test]
fn the_kotlin_text_reads_and_writes_the_slot_beside_the_tag() {
    let docketed = docketed_kotlin::kotlin_definition();
    for held in [
        "@SerialName(\"Weighed\") @Serializable data class DocketedWeighed(@Serializable(with = \
         TonnageUnjudged::class) val value: Tonnage) : Docketed {",
        "is DocketedWeighed -> buildJsonObject { put(\"kind\", \"Weighed\"); \
         output.json.encodeToJsonElement(TonnageUnjudged, value.value).jsonObject.forEach { (k, \
         v) -> put(k, v) } }",
        "\"Weighed\" -> DocketedWeighed(input.json.decodeFromJsonElement(TonnageUnjudged, \
         JsonObject(obj - \"kind\")))",
        "is DocketedExternal -> buildJsonObject { put(\"kind\", \"External\"); when (val written \
         = output.json.encodeToJsonElement(PalletedUnjudged, value.value)) { is JsonPrimitive -> \
         put(written.content, JsonNull); else -> written.jsonObject.forEach { (k, v) -> put(k, \
         v) } } }",
        "\"Trimmed\" -> DocketedTrimmed(input.json.decodeFromJsonElement(TonnageUnjudged, \
         lightened(JsonObject(obj - \"kind\"))))",
    ] {
        assert!(docketed.contains(held), "no `{held}` in: {docketed}");
    }
    assert!(
        !docketed.contains("\"value\" ->"),
        "a slot is still read under its `value` key: {docketed}"
    );
    let waybilled = waybilled_kotlin::kotlin_definition();
    for held in [
        "is WaybilledHeld -> buildJsonObject { put(\"kind\", \"Held\"); \
         output.json.encodeToJsonElement(heldSerializer, value.value).jsonObject.forEach { (k, \
         v) -> put(k, v) } }",
        "\"Held\" -> WaybilledHeld(input.json.decodeFromJsonElement(heldSerializer, \
         JsonObject(obj - \"kind\")))",
    ] {
        assert!(waybilled.contains(held), "no `{held}` in: {waybilled}");
    }
}

/// The generated Zod schemas, and what prints each row's verdict.
#[cfg(all(feature = "zod", feature = "typescript"))]
fn zod_module() -> String {
    let mut module = [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        LIGHTENED.to_owned(),
        Tonnage::ts_definition(),
        Tonnage::zod_schema(),
        Ballast::ts_definition(),
        Ballast::zod_schema(),
        Wharfage::ts_definition(),
        Wharfage::zod_schema(),
        drayage_schema::Schema::ts_definition(),
        drayage_schema::Schema::zod_schema(),
        Lighter::<String>::ts_definition(),
        Lighter::<String>::zod_schema(),
        Demurrage::ts_definition(),
        Demurrage::zod_schema(),
        Bunkered::ts_definition(),
        Bunkered::zod_schema(),
        Palleted::ts_definition(),
        Palleted::zod_schema(),
        lashing_schema::Schema::ts_definition(),
        lashing_schema::Schema::zod_schema(),
        Trussed::ts_definition(),
        Trussed::zod_schema(),
        Dray::ts_definition(),
        Dray::zod_schema(),
        Stevedore::ts_definition(),
        Stevedore::zod_schema(),
        Freighted::ts_definition(),
        Freighted::zod_schema(),
        Docketed::ts_definition(),
        Docketed::zod_schema(),
        Lightering::ts_definition(),
        Lightering::zod_schema(),
        Waybilled::<Tonnage>::ts_definition(),
        Waybilled::<Tonnage>::zod_schema(),
    ]
    .join("\n\n");
    module.push_str("\n\nconst told: string[] = [];\n");
    for row in every_row() {
        let schema = match row.read_as {
            "Docketed" => "Docketed$Schema",
            "Trussed" => "Trussed$Schema",
            _ => "Waybilled$SchemaDefault",
        };
        let _ = writeln!(
            module,
            "told.push(((parsed) => parsed.success ? \"read \" + JSON.stringify(parsed.data) : \
             \"refused\")({schema}.safeParse(JSON.parse({}))));",
            serde_json::to_string(row.json).unwrap()
        );
    }
    module.push_str("console.log(JSON.stringify(told));\n");
    module
}

/// The generated Zod schema reads the slot's members beside the tag, an externally tagged enum's
/// unit variant as an entry of its name over `null`, and refuses the slot under a `value` key and
/// a key no member reads.
#[cfg(all(feature = "zod", feature = "typescript"))]
#[test]
fn zod_reads_the_slot_beside_the_tag() {
    const REQUIRED: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED) else {
        stand_down_modules(
            REQUIRED,
            "the generated Zod schemas of a slot beside the tag",
        );
        return;
    };
    let Some(printed) = ran_with_modules(
        "tagged-slots",
        "TIXSCHEMA_NODE",
        "node",
        "slots.mts",
        &zod_module(),
        &modules,
    ) else {
        return;
    };
    let told: Vec<String> = serde_json::from_str(printed.trim()).unwrap();
    let pinned: Vec<String> = every_row().into_iter().map(|row| row.zod).collect();
    assert_eq!(
        told, pinned,
        "the Zod schema no longer answers each payload as it did"
    );
}

/// The README's sentence on a newtype variant under a tag alone declares the enum the run reads,
/// and shows the payload it reads and the refusal it measures.
#[test]
fn the_readme_shows_the_slot_the_run_reads() {
    let readme = include_str!("../../README.md");
    let (read_as, read) = WRITTEN[0];
    let refused = &ROWS[0];
    assert!(
        read_as == "Docketed" && refused.json == r#"{"kind":"Weighed","value":{"cost":"c"}}"#,
        "the rows are no longer the ones the README shows"
    );
    for shown in [
        "`#[serde(tag = \"kind\")] enum Docketed { Weighed(Tonnage) }` over `struct Tonnage { \
         cost: String }`"
            .to_owned(),
        format!("`{read}` reads `Weighed` and is written back as it was read"),
        format!(
            "`{}` is refused with `SerializationException: {}`",
            refused.json,
            refused.kotlin.strip_prefix("refused: ").unwrap()
        ),
    ] {
        assert!(readme.contains(&shown), "not in the README: {shown}");
    }
}

/// The README's sentences on an externally tagged enum held beside the tag show the payload serde
/// writes for its unit variant and the words the Dart and Swift reads refuse the tag alone and two
/// entries with.
#[test]
fn the_readme_shows_the_entries_the_runs_read_and_refuse() {
    let readme = include_str!("../../README.md");
    let [tag_alone, two_entries, _] = &ENTRIES_REFUSED;
    assert!(
        WRITTEN.contains(&("Docketed", r#"{"kind":"External","Loose":null}"#))
            && tag_alone.json == r#"{"kind":"External"}"#,
        "the rows are no longer the ones the README shows"
    );
    for shown in [
        "`#[serde(tag = \"kind\")] enum Docketed { External(Palleted) }` over `enum Palleted { \
         Loose, Stacked { layers: u32 } }`, `{\"kind\":\"External\",\"Loose\":null}` reads"
            .to_owned(),
        format!(
            "The tag alone, `{}`, is refused with `{}`, and the entries of two variants with `{}`",
            tag_alone.json, tag_alone.dart, two_entries.dart
        ),
        format!(
            "The tag alone, `{}`, is refused with `DecodingError.dataCorrupted` and `{}`, and the \
             entries of two variants with `{}`",
            tag_alone.json, tag_alone.swift, two_entries.swift
        ),
    ] {
        assert!(readme.contains(&shown), "not in the README: {shown}");
    }
}
