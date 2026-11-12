//! What the Swift backend writes for a map keyed by a brand or an alias: a brand whose Rust item
//! derives `Hash` declares `Hashable`, a map's wrapper reads a brand's key through its inner type,
//! judging each brand with a check below the key's own, and writes the inner type's text of
//! `key.value`, and an alias is read as the type it names.
//! `service_schema_emitted_client_tests/run_swift_wrapped_keys.rs` compiles the models of
//! `wrapped_keys_models.rs` under a Swift toolchain and reads a payload of each.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::wrapped_keys_models::{
    adjacent_squadron_swift, armada_swift, burgee_swift, callsign_swift, dockage_swift,
    hailer_swift, hull_swift, lanyard_swift, logroll_swift, signal_swift, stowed_taskforce_swift,
    taskforce_swift, unhashed_swift,
};
#[cfg(feature = "chrono")]
use super::wrapped_keys_models::{postmark_swift, regatta_swift, stowed_regatta_swift};

/// A brand over a string with a check, which a brand with a check holds.
#[model_schema(check = ["turning"])]
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Sheave(String);

/// A brand with a check over [`Sheave`] that a map is keyed by.
#[model_schema(check = ["raised"])]
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Gaff(Sheave);

/// A map keyed by the brand over a brand.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Topmast {
    pub gaffs: HashMap<Gaff, u8>,
}

/// Each of `written` in `swift`.
fn holds(swift: &str, written: &[&str]) {
    for text in written {
        assert!(swift.contains(text), "{text}\nin: {swift}");
    }
}

#[test]
fn a_brand_that_derives_hash_declares_hashable_and_one_that_does_not_keeps_its_text() {
    for (swift, declared) in [
        (callsign_swift::swift_definition(), "Callsign"),
        (burgee_swift::swift_definition(), "Burgee"),
        (signal_swift::swift_definition(), "Signal"),
        (lanyard_swift::swift_definition(), "Lanyard"),
    ] {
        holds(
            &swift,
            &[&format!(
                "public struct {declared}: Codable, Sendable, Hashable {{ public let value: "
            )],
        );
    }
    holds(
        &unhashed_swift::swift_definition(),
        &["public struct Unhashed: Codable, Sendable { public let value: String; "],
    );
    assert!(!unhashed_swift::swift_definition().contains("Hashable"));
}

#[cfg(feature = "chrono")]
#[test]
fn a_brand_over_a_date_declares_hashable() {
    holds(
        &postmark_swift::swift_definition(),
        &["public struct Postmark: Codable, Sendable, Hashable { public let value: Date; "],
    );
}

#[test]
fn a_map_is_keyed_by_the_brand_and_by_the_type_an_alias_names() {
    holds(
        &taskforce_swift::swift_definition(),
        &[
            "public let berthing: [Berthing: UInt8]; ",
            "public let burgee: [Burgee: UInt8]; ",
            "public let callsign: [Callsign: UInt8]; ",
            "public let greeting: [String: UInt8]; ",
            "public let hailer: [Callsign: UInt8]; ",
            "public let hull: [UInt32: UInt8]; ",
            "public let lanyard: [Lanyard: UInt8]; ",
            "public let signal: [Signal: UInt8]; ",
        ],
    );
    holds(
        &stowed_taskforce_swift::swift_definition(),
        &[
            "public let hull: [UInt32: Stowed]; ",
            "public let callsign: [Callsign: Stowed]; ",
        ],
    );
    assert!(!taskforce_swift::swift_definition().contains("HullType"));
    assert!(!hull_swift::swift_definition().contains("KeyedMap"));
}

#[test]
fn a_wrapper_reads_a_brand_key_through_its_inner_and_writes_the_inner_text_of_its_value() {
    let swift = taskforce_swift::swift_definition();
    holds(
        &swift,
        &[
            "let wireKey = codingKey.stringValue; let key = Callsign(value: wireKey); \
             result[key] = try container.decode(SwiftSchemaFit<UInt8>.self, forKey: codingKey).value",
            "let wireKey = key.value; ",
            "guard let key = ((UInt32(number)).map { Burgee(value: $0) }) else { ",
            "let wireKey = String(key.value); ",
            "guard let key = (((wireKey == \"true\" ? true : wireKey == \"false\" ? false : nil))\
             .map { Signal(value: $0) }) else {",
            "let wireKey = (key.value ? \"true\" : \"false\"); ",
            "guard let key = ((((UInt32(number)).map { Burgee(value: $0) })).map { \
             Lanyard(value: $0) }) else {",
            "let wireKey = String(key.value.value); ",
        ],
    );
    assert!(!swift.contains("Callsign(rawValue:"), "{swift}");
}

#[test]
fn a_key_an_alias_names_is_read_as_the_type_it_names() {
    let swift = taskforce_swift::swift_definition();
    holds(
        &swift,
        &[
            "public struct TaskforceHullKeyedMap: Codable, Sendable { \
             public var storage: [UInt32: UInt8]; ",
            "guard let key = UInt32(number) else {",
            "public let greeting: [String: UInt8]; ",
        ],
    );
    assert!(!swift.contains("TaskforceGreetingKeyedMap"), "{swift}");
    assert!(!swift.contains("Hull(rawValue:"), "{swift}");
    assert!(hailer_swift::swift_definition().contains("public typealias HailerType = Callsign;"));
}

#[test]
fn a_plain_enum_key_keeps_its_read_and_its_write() {
    holds(
        &taskforce_swift::swift_definition(),
        &[
            "guard let key = Berthing(rawValue: wireKey) else {",
            "let wireKey = key.rawValue; ",
        ],
    );
}

#[test]
fn a_map_of_models_under_a_brand_is_walked_by_the_inner_text_and_in_its_order() {
    holds(
        &dockage_swift::swift_definition(),
        &[
            ".sorted(by: { $0.key.value < $1.key.value })",
            ".sorted(by: { $0.key < $1.key })",
            ".sorted(by: { !$0.key.value && $1.key.value })",
        ],
    );
}

#[cfg(feature = "chrono")]
#[test]
fn a_brand_over_a_date_and_an_alias_of_one_declare_the_formatters_of_a_date_key() {
    for swift in [
        regatta_swift::swift_definition(),
        stowed_regatta_swift::swift_definition(),
    ] {
        holds(
            &swift,
            &[
                "public var storage: [Date: ",
                "private static var isoFormatterForKeys: ISO8601DateFormatter {",
                "guard let key = ((Self.isoFormatterForKeys.date(from: wireKey) ?? \
                 Self.isoWholeFormatterForKeys.date(from: wireKey)).map { Postmark(value: $0) }) \
                 else {",
                "let wireKey = Self.isoFormatterForKeys.string(from: key.value)\
                 .replacingOccurrences(of: \".000Z\", with: \"Z\"); ",
                "guard let key = Self.isoFormatterForKeys.date(from: wireKey) ?? \
                 Self.isoWholeFormatterForKeys.date(from: wireKey) else {",
                "let wireKey = Self.isoFormatterForKeys.string(from: key)\
                 .replacingOccurrences(of: \".000Z\", with: \"Z\"); ",
            ],
        );
    }
}

#[test]
fn a_variant_member_and_a_generic_holder_read_the_key_the_same_way() {
    holds(
        &adjacent_squadron_swift::swift_definition(),
        &["let key = Callsign(value: wireKey); "],
    );
    holds(
        &armada_swift::swift_definition(),
        &[
            "let key = Callsign(value: wireKey); ",
            "guard let key = UInt32(number) else {",
        ],
    );
    holds(
        &logroll_swift::swift_definition(),
        &["let key = Callsign(value: wireKey); "],
    );
}

/// The check of [`Sheave`]: a sheave that turns.
fn turning(value: &Sheave) -> Result<(), Vec<String>> {
    if value.as_inner() == "seized" {
        Err(vec!["seized does not turn".to_owned()])
    } else {
        Ok(())
    }
}

/// The check of [`Gaff`]: a gaff that is raised.
fn raised(value: &Gaff) -> Result<(), Vec<String>> {
    if value.as_inner().as_inner() == "lowered" {
        Err(vec!["lowered is not raised".to_owned()])
    } else {
        Ok(())
    }
}

#[test]
fn a_wrapper_judges_the_brand_a_brand_key_holds_and_the_walk_the_key_itself() {
    let swift = topmast_swift::swift_definition();
    holds(
        &swift,
        &[
            "let key = Gaff(value: Sheave(value: wireKey)); do { let violations = \
             key.value.validate(); if !violations.isEmpty { throw \
             DecodingError.dataCorruptedError(forKey: codingKey, in: container, debugDescription: \
             violations.joined(separator: \"; \")) } }; result[key] = ",
            "errors += key.validate().map { Topmast.swiftSchemaUnder(\"gaffs.\\(key.value.value)\", $0) }",
        ],
    );
    assert!(
        !taskforce_swift::swift_definition().contains("key.value.validate()"),
        "a brand over a brand with no check judges nothing"
    );
}

/// The Rust reference the Swift read is held to: serde refuses a key either brand's check refuses
/// as it reads the key, and the outer brand's `validate()` runs its own check alone.
#[test]
fn serde_judges_each_brand_of_a_key_as_it_reads_it_and_validate_the_brand_s_own_check() {
    for refused in [r#"{"gaffs":{"seized":1}}"#, r#"{"gaffs":{"lowered":1}}"#] {
        assert!(
            serde_json::from_str::<Topmast>(refused).is_err(),
            "{refused}"
        );
    }
    let raised = Topmast {
        gaffs: HashMap::from([(
            Gaff::new(Sheave::new("rope".to_owned()).unwrap()).unwrap(),
            1,
        )]),
    };
    assert_eq!(
        serde_json::to_string(&raised).unwrap(),
        r#"{"gaffs":{"rope":1}}"#
    );
    assert_eq!(
        Gaff::new(
            Sheave::model_schema_read_unjudged(serde_json::to_value("seized".to_owned()).unwrap())
                .unwrap()
        )
        .map(|_| ()),
        Ok(())
    );
    assert_eq!(
        Gaff::new(Sheave::new("lowered".to_owned()).unwrap()).map(|_| ()),
        Err(vec!["lowered is not raised".to_owned()])
    );
}
