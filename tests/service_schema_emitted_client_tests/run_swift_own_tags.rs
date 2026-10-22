//! A struct under its own tag, alone, with a type parameter, with a member under its alias, naming
//! a function on itself, with a flattened member of its own and flattened ahead of a member of its
//! holder's own, read by the emitted Swift types compiled with warnings as errors: a payload whose
//! tag is the struct's name is read and written back as serde writes it back, and one with no tag,
//! or with another value under it, is refused with `DecodingError.dataCorrupted`, as TypeScript and
//! Zod refuse it, where serde reads it. The models but [`Bunted`] are those the Kotlin runs are
//! held to, compiled with the `kotlin` feature.

#![cfg(all(feature = "swift", feature = "kotlin", feature = "serde"))]

use core::fmt::Write as _;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

use super::flattened_models::{
    DRESSING_TAGS, Dressing, FLAG_TAGS, Flag, Gonfalon, PENNANT_TAGS, Pennant, Semaphore,
    dressing_swift, ensign_swift, flag_swift, gonfalon_swift, pennant_swift, read_of,
    semaphore_swift,
};
use super::runtime::judged;
use super::tuple_variants_models::labelling_swift;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// The function [`Bunted`] names, which answers the payload as it was handed.
const AS_GIVEN: &str = "func asGiven(_ payload: Any) -> Any { payload }";

/// What prints a row: the value read, written back with its keys sorted, or the refusal of the
/// read.
const TOLD: &str = r#"
func told<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  let printed: String
  do {
    let read = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    printed = String(decoding: try encoder.encode(read), as: UTF8.self)
  } catch DecodingError.dataCorrupted(let context) {
    printed = "refused: dataCorrupted: \(context.debugDescription)"
  } catch {
    printed = "refused: \(error)"
  }
  print("\(name): \(printed)")
}
"#;

/// What each payload of a [`TagRows`] holds under the tag.
const TAGS: [&str; 4] = ["its tag", "no tag", "another tag", "a tag of another kind"];

/// [`Flag`] with `null` under its tag, which serde reads and Zod refuses.
const NULL_TAG: &str = r#"{"kind":null,"label":"n"}"#;

/// [`Semaphore`] naming a function on itself: its tag is checked on the object the function
/// answers, ahead of its member under its alias.
#[model_schema(preprocess = ["asGiven"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub struct Bunted {
    #[serde(alias = "name")]
    pub label: String,
}

/// A struct under its own tag, which writes `named` under it, read from the payloads of
/// [`FLAG_TAGS`] written for it, as `read_as` in Swift and by `serde` in Rust, which writes `back`
/// of every one, its keys sorted as the Swift run writes them.
struct TagRows {
    back: &'static str,
    name: &'static str,
    named: &'static str,
    payloads: [String; 4],
    read_as: &'static str,
    serde: fn(&str) -> String,
}

/// The payload as it was handed.
const fn as_given(payload: Value) -> Value {
    payload
}

/// A struct under its own tag alone, with a type parameter, with a member under its alias, naming
/// a function with a member read under its alias, with a flattened member of its own and flattened
/// ahead of a member of its holder's own.
fn tag_rows() -> [TagRows; 6] {
    let renamed =
        |named: &str| FLAG_TAGS.map(|row| row.replace("\"Flag\"", &format!("\"{named}\"")));
    [
        TagRows {
            back: r#"{"kind":"Flag","label":"n"}"#,
            name: "alone",
            named: "Flag",
            payloads: FLAG_TAGS.map(str::to_owned),
            read_as: "Flag",
            serde: read_of::<Flag>,
        },
        TagRows {
            back: r#"{"kind":"Gonfalon","label":"n"}"#,
            name: "with a type parameter",
            named: "Gonfalon",
            payloads: renamed("Gonfalon"),
            read_as: "Gonfalon<String>",
            serde: read_of::<Gonfalon<String>>,
        },
        TagRows {
            back: r#"{"kind":"Semaphore","label":"n"}"#,
            name: "with a member under its alias",
            named: "Semaphore",
            payloads: renamed("Semaphore"),
            read_as: "Semaphore",
            serde: read_of::<Semaphore>,
        },
        TagRows {
            back: r#"{"kind":"Bunted","label":"n"}"#,
            name: "naming a function",
            named: "Bunted",
            payloads: renamed("Bunted").map(|row| row.replace("\"label\"", "\"name\"")),
            read_as: "Bunted",
            serde: read_of::<Bunted>,
        },
        TagRows {
            back: r#"{"kind":"Pennant","label":"n","note":"a"}"#,
            name: "with a flattened member",
            named: "Pennant",
            payloads: PENNANT_TAGS.map(str::to_owned),
            read_as: "Pennant",
            serde: read_of::<Pennant>,
        },
        TagRows {
            back: r#"{"kind":"Ensign","label":"n","north":2}"#,
            name: "flattened ahead of a member",
            named: "Ensign",
            payloads: DRESSING_TAGS.map(str::to_owned),
            read_as: "Dressing",
            serde: read_of::<Dressing>,
        },
    ]
}

/// The emitted definitions of the structs of [`tag_rows`] and of what they flatten.
fn definitions() -> Vec<String> {
    vec![
        flag_swift::swift_definition(),
        gonfalon_swift::swift_definition(),
        semaphore_swift::swift_definition(),
        bunted_swift::swift_definition(),
        labelling_swift::swift_definition(),
        pennant_swift::swift_definition(),
        ensign_swift::swift_definition(),
        dressing_swift::swift_definition(),
    ]
}

/// The statements that refuse the object a decoder holds where it holds no tag `kind` of the
/// value `named`.
fn tag_held(named: &str) -> String {
    format!(
        "let tagged = try decoder.container(keyedBy: SwiftSchemaTagCodingKeys.self); guard \
         tagged.contains(.swiftSchemaTag) else {{ throw \
         DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, \
         debugDescription: \"missing field `kind`\")) }}; guard (try? tagged.decode(String.self, \
         forKey: .swiftSchemaTag)) == \"{named}\" else {{ let found = try \
         JSONSerialization.data(withJSONObject: tagged.decode(SwiftSchemaPayload.self, forKey: \
         .swiftSchemaTag).value, options: [.fragmentsAllowed, .sortedKeys, \
         .withoutEscapingSlashes]); throw DecodingError.dataCorrupted(DecodingError.Context(\
         codingPath: decoder.codingPath, debugDescription: \"expected tag \\\"{named}\\\", found \
         \\(String(decoding: found, as: UTF8.self))\")) }}; "
    )
}

/// The unjudged read of a struct under its own tag refuses an object that holds no tag, or another
/// value under it, ahead of its members: the payload itself, ahead of the container its aliases
/// are read from where a member has one, and the object a holder hands the struct flattened into
/// it. The holder reads no tag of its own.
#[test]
fn the_swift_read_of_a_struct_under_its_own_tag_checks_the_tag_ahead_of_its_members() {
    let pinned = [
        (
            flag_swift::swift_definition(),
            format!(
                "public init(swiftSchemaUnjudged decoder: Decoder) throws {{ {}let container = try \
                 decoder.container(keyedBy: CodingKeys.self); self.label = try \
                 container.decode(String.self, forKey: .label) }};",
                tag_held("Flag")
            ),
        ),
        (
            gonfalon_swift::swift_definition(),
            format!(
                "public init(swiftSchemaUnjudged decoder: Decoder) throws {{ {}let container = try \
                 decoder.container(keyedBy: CodingKeys.self); self.label = try \
                 container.decode(Held.self, forKey: .label) }};",
                tag_held("Gonfalon")
            ),
        ),
        (
            semaphore_swift::swift_definition(),
            format!(
                "public init(swiftSchemaUnjudged decoder: Decoder) throws {{ {}let aliased = try \
                 decoder.container(keyedBy: SwiftSchemaAnyKey.self);",
                tag_held("Semaphore")
            ),
        ),
        (
            bunted_swift::swift_definition(),
            format!(
                "private init(swiftSchemaCorrected decoder: Decoder) throws {{ {}let aliased = \
                 try decoder.container(keyedBy: SwiftSchemaAnyKey.self);",
                tag_held("Bunted")
            ),
        ),
        (
            pennant_swift::swift_definition(),
            format!(
                "public init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: \
                 [String]) throws {{ {}let container = try decoder.container(keyedBy: \
                 CodingKeys.self);",
                tag_held("Pennant")
            ),
        ),
        (
            ensign_swift::swift_definition(),
            format!(
                "public init(swiftSchemaUnjudged decoder: Decoder) throws {{ {}let container = try \
                 decoder.container(keyedBy: CodingKeys.self); self.label = try \
                 container.decode(String.self, forKey: .label) }};",
                tag_held("Ensign")
            ),
        ),
    ];
    for (swift, read) in pinned {
        assert!(swift.contains(&read), "missing `{read}` in: {swift}");
    }
    let holder = dressing_swift::swift_definition();
    assert!(
        !holder.contains("SwiftSchemaTagCodingKeys"),
        "the holder reads a tag of its own: {holder}"
    );
}

/// Each struct of [`tag_rows`] reads a payload under its own tag as serde reads it, and refuses one
/// with no tag, or with another value under it, `null` among them, as TypeScript and Zod refuse it
/// (`kind: z.literal("Flag")`), where serde reads each and writes the struct's name back.
#[test]
fn a_struct_under_its_own_tag_refuses_in_swift_every_tag_zod_refuses() {
    let mut expected = String::new();
    let mut main = String::new();
    for rows in tag_rows() {
        let name = rows.name;
        assert_eq!(
            rows.payloads
                .each_ref()
                .map(|payload| (rows.serde)(payload)),
            [rows.back; 4],
            "serde reads every payload {name} and writes the struct's name back"
        );
        for (tag, payload) in TAGS.iter().zip(&rows.payloads) {
            let _ = writeln!(
                main,
                "told(\"{name}, {tag}\", {}.self, #\"{payload}\"#)",
                rows.read_as
            );
        }
        let named = rows.named;
        let _ = write!(
            expected,
            "{name}, {}: {}\n\
             {name}, {}: refused: dataCorrupted: missing field `kind`\n\
             {name}, {}: refused: dataCorrupted: expected tag \"{named}\", found \"Other\"\n\
             {name}, {}: refused: dataCorrupted: expected tag \"{named}\", found 5\n",
            TAGS[0], rows.back, TAGS[1], TAGS[2], TAGS[3]
        );
    }
    assert_eq!(
        read_of::<Flag>(NULL_TAG),
        r#"{"kind":"Flag","label":"n"}"#,
        "serde reads a tag of null and writes the struct's name back"
    );
    let _ = writeln!(
        main,
        "told(\"alone, a tag of null\", Flag.self, #\"{NULL_TAG}\"#)"
    );
    expected.push_str(
        "alone, a tag of null: refused: dataCorrupted: expected tag \"Flag\", found null\n",
    );
    let mut parts = vec!["import Foundation".to_owned()];
    parts.extend(definitions());
    parts.extend([AS_GIVEN.to_owned(), TOLD.to_owned(), main]);
    let source = parts.join("\n\n");
    let Some(run) = judged(
        "swift-own-tags",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr \
         ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    assert_eq!(
        run.stdout, expected,
        "the Swift read is not the answer above"
    );
}

/// The README's sentence on the Swift read of a struct under its own tag shows the payloads and
/// the refusals the run above measures.
#[test]
fn the_readme_shows_the_swift_read_of_a_struct_under_its_own_tag() {
    let section = include_str!("../../README.md")
        .split_once("#### A Swift read refuses what `validate()` refuses")
        .and_then(|(_, rest)| rest.split_once("### Literal Values"))
        .map(|(section, _)| section)
        .unwrap();
    for shown in FLAG_TAGS
        .iter()
        .map(|payload| format!("`{payload}`"))
        .chain([
            "``missing field `kind` ``".to_owned(),
            "`expected tag \"Flag\", found \"Other\"`".to_owned(),
            "`expected tag \"Flag\", found 5`".to_owned(),
        ])
    {
        assert!(section.contains(&shown), "not in the README: {shown}");
    }
}
