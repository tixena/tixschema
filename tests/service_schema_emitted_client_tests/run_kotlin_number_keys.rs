//! A map keyed by a brand over a number or a flag, or by an alias of a number, read and written
//! back by the emitted Kotlin types compiled with `-Werror`, against serde, over each payload of
//! [`number_key_rows`](super::number_key_rows). A key serde refuses is refused, and a number or a
//! flag written between quotes as a value is refused.

#![cfg(feature = "kotlin")]

use core::fmt::Write as _;

use super::number_key_rows::{
    adjacent_pier_kotlin, anchor_kotlin, beacon_kotlin, burgee_kotlin, cutter_kotlin,
    depths_kotlin, dinghy_kotlin, gauge_kotlin, hull_kotlin, ketch_kotlin, lanyard_kotlin,
    moorings_kotlin, pennant_kotlin, pier_kotlin, rows, rudder_kotlin, shoal_kotlin, signal_kotlin,
    skiff_kotlin, slips_kotlin, sloop_kotlin, soundings_kotlin, yawl_kotlin,
};
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

/// Each payload is read and written back by the Kotlin read as serde reads and writes it.
#[test]
fn a_map_keyed_by_a_number_brand_or_alias_is_read_and_written_back_by_kotlin_as_serde_does() {
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
    assert!(expected.contains("refused"), "{expected}");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        adjacent_pier_kotlin::kotlin_definition(),
        beacon_kotlin::kotlin_definition(),
        rudder_kotlin::kotlin_definition(),
        anchor_kotlin::kotlin_definition(),
        burgee_kotlin::kotlin_definition(),
        cutter_kotlin::kotlin_definition(),
        depths_kotlin::kotlin_definition(),
        dinghy_kotlin::kotlin_definition(),
        gauge_kotlin::kotlin_definition(),
        hull_kotlin::kotlin_definition(),
        ketch_kotlin::kotlin_definition(),
        lanyard_kotlin::kotlin_definition(),
        moorings_kotlin::kotlin_definition(),
        pennant_kotlin::kotlin_definition(),
        pier_kotlin::kotlin_definition(),
        shoal_kotlin::kotlin_definition(),
        signal_kotlin::kotlin_definition(),
        skiff_kotlin::kotlin_definition(),
        slips_kotlin::kotlin_definition(),
        sloop_kotlin::kotlin_definition(),
        soundings_kotlin::kotlin_definition(),
        yawl_kotlin::kotlin_definition(),
        HARNESS.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected, "the Kotlin read is not serde's");
}
