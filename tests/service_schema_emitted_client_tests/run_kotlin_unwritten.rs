//! A member serde reads and never writes, read and written back by the emitted Kotlin types
//! compiled with `-Werror`, in a struct of its own and beside a flattened member, against what
//! serde reads and writes back.

#![cfg(feature = "kotlin")]

use super::runtime::ran_kotlin;
use super::unwritten_models::{PAYLOAD, WRITTEN, braiding_kotlin, furling_kotlin, reefing_kotlin};

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
