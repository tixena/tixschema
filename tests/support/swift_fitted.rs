//! What the Swift emitter writes into a type whose reads read a fixed-width integer, as the
//! emitter writes it.

/// `SwiftSchemaFit` and the read that reads a value again through it.
pub const FITTED: &str = "private struct SwiftSchemaFit<SwiftSchemaLeaf: Decodable>: Decodable { \
     init(from decoder: Decoder) throws { let single = try decoder.singleValueContainer(); do { \
     _ = try single.decode(SwiftSchemaLeaf.self) } catch let refused where !(refused is \
     DecodingError) { throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: \
     decoder.codingPath, debugDescription: Self.refusal(refused), underlyingError: refused)) } }; \
     private static func refusal(_ refused: Error) -> String { let said = \"\\(refused)\"; let \
     text = \
     said.range(of: \"parsed: \\\"\").flatMap { start in said[start.upperBound...].firstIndex(of: \
     \"\\\"\").map { String(said[start.upperBound..<$0]) } } ?? said; let expected = [\"Int\": \
     \"isize\", \"Int8\": \"i8\", \"Int16\": \"i16\", \"Int32\": \"i32\", \"Int64\": \"i64\", \
     \"UInt8\": \"u8\", \"UInt16\": \"u16\", \"UInt32\": \"u32\"][String(describing: \
     SwiftSchemaLeaf.self)] ?? String(describing: SwiftSchemaLeaf.self); let digits = \
     text.hasPrefix(\"-\") ? text.dropFirst() : Substring(text); if !digits.isEmpty, \
     digits.allSatisfy({ (\"0\"...\"9\").contains($0) }), text.hasPrefix(\"-\") ? Int64(text) != \
     nil : digits.count < 20 || (digits.count == 20 && digits <= \"18446744073709551615\") { \
     return \"invalid value: integer `\\(text)`, expected \\(expected)\" }; guard let number = \
     Double(text), number.isFinite else { return \"number out of range\" }; var spelled = \
     number.description; if let mark = spelled.firstIndex(of: \"e\") { let exponent = \
     spelled[spelled.index(after: mark)...]; let figures = exponent.drop { \"+-0\".contains($0) }; \
     spelled = String(spelled[..<mark]) + (exponent.hasPrefix(\"-\") ? \"e-\" : \"e+\") + \
     (figures.isEmpty ? \"0\" : String(figures)) }; return \"invalid type: floating point \
     `\\(spelled)`, expected \\(expected)\" } }; private static func swiftSchemaFitted<Read>(_ \
     read: () throws -> Read, again: () throws -> Void) throws -> Read { do { return try read() } \
     catch let refused where !(refused is DecodingError) { try again(); throw refused } }";
