//! Each limit of [`ROWS`] held by the emitted Kotlin types compiled with `-Werror`: `validate()` of
//! the value read with no judgement, and the read that refuses with what `validate()` answers.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::integer_limit_models::{
    REPORT, ROWS, limit_ceiling_kotlin, limit_ends_kotlin, limit_floor_kotlin, limit_forms_kotlin,
    limit_mixed_kotlin, limit_wide_kotlin,
};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: `passes` or each message of `validate()`, and a line more where the read
/// does not answer what `validate()` answered.
const REPORTER: &str = r#"
fun report(name: String, answered: List<String>, read: () -> Any) {
    if (answered.isEmpty()) {
        println("$name: passes")
    } else {
        println("$name: refused")
        for (message in answered) println("    $message")
    }
    try {
        read()
        if (answered.isNotEmpty()) println("    the read passed it")
    } catch (refused: SerializationException) {
        if (refused.message != answered.joinToString("; ")) println("    the read said ${refused.message}")
    }
}
"#;

#[test]
fn each_limit_holds_in_kotlin_as_it_is_written() {
    let mut main = String::from("fun main() {\n");
    for (name, read_as, json) in ROWS {
        let _ = writeln!(
            main,
            "    report(\"{name}\", Json.decodeFromString({read_as}Unjudged, \"\"\"{json}\"\"\").validationIssues()) {{ Json.decodeFromString(serializer<{read_as}>(), \"\"\"{json}\"\"\") }}"
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        limit_ceiling_kotlin::kotlin_definition(),
        limit_floor_kotlin::kotlin_definition(),
        limit_forms_kotlin::kotlin_definition(),
        limit_mixed_kotlin::kotlin_definition(),
        limit_ends_kotlin::kotlin_definition(),
        limit_wide_kotlin::kotlin_definition(),
        REPORTER.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, REPORT);
}
