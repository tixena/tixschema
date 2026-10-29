//! What the Swift backend writes for a map under a key Swift wraps held as a type argument, and for
//! an alias of such a map or of a date: the argument and the alias name the wrapper that reads the
//! wire form, whose `storage` and initializer are public, and a map Swift keys natively keeps its
//! text. `service_schema_emitted_client_tests/run_swift_held_maps.rs` compiles the models of
//! `held_maps_models.rs` under a Swift toolchain and reads a payload of each.

#[cfg(feature = "chrono")]
use super::held_maps_models::moonrise_swift;
use super::held_maps_models::{
    beaconing_swift, captioning_swift, haversack_swift, plumbline_swift, pocketing_swift,
    sequencing_swift, stowed_haversack_swift, stowed_pocketing_swift, tabulation_swift,
};

/// Each of `written` in `swift`.
fn holds(swift: &str, written: &[&str]) {
    for text in written {
        assert!(swift.contains(text), "{text}\nin: {swift}");
    }
}

#[test]
fn a_map_a_type_argument_holds_is_held_by_the_wrapper_written_for_it() {
    holds(
        &haversack_swift::swift_definition(),
        &[
            "public let byCurrent: Rucksack<HaversackByCurrentArgument0KeyedMap>; ",
            "public let byFlag: Rucksack<HaversackByFlagArgument0KeyedMap>; ",
            "public let byNumber: Rucksack<HaversackByNumberArgument0KeyedMap>; ",
            "public let listed: Rucksack<[HaversackListedArgument0KeyedMap]>; ",
            "public let maybe: Rucksack<HaversackMaybeArgument0KeyedMap?>; ",
            "public struct HaversackByNumberArgument0KeyedMap: Codable, Sendable { \
             public var storage: [UInt32: String]; ",
            "public init(storage: [UInt32: String]) { self.storage = storage }; ",
        ],
    );
    holds(
        &stowed_haversack_swift::swift_definition(),
        &[
            "public let byNumber: Rucksack<StowedHaversackByNumberArgument0KeyedMap<Stowed>>; ",
            "public let held: Stowed; ",
        ],
    );
}

#[test]
fn a_map_keyed_natively_keeps_its_text_as_a_type_argument() {
    let swift = haversack_swift::swift_definition();
    holds(&swift, &["public let byName: Rucksack<[String: String]>; "]);
    assert!(!swift.contains("ByNameArgument0KeyedMap"), "{swift}");
}

#[test]
fn an_alias_of_a_map_swift_wraps_names_the_wrapper() {
    holds(
        &tabulation_swift::swift_definition(),
        &[
            "public struct TabulationTypeKeyedMap<Stowed: Codable & Sendable>: Codable, \
             Sendable { public var storage: [UInt32: Stowed]; ",
            "public typealias TabulationType<Stowed: Codable & Sendable> = \
             TabulationTypeKeyedMap<Stowed>;",
            "public typealias Tabulation<Stowed: Codable & Sendable> = TabulationType<Stowed>;",
        ],
    );
    holds(
        &sequencing_swift::swift_definition(),
        &["public typealias SequencingType = SequencingTypeKeyedMap;"],
    );
    holds(
        &beaconing_swift::swift_definition(),
        &["public typealias BeaconingType = BeaconingTypeKeyedMap;"],
    );
}

#[test]
fn an_alias_of_a_map_swift_keys_natively_keeps_its_text() {
    let swift = captioning_swift::swift_definition();
    holds(
        &swift,
        &["public typealias CaptioningType = [String: String];"],
    );
    assert!(!swift.contains("KeyedMap"), "{swift}");
}

#[cfg(feature = "chrono")]
#[test]
fn an_alias_of_a_date_names_the_wrapper_that_reads_its_text() {
    holds(
        &moonrise_swift::swift_definition(),
        &[
            "public struct MoonriseTypeDate: Codable, Sendable { public let value: Date; ",
            "public typealias MoonriseType = MoonriseTypeDate;",
        ],
    );
}

#[test]
fn a_field_typed_with_an_alias_holds_the_alias() {
    holds(
        &pocketing_swift::swift_definition(),
        &[
            "public let beaconing: BeaconingType; ",
            "public let sequencing: SequencingType; ",
            "public let tabulation: TabulationType<String>; ",
        ],
    );
    holds(
        &stowed_pocketing_swift::swift_definition(),
        &["public let tabulation: TabulationType<Stowed>; "],
    );
    holds(
        &plumbline_swift::swift_definition(),
        &[
            "public let argument: Rucksack<PlumblineArgumentArgument0KeyedMap>; ",
            "public let sounded: SoundingsType; ",
        ],
    );
}
