//! An untagged enum over a bounded brand, a list of it, an optional one and a model, all declared
//! below the enum, chooses its variant in each emitted client as serde does: a payload that
//! breaks the bound of the brand goes on to the next variant. Run by Swift under
//! `-warnings-as-errors`, by Dart under the analyzer and the VM, and by Kotlin under `-Werror`.

#[cfg(feature = "dart")]
mod dart {
    use super::super::below_brand_models::{
        PAYLOADS, fastening_dart, finish_dart, gasket_dart, rivet_dart,
    };
    use super::expected;
    use core::fmt::Write as _;

    use crate::run_dart_validate::{dart, module};

    const DRIVER: &str = "
void told(int at, String payload) {
  try {
    print('$at: ${Fastening.fromJson(jsonDecode(payload)).runtimeType}');
  } on FormatException {
    print('$at: refused');
  }
}
";

    fn source() -> String {
        let mut main = String::from("void main() {\n");
        for (at, payload) in PAYLOADS.iter().enumerate() {
            let _ = writeln!(main, "  told({at}, r'''{payload}''');");
        }
        main.push_str("}\n");
        module(
            &[
                fastening_dart::dart_definition(),
                rivet_dart::dart_definition(),
                gasket_dart::dart_definition(),
                finish_dart::dart_definition(),
            ],
            &[DRIVER],
            &main,
        )
    }

    #[test]
    fn dart_chooses_the_variant_serde_chooses_with_the_brand_below() {
        let Some((ran, printed)) = dart("below-brands", &[], &source()) else {
            return;
        };
        assert!(ran, "{printed}");
        assert_eq!(
            printed,
            expected("Fastening"),
            "the Dart read is not serde's"
        );
    }

    #[test]
    fn the_dart_analyzer_passes_the_types_with_the_brand_below() {
        let Some((clean, said)) = dart("below-brands-analyze", &["analyze"], &source()) else {
            return;
        };
        assert!(clean, "{said}");
        assert!(said.contains("No issues found!"), "{said}");
    }
}

#[cfg(feature = "kotlin")]
mod kotlin {
    use super::super::below_brand_models::{
        PAYLOADS, fastening_kotlin, finish_kotlin, gasket_kotlin, rivet_kotlin,
    };
    use super::expected;
    use core::fmt::Write as _;

    use crate::runtime::ran_kotlin;

    const IMPORTS: &str = "import kotlinx.serialization.*\n\
         import kotlinx.serialization.json.*\n\
         import kotlinx.serialization.descriptors.*\n\
         import kotlinx.serialization.encoding.*\n\
         import kotlinx.serialization.builtins.*";

    const DRIVER: &str = r#"
fun told(at: Int, payload: String) {
    try {
        println("$at: " + Json.decodeFromString(serializer<Fastening>(), payload)::class.simpleName)
    } catch (refusal: SerializationException) {
        println("$at: refused")
    }
}
"#;

    #[test]
    fn kotlin_chooses_the_variant_serde_chooses_with_the_brand_below() {
        let mut main = String::from("fun main() {\n");
        for (at, payload) in PAYLOADS.iter().enumerate() {
            let _ = writeln!(main, "    told({at}, \"\"\"{payload}\"\"\")");
        }
        main.push_str("}\n");
        let source = [
            IMPORTS.to_owned(),
            fastening_kotlin::kotlin_definition(),
            rivet_kotlin::kotlin_definition(),
            gasket_kotlin::kotlin_definition(),
            finish_kotlin::kotlin_definition(),
            DRIVER.to_owned(),
            main,
        ]
        .join("\n\n");
        let Some(printed) = ran_kotlin(&source) else {
            return;
        };
        assert_eq!(
            printed,
            expected("Fastening"),
            "the Kotlin read is not serde's"
        );
    }
}

#[cfg(feature = "swift")]
mod swift {
    use super::super::below_brand_models::{
        PAYLOADS, fastening_swift, finish_swift, gasket_swift, rivet_swift,
    };
    use super::expected;
    use core::fmt::Write as _;

    use crate::runtime::judged;

    const DRIVER: &str = r#"
func told(_ at: Int, _ payload: String) {
  do {
    switch try JSONDecoder().decode(Fastening.self, from: Data(payload.utf8)) {
    case .anodized: print("\(at): Anodized")
    case .coded: print("\(at): Coded")
    case .listed: print("\(at): Listed")
    case .maybe: print("\(at): Maybe")
    case .whole: print("\(at): Whole")
    case .wordy: print("\(at): Wordy")
    }
  } catch {
    print("\(at): refused")
  }
}
"#;

    #[test]
    fn swift_chooses_the_variant_serde_chooses_with_the_brand_below() {
        let mut main = String::new();
        for (at, payload) in PAYLOADS.iter().enumerate() {
            let _ = writeln!(main, "told({at}, #\"{payload}\"#)");
        }
        let source = [
            "import Foundation".to_owned(),
            fastening_swift::swift_definition(),
            rivet_swift::swift_definition(),
            gasket_swift::swift_definition(),
            finish_swift::swift_definition(),
            DRIVER.to_owned(),
            main,
        ]
        .join("\n\n");
        let Some(run) = judged(
            "swift-below-brands",
            "TIXSCHEMA_SWIFT",
            "swift",
            &["-warnings-as-errors"],
            "main.swift",
            &source,
        ) else {
            return;
        };
        assert!(
            run.succeeded,
            "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}",
            run.stdout, run.stderr
        );
        assert_eq!(run.stdout, expected(""), "the Swift read is not serde's");
    }
}

use core::fmt::Write as _;

use super::below_brand_models;

/// What the Rust read takes of each payload, a line per payload, with the variant's name behind
/// `prefix`.
fn expected(prefix: &str) -> String {
    let mut lines = String::new();
    for (at, payload) in below_brand_models::PAYLOADS.iter().enumerate() {
        let taken = below_brand_models::rust_takes(payload);
        let named = if taken == "refused" {
            taken
        } else {
            format!("{prefix}{taken}")
        };
        let _ = writeln!(lines, "{at}: {named}");
    }
    lines
}
