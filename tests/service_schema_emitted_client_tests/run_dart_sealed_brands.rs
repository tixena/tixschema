//! A sealed Dart brand, held by a second library: built through `of` or a read, never its
//! constructor, and refusing what Rust's `new` refuses with the same messages. A tuple struct and
//! an alias keep their public constructor.

#![cfg(feature = "dart")]

use core::sync::atomic::{AtomicUsize, Ordering};
use std::env;
use std::fs;
use std::process::{Command, id};

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::run_dart_validate::{dart, module};

static RUNS: AtomicUsize = AtomicUsize::new(0);

#[model_schema(minLength = 1, maxLength = 80)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct PennantText(String);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pennant {
    pub fly: PennantFly,
    pub text: PennantText,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PennantCount(pub u32);

#[model_schema()]
pub type PennantNote = String;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PennantFly {
    pub late: PennantLate,
}

#[model_schema(minLength = 2)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct PennantLate(String);

fn models() -> String {
    module(
        &[
            pennant_text_dart::dart_definition(),
            pennant_count_dart::dart_definition(),
            pennant_note_dart::dart_definition(),
            pennant_late_dart::dart_definition(),
        ],
        &[],
        "",
    )
}

fn holders() -> String {
    format!(
        "import 'models.dart';\n\n{}",
        module(
            &[
                pennant_fly_dart::dart_definition(),
                pennant_dart::dart_definition(),
            ],
            &[],
            "",
        )
    )
}

/// Runs `dart` over `files`, which sit in one directory, with `arguments` ahead of `entry`.
fn dart_files(
    named: &str,
    arguments: &[&str],
    files: &[(&str, String)],
    entry: &str,
) -> Option<(bool, String)> {
    dart("sealed-probe", &[], "void main() {}")?;
    let at = env::temp_dir().join(format!(
        "tixschema-emitted-dart-{named}-{}-{}",
        id(),
        RUNS.fetch_add(1, Ordering::Relaxed)
    ));
    if at.exists() {
        fs::remove_dir_all(&at).unwrap();
    }
    fs::create_dir_all(&at).unwrap();
    for (file, text) in files {
        fs::write(at.join(file), text).unwrap();
    }
    let runtime = env::var("TIXSCHEMA_DART").unwrap_or_else(|_| "dart".to_owned());
    let reported = Command::new(runtime)
        .args(arguments)
        .arg(entry)
        .current_dir(&at)
        .output()
        .unwrap();
    fs::remove_dir_all(&at).unwrap();
    Some((
        reported.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&reported.stdout),
            String::from_utf8_lossy(&reported.stderr)
        ),
    ))
}

#[test]
fn every_declared_type_is_constructible() {
    let pennant = PennantDraft {
        text: PennantText::new("a".to_owned()).unwrap(),
        fly: PennantFlyDraft {
            late: PennantLate::new("ab".to_owned()).unwrap(),
        }
        .validate()
        .unwrap(),
    }
    .validate()
    .unwrap();
    assert_eq!(pennant.text.as_inner(), "a");
    assert_eq!(PennantCountDraft(3).validate().unwrap().0, 3);
    let note: PennantNote = "n".to_owned();
    assert_eq!(note, "n");
}

#[test]
fn a_second_library_cannot_build_a_brand() {
    for (named, line) in [
        ("plain", "PennantText('');"),
        ("const", "const PennantText('');"),
        ("private", "PennantText._('');"),
    ] {
        let user = format!("import 'models.dart';\n\nvoid main() {{ {line} }}\n");
        let Some((clean, said)) = dart_files(
            &format!("sealed-{named}"),
            &["analyze"],
            &[("models.dart", models()), ("user.dart", user)],
            "user.dart",
        ) else {
            return;
        };
        assert!(!clean, "{named}: {said}");
        assert!(said.contains("user.dart"), "{named}: {said}");
    }
}

#[test]
fn a_second_library_builds_a_tuple_struct_and_an_alias_and_reads_them() {
    let user = "import 'models.dart';\n\nvoid main() {\n  const PennantCountDraft(3);\n  PennantCount.fromJsonUnjudged(3);\n  PennantNote.fromJsonUnjudged('a');\n  PennantNote('a');\n}\n";
    let Some((clean, said)) = dart_files(
        "sealed-public",
        &["analyze"],
        &[("models.dart", models()), ("user.dart", user.to_owned())],
        "user.dart",
    ) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}

#[test]
fn of_and_from_json_refuse_what_rust_refuses_with_its_messages() {
    let rust = PennantText::new(String::new()).unwrap_err().join("; ");
    let late = PennantLate::new("a".to_owned()).unwrap_err().join("; ");
    let user = "import 'dart:convert';\nimport 'holders.dart';\nimport 'models.dart';\n\nString refused(void Function() build) {\n  try {\n    build();\n  } on FormatException catch (e) {\n    return e.message;\n  }\n  return 'built';\n}\n\nvoid main() {\n  print(refused(() => PennantText.of('')));\n  print(PennantText.of('Shipping').value);\n  print(refused(() => Pennant.fromJson(jsonDecode('{\"text\":\"\",\"fly\":{\"late\":\"ab\"}}'))));\n  print(refused(() => Pennant.fromJson(jsonDecode('{\"text\":\"x\",\"fly\":{\"late\":\"a\"}}'))));\n  print(jsonEncode(Pennant.fromJson(jsonDecode('{\"text\":\"x\",\"fly\":{\"late\":\"ab\"}}'))));\n}\n";
    let Some((ran, printed)) = dart_files(
        "sealed-run",
        &[],
        &[
            ("models.dart", models()),
            ("holders.dart", holders()),
            ("user.dart", user.to_owned()),
        ],
        "user.dart",
    ) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(
        printed,
        format!(
            "{rust}\nShipping\n'text': {rust}\n'fly.late': {late}\n{}\n",
            r#"{"fly":{"late":"ab"},"text":"x"}"#
        )
    );
}
