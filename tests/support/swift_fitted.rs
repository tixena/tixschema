//! What the Swift emitter writes into a type whose reads read a fixed-width integer, as the
//! emitter writes it.

/// `SwiftSchemaFit`, which reads one value and refuses an integer that does not fit, or one
/// written with a fraction or an exponent.
pub const FITTED: &str = "private struct SwiftSchemaFit<SwiftSchemaLeaf: Decodable>: Decodable { \
     let value: SwiftSchemaLeaf; \
     init(from decoder: Decoder) throws { let single = try decoder.singleValueContainer(); do { \
     value = try single.decode(SwiftSchemaLeaf.self) } catch let refused where !(refused is \
     DecodingError) { throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: \
     decoder.codingPath, debugDescription: Self.refusal(refused), underlyingError: refused)) }; \
     if let integer = value as? any FixedWidthInteger, let written = Self.written(decoder), \
     written.exponent || written.count != \"\\(integer)\".utf8.count { throw \
     DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
     debugDescription: Self.floating(try single.decode(Double.self)))) } }; \
     private static func written(_ decoder: Decoder) -> (count: Int, exponent: Bool)? { func \
     child(_ of: Any, _ label: String) -> Any? { Mirror(reflecting: of).children.first { \
     $0.label == label }?.value }; guard let values = child(decoder, \"values\") as? [Any], let \
     top = values.last, let number = child(top, \"number\") else { return nil }; let parts = \
     Mirror(reflecting: number).children.map(\\.value); guard parts.count == 2, let count = \
     child(parts[0], \"count\") as? Int, let exponent = parts[1] as? Bool else { return nil }; \
     return (count, exponent) }; private static var expected: String { [\"Int\": \
     \"isize\", \"Int8\": \"i8\", \"Int16\": \"i16\", \"Int32\": \"i32\", \"Int64\": \"i64\", \
     \"UInt8\": \"u8\", \"UInt16\": \"u16\", \"UInt32\": \"u32\"][String(describing: \
     SwiftSchemaLeaf.self)] ?? String(describing: SwiftSchemaLeaf.self) }; \
     private static func floating(_ number: Double) -> String { var spelled = \
     number.description; if let mark = spelled.firstIndex(of: \"e\") { let exponent = \
     spelled[spelled.index(after: mark)...]; let figures = exponent.drop { \"+-0\".contains($0) }; \
     spelled = String(spelled[..<mark]) + (exponent.hasPrefix(\"-\") ? \"e-\" : \"e+\") + \
     (figures.isEmpty ? \"0\" : String(figures)) }; return \"invalid type: floating point \
     `\\(spelled)`, expected \\(expected)\" }; \
     private static func refusal(_ refused: Error) -> String { let said = \"\\(refused)\"; let \
     text = \
     said.range(of: \"parsed: \\\"\").flatMap { start in said[start.upperBound...].firstIndex(of: \
     \"\\\"\").map { String(said[start.upperBound..<$0]) } } ?? said; let digits = \
     text.hasPrefix(\"-\") ? text.dropFirst() : Substring(text); if !digits.isEmpty, \
     digits.allSatisfy({ (\"0\"...\"9\").contains($0) }), text.hasPrefix(\"-\") ? Int64(text) != \
     nil : digits.count < 20 || (digits.count == 20 && digits <= \"18446744073709551615\") { \
     return \"invalid value: integer `\\(text)`, expected \\(expected)\" }; guard let number = \
     Double(text), number.isFinite else { return \"number out of range\" }; return \
     floating(number) } }";
