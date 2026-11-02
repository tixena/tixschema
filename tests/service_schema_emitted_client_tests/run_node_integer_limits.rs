//! The Zod schema of each limit of [`ROWS`], run by node with the real `zod`: each payload is read
//! with the source-text reviver the emitted readers use, so an integer past what a `number` holds
//! reaches the schema as its `bigint`.

use core::fmt::Write as _;

use super::integer_limit_models::{
    LimitCeiling, LimitEnds, LimitFloor, LimitForms, LimitMixed, REPORT, ROWS,
};
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

const REQUIRED_PACKAGES: &[&str] = &["zod"];

/// What prints a row: the reader, and `passes` or each issue under the key it names.
const REPORTER: &str = r#"
const read = (text) =>
  JSON.parse(text, (_key, value, context) =>
    typeof value === "number" && !Number.isSafeInteger(value) && /^-?(0|[1-9][0-9]*)$/.test(context?.source ?? "")
      ? BigInt(context.source)
      : value,
  );
const report = (name, schema, text) => {
  const parsed = schema.safeParse(read(text));
  if (parsed.success) {
    console.log(`${name}: passes`);
    return;
  }
  console.log(`${name}: refused`);
  for (const issue of parsed.error.issues) {
    console.log(`    '${issue.path.join(".")}': ${issue.message}`);
  }
};
"#;

#[test]
fn each_limit_holds_in_zod_as_it_is_written() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "the integer limits of the Zod schemas");
        return;
    };
    let mut module = "import { z, type ZodType } from \"zod\";".to_owned();
    for declared in [
        LimitCeiling::ts_definition(),
        LimitCeiling::zod_schema(),
        LimitFloor::ts_definition(),
        LimitFloor::zod_schema(),
        LimitForms::ts_definition(),
        LimitForms::zod_schema(),
        LimitMixed::ts_definition(),
        LimitMixed::zod_schema(),
        LimitEnds::ts_definition(),
        LimitEnds::zod_schema(),
    ] {
        let _ = write!(module, "\n\n{declared}");
    }
    module.push_str(REPORTER);
    for (name, read_as, json) in ROWS {
        let _ = writeln!(module, "report('{name}', {read_as}$Schema, '{json}');");
    }
    let Some(printed) = ran_with_modules(
        "integer_limits",
        RUNTIME_VAR,
        "node",
        "integer_limits.mts",
        &module,
        &modules,
    ) else {
        return;
    };
    assert_eq!(printed, REPORT);
}
