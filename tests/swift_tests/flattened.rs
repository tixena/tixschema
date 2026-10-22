//! What the Swift emitter reads of a flattened member: a map entry by entry, without the keys the
//! types around its holder took, as serde hands it, and any other member after a struct without
//! the keys the struct took. A member that names a function is handed the payload without the keys
//! its holder's own members and the members flattened ahead of it took. Each flattened member is
//! written at its place. The reads are run under the Swift toolchain in
//! `tests/service_schema_emitted_client_tests/run_swift_flattened.rs`.

#[cfg(feature = "chrono")]
use super::flattened_models::serving_swift;
use super::flattened_models::{
    bowsing_swift, clewing_swift, faking_swift, footing_swift, frapping_swift, gripping_swift,
    kedging_swift, marling_swift, mousing_swift, parceling_swift, plying_swift, snaking_swift,
    splaying_swift, stopping_swift, toggling_swift, twinning_swift, worming_swift,
};

/// The read a holder takes the keys the types around it took with.
const TAKING: &str =
    "public init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: [String]) throws";

/// The read of a map flattened into `property`, keyed by text, of entries `value` reads under
/// `entry`, gathered in a `[String: {value_type}]`: of every key the types around its holder did
/// not take.
fn entries_read(property: &str, value_type: &str, value: &str) -> String {
    format!(
        "let keyed = try decoder.container(keyedBy: SwiftSchemaAnyKey.self); \
         var entries: [String: {value_type}] = [:]; \
         for entry in keyed.allKeys where !taken.contains(entry.stringValue) {{ \
         entries[entry.stringValue] = {value} }}; self.{property} = entries"
    )
}

/// The read of the member `property` that names `withItsKeys`, from the payload without `taken`.
fn handed(property: &str, taken: &str) -> String {
    format!(
        "let {property}Payload = try SwiftSchemaPayload(from: decoder, without: {taken}).value; \
         self.{property} = try SwiftSchemaPayload.read(Counting.self, \
         from: withItsKeys({property}Payload), at: decoder.codingPath)"
    )
}

/// Asserts that `definition` holds `fragment`.
fn assert_written(definition: &str, fragment: &str) {
    assert!(
        definition.contains(fragment),
        "missing `{fragment}` in: {definition}"
    );
}

/// A map that is the one member of its struct is read from every key the types around it did not
/// take, and written at its place. In a variant under a tag key the enum hands it the tag as taken.
#[test]
fn a_map_alone_is_read_without_the_keys_the_types_around_its_holder_took() {
    let stopping = stopping_swift::swift_definition();
    assert_written(
        &stopping,
        &entries_read(
            "rest",
            "UInt8",
            "try keyed.decode(UInt8.self, forKey: entry)",
        ),
    );
    assert_written(&stopping, TAKING);
    assert_written(&stopping, "try rest.encode(to: encoder)");
    let clewing = clewing_swift::swift_definition();
    assert_written(
        &clewing,
        &entries_read(
            "rest",
            "UInt8",
            "try keyed.decode(UInt8.self, forKey: entry)",
        ),
    );
    assert_written(
        &clewing,
        "case \"Held\": self = .held(try ClewingHeldPayload(swiftSchemaUnjudged: decoder, \
         swiftSchemaWithout: taken + [\"kind\"]));",
    );
}

/// A map under a key Swift wraps is read by its wrapper, through an initializer written for the
/// flattened member that reads no entry under a key taken, and written through its wrapper, in a
/// type that declares a parameter too. Its models are read unjudged, as a map Swift keys natively
/// reads them.
#[test]
fn a_map_under_a_key_swift_wraps_is_read_by_its_wrapper_without_the_keys_taken() {
    let parceling = parceling_swift::swift_definition();
    assert_written(
        &parceling,
        "self.rest = try ParcelingRestKeyedMap(swiftSchemaUnjudged: decoder, \
         swiftSchemaWithout: taken).storage",
    );
    assert_written(
        &parceling,
        "extension ParcelingRestKeyedMap { init(swiftSchemaUnjudged decoder: Decoder, \
         swiftSchemaWithout taken: [String]) throws { \
         let container = try decoder.container(keyedBy: DynamicCodingKey.self); \
         var result: [Quarter: UInt8] = [:]; \
         for codingKey in container.allKeys where !taken.contains(codingKey.stringValue) { \
         let wireKey = codingKey.stringValue; guard let key = Quarter(rawValue: wireKey) else { \
         throw DecodingError.dataCorruptedError(forKey: codingKey, in: container, \
         debugDescription: \"bad key \\(wireKey)\") }; \
         result[key] = try container.decode(UInt8.self, forKey: codingKey) }; \
         storage = result } };",
    );
    assert_written(
        &parceling,
        "try ParcelingRestKeyedMap(storage: rest).encode(to: encoder)",
    );
    let toggling = toggling_swift::swift_definition();
    assert_written(
        &toggling,
        "self.rest = try TogglingRestKeyedMap<Held>(swiftSchemaUnjudged: decoder, \
         swiftSchemaWithout: taken).storage",
    );
    assert_written(
        &toggling,
        "extension TogglingRestKeyedMap { init(swiftSchemaUnjudged decoder: Decoder, \
         swiftSchemaWithout taken: [String]) throws { \
         let container = try decoder.container(keyedBy: DynamicCodingKey.self); \
         var result: [Quarter: Held] = [:];",
    );
    assert_written(
        &faking_swift::swift_definition(),
        "result[key] = try Notch(swiftSchemaUnjudged: container.superDecoder(forKey: codingKey)) }; \
         storage = result } };",
    );
    assert_written(
        &worming_swift::swift_definition(),
        &entries_read(
            "rest",
            "Notch",
            "try Notch(swiftSchemaUnjudged: keyed.superDecoder(forKey: entry))",
        ),
    );
}

/// A map whose values Swift converts reads each from the text serde writes, and writes each back
/// as that text.
#[cfg(feature = "chrono")]
#[test]
fn a_map_of_dates_is_read_and_written_through_the_text_serde_writes() {
    let serving = serving_swift::swift_definition();
    assert_written(
        &serving,
        &entries_read(
            "rest",
            "Date",
            "try Self.swiftSchemaParseIso8601Date(wireRestEntry, codingPath: decoder.codingPath)",
        )
        .replace(
            "{ entries[",
            "{ let wireRestEntry = try keyed.decode(String.self, forKey: entry); entries[",
        ),
    );
    assert_written(
        &serving,
        "try rest.mapValues { Self.swiftSchemaFormatIso8601Date($0) }.encode(to: encoder)",
    );
}

/// A map that names a function is handed it the payload without the keys the types around its
/// holder took.
#[test]
fn a_map_that_names_a_function_hands_it_the_keys_the_types_around_took() {
    assert_written(
        &snaking_swift::swift_definition(),
        "let restPayload = try SwiftSchemaPayload(from: decoder, without: taken).value; \
         self.rest = try SwiftSchemaPayload.read([String: String].self, \
         from: asGiven(restPayload), at: decoder.codingPath)",
    );
}

/// A member that names a function is handed the payload without its holder's own keys, ahead of a
/// struct and after a member that takes no key as well. The holder reads the keys the types around
/// it took.
#[test]
fn a_function_is_handed_every_key_but_its_holders_own_where_no_member_ahead_took_one() {
    let frapping = frapping_swift::swift_definition();
    assert_written(&frapping, &handed("tally", "taken + [\"note\"]"));
    assert_written(&frapping, TAKING);
    let kedging = kedging_swift::swift_definition();
    assert_written(
        &kedging,
        &format!(
            "{}; self.note = try container.decode(String.self, forKey: .note); \
             self.pennant = try Pennant(from: decoder)",
            handed("extra", "taken + [\"note\"]")
        ),
    );
    assert_written(
        &kedging,
        "try extra.encode(to: encoder); try container.encode(note, forKey: .note); \
         try pennant.encode(to: encoder)",
    );
    for definition in [
        marling_swift::swift_definition(),
        mousing_swift::swift_definition(),
        bowsing_swift::swift_definition(),
    ] {
        assert_written(&definition, &handed("tally", "taken + [\"note\"]"));
    }
}

/// A member flattened after a struct is handed the object without the keys the struct and its
/// holder took: a struct with a flattened member of its own through the read that takes those
/// keys, and a member that names a function, a model with a rule and a struct that reads the same
/// key from the payload without them, the model read unjudged. The struct flattened first is read
/// as before.
#[test]
fn a_member_after_a_flattened_struct_is_read_without_the_keys_the_struct_took() {
    let flag = "self.flag = try Pennant(from: decoder); ";
    let note = "self.note = try container.decode(String.self, forKey: .note)";
    let ahead = format!("{flag}{note}; ");
    let without = "without: taken + [\"note\", \"label\"]).value";
    let splaying = splaying_swift::swift_definition();
    assert_written(
        &splaying,
        &format!(
            "{flag}self.footing = try Footing(swiftSchemaUnjudged: decoder, \
             swiftSchemaWithout: taken + [\"note\", \"label\"]); {note}"
        ),
    );
    assert_written(
        &footing_swift::swift_definition(),
        &handed("tally", "taken + [\"marker\"]"),
    );
    let twinning = twinning_swift::swift_definition();
    assert_written(
        &twinning,
        &format!(
            "{ahead}{}",
            handed("tally", "taken + [\"note\", \"label\"]")
        ),
    );
    let gripping = gripping_swift::swift_definition();
    assert_written(
        &gripping,
        "private struct SwiftSchemaReadNotch: Decodable { let value: Notch; \
         init(from decoder: Decoder) throws { self.value = try Notch(swiftSchemaUnjudged: decoder) } }",
    );
    assert_written(
        &gripping,
        &format!(
            "{flag}let notchPayload = try SwiftSchemaPayload(from: decoder, {without}; \
             self.notch = try SwiftSchemaPayload.read(SwiftSchemaReadNotch.self, \
             from: notchPayload, at: decoder.codingPath).value; {note}"
        ),
    );
    let plying = plying_swift::swift_definition();
    assert_written(
        &plying,
        &format!(
            "{ahead}let twicePayload = try SwiftSchemaPayload(from: decoder, {without}; \
             self.twice = try SwiftSchemaPayload.read(Pennant.self, from: twicePayload, \
             at: decoder.codingPath)"
        ),
    );
    for definition in [splaying, twinning, gripping, plying] {
        assert_written(&definition, TAKING);
        assert_written(&definition, "try flag.encode(to: encoder); try ");
    }
}
