//! A name on the wire that holds a double quote or a backslash, in the Swift backend: each is
//! written between the quotes of a Swift string with the character escaped, so the file compiles
//! and the read and the write take the name serde writes. A `$` and a quote need no escape there.

use super::quoted_models::{
    UNDER_AN_ALIAS, WRITTEN, dutied_swift, excised_swift, levied_swift, mulcted_swift,
    serde_written, tariffed_swift, tolled_swift,
};

/// Asserts that `definition` holds each of `pieces`.
fn holds_each(definition: &str, pieces: &[&str]) {
    for piece in pieces {
        assert!(definition.contains(piece), "no `{piece}` in: {definition}");
    }
}

/// serde reads each payload the rows state and writes it back, and reads each alias as the name it
/// writes.
#[test]
fn serde_reads_and_writes_back_each_name_the_rows_state() {
    for (read_as, json) in WRITTEN {
        assert_eq!(
            serde_written(read_as, json),
            json,
            "serde no longer writes back {json} as it read it"
        );
    }
    for (read_as, json, back) in UNDER_AN_ALIAS {
        assert_eq!(
            serde_written(read_as, json),
            back,
            "serde no longer reads {json} as {back}"
        );
    }
}

/// A struct's members, its own tag key and its name are each written with a double quote and a
/// backslash escaped, in its keys, in the read of its tag and in `encode(to:)`.
#[test]
fn a_structs_names_are_escaped_in_its_swift_text() {
    holds_each(
        &tariffed_swift::swift_definition(),
        &[
            r#"enum CodingKeys: String, CodingKey { case backslash = "back\\slash"; case cost = "cost$usd"; case quoted = "say \"hi\"";  };"#,
            r#"private enum SwiftSchemaTagCodingKeys: String, CodingKey { case swiftSchemaTag = "ki$nd" };"#,
            r#"guard (try? tagged.decode(String.self, forKey: .swiftSchemaTag)) == "Price's$Tag\\" else"#,
            r#"try tagged.encode("Price's$Tag\\", forKey: .swiftSchemaTag);"#,
        ],
    );
}

/// An enum's variants, under each tagging, its tag key, its content key and a variant's alias are
/// each written with a double quote and a backslash escaped, in the read and in `encode(to:)`, and
/// so are a plain enum's values and a flattened enum's names.
#[test]
fn an_enums_names_are_escaped_in_its_swift_text() {
    holds_each(
        &tolled_swift::swift_definition(),
        &[
            r#"case "Calm\"": self = .still; return;"#,
            r#"case "Bid$Ask", "O'Neil": self = .bid("#,
            r#"case "C:\\Path": self = .held(try container.decode(String.self, forKey: key));"#,
            r#"forKey: SwiftSchemaExternalCodingKey(stringValue: "C:\\Path")!);"#,
            r#"try single.encode("Calm\"");"#,
        ],
    );
    holds_each(
        &levied_swift::swift_definition(),
        &[
            r#"case swiftSchemaTag = "ki'nd""#,
            r#"case "He\"ld": self = .held(try Leveed(from: decoder));"#,
            r#"case .still: try container.encode("Calm\\", forKey: .swiftSchemaTag);"#,
        ],
    );
    holds_each(
        &dutied_swift::swift_definition(),
        &[
            r#"case swiftSchemaContent = "bo'dy", swiftSchemaTag = "ki$nd""#,
            r#"case "He\"ld": self = .held(try container.decode(String.self, forKey: .swiftSchemaContent));"#,
        ],
    );
    holds_each(
        &excised_swift::swift_definition(),
        &[
            r#"case high = "hi'gh"; case low = "lo$w";"#,
            r#"case "hi'gh", "to\"p": self = .high;"#,
        ],
    );
    holds_each(
        &mulcted_swift::swift_definition(),
        &[
            r#"["Bid$Ask", "O'Neil", "C:\\Path", "Calm\""].filter"#,
            r#"try entry.encodeNil(forKey: SwiftSchemaAnyKey(stringValue: "Calm\"")!)"#,
        ],
    );
}
