//! An `ObjectId` in each position serde writes one, read and written back by the emitted Kotlin
//! types compiled with `-Werror` beside the `ObjectId` the README declares, under the default
//! `Json`, against serde: a key beside `$oid`, and a `$oid` that is not 24 hexadecimal digits in
//! lower case, are refused where serde refuses them, and a flattened one is read with
//! `ignoreUnknownKeys`, as serde reads past any other key.

#![cfg(all(feature = "kotlin", feature = "mongodb"))]

use core::fmt::Write as _;

use super::object_id_models::{
    belated_kotlin, bullae_kotlin, cachet_kotlin, countersigned_kotlin, crossed_kotlin,
    crossing_kotlin, docketing_kotlin, embossed_kotlin, endorsed_kotlin, endorsing_kotlin,
    enveloping_kotlin, enwafered_kotlin, folding_kotlin, late_sigil_kotlin, lettering_kotlin,
    notation_kotlin, relayed_kotlin, relaying_kotlin, rows, sigil_kotlin, sigiled_kotlin,
    sigillary_kotlin, sigillum_kotlin, stampage_kotlin, wafer_kotlin, wafered_kotlin,
    wafering_kotlin, waxing_kotlin,
};
use super::object_id_runs::{printed, readme_object_id};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// What prints a row: the value read and written back, or `refused`.
const TOLD: &str = r#"
fun <T> told(at: Int, serializer: KSerializer<T>, payload: String) {
    val told = try {
        "read " + Json.encodeToString(serializer, Json.decodeFromString(serializer, payload))
    } catch (refusal: SerializationException) {
        "refused"
    }
    println("$at: $told")
}
"#;

/// `json` as the text of a Kotlin raw string, where a `$` opens a template.
fn templated(json: &str) -> String {
    json.replace('$', "${'$'}")
}

/// Each payload of [`rows`] is read by the Kotlin read where the row states a client reads it, and
/// written back as serde writes it, and refused where it states none does.
#[test]
fn an_object_id_is_read_and_written_back_by_kotlin_as_serde_reads_it() {
    let mut main = String::from("fun main() {\n");
    let mut expected = String::new();
    for (at, row) in rows().iter().enumerate() {
        let _ = writeln!(
            main,
            "    told({at}, serializer<{}>(), \"\"\"{}\"\"\")",
            row.read_as,
            templated(row.json)
        );
        let _ = writeln!(
            expected,
            "{at}: {}",
            printed(row.kotlin, row.read_as, row.json)
        );
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        readme_object_id("kotlin"),
        notation_kotlin::kotlin_definition(),
        countersigned_kotlin::kotlin_definition(),
        sigil_kotlin::kotlin_definition(),
        sigillum_kotlin::kotlin_definition(),
        endorsed_kotlin::kotlin_definition(),
        endorsing_kotlin::kotlin_definition(),
        cachet_kotlin::kotlin_definition(),
        embossed_kotlin::kotlin_definition(),
        docketing_kotlin::kotlin_definition(),
        stampage_kotlin::kotlin_definition(),
        sigiled_kotlin::kotlin_definition(),
        sigillary_kotlin::kotlin_definition(),
        enveloping_kotlin::kotlin_definition(),
        waxing_kotlin::kotlin_definition(),
        relaying_kotlin::kotlin_definition(),
        wafer_kotlin::kotlin_definition(),
        lettering_kotlin::kotlin_definition(),
        folding_kotlin::kotlin_definition(),
        relayed_kotlin::kotlin_definition(),
        crossed_kotlin::kotlin_definition(),
        crossing_kotlin::kotlin_definition(),
        bullae_kotlin::kotlin_definition(),
        wafered_kotlin::kotlin_definition(),
        wafering_kotlin::kotlin_definition(),
        enwafered_kotlin::kotlin_definition(),
        belated_kotlin::kotlin_definition(),
        late_sigil_kotlin::kotlin_definition(),
        TOLD.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not serde's");
}
