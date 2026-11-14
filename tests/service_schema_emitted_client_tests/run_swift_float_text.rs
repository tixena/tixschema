//! A float quoted back by the emitted Swift `validate()` is written as Rust's `Display` writes it:
//! the digits, with no exponent and no `.0` on a whole float.

#![cfg(all(feature = "swift", feature = "serde"))]

use core::fmt::Write as _;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// A limit no row reaches, on a double and on a single.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FloatText {
    #[model_schema_prop(minimum = 1e38)]
    pub single: f32,
    #[model_schema_prop(minimum = 1e300)]
    pub wide: f64,
}

/// What each row writes into `wide` and `single`, and the text Rust's `Display` gives the member
/// under test.
fn rows() -> Vec<(String, String, String)> {
    let doubles: [f64; 5] = [1e30, 1.5, -2e-7, 12_345.678_9, 1e-10];
    let singles: [f32; 3] = [4_000_000_000.0, 0.1, 1.5e-5];
    let mut rows = Vec::new();
    for value in doubles {
        rows.push((
            format!(r#"{{"wide":{value:e},"single":1e38}}"#),
            "wide".to_owned(),
            value.to_string(),
        ));
    }
    for value in singles {
        rows.push((
            format!(r#"{{"wide":1e300,"single":{value:e}}}"#),
            "single".to_owned(),
            value.to_string(),
        ));
    }
    rows
}

#[test]
fn a_float_is_quoted_in_swift_as_rust_displays_it() {
    let rows = rows();
    let mut main = String::new();
    for (json, _, _) in &rows {
        let _ = writeln!(
            main,
            "for message in try JSONDecoder().decode(Unjudged.self, from: Data(#\"{json}\"#.utf8)).held.validate() {{ print(message) }}"
        );
    }
    let source = [
        "import Foundation".to_owned(),
        float_text_swift::swift_definition(),
        "
fileprivate struct Unjudged: Decodable {
  let held: FloatText
  init(from decoder: Decoder) throws { held = try FloatText(swiftSchemaUnjudged: decoder) }
}
"
        .to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-float-text",
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
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    let said: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(said.len(), rows.len(), "{}", run.stdout);
    for ((json, member, shown), line) in rows.iter().zip(said) {
        let refused = serde_json::from_str::<FloatText>(json)
            .unwrap_err()
            .to_string();
        assert!(
            refused.starts_with(&format!("'{member}': too small: minimum is ")),
            "{json}: {refused}"
        );
        assert!(
            refused.ends_with(&format!(", got {shown}")),
            "{json}: {refused}"
        );
        assert_eq!(line, refused, "{json}");
    }
}
