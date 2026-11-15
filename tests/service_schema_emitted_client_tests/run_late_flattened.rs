//! A flattened member read and written back by the emitted Dart and Swift types, whose type is
//! declared above its holder, below it and in another crate: the three layouts answer every
//! payload with the same JSON and the same message. A struct beside a struct, an externally
//! tagged enum, one with a rename and an alias, and an `Option` of a struct and of an enum are
//! held by a strict holder and by a `nonstrict` one.

#![cfg(all(feature = "dart", feature = "swift"))]

use core::fmt::Write as _;

use super::late_flattened_models::{LAYOUTS, dart_definitions, serde_written, swift_definitions};
use super::run_dart_validate::{analyzed, dart, module};
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const SWIFT_VAR: &str = "TIXSCHEMA_SWIFT";

/// Each holder, and the payloads it is read from.
const ROWS: [(&str, &[&str]); 7] = [
    (
        "LateStructs",
        &[
            r#"{"width":1,"strand":"s","turns":2,"tail":"t"}"#,
            r#"{"width":1,"strand":"s","tail":"t"}"#,
            r#"{"width":1,"strand":"s","turns":"x","tail":"t"}"#,
            r#"{"width":1,"strand":"s","turns":2}"#,
            r#"{"width":1,"strand":"s","turns":2,"tail":"t","extra":1}"#,
        ],
    ),
    (
        "LateEnum",
        &[
            r#"{"width":1,"Bowline":{"loops":2}}"#,
            r#"{"width":1,"Bowline":{"loops":2},"Sheet":{"label":"l"}}"#,
            r#"{"width":1,"Bowline":{"loops":"x"}}"#,
            r#"{"width":1}"#,
            r#"{"width":1,"Bowline":{"loops":2},"extra":1}"#,
            r#"{"width":1,"Bowline":{"loops":2,"extra":1}}"#,
        ],
    ),
    (
        "LateNamed",
        &[
            r#"{"width":1,"Bowline":{"loops":2}}"#,
            r#"{"width":1,"sheet":{"label":"l"}}"#,
            r#"{"width":1,"Sheet":{"label":"l"}}"#,
            r#"{"width":1,"Hitch":{"turns":2}}"#,
            r#"{"width":1,"hitch":{"turns":2}}"#,
            r#"{"width":1,"Bowline":{"loops":2},"hitch":{"turns":2}}"#,
            r#"{"width":1}"#,
            r#"{"width":1,"Bowline":{"loops":2},"extra":1}"#,
        ],
    ),
    (
        "LateOption",
        &[
            r#"{"note":"n","strand":"s","turns":2}"#,
            r#"{"note":"n"}"#,
            r#"{"note":"n","strand":"s"}"#,
            r#"{"note":"n","strand":"s","turns":"x"}"#,
            r#"{"note":"n","extra":1}"#,
        ],
    ),
    (
        "LateOptionEnum",
        &[
            r#"{"note":"n"}"#,
            r#"{"note":"n","Bowline":{"loops":2}}"#,
            r#"{"note":"n","sheet":{"label":"l"}}"#,
            r#"{"note":"n","Bowline":{"loops":"x"}}"#,
            r#"{"note":"n","Bowline":{"loops":2},"sheet":{"label":"l"}}"#,
            r#"{"note":"n","extra":1}"#,
        ],
    ),
    (
        "LateLoose",
        &[
            r#"{"note":"n","strand":"s","turns":2,"extra":1}"#,
            r#"{"note":"n","strand":"s"}"#,
            r#"{"note":"n","extra":1}"#,
            r#"{"note":"n","strand":"s","turns":"x"}"#,
        ],
    ),
    (
        "LateLooseEnum",
        &[
            r#"{"note":"n","Bowline":{"loops":2},"extra":1}"#,
            r#"{"note":"n","Bowline":{"loops":2}}"#,
            r#"{"note":"n"}"#,
        ],
    ),
];

/// What the Dart read of each of the rows prints for `layout`.
fn dart_main(layout: &str) -> String {
    let mut main = String::from(
        "String told(String Function() read) {\n  try {\n    return read();\n  } on FormatException \
         catch (refused) {\n    return 'refused: ${refused.message}';\n  } catch (refused) {\n    \
         return 'error: $refused';\n  }\n}\n\nvoid main() {\n",
    );
    for (holder, payloads) in ROWS {
        for payload in payloads {
            writeln!(
                main,
                "  print('{holder} {payload} => ' + told(() => \
                 jsonEncode({holder}{layout}Draft.fromJson(jsonDecode('{payload}') as Map<String, \
                 dynamic>).toJson())));"
            )
            .unwrap();
        }
    }
    main.push_str("}\n");
    main
}

/// What the Swift read of each of the rows prints for `layout`.
fn swift_main(layout: &str) -> String {
    let mut main = String::from(
        "func told<Read: Codable>(_ type: Read.Type, _ payload: String) -> String {\n  do {\n    \
         let read = try JSONDecoder().decode(type, from: Data(payload.utf8))\n    let encoder = \
         JSONEncoder()\n    encoder.outputFormatting = [.sortedKeys]\n    return String(data: try \
         encoder.encode(read), encoding: .utf8)!\n  } catch DecodingError.dataCorrupted(let \
         context) {\n    return \"refused: \" + context.debugDescription\n  } catch {\n    return \
         \"error: \\(error)\"\n  }\n}\n\n",
    );
    for (holder, payloads) in ROWS {
        for payload in payloads {
            let literal = payload.replace('"', "\\\"");
            writeln!(
                main,
                "print(\"{holder} {literal} => \" + told({holder}{layout}.self, \"{literal}\"))"
            )
            .unwrap();
        }
    }
    main
}

/// What `layout` prints under Dart, with the name of the layout taken from every type, or `None`
/// where no runtime was reachable.
fn dart_printed(layout: &str) -> Option<String> {
    let source = module(&dart_definitions(layout), &[], &dart_main(layout));
    analyzed(&format!("late-{layout}"), &source);
    let (ran, said) = dart(&format!("late-{layout}"), &["run"], &source)?;
    assert!(ran, "{said}\n--- source ---\n{source}");
    Some(said.replace(layout, ""))
}

/// What `layout` prints under Swift, with the name of the layout taken from every type, or `None`
/// where no runtime was reachable.
fn swift_printed(layout: &str) -> Option<String> {
    let source = [
        "import Foundation".to_owned(),
        swift_definitions(layout).join("\n\n"),
        swift_main(layout),
    ]
    .join("\n\n");
    let run = judged(
        &format!("late-swift-{layout}"),
        SWIFT_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    )?;
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    Some(run.stdout.replace(layout, ""))
}

/// Asserts that every layout printed what the first did, and that the first held `rows`.
fn assert_alike(printed: &[(&str, String)], rows: &[&str]) {
    let (first, expected) = &printed[0];
    for row in rows {
        assert!(
            expected.lines().any(|line| line == *row),
            "{first} printed no `{row}`:\n{expected}"
        );
    }
    for (layout, told) in &printed[1..] {
        assert_eq!(told, expected, "{layout} printed what {first} did not");
    }
}

#[test]
fn a_flattened_member_is_read_alike_by_dart_in_every_layout() {
    let printed: Vec<(&str, String)> = LAYOUTS
        .iter()
        .map_while(|&layout| dart_printed(layout).map(|told| (layout, told)))
        .collect();
    if printed.len() != LAYOUTS.len() {
        return;
    }
    assert_alike(
        &printed,
        &[
            r#"LateStructs {"width":1,"strand":"s","turns":2,"tail":"t"} => {"strand":"s","turns":2,"tail":"t","width":1}"#,
            r#"LateEnum {"width":1,"Bowline":{"loops":2},"Sheet":{"label":"l"}} => refused: more than one variant of enum LateKnot found in flattened data: Bowline, Sheet"#,
            r#"LateNamed {"width":1,"hitch":{"turns":2}} => {"Hitch":{"turns":2},"width":1}"#,
            r#"LateOption {"note":"n","strand":"s"} => refused: missing field `turns`"#,
            r#"LateOption {"note":"n","strand":"s","turns":"x"} => refused: type 'String' is not a subtype of type 'int' in type cast"#,
            r#"LateOptionEnum {"note":"n"} => {"note":"n"}"#,
            r#"LateLoose {"note":"n","strand":"s"} => {"note":"n"}"#,
        ],
    );
}

#[test]
fn a_flattened_member_is_read_alike_by_swift_in_every_layout() {
    let printed: Vec<(&str, String)> = LAYOUTS
        .iter()
        .map_while(|&layout| swift_printed(layout).map(|told| (layout, told)))
        .collect();
    if printed.len() != LAYOUTS.len() {
        return;
    }
    assert_alike(
        &printed,
        &[
            r#"LateStructs {"width":1,"strand":"s","turns":2,"tail":"t"} => {"strand":"s","tail":"t","turns":2,"width":1}"#,
            r#"LateEnum {"width":1,"Bowline":{"loops":2},"Sheet":{"label":"l"}} => refused: more than one variant of enum LateKnot found in flattened data: Bowline, Sheet"#,
            r#"LateEnum {"width":1} => refused: no variant of enum LateKnot found in flattened data"#,
            r#"LateNamed {"width":1,"hitch":{"turns":2}} => {"Hitch":{"turns":2},"width":1}"#,
            r#"LateOption {"note":"n"} => {"note":"n"}"#,
            r#"LateOptionEnum {"note":"n","Bowline":{"loops":2},"sheet":{"label":"l"}} => refused: more than one variant of enum LateHitch found in flattened data: Bowline, sheet"#,
            r#"LateLoose {"note":"n","strand":"s"} => {"note":"n"}"#,
        ],
    );
}

#[test]
fn serde_reads_a_payload_and_writes_it_back_in_every_layout() {
    let rows = [
        (
            "LateStructs",
            r#"{"width":1,"strand":"s","turns":2,"tail":"t"}"#,
        ),
        ("LateEnum", r#"{"width":1,"Bowline":{"loops":2}}"#),
        ("LateNamed", r#"{"width":1,"sheet":{"label":"l"}}"#),
        ("LateOption", r#"{"note":"n","strand":"s","turns":2}"#),
        ("LateOptionEnum", r#"{"note":"n","sheet":{"label":"l"}}"#),
        ("LateLoose", r#"{"note":"n"}"#),
        ("LateLooseEnum", r#"{"note":"n","Bowline":{"loops":2}}"#),
    ];
    for (holder, payload) in rows {
        let expected: serde_json::Value = serde_json::from_str(payload).unwrap();
        for layout in LAYOUTS {
            assert_eq!(
                serde_written(holder, layout, payload),
                Some(expected.clone()),
                "{holder}{layout} did not write back {payload}"
            );
        }
    }
}
