//! A plain enum flattened into a holder declared above it, under the Dart analyzer. The holder
//! reads the member through the enum's `fromJsonUnjudged`, which takes a `String`, from the object
//! the holder is handed, so the analyzer refuses the library.

#![cfg(feature = "dart")]

use super::plain_enum_below_models::{casement_dart, weathering_dart};
use super::run_dart_validate::{dart, module};

/// The analyzer refuses the holder's read of the enum, as the README says.
#[test]
fn the_dart_analyzer_refuses_a_plain_enum_flattened_below_its_holder() {
    let source = module(
        &[
            casement_dart::dart_definition(),
            weathering_dart::dart_definition(),
        ],
        &[],
        "void main() { print(jsonEncode(Casement.fromJson(jsonDecode(r'{}')))); }",
    );
    let Some((clean, said)) = dart("plain-enum-below-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(!clean, "{said}");
    assert!(
        said.contains(
            "error - module.dart:3:680 - The argument type 'Map<String, dynamic>' can't be \
             assigned to the parameter type 'String'.  - argument_type_not_assignable"
        ),
        "{said}"
    );
    assert!(said.contains("1 issue found."), "{said}");
}
