//! What the Swift emitter reads of a flattened member: a map entry by entry, without the keys the
//! types around its holder took, as serde hands it, an optional one `nil` wherever an entry does
//! not read, and any other member without the keys its holder's own members, the structs
//! flattened ahead of it and the types around its holder took. An externally tagged enum is read
//! from the one entry under a variant's name, which no member flattened after it is handed. Each
//! flattened member is written at its place. What a variant under a tag alone holds as its one
//! slot or flattens beside no member of its own is read from the object without the tag. The reads
//! are run under the Swift toolchain in `tests/service_schema_emitted_client_tests/`, in
//! `run_swift_flattened.rs` and `run_swift_flattened_enums.rs`.

#[cfg(feature = "chrono")]
use super::flattened_models::serving_swift;
use super::flattened_models::{
    anchoring_swift, baling_swift, bowsing_swift, boxing_swift, clewing_swift, dredging_swift,
    faking_swift, fending_swift, footing_swift, frapping_swift, grafting_swift, gripping_swift,
    halyard_swift, hanking_swift, kedging_swift, marling_swift, marrying_swift, mousing_swift,
    parceling_swift, plying_swift, pointing_swift, ratlining_swift, reeling_swift, seaming_swift,
    slacking_swift, snaking_swift, splaying_swift, stopping_swift, stropping_swift, tailing_swift,
    toggling_swift, trussing_swift, twinning_swift, whipping_swift, winching_swift, worming_swift,
};

/// The read a holder takes the keys the types around it took with.
const TAKING: &str =
    "public init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: [String]) throws";

/// The statements that read `Careen` unjudged from the entry found, a unit variant over `null`
/// as its name and an alias under its variant's name.
const CAREEN_READ: &str = "let careenPayload: Any; switch careenKey { \
     case \"Calm\" where careenHeld is NSNull: careenPayload = \"Calm\"; \
     case \"Quay\": careenPayload = [\"Dock\": careenHeld]; \
     default: careenPayload = [careenKey: careenHeld] }; \
     self.careen = try SwiftSchemaPayload.read(SwiftSchemaReadCareen.self, from: careenPayload, \
     at: decoder.codingPath).value";

/// The entry found, where the payload must hold one.
const CAREEN_HELD: &str = "guard let careenKey = careenFound.first, \
     let careenHeld = careenObject[careenKey] else { \
     throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
     debugDescription: \"no variant of enum Careen found in flattened data\")) }; ";

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

/// The read of the `Pennant` flattened into `property`, from the payload without `taken`.
fn pennant(property: &str, taken: &str) -> String {
    format!(
        "let {property}Payload = try SwiftSchemaPayload(from: decoder, without: {taken}).value; \
         self.{property} = try SwiftSchemaPayload.read(Pennant.self, \
         from: {property}Payload, at: decoder.codingPath)"
    )
}

/// Asserts that `definition` holds `fragment`.
fn assert_written(definition: &str, fragment: &str) {
    assert!(
        definition.contains(fragment),
        "missing `{fragment}` in: {definition}"
    );
}

/// Asserts that `definition`, whose structs read and write every member flattened, declares no
/// container of their keys.
fn assert_no_keyed_container(definition: &str) {
    assert!(
        !definition.contains("keyedBy: CodingKeys.self"),
        "a container of the keys in: {definition}"
    );
}

/// A map that is the one member of its struct is read from every key the types around it did not
/// take, and written at its place. In a variant written under its name the enum hands it the
/// variant's object. Neither reads or writes a key of its own, so neither declares a container of
/// its keys.
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
    assert_written(&stopping, &format!("{TAKING} {{ let keyed = "));
    assert_written(
        &stopping,
        "public func encode(to encoder: Encoder) throws { try self.rest.encode(to: encoder) }",
    );
    assert_no_keyed_container(&stopping);
    let clewing = clewing_swift::swift_definition();
    assert_written(
        &clewing,
        &entries_read(
            "rest",
            "UInt8",
            "try keyed.decode(UInt8.self, forKey: entry)",
        ),
    );
    assert_written(&clewing, &format!("{TAKING} {{ let keyed = "));
    assert_written(
        &clewing,
        "public func encode(to encoder: Encoder) throws { try self.rest.encode(to: encoder) }",
    );
    assert_no_keyed_container(&clewing);
    assert_written(
        &clewing,
        "case \"Held\": self = .held(try ClewingHeldPayload(swiftSchemaUnjudged: \
         container.superDecoder(forKey: key)));",
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
        "try ParcelingRestKeyedMap(storage: self.rest).encode(to: encoder)",
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
        "try self.rest.mapValues { Self.swiftSchemaFormatIso8601Date($0) }.encode(to: encoder)",
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
            "{}; self.note = try container.decode(String.self, forKey: .note); {}",
            handed("extra", "taken + [\"note\"]"),
            pennant("pennant", "taken + [\"note\"]")
        ),
    );
    assert_written(
        &kedging,
        "try self.extra.encode(to: encoder); try container.encode(self.note, forKey: .note); \
         try self.pennant.encode(to: encoder)",
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
/// from the payload without its holder's own keys.
#[test]
fn a_member_after_a_flattened_struct_is_read_without_the_keys_the_struct_took() {
    let flag = format!("{}; ", pennant("flag", "taken + [\"note\"]"));
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
        assert_written(&definition, "try self.flag.encode(to: encoder); try ");
    }
}

/// A struct flattened first is read from the payload without its holder's own keys, plain and
/// through the function it names, and so is a struct beside a member of its holder's own whose
/// key it does not read. In a holder of no member of its own it is read without the keys the
/// types around the holder took, where another read of the holder takes them, and the holder
/// declares no container of its keys.
#[test]
fn a_struct_flattened_first_is_read_without_its_holders_own_keys() {
    let label = "self.label = try container.decode(String.self, forKey: .label)";
    let whipping = whipping_swift::swift_definition();
    assert_written(
        &whipping,
        &format!("{}; {label}", pennant("flag", "taken + [\"label\"]")),
    );
    assert_written(&whipping, TAKING);
    assert_written(
        &marrying_swift::swift_definition(),
        &format!(
            "let flagPayload = try SwiftSchemaPayload(from: decoder, \
             without: taken + [\"label\"]).value; \
             self.flag = try SwiftSchemaPayload.read(Pennant.self, from: asGiven(flagPayload), \
             at: decoder.codingPath); {label}"
        ),
    );
    let hanking = hanking_swift::swift_definition();
    assert_written(
        &hanking,
        &format!(
            "self.marker = try container.decode(String.self, forKey: .marker); {}",
            pennant("pennant", "taken + [\"marker\"]")
        ),
    );
    assert_written(&hanking, TAKING);
    assert_written(
        &bowsing_swift::swift_definition(),
        "self.hanking = try Hanking(swiftSchemaUnjudged: decoder, \
         swiftSchemaWithout: taken + [\"note\"])",
    );
    let stropping = stropping_swift::swift_definition();
    assert_written(
        &stropping,
        &format!(
            "{}; {}",
            pennant("flag", "taken"),
            handed("tally", "taken + [\"label\"]")
        ),
    );
    assert_written(&stropping, &format!("{TAKING} {{ let flagPayload = "));
    assert_written(
        &stropping,
        "public func encode(to encoder: Encoder) throws { \
         try self.flag.encode(to: encoder); try self.tally.encode(to: encoder) }",
    );
    assert_no_keyed_container(&stropping);
    assert_written(
        &ratlining_swift::swift_definition(),
        "self.stropping = try Stropping(swiftSchemaUnjudged: decoder, \
         swiftSchemaWithout: taken + [\"label\"])",
    );
}

/// A struct whose every member is a flattened struct reads the first from the payload without the
/// keys the types around it took, and the second, a model with a rule, unjudged and without the
/// keys the first took too. It writes each through its encoder and declares no container of its
/// keys.
#[test]
fn a_struct_of_flattened_structs_alone_declares_no_container_of_its_keys() {
    let seaming = seaming_swift::swift_definition();
    assert_written(
        &seaming,
        &format!(
            "{TAKING} {{ {}; let notchPayload = try SwiftSchemaPayload(from: decoder, \
             without: taken + [\"label\"]).value; \
             self.notch = try SwiftSchemaPayload.read(SwiftSchemaReadNotch.self, \
             from: notchPayload, at: decoder.codingPath).value }}",
            pennant("flag", "taken")
        ),
    );
    assert_written(
        &seaming,
        "public func encode(to encoder: Encoder) throws { \
         try self.flag.encode(to: encoder); try self.notch.encode(to: encoder) }",
    );
    assert_no_keyed_container(&seaming);
}

/// A member named after the container a written `encode(to:)` declares is read through `self.`,
/// a member of the holder's own as the value it encodes and a flattened one as what it encodes
/// through, so the method never reads the container in its place.
#[test]
fn a_member_named_after_the_container_is_written_through_self() {
    assert_written(
        &boxing_swift::swift_definition(),
        "public func encode(to encoder: Encoder) throws { var container = \
         encoder.container(keyedBy: CodingKeys.self); try container.encode(self.container, \
         forKey: .container); try self.flag.encode(to: encoder) }",
    );
    assert_written(
        &baling_swift::swift_definition(),
        "public func encode(to encoder: Encoder) throws { var container = \
         encoder.container(keyedBy: CodingKeys.self); try self.container.encode(to: encoder); \
         try container.encode(self.note, forKey: .note) }",
    );
}

/// The read of the variant `wire` into the case `case` of a value `held` reads from the object
/// without the tag, under the local `{case}Payload`.
fn without_the_tag(wire: &str, case: &str, held: &str) -> String {
    format!(
        "case \"{wire}\": let {case}Payload = try SwiftSchemaPayload(from: decoder, \
         without: taken + [\"kind\"]).value; self = .{case}(try SwiftSchemaPayload.read({held}, \
         from: {case}Payload, at: decoder.codingPath)); "
    )
}

/// What a variant under a tag alone holds as its one slot is read from the object without the
/// tag, through a type written for it where `validate()` walks the slot, so a model is still read
/// unjudged. The struct written for a variant whose every member is flattened takes the keys the
/// types around it took, the tag among them, and reads each member from the object without them;
/// one with a member of its own reads as it did.
#[test]
fn what_a_variant_under_a_tag_alone_holds_is_read_without_the_tag() {
    let handed = |payload: &str, case: &str, wire: &str| {
        format!(
            "case \"{wire}\": self = .{case}(try {payload}(swiftSchemaUnjudged: decoder, \
             swiftSchemaWithout: taken + [\"kind\"])); "
        )
    };
    let grafting = grafting_swift::swift_definition();
    assert_written(
        &grafting,
        &format!(
            "{TAKING} {{ let flagPayload = try SwiftSchemaPayload(from: decoder, \
             without: taken).value; self.flag = try SwiftSchemaPayload.read(Kinded.self, \
             from: flagPayload, at: decoder.codingPath) }}"
        ),
    );
    assert_written(&grafting, &handed("GraftingHeldPayload", "held", "Held"));
    assert_written(&grafting, &without_the_tag("Slot", "slot", "Kinded.self"));
    assert_no_keyed_container(&grafting);
    let pointing = pointing_swift::swift_definition();
    for (payload, case, wire) in [
        ("PointingHeldPayload", "held", "Held"),
        ("PointingMarkedPayload", "marked", "Marked"),
        ("PointingNotedPayload", "noted", "Noted"),
    ] {
        assert_written(&pointing, &handed(payload, case, wire));
    }
    assert_written(
        &pointing,
        &format!(
            "{TAKING} {{ let notchPayload = try SwiftSchemaPayload(from: decoder, \
             without: taken).value; self.notch = try SwiftSchemaPayload.read(\
             SwiftSchemaReadNotch.self, from: notchPayload, at: decoder.codingPath).value }}"
        ),
    );
    assert_written(
        &pointing,
        &format!(
            "{TAKING} {{ let container = try decoder.container(keyedBy: CodingKeys.self); {}",
            pennant("flag", "taken + [\"note\"]")
        ),
    );
    assert_written(
        &pointing,
        "private struct SwiftSchemaReadNotched: Decodable { let value: Notch; \
         init(from decoder: Decoder) throws { self.value = try Notch(swiftSchemaUnjudged: decoder) } }",
    );
    assert_written(
        &pointing,
        &without_the_tag("Notched", "notched", "SwiftSchemaReadNotched.self")
            .replace("decoder.codingPath))", "decoder.codingPath).value)"),
    );
    assert_written(&pointing, &without_the_tag("Slot", "slot", "Pennant.self"));
}

/// An optional map, the one member of its struct or of a variant under its name, under a content
/// key or untagged, is read entry by entry as a map is, and is `nil` wherever an entry does not
/// read: under a key Swift wraps too, and of models read with no judgement. `nil` writes nothing.
#[test]
fn an_optional_map_is_nil_wherever_an_entry_does_not_read() {
    let trussing = trussing_swift::swift_definition();
    let counts = format!(
        "{TAKING} {{ do {{ {} }} catch {{ self.counts = nil }} }}",
        entries_read(
            "counts",
            "UInt8",
            "try keyed.decode(UInt8.self, forKey: entry)",
        )
    );
    assert_written(&trussing, &counts);
    assert_written(
        &trussing,
        "public func encode(to encoder: Encoder) throws { _ = encoder.container(keyedBy: \
         CodingKeys.self); if let held = self.counts { try held.encode(to: encoder) } }",
    );
    let halyard = halyard_swift::swift_definition();
    assert_written(&halyard, &counts);
    assert_written(
        &halyard,
        "case \"Open\": self = .open(try HalyardOpenPayload(swiftSchemaUnjudged: \
         container.superDecoder(forKey: key)));",
    );
    let tailing = tailing_swift::swift_definition();
    assert_written(&tailing, &counts);
    assert_written(
        &tailing,
        "self = .open(try TailingOpenPayload(swiftSchemaUnjudged: \
         container.superDecoder(forKey: .swiftSchemaContent)));",
    );
    let slacking = slacking_swift::swift_definition();
    assert_written(&slacking, &counts);
    assert_written(
        &slacking,
        "if let value = try? SlackingOpenPayload(swiftSchemaUnjudged: decoder, \
         swiftSchemaWithout: taken) { self = .open(value); return };",
    );
    let reeling = reeling_swift::swift_definition();
    assert_written(
        &reeling,
        &format!(
            "{TAKING} {{ do {{ self.counts = try ReelingCountsKeyedMap(swiftSchemaUnjudged: \
             decoder, swiftSchemaWithout: taken).storage }} catch {{ self.counts = nil }} }}"
        ),
    );
    assert_written(
        &reeling,
        "if let held = self.counts { try ReelingCountsKeyedMap(storage: held).encode(to: encoder) }",
    );
    assert_written(
        &winching_swift::swift_definition(),
        &format!(
            "{TAKING} {{ do {{ {} }} catch {{ self.notches = nil }} }}",
            entries_read(
                "notches",
                "Notch",
                "try Notch(swiftSchemaUnjudged: keyed.superDecoder(forKey: entry))",
            )
        ),
    );
}

/// The statements that find the entry of `Careen` the member `careen` takes in the object
/// without `taken`, and refuse entries under two names.
fn careen_found(taken: &str) -> String {
    format!(
        "let careenObject = try SwiftSchemaPayload(from: decoder, without: {taken}).value \
         as? [String: Any] ?? [:]; \
         let careenFound = [\"Bay\", \"Calm\", \"Dock\", \"Quay\"]\
         .filter {{ careenObject[$0] != nil }}; \
         if careenFound.count > 1 {{ throw DecodingError.dataCorrupted(DecodingError.Context(\
         codingPath: decoder.codingPath, debugDescription: \"more than one variant of enum Careen \
         found in flattened data: \\(careenFound.joined(separator: \", \"))\")) }}; "
    )
}

/// A flattened externally tagged enum is read from the one entry of the object it is handed under
/// a variant's name or an alias, and refused where it holds two or none, or, optional, read as
/// `nil` where it holds none and wherever the enum's read throws. Its model is read unjudged, and
/// a member flattened after it is handed the object without that entry.
#[test]
fn a_flattened_enum_is_read_from_the_one_entry_under_a_variants_name() {
    let unjudged = "private struct SwiftSchemaReadCareen: Decodable { let value: Careen; \
         init(from decoder: Decoder) throws { \
         self.value = try Careen(swiftSchemaUnjudged: decoder) } }";
    let anchoring = anchoring_swift::swift_definition();
    assert_written(&anchoring, unjudged);
    assert_written(
        &anchoring,
        &format!(
            "{TAKING} {{ let container = try decoder.container(keyedBy: CodingKeys.self); \
             {}{CAREEN_HELD}{CAREEN_READ}; \
             self.note = try container.decode(String.self, forKey: .note) }}",
            careen_found("taken + [\"note\"]")
        ),
    );
    let dredging = dredging_swift::swift_definition();
    assert_written(
        &dredging,
        &format!(
            "{TAKING} {{ {}{CAREEN_HELD}{CAREEN_READ}; {} }}",
            careen_found("taken"),
            handed("tally", "taken + careenFound")
        ),
    );
    assert_written(
        &fending_swift::swift_definition(),
        &format!(
            "{TAKING} {{ let container = try decoder.container(keyedBy: CodingKeys.self); {}\
             if let careenKey = careenFound.first, let careenHeld = careenObject[careenKey] {{ \
             do {{ {CAREEN_READ} }} catch {{ self.careen = nil }} }} \
             else {{ self.careen = nil }}; \
             self.note = try container.decode(String.self, forKey: .note) }}",
            careen_found("taken + [\"note\"]")
        ),
    );
}
