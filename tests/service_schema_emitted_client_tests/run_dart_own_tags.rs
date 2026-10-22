//! A struct under its own tag, alone, with a type parameter, with a member under its alias, with a
//! flattened member of its own and flattened ahead of a member of its holder's own, read by the
//! emitted Dart types under the Dart VM: a payload whose tag is the struct's name is read and
//! written back as serde writes it back, and one with no tag, or with another value under it, is
//! refused with `FormatException`, as TypeScript and Zod refuse it, where serde reads it. The
//! models are those the Kotlin runs are held to, compiled with the `kotlin` feature.

#![cfg(all(feature = "dart", feature = "kotlin", feature = "serde"))]

use core::fmt::Write as _;

use super::flattened_models::{
    DRESSING_TAGS, Dressing, FLAG_TAGS, Flag, Gonfalon, PENNANT_TAGS, Pennant, Semaphore,
    dressing_dart, ensign_dart, flag_dart, gonfalon_dart, pennant_dart, read_of, semaphore_dart,
};
use super::run_dart_validate::{dart, from_until, module};
use super::tuple_variants_models::labelling_dart;

/// What prints a row: the value read, written back, or the refusal of the read.
const TOLD: &str = "
void told(String name, String payload, String Function(dynamic) read) {
  String printed;
  try {
    printed = read(jsonDecode(payload));
  } catch (refusal) {
    printed = 'refused: $refusal';
  }
  print('$name: $printed');
}
";

/// What each payload of a [`TagRows`] holds under the tag.
const TAGS: [&str; 4] = ["its tag", "no tag", "another tag", "a tag of another kind"];

/// [`Flag`] with `null` under its tag, which serde reads and Zod refuses.
const NULL_TAG: &str = r#"{"kind":null,"label":"n"}"#;

/// A struct under its own tag, which writes `named` under it, read from the payloads of
/// [`FLAG_TAGS`] written for it, by `read` in Dart and by `serde` in Rust, which writes `back` of
/// every one.
struct TagRows {
    back: &'static str,
    name: &'static str,
    named: &'static str,
    payloads: [String; 4],
    read: &'static str,
    serde: fn(&str) -> String,
}

/// A struct under its own tag alone, with a type parameter, with a member under its alias, with a
/// flattened member of its own and flattened ahead of a member of its holder's own.
fn tag_rows() -> [TagRows; 5] {
    let renamed =
        |named: &str| FLAG_TAGS.map(|row| row.replace("\"Flag\"", &format!("\"{named}\"")));
    [
        TagRows {
            back: r#"{"kind":"Flag","label":"n"}"#,
            name: "alone",
            named: "Flag",
            payloads: FLAG_TAGS.map(str::to_owned),
            read: "(json) => jsonEncode(Flag.fromJson(json).toJson())",
            serde: read_of::<Flag>,
        },
        TagRows {
            back: r#"{"kind":"Gonfalon","label":"n"}"#,
            name: "with a type parameter",
            named: "Gonfalon",
            payloads: renamed("Gonfalon"),
            read: "(json) => jsonEncode(Gonfalon<String>.fromJson(json, (held) => held as \
                   String).toJson((held) => held))",
            serde: read_of::<Gonfalon<String>>,
        },
        TagRows {
            back: r#"{"kind":"Semaphore","label":"n"}"#,
            name: "with a member under its alias",
            named: "Semaphore",
            payloads: renamed("Semaphore"),
            read: "(json) => jsonEncode(Semaphore.fromJson(json).toJson())",
            serde: read_of::<Semaphore>,
        },
        TagRows {
            back: r#"{"kind":"Pennant","label":"n","note":"a"}"#,
            name: "with a flattened member",
            named: "Pennant",
            payloads: PENNANT_TAGS.map(str::to_owned),
            read: "(json) => jsonEncode(Pennant.fromJson(json).toJson())",
            serde: read_of::<Pennant>,
        },
        TagRows {
            back: r#"{"kind":"Ensign","label":"n","north":2}"#,
            name: "flattened ahead of a member",
            named: "Ensign",
            payloads: DRESSING_TAGS.map(str::to_owned),
            read: "(json) => jsonEncode(Dressing.fromJson(json).toJson())",
            serde: read_of::<Dressing>,
        },
    ]
}

/// The emitted definitions of the structs of [`tag_rows`] and of what they flatten.
fn definitions() -> Vec<String> {
    vec![
        flag_dart::dart_definition(),
        gonfalon_dart::dart_definition(),
        semaphore_dart::dart_definition(),
        labelling_dart::dart_definition(),
        pennant_dart::dart_definition(),
        ensign_dart::dart_definition(),
        dressing_dart::dart_definition(),
    ]
}

/// The statements that refuse `object` where it holds no tag `kind` of the value `named`.
fn tag_held(object: &str, named: &str) -> String {
    format!(
        "if (!{object}.containsKey('kind')) throw FormatException('missing field `kind`'); if \
         ({object}['kind'] != '{named}') throw FormatException('expected tag \"{named}\", found ' \
         + jsonEncode({object}['kind']));"
    )
}

/// The unjudged read of a struct under its own tag refuses an object that holds no tag, or another
/// value under it, ahead of its members: the payload itself, the copy its aliases are read from
/// where a member has one, and the object a holder hands the struct flattened into it.
#[test]
fn the_dart_read_of_a_struct_under_its_own_tag_checks_the_tag_ahead_of_its_members() {
    let pinned = [
        (
            flag_dart::dart_definition(),
            format!(
                "factory Flag.fromJsonUnjudged(Map<String, dynamic> json) {{ {} return \
                 Flag(label: json['label'] as String,); }}",
                tag_held("json", "Flag")
            ),
        ),
        (
            gonfalon_dart::dart_definition(),
            format!(
                "factory Gonfalon.fromJsonUnjudged(Map<String, dynamic> json, Held Function(dynamic) \
                 heldFromJson) {{ {} return Gonfalon(label: heldFromJson(json['label']),); }}",
                tag_held("json", "Gonfalon")
            ),
        ),
        (
            semaphore_dart::dart_definition(),
            format!(
                "throw FormatException('duplicate field `$key`'); named[key] = entry.value; }} {} \
                 return Semaphore(label: named['label'] as String,); }}",
                tag_held("named", "Semaphore")
            ),
        ),
        (
            pennant_dart::dart_definition(),
            format!(
                "factory Pennant.fromJsonUnjudged(Map<String, dynamic> json) {{ {} return \
                 Pennant(labelling: Labelling.fromJsonUnjudged(json),note: json['note'] as \
                 String,); }}",
                tag_held("json", "Pennant")
            ),
        ),
        (
            dressing_dart::dart_definition(),
            "Dressing(ensign: Ensign.fromJsonUnjudged(json),north: (switch (json['north'] as \
             int) { final number when number >= 0 && number <= 255 => number, final number => \
             throw FormatException('expected an integer from 0 to 255, found $number') }),)"
                .to_owned(),
        ),
        (
            ensign_dart::dart_definition(),
            format!(
                "factory Ensign.fromJsonUnjudged(Map<String, dynamic> json) {{ {} return \
                 Ensign(label: json['label'] as String,); }}",
                tag_held("json", "Ensign")
            ),
        ),
    ];
    for (dart, read) in pinned {
        assert!(dart.contains(&read), "missing `{read}` in: {dart}");
    }
}

/// Each struct of [`tag_rows`] reads a payload under its own tag as serde reads it, and refuses one
/// with no tag, or with another value under it, `null` among them, as TypeScript and Zod refuse it
/// (`kind: z.literal("Flag")`), where serde reads each and writes the struct's name back.
#[test]
fn a_struct_under_its_own_tag_refuses_in_dart_every_tag_zod_refuses() {
    let mut expected = String::new();
    let mut main = String::from("void main() {\n");
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
                "  told('{name}, {tag}', r'{payload}', {});",
                rows.read
            );
        }
        let named = rows.named;
        let _ = write!(
            expected,
            "{name}, {}: {}\n\
             {name}, {}: refused: FormatException: missing field `kind`\n\
             {name}, {}: refused: FormatException: expected tag \"{named}\", found \"Other\"\n\
             {name}, {}: refused: FormatException: expected tag \"{named}\", found 5\n",
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
        "  told('alone, a tag of null', r'{NULL_TAG}', {});",
        tag_rows()[0].read
    );
    expected.push_str(
        "alone, a tag of null: refused: FormatException: expected tag \"Flag\", found null\n",
    );
    main.push_str("}\n");
    let source = module(&definitions(), &[TOLD], &main);
    let Some((ran, printed)) = dart("own-tags", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not the answer above");
}

/// The README's sentence on the Dart read of a struct under its own tag shows the payloads and
/// the refusals the run above measures.
#[test]
fn the_readme_shows_the_dart_read_of_a_struct_under_its_own_tag() {
    let section = from_until(
        include_str!("../../README.md"),
        "#### The Dart read refuses what `validate()` refuses",
        "#### What stays yours",
    );
    for shown in FLAG_TAGS
        .iter()
        .map(|payload| format!("`{payload}`"))
        .chain([
            "``FormatException: missing field `kind` ``".to_owned(),
            "`FormatException: expected tag \"Flag\", found \"Other\"`".to_owned(),
            "`FormatException: expected tag \"Flag\", found 5`".to_owned(),
        ])
    {
        assert!(section.contains(&shown), "not in the README: {shown}");
    }
}

/// The analyzer passes the structs under their own tag that flatten nothing. `Pennant` and
/// `Dressing` are run above alone: the `toJson` of each casts a value whose type the analyzer
/// already knows.
#[test]
fn the_emitted_structs_under_their_own_tag_have_no_issue() {
    let source = module(
        &[
            flag_dart::dart_definition(),
            gonfalon_dart::dart_definition(),
            semaphore_dart::dart_definition(),
            ensign_dart::dart_definition(),
        ],
        &[],
        "void main() => print(jsonEncode(Flag(label: 'n').toJson()));\n",
    );
    let Some((clean, said)) = dart("own-tags-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
