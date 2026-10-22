//! Models whose names on the wire hold a `$`, a quote, a double quote and a backslash, read and
//! written back by the emitted Kotlin types compiled with `-Werror`, against what serde writes: a
//! struct's members, its own tag key and its name, an alias of a member and of a variant, an
//! enum's variants under each tagging, its tag key and its content key, a plain enum's values, and
//! a struct and an externally tagged enum flattened beside a member of their holder's own. A
//! struct's tag of another value, or none, is refused in words that name the key and the tag as
//! serde writes them. The models are declared where the Dart client is built.

#![cfg(all(feature = "kotlin", feature = "dart", feature = "serde"))]

use core::fmt::Write as _;

use super::quoted_models::{
    UNDER_AN_ALIAS, WRITTEN, dutied_kotlin, excised_kotlin, leveed_kotlin, levied_kotlin,
    mulcted_kotlin, serde_written, tariffed_kotlin, tithed_kotlin, tolled_kotlin,
};
use super::run_dart_quoted::TARIFFED;
use super::run_dart_validate::from_until;
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read, written back, or the first line of the message of the
/// `SerializationException` the read throws.
const TOLD: &str = r#"
fun <T> told(at: Int, serializer: KSerializer<T>, payload: String) {
    val printed = try {
        "read " + Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))
    } catch (refused: SerializationException) {
        "refused: " + refused.message?.lineSequence()?.first()
    }
    println("$at: $printed")
}
"#;

/// The payload serde writes that the Kotlin read refuses, and what it answers: a newtype variant
/// over a struct under a tag alone, which the Kotlin read takes under a `value` key, where serde
/// writes the struct's members beside the tag. The names in it are read as written.
const PARTED: (&str, &str) = (
    r#"{"ki'nd":"He\"ld","cost$usd":"c"}"#,
    "refused: Encountered an unknown key 'cost$usd' at path: $",
);

/// A struct's payload with no tag, and with a tag of another value: each refused by the Kotlin
/// read, as TypeScript and Zod refuse it, where serde reads past the tag.
const REFUSED: [(&str, &str); 2] = [
    (
        r#"{"back\\slash":"b","cost$usd":"c","say \"hi\"":"s"}"#,
        "refused: missing field `ki$nd`",
    ),
    (
        r#"{"ki$nd":"Other","back\\slash":"b","cost$usd":"c","say \"hi\"":"s"}"#,
        r#"refused: expected tag "Price's$Tag\", found "Other""#,
    ),
];

/// The call that prints the row `at`: `json` read as `read_as`, between the quotes of a raw Kotlin
/// string, which reads `$` as a template.
fn told(at: usize, read_as: &str, json: &str) -> String {
    format!(
        "    told({at}, serializer<{read_as}>(), \"\"\"{}\"\"\")\n",
        json.replace('$', "${'$'}")
    )
}

/// Each payload serde writes is read by the Kotlin read and written back as serde wrote it, each
/// payload under an alias is written back under the name serde writes, and a struct's tag of
/// another value, or none, is refused in words that name the key and the tag as serde writes them.
#[test]
fn a_name_that_holds_a_quote_is_read_and_written_back_as_serde_writes_it() {
    let mut main = String::from("fun main() {\n");
    let mut expected = String::new();
    let mut at = 0_usize;
    for (read_as, json) in WRITTEN {
        main.push_str(&told(at, read_as, json));
        let written = serde_written(read_as, json);
        if json == PARTED.0 {
            let _ = writeln!(expected, "{at}: {}", PARTED.1);
        } else {
            let _ = writeln!(expected, "{at}: read {written}");
        }
        at += 1;
    }
    for (read_as, json, back) in UNDER_AN_ALIAS {
        assert_eq!(
            serde_written(read_as, json),
            back,
            "serde no longer reads {json} as {back}"
        );
        main.push_str(&told(at, read_as, json));
        let _ = writeln!(expected, "{at}: read {back}");
        at += 1;
    }
    for (json, refused) in REFUSED {
        main.push_str(&told(at, "Tariffed", json));
        let _ = writeln!(expected, "{at}: {refused}");
        at += 1;
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        tariffed_kotlin::kotlin_definition(),
        tolled_kotlin::kotlin_definition(),
        leveed_kotlin::kotlin_definition(),
        levied_kotlin::kotlin_definition(),
        dutied_kotlin::kotlin_definition(),
        excised_kotlin::kotlin_definition(),
        tithed_kotlin::kotlin_definition(),
        mulcted_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not serde's");
}

/// The README's sentence on a name that holds a quote declares the struct the run reads, and shows
/// a property it declares and the payload it reads.
#[test]
fn the_readme_shows_the_payload_the_run_reads() {
    let section = from_until(
        include_str!("../../README.md"),
        "#### The read in the Kotlin client",
        "#### `preprocess` in the Kotlin client",
    );
    let declared = r#"@SerialName("cost\$usd") val cost: String"#;
    assert!(
        tariffed_kotlin::kotlin_definition().contains(declared),
        "the struct no longer declares {declared}"
    );
    let (read_as, json) = WRITTEN[0];
    assert_eq!(
        read_as, "Tariffed",
        "the first row no longer reads the struct"
    );
    for shown in [TARIFFED, declared, json] {
        assert!(
            section.contains(&format!("`{shown}`")),
            "not in the README: {shown}"
        );
    }
}
