//! A flattened `Option` member beside a member of its holder's own, alone in its holder and in a
//! variant under a tag key, a struct of two keys, a struct declared below its holder, a struct
//! flattened bare beside a member of its holder's own, a model `validate()` walks, a struct that
//! reads the keys no member around it took, and a struct read from what a function answers, each
//! read and written back by the emitted Dart types under the Dart VM, against what serde reads and
//! writes back. A flattened `Option` is `null` where the object holds none of its type's keys, as
//! serde reads it. Where the object holds a key of the type that does not read, serde reads `None`
//! and the Dart read refuses the payload, as TypeScript and Zod refuse it. A type whose keys are
//! not known is read as `null` wherever it does not read, as serde reads it.

#![cfg(all(feature = "dart", feature = "serde"))]

use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::run_dart_validate::{dart, module};
use super::swift_flattened_options_models::{
    Crowning, FLAKING, FLAKING_ABSENT, FLAKING_MISTYPED, Fidding, Flaking, KINKING_SHORT, Kinking,
    LAPPING, LAPPING_ABSENT, LAPPING_MISTYPED, Lapping, PAINTER, Painter, Ratline, SENNIT,
    SENNIT_ABSENT, SENNIT_MISTYPED, Sennit, TARRING_SHORT, Tarring, Veering, crowning_dart,
    fidding_dart, flaking_dart, gasket_dart, kinking_dart, lapping_dart, nippering_dart,
    painter_dart, plaiting_dart, ratline_dart, sennit_dart, tabling_dart, tarring_dart,
    veering_dart,
};

/// The function [`Veering`] names, which answers the payload as it was handed.
const AS_GIVEN: &str = "dynamic asGiven(dynamic payload) => payload;";

/// What prints a row: the value read, as the row tells it, or what the read says as it refuses
/// the payload.
const TOLD: &str = "
void row(String name, String Function() read) {
  String told;
  try {
    told = read();
  } on FormatException catch (refused) {
    told = 'refused: ${refused.message}';
  } catch (refused) {
    told = 'refused: $refused';
  }
  print('$name: $told');
}
";

/// What the Dart read says of a key held as a number where its type reads a text.
const MISTYPED: &str = "refused: type 'int' is not a subtype of type 'String' in type cast";

/// A holder printed by the run: its name, the payload, the Dart that tells what is read of it,
/// what serde tells of it, and what the Dart read tells in its place where it differs.
struct Row {
    dart: String,
    in_dart: Option<&'static str>,
    json: &'static str,
    name: &'static str,
    serde: String,
}

impl Row {
    /// This row, which the Dart read tells as `told`.
    const fn in_dart(mut self, told: &'static str) -> Self {
        self.in_dart = Some(told);
        self
    }

    /// The holder read as `T`, named `read_as` in Dart, from `json`: the member `member` reads,
    /// then the holder written back, as serde tells them through `held`.
    fn read<T>(
        name: &'static str,
        read_as: &str,
        member: &str,
        json: &'static str,
        held: fn(&T) -> String,
    ) -> Self
    where
        T: DeserializeOwned + Serialize,
    {
        let serde = serde_json::from_str::<T>(json).map_or_else(
            |refused| {
                let said = refused.to_string();
                let told = said
                    .split_once(" at line")
                    .map_or(said.as_str(), |(message, _position)| message);
                format!("refused: {told}")
            },
            |read| format!("{} {}", held(&read), written(&read)),
        );
        Self {
            dart: format!(
                "final read = {read_as}.fromJson(jsonDecode(payload)); return \
                 '${{jsonEncode({member}?.toJson())}} ${{jsonEncode(read.toJson())}}';"
            ),
            in_dart: None,
            json,
            name,
            serde,
        }
    }
}

/// The text of `value` as serde writes it.
fn written<T>(value: &T) -> String
where
    T: Serialize,
{
    serde_json::to_string(value).unwrap()
}

/// Each holder read from a payload that holds the flattened member, then from one that holds
/// none of its type's keys, then from one that holds a key its type does not read, and each whose
/// model breaks its rule or whose function answers what its type does not read.
fn rows() -> Vec<Row> {
    let lapping = |name, json| {
        Row::read(name, "Lapping", "read.tabling", json, |read: &Lapping| {
            written(&read.tabling)
        })
    };
    let flaking = |name, json| {
        Row::read(name, "Flaking", "read.tabling", json, |read: &Flaking| {
            written(&read.tabling)
        })
    };
    let sennit = |name, json| {
        Row::read(
            name,
            "Sennit",
            "(read as SennitLaid).tabling",
            json,
            |read: &Sennit| {
                let Sennit::Laid {
                    tabling,
                    note: _note,
                } = read;
                written(tabling)
            },
        )
    };
    let painter = |name, json| {
        Row::read(name, "Painter", "read.gasket", json, |read: &Painter| {
            written(&read.gasket)
        })
    };
    let ratline = |name, json| {
        Row::read(name, "Ratline", "read.nippering", json, |read: &Ratline| {
            written(&read.nippering)
        })
    };
    let crowning = |name, json| {
        Row::read(name, "Crowning", "read.tabling", json, |read: &Crowning| {
            written(&read.tabling)
        })
    };
    let kinking = |name, json| {
        Row::read(name, "Kinking", "read.plaiting", json, |read: &Kinking| {
            written(&read.plaiting)
        })
    };
    let tarring = |name, json| {
        Row::read(name, "Tarring", "read.plaiting", json, |read: &Tarring| {
            written(&read.plaiting)
        })
    };
    let fidding = |name, json| {
        Row::read(name, "Fidding", "read.crowning", json, |read: &Fidding| {
            written(&read.crowning)
        })
    };
    let veering = |name, json| {
        Row::read(name, "Veering", "read.tabling", json, |read: &Veering| {
            written(&read.tabling)
        })
    };
    vec![
        lapping("lapping", LAPPING),
        flaking("flaking", FLAKING),
        sennit("sennit", SENNIT),
        painter("painter", PAINTER),
        ratline("ratline", FLAKING),
        crowning("crowning", LAPPING),
        kinking("kinking", FLAKING),
        tarring("tarring", LAPPING),
        fidding("fidding", LAPPING),
        veering("veering", LAPPING),
        lapping("lapping absent", LAPPING_ABSENT),
        flaking("flaking absent", FLAKING_ABSENT),
        sennit("sennit absent", SENNIT_ABSENT),
        painter("painter absent", FLAKING_ABSENT),
        ratline("ratline absent", FLAKING_ABSENT),
        kinking("kinking absent", FLAKING_ABSENT),
        tarring("tarring absent", LAPPING_ABSENT),
        fidding("fidding absent", FLAKING_ABSENT),
        fidding("fidding unread", LAPPING_ABSENT),
        fidding("fidding mistyped", LAPPING_MISTYPED),
        ratline("ratline mistyped", FLAKING_MISTYPED),
        // A key of the type held as a number, or one of two keys: TypeScript and Zod refuse it.
        lapping("lapping mistyped", LAPPING_MISTYPED).in_dart(MISTYPED),
        flaking("flaking mistyped", FLAKING_MISTYPED).in_dart(MISTYPED),
        sennit("sennit mistyped", SENNIT_MISTYPED).in_dart(MISTYPED),
        kinking("kinking mistyped", FLAKING_MISTYPED).in_dart(MISTYPED),
        tarring("tarring mistyped", LAPPING_MISTYPED).in_dart(MISTYPED),
        painter("painter partial", FLAKING)
            .in_dart("refused: type 'Null' is not a subtype of type 'int' in type cast"),
        kinking("kinking refused", KINKING_SHORT),
        tarring("tarring refused", TARRING_SHORT),
        // Refused by both, by the Dart read in the words of its cast.
        veering("veering refused", LAPPING_ABSENT)
            .in_dart("refused: type 'Null' is not a subtype of type 'String' in type cast"),
    ]
}

/// The emitted definitions of every holder the analyzer passes whole, and of the types they hold.
/// [`Crowning`] and [`Fidding`], which holds it, are in [`definitions`] alone: the codec of a struct
/// flattened bare casts a value whose type the analyzer already knows.
fn analyzed_definitions() -> Vec<String> {
    vec![
        tabling_dart::dart_definition(),
        lapping_dart::dart_definition(),
        flaking_dart::dart_definition(),
        sennit_dart::dart_definition(),
        gasket_dart::dart_definition(),
        painter_dart::dart_definition(),
        ratline_dart::dart_definition(),
        nippering_dart::dart_definition(),
        plaiting_dart::dart_definition(),
        kinking_dart::dart_definition(),
        tarring_dart::dart_definition(),
        veering_dart::dart_definition(),
    ]
}

/// The emitted definitions of every holder and of the types they hold.
fn definitions() -> Vec<String> {
    let mut definitions = analyzed_definitions();
    definitions.extend([
        crowning_dart::dart_definition(),
        fidding_dart::dart_definition(),
    ]);
    definitions
}

/// Asserts that `definition` holds `fragment`.
fn assert_written(definition: &str, fragment: &str) {
    assert!(
        definition.contains(fragment),
        "missing `{fragment}` in: {definition}"
    );
}

/// A flattened `Option` of a struct whose keys are known is read where the object it is handed
/// holds one of them, and is `null` where it holds none: alone in its holder, beside a member of
/// its holder's own, and in a variant under a tag key, without the tag. One whose keys are not
/// known, a struct declared below its holder and one with a flattened member of its own, is read
/// in a closure that answers `null` where the read throws.
#[test]
fn a_flattened_option_is_read_where_one_of_its_keys_is_held() {
    assert_written(
        &lapping_dart::dart_definition(),
        "Lapping(note: json['note'] as String,tabling: (({...json}..remove('note')).keys.any(const \
         ['label'].contains) ? Tabling.fromJsonUnjudged(({...json}..remove('note'))) : null),);",
    );
    assert_written(
        &flaking_dart::dart_definition(),
        "Flaking(tabling: (json.keys.any(const ['label'].contains) ? \
         Tabling.fromJsonUnjudged(json) : null),);",
    );
    assert_written(
        &sennit_dart::dart_definition(),
        "SennitLaid(tabling: (({...json}..remove('kind')..remove('note')).keys.any(const \
         ['label'].contains) ? Tabling.fromJsonUnjudged(({...json}..remove('kind')..remove('note'))) \
         : null),note: json['note'] as String,);",
    );
    assert_written(
        &painter_dart::dart_definition(),
        "Painter(gasket: (json.keys.any(const ['label', 'turns'].contains) ? \
         Gasket.fromJsonUnjudged(json) : null),);",
    );
    assert_written(
        &fidding_dart::dart_definition(),
        "Fidding(crowning: (() { try { return Crowning.fromJsonUnjudged(json); } catch (_) { \
         return null; } })(),);",
    );
    assert_written(
        &ratline_dart::dart_definition(),
        "Ratline(nippering: (() { try { return Nippering.fromJsonUnjudged(json); } catch (_) { \
         return null; } })(),);",
    );
}

/// A flattened `Option` that names a function, and a struct flattened bare, are read as they were.
#[test]
fn a_flattened_option_that_names_a_function_is_read_as_it_was() {
    assert_written(
        &veering_dart::dart_definition(),
        "Veering(note: json['note'] as String,tabling: switch (asGiven(({...json}..remove('note')))) \
         { final corrected => (corrected == null ? null : Tabling.fromJsonUnjudged(corrected)) },);",
    );
    assert_written(
        &crowning_dart::dart_definition(),
        "Crowning(note: json['note'] as String,tabling: \
         Tabling.fromJsonUnjudged(({...json}..remove('note'))),);",
    );
}

/// A flattened `Option` is `null` where the object holds none of its type's keys, alone in its
/// holder, beside a member of its holder's own and in a variant under a tag key, as serde reads
/// it, and a model it holds that breaks its rule refuses the payload in serde's words. Where the
/// object holds a key its type does not read, a key held as a number or one of two keys, serde
/// reads `None` and the Dart read refuses the payload, as TypeScript and Zod refuse it, but for a
/// type whose keys are not known, read as `null` as serde reads it. Each holder is written back
/// as serde writes it back.
#[test]
fn a_flattened_option_reads_and_writes_in_dart_what_serde_reads_and_writes() {
    let rows = rows();
    let serde_told = rows.iter().fold(String::new(), |mut told, row| {
        let _ = writeln!(told, "{}: {}", row.name, row.serde);
        told
    });
    assert_eq!(
        serde_told,
        "lapping: {\"label\":\"north\"} {\"note\":\"n\",\"label\":\"north\"}\n\
         flaking: {\"label\":\"north\"} {\"label\":\"north\"}\n\
         sennit: {\"label\":\"north\"} {\"kind\":\"Laid\",\"label\":\"north\",\"note\":\"n\"}\n\
         painter: {\"label\":\"north\",\"turns\":2} {\"label\":\"north\",\"turns\":2}\n\
         ratline: {\"label\":\"north\"} {\"label\":\"north\"}\n\
         crowning: {\"label\":\"north\"} {\"note\":\"n\",\"label\":\"north\"}\n\
         kinking: {\"label\":\"north\"} {\"label\":\"north\"}\n\
         tarring: {\"label\":\"north\"} {\"note\":\"n\",\"label\":\"north\"}\n\
         fidding: {\"note\":\"n\",\"label\":\"north\"} {\"note\":\"n\",\"label\":\"north\"}\n\
         veering: {\"label\":\"north\"} {\"note\":\"n\",\"label\":\"north\"}\n\
         lapping absent: null {\"note\":\"n\"}\n\
         flaking absent: null {}\n\
         sennit absent: null {\"kind\":\"Laid\",\"note\":\"n\"}\n\
         painter absent: null {}\n\
         ratline absent: null {}\n\
         kinking absent: null {}\n\
         tarring absent: null {\"note\":\"n\"}\n\
         fidding absent: null {}\n\
         fidding unread: null {}\n\
         fidding mistyped: null {}\n\
         ratline mistyped: null {}\n\
         lapping mistyped: null {\"note\":\"n\"}\n\
         flaking mistyped: null {}\n\
         sennit mistyped: null {\"kind\":\"Laid\",\"note\":\"n\"}\n\
         kinking mistyped: null {}\n\
         tarring mistyped: null {\"note\":\"n\"}\n\
         painter partial: null {}\n\
         kinking refused: refused: 'label': too short: minimum length is 2, got 1\n\
         tarring refused: refused: 'label': too short: minimum length is 2, got 1\n\
         veering refused: refused: missing field `label`\n",
        "serde no longer reads each payload as it did"
    );
    let expected = rows.iter().fold(String::new(), |mut told, row| {
        let _ = writeln!(
            told,
            "{}: {}",
            row.name,
            row.in_dart.unwrap_or(row.serde.as_str())
        );
        told
    });
    let mut main = String::from("void main() {\n");
    for row in &rows {
        let _ = writeln!(
            main,
            "  row('{}', () {{ final payload = r'{}'; {} }});",
            row.name, row.json, row.dart
        );
    }
    main.push_str("}\n");
    let source = module(&definitions(), &[AS_GIVEN, TOLD], &main);
    let Some((ran, printed)) = dart("flattened-options", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart reads beside serde's");
}

/// The analyzer passes the holders of a flattened `Option`, read by its keys and in a closure.
#[test]
fn the_emitted_holders_of_a_flattened_option_have_no_issue() {
    let source = module(
        &analyzed_definitions(),
        &[AS_GIVEN],
        "void main() => print(jsonEncode(const Flaking().toJson()));\n",
    );
    let Some((clean, said)) = dart("flattened-options-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
