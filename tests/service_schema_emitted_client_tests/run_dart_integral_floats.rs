//! A number written with a fraction or an exponent, `-0.0` among them, read for a fixed-width integer
//! by the emitted Dart types under the Dart VM, as a member, a list's item, a map's value and under
//! an `Option`: each is refused in serde's words, naming the float as `serde_json` spells it and the
//! integer type, and every other refusal keeps its words. The analyzer passes every type.

#![cfg(all(feature = "dart", feature = "serde"))]

use core::fmt::Write as _;

use super::integral_float_models::{Fathometer, fathometer_dart};
use super::run_dart_validate::{analyzed, dart, module};

/// The numbers a payload is written with: floats spelled plainly and with an exponent, either side
/// of the bounds where `serde_json` changes its spelling, and numbers no float holds.
const NUMBERS: [&str; 22] = [
    "-0.0",
    "0.0",
    "1.0",
    "1e2",
    "1E2",
    "1.5",
    "-2.25",
    "0.1",
    "1e15",
    "1e16",
    "1e17",
    "1e-5",
    "1e-6",
    "1e-7",
    "-1e-7",
    "123456789.123456789",
    "123456789012345678901234567890",
    "18446744073709551616",
    "1.7976931348623157e308",
    "5e-324",
    "1e999",
    "-1e22",
];

/// Each place [`Fathometer`] holds an integer: the member that holds it, and what the member holds
/// with `{}` in the integer's place.
const PLACES: [(&str, &str); 4] = [
    ("depth", "{}"),
    ("fathoms", "[7,{}]"),
    ("marks", r#"{"a":7,"b":{}}"#),
    ("tide", "{}"),
];

/// What a row prints: its name, then the words its read is refused with, or that it was read.
const REPORTER: &str = "
void row(String name, String payload) {
  try {
    Fathometer.fromJson(jsonDecode(payload) as Map<String, dynamic>);
    print('$name: read');
  } on FormatException catch (refused) {
    print('$name: ${refused.message}');
  }
}
";

/// The payload of a [`Fathometer`] that holds `7` in every place but `member`, which holds
/// `number` in its place.
fn sounded(member: &str, place: &str, number: &str) -> String {
    let members: Vec<String> = [
        ("depth", "7"),
        ("fathoms", "[7]"),
        ("marks", r#"{"a":7}"#),
        ("tide", "7"),
    ]
    .into_iter()
    .map(|(named, held)| {
        if named == member {
            format!("\"{named}\":{}", place.replace("{}", number))
        } else {
            format!("\"{named}\":{held}")
        }
    })
    .collect();
    format!("{{{}}}", members.join(","))
}

/// What serde answers of `payload`: that it reads it, or the words it refuses it with.
fn serde_answer(name: &str, payload: &str) -> String {
    match serde_json::from_str::<Fathometer>(payload) {
        Ok(_) => format!("{name}: read"),
        Err(refused) => {
            let said = refused.to_string();
            let words = said.split(" at line ").next().unwrap();
            format!("{name}: {words}")
        }
    }
}

#[test]
fn a_number_written_as_a_float_is_refused_for_an_integer_in_the_words_of_serde_in_dart() {
    let mut rows = Vec::new();
    for number in NUMBERS {
        for (member, place) in PLACES {
            rows.push((
                format!("{number} in {member}"),
                sounded(member, place, number),
            ));
        }
    }
    rows.push(("an integer in each place".to_owned(), sounded("", "", "")));
    let mut main = String::from("void main() {\n");
    let mut expected = String::new();
    for (name, payload) in &rows {
        let _ = writeln!(main, "  row('{name}', r'{payload}');");
        let _ = writeln!(expected, "{}", serde_answer(name, payload));
    }
    main.push_str("}\n");
    let source = module(&[fathometer_dart::dart_definition()], &[REPORTER], &main);
    let Some((ran, printed)) = dart("integral-floats", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected);
}

#[test]
fn the_emitted_text_of_the_integral_float_models_passes_the_analyzer() {
    let source = module(
        &[fathometer_dart::dart_definition()],
        &[REPORTER],
        "void main() => row('none', '{}');\n",
    );
    analyzed("integral-floats-analyze", &source);
}
