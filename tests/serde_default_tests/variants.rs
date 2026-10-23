//! A member of an enum's variant under `#[serde(default)]` the payload leaves out: what serde
//! reads, what the text of each surface says of it, and the members that build as before. The
//! runs of each client are in the emitted-client groups, and the run of the Zod schemas under node
//! in `generic_types_tests`.

#[cfg(feature = "kotlin")]
use crate::kotlin_numbers::without_number_readers;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[cfg(feature = "zod")]
use super::models::Quay;
use super::models::{
    AdjacentBerthing, Berthing, ExternalBerthing, UntaggedBerthing, VARIANT_CASES, VARIANT_REPORT,
    north_dock,
};
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::models::{untagged_berthing_schema, variant_report};

/// The untagged enum under another name.
#[cfg(feature = "zod")]
#[model_schema()]
pub type AliasedBerthing = UntaggedBerthing;

/// An untagged enum one variant of which holds the untagged enum whose members are read as their
/// defaults.
#[cfg(feature = "zod")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Anchorage {
    Held(UntaggedBerthing),
    Named { harbour: String },
}

/// A struct that flattens the untagged enum that holds the other.
#[cfg(feature = "zod")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Mooring {
    #[serde(flatten)]
    pub anchorage: Anchorage,
    pub label: String,
}

/// A member under `default` that is an `Option`: every target reads its absent key as `None`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum OptionalBerth {
    Docked {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        dock: Option<String>,
        pier: u32,
    },
}

/// A struct that flattens the alias of the untagged enum.
#[cfg(feature = "zod")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Pier {
    #[serde(flatten)]
    pub berth: AliasedBerthing,
    pub label: String,
}

/// An enum serde reads through another type, and so reads no member of: built as before.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", from = "Berthing")]
pub enum ReadThrough {
    Docked {
        #[serde(default)]
        dock: String,
        pier: u32,
    },
}

/// Members whose key serde never reads: built as before.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum SkippedBerth {
    Docked {
        #[serde(default = "north_dock", skip)]
        dock: String,
        #[serde(default, skip_deserializing)]
        line: String,
        pier: u32,
    },
}

/// A struct that flattens the untagged enum among its own keys.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Slipway {
    #[serde(flatten)]
    pub berth: UntaggedBerthing,
    pub label: String,
}

/// An enum none of whose members is read as a default.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum Tide {
    Ebbing { height: u32 },
    Slack,
}

/// An enum under no tag attribute with no unit variant, which a struct flattens.
#[cfg(feature = "zod")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum Towage {
    Docked {
        #[serde(default)]
        dock: String,
        pier: u32,
    },
}

/// An enum serde reads through another type it may refuse, and so reads no member of: built as
/// before.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", try_from = "Berthing")]
pub enum TriedThrough {
    Docked {
        #[serde(default)]
        dock: String,
        pier: u32,
    },
}

/// A struct that flattens the enum under no tag attribute.
#[cfg(feature = "zod")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Wake {
    pub label: String,
    #[serde(flatten)]
    pub towage: Towage,
}

/// An enum serde never reads: built as before.
#[model_schema()]
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum WrittenOnly {
    Docked {
        #[serde(default)]
        dock: String,
        pier: u32,
    },
}

impl From<Berthing> for ReadThrough {
    fn from(read: Berthing) -> Self {
        if let Berthing::Docked { dock, pier } = read {
            Self::Docked { dock, pier }
        } else {
            Self::Docked {
                dock: String::new(),
                pier: 0,
            }
        }
    }
}

impl TryFrom<Berthing> for TriedThrough {
    type Error = String;

    fn try_from(read: Berthing) -> Result<Self, Self::Error> {
        if let Berthing::Docked { dock, pier } = read {
            Ok(Self::Docked { dock, pier })
        } else {
            Err("not docked".to_owned())
        }
    }
}

/// What serde writes back of the value it reads from `payload` as a `T`.
fn written_back<T>(payload: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::to_string(&serde_json::from_str::<T>(payload).unwrap()).unwrap()
}

/// serde's derive reads a missing key of a member of a variant under `default` as the default,
/// under each of the four taggings, and writes the member back.
#[test]
fn serde_reads_a_missing_member_of_a_variant_as_its_default_under_each_tagging() {
    assert_eq!(
        serde_json::from_str::<Berthing>(r#"{"kind":"Docked","pier":3}"#).unwrap(),
        Berthing::Docked {
            dock: String::new(),
            pier: 3
        }
    );
    for (written, expected) in [
        (
            written_back::<ExternalBerthing>(r#"{"Docked":{"pier":3}}"#),
            r#"{"Docked":{"dock":"","pier":3}}"#,
        ),
        (
            written_back::<Berthing>(r#"{"kind":"Docked","pier":3}"#),
            r#"{"kind":"Docked","dock":"","pier":3}"#,
        ),
        (
            written_back::<AdjacentBerthing>(r#"{"kind":"Docked","data":{"pier":3}}"#),
            r#"{"kind":"Docked","data":{"dock":"","pier":3}}"#,
        ),
        (
            written_back::<UntaggedBerthing>(r#"{"pier":3}"#),
            r#"{"dock":"","pier":3}"#,
        ),
    ] {
        assert_eq!(written, expected);
    }
}

/// The table prints one row for each payload, a refusal followed by its messages.
#[test]
fn the_variant_table_prints_a_row_for_each_payload() {
    let rows: Vec<&str> = VARIANT_REPORT
        .lines()
        .filter(|line| !line.starts_with("    "))
        .collect();
    assert_eq!(rows.len(), VARIANT_CASES.len());
    for (row, &(name, _, _)) in rows.iter().zip(&VARIANT_CASES) {
        assert!(row.starts_with(&format!("{name}: ")), "{row}");
    }
}

/// Every row of the table, as serde reads it where a schema surface is on: a build with none
/// holds `line` to no bound.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn serde_reads_each_variant_payload_as_the_table_says() {
    assert_eq!(variant_report(), VARIANT_REPORT);
}

/// An `Option`, a member whose key serde never reads, and a member of an enum serde never reads
/// or reads through another type are read as before.
#[test]
fn the_members_no_refusal_reaches_read_as_before() {
    assert_eq!(
        serde_json::from_str::<OptionalBerth>(r#"{"kind":"Docked","pier":3}"#).unwrap(),
        OptionalBerth::Docked {
            dock: None,
            pier: 3
        }
    );
    assert_eq!(
        serde_json::from_str::<SkippedBerth>(r#"{"kind":"Docked","line":"l","pier":3}"#).unwrap(),
        SkippedBerth::Docked {
            dock: north_dock(),
            line: String::new(),
            pier: 3
        }
    );
    assert_eq!(
        serde_json::from_str::<ReadThrough>(r#"{"kind":"Docked","pier":3}"#).unwrap(),
        ReadThrough::Docked {
            dock: String::new(),
            pier: 3
        }
    );
    assert_eq!(
        serde_json::from_str::<TriedThrough>(r#"{"kind":"Adrift"}"#)
            .unwrap_err()
            .to_string(),
        "not docked"
    );
    assert_eq!(
        serde_json::to_string(&WrittenOnly::Docked {
            dock: "d".to_owned(),
            pier: 3
        })
        .unwrap(),
        r#"{"kind":"Docked","dock":"d","pier":3}"#
    );
}

/// The enum none of whose members is read as a default, as serde writes it.
#[test]
fn the_enum_with_no_default_is_built_and_written_by_serde() {
    assert_eq!(
        serde_json::to_string(&[Tide::Ebbing { height: 2 }, Tide::Slack]).unwrap(),
        r#"[{"kind":"Ebbing","height":2},{"kind":"Slack"}]"#
    );
}

/// serde reads the default of a member of a variant of the untagged enum a struct flattens.
#[test]
fn a_struct_that_flattens_the_untagged_enum_reads_the_default_of_its_member() {
    assert_eq!(
        serde_json::from_str::<Slipway>(r#"{"label":"q","line":"l"}"#).unwrap(),
        Slipway {
            berth: UntaggedBerthing::Moored {
                dock: north_dock(),
                line: "l".to_owned()
            },
            label: "q".to_owned()
        }
    );
}

/// The read of each client reads a member of a variant left out from the JSON of its default,
/// through the read the member has, as it reads a member of a struct: the text each writes for
/// `Berthing` and for the list of the parameter of `Manifest`.
#[cfg(feature = "dart")]
#[test]
fn dart_reads_a_member_of_a_variant_left_out_from_the_json_of_its_default() {
    let written = super::models::berthing_dart::dart_definition();
    for read in [
        "BerthingDocked(dock: json.containsKey('dock') ? (json['dock'] as String) : \
         (jsonDecode('\"\"') as String),pier: (switch (json['pier'] as int) { final number when \
         number >= 0 && number <= 4294967295 => number, final number => throw \
         FormatException('expected an integer from 0 to 4294967295, found $number') }),)",
        "BerthingMoored(dock: json.containsKey('dock') ? (json['dock'] as String) : \
         (jsonDecode('\"north\"') as String),line: json.containsKey('line') ? (json['line'] as \
         String) : (jsonDecode('\"\"') as String),)",
    ] {
        assert!(written.contains(read), "{read}\n{written}");
    }
    let manifest = super::models::manifest_dart::dart_definition();
    assert!(
        manifest.contains("(jsonDecode('[]') as List<dynamic>)"),
        "{manifest}"
    );
}

#[cfg(feature = "swift")]
#[test]
fn swift_reads_a_member_of_a_variant_left_out_from_the_json_of_its_default() {
    let written = super::models::berthing_swift::swift_definition();
    assert!(
        written.contains(
            "if container.contains(.dock) { self.dock = try container.decode(String.self, \
             forKey: .dock) } else { let dockDefaultPayload = try \
             JSONSerialization.jsonObject(with: Data(\"\\\"north\\\"\".utf8), options: \
             [.fragmentsAllowed]); self.dock = try SwiftSchemaPayload.read(String.self, from: \
             dockDefaultPayload, at: container.codingPath + [CodingKeys.dock]) }"
        ),
        "{written}"
    );
}

#[cfg(feature = "kotlin")]
#[test]
fn kotlin_reads_a_member_of_a_variant_left_out_from_the_json_of_its_default() {
    let written = without_number_readers(&super::models::berthing_kotlin::kotlin_definition());
    for class in [
        "data class BerthingDocked(val dock: String = Json.decodeFromString(serializer<String>(), \
         \"\\\"\\\"\"), val pier: UInt)",
        "data class BerthingMoored(val dock: String = Json.decodeFromString(serializer<String>(), \
         \"\\\"north\\\"\"), val line: String = Json.decodeFromString(serializer<String>(), \
         \"\\\"\\\"\"))",
    ] {
        assert!(written.contains(class), "{class}\n{written}");
    }
    let manifest = super::models::manifest_kotlin::kotlin_definition();
    assert!(
        manifest.contains("val moored: List<Moored> = emptyList()"),
        "{manifest}"
    );
}

/// The Zod schema of each tagging parses a default as it parses a payload: `.prefault` hands the
/// parse the JSON of the default, which one index across the enum keeps apart from another
/// variant's member of the same name.
#[cfg(feature = "zod")]
#[test]
fn the_zod_schema_parses_the_default_of_each_member_of_a_variant() {
    let docked = "dock: z.string().prefault(\"\")";
    let moored = "dock: z.string().prefault(\"north\")";
    let line = "line: z.string().min(1, { error: (issue) => `too short: minimum length is 1, got \
                ${String(issue.input).length}` }).prefault(\"\")";
    for schema in [
        Berthing::zod_schema(),
        ExternalBerthing::zod_schema(),
        AdjacentBerthing::zod_schema(),
        UntaggedBerthing::zod_schema(),
    ] {
        for member in [
            docked,
            moored,
            line,
            "pier: z.number().int().min(0).max(4294967295),",
        ] {
            assert!(schema.contains(member), "{member}\n{schema}");
        }
        assert!(!schema.contains('\u{1}'), "{schema}");
    }
}

/// The JSON Schema of each tagging leaves each key read as a default out of `required` and states
/// the default.
#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_states_the_default_of_each_member_of_a_variant() {
    let docked = |object: &serde_json::Value| {
        assert_eq!(object["required"], serde_json::json!(["pier"]), "{object}");
        assert_eq!(object["properties"]["dock"]["default"], "", "{object}");
        assert!(
            object["properties"]["pier"].get("default").is_none(),
            "{object}"
        );
    };
    let moored = |object: &serde_json::Value| {
        assert_eq!(object["required"], serde_json::json!([]), "{object}");
        assert_eq!(object["properties"]["dock"]["default"], "north", "{object}");
        assert_eq!(object["properties"]["line"]["default"], "", "{object}");
    };
    let external = ExternalBerthing::json_schema();
    docked(&external["oneOf"][1]["properties"]["Docked"]);
    moored(&external["oneOf"][2]["properties"]["Moored"]);
    let adjacent = AdjacentBerthing::json_schema();
    docked(&adjacent["oneOf"][1]["properties"]["data"]);
    moored(&adjacent["oneOf"][2]["properties"]["data"]);
    let untagged = UntaggedBerthing::json_schema();
    docked(&untagged["anyOf"][0]);
    moored(&untagged["anyOf"][1]);

    let internal = Berthing::json_schema();
    let beside_the_tag = &internal["oneOf"][1];
    assert_eq!(
        beside_the_tag["required"],
        serde_json::json!(["kind", "pier"]),
        "{internal}"
    );
    assert_eq!(beside_the_tag["properties"]["dock"]["default"], "");
    assert_eq!(
        internal["oneOf"][2]["required"],
        serde_json::json!(["kind"]),
        "{internal}"
    );
    assert_eq!(
        internal["oneOf"][2]["properties"]["dock"]["default"],
        "north"
    );
}

/// No text of the defaults of the untagged enum reaches the Zod schema of a type that holds it,
/// flattens it, flattens its alias, or flattens a union that holds it, nor of a type that
/// flattens an enum under no tag attribute: each joins the enum by its binding.
#[cfg(feature = "zod")]
#[test]
fn no_placeholder_reaches_the_zod_schema_of_another_type() {
    for (schema, binding) in [
        (Quay::zod_schema(), "berth: UntaggedBerthing$Schema,"),
        (Slipway::zod_schema(), "UntaggedBerthing$Schema"),
        (Pier::zod_schema(), "AliasedBerthingType$Schema"),
        (Mooring::zod_schema(), "UntaggedBerthing$Schema"),
        (Wake::zod_schema(), "Towage$Schema"),
    ] {
        assert!(!schema.contains('\u{1}'), "{schema}");
        assert!(!schema.contains(".prefault("), "{schema}");
        assert!(schema.contains(binding), "{binding}\n{schema}");
    }
    assert!(!Anchorage::zod_schema().contains('\u{1}'));
    assert!(!Towage::zod_schema().contains('\u{1}'));
}

/// The text of each surface for a member no refusal reaches is the text it was: no default is
/// written, and the key the targets read as before keeps its spelling.
#[cfg(feature = "zod")]
#[test]
fn the_zod_schema_of_a_member_no_refusal_reaches_is_as_before() {
    for schema in [
        SkippedBerth::zod_schema(),
        ReadThrough::zod_schema(),
        TriedThrough::zod_schema(),
        WrittenOnly::zod_schema(),
        OptionalBerth::zod_schema(),
    ] {
        assert!(!schema.contains(".prefault(\""), "{schema}");
    }
    assert!(ReadThrough::zod_schema().contains("  dock: z.string(),\n"));
    assert!(WrittenOnly::zod_schema().contains("  dock: z.string(),\n"));
}

/// An enum with no `#[serde(default)]` writes on every surface the text it wrote before a member
/// of a variant was read as its default: `Tide`, pinned whole.
#[cfg(all(feature = "typescript", feature = "jsonschema"))]
#[test]
fn an_enum_with_no_default_writes_the_typescript_text_it_wrote() {
    let lines = [
        "/**",
        " * An enum none of whose members is read as a default.",
        " * ",
        " */",
        "export type Tide = {",
        "  /**",
        "   * Ebbing",
        "   * ",
        "   */",
        "  kind: \"Ebbing\";",
        "  /**",
        "   * height",
        "   * ",
        "   */",
        "  height: number;",
        "} | {",
        "  /**",
        "   * Slack",
        "   * ",
        "   */",
        "  kind: \"Slack\";",
        "};",
        "",
        "/** The Rust variant name `value` carries, or \"\" where it carries none. */",
        "export function Tide$Variant(value: unknown): string {",
        "  if (typeof value !== \"object\" || value === null) return \"\";",
        "  switch ((value as { kind?: unknown }).kind) {",
        "    case \"Ebbing\": return \"Ebbing\";",
        "    case \"Slack\": return \"Slack\";",
        "    default: return \"\";",
        "  }",
        "}",
    ];
    assert_eq!(Tide::ts_definition(), lines.join("\n"));
}

#[cfg(feature = "zod")]
#[test]
fn an_enum_with_no_default_writes_the_zod_text_it_wrote() {
    assert_eq!(
        Tide::zod_schema(),
        "const Tide$RawSchema = z.discriminatedUnion(\"kind\", [z.strictObject({
  kind: z.literal(\"Ebbing\"),
  height: z.number().int().min(0).max(4294967295),
}), z.strictObject({
  kind: z.literal(\"Slack\"),
})]);

export const Tide$Schema: ZodType<Tide> = Tide$RawSchema.meta({
  examples: []
});"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_enum_with_no_default_writes_the_json_schema_it_wrote() {
    assert_eq!(
        serde_json::to_string(&Tide::json_schema()).unwrap(),
        "{\"type\":\"object\",\"oneOf\":[{\"additionalProperties\":false,\
         \"properties\":{\"kind\":{\"type\":\"string\",\"const\":\"Ebbing\"},\
         \"height\":{\"type\":\"integer\"}},\"required\":[\"kind\",\"height\"]},\
         {\"additionalProperties\":false,\"properties\":{\"kind\":{\"type\":\"string\",\
         \"const\":\"Slack\"}},\"required\":[\"kind\"]}]}"
    );
}

#[cfg(feature = "dart")]
#[test]
fn an_enum_with_no_default_writes_the_dart_text_it_wrote() {
    assert_eq!(
        tide_dart::dart_definition(),
        "sealed class Tide { const Tide(); factory Tide.fromJson(Map<String, dynamic> json) { \
         final read = Tide.fromJsonUnjudged(json); final violations = read.validate(); if \
         (violations.isNotEmpty) throw FormatException(violations.join('; ')); return read; } \
         factory Tide.fromJsonUnjudged(Map<String, dynamic> json) { final tag = json['kind'] as \
         String; switch (tag) { case 'Ebbing': return TideEbbing.fromJsonUnjudged(json); case \
         'Slack': return TideSlack(); default: throw FormatException('Unknown Tide: ' + tag); } } \
         dynamic toJson(); List<String> validate(); } class TideEbbing extends Tide { const \
         TideEbbing({required this.height,}); final int height; factory \
         TideEbbing.fromJson(Map<String, dynamic> json) { final read = \
         TideEbbing.fromJsonUnjudged(json); final violations = read.validate(); if \
         (violations.isNotEmpty) throw FormatException(violations.join('; ')); return read; } \
         factory TideEbbing.fromJsonUnjudged(Map<String, dynamic> json) => TideEbbing(height: \
         (switch (json['height'] as int) { final number when number >= 0 && number <= \
         4294967295 => number, final number => throw FormatException('expected an integer from \
         0 to 4294967295, found $number') }),); Map<String, dynamic> toJson() => { 'kind': \
         'Ebbing','height': (switch (height) { final number when number >= 0 && number <= \
         4294967295 => number, final number => throw FormatException('expected an integer from \
         0 to 4294967295, found $number') }), }; @override List<String> validate() => const []; \
         } class TideSlack extends \
         Tide { const TideSlack(); @override dynamic toJson() => { 'kind': 'Slack' }; @override \
         List<String> validate() => const []; }"
    );
}

#[cfg(feature = "swift")]
#[test]
fn an_enum_with_no_default_writes_the_swift_text_it_wrote() {
    assert_eq!(
        tide_swift::swift_definition(),
        "public struct TideEbbingPayload: Codable, Sendable { public let height: UInt32; enum \
         CodingKeys: String, CodingKey { case height;  }; public init(height: UInt32) { \
         self.height = height;  }; public init(swiftSchemaUnjudged decoder: Decoder) throws { \
         let container = try decoder.container(keyedBy: CodingKeys.self); self.height = try \
         container.decode(UInt32.self, forKey: .height) }; public init(from decoder: Decoder) \
         throws { try self.init(swiftSchemaUnjudged: decoder); let violations = validate(); if \
         !violations.isEmpty { throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
         debugDescription: violations.joined(separator: \"; \"))) } }; public func validate() -> \
         [String] { [] } };  public enum Tide: Codable, Sendable { case \
         ebbing(TideEbbingPayload); case slack;  private enum SwiftSchemaTagCodingKeys: String, \
         CodingKey { case swiftSchemaTag = \"kind\" }; public init(swiftSchemaUnjudged decoder: \
         Decoder) throws { let container = try decoder.container(keyedBy: \
         SwiftSchemaTagCodingKeys.self); let tag = try container.decode(String.self, forKey: \
         .swiftSchemaTag); switch tag { case \"Ebbing\": self = .ebbing(try \
         TideEbbingPayload(swiftSchemaUnjudged: decoder)); case \"Slack\": self = .slack;  \
         default: throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: \
         decoder.codingPath, debugDescription: \"unknown tag \\(tag)\")) } }; public init(from \
         decoder: Decoder) throws { try self.init(swiftSchemaUnjudged: decoder); let violations \
         = validate(); if !violations.isEmpty { throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
         debugDescription: violations.joined(separator: \"; \"))) } }; public func encode(to \
         encoder: Encoder) throws { var container = encoder.container(keyedBy: \
         SwiftSchemaTagCodingKeys.self); switch self { case .ebbing(let payload): try \
         container.encode(\"Ebbing\", forKey: .swiftSchemaTag); try payload.encode(to: encoder); \
         case .slack: try container.encode(\"Slack\", forKey: .swiftSchemaTag);  } }; public \
         func validate() -> [String] { [] } }"
    );
}

#[cfg(feature = "kotlin")]
#[test]
fn an_enum_with_no_default_writes_the_kotlin_text_it_wrote() {
    assert_eq!(
        without_number_readers(&tide_kotlin::kotlin_definition()),
        "@Serializable(with = TideSerializer::class) sealed interface Tide { fun validate(): \
         List<String> } @SerialName(\"Ebbing\") @Serializable data class TideEbbing(val height: \
         UInt) : Tide { override fun validate(): List<String> = emptyList() } \
         @SerialName(\"Slack\") @Serializable data object TideSlack : Tide { override fun \
         validate(): List<String> = emptyList() } object TideUnjudged : KSerializer<Tide> { \
         override val descriptor: SerialDescriptor = buildClassSerialDescriptor(\"Tide\"); \
         override fun serialize(encoder: Encoder, value: Tide) { val output = encoder as \
         JsonEncoder; output.encodeJsonElement(when (value) { is TideEbbing -> buildJsonObject { \
         put(\"kind\", \"Ebbing\"); output.json.encodeToJsonElement(serializer<TideEbbing>(), \
         value).jsonObject.forEach { (k, v) -> put(k, v) } }; is TideSlack -> buildJsonObject { \
         put(\"kind\", \"Slack\") } }) }; override fun deserialize(decoder: Decoder): Tide { val \
         input = decoder as JsonDecoder; val obj = input.decodeJsonElement().let { it as? \
         JsonObject ?: throw SerializationException(\"expected an object, found \" + it) }; val \
         tag = (obj[\"kind\"] as? JsonPrimitive)?.content; return when (tag) { \"Ebbing\" -> \
         input.json.decodeFromJsonElement(serializer<TideEbbing>(), JsonObject(obj - \"kind\")); \
         \"Slack\" -> { if (!input.json.configuration.ignoreUnknownKeys) (obj - \
         \"kind\").keys.firstOrNull()?.let { throw SerializationException(\"Encountered an \
         unknown key '\" + it + \"'\") }; TideSlack } else -> throw \
         SerializationException(\"unknown tag \" + tag) } } } object TideSerializer : \
         KSerializer<Tide>, () -> KSerializer<Tide> { \
         override fun invoke(): KSerializer<Tide> = TideUnjudged; override val descriptor: \
         SerialDescriptor = TideUnjudged.descriptor; override fun serialize(encoder: Encoder, \
         value: Tide) = \
         TideUnjudged.serialize(encoder, value); override fun deserialize(decoder: Decoder): \
         Tide { val read = TideUnjudged.deserialize(decoder); val violations = read.validate(); \
         if (violations.isNotEmpty()) throw SerializationException(violations.joinToString(\"; \
         \")); return read } }"
    );
}
