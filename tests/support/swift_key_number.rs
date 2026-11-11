//! What the Swift emitter writes into the wrapper of a map keyed by a number, as the emitter writes
//! it.

/// `swiftSchemaKeyNumber`, which reads the number of a key's text as serde reads the text between
/// a key's quotes.
pub const KEY_NUMBER: &str = "private static func swiftSchemaKeyNumber(_ text: String, at \
     codingPath: [CodingKey]) throws -> (String, Bool) { let bytes = Array(text.utf8); func \
     refused(_ said: String) -> DecodingError { DecodingError.dataCorrupted(DecodingError.Context(\
     codingPath: codingPath, debugDescription: said)) }; func digit(_ at: Int) -> Bool { at < \
     bytes.count && bytes[at] >= 48 && bytes[at] <= 57 }; func digits(_ from: Int) throws -> Int { \
     guard digit(from) else { throw refused(\"invalid number\") }; var at = from; while digit(at) \
     { at += 1 }; return at }; guard let first = bytes.first, first == 45 || \
     (48...57).contains(first) else { throw refused(\"invalid value: expected key to be a number \
     in quotes\") }; let sign = first == 45 ? 1 : 0; let zero = sign < bytes.count && bytes[sign] \
     == 48; var at = try (zero ? sign + 1 : digits(sign)); if zero && digit(at) { throw \
     refused(\"invalid number\") }; var float = false; if at < bytes.count && bytes[at] == 46 { at \
     = try digits(at + 1); float = true }; if at < bytes.count && (bytes[at] == 101 || bytes[at] \
     == 69) { at = try digits(at + 1 < bytes.count && (bytes[at + 1] == 43 || bytes[at + 1] == \
     45) ? at + 2 : at + 1); float = true }; let number = String(decoding: bytes[..<at], as: \
     UTF8.self); let wide = float || (sign == 1 ? number == \"-0\" || Int64(number) == nil : \
     UInt64(number) == nil); if wide && !(Double(number)?.isFinite ?? false) { throw \
     refused(\"number out of range\") }; return (number, wide) }; ";

/// The statements that bind `key`, `parsed` from the number of the key's text, for a key serde
/// reads as an integer it names `expected`.
pub fn integer_key_bound(parsed: &str, expected: &str) -> String {
    format!(
        "let (number, wide) = try Self.swiftSchemaKeyNumber(wireKey, at: container.codingPath); \
         if wide {{ throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: \
         container.codingPath, debugDescription: \"invalid type: floating point `\\(number)`, \
         expected {expected}\")) }}; guard let key = {parsed} else {{ throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: container.codingPath + \
         [DynamicCodingKey(stringValue: number)! as CodingKey], debugDescription: \"invalid value: \
         integer `\\(number)`, expected {expected}\")) }}; if number.utf8.count < \
         wireKey.utf8.count {{ throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: \
         container.codingPath, debugDescription: \"expected `\\\"`\")) }}; "
    )
}
