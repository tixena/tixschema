//! A name serde reads beside the one it writes, in the Swift read: a variant is read under each of
//! its aliases, under each tagging and in a plain enum, a member under each of its aliases, in a
//! struct, in a variant's payload, beside a flattened member and in each way the read reads a
//! member under its key, and each is written under the name serde writes.

use super::aliased_members_models::tholing_swift;
use super::aliased_models::{
    careening_swift, fairleading_swift, gybing_swift, hawsing_swift, lulling_swift, pennon_swift,
    scending_swift, surging_swift, swell_swift, veering_swift,
};

/// What finds the name a payload holds `label` under, of `label` and `name`, and refuses the two
/// together.
const LABEL_OR_NAME: &str = "let aliased = try decoder.container(keyedBy: \
     SwiftSchemaAnyKey.self); let labelNames = aliased.allKeys.filter { [\"label\", \
     \"name\"].contains($0.stringValue) }; if labelNames.count > 1 { throw \
     DecodingError.dataCorrupted(DecodingError.Context(codingPath: aliased.codingPath, \
     debugDescription: \"duplicate field `label`\")) }; let labelKey = labelNames.first ?? \
     SwiftSchemaAnyKey(stringValue: \"label\")!; self.label = try aliased.decode(String.self, \
     forKey: labelKey)";

/// An enum reads a variant under each of its aliases, under each tagging, a unit variant under a
/// content key from an object with no content, and writes it under the name serde writes.
#[test]
fn a_tagged_enum_reads_a_variant_under_each_alias() {
    let surging = surging_swift::swift_definition();
    for held in [
        "switch tag { case \"Calm\", \"Still\", \"Slack\": self = .calm; return;  default: break }",
        "case \"Berth\", \"Pier\": self = .berth(try \
         SurgingBerthPayload(swiftSchemaUnjudged: container.superDecoder(forKey: key)));",
        "case \"Held\", \"Tied\": self = .held(try container.decode(String.self, forKey: key));",
        "try container.encode(payload, forKey: SwiftSchemaExternalCodingKey(stringValue: \
         \"Berth\")!); case .calm: var single = encoder.singleValueContainer(); try \
         single.encode(\"Calm\");",
    ] {
        assert!(surging.contains(held), "no `{held}` in: {surging}");
    }
    let careening = careening_swift::swift_definition();
    let arms = "switch tag { case \"Berth\", \"Pier\": self = .berth(try \
                CareeningBerthPayload(swiftSchemaUnjudged: decoder)); case \"Calm\", \"Still\": \
                self = .calm;  default:";
    assert!(careening.contains(arms), "no `{arms}` in: {careening}");
    let scending = scending_swift::swift_definition();
    for held in [
        "case \"Berth\", \"Pier\": _ = try container.decodeNil(forKey: .swiftSchemaContent);",
        "case \"Calm\", \"Still\": guard !container.contains(.swiftSchemaContent) else { throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
         debugDescription: \"Scending: \\(tag) takes no value\")) }; self = .calm;",
        "case \"Held\", \"Tied\": self = .held(",
        "case .held(let payload): try container.encode(\"Held\", forKey: .swiftSchemaTag);",
    ] {
        assert!(scending.contains(held), "no `{held}` in: {scending}");
    }
}

/// A plain enum that reads a variant under an alias declares the `init?(rawValue:)` Swift then
/// synthesizes no more, which takes each variant under its raw value and each alias: the read
/// Swift synthesizes calls it, and so do a read that calls a function and the key of a map.
#[test]
fn a_plain_enum_reads_a_variant_under_each_alias() {
    assert_eq!(
        swell_swift::swift_definition(),
        "public enum Swell: String, Codable, Sendable { case ebb = \"Ebb\"; case flood = \
         \"Flood\"; public init?(rawValue: String) { switch rawValue { case \"Ebb\", \"Low\", \
         \"Neap\": self = .ebb; case \"Flood\": self = .flood; default: return nil } }; public \
         init(swiftSchemaUnjudged decoder: Decoder) throws { try self.init(from: decoder) }; \
         public func validate() -> [String] { [] }; public func swiftSchemaValidateBound() -> \
         [String] { [] } }",
        "the text of a plain enum with a variant under two aliases"
    );
    let lulling = lulling_swift::swift_definition();
    for held in [
        "public init?(rawValue: String) { switch rawValue { case \"Ebb\", \"Low\": self = .ebb; \
         case \"Flood\": self = .flood; default: return nil } };",
        "guard let read = Lulling(rawValue: raw) else {",
    ] {
        assert!(lulling.contains(held), "no `{held}` in: {lulling}");
    }
}

/// A struct reads a member under each of its aliases from the container of every key, under the
/// one name the payload holds it under, and refuses two of them together. An optional member is
/// read as it was where the payload holds none, and the write keeps the member's own key.
#[test]
fn a_struct_reads_a_member_under_each_alias() {
    let hawsing = hawsing_swift::swift_definition();
    for held in [
        "enum CodingKeys: String, CodingKey { case label; case note;  };",
        "let labelNames = aliased.allKeys.filter { [\"label\", \"name\", \
         \"title\"].contains($0.stringValue) }; if labelNames.count > 1 { throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: aliased.codingPath, \
         debugDescription: \"duplicate field `label`\")) }; let labelKey = labelNames.first ?? \
         SwiftSchemaAnyKey(stringValue: \"label\")!; self.label = try \
         aliased.decode(String.self, forKey: labelKey);",
        "let noteKey = noteNames.first ?? SwiftSchemaAnyKey(stringValue: \"note\")!; self.note = \
         try aliased.decodeIfPresent(String.self, forKey: noteKey) };",
    ] {
        assert!(hawsing.contains(held), "no `{held}` in: {hawsing}");
    }
    let gybing = gybing_swift::swift_definition();
    let held = "self.label = try aliased.decode(LabelType.self, forKey: labelKey);";
    assert!(gybing.contains(held), "no `{held}` in: {gybing}");
}

/// The struct written for a variant's members reads a member under its alias, under each tagging
/// and in an untagged enum, which tries the variant with it.
#[test]
fn a_variant_reads_a_member_under_its_alias() {
    for (named, text) in [
        ("Surging", surging_swift::swift_definition()),
        ("Careening", careening_swift::swift_definition()),
        ("Scending", scending_swift::swift_definition()),
        ("Veering", veering_swift::swift_definition()),
    ] {
        let held = format!(
            "public struct {named}BerthPayload: Codable, Sendable {{ public let label: String; \
             enum CodingKeys: String, CodingKey {{ case label;  }}; public init(label: String) \
             {{ self.label = label;  }}; private struct SwiftSchemaAnyKey: CodingKey {{ var \
             stringValue: String; var intValue: Int? {{ nil }}; init?(stringValue: String) {{ \
             self.stringValue = stringValue }}; init?(intValue: Int) {{ nil }} }}; public \
             init(swiftSchemaUnjudged decoder: Decoder) throws {{ {LABEL_OR_NAME} }};"
        );
        assert!(text.contains(&held), "no `{held}` in: {text}");
    }
}

/// A member flattened after a member under an alias is handed the object without each name the
/// member is read under, as serde hands it.
#[test]
fn a_member_flattened_after_one_under_an_alias_is_handed_neither_name() {
    let fairleading = fairleading_swift::swift_definition();
    let held = format!(
        "{LABEL_OR_NAME}; let tallyPayload = try SwiftSchemaPayload(from: decoder, without: \
         taken + [\"label\", \"name\"]).value;"
    );
    assert!(fairleading.contains(&held), "no `{held}` in: {fairleading}");
}

/// A struct with no member under an alias reads each member from the container of its own keys,
/// as it did.
#[test]
fn a_struct_with_no_alias_is_read_as_it_was() {
    assert_eq!(
        pennon_swift::swift_definition(),
        "public struct Pennon: Codable, Sendable { public let keys: [String]; enum CodingKeys: \
         String, CodingKey { case keys;  }; public init(keys: [String]) { self.keys = keys;  }; \
         public init(swiftSchemaUnjudged decoder: Decoder) throws { let container = try \
         decoder.container(keyedBy: CodingKeys.self); self.keys = try \
         container.decode([String].self, forKey: .keys) }; public init(from decoder: Decoder) \
         throws { try self.init(swiftSchemaUnjudged: decoder); let violations = validate(); if \
         !violations.isEmpty { throw DecodingError.dataCorrupted(DecodingError.Context(\
         codingPath: decoder.codingPath, debugDescription: violations.joined(separator: \"; \
         \"))) } }; public func validate() -> [String] { [] }; public func \
         swiftSchemaValidateBound() -> [String] { [] } }; ",
        "the text of a struct with no alias"
    );
}

/// A member under an alias is read in each way the read reads a member under its key, over the
/// container of every key and the key found: through the wrapper of a map, as its default where
/// the payload holds none of its names, from what its function answers, and as a model
/// `validate()` walks, optional too. The write keeps each member's own key.
#[test]
fn each_read_of_a_member_under_its_key_takes_the_key_found() {
    let tholing = tholing_swift::swift_definition();
    for held in [
        "let byTideKey = byTideNames.first ?? SwiftSchemaAnyKey(stringValue: \"by_tide\")!; let \
         wireByTide = try aliased.decode(TholingByTideKeyedMap.self, forKey: byTideKey);",
        "if aliased.contains(countKey) { self.count = try aliased.decode(UInt32.self, forKey: \
         countKey) } else { let countDefaultPayload = try JSONSerialization.jsonObject(with: \
         Data(\"0\".utf8), options: [.fragmentsAllowed]); self.count = try \
         SwiftSchemaPayload.read(UInt32.self, from: countDefaultPayload, at: aliased.codingPath + \
         [countKey]) };",
        "let markPayload = try aliased.decode(SwiftSchemaPayload.self, forKey: markKey).value; \
         self.mark = try SwiftSchemaPayload.read(String.self, from: unpadded(markPayload), at: \
         aliased.codingPath + [markKey]);",
        "if aliased.contains(spareKey), try !aliased.decodeNil(forKey: spareKey) { self.spare = \
         try Spud(swiftSchemaUnjudged: aliased.superDecoder(forKey: spareKey)) } else { \
         self.spare = nil };",
        "_ = try aliased.decodeNil(forKey: spudKey); self.spud = try \
         Spud(swiftSchemaUnjudged: aliased.superDecoder(forKey: spudKey)) };",
        "try container.encode(wireByTide, forKey: .byTide); try container.encode(self.count, \
         forKey: .count);",
    ] {
        assert!(tholing.contains(held), "no `{held}` in: {tholing}");
    }
}
