//! The README's "Checks" section, held to what compiles and runs. Its Rust example is the one the
//! crate's rustdoc runs, its TypeScript functions are the ones the node test compiles and loads,
//! and each build it shows failing is recorded beside the run that earned its error.

/// The build the README shows for a name with no function, and the error it earns.
const MISSING_ERROR: &str = "\
error[E0425]: cannot find function `within_parcel` in this scope
  |
4 | #[model_schema(check = [\"withinParcel\"])]
  |                         ^^^^^^^^^^^^^^ help: try calling `within_parcel` as a method: `self.within_parcel()`";
const MISSING_SOURCE: &str = "\
#[model_schema(check = [\"withinParcel\"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
}";

/// The build the README shows for a function that asks a bound the type does not declare.
const UNBOUND_ERROR: &str = "\
error[E0277]: can't compare `CodeType` with `CodeType`
   |
 4 | #[model_schema(default_types(CodeType = String), check = [\"withoutRepeats\"])]
   |                                                           ^^^^^^^^^^^^^^^^ no implementation for `CodeType == CodeType`
   |
help: consider restricting type parameter `CodeType` with trait `PartialEq`
   |
 6 | pub struct Crate<CodeType: std::cmp::PartialEq> {
   |                          +++++++++++++++++++++";
const UNBOUND_SOURCE: &str = "\
#[model_schema(default_types(CodeType = String), check = [\"withoutRepeats\"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Crate<CodeType> {
    pub codes: Vec<CodeType>,
}

fn without_repeats<CodeType>(value: &Crate<CodeType>) -> Result<(), Vec<String>>
where
    CodeType: PartialEq,
{
    for (index, code) in value.codes.iter().enumerate() {
        if value.codes[..index].contains(code) {
            return Err(vec![format!(\"'codes.{index}': a code is listed once\")]);
        }
    }
    Ok(())
}";

/// The wrap one check adds to a Zod schema, as the README shows it for the parcel's own.
#[cfg(feature = "zod")]
const WRAP: &str = "\
.check((ctx) => {
  const own = { value: ctx.value, issues: ctx.issues.slice(0, 0) };
  withinParcel(own.value, own);
  for (const issue of own.issues) ctx.issues.push({ ...issue, continue: true });
})";

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
    include_str!("../../src/check_example.md")
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
        "the README no longer shows the example of `src/check_example.md`"
    );
    assert!(
        include_str!("../../src/lib.rs").contains("doc = include_str!(\"check_example.md\")"),
        "the rustdoc no longer runs the example"
    );
}

#[test]
fn the_readme_shows_the_typescript_functions_the_bundle_compiles() {
    let shown = from_until(
        compiled_typescript(),
        "import type { ParsePayload }",
        "\n\nexport function withARoute",
    );
    assert!(
        readme().contains(&format!("```ts\n{shown}\n```")),
        "the README no longer shows the functions the bundle compiles:\n{shown}"
    );
}

/// Each signature the documents state is the one of a function that compiles.
#[test]
fn the_documents_state_the_signatures_of_the_functions_that_compile() {
    let example = shown_example();
    let rustdoc = include_str!("../../src/lib.rs");
    for signature in [
        "fn without_duplicates(value: &[String]) -> Result<(), Vec<String>>",
        "fn within_parcel(value: &Parcel) -> Result<(), Vec<String>>",
    ] {
        assert!(example.contains(&format!("{signature} {{")), "{signature}");
        assert!(readme().contains(&format!("`{signature}`")), "{signature}");
        assert!(rustdoc.contains(&format!("/// {signature}")), "{signature}");
    }
    let generic = "fn with_a_code<CodeType>(value: &Crate<CodeType>) -> Result<(), Vec<String>>";
    assert!(include_str!("models.rs").contains(&format!("{generic} {{")));
    assert!(readme().contains(&format!("`{generic}`")));
    for signature in [
        "function withoutDuplicates(value: Array<string>, ctx: ParsePayload<Array<string>>): void",
        "function withinParcel(value: Parcel, ctx: ParsePayload<Parcel>): void",
        "function withACode<CodeType>(value: Crate<CodeType>, ctx: \
         ParsePayload<Crate<CodeType>>): void",
    ] {
        assert!(
            compiled_typescript().contains(&format!("export {signature} {{")),
            "{signature}"
        );
        assert!(readme().contains(&format!("`{signature}`")), "{signature}");
    }
}

#[cfg(feature = "zod")]
#[test]
fn the_readme_shows_the_wrap_the_schema_carries() {
    let schema = super::models::Parcel::zod_schema();
    assert!(schema.contains(WRAP), "got: {schema}");
    assert!(
        readme().contains(&format!("```ts\n{WRAP}\n```")),
        "the README no longer shows the wrap"
    );
}

/// Every line the README shows of a failing build is one the emitter's doc comment records
/// under the standalone run of that build.
#[test]
fn the_readme_quotes_the_errors_recorded_beside_the_runs_that_earned_them() {
    let emitter = include_str!("../../src/model_schema/checks.rs");
    for (source, error) in [
        (MISSING_SOURCE, MISSING_ERROR),
        (UNBOUND_SOURCE, UNBOUND_ERROR),
    ] {
        assert!(
            readme().contains(&format!(
                "```rust,compile_fail\n{source}\n```\n\n```text\n{error}\n```"
            )),
            "the README no longer shows this build and its error:\n{source}\n{error}"
        );
        for line in source.lines().chain(error.lines()) {
            let recorded = if line.is_empty() {
                "///\n".to_owned()
            } else {
                format!("/// {line}\n")
            };
            assert!(
                emitter.contains(&recorded),
                "no doc comment of the emitter records this line any more:\n{line}"
            );
        }
    }
}
