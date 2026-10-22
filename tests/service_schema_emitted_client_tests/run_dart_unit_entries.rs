//! An externally tagged enum flattened beside a member of its holder's own, optional and under a
//! function too, each variant read in either order and written back by the emitted Dart types
//! under the Dart VM, against what serde reads and writes back: a unit variant is an entry of its
//! name over `null`. The enum read alone takes that entry too. Entries under two of its names are
//! refused, as TypeScript and Zod refuse them. A unit variant under a tag and a content key is read
//! from an object with no content key alone, and an enum under a tag and a content key flattened
//! into its holder takes both keys. The models are those the Kotlin runs are held to, compiled
//! with the `kotlin` feature.

#![cfg(all(feature = "dart", feature = "kotlin", feature = "serde"))]

use core::fmt::Write as _;

use super::flattened_models::{
    Belting, CHAFING, Chafing, FRAYING, Fraying, STOWING, Stowing, Tacking, UNDER_AN_ALIAS,
    UNDER_TWO_NAMES, Yawing, belting_dart, bunting_dart, chafing_dart, fraying_dart, read_of,
    snubber_dart, stowing_dart, tacking_dart, yawing_dart,
};
use super::run_dart_validate::{dart, from_until, module};
use super::run_kotlin_refusals::{Becalming, becalming_dart};
use super::tuple_variants_models::labelling_dart;

/// What prints a read of [`Becalming`]: the value read, written back, or what the read throws.
const BECALMING_TOLD: &str = "
void told(String payload) {
  String printed;
  try {
    printed = jsonEncode(Becalming.fromJson(jsonDecode(payload)).toJson());
  } catch (refusal) {
    printed = '$refusal';
  }
  print('$payload: $printed');
}
";

/// [`Becalming`]'s unit variant with no content, with `null` for one and with a value for one:
/// each payload, what serde reads of it written back or its refusal, and what the Dart read
/// answers of it.
const UNDER_A_CONTENT_KEY: [(&str, &str, &str); 3] = [
    (
        r#"{"kind":"Lull"}"#,
        r#"{"kind":"Lull"}"#,
        r#"{"kind":"Lull"}"#,
    ),
    (
        r#"{"kind":"Lull","held":null}"#,
        r#"{"kind":"Lull"}"#,
        "FormatException: Becalming: Lull takes no value",
    ),
    (
        r#"{"kind":"Lull","held":5}"#,
        "invalid type: integer `5`, expected unit variant Becalming::Lull at line 1 column 23",
        "FormatException: Becalming: Lull takes no value",
    ),
];

/// What prints a row: the value read, written back, or that the read is refused.
const TOLD: &str = "
void row(String name, String payload, String Function(dynamic) told) {
  String printed;
  try {
    printed = told(jsonDecode(payload));
  } catch (_) {
    printed = 'refused';
  }
  print('$name $payload: $printed');
}
";

/// The function [`Yawing`] names, which answers the payload as it was handed.
const AS_GIVEN: &str = "dynamic asGiven(dynamic payload) => payload;";

/// Each variant's entry ahead of the holder's own key and after it, a variant under its alias, and
/// a unit variant's entry over a value, which serde refuses.
const HELD: [&str; 9] = [
    r#"{"Bay":{"label":"n"},"note":"a"}"#,
    r#"{"note":"a","Bay":{"label":"n"}}"#,
    r#"{"Calm":null,"note":"a"}"#,
    r#"{"note":"a","Calm":null}"#,
    r#"{"Dock":{"label":"s"},"note":"a"}"#,
    r#"{"note":"a","Quay":{"label":"s"}}"#,
    r#"{"Held":"x","note":"a"}"#,
    r#"{"note":"a","Held":"x"}"#,
    r#"{"note":"a","Calm":1}"#,
];

/// A unit variant's entry over an empty object, which serde reads flattened and the Zod schema,
/// which takes the entry over `null` alone, refuses: the Dart read refuses it with the schema.
const OVER_AN_OBJECT: &str = r#"{"note":"a","Calm":{}}"#;

/// The enum read alone: a unit variant's name, its entry over `null`, and its entry over a value
/// and over an empty object, which serde refuses.
const ALONE: [&str; 4] = [
    r#""Calm""#,
    r#"{"Calm":null}"#,
    r#"{"Calm":1}"#,
    r#"{"Calm":{}}"#,
];

/// What prints a [`Stowing`] read from each payload and written back, or what its read throws.
const STOWING_TOLD: &str = "
void stowing(int at, String payload) {
  String printed;
  try {
    printed = jsonEncode(Stowing.fromJson(jsonDecode(payload)).toJson());
  } catch (refusal) {
    printed = '$refusal';
  }
  print('$at: $printed');
}
";

/// The function [`Chafing`] names, which answers the keys of the object it is handed.
const WITH_ITS_KEYS: &str = "dynamic withItsKeys(dynamic payload) => \
     {'keys': (payload as Map<String, dynamic>).keys.toList()};";

/// Each type a row reads, the payloads it reads, and what serde writes back of each.
fn rows() -> Vec<(&'static str, &'static str, String)> {
    let mut rows = Vec::new();
    let mut of = |named: &'static str, payloads: &[&'static str], serde: fn(&str) -> String| {
        rows.extend(
            payloads
                .iter()
                .map(|payload| (named, *payload, serde(payload))),
        );
    };
    of("Tacking", &ALONE, read_of::<Tacking>);
    of("Stowing", &HELD, read_of::<Stowing>);
    // serde reads the entry over a value as no enum where the enum is optional, and the Zod schema
    // refuses it.
    of("Belting", &HELD[..8], read_of::<Belting>);
    of("Yawing", &HELD, read_of::<Yawing>);
    rows
}

/// serde reads a flattened unit variant from its entry over `null` in either order, and writes it
/// as that entry; it refuses the entry over a value, and reads it over an empty object.
#[test]
fn serde_writes_and_reads_a_flattened_unit_variant_as_its_entry_over_null() {
    assert_eq!(
        [read_of::<Stowing>(HELD[2]), read_of::<Stowing>(HELD[3])],
        [r#"{"note":"a","Calm":null}"#; 2],
        "serde reads the entry in either order and writes it"
    );
    assert_eq!(
        read_of::<Stowing>(HELD[8]),
        "refused",
        "serde refuses the entry over a value"
    );
    assert_eq!(
        read_of::<Stowing>(OVER_AN_OBJECT),
        r#"{"note":"a","Calm":null}"#,
        "serde reads the entry over an empty object"
    );
    assert_eq!(
        ALONE.map(read_of::<Tacking>),
        [r#""Calm""#, r#""Calm""#, "refused", "refused"],
        "serde reads the enum alone from the name and from the entry over null"
    );
}

/// Each variant of a flattened enum is read in Dart from its entry in either order, a unit
/// variant's over `null`, and written back as serde writes it back, in a holder of the enum,
/// of the enum optional and of the enum under a function. The enum read alone takes a unit
/// variant's entry over `null`. Each read refuses the entry over a value where serde refuses it,
/// and over an empty object, as the Zod schema refuses it.
#[test]
fn a_flattened_unit_variant_is_written_and_read_in_dart_as_serde_writes_and_reads_it() {
    let mut expected = String::new();
    let mut main = String::from("void main() {\n");
    for (named, payload, serde) in rows() {
        let _ = writeln!(expected, "{named} {payload}: {serde}");
        let _ = writeln!(
            main,
            "  row('{named}', r'{payload}', (json) => jsonEncode({named}.fromJson(json).toJson()));"
        );
    }
    let _ = writeln!(expected, "Stowing {OVER_AN_OBJECT}: refused");
    let _ = writeln!(
        main,
        "  row('Stowing', r'{OVER_AN_OBJECT}', (json) => jsonEncode(Stowing.fromJson(json).toJson()));"
    );
    main.push_str("}\n");
    let source = module(
        &[
            tacking_dart::dart_definition(),
            stowing_dart::dart_definition(),
            belting_dart::dart_definition(),
            yawing_dart::dart_definition(),
        ],
        &[AS_GIVEN, TOLD],
        &main,
    );
    let Some((ran, printed)) = dart("unit-entries", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(
        printed, expected,
        "the Dart reads and writes beside serde's"
    );
}

/// A unit variant under a tag and a content key is read in Dart where the object holds no content
/// key, and an object that holds one is refused with `FormatException`, `null` among its values,
/// as Zod refuses it: its schema of the variant is a strict object of the tag alone. serde reads
/// the variant over `null`.
#[test]
fn a_unit_variant_under_a_content_key_is_read_in_dart_where_the_object_holds_no_content() {
    let serde_told = UNDER_A_CONTENT_KEY.map(|(json, _, _)| {
        serde_json::from_str::<Becalming>(json).map_or_else(
            |refusal| refusal.to_string(),
            |read| serde_json::to_string(&read).unwrap(),
        )
    });
    assert_eq!(
        serde_told,
        UNDER_A_CONTENT_KEY.map(|(_, serde, _)| serde.to_owned()),
        "serde no longer answers each payload as it did"
    );
    let zod = Becalming::zod_schema();
    assert!(
        zod.contains("z.strictObject({\n  kind: z.literal(\"Lull\"),\n})"),
        "Zod no longer reads the unit variant as an object of its tag alone: {zod}"
    );
    let mut expected = String::new();
    let mut main = String::from("void main() {\n");
    for (json, _, dart) in UNDER_A_CONTENT_KEY {
        let _ = writeln!(expected, "{json}: {dart}");
        let _ = writeln!(main, "  told(r'{json}');");
    }
    main.push_str("}\n");
    let source = module(
        &[becalming_dart::dart_definition()],
        &[BECALMING_TOLD],
        &main,
    );
    let Some((ran, printed)) = dart("unit-under-a-content-key", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not the answer above");
}

/// The README's sentence on the Dart read of a unit variant under a content key shows the payloads
/// and the refusal the run above measures.
#[test]
fn the_readme_shows_the_read_of_a_unit_variant_under_a_content_key() {
    let section = from_until(
        include_str!("../../README.md"),
        "#### The Dart read refuses what `validate()` refuses",
        "#### What stays yours",
    );
    for (json, _, dart) in UNDER_A_CONTENT_KEY {
        assert!(section.contains(json), "not in the README: {json}");
        assert!(
            dart.starts_with('{') || section.contains(&format!("`{dart}`")),
            "not in the README: {dart}"
        );
    }
}

/// The analyzer passes the enums and the holders.
#[test]
fn the_emitted_holders_of_a_flattened_unit_variant_have_no_issue() {
    let source = module(
        &[
            tacking_dart::dart_definition(),
            stowing_dart::dart_definition(),
            belting_dart::dart_definition(),
            yawing_dart::dart_definition(),
            becalming_dart::dart_definition(),
        ],
        &[AS_GIVEN],
        "void main() => print(jsonEncode(Stowing(note: 'a', tacking: TackingCalm()).toJson()));\n",
    );
    let Some((clean, said)) = dart("unit-entries-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}

/// A flattened externally tagged enum with entries under two of its names is refused, as
/// TypeScript and Zod refuse it, where serde reads the first, under two variants' names in each
/// order and under a variant's name beside its alias, naming each variant once, in the order the
/// enum declares them. One entry, a unit variant's over `null`, an entry under an alias and no
/// entry are read and refused as serde reads and refuses them.
#[test]
fn a_flattened_enum_with_entries_under_two_names_is_refused_in_dart() {
    let refused = |variants: &str| {
        format!(
            "FormatException: more than one variant of enum Tacking found in flattened data: \
             {variants}"
        )
    };
    let mut rows: Vec<(&str, String)> = UNDER_TWO_NAMES
        .iter()
        .zip([refused("Bay, Dock"), refused("Bay, Dock"), refused("Dock")])
        .map(|(&json, told)| {
            assert!(
                serde_json::from_str::<Stowing>(json).is_ok(),
                "serde reads the first entry of {json}"
            );
            (json, told)
        })
        .collect();
    for json in [STOWING[1], STOWING[4], UNDER_AN_ALIAS[0]] {
        rows.push((json, read_of::<Stowing>(json)));
    }
    assert_eq!(
        read_of::<Stowing>(STOWING[6]),
        "refused",
        "serde refuses a payload with no entry"
    );
    let mut expected = String::new();
    let mut main = String::from("void main() {\n");
    for (at, (json, told)) in rows.iter().enumerate() {
        let _ = writeln!(expected, "{at}: {told}");
        let _ = writeln!(main, "  stowing({at}, r'{json}');");
    }
    main.push_str("}\n");
    assert_eq!(
        expected,
        "0: FormatException: more than one variant of enum Tacking found in flattened data: Bay, \
         Dock\n\
         1: FormatException: more than one variant of enum Tacking found in flattened data: Bay, \
         Dock\n\
         2: FormatException: more than one variant of enum Tacking found in flattened data: Dock\n\
         3: {\"note\":\"a\",\"Bay\":{\"label\":\"n\"}}\n\
         4: {\"note\":\"a\",\"Calm\":null}\n\
         5: {\"note\":\"a\",\"Dock\":{\"label\":\"s\"}}\n"
    );
    let source = module(
        &[
            tacking_dart::dart_definition(),
            stowing_dart::dart_definition(),
        ],
        &[STOWING_TOLD],
        &main,
    );
    let Some((ran, printed)) = dart("under-two-names", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not the answer above");
}

/// An enum under a tag and a content key flattened into its holder takes both keys, as serde reads
/// it as a struct of the two: a member that names a function flattened after it is handed every
/// other key, and a struct flattened after it reads the whole object. Each holder is written back
/// as serde writes it back.
#[test]
fn an_enum_under_a_tag_and_a_content_key_takes_both_keys_in_dart() {
    let chafing: Chafing = serde_json::from_str(CHAFING).unwrap();
    let fraying: Fraying = serde_json::from_str(FRAYING[0]).unwrap();
    let expected = format!(
        "chafing: {} {}\nfraying: {} {}\n",
        serde_json::to_string(&chafing.tally).unwrap(),
        serde_json::to_string(&chafing).unwrap(),
        serde_json::to_string(&fraying.labelling).unwrap(),
        serde_json::to_string(&fraying).unwrap(),
    );
    assert_eq!(
        expected,
        "chafing: {\"keys\":[\"north\"]} {\"kind\":\"Rope\",\"held\":{\"label\":\"n\"},\"keys\":[\"north\"]}\n\
         fraying: {\"label\":\"m\"} {\"kind\":\"Rope\",\"held\":{\"label\":\"n\"},\"label\":\"m\"}\n"
    );
    let main = format!(
        "void main() {{\n  final chafing = Chafing.fromJson(jsonDecode(r'{CHAFING}'));\n  \
         print('chafing: ${{jsonEncode(chafing.tally.toJson())}} \
         ${{jsonEncode(chafing.toJson())}}');\n  final fraying = \
         Fraying.fromJson(jsonDecode(r'{}'));\n  print('fraying: \
         ${{jsonEncode(fraying.labelling.toJson())}} ${{jsonEncode(fraying.toJson())}}');\n}}\n",
        FRAYING[0]
    );
    let source = module(
        &[
            labelling_dart::dart_definition(),
            bunting_dart::dart_definition(),
            snubber_dart::dart_definition(),
            chafing_dart::dart_definition(),
            fraying_dart::dart_definition(),
        ],
        &[WITH_ITS_KEYS],
        &main,
    );
    let Some((ran, printed)) = dart("tag-and-content", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not serde's");
}
