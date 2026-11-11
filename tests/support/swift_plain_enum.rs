//! The statements the Swift emitter writes into the read of a plain enum, as the emitter writes
//! them.

/// The key type the read of a plain enum names.
pub const ANY_KEY: &str = "private struct SwiftSchemaAnyKey: CodingKey { var stringValue: String; \
                           var intValue: Int? { nil }; init?(stringValue: String) { \
                           self.stringValue = stringValue }; init?(intValue: Int) { nil } }";

/// The statements that read `named`, a plain enum, from a string or from an object of one key
/// over `null`.
pub fn plain_enum_read(named: &str) -> String {
    format!(
        "let raw: String; var entry: SwiftSchemaAnyKey? = nil; if let single = try? \
         decoder.singleValueContainer(), let text = try? single.decode(String.self) {{ raw = text \
         }} else {{ guard let container = try? decoder.container(keyedBy: SwiftSchemaAnyKey.self), \
         container.allKeys.count == 1, let key = container.allKeys.first else {{ throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
         debugDescription: \"expected value\")) }}; entry = key; raw = key.stringValue }}; guard \
         let read = {named}(rawValue: raw) else {{ throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
         debugDescription: \"unknown variant `\\(raw)`\")) }}; if let entry, let container = try? \
         decoder.container(keyedBy: SwiftSchemaAnyKey.self), !((try? container.decodeNil(forKey: \
         entry)) ?? false) {{ throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: \
         decoder.codingPath + [entry], debugDescription: \"invalid type: expected unit\")) }}; \
         self = read"
    )
}

/// The reads of the plain enum `named` that names no check and no function: the key type, the
/// read of its own, and the one that judges.
pub fn plain_enum_reads(named: &str) -> String {
    format!(
        "{ANY_KEY}; public init(swiftSchemaUnjudged decoder: Decoder) throws {{ {} }}; public \
         init(from decoder: Decoder) throws {{ try self.init(swiftSchemaUnjudged: decoder); let \
         violations = validate(); if !violations.isEmpty {{ throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
         debugDescription: violations.joined(separator: \"; \"))) }} }};",
        plain_enum_read(named)
    )
}
