//! A struct whose every member is flattened, and an enum whose variant holds only a flattened
//! member, read and written back by the emitted Dart types, which the analyzer passes with
//! warnings as errors.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::all_flattened_models::{
    REFUSED, ROWS, assert_printed_as_serde_writes, berth_dart, gimbal_dart, hoist_dart,
    keelson_dart, lantern_dart, mast_dart, rigged_dart, rigging_dart, rigs_dart,
};
use super::run_dart_validate::{dart, module};

/// The emitted definitions of every model of the table.
fn definitions() -> Vec<String> {
    vec![
        lantern_dart::dart_definition(),
        mast_dart::dart_definition(),
        berth_dart::dart_definition(),
        keelson_dart::dart_definition(),
        gimbal_dart::dart_definition(),
        rigging_dart::dart_definition(),
        rigged_dart::dart_definition(),
        rigs_dart::dart_definition(),
        hoist_dart::dart_definition(),
    ]
}

/// What prints each row read and written back, then whether each refused payload is refused.
fn main_of_the_rows() -> String {
    let mut main = String::from(
        "bool refused(void Function() read) {\n  try {\n    read();\n    return false;\n  } catch \
         (_) {\n    return true;\n  }\n}\n\nvoid main() {\n",
    );
    for (name, payload) in ROWS {
        let _ = writeln!(
            main,
            "  print('{name}: ${{jsonEncode({name}.fromJson(jsonDecode(r'{payload}')).toJson())}}');"
        );
    }
    for (name, payload) in REFUSED {
        let _ = writeln!(
            main,
            "  print('{name} refused: ${{refused(() => {name}.fromJson(jsonDecode(r'{payload}')))}}');"
        );
    }
    main.push_str("}\n");
    main
}

/// Each model reads its payload and writes back what serde writes, and refuses a payload serde
/// refuses.
#[test]
fn a_model_of_flattened_members_alone_is_read_and_written_back_in_dart() {
    let source = module(&definitions(), &[], &main_of_the_rows());
    let Some((ran, printed)) = dart("all-flattened", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_printed_as_serde_writes(&printed);
}

/// The analyzer passes the models, warnings included.
#[test]
fn the_emitted_models_of_flattened_members_alone_have_no_issue() {
    let source = module(&definitions(), &[], &main_of_the_rows());
    let Some((clean, said)) = dart(
        "all-flattened-analyze",
        &["analyze", "--fatal-warnings"],
        &source,
    ) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
