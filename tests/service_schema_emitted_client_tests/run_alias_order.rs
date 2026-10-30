//! An untagged enum over an alias of a string chooses its variant in each emitted client as serde
//! does: with the alias declared above the enum in Swift, Dart and Kotlin, and declared below it
//! in Dart and Kotlin. Run by Swift under `-warnings-as-errors`, by Dart under the analyzer and the
//! VM, and by Kotlin under `-Werror`.

#[cfg(feature = "dart")]
mod dart {
    use super::super::alias_order_models::{
        Binding, PAYLOADS, Tethering, binding_dart, cinch_dart, strap_dart, tethering_dart,
    };
    use super::expected;
    use core::fmt::Write as _;

    use crate::run_dart_validate::{dart, module};

    fn source(held: &str, definitions: &[String]) -> String {
        let driver = format!(
            "
void told(int at, String payload) {{
  try {{
    print('$at: ${{{held}.fromJson(jsonDecode(payload)).runtimeType}}');
  }} on FormatException {{
    print('$at: refused');
  }}
}}
"
        );
        let mut main = String::from("void main() {\n");
        for (at, payload) in PAYLOADS.iter().enumerate() {
            let _ = writeln!(main, "  told({at}, r'''{payload}''');");
        }
        main.push_str("}\n");
        module(definitions, &[&driver], &main)
    }

    fn above() -> String {
        source(
            "Tethering",
            &[
                strap_dart::dart_definition(),
                tethering_dart::dart_definition(),
            ],
        )
    }

    fn below() -> String {
        source(
            "Binding",
            &[
                binding_dart::dart_definition(),
                cinch_dart::dart_definition(),
            ],
        )
    }

    #[test]
    fn dart_chooses_the_variant_serde_chooses_with_the_alias_above() {
        let Some((ran, printed)) = dart("alias-above", &[], &above()) else {
            return;
        };
        assert!(ran, "{printed}");
        assert_eq!(
            printed,
            expected::<Tethering>("Tethering"),
            "the Dart read is not serde's"
        );
    }

    #[test]
    fn dart_chooses_the_variant_serde_chooses_with_the_alias_below() {
        let Some((ran, printed)) = dart("alias-below", &[], &below()) else {
            return;
        };
        assert!(ran, "{printed}");
        assert_eq!(
            printed,
            expected::<Binding>("Binding"),
            "the Dart read is not serde's"
        );
    }

    #[test]
    fn the_dart_analyzer_passes_the_types_with_the_alias_above_and_below() {
        for (named, source) in [
            ("alias-above-analyze", above()),
            ("alias-below-analyze", below()),
        ] {
            let Some((clean, said)) = dart(named, &["analyze"], &source) else {
                return;
            };
            assert!(clean, "{said}");
            assert!(said.contains("No issues found!"), "{said}");
        }
    }
}

#[cfg(feature = "kotlin")]
mod kotlin {
    use super::super::alias_order_models::{
        Binding, PAYLOADS, Tethering, binding_kotlin, cinch_kotlin, strap_kotlin, tethering_kotlin,
    };
    use super::expected;
    use core::fmt::Write as _;

    use crate::runtime::ran_kotlin;

    const IMPORTS: &str = "import kotlinx.serialization.*\n\
         import kotlinx.serialization.json.*\n\
         import kotlinx.serialization.descriptors.*\n\
         import kotlinx.serialization.encoding.*\n\
         import kotlinx.serialization.builtins.*";

    fn run(held: &str, definitions: [String; 2]) -> Option<String> {
        let driver = format!(
            "\nfun told(at: Int, payload: String) {{\n    try {{\n        \
             println(\"$at: \" + Json.decodeFromString(serializer<{held}>(), payload)::class.simpleName)\n    \
             }} catch (refusal: SerializationException) {{\n        println(\"$at: refused\")\n    }}\n}}\n"
        );
        let mut main = String::from("fun main() {\n");
        for (at, payload) in PAYLOADS.iter().enumerate() {
            let _ = writeln!(main, "    told({at}, \"\"\"{payload}\"\"\")");
        }
        main.push_str("}\n");
        let [first, second] = definitions;
        ran_kotlin(&[IMPORTS.to_owned(), first, second, driver, main].join("\n\n"))
    }

    #[test]
    fn kotlin_chooses_the_variant_serde_chooses_with_the_alias_above() {
        let Some(printed) = run(
            "Tethering",
            [
                strap_kotlin::kotlin_definition(),
                tethering_kotlin::kotlin_definition(),
            ],
        ) else {
            return;
        };
        assert_eq!(
            printed,
            expected::<Tethering>("Tethering"),
            "the Kotlin read is not serde's"
        );
    }

    #[test]
    fn kotlin_chooses_the_variant_serde_chooses_with_the_alias_below() {
        let Some(printed) = run(
            "Binding",
            [
                binding_kotlin::kotlin_definition(),
                cinch_kotlin::kotlin_definition(),
            ],
        ) else {
            return;
        };
        assert_eq!(
            printed,
            expected::<Binding>("Binding"),
            "the Kotlin read is not serde's"
        );
    }
}

#[cfg(feature = "swift")]
mod swift {
    use super::super::alias_order_models::{PAYLOADS, Tethering, strap_swift, tethering_swift};
    use super::expected;
    use core::fmt::Write as _;

    use crate::runtime::judged;

    const DRIVER: &str = r#"
func told(_ at: Int, _ payload: String) {
  do {
    switch try JSONDecoder().decode(Tethering.self, from: Data(payload.utf8)) {
    case .strapped: print("\(at): Strapped")
    case .tallied: print("\(at): Tallied")
    }
  } catch {
    print("\(at): refused")
  }
}
"#;

    #[test]
    fn swift_chooses_the_variant_serde_chooses_with_the_alias_above() {
        let mut main = String::new();
        for (at, payload) in PAYLOADS.iter().enumerate() {
            let _ = writeln!(main, "told({at}, #\"{payload}\"#)");
        }
        let source = [
            "import Foundation".to_owned(),
            strap_swift::swift_definition(),
            tethering_swift::swift_definition(),
            DRIVER.to_owned(),
            main,
        ]
        .join("\n\n");
        let Some(run) = judged(
            "swift-alias-order",
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
        assert_eq!(
            run.stdout,
            expected::<Tethering>(""),
            "the Swift read is not serde's"
        );
    }
}

use core::fmt::{Debug, Write as _};

use serde::de::DeserializeOwned;

use super::alias_order_models;

/// What the Rust read takes of each payload as `Held`, a line per payload, with the variant's name
/// behind `prefix`.
fn expected<Held>(prefix: &str) -> String
where
    Held: DeserializeOwned + Debug,
{
    let mut lines = String::new();
    for (at, payload) in alias_order_models::PAYLOADS.iter().enumerate() {
        let taken = alias_order_models::rust_takes::<Held>(payload);
        let named = if taken == "refused" {
            taken
        } else {
            format!("{prefix}{taken}")
        };
        let _ = writeln!(lines, "{at}: {named}");
    }
    lines
}
