//! A name serde reads beside the one it writes, read by the emitted Dart types under the Dart VM:
//! a variant under its `alias`, under each tagging, flattened and in a plain enum, and a member
//! under its `alias`, in a struct, flattened and in a variant. Each payload is written back as
//! serde writes it back, under the name serde writes, and a member read under two of its names is
//! refused, as serde refuses it.

#![cfg(all(feature = "dart", feature = "serde"))]

use core::fmt::Write as _;

use super::aliased_models::{
    CAREENING, Careening, FAIRLEADING, Fairleading, Fothering, Gybing, HAWSING, Hawsing, LULLING,
    Lulling, SCENDING, SURGING, SWELL, Scending, Surging, Swell, TIDING, Tiding, VEERING, Veering,
    careening_dart, fairleading_dart, fothering_dart, gybing_dart, hawsing_dart, lulling_dart,
    pennon_dart, scending_dart, surging_dart, swell_dart, tiding_dart, veering_dart, written_back,
};
use super::dart_flattened_aliased_models::{
    DRIFTING, Drifting, RIDING, Riding, SWINGING, Swinging, drifting_dart, riding_dart,
    swinging_dart,
};
use super::run_dart_validate::{dart, module};

/// What prints a row: the value read, written back, or that the read is refused.
const TOLD: &str = "
void row(String name, String payload, String Function(dynamic) told) {
  String printed;
  try {
    printed = told(jsonDecode(payload));
  } catch (_) {
    printed = 'refused';
  }
  print('$name $payload: $printed');
}
";

/// The functions the models name.
const FUNCTIONS: [&str; 2] = [
    "dynamic unpadded(dynamic payload) => payload is String ? payload.trim() : payload;",
    "dynamic withItsKeys(dynamic payload) => \
     {'keys': (payload as Map<String, dynamic>).keys.toList()};",
];

/// Each payload with the name it is printed under, the Dart that reads it and writes it back, and
/// what serde writes back of it.
fn rows() -> Vec<(&'static str, &'static str, &'static str, String)> {
    let mut rows = Vec::new();
    let mut of = |named: &'static str,
                  told: &'static str,
                  payloads: &[&'static str],
                  serde: fn(&str) -> String| {
        rows.extend(
            payloads
                .iter()
                .map(|payload| (named, told, *payload, serde(payload))),
        );
    };
    let plain = "jsonEncode(TYPE.fromJson(json).toJson())";
    of("Surging", plain, &SURGING, written_back::<Surging>);
    of("Careening", plain, &CAREENING, written_back::<Careening>);
    of("Scending", plain, &SCENDING, written_back::<Scending>);
    of("Swell", plain, &SWELL, written_back::<Swell>);
    of("Lulling", plain, &LULLING, written_back::<Lulling>);
    of("Tiding", plain, &[TIDING], written_back::<Tiding>);
    of("Veering", plain, &VEERING, written_back::<Veering>);
    of("Hawsing", plain, &HAWSING, written_back::<Hawsing>);
    of(
        "Gybing<String>",
        "jsonEncode(Gybing<String>.fromJson(json, (e) => e as String).toJson((e) => e))",
        &HAWSING,
        written_back::<Gybing<String>>,
    );
    of(
        "Fairleading",
        plain,
        &FAIRLEADING,
        written_back::<Fairleading>,
    );
    of("Fothering", plain, &FAIRLEADING, written_back::<Fothering>);
    of("Riding", plain, &RIDING, written_back::<Riding>);
    of("Swinging", plain, &SWINGING, written_back::<Swinging>);
    of("Drifting", plain, &DRIFTING, written_back::<Drifting>);
    rows
}

/// Each payload is read under each name serde reads it under, a unit variant's entry over `null`
/// among them, written back under the name serde writes, and refused where serde refuses it. A
/// member under two of its names is refused with the words serde refuses it with.
#[test]
fn a_name_serde_reads_beside_the_one_it_writes_is_read_in_dart() {
    let rows = rows();
    let mut expected = String::new();
    let mut main = String::from("void main() {\n");
    for (named, told, payload, serde) in &rows {
        let _ = writeln!(expected, "{named} {payload}: {serde}");
        let _ = writeln!(
            main,
            "  row('{named}', r'{payload}', (json) => {});",
            told.replace("TYPE", named)
        );
    }
    // serde refuses a member under two of its names with `duplicate field`, and so does Dart.
    let _ = writeln!(
        main,
        "  try {{ Hawsing.fromJson(jsonDecode(r'{}')); }} catch (refused) {{ print(refused); }}",
        HAWSING[3]
    );
    expected.push_str("FormatException: duplicate field `label`\n");
    main.push_str("}\n");
    let source = module(
        &[
            pennon_dart::dart_definition(),
            surging_dart::dart_definition(),
            careening_dart::dart_definition(),
            scending_dart::dart_definition(),
            swell_dart::dart_definition(),
            lulling_dart::dart_definition(),
            tiding_dart::dart_definition(),
            veering_dart::dart_definition(),
            hawsing_dart::dart_definition(),
            gybing_dart::dart_definition(),
            fairleading_dart::dart_definition(),
            fothering_dart::dart_definition(),
            riding_dart::dart_definition(),
            swinging_dart::dart_definition(),
            drifting_dart::dart_definition(),
        ],
        &[FUNCTIONS[0], FUNCTIONS[1], TOLD],
        &main,
    );
    let Some((ran, printed)) = dart("aliased", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected);
}
