//! A struct that carries its own `#[serde(tag = "...")]` writes its serde name under that key, and
//! every surface describes the key.

#[cfg(all(feature = "serde", feature = "typescript"))]
mod on_a_schema_surface {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind")]
    pub struct Tagged {
        pub name: String,
    }

    /// Flattens a tagged struct, whose key serde then writes into this one's object.
    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct TaggedOuter {
        pub id: u32,
        #[serde(flatten)]
        pub tagged: Tagged,
    }

    /// The container's rename moves the tag's value; its `rename_all` reaches the fields alone.
    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind", rename = "tagged-one", rename_all = "camelCase")]
    pub struct TaggedRenamed {
        pub first_name: String,
    }

    /// The lines of `definition` that declare a member, without the doc comments around them.
    fn members(definition: &str) -> Vec<&str> {
        definition
            .lines()
            .map(str::trim)
            .filter(|line| line.contains(": ") && line.ends_with(';'))
            .collect()
    }

    #[test]
    fn the_tag_is_a_member_whose_only_value_is_the_structs_name() {
        let written = serde_json::to_value(Tagged {
            name: "x".to_owned(),
        })
        .unwrap();
        assert_eq!(
            written,
            serde_json::json!({ "kind": "Tagged", "name": "x" })
        );
        let definition = Tagged::ts_definition();
        assert_eq!(
            members(&definition),
            ["kind: \"Tagged\";", "name: string;"],
            "got: {definition}"
        );
    }

    #[test]
    fn a_rename_of_the_struct_moves_the_value_and_rename_all_leaves_it() {
        let written = serde_json::to_value(TaggedRenamed {
            first_name: "x".to_owned(),
        })
        .unwrap();
        assert_eq!(
            written,
            serde_json::json!({ "kind": "tagged-one", "firstName": "x" })
        );
        let definition = TaggedRenamed::ts_definition();
        assert_eq!(
            members(&definition),
            ["kind: \"tagged-one\";", "firstName: string;"],
            "got: {definition}"
        );
    }

    #[test]
    fn serde_reads_the_struct_with_the_key_and_without_it() {
        let expected = Tagged {
            name: "x".to_owned(),
        };
        for stored in [
            serde_json::json!({ "kind": "Tagged", "name": "x" }),
            serde_json::json!({ "name": "x" }),
        ] {
            assert_eq!(serde_json::from_value::<Tagged>(stored).unwrap(), expected);
        }
    }

    #[test]
    fn a_struct_that_flattens_a_tagged_one_writes_the_key_it_describes() {
        let written = serde_json::to_value(TaggedOuter {
            id: 1,
            tagged: Tagged {
                name: "x".to_owned(),
            },
        })
        .unwrap();
        assert_eq!(
            written,
            serde_json::json!({ "id": 1_u32, "kind": "Tagged", "name": "x" })
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn the_zod_schema_holds_the_tag_as_a_literal() {
        let zod = Tagged::zod_schema();
        assert!(
            zod.contains(
                "z.strictObject({\n  kind: z.literal(\"Tagged\"),\n  name: z.string(),\n})"
            ),
            "got: {zod}"
        );
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn the_json_schema_requires_the_tag_at_its_one_value() {
        assert_eq!(
            Tagged::json_schema(),
            serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "kind": { "type": "string", "const": "Tagged" },
                    "name": { "type": "string" },
                },
                "required": ["kind", "name"],
            })
        );
    }
}

/// Each of the three languages writes the key with the struct's name and reads a payload with it
/// or without it, as serde does.
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
mod on_a_mobile_surface {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind")]
    pub struct TaggedMobile {
        pub name: String,
    }

    #[test]
    fn serde_writes_the_key_the_three_languages_write() {
        let written = serde_json::to_value(TaggedMobile {
            name: "x".to_owned(),
        })
        .unwrap();
        assert_eq!(
            written,
            serde_json::json!({ "kind": "TaggedMobile", "name": "x" })
        );
    }

    #[cfg(feature = "dart")]
    #[test]
    fn dart_writes_the_tag_ahead_of_the_fields() {
        let definition = tagged_mobile_dart::dart_definition();
        assert!(
            definition.contains(
                "Map<String, dynamic> toJson() => { 'kind': 'TaggedMobile','name': name, };"
            ),
            "got: {definition}"
        );
        assert!(
            definition.contains("=> TaggedMobile(name: json['name'] as String,);"),
            "got: {definition}"
        );
    }

    #[cfg(feature = "swift")]
    #[test]
    fn swift_encodes_the_tag_under_a_key_set_of_its_own() {
        let definition = tagged_mobile_swift::swift_definition();
        for written in [
            "private enum SwiftSchemaTagCodingKeys: String, CodingKey { case swiftSchemaTag = \"kind\" };",
            "var tagged = encoder.container(keyedBy: SwiftSchemaTagCodingKeys.self); try tagged.encode(\"TaggedMobile\", forKey: .swiftSchemaTag);",
            "public init(from decoder: Decoder) throws { let container = try decoder.container(keyedBy: CodingKeys.self); self.name = try container.decode(String.self, forKey: .name) };",
        ] {
            assert!(
                definition.contains(written),
                "want: {written}\ngot: {definition}"
            );
        }
    }

    #[cfg(feature = "kotlin")]
    #[test]
    fn kotlin_holds_the_tag_in_a_property_it_always_encodes() {
        assert_eq!(
            tagged_mobile_kotlin::kotlin_definition(),
            "@OptIn(ExperimentalSerializationApi::class) @Serializable data class \
             TaggedMobile(val name: String) { @SerialName(\"kind\") @EncodeDefault val \
             kotlinSchemaTag: String = \"TaggedMobile\" }"
        );
    }
}
