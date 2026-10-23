//! A name on the wire that holds a `$`, a double quote or a backslash, in the Kotlin backend: each
//! is written between the quotes of a Kotlin string with the character escaped, so the file
//! compiles and the read and the write take the name serde writes. A quote needs no escape there.

use super::quoted_models::{
    UNDER_AN_ALIAS, WRITTEN, dutied_kotlin, excised_kotlin, levied_kotlin, mulcted_kotlin,
    serde_written, tariffed_kotlin, tithed_kotlin, tolled_kotlin,
};

/// Asserts that `definition` holds each of `pieces`.
fn holds_each(definition: &str, pieces: &[&str]) {
    let types = super::tests::without_number_readers(definition);
    for piece in pieces {
        assert!(types.contains(piece), "no `{piece}` in: {definition}");
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

/// A struct's members, its own tag key and its name are each written with a `$`, a double quote
/// and a backslash escaped, in the names the plugin reads, in the read of its tag and in the write
/// that moves the tag ahead of the members.
#[test]
fn a_structs_names_are_escaped_in_its_kotlin_text() {
    holds_each(
        &tariffed_kotlin::kotlin_definition(),
        &[
            r#"@SerialName("back\\slash") val backslash: String"#,
            r#"@SerialName("cost\$usd") val cost: String"#,
            r#"@SerialName("say \"hi\"") val quoted: String"#,
            r#"@SerialName("ki\$nd") @EncodeDefault val kotlinSchemaTag: String = "Price's\$Tag\\";"#,
            r#"JsonObject(mapOf("ki\$nd" to element.jsonObject.getValue("ki\$nd")) + element.jsonObject)"#,
            r#"JsonPrimitive("Price's\$Tag\\") -> {}"#,
        ],
    );
}

/// An enum's variants, under each tagging, its tag key, its content key and a variant's alias are
/// each written with a `$`, a double quote and a backslash escaped, in the read and in the write,
/// and so are a plain enum's values and the names a flattened member is read under.
#[test]
fn an_enums_names_are_escaped_in_its_kotlin_text() {
    holds_each(
        &tolled_kotlin::kotlin_definition(),
        &[
            r#"@SerialName("lo\$t") val lot: UByte"#,
            r#"put("Bid\$Ask", output.json.encodeToJsonElement(serializer<TolledBid>(), value))"#,
            r#"put("C:\\Path", output.json.encodeToJsonElement(serializer<String>(), value.value))"#,
            r#"JsonPrimitive("Calm\"")"#,
            r#"when (tag) { "Bid\$Ask", "O'Neil" -> "#,
        ],
    );
    holds_each(
        &levied_kotlin::kotlin_definition(),
        &[
            r#"@SerialName("Bid\$Ask") @Serializable data class LeviedBid(@SerialName("lo\$t") val"#,
            r#"@SerialName("He\"ld") @Serializable data class LeviedHeld("#,
            r#"@SerialName("Calm\\") @Serializable data object LeviedStill : Levied"#,
            r#"put("ki'nd", "He\"ld")"#,
            r#""Bid\$Ask" -> input.json.decodeFromJsonElement(serializer<LeviedBid>(), JsonObject(obj - "ki'nd"))"#,
        ],
    );
    holds_each(
        &dutied_kotlin::kotlin_definition(),
        &[
            r#"put("ki\$nd", "Calm\\")"#,
            r#"val data = obj["bo'dy"]"#,
            r#"(obj - "ki\$nd" - "bo'dy").keys"#,
            r#"(obj["ki\$nd"] ?: throw SerializationException("missing field `ki\$nd`"))"#,
        ],
    );
    holds_each(
        &excised_kotlin::kotlin_definition(),
        &[
            r#"@SerialName("hi'gh") High, @SerialName("lo\$w") Low"#,
            r#""lo\$w" -> Excised.Low"#,
        ],
    );
    holds_each(
        &tithed_kotlin::kotlin_definition(),
        &[r#"setOf("cost\$usd")"#],
    );
    holds_each(
        &mulcted_kotlin::kotlin_definition(),
        &[r#"setOf("Bid\$Ask", "O'Neil", "C:\\Path", "Calm\"")"#],
    );
}
