//! The README's "Preprocessing" section, held to what compiles and runs. Its Rust example is the
//! one the crate's rustdoc runs, its TypeScript functions are the ones the node test compiles and
//! loads, and the build it shows failing is recorded beside the run that earned its error.

use super::models::{SAMPLE, SAMPLE_ROWS};

/// The build the README shows for a name with no function, and the error it earns.
const MISSING_ERROR: &str = "\
error[E0425]: cannot find function `without_fragile` in this scope
  |
4 | #[model_schema(preprocess = [\"withoutFragile\"])]
  |                              ^^^^^^^^^^^^^^^^ not found in this scope";
const MISSING_SOURCE: &str = "\
#[model_schema(preprocess = [\"withoutFragile\"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
}";

/// The schema the README shows for the parcel of its example, which declares its members in
/// another order than the parcel of the models does.
#[cfg(all(feature = "serde", feature = "typescript", feature = "zod"))]
const WRAPPED: &str = "\
const Parcel$RawSchema = z.preprocess(withoutFragile, z.strictObject({
  label: z.preprocess(trimmed, z.string().min(1, { error: (issue) => `too short: minimum length is 1, got ${String(issue.input).length}` })),
  itemCodeList: z.array(z.string()),
  fragileItemCodes: z.array(z.string()),
}));";

/// The TypeScript the node test compiles with the bundle and loads.
fn compiled_typescript() -> &'static str {
    include_str!("../generic_types_tests/declarations.rs")
}

/// What `text` holds from `opens` up to `closes`.
fn from_until<'text>(text: &'text str, opens: &str, closes: &str) -> &'text str {
    let rest = text
        .find(opens)
        .and_then(|start| text.get(start..))
        .unwrap();
    rest.split_once(closes).unwrap().0
}

fn readme() -> &'static str {
    include_str!("../../README.md")
}

/// The example the rustdoc runs, as a reader sees it: without the lines rustdoc hides.
fn shown_example() -> String {
    include_str!("../../src/preprocess_example.md")
        .trim_end()
        .strip_prefix("```rust\n")
        .and_then(|rest| rest.strip_suffix("\n```"))
        .unwrap()
        .lines()
        .filter(|line| !line.starts_with("# "))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_readme_shows_the_example_the_rustdoc_runs() {
    assert!(
        readme().contains(&format!("```rust\n{}\n```", shown_example())),
        "the README no longer shows the example of `src/preprocess_example.md`"
    );
    assert!(
        include_str!("../../src/lib.rs").contains("doc = include_str!(\"preprocess_example.md\")"),
        "the rustdoc no longer runs the example"
    );
}

#[test]
fn the_readme_shows_the_typescript_functions_the_bundle_compiles() {
    let shown = from_until(
        compiled_typescript(),
        "export function trimmed(payload: unknown)",
        "\n\nexport function unwrapped",
    );
    assert!(
        readme().contains(&format!("```ts\n{shown}\n```")),
        "the README no longer shows the functions the bundle compiles:\n{shown}"
    );
}

/// Each signature the documents state is the one of a function that compiles. The example
/// imports `Value`, and takes the payload its type's function changes as `mut`.
#[test]
fn the_documents_state_the_signatures_of_the_functions_that_compile() {
    let example = shown_example();
    assert!(example.contains("use serde_json::Value;"));
    let spelled = example
        .replace("(mut payload: Value)", "(payload: Value)")
        .replace(
            "(payload: Value) -> Value",
            "(payload: serde_json::Value) -> serde_json::Value",
        );
    let rustdoc = include_str!("../../src/lib.rs");
    for signature in [
        "fn trimmed(payload: serde_json::Value) -> serde_json::Value",
        "fn without_fragile(payload: serde_json::Value) -> serde_json::Value",
    ] {
        assert!(spelled.contains(&format!("{signature} {{")), "{signature}");
        assert!(readme().contains(&format!("`{signature}`")), "{signature}");
        assert!(rustdoc.contains(&format!("/// {signature}")), "{signature}");
    }
    for signature in [
        "function trimmed(payload: unknown): unknown",
        "function withoutFragile(payload: unknown): unknown",
    ] {
        assert!(
            compiled_typescript().contains(&format!("export {signature} {{")),
            "{signature}"
        );
        assert!(readme().contains(&format!("`{signature}`")), "{signature}");
        assert!(rustdoc.contains(&format!("/// {signature}")), "{signature}");
    }
}

/// The payloads of the README's table are the ones every side reads, and the line it shows for
/// the refused row is the one every side prints.
#[test]
fn the_readme_shows_the_rows_every_side_reads() {
    for (number, &(row, payload)) in (1_u8..).zip(&SAMPLE_ROWS) {
        if row == "row 5" {
            let held = SAMPLE_ROWS[0].1;
            assert_eq!(payload, format!("{{\"parcel\":{held}}}"));
            assert!(readme().contains("| 5 | row 1 inside `Manifest`, under `parcel` |"));
        } else {
            assert_eq!(row, format!("row {number}"));
            assert!(
                readme().contains(&format!("| {number} | `{payload}` |")),
                "the README no longer shows {row}"
            );
        }
    }
    let refused = "'label': too short: minimum length is 1, got 0";
    assert!(SAMPLE.contains(&format!("row 2: refused\n    {refused}\n")));
    assert!(readme().contains(&format!("| refused: `{refused}` | the same line |")));
}

/// Each line of the schema the README shows is a line the parcel of the models publishes.
#[cfg(all(feature = "serde", feature = "typescript", feature = "zod"))]
#[test]
fn the_readme_shows_the_wrap_the_schema_carries() {
    let schema = super::models::Parcel::zod_schema();
    for line in WRAPPED.lines() {
        assert!(
            schema.lines().any(|published| published == line),
            "no `{line}` in: {schema}"
        );
    }
    assert!(
        readme().contains(&format!("```ts\n{WRAPPED}\n```")),
        "the README no longer shows the wrap"
    );
}

/// Every line the README shows of the failing build is one the emitter's doc comment records
/// under the standalone run of that build.
#[test]
fn the_readme_quotes_the_error_recorded_beside_the_run_that_earned_it() {
    let emitter = include_str!("../../src/model_schema/preprocess.rs");
    assert!(
        readme().contains(&format!(
            "```rust,compile_fail\n{MISSING_SOURCE}\n```\n\n```text\n{MISSING_ERROR}\n```"
        )),
        "the README no longer shows this build and its error:\n{MISSING_SOURCE}\n{MISSING_ERROR}"
    );
    for line in MISSING_SOURCE.lines().chain(MISSING_ERROR.lines()) {
        assert!(
            emitter.contains(&format!("/// {line}\n")),
            "no doc comment of the emitter records this line any more:\n{line}"
        );
    }
}

/// What the README says a document hands a function, and what a read answers for a value no
/// JSON value holds, is what the tests of a document assert under each major.
#[test]
fn the_readme_states_what_the_tests_of_a_document_measure() {
    let measured = include_str!("../recovering_decode_bson_tests/preprocessed.rs");
    for (shown, asserted) in [
        (
            "`{\"$oid\": \"0123456789abcdef01234567\"}`",
            "\"id\": { \"$oid\": \"0123456789abcdef01234567\" },",
        ),
        (
            "`{\"$date\": \"2023-11-14T22:13:20Z\"}`",
            "\"at\": { \"$date\": \"2023-11-14T22:13:20Z\" },",
        ),
        (
            "`invalid type: null, expected f64`",
            "\"invalid type: null, expected f64\"",
        ),
        (
            "`invalid type: byte array, expected any valid JSON value`",
            "\"invalid type: byte array, expected any valid JSON value\"",
        ),
        (
            "`BSON error. Kind: A deserialization-related error occurred. Message: ... .`",
            "\"BSON error. Kind: A deserialization-related error occurred. Message: {refused}.\"",
        ),
    ] {
        assert!(
            readme().contains(shown),
            "the README no longer shows {shown}"
        );
        assert!(measured.contains(asserted), "no test asserts {asserted}");
    }
}
