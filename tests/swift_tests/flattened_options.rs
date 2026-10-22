//! What the Swift emitter writes of a flattened `Option`: the keys of the value it holds into its
//! holder's object, and nothing for `nil`. A struct flattened bare writes what it wrote. The writes
//! are run under the Swift toolchain in
//! `tests/service_schema_emitted_client_tests/run_swift_flattened_options.rs`.

#[cfg(feature = "chrono")]
use super::flattened_options_models::swaging_swift;
use super::flattened_options_models::{crowning_swift, flaking_swift, lapping_swift, sennit_swift};

/// Asserts that `definition` holds `fragment`.
fn assert_written(definition: &str, fragment: &str) {
    assert!(
        definition.contains(fragment),
        "missing `{fragment}` in: {definition}"
    );
}

/// A flattened `Option` writes the value it holds through the holder's encoder, after the holder's
/// own member in a struct and ahead of it in a variant under a tag key.
#[test]
fn a_flattened_option_writes_the_value_it_holds_beside_its_holders_own_members() {
    assert_written(
        &lapping_swift::swift_definition(),
        "public func encode(to encoder: Encoder) throws { var container = \
         encoder.container(keyedBy: CodingKeys.self); try container.encode(note, forKey: .note); \
         if let held = tabling { try held.encode(to: encoder) } }",
    );
    assert_written(
        &sennit_swift::swift_definition(),
        "public func encode(to encoder: Encoder) throws { var container = \
         encoder.container(keyedBy: CodingKeys.self); if let held = tabling { try \
         held.encode(to: encoder) }; try container.encode(note, forKey: .note) }",
    );
}

/// A flattened `Option` alone in its holder opens the holder's object, which it may write nothing
/// into, and a map of dates is written as the text serde writes.
#[test]
fn a_flattened_option_alone_opens_its_holders_object() {
    assert_written(
        &flaking_swift::swift_definition(),
        "public func encode(to encoder: Encoder) throws { _ = encoder.container(keyedBy: \
         CodingKeys.self); if let held = tabling { try held.encode(to: encoder) } }",
    );
    #[cfg(feature = "chrono")]
    assert_written(
        &swaging_swift::swift_definition(),
        "public func encode(to encoder: Encoder) throws { _ = encoder.container(keyedBy: \
         CodingKeys.self); if let held = stamps { try held.mapValues { \
         Self.swiftSchemaFormatIso8601Date($0) }.encode(to: encoder) } }",
    );
}

/// A struct flattened bare writes through the holder's encoder, as it wrote before a flattened
/// `Option` had a write of its own.
#[test]
fn a_struct_flattened_bare_writes_what_it_wrote() {
    assert_written(
        &crowning_swift::swift_definition(),
        "public func encode(to encoder: Encoder) throws { var container = \
         encoder.container(keyedBy: CodingKeys.self); try container.encode(note, forKey: .note); \
         try tabling.encode(to: encoder) }",
    );
}
