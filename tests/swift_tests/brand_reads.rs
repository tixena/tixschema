//! What the Swift backend writes for the read of a brand whose value is a map under a key Swift
//! reads through a wrapper: the value is read as the wrapper and handed to the brand in a closure
//! that throws, and the closure is called with `try`. A tuple struct reads its tuple through the
//! struct written for it, and writes no such closure.
//! `service_schema_emitted_client_tests/run_swift_brand_reads.rs` compiles each brand under a Swift
//! toolchain and reads a payload of it, but the brand over a list of maps, whose Swift text does
//! not compile for a reason of its own.

use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A brand over a map under a key Swift wraps.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Hoarded(pub HashMap<u32, String>);

/// A brand over a list of maps under a key Swift wraps.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Hoards(pub Vec<HashMap<u32, String>>);

/// A brand with named members over a map under a key Swift wraps.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Coffer {
    pub by_number: HashMap<u32, String>,
}

/// A brand over a map of the parameter under a key Swift wraps.
#[model_schema(default_types(Stowed = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Stash<Stowed>(pub HashMap<u32, Stowed>);

/// A tuple struct whose slot is a map under a key Swift wraps.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Trunk(pub HashMap<u32, String>, pub u8);

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

/// Each of `written` in `swift`.
fn holds(swift: &str, written: &[&str]) {
    for text in written {
        assert!(swift.contains(text), "{text}\nin: {swift}");
    }
}

/// A map of `held` under the key `1`.
fn at_one<Held>(held: Held) -> HashMap<u32, Held> {
    HashMap::from([(1, held)])
}

#[test]
fn each_brand_is_written_as_serde_writes_it() {
    let held = || at_one("a".to_owned());
    assert_eq!(read_back(&Hoarded(held())), r#"{"1":"a"}"#);
    assert_eq!(read_back(&Hoards(vec![held()])), r#"[{"1":"a"}]"#);
    assert_eq!(read_back(&Coffer { by_number: held() }), r#"{"1":"a"}"#);
    assert_eq!(read_back(&Stash(held())), r#"{"1":"a"}"#);
    assert_eq!(read_back(&Trunk(held(), 2)), r#"[{"1":"a"},2]"#);
}

#[test]
fn a_brand_calls_the_closure_that_reads_a_wrapped_map_with_try() {
    for (swift, written) in [
        (
            hoarded_swift::swift_definition(),
            "self.value = try { let wireValue = try container.decode(HoardedKeyedMap.self); \
             return wireValue.storage }() }; ",
        ),
        (
            coffer_swift::swift_definition(),
            "self.value = try { let wireValue = try container.decode(CofferKeyedMap.self); \
             return wireValue.storage }() }; ",
        ),
        (
            stash_swift::swift_definition(),
            "self.value = try { let wireValue = try container.decode(StashKeyedMap<Stowed>.self); \
             return wireValue.storage }() }; ",
        ),
    ] {
        holds(&swift, &[written]);
    }
    // The conversion of a list of maps is left unpinned: it writes a `try` nothing throws in.
    holds(
        &hoards_swift::swift_definition(),
        &[
            "self.value = try { let wireValue = try container.decode([HoardsKeyedMap].self); \
             return ",
        ],
    );
}

#[test]
fn a_tuple_struct_reads_its_tuple_through_the_struct_written_for_it() {
    let swift = trunk_swift::swift_definition();
    holds(
        &swift,
        &[
            "public init(swiftSchemaUnjudged decoder: Decoder) throws { \
             self.value = try TrunkTuple(swiftSchemaUnjudged: decoder) }; ",
        ],
    );
    assert!(!swift.contains("}()"), "got: {swift}");
}
