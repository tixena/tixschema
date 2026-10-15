//! What the Swift backend writes for a type that declares a parameter and publishes under a name
//! other than its Rust ident: the `typealias` that republishes the ident declares each parameter
//! under the bound the published type declares it with. A type that declares no parameter
//! republishes its ident as it did. `service_schema_emitted_client_tests/run_swift_renamed_generics.rs`
//! compiles the models of `renamed_generics_models.rs` under a Swift toolchain and reads a payload
//! through each name.

use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::renamed_generics_models::{
    AdjacentScroll, Catalog, Diptych, Folio, Leaves, Lectern, Library, Margin, Quire, Scroll,
    Shelfmark, Spread, TaggedScroll, Tome, UntaggedScroll, Volume, adjacent_scroll_swift,
    catalog_swift, diptych_swift, lectern_swift, leaves_swift, library_swift, margin_swift,
    quire_swift, scroll_swift, shelfmark_swift, spread_swift, tagged_scroll_swift, tome_swift,
    untagged_scroll_swift, volume_swift,
};

/// An alias that declares no parameter.
#[model_schema()]
pub type Gathering = Vec<String>;

/// A struct that declares no parameter, renamed by `name`.
#[model_schema(name = "Verse")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stanza {
    pub line: String,
}

/// An enum that declares no parameter, renamed by `name`.
#[model_schema(name = "Passage")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Chapter {
    Opening { line: String },
}

/// A plain enum renamed by `name`.
#[model_schema(name = "Category")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Genre {
    Poetry,
}

/// A brand that declares no parameter, renamed by `name`.
#[model_schema(name = "BookNumber")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Isbn(pub String);

/// The JSON serde writes for `held`, read back by serde as it was written.
fn read_back<Held>(held: &Held) -> String
where
    Held: DeserializeOwned + Serialize,
{
    let written = serde_json::to_string(held).unwrap();
    let read: Held = serde_json::from_str(&written).unwrap();
    assert_eq!(serde_json::to_string(&read).unwrap(), written);
    written
}

/// That `swift` ends on `republished`, the one `typealias` that republishes an ident.
fn republishes(swift: &str, republished: &str) {
    assert!(
        swift.ends_with(&format!("\n\n{republished}")),
        "{republished}\nat the end of: {swift}"
    );
    assert_eq!(swift.matches(republished).count(), 1, "{swift}");
}

/// Each of `written` in `swift`.
fn holds(swift: &str, written: &[&str]) {
    for text in written {
        assert!(swift.contains(text), "{text}\nin: {swift}");
    }
}

/// A folio of `leaf` at page 1.
fn folio(leaf: &str) -> Folio<String> {
    Folio {
        leaf: leaf.to_owned(),
        page: 1,
    }
}

#[test]
fn each_model_is_written_as_serde_writes_it() {
    let leaves: Leaves<String> = vec!["a".to_owned()];
    assert_eq!(read_back(&leaves), r#"["a"]"#);
    let catalog: Catalog<String> = HashMap::from([("a".to_owned(), "b".to_owned())]);
    assert_eq!(read_back(&catalog), r#"{"a":"b"}"#);
    let volume: Volume<String> = folio("a");
    assert_eq!(read_back(&volume), r#"{"leaf":"a","page":1}"#);
    let margin: Margin<String> = Some("a".to_owned());
    assert_eq!(read_back(&margin), r#""a""#);
    let spread: Spread<String> = ("a".to_owned(), 1);
    assert_eq!(read_back(&spread), r#"["a",1]"#);
    let quire: Quire<String> = HashMap::from([(1, "a".to_owned())]);
    assert_eq!(read_back(&quire), r#"{"1":"a"}"#);
    let lectern = Lectern {
        adjacent_scroll: AdjacentScroll::Sealed("a".to_owned()),
        diptych: Diptych("b".to_owned(), 1),
        scroll: Scroll::Paired("c".to_owned(), 2),
        shelfmark: Shelfmark("d".to_owned()),
        spread: ("e".to_owned(), 3),
        tagged_scroll: TaggedScroll::Bound(folio("f")),
        tome: Tome {
            folio: folio("g"),
            leaf: "h".to_owned(),
        },
        untagged_scroll: UntaggedScroll::Sealed("i".to_owned()),
        volume: folio("j"),
    };
    assert_eq!(
        read_back(&lectern),
        r#"{"adjacent_scroll":{"kind":"Sealed","held":"a"},"diptych":["b",1],"scroll":{"Paired":["c",2]},"shelfmark":"d","spread":["e",3],"tagged_scroll":{"kind":"Bound","leaf":"f","page":1},"tome":{"folio":{"leaf":"g","page":1},"leaf":"h"},"untagged_scroll":"i","volume":{"leaf":"j","page":1}}"#
    );
    let library = Library {
        adjacent_scroll: AdjacentScroll::Unrolled {
            folio: folio("a"),
            leaf: "b".to_owned(),
        },
        catalog,
        diptych: lectern.diptych,
        leaves,
        margin: None,
        scroll: Scroll::Sealed("c".to_owned()),
        shelfmark: lectern.shelfmark,
        spread: lectern.spread,
        tagged_scroll: TaggedScroll::Unrolled {
            folio: folio("d"),
            leaf: "e".to_owned(),
        },
        tome: lectern.tome,
        untagged_scroll: UntaggedScroll::Unrolled {
            folio: folio("f"),
            leaf: "g".to_owned(),
        },
        volume: lectern.volume,
    };
    assert_eq!(
        read_back(&library),
        r#"{"adjacent_scroll":{"kind":"Unrolled","held":{"folio":{"leaf":"a","page":1},"leaf":"b"}},"catalog":{"a":"b"},"diptych":["b",1],"leaves":["a"],"margin":null,"scroll":{"Sealed":"c"},"shelfmark":"d","spread":["e",3],"tagged_scroll":{"kind":"Unrolled","folio":{"leaf":"d","page":1},"leaf":"e"},"tome":{"folio":{"leaf":"g","page":1},"leaf":"h"},"untagged_scroll":{"folio":{"leaf":"f","page":1},"leaf":"g"},"volume":{"leaf":"j","page":1}}"#
    );
}

#[test]
fn an_alias_that_declares_a_parameter_republishes_its_ident_under_the_bound() {
    for (swift, published, republished) in [
        (
            leaves_swift::swift_definition(),
            "public typealias LeavesType<Leaf: Codable & Sendable> = [Leaf];",
            "public typealias Leaves<Leaf: Codable & Sendable> = LeavesType<Leaf>;",
        ),
        (
            catalog_swift::swift_definition(),
            "public typealias CatalogType<Leaf: Codable & Sendable> = [String: Leaf];",
            "public typealias Catalog<Leaf: Codable & Sendable> = CatalogType<Leaf>;",
        ),
        (
            volume_swift::swift_definition(),
            "public typealias VolumeType<Leaf: Codable & Sendable> = Folio<Leaf>;",
            "public typealias Volume<Leaf: Codable & Sendable> = VolumeType<Leaf>;",
        ),
        (
            margin_swift::swift_definition(),
            "public typealias MarginType<Leaf: Codable & Sendable> = Leaf?;",
            "public typealias Margin<Leaf: Codable & Sendable> = MarginType<Leaf>;",
        ),
        (
            spread_swift::swift_definition(),
            "public typealias SpreadType<Leaf: Codable & Sendable> = SpreadTypeTuple<Leaf>;",
            "public typealias Spread<Leaf: Codable & Sendable> = SpreadType<Leaf>;",
        ),
        (
            quire_swift::swift_definition(),
            "public typealias QuireType<Leaf: Codable & Sendable> = [UInt32: Leaf];",
            "public typealias Quire<Leaf: Codable & Sendable> = QuireType<Leaf>;",
        ),
    ] {
        holds(&swift, &[published]);
        republishes(&swift, republished);
    }
}

#[test]
fn a_renamed_struct_republishes_its_ident_under_the_bound() {
    let tome = tome_swift::swift_definition();
    holds(
        &tome,
        &["public struct Codex<Leaf: Codable & Sendable>: Codable, Sendable { "],
    );
    republishes(
        &tome,
        "public typealias Tome<Leaf: Codable & Sendable> = Codex<Leaf>;",
    );
    let diptych = diptych_swift::swift_definition();
    holds(
        &diptych,
        &["public struct Twofold<Leaf: Codable & Sendable>: Codable, Sendable { "],
    );
    republishes(
        &diptych,
        "public typealias Diptych<Leaf: Codable & Sendable> = Twofold<Leaf>;",
    );
}

#[test]
fn a_renamed_enum_republishes_its_ident_under_the_bound_under_each_tagging() {
    for (swift, published, republished) in [
        (
            scroll_swift::swift_definition(),
            "public enum Parchment<Leaf: Codable & Sendable>: Codable, Sendable { ",
            "public typealias Scroll<Leaf: Codable & Sendable> = Parchment<Leaf>;",
        ),
        (
            tagged_scroll_swift::swift_definition(),
            "public enum TaggedParchment<Leaf: Codable & Sendable>: Codable, Sendable { ",
            "public typealias TaggedScroll<Leaf: Codable & Sendable> = TaggedParchment<Leaf>;",
        ),
        (
            adjacent_scroll_swift::swift_definition(),
            "public enum AdjacentParchment<Leaf: Codable & Sendable>: Codable, Sendable { ",
            "public typealias AdjacentScroll<Leaf: Codable & Sendable> = AdjacentParchment<Leaf>;",
        ),
        (
            untagged_scroll_swift::swift_definition(),
            "public enum UntaggedParchment<Leaf: Codable & Sendable>: Codable, Sendable { ",
            "public typealias UntaggedScroll<Leaf: Codable & Sendable> = \
             UntaggedParchment<Leaf>;",
        ),
    ] {
        holds(&swift, &[published]);
        republishes(&swift, republished);
    }
}

#[test]
fn a_renamed_brand_republishes_its_ident_under_the_bound() {
    let swift = shelfmark_swift::swift_definition();
    holds(
        &swift,
        &["public struct CallNumber<Leaf: Codable & Sendable>: Codable, Sendable { "],
    );
    republishes(
        &swift,
        "public typealias Shelfmark<Leaf: Codable & Sendable> = CallNumber<Leaf>;",
    );
}

#[test]
fn a_holder_declared_above_names_the_republished_ident_and_one_declared_below_the_published_name()
{
    holds(
        &lectern_swift::swift_definition(),
        &[
            "public struct Lectern<Leaf: Codable & Sendable>: Codable, Sendable { \
             public let adjacentScroll: AdjacentScroll<Leaf>; \
             public let diptych: Diptych<Leaf>; public let scroll: Scroll<Leaf>; \
             public let shelfmark: Shelfmark<Leaf>; public let spread: Spread<Leaf>; \
             public let taggedScroll: TaggedScroll<Leaf>; public let tome: Tome<Leaf>; \
             public let untaggedScroll: UntaggedScroll<Leaf>; public let volume: Volume<Leaf>; ",
        ],
    );
    holds(
        &library_swift::swift_definition(),
        &[
            "public struct Library<Leaf: Codable & Sendable>: Codable, Sendable { \
             public let adjacentScroll: AdjacentParchment<Leaf>; \
             public let catalog: CatalogType<Leaf>; public let diptych: Twofold<Leaf>; \
             public let leaves: LeavesType<Leaf>; public let margin: MarginType<Leaf>; \
             public let scroll: Parchment<Leaf>; public let shelfmark: CallNumber<Leaf>; \
             public let spread: SpreadType<Leaf>; \
             public let taggedScroll: TaggedParchment<Leaf>; public let tome: Codex<Leaf>; \
             public let untaggedScroll: UntaggedParchment<Leaf>; \
             public let volume: VolumeType<Leaf>; ",
        ],
    );
}

#[test]
fn an_item_that_declares_no_parameter_republishes_its_ident_as_it_did() {
    republishes(
        &gathering_swift::swift_definition(),
        "public typealias Gathering = GatheringType;",
    );
    republishes(
        &stanza_swift::swift_definition(),
        "public typealias Stanza = Verse;",
    );
    republishes(
        &chapter_swift::swift_definition(),
        "public typealias Chapter = Passage;",
    );
    republishes(
        &genre_swift::swift_definition(),
        "public typealias Genre = Category;",
    );
    republishes(
        &isbn_swift::swift_definition(),
        "public typealias Isbn = BookNumber;",
    );
    let gathering: Gathering = vec!["a".to_owned()];
    assert_eq!(read_back(&gathering), r#"["a"]"#);
    assert_eq!(read_back(&Isbn("a".to_owned())), r#""a""#);
    assert_eq!(
        read_back(&Stanza {
            line: "a".to_owned()
        }),
        r#"{"line":"a"}"#
    );
    assert_eq!(
        read_back(&Chapter::Opening {
            line: "a".to_owned()
        }),
        r#"{"kind":"Opening","line":"a"}"#
    );
    assert_eq!(read_back(&Genre::Poetry), r#""Poetry""#);
}
