//! A member serde reads and never writes: the Swift read takes it, and `encode(to:)` is written
//! with no key for it, in a struct of its own and beside a flattened member.

use super::unwritten_models::furling_swift;
#[cfg(feature = "serde")]
use super::unwritten_models::{flemishing_swift, ranging_swift, reefing_swift, splaying_swift};

/// The member is read, and the `encode(to:)` written in place of Swift's own encodes no key for
/// it.
#[test]
fn a_member_serde_never_writes_is_read_and_not_encoded() {
    let furling = furling_swift::swift_definition();
    assert!(
        furling.contains("self.spare = try container.decodeIfPresent(SwiftSchemaFit<UInt32>.self, forKey: .spare)?.value"),
        "got: {furling}"
    );
    assert!(
        furling.contains(
            "public func encode(to encoder: Encoder) throws { var container = \
             encoder.container(keyedBy: CodingKeys.self); try container.encode(self.label, \
             forKey: .label) };"
        ),
        "got: {furling}"
    );
}

/// Beside a flattened member, the member is read, and `encode(to:)` opens the object and writes
/// the flattened member alone.
#[cfg(feature = "serde")]
#[test]
fn a_member_serde_never_writes_beside_a_flattened_one_is_not_encoded() {
    let reefing = reefing_swift::swift_definition();
    assert!(
        reefing.contains("self.spare = try container.decodeIfPresent(SwiftSchemaFit<UInt32>.self, forKey: .spare)?.value"),
        "got: {reefing}"
    );
    assert!(
        reefing.contains(
            "public func encode(to encoder: Encoder) throws { _ = encoder.container(keyedBy: \
             CodingKeys.self); try self.braiding.encode(to: encoder) };"
        ),
        "got: {reefing}"
    );
}

/// A flattened struct and a flattened map serde never writes are read, and `encode(to:)` encodes
/// nothing of them, the object opened all the same. One that is no `Option` is read as a flattened
/// member is, its key being none to leave out.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_member_serde_never_writes_is_read_and_not_encoded() {
    let encoded_note = "public func encode(to encoder: Encoder) throws { var container = \
                        encoder.container(keyedBy: CodingKeys.self); try \
                        container.encode(self.note, forKey: .note) };";
    let flemishing = flemishing_swift::swift_definition();
    for held in [
        "self.braiding = try Braiding(swiftSchemaUnjudged: decoder, swiftSchemaWithout: taken + [\"note\"]) } else { self.braiding = nil }",
        encoded_note,
    ] {
        assert!(flemishing.contains(held), "no `{held}` in: {flemishing}");
    }
    let splaying = splaying_swift::swift_definition();
    for held in [
        "public let braiding: Braiding;",
        "do { self.braiding = try Braiding(swiftSchemaUnjudged: decoder, \
         swiftSchemaWithout: taken + [\"note\"]) } catch let refused as DecodingError { \
         throw Self.swiftSchemaFlattened(refused, at: decoder.codingPath) }; self.note = \
         try container.decode(String.self, forKey: .note) }; public \
         init(swiftSchemaUnjudged decoder: Decoder) throws { if \
         !(decoder.codingPath.isEmpty && decoder.userInfo[CodingUserInfoKey(rawValue: \
         \"swiftSchemaFlat\")!] != nil) { for unknown in try decoder.container(keyedBy: \
         SwiftSchemaAnyKey.self).allKeys where ![\"label\", \
         \"note\"].contains(unknown.stringValue) { throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath + \
         [unknown], debugDescription: \"unknown field `\\(unknown.stringValue)`\")) } }; \
         try self.init(swiftSchemaUnjudged: decoder, swiftSchemaWithout: []) }; public \
         init(from decoder: Decoder) throws { try self.init(swiftSchemaUnjudged: decoder);",
        encoded_note,
    ] {
        assert!(splaying.contains(held), "no `{held}` in: {splaying}");
    }
    let ranging = ranging_swift::swift_definition();
    for held in [
        "public let counts: [String: UInt8];",
        "for entry in keyed.allKeys where !taken.contains(entry.stringValue) { \
         entries[entry.stringValue] = try keyed.decode(SwiftSchemaFit<UInt8>.self, forKey: entry).value }; self.counts = \
         entries } catch let refused as DecodingError { throw \
         Self.swiftSchemaFlattened(refused, at: decoder.codingPath) } };",
        "public func encode(to encoder: Encoder) throws { _ = encoder.container(keyedBy: \
         CodingKeys.self);  };",
    ] {
        assert!(ranging.contains(held), "no `{held}` in: {ranging}");
    }
}
