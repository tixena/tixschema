//! The statements the Swift emitter writes into the read of a plain enum, as the emitter writes
//! them.

/// The key type the read of a plain enum names.
pub const ANY_KEY: &str = "private struct SwiftSchemaAnyKey: CodingKey { var stringValue: String; \
                           var intValue: Int? { nil }; init?(stringValue: String) { \
                           self.stringValue = stringValue }; init?(intValue: Int) { nil } }";

/// The broken rules `violations` names, each placed under the coding path the value was read at.
pub const PLACED: &str = concat!(
    r#"{ () -> [String] in let at = ((decoder.userInfo[CodingUserInfoKey(rawValue: "swiftSchemaOuter")!] as? [String] ?? []) + decoder.codingPath.map { $0.intValue.map(String.init) ?? "#,
    r#"$0.stringValue }).joined(separator: "."); return at.isEmpty ? violations : violations.map { "#,
    r#"said in guard said.hasPrefix("'"), let end = said.dropFirst().firstIndex(of: "'") else { "#,
    r#"return "'\(at)': \(said)" }; return "'\(at).\(said[said.index(after: "#,
    r#"said.startIndex)..<end])'\(said[said.index(after: end)...])" } }()"#,
);

/// The struct the text of the type published as `named` opens with, which its reads throw.
pub fn refusal(named: &str) -> String {
    format!(
        "public struct {named}Refusal: Error, CustomStringConvertible {{ public let issues: \
         [String]; public var description: String {{ \"{named}Refusal: [\\(issues.joined(separator: \", \"))]\" }} }};"
    )
}

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

/// The reads of a plain enum `named` that names no check, declared as written: the key type, the
/// read of its own, and the one that judges what a type with no rule cannot refuse.
pub fn plain_enum_reads(named: &str) -> String {
    format!(
        "{ANY_KEY}; public init(swiftSchemaUnjudged decoder: Decoder) throws {{ {} }}; public \
         init(from decoder: Decoder) throws {{ do {{ try self.init(swiftSchemaUnjudged: decoder) }} catch {{ guard let held = Mirror(reflecting: error).children.first(where: {{ $0.label == \"issues\" }})?.value as? [String] else {{ throw error }}; throw {named}Refusal(issues: held) }}; let \
         violations = validate(); if !violations.isEmpty {{ throw {named}Refusal(issues: \
         {PLACED}) }} }};",
        plain_enum_read(named)
    )
}
