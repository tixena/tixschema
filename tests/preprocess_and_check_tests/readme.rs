//! The README's "Preprocess and check" section, held to what compiles and runs. The parcel it
//! declares is the one of `models.rs`, each cell of its table is text the test of that language
//! holds, and the 13 lines it shows are the ones the test of each language prints.

/// The payload of each of the six shared cases, as `tests/check_tests/table.rs` holds it.
const PAYLOADS: [&str; 6] = [
    r#"{"label":"A","itemCodeList":["x"],"fragileItemCodes":["x"]}"#,
    r#"{"label":"","itemCodeList":["x"],"fragileItemCodes":["x"]}"#,
    r#"{"label":"A","itemCodeList":["x","x"],"fragileItemCodes":["x"]}"#,
    r#"{"label":"A","itemCodeList":["x"],"fragileItemCodes":["x","y"]}"#,
    r#"{"label":"","itemCodeList":["x","x"],"fragileItemCodes":["y"]}"#,
    r#"{"parcel":{"label":"A","itemCodeList":["x"],"fragileItemCodes":["x","y"]}}"#,
];

/// The 13 lines every language prints for the six shared cases.
const TABLE: &str = "\
case 1: passes
case 2: refused
    'label': too short: minimum length is 1, got 0
case 3: refused
    'itemCodeList': duplicate item code x
case 4: refused
    'fragileItemCodes.1': y is outside of itemCodeList
case 5: refused
    'itemCodeList': duplicate item code x
    'label': too short: minimum length is 1, got 0
    'fragileItemCodes.0': y is outside of itemCodeList
case 6: refused
    'parcel.fragileItemCodes.1': y is outside of itemCodeList
";

/// The test of each client that holds the functions and the errors of a `check`, of a
/// `preprocess` and of a read.
const DART_CHECK: &str =
    include_str!("../service_schema_emitted_client_tests/run_dart_validate.rs");
const DART_PREPROCESS: &str =
    include_str!("../service_schema_emitted_client_tests/run_dart_preprocess.rs");
const DART_READ: &str = include_str!("../service_schema_emitted_client_tests/run_dart_read.rs");
const KOTLIN_CHECK: &str =
    include_str!("../service_schema_emitted_client_tests/run_kotlin_validate.rs");
const KOTLIN_PREPROCESS: &str =
    include_str!("../service_schema_emitted_client_tests/run_kotlin_preprocess.rs");
const KOTLIN_READ: &str = include_str!("../service_schema_emitted_client_tests/run_kotlin_read.rs");
const SWIFT_CHECK: &str =
    include_str!("../service_schema_emitted_client_tests/run_swift_validate.rs");
const SWIFT_PREPROCESS: &str =
    include_str!("../service_schema_emitted_client_tests/run_swift_preprocess.rs");
const SWIFT_READ: &str = include_str!("../service_schema_emitted_client_tests/run_swift_read.rs");

/// The TypeScript the node test compiles with the bundle and loads.
const TYPESCRIPT: &str = include_str!("../generic_types_tests/declarations.rs");

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

/// The section, from its heading up to the heading of the section after it.
fn section() -> &'static str {
    from_until(
        readme(),
        "### Preprocess and check\n",
        "\n### Checks (`check`)\n",
    )
}

/// The anchor a heading of the README is linked by.
fn anchor(heading: &str) -> String {
    heading
        .trim_start_matches('#')
        .trim()
        .to_lowercase()
        .chars()
        .filter(|&letter| letter.is_alphanumeric() || matches!(letter, ' ' | '-' | '_'))
        .map(|letter| if letter == ' ' { '-' } else { letter })
        .collect()
}

#[test]
fn the_readme_declares_the_parcel_that_compiles() {
    let opens = "#[model_schema(check = [\"withinParcel\"])]";
    let declared = from_until(include_str!("models.rs"), opens, "\n}\n");
    assert!(
        section().contains(&format!("```rust\n{declared}\n}}\n```")),
        "the section no longer shows the parcel of `models.rs`:\n{declared}\n}}"
    );
}

/// Holds one signature of the table to the section and to the source that compiles it, which
/// writes `opened` after it.
fn compiles(signature: &str, compiled: &str, opened: &str) {
    assert!(
        section().contains(&format!("`{signature}`")),
        "the section no longer states `{signature}`"
    );
    assert!(
        compiled.contains(&format!("{signature}{opened}")),
        "no test compiles `{signature}` any more"
    );
}

/// The Rust functions of the table are the ones the rustdoc compiles, and the TypeScript ones
/// the ones the node test compiles with the bundle.
#[test]
fn each_rust_and_typescript_function_of_the_table_is_one_a_test_compiles() {
    let checks = include_str!("../../src/check_example.md");
    let example = include_str!("../../src/preprocess_example.md");
    assert!(example.contains("use serde_json::Value;"));
    let preprocess = example.replace(
        "(payload: Value) -> Value",
        "(payload: serde_json::Value) -> serde_json::Value",
    );
    let trimmed = "fn trimmed(payload: serde_json::Value) -> serde_json::Value";
    compiles(trimmed, &preprocess, " {");
    for signature in [
        "fn without_duplicates(value: &[String]) -> Result<(), Vec<String>>",
        "fn within_parcel(value: &Parcel) -> Result<(), Vec<String>>",
    ] {
        compiles(signature, checks, " {");
    }
    for signature in [
        "function trimmed(payload: unknown): unknown",
        "function withoutDuplicates(value: Array<string>, ctx: ParsePayload<Array<string>>): void",
        "function withinParcel(value: Parcel, ctx: ParsePayload<Parcel>): void",
    ] {
        compiles(signature, TYPESCRIPT, " {");
    }
}

/// The Dart, Swift and Kotlin functions of the table are the ones the run of each client
/// compiles.
#[test]
fn each_client_function_of_the_table_is_one_the_run_of_that_client_compiles() {
    for (signature, compiled, opened) in [
        ("dynamic trimmed(dynamic payload)", DART_PREPROCESS, " =>"),
        (
            "List<String> withoutDuplicates(List<String> value)",
            DART_CHECK,
            " {",
        ),
        ("List<String> withinParcel(Parcel value)", DART_CHECK, " =>"),
        (
            "func trimmed(_ payload: Any) -> Any",
            SWIFT_PREPROCESS,
            " {",
        ),
        (
            "func withoutDuplicates(_ value: [String]) -> [String]",
            SWIFT_CHECK,
            " {",
        ),
        (
            "func withinParcel(_ value: Parcel) -> [String]",
            SWIFT_CHECK,
            " {",
        ),
        (
            "fun trimmed(payload: JsonElement): JsonElement",
            KOTLIN_PREPROCESS,
            " =",
        ),
        (
            "fun withoutDuplicates(value: List<String>): List<String>",
            KOTLIN_CHECK,
            " {",
        ),
        (
            "fun withinParcel(value: Parcel): List<String>",
            KOTLIN_CHECK,
            " =",
        ),
    ] {
        compiles(signature, compiled, opened);
    }
}

/// Each error the section quotes for a name with no function is one a run of that language is
/// held to. The TypeScript error is read off `tsc --strict`, and no test holds it.
#[test]
fn each_error_of_a_name_with_no_function_is_one_a_run_asserts() {
    for (shown, recorded, asserted) in [
        (
            "``error[E0425]: cannot find function `within_parcel` in this scope``",
            "/// error[E0425]: cannot find function `within_parcel` in this scope\n",
            include_str!("../../src/model_schema/checks.rs"),
        ),
        (
            "``error[E0425]: cannot find function `without_fragile` in this scope``",
            "/// error[E0425]: cannot find function `without_fragile` in this scope\n",
            include_str!("../../src/model_schema/preprocess.rs"),
        ),
        (
            "`Error: Method not found: 'trimmed'.`",
            "Error: Method not found: 'trimmed'.",
            DART_PREPROCESS,
        ),
        (
            "`Error: The method 'withinParcel' isn't defined for the type 'Parcel'.`",
            "Error: The method 'withinParcel' isn't defined for the type 'Parcel'.",
            DART_CHECK,
        ),
        (
            "`error: cannot find 'trimmed' in scope`",
            "\"error: cannot find 'trimmed' in scope\"",
            SWIFT_PREPROCESS,
        ),
        (
            "`error: cannot find 'withinParcel' in scope`",
            "\"error: cannot find 'withinParcel' in scope\"",
            SWIFT_CHECK,
        ),
        (
            "`error: unresolved reference 'trimmed'.`",
            "\"error: unresolved reference 'trimmed'.\"",
            KOTLIN_PREPROCESS,
        ),
        (
            "`error: unresolved reference 'withinParcel'.`",
            "error: unresolved reference 'withinParcel'.",
            KOTLIN_CHECK,
        ),
        (
            "``error[E0592]: duplicate definitions with name `validate` ``",
            "/// error[E0592]: duplicate definitions with name `validate`\n",
            include_str!("../../src/model_schema.rs"),
        ),
    ] {
        assert!(
            section().contains(shown),
            "the section no longer quotes {shown}"
        );
        assert!(asserted.contains(recorded), "no run asserts {recorded}");
    }
    for measured in [
        "`error TS2304: Cannot find name 'withinParcel'.`",
        "`error TS2304: Cannot find name 'trimmed'.`",
    ] {
        assert!(
            section().contains(measured),
            "the section no longer quotes {measured}"
        );
    }
}

/// The read each language calls a function from, and what it throws, are the names the test of
/// that language's read holds.
#[test]
fn each_read_of_the_table_is_one_a_test_of_its_language_holds() {
    for (shown, held, by) in [
        (
            "`model_schema_read_unjudged`",
            "model_schema_read_unjudged",
            include_str!("../../src/model_schema/judged_read/tests.rs"),
        ),
        (
            "`fromJsonUnjudged`",
            ".fromJsonUnjudged(",
            include_str!("../dart_tests/read.rs"),
        ),
        ("a `FormatException`", "on FormatException catch", DART_READ),
        (
            "`init(swiftSchemaUnjudged:)`",
            "public init(swiftSchemaUnjudged decoder: Decoder) throws",
            include_str!("../swift_tests/read.rs"),
        ),
        (
            "`DecodingError.dataCorrupted`",
            "catch DecodingError.dataCorrupted(let context)",
            SWIFT_READ,
        ),
        (
            "`{Type}Unjudged`",
            "object {named}Unjudged : ",
            include_str!("../kotlin_tests/read.rs"),
        ),
        (
            "`{Type}Serializer`",
            "object {named}Serializer : KSerializer<{named}>",
            include_str!("../kotlin_tests/read.rs"),
        ),
        (
            "a `SerializationException`",
            "catch (refused: SerializationException)",
            KOTLIN_READ,
        ),
    ] {
        assert!(
            section().contains(shown),
            "the section no longer names {shown}"
        );
        assert!(by.contains(held), "no test holds `{held}` any more");
    }
}

/// The block the section shows is the one text the test of each of the five languages holds its
/// run to: the Rust `validate()` and the Zod schema to the table of the check models, and each
/// client to its own copy.
#[test]
fn the_13_lines_are_the_ones_the_test_of_each_language_prints() {
    assert_eq!(TABLE.lines().count(), 13);
    assert!(
        section().contains(&format!("```text\n{TABLE}```")),
        "the section no longer shows the 13 lines"
    );
    for (language, held, compared) in [
        (
            "Rust",
            include_str!("../check_tests/models.rs"),
            "pub const TABLE: &str",
        ),
        ("Rust", include_str!("../check_tests/table.rs"), "TABLE);"),
        ("TypeScript", TYPESCRIPT, "checked::TABLE,"),
        ("Dart", DART_CHECK, "assert_eq!(wrote, TABLE);"),
        ("Swift", SWIFT_CHECK, "assert_eq!(swift, expected);"),
        ("Kotlin", KOTLIN_CHECK, "assert_eq!(printed, TABLE);"),
    ] {
        assert!(
            held.contains(compared),
            "the {language} test no longer compares its run: `{compared}`"
        );
    }
    for (language, held) in [
        (
            "Rust and TypeScript",
            include_str!("../check_tests/models.rs"),
        ),
        ("Dart", DART_CHECK),
        ("Swift", SWIFT_CHECK),
        ("Kotlin", KOTLIN_CHECK),
    ] {
        assert!(
            held.contains(TABLE),
            "the {language} test no longer holds the 13 lines"
        );
    }
}

#[test]
fn the_six_payloads_are_the_ones_the_shared_table_is_built_from() {
    let shared = include_str!("../check_tests/table.rs");
    for (number, payload) in (1_u8..).zip(PAYLOADS) {
        assert!(
            section().contains(&format!("| {number} | `{payload}` |")),
            "the section no longer shows case {number}"
        );
        assert!(
            shared.contains(&format!("r#\"{payload}\"#,")),
            "the shared table no longer holds case {number}"
        );
    }
}

/// The documents say the key is a last resort in every place they introduce it: the section, the
/// section of the key, and the section of each client.
#[test]
fn every_place_that_introduces_a_preprocess_calls_it_a_last_resort() {
    assert!(section().contains("**`preprocess` is a last resort.** It is not encouraged."));
    for (introduced, said) in [
        (
            "### Preprocessing (`preprocess`)\n",
            "**The key is a last resort.**",
        ),
        (
            "#### A `preprocess` in the Dart client\n",
            "- **The key stays a last resort.**",
        ),
        (
            "#### `preprocess` in the Swift client\n",
            "**The key is a last resort in Swift as it is in Rust.**",
        ),
        (
            "#### `preprocess` in the Kotlin client\n",
            "**The key is a last resort** here as it is in Rust",
        ),
    ] {
        let rest = from_until(readme(), introduced, "\n### ");
        let own = rest.get(1..).unwrap().split("\n#### ").next().unwrap();
        assert!(own.contains(said), "{introduced}no longer says: {said}");
    }
}

/// Every section the section points at is a heading of the README.
#[test]
fn every_section_the_section_names_is_a_heading_of_the_readme() {
    let headings: Vec<String> = readme()
        .lines()
        .filter(|line| line.starts_with('#'))
        .map(anchor)
        .collect();
    let mut named = 0_u32;
    for link in section().split("](#").skip(1) {
        let target = link.split_once(')').unwrap().0;
        assert!(
            headings.iter().any(|heading| heading == target),
            "no heading of the README is linked by `#{target}`"
        );
        named += 1;
    }
    assert!(named >= 15, "the section names {named} sections");
}

/// The parcel of the section answers the 13 lines the parcel of the check models answers.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn the_parcel_of_the_section_answers_the_13_lines() {
    use core::fmt::Write as _;

    use super::models::{Manifest, Parcel};

    let parcel = |label: &str, item_code_list: &[&str], fragile_item_codes: &[&str]| {
        let texts = |written: &[&str]| written.iter().map(|&text| text.to_owned()).collect();
        Parcel {
            fragile_item_codes: texts(fragile_item_codes),
            item_code_list: texts(item_code_list),
            label: label.to_owned(),
        }
    };
    let mut printed = String::new();
    let mut print = |name: &str, answered: Result<(), Vec<String>>| match answered {
        Ok(()) => writeln!(printed, "{name}: passes").unwrap(),
        Err(messages) => {
            writeln!(printed, "{name}: refused").unwrap();
            for message in messages {
                writeln!(printed, "    {message}").unwrap();
            }
        }
    };
    let parcels = [
        parcel("A", &["x"], &["x"]),
        parcel("", &["x"], &["x"]),
        parcel("A", &["x", "x"], &["x"]),
        parcel("A", &["x"], &["x", "y"]),
        parcel("", &["x", "x"], &["y"]),
    ];
    for (number, held) in (1_u8..).zip(&parcels) {
        print(&format!("case {number}"), held.validate());
    }
    let manifest = Manifest {
        parcel: parcel("A", &["x"], &["x", "y"]),
    };
    print("case 6", manifest.validate());
    assert_eq!(printed, TABLE);
}

/// The two reads the section shows are what a run answers.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn the_readme_shows_what_the_read_of_the_parcel_answers() {
    use super::models::Parcel;

    let taken = r#"{"fragileItemCodes":["x"],"itemCodeList":["x"],"label":"  A  "}"#;
    let broken = r#"{"fragileItemCodes":["y"],"itemCodeList":["x","x"],"label":"   "}"#;
    let read: Parcel = serde_json::from_str(taken).unwrap();
    let refused = serde_json::from_str::<Parcel>(broken)
        .unwrap_err()
        .to_string();
    for shown in [
        format!(
            "serde_json::from_str::<Parcel>(r#\"{taken}\"#);\n// Ok: `label` is {:?}\n",
            read.label
        ),
        format!("serde_json::from_str::<Parcel>(r#\"{broken}\"#);\n// Err: {refused}\n"),
    ] {
        assert!(
            section().contains(&shown),
            "the section no longer shows:\n{shown}"
        );
    }
}
