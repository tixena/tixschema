//! An `i64` and a `u64` in an example or in a default, run by node with the real `zod`: each
//! example is read back from `.meta()` as the bigint the Rust value is and parses with the schema
//! it is attached to, and a default is the bigint Rust declares. The models that hold a `u64` are
//! declared where neither `swift` nor `kotlin` is on, which refuse it.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

const REQUIRED_PACKAGES: &[&str] = &["zod"];

/// Reads each example back from `.meta()`, compares each value with `===` against its bigint,
/// parses it with its own schema, and parses a payload that leaves a defaulted key out.
const DRIVER: &str = r#"
const shown = (value) => JSON.stringify(value, (_, held) => (typeof held === "bigint" ? `${held}n` : held));
const examplesOf = (schema) => schema.meta().examples;
const [ledger] = examplesOf(Ledger$Schema);
const [extremes] = examplesOf(Extremes$Schema);
const [far] = examplesOf(FarLines$Schema);
const [short] = examplesOf(ShortLines$Schema);
const [none] = examplesOf(NoLines$Schema);
const [many] = examplesOf(EitherMany$Schema);
const [one] = examplesOf(EitherOne$Schema);
const everyParses = [
  [Ledger$Schema, ledger], [Extremes$Schema, extremes], [FarLines$Schema, far],
  [ShortLines$Schema, short], [NoLines$Schema, none], [EitherMany$Schema, many], [EitherOne$Schema, one],
].map(([schema, example]) => schema.safeParse(example).success);
const defaulted = Ledger$Schema.parse({ signed: 1, count: 2 });
const checks = {
  ledger: ledger.signed === 12n && ledger.count === 3 && ledger.carried === 9007199254740993n,
  extremes: extremes.greatest === 9223372036854775807n && extremes.least === -9223372036854775808n,
  far: far.lines.length === 2 && far.lines[0] === 12n && far.lines[1] === 9007199254740993n,
  short: short.lines.length === 2 && short.lines[0] === 12n && short.lines[1] === 240n,
  none: none.lines.length === 0,
  many: many.length === 2 && many[0] === 12n && many[1] === 9007199254740993n,
  one: one === 240,
  defaulted: defaulted.carried === 9007199254740993n,
  everyParses,
};
const parsed = {
  ledger: Ledger$Schema.parse(ledger),
  far: FarLines$Schema.parse(far),
  short: ShortLines$Schema.parse(short),
  many: EitherMany$Schema.parse(many),
};
const out = { checks, parsed: JSON.parse(shown(parsed)), examples: JSON.parse(shown({ ledger, extremes, far, short, none, many, one })) };
UNSIGNED
console.log(JSON.stringify(out));
"#;

/// The part of the driver that reads the models that hold a `u64`.
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
const UNSIGNED: &str = "
const [account] = examplesOf(AccountNumber$Schema);
const [greatest] = examplesOf(Greatest$Schema);
out.unsigned = {
  account: account.signed === 9007199254740993n && account.unsigned === 9007199254740993n
    && account.numbers.length === 2 && account.numbers[0] === 12n && account.numbers[1] === 9007199254740993n,
  greatest: greatest.top === 18446744073709551615n,
  parses: AccountNumber$Schema.safeParse(account).success && Greatest$Schema.safeParse(greatest).success,
  defaulted: AccountNumber$Schema.parse({ numbers: [], signed: 1, unsigned: 2 }).carried === 9007199254740993n,
  shown: JSON.parse(shown({ account, greatest })),
};
";

#[cfg(any(feature = "swift", feature = "kotlin"))]
const UNSIGNED: &str = "";

/// A ledger row.
///
/// ```rust example
/// Ledger { carried: far(), count: 3, signed: 12 }
/// ```
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ledger {
    #[serde(default = "far")]
    pub carried: i64,
    pub count: u32,
    pub signed: i64,
}

/// The extremes of an `i64`.
///
/// ```rust example
/// Extremes { greatest: i64::MAX, least: i64::MIN }
/// ```
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Extremes {
    pub greatest: i64,
    pub least: i64,
}

/// A list of an `i64` beyond what a JavaScript `number` holds exactly.
///
/// ```rust example
/// FarLines { lines: vec![12, far()] }
/// ```
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FarLines {
    pub lines: Vec<i64>,
}

/// A list of an `i64` with two values.
///
/// ```rust example
/// ShortLines { lines: vec![12, 240] }
/// ```
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShortLines {
    pub lines: Vec<i64>,
}

/// A list of an `i64` with no value.
///
/// ```rust example
/// NoLines { lines: vec![] }
/// ```
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NoLines {
    pub lines: Vec<i64>,
}

/// An untagged enum over a list.
///
/// ```rust example
/// EitherMany::Many(vec![12, far()])
/// ```
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum EitherMany {
    Many(Vec<i64>),
    One(i64),
}

/// An untagged enum over an `i64`.
///
/// ```rust example
/// EitherOne::One(240)
/// ```
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum EitherOne {
    Many(Vec<i64>),
    One(i64),
}

/// An account number with an `i64`, a `u64` and a list of them.
///
/// ```rust example
/// AccountNumber {
///     carried: 7,
///     numbers: vec![12, far_unsigned()],
///     signed: far(),
///     unsigned: far_unsigned(),
/// }
/// ```
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccountNumber {
    #[serde(default = "far_unsigned")]
    pub carried: u64,
    pub numbers: Vec<u64>,
    pub signed: i64,
    pub unsigned: u64,
}

/// The greatest `u64`.
///
/// ```rust example
/// Greatest { top: u64::MAX }
/// ```
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Greatest {
    pub top: u64,
}

/// The schemas the driver reads, beside the types.
fn emitted() -> String {
    let mut parts = vec!["import { z, type ZodType } from \"zod\";".to_owned()];
    parts.extend([
        Ledger::ts_definition(),
        Ledger::zod_schema(),
        Extremes::ts_definition(),
        Extremes::zod_schema(),
        FarLines::ts_definition(),
        FarLines::zod_schema(),
        ShortLines::ts_definition(),
        ShortLines::zod_schema(),
        NoLines::ts_definition(),
        NoLines::zod_schema(),
        EitherMany::ts_definition(),
        EitherMany::zod_schema(),
        EitherOne::ts_definition(),
        EitherOne::zod_schema(),
    ]);
    #[cfg(not(any(feature = "swift", feature = "kotlin")))]
    parts.extend([
        AccountNumber::ts_definition(),
        AccountNumber::zod_schema(),
        Greatest::ts_definition(),
        Greatest::zod_schema(),
    ]);
    parts.join("\n\n")
}

/// What the driver prints for the models that hold an `i64`.
fn expected() -> Value {
    json!({
        "checks": {
            "ledger": true, "extremes": true, "far": true, "short": true, "none": true,
            "many": true, "one": true, "defaulted": true,
            "everyParses": [true, true, true, true, true, true, true],
        },
        "parsed": {
            "ledger": { "signed": "12n", "count": 3, "carried": "9007199254740993n" },
            "far": { "lines": ["12n", "9007199254740993n"] },
            "short": { "lines": ["12n", "240n"] },
            "many": ["12n", "9007199254740993n"],
        },
        "examples": {
            "ledger": { "signed": "12n", "count": 3, "carried": "9007199254740993n" },
            "extremes": { "greatest": "9223372036854775807n", "least": "-9223372036854775808n" },
            "far": { "lines": ["12n", "9007199254740993n"] },
            "short": { "lines": ["12n", "240n"] },
            "none": { "lines": [] },
            "many": ["12n", "9007199254740993n"],
            "one": 240,
        },
    })
}

/// Each example is the bigint the Rust value is, parses with its own schema, and a default is the
/// bigint Rust declares.
#[test]
fn an_example_and_a_default_read_back_in_node_as_the_bigint_rust_holds() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "the examples written as bigint literals");
        return;
    };
    let module = format!("{}\n\n{}", emitted(), DRIVER.replace("UNSIGNED", UNSIGNED));
    let Some(wrote) = ran_with_modules(
        "bigint_literals",
        RUNTIME_VAR,
        "node",
        "bigint_literals.mts",
        &module,
        &modules,
    ) else {
        return;
    };
    let mut ran: Value = serde_json::from_str(wrote.trim()).unwrap();
    without_unsigned(&mut ran);
    assert_eq!(ran, expected());
}

/// Holds what the driver printed for the models that hold a `u64`, and takes it out of `ran`.
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
fn without_unsigned(ran: &mut Value) {
    let unsigned = ran.as_object_mut().unwrap().remove("unsigned").unwrap();
    assert_eq!(
        unsigned,
        json!({
            "account": true,
            "greatest": true,
            "parses": true,
            "defaulted": true,
            "shown": {
                "account": {
                    "numbers": ["12n", "9007199254740993n"],
                    "signed": "9007199254740993n",
                    "unsigned": "9007199254740993n",
                    "carried": "7n",
                },
                "greatest": { "top": "18446744073709551615n" },
            },
        })
    );
}

/// The driver prints nothing for a `u64` where `swift` or `kotlin` refuses one.
#[cfg(any(feature = "swift", feature = "kotlin"))]
const fn without_unsigned(_ran: &mut Value) {}

/// An `i64` beyond what a JavaScript `number` holds exactly.
const fn far() -> i64 {
    (1_i64 << 53_i32) + 1_i64
}

/// A `u64` beyond what a JavaScript `number` holds exactly.
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
const fn far_unsigned() -> u64 {
    (1_u64 << 53_i32) + 1_u64
}
