//! A member of an enum's variant under `#[serde(default)]` the payload leaves out: what serde
//! reads, what the text of each surface says of it, and the members that build as before. The
//! runs of each client are in the emitted-client groups, and the run of the Zod schemas under node
//! in `generic_types_tests`.

#[cfg(all(feature = "kotlin", feature = "serde"))]
use crate::kotlin_numbers::without_number_readers;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[cfg(all(feature = "serde", feature = "zod"))]
use super::models::Quay;
#[cfg(feature = "serde")]
use super::models::variant_report;
use super::models::{
    AdjacentBerthing, Berthing, BerthingDraft, ExternalBerthing, UntaggedBerthing,
    UntaggedBerthingDraft, VARIANT_CASES, VARIANT_REPORT, north_dock,
};

/// The untagged enum under another name.
#[cfg(all(feature = "serde", feature = "zod"))]
#[model_schema()]
pub type AliasedBerthing = UntaggedBerthing;

/// An untagged enum one variant of which holds the untagged enum whose members are read as their
/// defaults.
#[cfg(all(feature = "serde", feature = "zod"))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Anchorage {
    Held(UntaggedBerthing),
    Named { harbour: String },
}

/// A struct that flattens the untagged enum that holds the other.
#[cfg(all(feature = "serde", feature = "zod"))]
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
#[cfg(all(feature = "serde", feature = "zod"))]
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
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum Tide {
    Ebbing { height: u32 },
    Slack,
}

/// An enum under no tag attribute with no unit variant, which a struct flattens.
#[cfg(all(feature = "serde", feature = "zod"))]
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
#[cfg(all(feature = "serde", feature = "zod"))]
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

impl From<Berthing> for ReadThroughDraft {
    fn from(read: Berthing) -> Self {
        if let BerthingDraft::Docked { dock, pier } = BerthingDraft::from(read) {
            Self::Docked { dock, pier }
        } else {
            Self::Docked {
                dock: String::new(),
                pier: 0,
            }
        }
    }
}

impl TryFrom<Berthing> for TriedThroughDraft {
    type Error = String;

    fn try_from(read: Berthing) -> Result<Self, Self::Error> {
        if let BerthingDraft::Docked { dock, pier } = BerthingDraft::from(read) {
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
        BerthingDraft::Docked {
            dock: String::new(),
            pier: 3
        }
        .validate()
        .unwrap()
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

/// Every row of the table, as serde reads it.
#[cfg(feature = "serde")]
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
        OptionalBerthDraft::Docked {
            dock: None,
            pier: 3
        }
        .validate()
        .unwrap()
    );
    assert_eq!(
        serde_json::from_str::<SkippedBerth>(r#"{"kind":"Docked","line":"l","pier":3}"#).unwrap(),
        SkippedBerthDraft::Docked {
            dock: north_dock(),
            line: String::new(),
            pier: 3
        }
        .validate()
        .unwrap()
    );
    assert_eq!(
        serde_json::from_str::<ReadThrough>(r#"{"kind":"Docked","pier":3}"#).unwrap(),
        ReadThroughDraft::Docked {
            dock: String::new(),
            pier: 3
        }
        .validate()
        .unwrap()
    );
    assert_eq!(
        serde_json::from_str::<TriedThrough>(r#"{"kind":"Adrift"}"#)
            .unwrap_err()
            .to_string(),
        "not docked"
    );
    assert_eq!(
        serde_json::to_string(&WrittenOnlyDraft::Docked {
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
        serde_json::to_string(&[TideDraft::Ebbing { height: 2 }, TideDraft::Slack]).unwrap(),
        r#"[{"kind":"Ebbing","height":2},{"kind":"Slack"}]"#
    );
}

/// serde reads the default of a member of a variant of the untagged enum a struct flattens.
#[test]
fn a_struct_that_flattens_the_untagged_enum_reads_the_default_of_its_member() {
    assert_eq!(
        serde_json::from_str::<Slipway>(r#"{"label":"q","line":"l"}"#).unwrap(),
        SlipwayDraft {
            berth: UntaggedBerthingDraft::Moored {
                dock: north_dock(),
                line: "l".to_owned()
            }
            .validate()
            .unwrap(),
            label: "q".to_owned()
        }
        .validate()
        .unwrap()
    );
}

/// The read of each client reads a member of a variant left out from the JSON of its default,
/// through the read the member has, as it reads a member of a struct: the text each writes for
/// `Berthing` and for the list of the parameter of `Manifest`.
#[cfg(all(feature = "dart", feature = "serde"))]
#[test]
fn dart_reads_a_member_of_a_variant_left_out_from_the_json_of_its_default() {
    let written = super::models::berthing_dart::dart_definition();
    for read in [
        "BerthingDocked(dock: json.containsKey('dock') ? _readAt(json, 'dock', () => json['dock'] \
         as String) : (jsonDecode('\"\"') as String),pier: _readAt(json, 'pier', () => (switch \
         (json['pier'] as int) { final number when number >= 0 && number <= 4294967295 => number, \
         final number => throw FormatException('expected an integer from 0 to 4294967295, found \
         $number') })),)",
        "BerthingMoored(dock: json.containsKey('dock') ? _readAt(json, 'dock', () => json['dock'] \
         as String) : (jsonDecode('\"north\"') as String),line: json.containsKey('line') ? \
         _readAt(json, 'line', () => json['line'] as String) : (jsonDecode('\"\"') as String),)",
    ] {
        assert!(written.contains(read), "{read}\n{written}");
    }
    let manifest = super::models::manifest_dart::dart_definition();
    assert!(
        manifest.contains("(jsonDecode('[]') as List<dynamic>)"),
        "{manifest}"
    );
}

#[cfg(all(feature = "swift", feature = "serde"))]
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

#[cfg(all(feature = "kotlin", feature = "serde"))]
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
#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn the_zod_schema_parses_the_default_of_each_member_of_a_variant() {
    let docked = "dock: z.string().prefault(\"\")";
    let moored = "dock: z.string().prefault(\"north\")";
    let line = "line: z.string().min(1, { error: (issue) => `too short: minimum length is 1, got \
                ${[...String(issue.input)].length}` }).prefault(\"\")";
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
#[cfg(feature = "serde")]
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
/// flattens an enum under no tag attribute: each joins the enum by its binding, or by the members
/// it publishes.
#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn no_placeholder_reaches_the_zod_schema_of_another_type() {
    for (schema, binding) in [
        (Quay::zod_schema(), "berth: UntaggedBerthing$Schema,"),
        (Slipway::zod_schema(), "UntaggedBerthing$Members"),
        (Pier::zod_schema(), "UntaggedBerthing$Members"),
        (Mooring::zod_schema(), "UntaggedBerthing$Members"),
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
#[cfg(all(feature = "serde", feature = "zod"))]
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
#[cfg(all(feature = "serde", feature = "typescript"))]
#[test]
fn an_enum_with_no_default_writes_the_typescript_text_it_wrote() {
    let lines = [
        "/**",
        " * An enum none of whose members is read as a default.",
        " * ",
        " */",
        "export type TideDraft = {",
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
        "export type Tide = Readonly<TideDraft> & $brand<\"Tide\">;",
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

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn an_enum_with_no_default_writes_the_zod_text_it_wrote() {
    assert_eq!(
        Tide::zod_schema(),
        "const TideDraft$RawSchema = z.discriminatedUnion(\"kind\", [z.strictObject({
  kind: z.literal(\"Ebbing\"),
  height: z.number().int().min(0).max(4294967295),
}), z.strictObject({
  kind: z.literal(\"Slack\"),
})]).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true });

export const TideDraft$Schema: ZodType<TideDraft> = TideDraft$RawSchema;

export const Tide$Schema = TideDraft$Schema.readonly().brand<\"Tide\">().meta(TideDraft$Schema.meta() ?? {}).meta({
  examples: []
});"
    );
}

#[cfg(feature = "serde")]
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

#[cfg(all(feature = "dart", feature = "serde"))]
#[test]
fn an_enum_with_no_default_writes_the_dart_text_it_wrote() {
    assert_eq!(
        tide_dart::dart_definition(),
        "sealed class TideDraft { const TideDraft(); factory TideDraft.fromJson(Map<String, \
         dynamic> json) { final tag = _readAt(json, 'kind', () => json['kind'] as String); \
         switch (tag) { case 'Ebbing': return _readFlattened(() => TideEbbing.fromJson(json)); \
         case 'Slack': return (() { for (final key in json.keys) { if (!const \
         <String>{'kind'}.contains(key)) throw FormatException('Unknown key of a Tide: $key', \
         <Object>[key]); } return TideSlack(); })(); default: throw FormatException('Unknown \
         Tide: ' + tag); } } static Read _readAt<Read>(Map<String, dynamic>? object, Object \
         key, Read Function() read) { try { return read(); } catch (refused) { if (object != \
         null && !object.containsKey(key)) throw FormatException('missing field `$key`', \
         <Object>[key]); if (refused is TypeError) throw FormatException('$refused', \
         <Object>[key]); if (refused is FormatException) { final at = refused.source; throw \
         FormatException(refused.message, <Object>[key, if (at is List<Object>) ...at]); } \
         rethrow; } } static Read _readBuffered<Read>(Read Function() read) { try { return \
         read(); } on FormatException catch (refused) { if (refused.message != 'expected \
         value') rethrow; throw const FormatException('invalid type: expected string or map'); \
         } } static Read _readFlattened<Read>(Read Function() read) { try { return \
         _readBuffered(read); } on FormatException catch (refused) { final at = refused.source; \
         if (at is! List<Object> || at.isEmpty) rethrow; final message = refused.message; throw \
         FormatException(message, <Object>[if (message.startsWith('missing field ') || \
         message.startsWith('Unknown key of a ')) at.last]); } } dynamic toJson(); List<String> \
         validationIssues(); Tide validate() { final found = validationIssues(); if \
         (found.isNotEmpty) throw FormatException(found.join('; ')); return Tide._(this); } } \
         class Tide { const Tide._(this._draft); final TideDraft _draft; TideDraft toDraft() => \
         _draft; factory Tide.fromJson(Map<String, dynamic> json) { final read = \
         TideDraft.fromJson(json); final violations = read.validationIssues(); if \
         (violations.isNotEmpty) throw FormatException(violations.join('; ')); return \
         Tide._(read); } factory Tide.fromJsonUnjudged(Map<String, dynamic> json) => \
         Tide._(TideDraft.fromJson(json)); dynamic toJson() => _draft.toJson(); List<String> \
         validationIssues() => _draft.validationIssues(); static Object $thrown(Object thrown) => \
         thrown is TideDraft ? thrown.validate() : thrown; } class TideEbbing extends TideDraft \
         { const TideEbbing({required this.height,}); final int height; factory \
         TideEbbing.fromJson(Map<String, dynamic> json) { for (final key in json.keys) { if \
         (!const <String>{'kind', 'height'}.contains(key)) throw FormatException('Unknown key \
         of a TideEbbing: $key', <Object>[key]); } return TideEbbing(height: _readAt(json, \
         'height', () => (switch (json['height'] as int) { final number when number >= 0 && \
         number <= 4294967295 => number, final number => throw FormatException('expected an \
         integer from 0 to 4294967295, found $number') })),); } static Read \
         _readAt<Read>(Map<String, dynamic>? object, Object key, Read Function() read) { try { \
         return read(); } catch (refused) { if (object != null && !object.containsKey(key)) \
         throw FormatException('missing field `$key`', <Object>[key]); if (refused is \
         TypeError) throw FormatException('$refused', <Object>[key]); if (refused is \
         FormatException) { final at = refused.source; throw FormatException(refused.message, \
         <Object>[key, if (at is List<Object>) ...at]); } rethrow; } } Map<String, dynamic> \
         toJson() => { 'kind': 'Ebbing','height': (switch (height) { final number when number \
         >= 0 && number <= 4294967295 => number, final number => throw \
         FormatException('expected an integer from 0 to 4294967295, found $number') }), }; \
         @override List<String> validationIssues() => const []; } class TideSlack extends \
         TideDraft { const TideSlack(); @override dynamic toJson() => { 'kind': 'Slack' }; \
         @override List<String> validationIssues() => const []; }"
    );
}

#[cfg(all(feature = "swift", feature = "serde"))]
#[test]
fn an_enum_with_no_default_writes_the_swift_text_it_wrote() {
    use crate::swift_fitted::FITTED;
    use crate::swift_valid;

    assert_eq!(
        swift_valid::draft_of(&tide_swift::swift_definition()),
        format!(
            "{}{FITTED}; {}{} }}",
            "public struct TideEbbingPayload: Codable, Sendable { public let height: UInt32; enum \
             CodingKeys: String, CodingKey { case height;  }; public init(height: UInt32) { \
             self.height = height;  }; private struct SwiftSchemaAnyKey: CodingKey { var \
             stringValue: String; var intValue: Int? { nil }; init?(stringValue: String) { \
             self.stringValue = stringValue }; init?(intValue: Int) { nil } }; ",
            "public init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: [String]) \
             throws { let container = try decoder.container(keyedBy: CodingKeys.self); self.height \
             = try container.decode(SwiftSchemaFit<UInt32>.self, forKey: .height).value }; public init(swiftSchemaUnjudged decoder: Decoder) throws { if !(decoder.codingPath.isEmpty && decoder.userInfo[CodingUserInfoKey(rawValue: \"swiftSchemaFlat\")!] != nil) { for unknown in try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys where ![\"height\"].contains(unknown.stringValue) { throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath + [unknown], debugDescription: \"unknown field `\\(unknown.stringValue)`\")) } }; try self.init(swiftSchemaUnjudged: decoder, swiftSchemaWithout: []) }; public init(from decoder: Decoder) throws { try self.init(swiftSchemaUnjudged: decoder); let violations = validate(); if !violations.isEmpty { throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: violations.joined(separator: \"; \"))) } }; public func validate() -> [String] { [] }; public func swiftSchemaValidateBound() -> [String] { [] } };  public enum TideDraft: Codable, Sendable { case ebbing(TideEbbingPayload); case slack;  private enum SwiftSchemaTagCodingKeys: String, CodingKey { case swiftSchemaTag = \"kind\" }; private struct SwiftSchemaAnyKey: CodingKey { var stringValue: String; var intValue: Int? { nil }; init?(stringValue: String) { self.stringValue = stringValue }; init?(intValue: Int) { nil } }; private static func swiftSchemaFlattened(_ refused: DecodingError, at path: [CodingKey]) -> DecodingError { func placed(_ context: DecodingError.Context, keeping kept: [CodingKey] = []) -> DecodingError.Context { DecodingError.Context(codingPath: path + kept, debugDescription: context.debugDescription == \"expected value\" ? \"invalid type: expected string or map\" : context.debugDescription, underlyingError: context.underlyingError) }; if case .keyNotFound(let key, let context) = refused { return .keyNotFound(key, placed(context)) }; if case .typeMismatch(let type, let context) = refused { return .typeMismatch(type, placed(context)) }; if case .valueNotFound(let type, let context) = refused { return .valueNotFound(type, placed(context)) }; if case .dataCorrupted(let context) = refused { return .dataCorrupted(placed(context, keeping: context.debugDescription.hasPrefix(\"unknown field `\") ? Array(context.codingPath.suffix(1)) : [])) }; return refused }; fileprivate init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: [String]) throws { let container = try decoder.container(keyedBy: SwiftSchemaTagCodingKeys.self); let tag = try container.decode(String.self, forKey: .swiftSchemaTag); switch tag { case \"Ebbing\": do { self = .ebbing(try TideEbbingPayload(swiftSchemaUnjudged: decoder, swiftSchemaWithout: taken + [\"kind\"])) } catch let refused as DecodingError { throw Self.swiftSchemaFlattened(refused, at: decoder.codingPath) }; case \"Slack\": if !(decoder.codingPath.isEmpty && decoder.userInfo[CodingUserInfoKey(rawValue: \"swiftSchemaFlat\")!] != nil) { for unknown in try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys where unknown.stringValue != \"kind\" && !taken.contains(unknown.stringValue) { throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath + [unknown], debugDescription: \"unknown field `\\(unknown.stringValue)`\")) } }; self = .slack;  default: throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: \"unknown tag \\(tag)\")) } }; fileprivate init(swiftSchemaUnjudged decoder: Decoder) throws { if !(decoder.codingPath.isEmpty && decoder.userInfo[CodingUserInfoKey(rawValue: \"swiftSchemaFlat\")!] != nil) { for unknown in try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys where ![\"kind\", \"height\"].contains(unknown.stringValue) { throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath + [unknown], debugDescription: \"unknown field `\\(unknown.stringValue)`\")) } }; try self.init(swiftSchemaUnjudged: decoder, swiftSchemaWithout: []) }; public init(from decoder: Decoder) throws { try self.init(swiftSchemaUnjudged: decoder); let violations = validationIssues(); if !violations.isEmpty { throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: violations.joined(separator: \"; \"))) } }; public func encode(to encoder: Encoder) throws { var container = encoder.container(keyedBy: SwiftSchemaTagCodingKeys.self); switch self { case .ebbing(let payload): try container.encode(\"Ebbing\", forKey: .swiftSchemaTag); try payload.encode(to: encoder); case .slack: try container.encode(\"Slack\", forKey: .swiftSchemaTag);  } }; public func validationIssues() -> [String] { [] }; public func swiftSchemaValidateBound() -> [String] { [] }; ",
            swift_valid::validating("Tide")
        )
    );
}

#[cfg(all(feature = "kotlin", feature = "serde"))]
#[test]
fn an_enum_with_no_default_writes_the_kotlin_text_it_wrote() {
    assert_eq!(
        without_number_readers(&tide_kotlin::kotlin_definition()),
        "@Serializable(with = TideDraftUnjudged::class) sealed interface TideDraft { fun \
         validationIssues(): List<String>; fun validate(): Tide { val violations = \
         validationIssues(); if (violations.isNotEmpty()) throw \
         SerializationException(violations.joinToString(\"; \"), \
         IllegalArgumentException(\"validate()\")); return Tide.checked(this) } } \
         @SerialName(\"Ebbing\") @Serializable data class TideEbbing(val height: UInt) : TideDraft \
         { override fun validationIssues(): List<String> = emptyList() } @SerialName(\"Slack\") \
         @Serializable data object TideSlack : TideDraft { override fun validationIssues(): \
         List<String> = emptyList() } object TideDraftUnjudged : KSerializer<TideDraft> { override \
         val descriptor: SerialDescriptor = buildClassSerialDescriptor(\"Tide\"); override fun \
         serialize(encoder: Encoder, value: TideDraft) { val output = encoder as JsonEncoder; \
         output.encodeJsonElement(when (value) { is TideEbbing -> buildJsonObject { put(\"kind\", \
         \"Ebbing\"); output.json.encodeToJsonElement(serializer<TideEbbing>(), \
         value).jsonObject.forEach { (k, v) -> put(k, v) } }; is TideSlack -> buildJsonObject { \
         put(\"kind\", \"Slack\") } }) }; private fun kotlinSchemaRebased(refused: \
         SerializationException, at: String): SerializationException { if (refused.cause?.message \
         == \"validate()\") return refused; val said = \
         refused.message.orEmpty().replaceFirst(Regex(\"^Unexpected JSON token at offset \\\\d+: \
         \"), \"\"); return SerializationException(if (\" at path: \\$\" in said) \
         said.replaceFirst(\" at path: \\$\", \" at path: \" + at) else said + \" at path: \" + \
         at, refused) }; @OptIn(ExperimentalSerializationApi::class) private inline fun \
         <KotlinSchemaRead> kotlinSchemaAt(input: Decoder, read: () -> KotlinSchemaRead): \
         KotlinSchemaRead = try { read() } catch (refused: SerializationException) { val at = try \
         { (input as? JsonDecoder)?.decodeJsonElement(); null } catch (probe: \
         JsonDecodingException) { probe.path } ?: throw refused; throw \
         kotlinSchemaRebased(refused, at) }; private inline fun <KotlinSchemaRead> \
         kotlinSchemaUnder(step: String, read: () -> KotlinSchemaRead): KotlinSchemaRead = try { \
         read() } catch (refused: SerializationException) { throw kotlinSchemaRebased(refused, \
         \"\\$\" + step) }; private fun <KotlinSchemaRead> kotlinSchemaInner(json: Json, step: \
         String, serializer: DeserializationStrategy<KotlinSchemaRead>, element: JsonElement): \
         KotlinSchemaRead = kotlinSchemaUnder(step) { json.decodeFromString(serializer, \
         element.toString()) }; private inline fun <KotlinSchemaRead> kotlinSchemaFlattened(read: \
         () -> KotlinSchemaRead): KotlinSchemaRead = try { read() } catch (refused: \
         SerializationException) { if (refused.cause?.message == \"validate()\") throw refused; \
         throw SerializationException(refused.message.orEmpty().replaceFirst(Regex(\"^Unexpected \
         JSON token at offset \\\\d+: \"), \"\").replaceFirst(Regex(\"^'[^']*': \"), \
         \"\").replace(Regex(\" at path: \\\\$[^\\\\n]*\"), \" at path: \\\\$\").let { said -> if \
         (said.substringBefore(\" at path: \") == \"expected value\") \"invalid type: expected \
         string or map at path: \\$\" else said }, refused) }; override fun deserialize(decoder: \
         Decoder): TideDraft { val input = decoder as JsonDecoder; val element = \
         input.decodeJsonElement(); return kotlinSchemaAt(input) { val obj = element.let { it as? \
         JsonObject ?: throw SerializationException(\"expected an object, found \" + it) }; val \
         tag = ((obj[\"kind\"] ?: throw SerializationException(\"missing field `kind`\")) as? \
         JsonPrimitive)?.content; when (tag) { \"Ebbing\" -> kotlinSchemaFlattened { \
         kotlinSchemaInner(input.json, \"\", serializer<TideEbbing>(), JsonObject(obj - \"kind\")) \
         }; \"Slack\" -> { if (!input.json.configuration.ignoreUnknownKeys) (obj - \
         \"kind\").keys.firstOrNull()?.let { throw SerializationException(\"Encountered an unknown \
         key '\" + it + \"'\") }; TideSlack } else -> throw SerializationException(\"unknown tag \
         \" + tag) } } } } @ConsistentCopyVisibility @Serializable(with = TideSerializer::class) \
         data class Tide private constructor(val draft: TideDraft) { fun validationIssues(): \
         List<String> = draft.validationIssues(); companion object { internal fun checked(draft: \
         TideDraft): Tide = Tide(draft) } } @OptIn(ExperimentalSerializationApi::class) private \
         object TideUnjudged : KSerializer<Tide> { override val descriptor: SerialDescriptor = \
         TideDraftUnjudged.descriptor; override fun serialize(encoder: Encoder, value: Tide) = \
         TideDraftUnjudged.serialize(encoder, value.draft); override fun deserialize(decoder: \
         Decoder): Tide = Tide.checked(try { TideDraftUnjudged.deserialize(decoder) } catch \
         (refused: SerializationException) { val named = TideDraftUnjudged.descriptor.serialName; \
         val said = refused.message.orEmpty(); if (\"'\" + named + \"'\" !in said) throw refused; \
         val declared = named.removeSuffix(\"Draft\"); throw if (refused is MissingFieldException) \
         MissingFieldException(refused.missingFields, declared) else \
         SerializationException(said.replace(\"'\" + named + \"'\", \"'\" + declared + \"'\"), \
         refused.cause) }) } object TideSerializer : KSerializer<Tide>, () -> KSerializer<Tide> { \
         override fun invoke(): KSerializer<Tide> = TideUnjudged; override val descriptor: \
         SerialDescriptor = TideUnjudged.descriptor; override fun serialize(encoder: Encoder, \
         value: Tide) = TideUnjudged.serialize(encoder, value); override fun deserialize(decoder: \
         Decoder): Tide { val read = TideUnjudged.deserialize(decoder); val violations = \
         read.validationIssues(); if (violations.isNotEmpty()) throw \
         SerializationException(violations.joinToString(\"; \"), \
         IllegalArgumentException(\"validate()\")); return read } }"
    );
}

/// A bound on a member of the inner untagged enum takes its variant out of the candidate set, so
/// the later variant reads a payload only it names, a stray key the later variant does not name is
/// refused, and a payload the bound accepts is read as the inner variant.
#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_bound_on_a_member_takes_its_variant_out_of_an_untagged_enum_held_in_another() {
    let read = |payload: &str| {
        serde_json::from_str::<Anchorage>(payload).map_err(|error| error.to_string())
    };
    assert_eq!(
        read(r#"{"harbour":"h"}"#),
        Ok(AnchorageDraft::Named {
            harbour: "h".to_owned()
        }
        .validate()
        .unwrap())
    );
    assert_eq!(
        read(r#"{"harbour":"h","line":""}"#),
        Err("data did not match any variant of untagged enum Anchorage".to_owned())
    );
    assert_eq!(
        read(r#"{"line":"x"}"#),
        Ok(AnchorageDraft::Held(
            UntaggedBerthingDraft::Moored {
                dock: north_dock(),
                line: "x".to_owned(),
            }
            .validate()
            .unwrap()
        )
        .validate()
        .unwrap())
    );
}
