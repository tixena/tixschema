//! A key under `#[serde(default)]` the payload leaves out: what serde reads, and what the text of
//! each surface says of it. The runs of each client are in the emitted-client groups, and the run
//! of the Zod schemas under node in `generic_types_tests`.

#[cfg(feature = "serde")]
use super::models::{Basin, Config, Fleet, Gangway, GangwayDraft, Logbook, PontoonDraft, report};
use super::models::{
    Bollard, CASES, FILLED, FILLED_REPORT, Harbour, Jetty, JettyDraft, Marina, PREPROCESSED,
    PREPROCESSED_REPORT, Pontoon, REPORT, REQUIRED, REQUIRED_REPORT, Wharf, filled_expected,
    filled_report, preprocessed_values, read_values, unwritten_defaults,
};
#[cfg(feature = "indexmap")]
use super::models::{IndexedFleet, IndexedLogbook};
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
use super::omitted::OMITTED_WRITTEN;
use super::omitted::{
    Muster, MusterDraft, OMITTED, OMITTED_REPORT, Roster, RosterDraft, Squad, SquadDraft,
};
#[cfg(feature = "kotlin")]
use crate::kotlin_numbers::without_number_readers;

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

/// Every row of the table, as serde reads it.
#[cfg(feature = "serde")]
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
        JettyDraft::default().into()
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
#[cfg(feature = "serde")]
#[test]
fn a_default_that_breaks_its_bound_is_refused_with_its_member_named() {
    let refused = serde_json::from_str::<Pontoon>(r#"{"label":"p"}"#).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "'dock': too short: minimum length is 1, got 0"
    );
    let built = PontoonDraft {
        dock: String::new(),
        label: "p".to_owned(),
    };
    assert_eq!(
        built.validation_issues(),
        ["'dock': too short: minimum length is 1, got 0"]
    );
}

/// The Zod schema parses a default as it parses a payload: `.prefault` hands the parse the JSON
/// of the default where the key is left out, so the checks of the member judge it.
#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn the_zod_schema_parses_the_default_of_each_member() {
    assert_eq!(
        Wharf::zod_schema(),
        "const WharfDraft$RawSchema = z.strictObject({
  dock: z.string().prefault(\"\"),
  gangway: Gangway$Schema.prefault({\"label\":\"main\",\"width\":2}),
  gate: z.string().prefault(\"north\"),
  label: z.string(),
});

export const WharfDraft$Schema: ZodType<WharfDraft> = WharfDraft$RawSchema;

export const Wharf$Schema = WharfDraft$Schema.readonly().brand<\"Wharf\">().meta(WharfDraft$Schema.meta() ?? {}).meta({
  examples: []
});"
    );
    assert_eq!(
        Jetty::zod_schema(),
        "const JettyDraft$RawSchema = z.strictObject({
  berths: z.number().int().min(0).max(4294967295).prefault(4),
  dock: z.string().prefault(\"east\"),
  moorings: z.array(z.string()).prefault([\"a\",\"b\"]),
});

export const JettyDraft$Schema: ZodType<JettyDraft> = JettyDraft$RawSchema;

export const Jetty$Schema = JettyDraft$Schema.readonly().brand<\"Jetty\">().meta(JettyDraft$Schema.meta() ?? {}).meta({
  examples: []
});"
    );
    assert!(
        Pontoon::zod_schema().contains(
            "  dock: z.string().min(1, { error: (issue) => `too short: minimum length is 1, got \
             ${[...String(issue.input)].length}` }).prefault(\"\"),\n"
        ),
        "{}",
        Pontoon::zod_schema()
    );
}

/// The type a parse answers keeps every member: the key leaves the input alone.
#[cfg(all(feature = "serde", feature = "typescript"))]
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
#[cfg(feature = "serde")]
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
        serde_json::to_value(GangwayDraft::default()).unwrap()
    );
    assert!(
        wharf["properties"]["label"].get("default").is_none(),
        "{wharf}"
    );

    let jetty = Jetty::json_schema();
    assert_eq!(jetty["required"], serde_json::json!([]));
    let whole = serde_json::to_value(JettyDraft::default()).unwrap();
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
#[cfg(feature = "serde")]
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

#[cfg(all(feature = "serde", feature = "zod"))]
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
#[cfg(feature = "serde")]
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

#[cfg(all(feature = "serde", feature = "zod"))]
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
#[cfg(feature = "serde")]
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

#[cfg(all(feature = "serde", feature = "zod"))]
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
#[cfg(feature = "serde")]
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

#[cfg(all(feature = "serde", feature = "zod"))]
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
#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_type_with_no_default_writes_the_zod_text_it_wrote() {
    assert_eq!(
        Gangway::zod_schema(),
        "const GangwayDraft$RawSchema = z.strictObject({
  label: z.string(),
  width: z.number().int().min(0).max(4294967295),
});

export const GangwayDraft$Schema: ZodType<GangwayDraft> = GangwayDraft$RawSchema;

export const Gangway$Schema = GangwayDraft$Schema.readonly().brand<\"Gangway\">().meta(GangwayDraft$Schema.meta() ?? {}).meta({
  examples: []
});"
    );
}

#[cfg(all(feature = "serde", feature = "typescript"))]
#[test]
fn a_type_with_no_default_writes_the_typescript_text_it_wrote() {
    let lines = [
        "/**",
        " * A gangway, a type of the consumer's own with a `Default` of its own.",
        " * ",
        " */",
        "export type GangwayDraft = {",
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
        "",
        "export type Gangway = Readonly<GangwayDraft> & $brand<\"Gangway\">;",
        "",
        "export function GangwayDraft$valid(draft: GangwayDraft): Gangway {",
        "  return draft as unknown as Gangway;",
        "}",
    ];
    assert_eq!(Gangway::ts_definition(), lines.join("\n"));
}

#[cfg(feature = "serde")]
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
            "class GangwayDraft {{ const GangwayDraft({{required this.label,required this.width,}}); \
             final String label;final int width; factory GangwayDraft.fromJson(Map<String, \
             dynamic> json) {{ for (final key in json.keys) {{ if (!const <String>{{'label', \
             'width'}}.contains(key)) throw FormatException('Unknown key of a Gangway: $key', \
             <Object>[key]); }} return GangwayDraft(label: _readAt(json, 'label', () => \
             json['label'] as String),width: _readAt(json, 'width', () => {WIDTH_READ}),); }} \
             static Read _readAt<Read>(Map<String, dynamic>? object, Object key, Read \
             Function() read) {{ try {{ return read(); }} catch (refused) {{ if (object != null \
             && !object.containsKey(key)) throw FormatException('missing field `$key`', \
             <Object>[key]); if (refused is TypeError) throw FormatException('$refused', \
             <Object>[key]); if (refused is FormatException) {{ final at = refused.source; throw \
             FormatException(refused.message, <Object>[key, if (at is List<Object>) ...at]); }} \
             rethrow; }} }} Map<String, dynamic> toJson() => {{ 'label': label,'width': \
             {WIDTH_WRITTEN}, }}; List<String> validationIssues() => const []; Gangway \
             validate() {{ final found = validationIssues(); if (found.isNotEmpty) throw \
             FormatException(found.join('; ')); return Gangway._(this); }} Gangway toValid() => Gangway._(this); }} class Gangway {{ \
             const Gangway._(this._draft); final GangwayDraft _draft; String get label => \
             _draft.label; int get width => _draft.width; GangwayDraft toDraft() => _draft; \
             factory Gangway.fromJson(Map<String, dynamic> json) {{ final read = \
             GangwayDraft.fromJson(json); final violations = read.validationIssues(); if \
             (violations.isNotEmpty) throw FormatException(violations.join('; ')); return \
             Gangway._(read); }} factory Gangway.fromJsonUnjudged(Map<String, dynamic> json) => \
             Gangway._(GangwayDraft.fromJson(json)); factory Gangway.of({{required String label,required int width,}}) => GangwayDraft(label: label,width: width,).validate(); Map<String, dynamic> toJson() => \
             _draft.toJson(); List<String> validationIssues() => _draft.validationIssues(); static \
             Object $thrown(Object thrown) => thrown is GangwayDraft ? thrown.validate() : thrown; }}"
        )
    );
}

#[cfg(feature = "swift")]
#[test]
fn a_type_with_no_default_writes_the_swift_text_it_wrote() {
    use crate::swift_fitted::FITTED;
    use crate::swift_valid;

    assert_eq!(
        swift_valid::draft_of(&super::models::gangway_swift::swift_definition()),
        format!(
            "{}{FITTED}; {}{} }};",
            "public struct GangwayDraft: Codable, Sendable { public var label: String; public var \
             width: UInt32; enum CodingKeys: String, CodingKey { case label; case width;  }; \
             public init(label: String, width: UInt32) { self.label = label; self.width = width;  \
             }; private struct SwiftSchemaAnyKey: CodingKey { var stringValue: String; var \
             intValue: Int? { nil }; init?(stringValue: String) { self.stringValue = stringValue \
             }; init?(intValue: Int) { nil } }; ",
            "fileprivate init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: [String]) \
             throws { let container = try decoder.container(keyedBy: CodingKeys.self); self.label \
             = try container.decode(String.self, forKey: .label); self.width = try \
             container.decode(SwiftSchemaFit<UInt32>.self, forKey: .width).value }; fileprivate init(swiftSchemaUnjudged decoder: Decoder) throws { if !(decoder.codingPath.isEmpty && decoder.userInfo[CodingUserInfoKey(rawValue: \"swiftSchemaFlat\")!] != nil) { for unknown in try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys where ![\"label\", \"width\"].contains(unknown.stringValue) { throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath + [unknown], debugDescription: \"unknown field `\\(unknown.stringValue)`\")) } }; try self.init(swiftSchemaUnjudged: decoder, swiftSchemaWithout: []) }; public init(from decoder: Decoder) throws { try self.init(swiftSchemaUnjudged: decoder); let violations = validationIssues(); if !violations.isEmpty { throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: violations.joined(separator: \"; \"))) } }; public func validationIssues() -> [String] { [] }; public func swiftSchemaValidateBound() -> [String] { [] }; ",
            swift_valid::validating_free("Gangway")
        )
    );
}

#[cfg(feature = "kotlin")]
#[test]
fn a_type_with_no_default_writes_the_kotlin_text_it_wrote() {
    assert_eq!(
        without_number_readers(&super::models::gangway_kotlin::kotlin_definition()),
        "@OptIn(ExperimentalSerializationApi::class) @Serializable(with = \
         GangwayDraftUnjudged::class) @KeepGeneratedSerializer data class GangwayDraft(val label: \
         String, val width: UInt) { fun validationIssues(): List<String> = emptyList() fun \
         validate(): Gangway { val violations = validationIssues(); if (violations.isNotEmpty()) \
         throw SerializationException(violations.joinToString(\"; \"), \
         IllegalArgumentException(\"validate()\")); return Gangway.checked(this) } fun toValid(): Gangway = Gangway.checked(this) } \
         @OptIn(ExperimentalSerializationApi::class) object GangwayDraftUnjudged : \
         KSerializer<GangwayDraft> by GangwayDraft.generatedSerializer() @ConsistentCopyVisibility \
         @Serializable(with = GangwaySerializer::class) data class Gangway private constructor(val \
         draft: GangwayDraft) { val label: String get() = draft.label; val width: UInt get() = \
         draft.width; fun validationIssues(): List<String> = draft.validationIssues(); fun \
         kotlinSchemaChecked(): GangwayDraft = draft; companion object { internal fun \
         checked(draft: GangwayDraft): Gangway = Gangway(draft); fun of(label: String, width: UInt): Gangway = GangwayDraft(label = label, width = width).validate() } } \
         @OptIn(ExperimentalSerializationApi::class) private object GangwayUnjudged : \
         KSerializer<Gangway> { override val descriptor: SerialDescriptor = \
         GangwayDraftUnjudged.descriptor; override fun serialize(encoder: Encoder, value: Gangway) \
         = GangwayDraftUnjudged.serialize(encoder, value.draft); override fun deserialize(decoder: \
         Decoder): Gangway = Gangway.checked(try { GangwayDraftUnjudged.deserialize(decoder) } \
         catch (refused: SerializationException) { val named = \
         GangwayDraftUnjudged.descriptor.serialName; val said = refused.message.orEmpty(); if \
         (\"'\" + named + \"'\" !in said) throw refused; val declared = \
         named.removeSuffix(\"Draft\"); throw if (refused is MissingFieldException) \
         MissingFieldException(refused.missingFields, declared) else \
         SerializationException(said.replace(\"'\" + named + \"'\", \"'\" + declared + \"'\"), \
         refused.cause) }) } object GangwaySerializer : KSerializer<Gangway>, () -> \
         KSerializer<Gangway> { override fun invoke(): KSerializer<Gangway> = GangwayUnjudged; \
         override val descriptor: SerialDescriptor = GangwayUnjudged.descriptor; override fun \
         serialize(encoder: Encoder, value: Gangway) = GangwayUnjudged.serialize(encoder, value); \
         override fun deserialize(decoder: Decoder): Gangway { val read = \
         GangwayUnjudged.deserialize(decoder); val violations = read.validationIssues(); if \
         (violations.isNotEmpty()) throw SerializationException(violations.joinToString(\"; \"), \
         IllegalArgumentException(\"validate()\")); return read } }"
    );
}

/// The read of each client reads a key left out from the JSON of its default, through the read
/// the member has: Dart casts what `jsonDecode` answers, Swift reads a payload of it, and Kotlin
/// decodes it through the member's serializer as the default of the property.
#[cfg(all(feature = "dart", feature = "serde"))]
#[test]
fn dart_reads_a_key_left_out_from_the_json_of_its_default() {
    let written = super::models::wharf_dart::dart_definition();
    assert!(
        written.contains(
            "dock: json.containsKey('dock') ? _readAt(json, 'dock', () => json['dock'] as String) \
             : (jsonDecode('\"\"') as String),gangway: json.containsKey('gangway') ? \
             _readAt(json, 'gangway', () => Gangway.fromJsonUnjudged(json['gangway'])) : \
             (Gangway.fromJsonUnjudged(jsonDecode('{\"label\":\"main\",\"width\":2}'))),gate: \
             json.containsKey('gate') ? _readAt(json, 'gate', () => json['gate'] as String) : \
             (jsonDecode('\"north\"') as String),label: _readAt(json, 'label', () => json['label'] \
             as String),"
        ),
        "{written}"
    );
}

#[cfg(all(feature = "swift", feature = "serde"))]
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

#[cfg(all(feature = "kotlin", feature = "serde"))]
#[test]
fn kotlin_reads_a_key_left_out_from_the_json_of_its_default() {
    let written = super::models::wharf_kotlin::kotlin_definition();
    assert!(
        written.contains(
            "data class WharfDraft(val dock: String = Json.decodeFromString(serializer<String>(), \
             \"\\\"\\\"\"), @Serializable(with = GangwayUnjudged::class) val gangway: Gangway = \
             Json.decodeFromString(GangwayUnjudged, \
             \"{\\\"label\\\":\\\"main\\\",\\\"width\\\":2}\"), val gate: String = \
             Json.decodeFromString(serializer<String>(), \"\\\"north\\\"\"), val label: String)"
        ),
        "{written}"
    );
}

/// serde reads a list under `default` with `skip_serializing_if` or `skip_serializing` as `[]`
/// where the key is left out, and what each client prints is read back to the same value.
#[test]
fn serde_reads_an_omitted_key_with_a_default_as_its_default() {
    let roles = |held: &[&str]| held.iter().map(|&role| role.to_owned()).collect::<Vec<_>>();
    assert_eq!(
        serde_json::from_str::<Roster>(r#"{"id":"1"}"#).unwrap(),
        RosterDraft {
            id: "1".to_owned(),
            roles: roles(&[])
        }
        .into()
    );
    assert_eq!(
        serde_json::from_str::<Muster>(r#"{"id":"1"}"#).unwrap(),
        MusterDraft {
            id: "1".to_owned(),
            roles: roles(&[])
        }
        .into()
    );
    assert_eq!(
        serde_json::from_str::<Squad>(r#"{"id":"1"}"#).unwrap(),
        SquadDraft {
            id: "1".to_owned(),
            roles: roles(&["guest"])
        }
        .into()
    );
    let reports = [
        OMITTED_REPORT,
        #[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
        OMITTED_WRITTEN,
    ];
    for report in reports {
        assert_eq!(OMITTED.len(), report.lines().count());
        for (&(name, read_as, payload), printed) in OMITTED.iter().zip(report.lines()) {
            let written = printed.strip_prefix(&format!("{name}: read ")).unwrap();
            if read_as == "Roster" {
                assert_eq!(
                    serde_json::from_str::<Roster>(payload).unwrap(),
                    serde_json::from_str::<Roster>(written).unwrap()
                );
            } else if read_as == "Squad" {
                assert_eq!(
                    serde_json::from_str::<Squad>(payload).unwrap(),
                    serde_json::from_str::<Squad>(written).unwrap()
                );
            } else {
                let rewritten = report == OMITTED_REPORT || !payload.contains("roles");
                if rewritten {
                    assert_eq!(
                        serde_json::from_str::<Muster>(payload).unwrap(),
                        serde_json::from_str::<Muster>(written).unwrap()
                    );
                }
            }
        }
    }
}

/// The Zod schema reads the omitted key as the default, and the parsed type holds the member.
#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn the_zod_schema_prefaults_an_omitted_key_with_a_default() {
    let roster = Roster::zod_schema();
    assert!(
        roster.contains("  roles: z.array(z.string()).prefault([]),\n"),
        "{roster}"
    );
    assert!(!roster.contains(".optional()"), "{roster}");
    let muster = Muster::zod_schema();
    assert!(
        muster.contains("  roles: z.array(z.string()).prefault([]),\n"),
        "{muster}"
    );
    let squad = Squad::zod_schema();
    assert!(
        squad.contains("  roles: z.array(z.string()).prefault([\"guest\"]),\n"),
        "{squad}"
    );
}

/// The TypeScript type holds the member.
#[cfg(all(feature = "serde", feature = "typescript"))]
#[test]
fn the_typescript_type_holds_an_omitted_key_with_a_default() {
    let written = Roster::ts_definition();
    assert!(written.contains("  roles: Array<string>;"), "{written}");
    assert!(!written.contains("roles?"), "{written}");
}

/// The JSON Schema leaves the key out of `required` and states the default.
#[cfg(feature = "serde")]
#[test]
fn the_json_schema_states_the_default_of_an_omitted_key() {
    for schema in [Roster::json_schema(), Muster::json_schema()] {
        assert_eq!(schema["required"], serde_json::json!(["id"]), "{schema}");
        assert_eq!(
            schema["properties"]["roles"]["default"],
            serde_json::json!([]),
            "{schema}"
        );
    }
    let squad = Squad::json_schema();
    assert_eq!(
        squad["properties"]["roles"]["default"],
        serde_json::json!(["guest"]),
        "{squad}"
    );
    assert!(
        !squad["required"]
            .as_array()
            .is_some_and(|required| required.iter().any(|key| key == "roles")),
        "{squad}"
    );
}
