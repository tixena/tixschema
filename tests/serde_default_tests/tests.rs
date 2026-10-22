//! A key under `#[serde(default)]` the payload leaves out: what serde reads, and what the text of
//! each surface says of it. The runs of each client are in the emitted-client groups, and the run
//! of the Zod schemas under node in `generic_types_tests`.

#[cfg(any(feature = "zod", feature = "jsonschema"))]
use super::models::{Basin, Config, Fleet, Logbook};
use super::models::{
    Bollard, CASES, FILLED, FILLED_REPORT, Harbour, Jetty, Marina, PREPROCESSED,
    PREPROCESSED_REPORT, Pontoon, REPORT, REQUIRED, REQUIRED_REPORT, Wharf, filled_expected,
    filled_report, preprocessed_values, read_values, unwritten_defaults,
};
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::models::{Gangway, report};
#[cfg(all(feature = "indexmap", any(feature = "zod", feature = "jsonschema")))]
use super::models::{IndexedFleet, IndexedLogbook};

/// The read of `Gangway`'s `u32`, held to its range.
#[cfg(feature = "dart")]
const WIDTH_READ: &str = "(switch (json['width'] as int) { final number when number >= 0 && \
                          number <= 4294967295 => number, final number => throw \
                          FormatException('expected an integer from 0 to 4294967295, found \
                          $number') })";

/// The write of `Gangway`'s `u32`, held to its range.
#[cfg(feature = "dart")]
const WIDTH_WRITTEN: &str = "(switch (width) { final number when number >= 0 && number <= \
                             4294967295 => number, final number => throw \
                             FormatException('expected an integer from 0 to 4294967295, found \
                             $number') })";

/// The table prints one row for each payload, a refusal followed by its messages.
#[test]
fn the_table_prints_a_row_for_each_payload() {
    let rows: Vec<&str> = REPORT
        .lines()
        .filter(|line| !line.starts_with("    "))
        .collect();
    assert_eq!(rows.len(), CASES.len());
    for (row, &(name, _, _)) in rows.iter().zip(&CASES) {
        assert!(row.starts_with(&format!("{name}: ")), "{row}");
    }
}

/// Every row of the table, as serde reads it where a schema surface is on: a build with none
/// holds the member of the last rows to no bound.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn serde_reads_each_payload_as_the_table_says() {
    assert_eq!(report(), REPORT);
}

/// The serde read of every build: a key left out is read as its default, a key held as it is.
#[test]
fn serde_reads_a_key_left_out_as_its_default_in_every_build() {
    let (every_default, both_held) = read_values();
    assert_eq!(
        serde_json::from_str::<Wharf>(r#"{"label":"w"}"#).unwrap(),
        every_default
    );
    assert_eq!(
        serde_json::from_str::<Pontoon>(r#"{"dock":"d","label":"p"}"#).unwrap(),
        both_held
    );
    assert_eq!(
        serde_json::from_str::<Jetty>("{}").unwrap(),
        Jetty::default()
    );
}

/// No function a `preprocess` names is called for a default: serde reads it as it is, and calls
/// the function for a key the payload holds.
#[test]
fn no_function_is_called_for_a_default() {
    for ((name, payload), read) in PREPROCESSED.iter().zip(preprocessed_values()) {
        let held = serde_json::from_str::<Bollard>(payload).unwrap();
        assert_eq!(held, read);
        let line = format!("{name}: read {}\n", serde_json::to_string(&held).unwrap());
        assert!(PREPROCESSED_REPORT.contains(&line), "{line}");
    }
}

/// serde reads the default of a member whatever the surfaces are handed: here no JSON, so the
/// surfaces keep the key required.
#[test]
fn serde_reads_the_default_of_a_member_whose_default_has_no_json() {
    let (marina, harbours) = unwritten_defaults();
    assert_eq!(
        serde_json::from_str::<Marina>(r#"{"label":"m"}"#).unwrap(),
        marina
    );
    for ((name, payload), harbour) in REQUIRED.iter().zip(harbours) {
        assert_eq!(
            serde_json::from_str::<Harbour<String>>(payload).unwrap(),
            harbour,
            "{name}"
        );
    }
    assert_eq!(REQUIRED_REPORT.lines().count(), REQUIRED.len());
    for (row, &(name, _)) in REQUIRED_REPORT.lines().zip(&REQUIRED) {
        assert_eq!(row, format!("{name}: refused"));
    }
}

/// The table of the structs that declare a parameter prints one row for each payload, each one
/// what serde reads: a list, a set or a map left out is read as its empty collection.
#[test]
fn serde_reads_each_payload_of_a_struct_that_declares_a_parameter_as_the_table_says() {
    assert_eq!(FILLED_REPORT.lines().count(), FILLED.len());
    for (row, &(name, _, _)) in FILLED_REPORT.lines().zip(&FILLED) {
        assert!(row.starts_with(&format!("{name}: read ")), "{row}");
    }
    assert_eq!(filled_report(), filled_expected());
}

/// `validate()` and the read that refuses judge a default as they judge a payload: the default
/// that breaks the bound is refused with its member named.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_default_that_breaks_its_bound_is_refused_with_its_member_named() {
    let refused = serde_json::from_str::<Pontoon>(r#"{"label":"p"}"#).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "'dock': too short: minimum length is 1, got 0"
    );
    let built = Pontoon {
        dock: String::new(),
        label: "p".to_owned(),
    };
    assert_eq!(
        built.validate().unwrap_err(),
        ["'dock': too short: minimum length is 1, got 0"]
    );
}

/// The Zod schema parses a default as it parses a payload: `.prefault` hands the parse the JSON
/// of the default where the key is left out, so the checks of the member judge it.
#[cfg(feature = "zod")]
#[test]
fn the_zod_schema_parses_the_default_of_each_member() {
    assert_eq!(
        Wharf::zod_schema(),
        "const Wharf$RawSchema = z.strictObject({
  dock: z.string().prefault(\"\"),
  gangway: Gangway$Schema.prefault({\"label\":\"main\",\"width\":2}),
  gate: z.string().prefault(\"north\"),
  label: z.string(),
});

export const Wharf$Schema: ZodType<Wharf> = Wharf$RawSchema.meta({
  examples: []
});"
    );
    assert_eq!(
        Jetty::zod_schema(),
        "const Jetty$RawSchema = z.strictObject({
  berths: z.number().int().min(0).max(4294967295).prefault(4),
  dock: z.string().prefault(\"east\"),
  moorings: z.array(z.string()).prefault([\"a\",\"b\"]),
});

export const Jetty$Schema: ZodType<Jetty> = Jetty$RawSchema.meta({
  examples: []
});"
    );
    assert!(
        Pontoon::zod_schema().contains(
            "  dock: z.string().min(1, { error: (issue) => `too short: minimum length is 1, got \
             ${String(issue.input).length}` }).prefault(\"\"),\n"
        ),
        "{}",
        Pontoon::zod_schema()
    );
}

/// The type a parse answers keeps every member: the key leaves the input alone.
#[cfg(feature = "typescript")]
#[test]
fn the_typescript_type_keeps_each_member_required() {
    let written = Wharf::ts_definition();
    for member in [
        "  dock: string;",
        "  gangway: Gangway;",
        "  gate: string;",
        "  label: string;",
    ] {
        assert!(written.contains(member), "{written}");
    }
    assert!(!written.contains("?:"), "{written}");
}

/// The JSON Schema leaves the key out of `required` and states the default, the JSON serde
/// writes for it.
#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_states_each_default_and_requires_no_key_it_answers() {
    let wharf = Wharf::json_schema();
    assert_eq!(wharf["required"], serde_json::json!(["label"]));
    assert_eq!(
        wharf["properties"]["dock"]["default"],
        serde_json::json!("")
    );
    assert_eq!(
        wharf["properties"]["gate"]["default"],
        serde_json::json!(super::models::north_dock())
    );
    assert_eq!(
        wharf["properties"]["gangway"]["default"],
        serde_json::to_value(Gangway::default()).unwrap()
    );
    assert!(
        wharf["properties"]["label"].get("default").is_none(),
        "{wharf}"
    );

    let jetty = Jetty::json_schema();
    assert_eq!(jetty["required"], serde_json::json!([]));
    let whole = serde_json::to_value(Jetty::default()).unwrap();
    for key in ["berths", "dock", "moorings"] {
        assert_eq!(jetty["properties"][key]["default"], whole[key], "{jetty}");
    }

    let pontoon = Pontoon::json_schema();
    assert_eq!(pontoon["required"], serde_json::json!(["label"]));
    assert_eq!(
        pontoon["properties"]["dock"],
        serde_json::json!({ "type": "string", "minLength": 1_u32, "default": "" })
    );
}

/// A default with no JSON keeps its key required on the two validating surfaces: `Marina` writes
/// no `Serialize`, and each member of `Harbour` names its parameter outside a list read as empty.
#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_requires_a_key_whose_default_has_no_json() {
    let marina = Marina::json_schema();
    assert_eq!(marina["required"], serde_json::json!(["dock", "label"]));
    assert!(
        marina["properties"]["dock"].get("default").is_none(),
        "{marina}"
    );
    let harbour = Harbour::<String>::json_schema();
    assert_eq!(
        harbour["required"],
        serde_json::json!(["berths", "flagship", "slips"])
    );
    for key in ["berths", "flagship", "slips"] {
        assert!(
            harbour["properties"][key].get("default").is_none(),
            "{harbour}"
        );
    }
}

#[cfg(feature = "zod")]
#[test]
fn the_zod_schema_requires_a_key_whose_default_has_no_json() {
    let marina = Marina::zod_schema();
    assert!(marina.contains("  dock: z.string(),\n"), "{marina}");
    let harbour = Harbour::<String>::zod_schema();
    for member in [
        "  berths: z.array(moored),\n",
        "  flagship: moored,\n",
        "  slips: z.array(moored),\n",
    ] {
        assert!(harbour.contains(member), "{harbour}");
    }
    assert!(!harbour.contains(".prefault("), "{harbour}");
}

/// A list of the struct's parameter is read as the empty list at every filling, and the member
/// beside it that names no parameter as before.
#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_states_the_empty_list_of_a_parameter() {
    let basin = Basin::<String>::json_schema();
    assert_eq!(basin["required"], serde_json::json!([]));
    assert_eq!(
        basin["properties"]["dock"]["default"],
        serde_json::json!("")
    );
    assert_eq!(
        basin["properties"]["moored"]["default"],
        serde_json::json!([])
    );
}

#[cfg(feature = "zod")]
#[test]
fn the_zod_schema_parses_the_empty_list_of_a_parameter() {
    let basin = Basin::<String>::zod_schema();
    assert!(
        basin.contains("  dock: z.string().prefault(\"\"),\n"),
        "{basin}"
    );
    assert!(
        basin.contains("  moored: z.array(moored).prefault([]),\n"),
        "{basin}"
    );
}

/// A list of a model filled with the parameter is read as `[]`, as the list beside it that names
/// no parameter is.
#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_states_the_empty_list_of_a_model_filled_with_a_parameter() {
    let config = Config::<String>::json_schema();
    assert_eq!(config["required"], serde_json::json!([]));
    for key in ["dataLinks", "sequences"] {
        assert_eq!(
            config["properties"][key]["default"],
            serde_json::json!([]),
            "{config}"
        );
    }
}

#[cfg(feature = "zod")]
#[test]
fn the_zod_schema_parses_the_empty_list_of_a_model_filled_with_a_parameter() {
    let config = Config::<String>::zod_schema();
    for member in [
        "  dataLinks: z.array(Link$SchemaFactory(idType)).prefault([]),\n",
        "  sequences: z.array(z.string()).prefault([]),\n",
    ] {
        assert!(config.contains(member), "{config}");
    }
}

/// Each list and set is read as `[]`, and each map as `{}`.
#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_states_the_empty_collection_of_each_list_set_and_map() {
    let held = [
        (
            Fleet::<String>::json_schema(),
            "binary_heap",
            serde_json::json!([]),
        ),
        (
            Fleet::<String>::json_schema(),
            "btree_set",
            serde_json::json!([]),
        ),
        (
            Fleet::<String>::json_schema(),
            "hash_set",
            serde_json::json!([]),
        ),
        (Fleet::<String>::json_schema(), "vec", serde_json::json!([])),
        (
            Fleet::<String>::json_schema(),
            "vec_deque",
            serde_json::json!([]),
        ),
        (
            Logbook::<String>::json_schema(),
            "btree_map",
            serde_json::json!({}),
        ),
        (
            Logbook::<String>::json_schema(),
            "hash_map",
            serde_json::json!({}),
        ),
        #[cfg(feature = "indexmap")]
        (
            IndexedFleet::<String>::json_schema(),
            "index_set",
            serde_json::json!([]),
        ),
        #[cfg(feature = "indexmap")]
        (
            IndexedLogbook::<String>::json_schema(),
            "index_map",
            serde_json::json!({}),
        ),
    ];
    for (schema, key, empty) in held {
        assert_eq!(schema["required"], serde_json::json!([]), "{schema}");
        assert_eq!(schema["properties"][key]["default"], empty, "{schema}");
    }
}

#[cfg(feature = "zod")]
#[test]
fn the_zod_schema_parses_the_empty_collection_of_each_list_set_and_map() {
    // A set's array refuses a repeat, with a check that ends on the line the default follows.
    let set_of_links = "z.array(Link$SchemaFactory(idType)).check((ctx) => {";
    let held: &[(String, &[&str])] = &[
        (
            Fleet::<String>::zod_schema(),
            &[
                "  binary_heap: z.array(Link$SchemaFactory(idType)).prefault([]),\n",
                "  vec: z.array(Link$SchemaFactory(idType)).prefault([]),\n",
                "  vec_deque: z.array(Link$SchemaFactory(idType)).prefault([]),\n",
            ],
        ),
        (
            Logbook::<String>::zod_schema(),
            &[
                "  btree_map: z.record(z.string(), Link$SchemaFactory(idType)).prefault({}),\n",
                "  hash_map: z.record(z.string(), Link$SchemaFactory(idType)).prefault({}),\n",
            ],
        ),
        #[cfg(feature = "indexmap")]
        (
            IndexedLogbook::<String>::zod_schema(),
            &["  index_map: z.record(z.string(), Link$SchemaFactory(idType)).prefault({}),\n"],
        ),
    ];
    for (schema, members) in held {
        for member in *members {
            assert!(schema.contains(member), "{schema}");
        }
    }
    let sets = [
        (
            Fleet::<String>::zod_schema(),
            &["btree_set", "hash_set"][..],
        ),
        #[cfg(feature = "indexmap")]
        (IndexedFleet::<String>::zod_schema(), &["index_set"][..]),
    ];
    for (schema, members) in sets {
        for member in members {
            assert!(
                schema.contains(&format!("  {member}: {set_of_links}")),
                "{schema}"
            );
        }
        assert_eq!(
            schema.matches("\n}).prefault([]),\n").count(),
            members.len(),
            "{schema}"
        );
    }
}

/// A type with no `#[serde(default)]` writes on every surface the text it wrote before a default
/// was read: `Gangway`, pinned whole.
#[cfg(feature = "zod")]
#[test]
fn a_type_with_no_default_writes_the_zod_text_it_wrote() {
    assert_eq!(
        Gangway::zod_schema(),
        "const Gangway$RawSchema = z.strictObject({
  label: z.string(),
  width: z.number().int().min(0).max(4294967295),
});

export const Gangway$Schema: ZodType<Gangway> = Gangway$RawSchema.meta({
  examples: []
});"
    );
}

#[cfg(all(feature = "typescript", feature = "jsonschema"))]
#[test]
fn a_type_with_no_default_writes_the_typescript_text_it_wrote() {
    let lines = [
        "/**",
        " * A gangway, a type of the consumer's own with a `Default` of its own.",
        " * ",
        " */",
        "export type Gangway = {",
        "  /**",
        "   * label",
        "   * ",
        "   */",
        "  label: string;",
        "  /**",
        "   * width",
        "   * ",
        "   */",
        "  width: number;",
        "};",
    ];
    assert_eq!(Gangway::ts_definition(), lines.join("\n"));
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_type_with_no_default_writes_the_json_schema_it_wrote() {
    assert_eq!(
        serde_json::to_string(&Gangway::json_schema()).unwrap(),
        "{\"type\":\"object\",\"additionalProperties\":false,\"properties\":{\"label\":{\"type\":\
         \"string\"},\"width\":{\"type\":\"integer\"}},\"required\":[\"label\",\"width\"]}"
    );
}

#[cfg(feature = "dart")]
#[test]
fn a_type_with_no_default_writes_the_dart_text_it_wrote() {
    assert_eq!(
        super::models::gangway_dart::dart_definition(),
        format!(
            "class Gangway {{ const Gangway({{required this.label,required this.width,}}); final \
             String label;final int width; factory Gangway.fromJson(Map<String, dynamic> json) {{ \
             final read = Gangway.fromJsonUnjudged(json); final violations = read.validate(); if \
             (violations.isNotEmpty) throw FormatException(violations.join('; ')); return read; \
             }} factory Gangway.fromJsonUnjudged(Map<String, dynamic> json) => Gangway(label: \
             json['label'] as String,width: {WIDTH_READ},); Map<String, dynamic> toJson() => {{ \
             'label': label,'width': {WIDTH_WRITTEN}, }}; List<String> validate() => const []; }}"
        )
    );
}

#[cfg(feature = "swift")]
#[test]
fn a_type_with_no_default_writes_the_swift_text_it_wrote() {
    assert_eq!(
        super::models::gangway_swift::swift_definition(),
        "public struct Gangway: Codable, Sendable { public let label: String; public let width: \
         UInt32; enum CodingKeys: String, CodingKey { case label; case width;  }; public \
         init(label: String, width: UInt32) { self.label = label; self.width = width;  }; public \
         init(swiftSchemaUnjudged decoder: Decoder) throws { let container = try \
         decoder.container(keyedBy: CodingKeys.self); self.label = try \
         container.decode(String.self, forKey: .label); self.width = try \
         container.decode(UInt32.self, forKey: .width) }; public init(from decoder: Decoder) \
         throws { try self.init(swiftSchemaUnjudged: decoder); let violations = validate(); if \
         !violations.isEmpty { throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: \
         decoder.codingPath, debugDescription: violations.joined(separator: \"; \"))) } }; public \
         func validate() -> [String] { [] } }; "
    );
}

#[cfg(feature = "kotlin")]
#[test]
fn a_type_with_no_default_writes_the_kotlin_text_it_wrote() {
    assert_eq!(
        super::models::gangway_kotlin::kotlin_definition(),
        "@OptIn(ExperimentalSerializationApi::class) @Serializable(with = \
         GangwaySerializer::class) @KeepGeneratedSerializer data class Gangway(val label: String, \
         val width: UInt) { fun validate(): List<String> = emptyList() } \
         @OptIn(ExperimentalSerializationApi::class) object GangwayUnjudged : KSerializer<Gangway> \
         by Gangway.generatedSerializer() object GangwaySerializer : KSerializer<Gangway>, () -> \
         KSerializer<Gangway> { override fun invoke(): KSerializer<Gangway> = GangwayUnjudged; \
         override val descriptor: SerialDescriptor = GangwayUnjudged.descriptor; override fun \
         serialize(encoder: Encoder, value: Gangway) = GangwayUnjudged.serialize(encoder, value); \
         override fun deserialize(decoder: Decoder): Gangway { val read = \
         GangwayUnjudged.deserialize(decoder); val violations = read.validate(); if \
         (violations.isNotEmpty()) throw SerializationException(violations.joinToString(\"; \")); \
         return read } }"
    );
}

/// The read of each client reads a key left out from the JSON of its default, through the read
/// the member has: Dart casts what `jsonDecode` answers, Swift reads a payload of it, and Kotlin
/// decodes it through the member's serializer as the default of the property.
#[cfg(feature = "dart")]
#[test]
fn dart_reads_a_key_left_out_from_the_json_of_its_default() {
    let written = super::models::wharf_dart::dart_definition();
    assert!(
        written.contains(
            "dock: json.containsKey('dock') ? (json['dock'] as String) : (jsonDecode('\"\"') as \
             String),gangway: json.containsKey('gangway') ? \
             (Gangway.fromJsonUnjudged(json['gangway'])) : \
             (Gangway.fromJsonUnjudged(jsonDecode('{\"label\":\"main\",\"width\":2}'))),gate: \
             json.containsKey('gate') ? (json['gate'] as String) : (jsonDecode('\"north\"') as \
             String),label: json['label'] as String,"
        ),
        "{written}"
    );
}

#[cfg(feature = "swift")]
#[test]
fn swift_reads_a_key_left_out_from_the_json_of_its_default() {
    let written = super::models::wharf_swift::swift_definition();
    assert!(
        written.contains(
            "if container.contains(.gate) { self.gate = try container.decode(String.self, \
             forKey: .gate) } else { let gateDefaultPayload = try \
             JSONSerialization.jsonObject(with: Data(\"\\\"north\\\"\".utf8), options: \
             [.fragmentsAllowed]); self.gate = try SwiftSchemaPayload.read(String.self, from: \
             gateDefaultPayload, at: container.codingPath + [CodingKeys.gate]) }; self.label = \
             try container.decode(String.self, forKey: .label) }"
        ),
        "{written}"
    );
}

#[cfg(feature = "kotlin")]
#[test]
fn kotlin_reads_a_key_left_out_from_the_json_of_its_default() {
    let written = super::models::wharf_kotlin::kotlin_definition();
    assert!(
        written.contains(
            "data class Wharf(val dock: String = Json.decodeFromString(serializer<String>(), \
             \"\\\"\\\"\"), @Serializable(with = GangwayUnjudged::class) val gangway: Gangway = \
             Json.decodeFromString(GangwayUnjudged, \
             \"{\\\"label\\\":\\\"main\\\",\\\"width\\\":2}\"), val gate: String = \
             Json.decodeFromString(serializer<String>(), \"\\\"north\\\"\"), val label: String)"
        ),
        "{written}"
    );
}
