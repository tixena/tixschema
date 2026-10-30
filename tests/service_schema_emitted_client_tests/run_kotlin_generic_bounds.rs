//! `validate()` of the emitted Kotlin, Dart and Swift types for a generic brand whose declared
//! default is written with arguments, and for a map keyed by a type parameter, compiled with
//! `-Werror` where the language has the flag and run: each row prints what Rust's `validate()`
//! answers of the same value.

#![cfg(feature = "kotlin")]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::ran_kotlin;

/// The imports the emitted Kotlin reaches for.
const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What a row of the Kotlin run prints.
const REPORT: &str = r#"
fun report(name: String, violations: List<String>) {
    if (violations.isEmpty()) {
        println("$name: passes")
    } else {
        println("$name: refused")
        violations.forEach { println("    $it") }
    }
}
"#;

/// What the read of a payload prints: `passes`, or `refused` and each message the read throws.
const READ: &str = r#"
inline fun <reified T> read(name: String, payload: String) {
    try {
        Json.decodeFromString<T>(payload)
        println("$name: passes")
    } catch (thrown: SerializationException) {
        println("$name: refused")
        thrown.message!!.split("; ").forEach { println("    $it") }
    }
}
"#;

/// What a row of the Dart run prints.
#[cfg(feature = "dart")]
const DART_REPORT: &str = "
void report(String name, List<String> violations) {
  if (violations.isEmpty) {
    print('$name: passes');
  } else {
    print('$name: refused');
    for (final violation in violations) {
      print('    $violation');
    }
  }
}
";

/// What a row of the Swift run prints.
#[cfg(feature = "swift")]
const SWIFT_REPORT: &str = r#"
func report(_ name: String, _ violations: [String]) {
  if violations.isEmpty {
    print("\(name): passes")
  } else {
    print("\(name): refused")
    for violation in violations {
      print("    \(violation)")
    }
  }
}
"#;

/// Rows only Kotlin can build: the JVM erases the argument of a class, so a value of the
/// default's class that holds a text is judged whatever its argument says.
const ERASED: [(&str, &str, &[&str]); 1] = [(
    "a filling of the class that holds a text",
    "Wrapped<Tag<CharSequence>>(Tag<CharSequence>(\"ab\")).validate()",
    &["too short: minimum length is 3, got 2"],
)];

/// A generic brand that holds its parameter bare and declares a string for it.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Tag<IdType>(pub IdType);

/// A brand whose declared default is written with arguments.
#[model_schema(minLength = 3, maxLength = 5, default_types(WrapType = Tag<String>))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Wrapped<WrapType>(pub WrapType);

/// A member whose bound is read with no argument written after the declared default.
#[model_schema(minLength = 3, default_types(WrapType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Plain<WrapType>(pub WrapType);

/// A plain enum a map is keyed by, each member under the name serde writes it.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Slot {
    NorthDock,
    SouthDock,
}

/// A brand over a string a map is keyed by.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Berth(pub String);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Item {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A holder whose map is keyed by one of its parameters, which it declares `Serialize` for.
#[model_schema(default_types(KeyType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(bound(serialize = "", deserialize = "KeyType: Deserialize<'de>"))]
pub struct Shelf<KeyType: Ord + Serialize> {
    pub by_slot: BTreeMap<KeyType, Item>,
}

/// An internally tagged enum whose variant holds a map keyed by one of its parameters.
#[model_schema(default_types(KeyType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    bound(serialize = "", deserialize = "KeyType: Deserialize<'de>")
)]
pub enum Depot<KeyType: Ord + Serialize> {
    Empty,
    Stocked { by_slot: BTreeMap<KeyType, Item> },
}

/// An untagged enum whose variant holds a map keyed by one of its parameters.
#[model_schema(default_types(KeyType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    untagged,
    bound(serialize = "", deserialize = "KeyType: Deserialize<'de>")
)]
pub enum Pallet<KeyType: Ord + Serialize> {
    Counted(u32),
    Loaded { by_slot: BTreeMap<KeyType, Item> },
}

/// A holder whose field is a `Shelf` filled with a generic brand.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Aisle {
    pub shelf: Shelf<Tag<String>>,
}

/// A generic holder whose field is a `Shelf` filled with its own parameter.
#[model_schema(default_types(KeyType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(bound(serialize = "", deserialize = "KeyType: Deserialize<'de>"))]
pub struct Annex<KeyType: Ord + Serialize> {
    pub shelf: Shelf<KeyType>,
}

/// One row: the Kotlin expression of a value and the messages Rust's `validate()` answers of the
/// same value.
struct Row {
    dart: &'static str,
    kotlin: &'static str,
    name: &'static str,
    rust: Result<(), Vec<String>>,
    swift: &'static str,
}

fn item(label: &str) -> Item {
    Item {
        label: label.to_owned(),
    }
}

fn rows() -> Vec<Row> {
    let shelf = |entries: Vec<(Slot, &str)>| Shelf {
        by_slot: entries
            .into_iter()
            .map(|(slot, label)| (slot, item(label)))
            .collect(),
    };
    vec![
        Row {
            dart: "Wrapped<Tag<String>>(Tag<String>('ab')).validate()",
            kotlin: "Wrapped<Tag<String>>(Tag(\"ab\")).validate()",
            swift: "Wrapped<Tag<String>>(value: Tag<String>(value: \"ab\")).validate()",
            name: "the default, below the bound",
            rust: Wrapped(Tag("ab".to_owned())).validate(),
        },
        Row {
            dart: "Wrapped<Tag<String>>(Tag<String>('abcdef')).validate()",
            kotlin: "Wrapped<Tag<String>>(Tag(\"abcdef\")).validate()",
            swift: "Wrapped<Tag<String>>(value: Tag<String>(value: \"abcdef\")).validate()",
            name: "the default, past the bound",
            rust: Wrapped(Tag("abcdef".to_owned())).validate(),
        },
        Row {
            dart: "Wrapped<Tag<String>>(Tag<String>('abc')).validate()",
            kotlin: "Wrapped<Tag<String>>(Tag(\"abc\")).validate()",
            swift: "Wrapped<Tag<String>>(value: Tag<String>(value: \"abc\")).validate()",
            name: "the default, within the bound",
            rust: Wrapped(Tag("abc".to_owned())).validate(),
        },
        Row {
            dart: "Wrapped<Tag<int>>(Tag<int>(1)).validate()",
            kotlin: "Wrapped<Tag<UInt>>(Tag(1u)).validate()",
            swift: "Wrapped<Tag<UInt32>>(value: Tag<UInt32>(value: 1)).validate()",
            name: "another filling of the class",
            rust: Wrapped(Tag(1_u32)).validate(),
        },
        Row {
            dart: "Wrapped<String>('a').validate()",
            kotlin: "Wrapped<String>(\"a\").validate()",
            swift: "Wrapped<String>(value: \"a\").validate()",
            name: "another type",
            rust: Wrapped("a".to_owned()).validate(),
        },
        Row {
            dart: "Plain<String>('ab').validate()",
            kotlin: "Plain<String>(\"ab\").validate()",
            swift: "Plain<String>(value: \"ab\").validate()",
            name: "a default written with no argument",
            rust: Plain("ab".to_owned()).validate(),
        },
        Row {
            dart: "Shelf<Slot>(by_slot: {'north-dock': Item(label: ''), 'south-dock': Item(label: 'a')}).validate()",
            kotlin: "Shelf<Slot>(mapOf(Slot.NorthDock to Item(\"\"), Slot.SouthDock to Item(\"a\"))).validate()",
            swift: "Shelf<Slot>(bySlot: [\"north-dock\": Item(label: \"\"), \"south-dock\": Item(label: \"a\")]).validate()",
            name: "a map keyed by a plain enum",
            rust: shelf(vec![(Slot::NorthDock, ""), (Slot::SouthDock, "a")]).validate(),
        },
        Row {
            dart: "Shelf<Berth>(by_slot: {'b1': Item(label: '')}).validate()",
            kotlin: "Shelf<Berth>(mapOf(Berth(\"b1\") to Item(\"\"))).validate()",
            swift: "Shelf<Berth>(bySlot: [\"b1\": Item(label: \"\")]).validate()",
            name: "a map keyed by a brand",
            rust: Shelf {
                by_slot: BTreeMap::from([(Berth("b1".to_owned()), item(""))]),
            }
            .validate(),
        },
        Row {
            dart: "Shelf<String>(by_slot: {'k': Item(label: '')}).validate()",
            kotlin: "Shelf<String>(mapOf(\"k\" to Item(\"\"))).validate()",
            swift: "Shelf<String>(bySlot: [\"k\": Item(label: \"\")]).validate()",
            name: "a map keyed by the default",
            rust: Shelf {
                by_slot: BTreeMap::from([("k".to_owned(), item(""))]),
            }
            .validate(),
        },
        Row {
            dart: "Shelf<int>(by_slot: {'7': Item(label: '')}).validate()",
            kotlin: "Shelf<UInt>(mapOf(7u to Item(\"\"))).validate()",
            swift: "Shelf<UInt32>(bySlot: [\"7\": Item(label: \"\")]).validate()",
            name: "a map keyed by a number",
            rust: Shelf {
                by_slot: BTreeMap::from([(7_u32, item(""))]),
            }
            .validate(),
        },
    ]
}

fn kotlin_string(text: &str) -> String {
    format!(
        "\"{}\"",
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
    )
}

/// What the rows print: `passes`, or `refused` and each message under it.
fn printed_by_rust(rows: &[Row]) -> String {
    let mut text = String::new();
    for row in rows {
        match &row.rust {
            Ok(()) => {
                let _ = writeln!(text, "{}: passes", row.name);
            }
            Err(messages) => {
                let _ = writeln!(text, "{}: refused", row.name);
                for message in messages {
                    let _ = writeln!(text, "    {message}");
                }
            }
        }
    }
    text
}

#[test]
fn kotlin_answers_a_generic_brands_bound_and_a_maps_key_as_rust_does() {
    let rows = rows();
    let mut main = String::from("fun main() {\n");
    for row in &rows {
        let _ = writeln!(
            main,
            "    report({}, {})",
            kotlin_string(row.name),
            row.kotlin
        );
    }
    for (name, kotlin, _) in ERASED {
        let _ = writeln!(main, "    report({}, {kotlin})", kotlin_string(name));
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        tag_kotlin::kotlin_definition(),
        wrapped_kotlin::kotlin_definition(),
        plain_kotlin::kotlin_definition(),
        slot_kotlin::kotlin_definition(),
        berth_kotlin::kotlin_definition(),
        item_kotlin::kotlin_definition(),
        shelf_kotlin::kotlin_definition(),
        REPORT.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    let mut expected = printed_by_rust(&rows);
    for (name, _, messages) in ERASED {
        let _ = writeln!(expected, "{name}: refused");
        for message in messages {
            let _ = writeln!(expected, "    {message}");
        }
    }
    assert_eq!(printed, expected);
}

/// A map keyed by a generic brand is named by the text its serializer writes when the judged
/// serializer reads it, as Rust names it, and by the class's own print for a hand-built value.
#[test]
fn kotlin_names_a_key_filled_with_a_generic_brand_as_rust_does_when_it_reads() {
    let shelf = Shelf {
        by_slot: BTreeMap::from([(Tag("t".to_owned()), item(""))]),
    };
    let messages = shelf.validate().err().unwrap_or_default();
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        tag_kotlin::kotlin_definition(),
        item_kotlin::kotlin_definition(),
        shelf_kotlin::kotlin_definition(),
        REPORT.to_owned(),
        READ.to_owned(),
        "fun main() {\n    read<Shelf<Tag<String>>>(\"read\", \"\"\"{\"by_slot\":{\"t\":{\"label\":\"\"}}}\"\"\")\n    \
         report(\"hand built\", Shelf<Tag<String>>(mapOf(Tag(\"t\") to Item(\"\"))).validate())\n}\n"
            .to_owned(),
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    let mut expected = String::from("read: refused\n");
    let mut built = String::from("hand built: refused\n");
    for message in &messages {
        let _ = writeln!(expected, "    {message}");
        let _ = writeln!(
            built,
            "    {}",
            message.replace("by_slot.t.", "by_slot.Tag(value=t).")
        );
    }
    expected.push_str(&built);
    assert_eq!(printed, expected);
}

/// A map keyed by a generic brand is named by the text its serializer writes when a judged
/// serializer reads it from a variant of a tagged and of an untagged enum, and from a type held by
/// another, and by the class's own print for a value built by hand.
#[test]
fn kotlin_names_a_key_filled_with_a_generic_brand_as_rust_does_in_a_variant_and_a_held_type() {
    let stocked = || BTreeMap::from([(Tag("t".to_owned()), item(""))]);
    let reads = [
        (
            "tagged variant",
            "Depot<Tag<String>>",
            r#"{"kind":"Stocked","by_slot":{"t":{"label":""}}}"#,
            Depot::Stocked { by_slot: stocked() }.validate(),
        ),
        (
            "untagged variant",
            "Pallet<Tag<String>>",
            r#"{"by_slot":{"t":{"label":""}}}"#,
            Pallet::Loaded { by_slot: stocked() }.validate(),
        ),
        (
            "held type",
            "Aisle",
            r#"{"shelf":{"by_slot":{"t":{"label":""}}}}"#,
            Aisle {
                shelf: Shelf { by_slot: stocked() },
            }
            .validate(),
        ),
        (
            "generic holder",
            "Annex<Tag<String>>",
            r#"{"shelf":{"by_slot":{"t":{"label":""}}}}"#,
            Annex {
                shelf: Shelf { by_slot: stocked() },
            }
            .validate(),
        ),
    ];
    let built = [
        (
            "hand built tagged variant",
            "DepotStocked<Tag<String>>(mapOf(Tag(\"t\") to Item(\"\"))).validate()",
            reads[0].3.clone(),
        ),
        (
            "hand built untagged variant",
            "PalletLoaded<Tag<String>>(mapOf(Tag(\"t\") to Item(\"\"))).validate()",
            reads[1].3.clone(),
        ),
        (
            "hand built generic holder",
            "Annex<Tag<String>>(Shelf(mapOf(Tag(\"t\") to Item(\"\")))).validate()",
            reads[3].3.clone(),
        ),
    ];
    let mut main = String::from("fun main() {\n");
    let mut expected = String::new();
    for (name, kotlin_type, payload, rust) in &reads {
        let _ = writeln!(
            main,
            "    read<{kotlin_type}>({}, \"\"\"{payload}\"\"\")",
            kotlin_string(name)
        );
        let _ = writeln!(expected, "{name}: refused");
        for message in rust.as_ref().err().into_iter().flatten() {
            let _ = writeln!(expected, "    {message}");
        }
    }
    for (name, kotlin, rust) in &built {
        let _ = writeln!(main, "    report({}, {kotlin})", kotlin_string(name));
        let _ = writeln!(expected, "{name}: refused");
        for message in rust.as_ref().err().into_iter().flatten() {
            let _ = writeln!(
                expected,
                "    {}",
                message.replace("by_slot.t.", "by_slot.Tag(value=t).")
            );
        }
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        tag_kotlin::kotlin_definition(),
        item_kotlin::kotlin_definition(),
        shelf_kotlin::kotlin_definition(),
        depot_kotlin::kotlin_definition(),
        pallet_kotlin::kotlin_definition(),
        aisle_kotlin::kotlin_definition(),
        annex_kotlin::kotlin_definition(),
        REPORT.to_owned(),
        READ.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected);
}

/// The same rows in Dart: the VM keeps the arguments of a class, so a brand written over a brand
/// is told by the type it holds.
#[cfg(feature = "dart")]
#[test]
fn dart_answers_a_generic_brands_bound_and_a_maps_key_as_rust_does() {
    use super::run_dart_validate::module;

    let rows = rows();
    let mut main = String::from("void main() {\n");
    for row in &rows {
        let _ = writeln!(main, "  report('{}', {});", row.name, row.dart);
    }
    main.push_str("}\n");
    let definitions = vec![
        tag_dart::dart_definition(),
        wrapped_dart::dart_definition(),
        plain_dart::dart_definition(),
        slot_dart::dart_definition(),
        berth_dart::dart_definition(),
        item_dart::dart_definition(),
        shelf_dart::dart_definition(),
    ];
    let Some(printed) = super::runtime::ran(
        "dart",
        "TIXSCHEMA_DART",
        "dart",
        "generic_bounds.dart",
        &module(&definitions, &[DART_REPORT], &main),
    ) else {
        return;
    };
    assert_eq!(printed, printed_by_rust(&rows));
}

/// The same rows in Swift.
#[cfg(feature = "swift")]
#[test]
fn swift_answers_a_generic_brands_bound_and_a_maps_key_as_rust_does() {
    let rows = rows();
    let mut lines = String::new();
    for row in &rows {
        let _ = writeln!(lines, "report({}, {})", kotlin_string(row.name), row.swift);
    }
    let source = [
        "import Foundation".to_owned(),
        tag_swift::swift_definition(),
        wrapped_swift::swift_definition(),
        plain_swift::swift_definition(),
        slot_swift::swift_definition(),
        berth_swift::swift_definition(),
        item_swift::swift_definition(),
        shelf_swift::swift_definition(),
        SWIFT_REPORT.to_owned(),
        lines,
    ]
    .join("\n\n");
    let Some(run) = super::runtime::judged(
        "swift-generic-bounds",
        "TIXSCHEMA_SWIFT",
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(
        run.succeeded,
        "`swift main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    assert_eq!(run.stdout, printed_by_rust(&rows));
}
