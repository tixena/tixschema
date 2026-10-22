//! A member serde reads and never writes, read and written back by the emitted Kotlin types
//! compiled with `-Werror`, in a struct of its own, beside a flattened member and in an enum's
//! variant under each tagging that writes a variant's named members, against what serde reads and
//! writes back.

#![cfg(feature = "kotlin")]

use super::runtime::ran_kotlin;
use super::unwritten_models::{
    FLATTENED, PAYLOAD, REFUSED, WRITTEN, braiding_kotlin, flemishing_kotlin, furling_kotlin,
    ranging_kotlin, reefing_kotlin, splaying_kotlin,
};
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

/// What prints each struct of [`FLATTENED`] read from its payload: what its flattened member read
/// holds, and the value written back, and whether each payload of [`REFUSED`] is refused.
const FLATTENED_MAIN: &str = r#"
fun main() {
    val flemishing = Json.decodeFromString(Flemishing.serializer(), """PAYLOAD_FLEMISHING""")
    println("Flemishing: read ${flemishing.braiding?.label}, written ${Json.encodeToString(Flemishing.serializer(), flemishing)}")
    val splaying = Json.decodeFromString(Splaying.serializer(), """PAYLOAD_SPLAYING""")
    println("Splaying: read ${splaying.braiding.label}, written ${Json.encodeToString(Splaying.serializer(), splaying)}")
    val ranging = Json.decodeFromString(Ranging.serializer(), """PAYLOAD_RANGING""")
    println("Ranging: read ${ranging.counts["north"]}, written ${Json.encodeToString(Ranging.serializer(), ranging)}")
    println("Splaying refused: ${runCatching { Json.decodeFromString(Splaying.serializer(), """REFUSED_SPLAYING""") }.isFailure}")
    println("Ranging refused: ${runCatching { Json.decodeFromString(Ranging.serializer(), """REFUSED_RANGING""") }.isFailure}")
}
"#;

/// [`FLATTENED_MAIN`] with each payload of [`FLATTENED`] and [`REFUSED`] in its place.
fn flattened_main() -> String {
    FLATTENED_MAIN
        .replace("PAYLOAD_FLEMISHING", FLATTENED[0].1)
        .replace("PAYLOAD_SPLAYING", FLATTENED[1].1)
        .replace("PAYLOAD_RANGING", FLATTENED[2].1)
        .replace("REFUSED_SPLAYING", REFUSED[0].1)
        .replace("REFUSED_RANGING", REFUSED[1].1)
}

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

/// A flattened struct and a flattened map serde never writes are read, and written as serde
/// writes them: never.
#[test]
fn a_flattened_member_serde_never_writes_is_read_and_not_written_in_kotlin() {
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        braiding_kotlin::kotlin_definition(),
        flemishing_kotlin::kotlin_definition(),
        splaying_kotlin::kotlin_definition(),
        ranging_kotlin::kotlin_definition(),
        flattened_main(),
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(
        printed,
        format!(
            "Flemishing: read north, written {}\nSplaying: read north, written {}\nRanging: \
             read 2, written {}\nSplaying refused: true\nRanging refused: true\n",
            FLATTENED[0].2, FLATTENED[1].2, FLATTENED[2].2
        ),
        "each flattened member is read and not written"
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
