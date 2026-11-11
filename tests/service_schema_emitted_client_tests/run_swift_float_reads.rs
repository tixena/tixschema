//! An `f32` beyond its range, read by the emitted Swift types compiled with warnings as errors as
//! an infinity of its sign, as serde reads it: under an `Option`, as a map's value, in a list and
//! behind a type parameter filled with it. Foundation refuses such a `Float`.

#![cfg(all(feature = "swift", feature = "serde"))]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::judged;

/// Prints each `f32` a payload reads into, in the order of [`Rigging`]'s members.
const DRIVER: &str = r#"
func shown(_ value: Float?) -> String { value.map { "\($0)" } ?? "nil" }
for payload in payloads {
  do {
    let read = try JSONDecoder().decode(Rigging.self, from: Data(payload.utf8))
    print([shown(read.keyed["k"]), shown(read.listed.first), shown(read.maybe), shown(read.spar.held)].joined(separator: " "))
  } catch {
    print("refused: \(error)")
  }
}
"#;

/// Each payload, and the `f32`s serde reads it into, spelled as Swift prints them.
const PAYLOADS: [(&str, &str); 2] = [
    (
        r#"{"keyed":{"k":3.5e38},"listed":[-1e39],"maybe":1e39,"spar":{"held":1e39}}"#,
        "inf -inf inf inf",
    ),
    (
        r#"{"keyed":{"k":2.5},"listed":[0.5],"maybe":1.5,"spar":{"held":1.25}}"#,
        "2.5 0.5 1.5 1.25",
    ),
];

/// A member of the type its parameter is filled with.
#[model_schema(default_types(Held = f32))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Spar<Held> {
    pub held: Held,
}

/// An `f32` in each place a type holds one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Rigging {
    pub keyed: HashMap<String, f32>,
    pub listed: Vec<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<f32>,
    pub spar: Spar<f32>,
}

#[test]
fn an_f32_beyond_its_range_is_read_as_an_infinity_in_every_place_swift_reads_one() {
    for (payload, said) in PAYLOADS {
        let read: Rigging = serde_json::from_str(payload).unwrap();
        let shown = |value: f32| {
            if value.is_infinite() {
                if value < 0.0 { "-inf" } else { "inf" }.to_owned()
            } else {
                value.to_string()
            }
        };
        let serde_said = [
            shown(read.keyed["k"]),
            shown(read.listed[0]),
            shown(read.maybe.unwrap()),
            shown(read.spar.held),
        ]
        .join(" ");
        assert_eq!(serde_said, said, "serde reads {payload}");
    }
    let listed: Vec<String> = PAYLOADS
        .iter()
        .map(|(payload, _)| format!("  #\"{payload}\"#,"))
        .collect();
    let source = [
        "import Foundation".to_owned(),
        spar_swift::swift_definition(),
        rigging_swift::swift_definition(),
        format!("let payloads: [String] = [\n{}\n]", listed.join("\n")),
        DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-float-reads",
        "TIXSCHEMA_SWIFT",
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    let expected: Vec<&str> = PAYLOADS.iter().map(|(_, said)| *said).collect();
    assert_eq!(run.stdout.trim().lines().collect::<Vec<_>>(), expected);
}
