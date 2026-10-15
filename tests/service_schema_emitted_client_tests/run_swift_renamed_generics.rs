//! A type that declares a parameter and publishes under a name other than its Rust ident, read and
//! written by the emitted Swift types compiled with warnings as errors: an alias of each kind, a
//! struct, a tuple struct, an enum under each tagging and a brand renamed by `name`, each through
//! the `typealias` that republishes its ident and through the name it publishes under, and a holder
//! that names each by its Rust ident, declared above them and below them. Each payload the serde
//! read reads prints what it prints.

#![cfg(feature = "swift")]

use std::collections::HashMap;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::renamed_generics_models::{
    AdjacentScroll, Catalog, Diptych, Folio, Leaves, Lectern, Library, Margin, Quire, Scroll,
    Shelfmark, Spread, TaggedScroll, Tome, UntaggedScroll, Volume, adjacent_scroll_swift, catalog_swift,
    diptych_swift, folio_swift, lectern_swift, leaves_swift, library_swift, margin_swift,
    quire_swift, scroll_swift, shelfmark_swift, spread_swift, tagged_scroll_swift, tome_swift,
    untagged_scroll_swift, volume_swift,
};
use super::run_swift_read::normalized;
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What a run reads a payload with, and prints the answer with: the value written back, or the
/// error the read threw.
const HARNESS: &str = r#"
func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    print("\(name): read \(String(data: try encoder.encode(held), encoding: .utf8)!)")
  } catch {
    print("\(name): \(error)")
  }
}
"#;

/// One payload the run reads: its name, the Swift type that reads it, the payload, and what the
/// Swift read is held to.
struct Row {
    expected: String,
    name: String,
    payload: String,
    read_as: String,
}

/// The payload serde writes for `held`, read as `read_as`, held to what the serde read prints of
/// it: the value written back.
fn read_row<Held>(name: &str, read_as: &str, held: &Held) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    let payload = serde_json::to_string(held).unwrap();
    let read: Held = serde_json::from_str(&payload).unwrap();
    Row {
        expected: format!("{name}: read {}\n", serde_json::to_string(&read).unwrap()),
        name: name.to_owned(),
        payload,
        read_as: read_as.to_owned(),
    }
}

/// [`read_row`] of `held` through the name that republishes its ident, `ident`, and through the
/// name it publishes under, `published`, each at `String`.
fn both<Held>(name: &str, ident: &str, published: &str, held: &Held) -> [Row; 2]
where
    Held: DeserializeOwned + Serialize,
{
    [
        read_row(
            &format!("{name}, by its ident"),
            &format!("{ident}<String>"),
            held,
        ),
        read_row(
            &format!("{name}, by its published name"),
            &format!("{published}<String>"),
            held,
        ),
    ]
}

/// A folio of `leaf` at page 1.
fn folio(leaf: &str) -> Folio<String> {
    Folio {
        leaf: leaf.to_owned(),
        page: 1,
    }
}

/// `leaf` as a `String`.
fn owned(leaf: &str) -> String {
    leaf.to_owned()
}

/// A tome of folio `a` and leaf `b`.
fn tome() -> Tome<String> {
    Tome {
        folio: folio("a"),
        leaf: owned("b"),
    }
}

/// The payloads of each alias, through each of its names.
fn alias_rows() -> Vec<Row> {
    let leaves: Leaves<String> = vec![owned("a"), owned("b")];
    let catalog: Catalog<String> = HashMap::from([(owned("a"), owned("b"))]);
    let volume: Volume<String> = folio("a");
    let margin: Margin<String> = Some(owned("a"));
    let spread: Spread<String> = (owned("a"), 1);
    [
        both("an alias of a list", "Leaves", "LeavesType", &leaves),
        both("an alias of a map", "Catalog", "CatalogType", &catalog),
        both("an alias of a model", "Volume", "VolumeType", &volume),
        both("an alias of an Option", "Margin", "MarginType", &margin),
        both(
            "an alias of an Option, none",
            "Margin",
            "MarginType",
            &None::<String>,
        ),
        both("an alias of a tuple", "Spread", "SpreadType", &spread),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The payloads of the struct, the tuple struct and the brand, through each of their names.
fn struct_rows() -> Vec<Row> {
    [
        both("a struct", "Tome", "Codex", &tome()),
        both(
            "a tuple struct",
            "Diptych",
            "Twofold",
            &Diptych(owned("a"), 1),
        ),
        both(
            "a brand",
            "Shelfmark",
            "CallNumber",
            &Shelfmark(owned("a")),
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The payload of every variant of each enum, through each of its names.
fn variant_rows() -> Vec<Row> {
    [
        both(
            "several slots, no tag attribute",
            "Scroll",
            "Parchment",
            &Scroll::Paired(owned("a"), 1),
        ),
        both(
            "one slot, no tag attribute",
            "Scroll",
            "Parchment",
            &Scroll::Sealed(owned("a")),
        ),
        both(
            "named members, no tag attribute",
            "Scroll",
            "Parchment",
            &Scroll::Unrolled {
                folio: folio("a"),
                leaf: owned("b"),
            },
        ),
        both(
            "one slot, a tag alone",
            "TaggedScroll",
            "TaggedParchment",
            &TaggedScroll::Bound(folio("a")),
        ),
        both(
            "named members, a tag alone",
            "TaggedScroll",
            "TaggedParchment",
            &TaggedScroll::Unrolled {
                folio: folio("a"),
                leaf: owned("b"),
            },
        ),
        both(
            "several slots, a tag and a content key",
            "AdjacentScroll",
            "AdjacentParchment",
            &AdjacentScroll::Paired(owned("a"), 1),
        ),
        both(
            "one slot, a tag and a content key",
            "AdjacentScroll",
            "AdjacentParchment",
            &AdjacentScroll::Sealed(owned("a")),
        ),
        both(
            "named members, a tag and a content key",
            "AdjacentScroll",
            "AdjacentParchment",
            &AdjacentScroll::Unrolled {
                folio: folio("a"),
                leaf: owned("b"),
            },
        ),
        both(
            "one slot, untagged",
            "UntaggedScroll",
            "UntaggedParchment",
            &UntaggedScroll::Sealed(owned("a")),
        ),
        both(
            "named members, untagged",
            "UntaggedScroll",
            "UntaggedParchment",
            &UntaggedScroll::Unrolled {
                folio: folio("a"),
                leaf: owned("b"),
            },
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The payloads of the holder declared above the types it holds and of the one declared below.
fn holder_rows() -> Vec<Row> {
    let lectern = Lectern {
        adjacent_scroll: AdjacentScroll::Paired(owned("a"), 1),
        diptych: Diptych(owned("b"), 2),
        scroll: Scroll::Unrolled {
            folio: folio("c"),
            leaf: owned("d"),
        },
        shelfmark: Shelfmark(owned("e")),
        spread: (owned("f"), 3),
        tagged_scroll: TaggedScroll::Bound(folio("g")),
        tome: tome(),
        untagged_scroll: UntaggedScroll::Sealed(owned("h")),
        volume: folio("i"),
    };
    let library = Library {
        adjacent_scroll: AdjacentScroll::Sealed(owned("a")),
        catalog: HashMap::from([(owned("b"), owned("c"))]),
        diptych: Diptych(owned("d"), 2),
        leaves: vec![owned("e")],
        margin: Some(owned("f")),
        scroll: Scroll::Paired(owned("g"), 3),
        shelfmark: Shelfmark(owned("h")),
        spread: (owned("i"), 4),
        tagged_scroll: TaggedScroll::Unrolled {
            folio: folio("j"),
            leaf: owned("k"),
        },
        tome: tome(),
        untagged_scroll: UntaggedScroll::Unrolled {
            folio: folio("l"),
            leaf: owned("m"),
        },
        volume: folio("n"),
    };
    vec![
        read_row("a holder declared above", "Lectern<String>", &lectern),
        read_row("a holder declared below", "Library<String>", &library),
    ]
}

/// Every payload the run reads: one of each type through each of its names, every variant of each
/// enum, and the two holders.
fn rows() -> Vec<Row> {
    [alias_rows(), struct_rows(), variant_rows(), holder_rows()]
        .into_iter()
        .flatten()
        .collect()
}

/// The text of every model compiles with warnings as errors, and every payload reads and writes in
/// Swift as the serde read reads and writes it, through the `typealias` that republishes the
/// type's ident under the bound and through the name the type publishes under.
#[test]
fn each_renamed_type_that_declares_a_parameter_reads_and_writes_in_swift_by_both_names() {
    let rows = rows();
    let expected: String = rows.iter().map(|row| row.expected.as_str()).collect();
    let calls: Vec<String> = rows
        .iter()
        .map(|row| {
            format!(
                "row(\"{}\", {}.self, #\"{}\"#)",
                row.name, row.read_as, row.payload
            )
        })
        .collect();
    let source = [
        "import Foundation".to_owned(),
        lectern_swift::swift_definition(),
        folio_swift::swift_definition(),
        leaves_swift::swift_definition(),
        catalog_swift::swift_definition(),
        volume_swift::swift_definition(),
        margin_swift::swift_definition(),
        spread_swift::swift_definition(),
        quire_swift::swift_definition(),
        tome_swift::swift_definition(),
        diptych_swift::swift_definition(),
        scroll_swift::swift_definition(),
        tagged_scroll_swift::swift_definition(),
        adjacent_scroll_swift::swift_definition(),
        untagged_scroll_swift::swift_definition(),
        shelfmark_swift::swift_definition(),
        library_swift::swift_definition(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-renamed-generics",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    assert_eq!(normalized(&run.stdout), normalized(&expected));
}

/// serde writes an alias of a map under a key Swift wraps as an object, which is what [`Quire`] is
/// compiled and not read for.
#[test]
fn an_alias_of_a_map_under_a_key_swift_wraps_is_written_as_an_object() {
    let quire: Quire<String> = HashMap::from([(1, owned("a"))]);
    assert_eq!(serde_json::to_string(&quire).unwrap(), r#"{"1":"a"}"#);
}
