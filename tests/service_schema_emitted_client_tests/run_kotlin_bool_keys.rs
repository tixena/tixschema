//! A map keyed by a `bool` read and written back by the emitted Kotlin types compiled with
//! `-Werror`, against serde, over each payload of [`bool_key_rows`](super::bool_key_rows).

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::bool_key_rows::{adjacent_drawer_kotlin, cubby_kotlin, drawer_kotlin, rows};
use super::runtime::ran_kotlin;

/// The imports the emitted Kotlin and its driver reach for.
const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read and written back, or `refused`.
const HARNESS: &str = r#"
fun <T> told(at: String, serializer: KSerializer<T>, payload: String) {
    val printed = try {
        "read ${Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))}"
    } catch (refused: SerializationException) {
        "refused"
    }
    println("$at: $printed")
}
"#;

/// Each payload is read and written back by the Kotlin read as serde reads and writes it, and a
/// key that is neither `true` nor `false` is refused in every position.
#[test]
fn a_map_keyed_by_bool_is_read_and_written_back_by_kotlin_as_serde_does() {
    let rows = rows();
    let mut main = String::from("fun main() {\n");
    let mut expected = String::new();
    for (at, row) in rows.iter().enumerate() {
        let _ = writeln!(
            main,
            "    told(\"{at}\", serializer<{}>(), \"\"\"{}\"\"\")",
            row.named, row.json
        );
        let _ = writeln!(expected, "{at}: {}", row.serde);
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        cubby_kotlin::kotlin_definition(),
        drawer_kotlin::kotlin_definition(),
        adjacent_drawer_kotlin::kotlin_definition(),
        HARNESS.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not serde's");
}
