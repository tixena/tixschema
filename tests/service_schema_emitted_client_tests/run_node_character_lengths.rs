//! `minLength` and `maxLength` on a string, read by the generated Zod schema under node: the
//! verdict and the count its sentence reports are the code points Rust's `validate()` counts.

#![cfg(all(feature = "serde", feature = "typescript", feature = "zod"))]

use core::fmt::Write as _;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

const HARNESS: &str = "
function row(name: string, schema: ZodType, payload: string): void {
  const parsed = schema.safeParse(JSON.parse(payload));
  const said = parsed.success
    ? 'passes'
    : parsed.error.issues.map((issue) => `'${issue.path.join('.')}': ${issue.message}`).join('; ');
  console.log(`${name}: ${said}`);
}
";

const VALUES: [(&str, &str); 4] = [
    ("an accented letter", "\u{e9}"),
    ("a letter and a combining accent", "e\u{301}"),
    ("a character outside the plane", "\u{1f600}"),
    ("two characters outside the plane", "\u{1f600}\u{1f600}"),
];

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Initial {
    #[model_schema_prop(maxLength = 1)]
    pub name: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pair {
    #[model_schema_prop(minLength = 2)]
    pub name: String,
}

fn said(answered: Result<(), Vec<String>>) -> String {
    answered.map_or_else(|messages| messages.join("; "), |()| "passes".to_owned())
}

#[test]
fn a_length_counts_the_characters_rust_counts_under_zod() {
    const REQUIRED: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED) else {
        stand_down_modules(REQUIRED, "the generated Zod schemas of a length");
        return;
    };
    let mut source = String::from("import { z, type ZodType, type $brand } from \"zod\";\n\n");
    source.push_str(
        &[
            Initial::ts_definition(),
            Initial::zod_schema(),
            Pair::ts_definition(),
            Pair::zod_schema(),
        ]
        .join("\n\n"),
    );
    source.push_str(HARNESS);
    let mut expected = String::new();
    for (name, text) in VALUES {
        let initial = Initial {
            name: text.to_owned(),
        };
        let pair = Pair {
            name: text.to_owned(),
        };
        for (label, payload, answered) in [
            (
                "Initial",
                serde_json::to_string(&initial).unwrap(),
                initial.validate(),
            ),
            (
                "Pair",
                serde_json::to_string(&pair).unwrap(),
                pair.validate(),
            ),
        ] {
            let row = format!("{label}, {name}");
            let _ = writeln!(
                source,
                "row({}, {label}$Schema, {});",
                serde_json::to_string(&row).unwrap(),
                serde_json::to_string(&payload).unwrap()
            );
            let _ = writeln!(expected, "{row}: {}", said(answered));
        }
    }
    assert!(
        expected.contains("too long: maximum length is 1, got 2")
            && expected.contains("too short: minimum length is 2, got 1"),
        "{expected}"
    );
    let Some(printed) = ran_with_modules(
        "character-lengths",
        "TIXSCHEMA_NODE",
        "node",
        "lengths.mts",
        &source,
        &modules,
    ) else {
        return;
    };
    assert_eq!(printed, expected);
}
