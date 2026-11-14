//! A Dart model type, held by a second library: built through its draft and `validate()`, never
//! its own constructor, and read, written and refused as its draft is.

#![cfg(feature = "dart")]

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicUsize, Ordering};
use std::env;
use std::fs;
use std::process::{Command, id};

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::run_dart_validate::{dart, module};

static RUNS: AtomicUsize = AtomicUsize::new(0);

const TIER_FUNCTIONS: &str = "List<String> tierKept(CargoTierDraft value) => value == CargoTierDraft.retired ? const ['a tier is kept'] : const [];";

#[model_schema(minLength = 1, maxLength = 80)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct CargoLabel(String);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cargo {
    pub label: CargoLabel,
    #[model_schema_prop(minimum = 1, maximum = 100)]
    pub parcel_count: u32,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CargoPair(
    #[model_schema_prop(minLength = 1)] pub String,
    #[model_schema_prop(minimum = 1)] pub u32,
);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum CargoMove {
    Held,
    Loaded {
        #[model_schema_prop(minimum = 1)]
        parcel_count: u32,
    },
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CargoShelf {
    pub cargo: Cargo,
}

#[model_schema(check = ["tierKept"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum CargoTier {
    Retired,
    Standard,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum CargoSpeed {
    Fast,
    Slow,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CargoTiers {
    pub by_speed: BTreeMap<CargoSpeed, u32>,
    pub tier: CargoTier,
}

fn tier_kept(value: &CargoTierDraft) -> Result<(), Vec<String>> {
    if matches!(value, CargoTierDraft::Retired) {
        return Err(vec!["a tier is kept".to_owned()]);
    }
    Ok(())
}

fn tiers() -> String {
    module(
        &[
            cargo_tier_dart::dart_definition(),
            cargo_speed_dart::dart_definition(),
            cargo_tiers_dart::dart_definition(),
        ],
        &[TIER_FUNCTIONS],
        "",
    )
}

fn models() -> String {
    module(
        &[
            cargo_label_dart::dart_definition(),
            cargo_dart::dart_definition(),
            cargo_pair_dart::dart_definition(),
            cargo_move_dart::dart_definition(),
        ],
        &[],
        "",
    )
}

fn holders() -> String {
    format!(
        "import 'models.dart';\n\n{}",
        module(&[cargo_shelf_dart::dart_definition()], &[], "")
    )
}

/// Runs `dart` over `files`, which sit in one directory, with `arguments` ahead of `entry`.
pub fn dart_files(
    named: &str,
    arguments: &[&str],
    files: &[(&str, String)],
    entry: &str,
) -> Option<(bool, String)> {
    dart("valid-probe", &[], "void main() {}")?;
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

fn draft(parcel_count: u32) -> CargoDraft {
    CargoDraft {
        label: CargoLabel::new("Shipping".to_owned()).unwrap(),
        parcel_count,
    }
}

#[test]
fn every_declared_type_is_constructible() {
    let cargo = draft(3).validate().unwrap();
    assert_eq!(cargo.parcel_count, 3);
    assert_eq!(CargoPairDraft("a".to_owned(), 1).validate().unwrap().1, 1);
    assert!(matches!(&*CargoMove::Held, CargoMoveDraft::Held));
    let shelf = CargoShelfDraft { cargo }.validate().unwrap();
    assert_eq!(shelf.cargo.parcel_count, 3);
}

#[test]
fn a_second_library_cannot_build_a_valid_type() {
    for (named, line) in [
        (
            "plain",
            "Cargo(label: CargoLabel.of('a'), parcel_count: 1);",
        ),
        (
            "const",
            "const Cargo(label: CargoLabel.of('a'), parcel_count: 1);",
        ),
        (
            "private",
            "Cargo._(CargoDraft(label: CargoLabel.of('a'), parcel_count: 1));",
        ),
        ("tuple", "CargoPair('a', 1);"),
        ("tuple-private", "CargoPair._(CargoPairDraft(('a', 1)));"),
        ("enum-private", "CargoMove._(CargoMoveHeld());"),
    ] {
        let user = format!("import 'models.dart';\n\nvoid main() {{ {line} }}\n");
        let Some((clean, said)) = dart_files(
            &format!("valid-{named}"),
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
fn the_analyzer_names_what_a_second_library_cannot_build() {
    let user = "import 'models.dart';\n\nvoid main() {\n  Cargo(label: CargoLabel.of('a'), parcel_count: 1);\n  const Cargo(label: CargoLabel.of('a'), parcel_count: 1);\n  Cargo._(CargoDraft(label: CargoLabel.of('a'), parcel_count: 1));\n}\n";
    let Some((_, said)) = dart_files(
        "valid-named",
        &["analyze"],
        &[("models.dart", models()), ("user.dart", user.to_owned())],
        "user.dart",
    ) else {
        return;
    };
    for printed in [
        "The class 'Cargo' doesn't have an unnamed constructor.",
        "The class 'Cargo' doesn't have an unnamed constant constructor.",
        "The class 'Cargo' doesn't have a constructor named '_'.",
    ] {
        assert!(said.contains(printed), "no `{printed}` in: {said}");
    }
}

#[test]
fn a_draft_validates_into_the_value_its_getters_and_its_write_agree_on() {
    let rust = serde_json::to_string(&draft(3)).unwrap();
    let broken = draft(200).validation_issues().join("; ");
    let read_broken = serde_json::from_str::<Cargo>(r#"{"label":"Shipping","parcelCount":200}"#)
        .unwrap_err()
        .to_string();
    assert!(read_broken.starts_with(&broken), "{read_broken}");
    let user = "import 'dart:convert';\nimport 'holders.dart';\nimport 'models.dart';\n\nString refused(void Function() build) {\n  try {\n    build();\n  } on FormatException catch (e) {\n    return e.message;\n  }\n  return 'built';\n}\n\nvoid main() {\n  final cargo = CargoDraft(label: CargoLabel.of('Shipping'), parcel_count: 3).validate();\n  print(cargo.parcel_count);\n  print(cargo.label.value);\n  print(jsonEncode(cargo.toJson()));\n  print(jsonEncode(cargo.toJson()) == jsonEncode(cargo.toDraft().toJson()));\n  final changed = CargoDraft(label: cargo.label, parcel_count: 200);\n  print(changed.validationIssues().join('; '));\n  print(refused(() => changed.validate()));\n  print(refused(() => Cargo.fromJson(jsonDecode('{\"label\":\"Shipping\",\"parcelCount\":200}'))));\n  print(Cargo.fromJsonUnjudged(jsonDecode('{\"label\":\"Shipping\",\"parcelCount\":200}')).validationIssues().join('; '));\n  print(refused(() => CargoShelf.fromJson(jsonDecode('{\"cargo\":{\"label\":\"Shipping\",\"parcelCount\":200}}'))));\n  print(CargoPairDraft(('a', 1)).validate().value.$2);\n  final held = CargoMove.fromJson(jsonDecode('{\"kind\":\"Held\"}'));\n  print(held.toDraft() is CargoMoveHeld);\n  print(jsonEncode(held.toJson()));\n  print(refused(() => CargoMoveLoaded(parcel_count: 0).validate()));\n}\n";
    let Some((ran, printed)) = dart_files(
        "valid-run",
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
    let loaded = CargoMoveDraft::Loaded { parcel_count: 0 }
        .validation_issues()
        .join("; ");
    assert_eq!(
        printed,
        format!(
            "3\nShipping\n{rust}\ntrue\n{broken}\n{broken}\n{broken}\n{broken}\n'cargo.{}\n1\ntrue\n{}\n{loaded}\n",
            broken.strip_prefix('\'').unwrap(),
            r#"{"kind":"Held"}"#
        )
    );
}

#[test]
fn the_analyzer_passes_a_second_library_that_builds_through_the_draft() {
    let user = "import 'holders.dart';\nimport 'models.dart';\n\nvoid main() {\n  final cargo = CargoDraft(label: CargoLabel.of('Shipping'), parcel_count: 3).validate();\n  final shelf = CargoShelfDraft(cargo: cargo).validate();\n  final Cargo same = shelf.cargo;\n  final CargoDraft back = same.toDraft();\n  Cargo.fromJsonUnjudged(<String, dynamic>{'label': 'a', 'parcelCount': 1});\n  CargoMove.fromJsonUnjudged(<String, dynamic>{'kind': 'Held'});\n  print(back.parcel_count);\n}\n";
    let Some((clean, said)) = dart_files(
        "valid-clean",
        &["analyze"],
        &[
            ("models.dart", models()),
            ("holders.dart", holders()),
            ("user.dart", user.to_owned()),
        ],
        "user.dart",
    ) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}

#[test]
fn a_second_library_cannot_build_a_valid_plain_enum() {
    let user = "import 'tiers.dart';\n\nvoid main() {\n  CargoTier(CargoTierDraft.standard);\n  const CargoTier(CargoTierDraft.standard);\n  CargoTier._(CargoTierDraft.standard);\n  CargoTier.standard;\n}\n";
    let Some((clean, said)) = dart_files(
        "valid-tier",
        &["analyze"],
        &[("tiers.dart", tiers()), ("user.dart", user.to_owned())],
        "user.dart",
    ) else {
        return;
    };
    assert!(!clean, "{said}");
    for printed in [
        "The class 'CargoTier' doesn't have an unnamed constructor.",
        "The class 'CargoTier' doesn't have an unnamed constant constructor.",
        "The class 'CargoTier' doesn't have a constructor named '_'.",
        "The getter 'standard' isn't defined for the type 'CargoTier'.",
    ] {
        assert!(said.contains(printed), "no `{printed}` in: {said}");
    }
}

#[test]
fn a_plain_enum_validates_through_its_draft_and_keeps_its_wire() {
    let refused = CargoTierDraft::Retired.validation_issues().join("; ");
    let rust = serde_json::to_string(&CargoTiersDraft {
        by_speed: BTreeMap::from([(CargoSpeed::Fast, 2), (CargoSpeed::Slow, 1)]),
        tier: CargoTierDraft::Standard.validate().unwrap(),
    })
    .unwrap();
    let user = "import 'dart:convert';\nimport 'tiers.dart';\n\nString refused(void Function() build) {\n  try {\n    build();\n  } on FormatException catch (e) {\n    return e.message;\n  }\n  return 'built';\n}\n\nvoid main() {\n  final standard = CargoTierDraft.standard.validate();\n  print(standard.toDraft() == CargoTierDraft.standard);\n  print(jsonEncode(standard.toJson()));\n  print(refused(() => CargoTierDraft.retired.validate()));\n  print(refused(() => CargoTier.fromJson('Retired')));\n  print(CargoTier.fromJsonUnjudged('Retired').validationIssues().join('; '));\n  print(standard == CargoTier.fromJson('Standard'));\n  print(standard.hashCode == CargoTier.fromJson('Standard').hashCode);\n  print(<CargoTier>{standard, CargoTier.fromJson('Standard')}.length);\n  final held = CargoTiers.fromJson(jsonDecode('{\"bySpeed\":{\"Fast\":2,\"Slow\":1},\"tier\":\"Standard\"}'));\n  print(jsonEncode(held.toJson()));\n  print(held.by_speed[CargoSpeed.fromJson('Fast')]);\n  print(refused(() => CargoTiers.fromJson(jsonDecode('{\"bySpeed\":{},\"tier\":\"Retired\"}'))));\n}\n";
    let Some((ran, printed)) = dart_files(
        "valid-tier-run",
        &[],
        &[("tiers.dart", tiers()), ("user.dart", user.to_owned())],
        "user.dart",
    ) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(
        printed,
        format!(
            "true\n\"Standard\"\n{refused}\n{refused}\n{refused}\ntrue\ntrue\n1\n{rust}\n2\n'tier': {refused}\n"
        )
    );
}
