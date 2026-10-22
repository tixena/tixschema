//! A member serde reads and never writes, read and written back by the emitted Kotlin types
//! compiled with `-Werror`, in a struct of its own, beside a flattened member and in an enum's
//! variant under each tagging that writes a variant's named members, against what serde reads and
//! writes back.

#![cfg(feature = "kotlin")]

use super::runtime::ran_kotlin;
use super::unwritten_models::{PAYLOAD, WRITTEN, braiding_kotlin, furling_kotlin, reefing_kotlin};
#[cfg(feature = "serde")]
use super::unwritten_models::{
    VARIANTS, dousing_kotlin, ratcheting_kotlin, shackling_kotlin, wearing_kotlin,
};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints each struct read from the payload: the member read, and the value written back.
const MAIN: &str = r#"
fun main() {
    val furling = Json.decodeFromString(Furling.serializer(), """PAYLOAD""")
    println("in a struct of its own: read ${furling.spare}, written ${Json.encodeToString(Furling.serializer(), furling)}")
    val reefing = Json.decodeFromString(Reefing.serializer(), """PAYLOAD""")
    println("beside a flattened member: read ${reefing.spare}, written ${Json.encodeToString(Reefing.serializer(), reefing)}")
}
"#;

/// Each struct reads the member and writes it back as serde does: never.
#[test]
fn a_member_serde_never_writes_is_read_and_not_written_in_kotlin() {
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        braiding_kotlin::kotlin_definition(),
        furling_kotlin::kotlin_definition(),
        reefing_kotlin::kotlin_definition(),
        MAIN.replace("PAYLOAD", PAYLOAD),
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(
        printed,
        format!(
            "in a struct of its own: read 3, written {WRITTEN}\n\
             beside a flattened member: read 3, written {WRITTEN}\n"
        )
    );
}

/// What prints each enum of [`VARIANTS`] read from its payload: the member of its variant
/// `Furled` read, and the value written back.
#[cfg(feature = "serde")]
fn variants_main() -> String {
    let lines: Vec<String> = VARIANTS
        .iter()
        .map(|(name, payload, _)| {
            format!(
                "    val read{name} = Json.decodeFromString({name}.serializer(), \"\"\"{payload}\"\"\")\n    \
                 println(\"{name}: read ${{(read{name} as {name}Furled).spare}}, written \
                 ${{Json.encodeToString({name}.serializer(), read{name})}}\")"
            )
        })
        .collect();
    format!("fun main() {{\n{}\n}}\n", lines.join("\n"))
}

/// Each enum reads the member of its variant and writes it back as serde does: never.
#[cfg(feature = "serde")]
#[test]
fn a_variants_member_serde_never_writes_is_read_and_not_written_in_kotlin() {
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        dousing_kotlin::kotlin_definition(),
        wearing_kotlin::kotlin_definition(),
        shackling_kotlin::kotlin_definition(),
        ratcheting_kotlin::kotlin_definition(),
        variants_main(),
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    let expected = VARIANTS
        .iter()
        .map(|(name, _, written)| format!("{name}: read 3, written {written}\n"))
        .collect::<Vec<_>>()
        .concat();
    assert_eq!(printed, expected);
}
