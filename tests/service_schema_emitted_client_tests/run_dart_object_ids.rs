//! An `ObjectId` in each position serde writes one, read and written back under the Dart VM by the
//! emitted Dart types beside the `ObjectId` the README declares, against serde: a key beside
//! `$oid`, and a `$oid` that is not 24 hexadecimal digits in lower case, are refused where serde
//! refuses them, and a flattened one is read from its `$oid` key alone, as serde reads past any
//! other.

#![cfg(all(feature = "dart", feature = "mongodb"))]

use core::fmt::Write as _;

use super::object_id_models::{
    belated_dart, bullae_dart, cachet_dart, cartouche_dart, cartouching_dart, countersealing_dart,
    countersigned_dart, crossed_dart, crossing_dart, docketing_dart, embossed_dart, endorsed_dart,
    endorsing_dart, enveloping_dart, enwafered_dart, folding_dart, impress_dart, impressed_dart,
    impressing_dart, late_cartouche_dart, late_sigil_dart, latewaxed_dart, lettering_dart,
    notation_dart, relayed_dart, relaying_dart, rewaxed_dart, rewaxing_dart, rows, sigil_dart,
    sigiled_dart, sigillary_dart, sigillum_dart, stampage_dart, wafer_dart, wafered_dart,
    wafering_dart, wafers_dart, waxing_dart, waxwork_dart, waxworking_dart,
};
use super::object_id_runs::{printed, readme_object_id};
use super::run_dart_validate::{dart, module};

/// What prints a row: the value read and written back, or `refused`.
const TOLD: &str = "
void told(String at, Object? Function() read) {
  String printed;
  try {
    printed = 'read ${jsonEncode(read())}';
  } catch (_) {
    printed = 'refused';
  }
  print('$at: $printed');
}
";

/// The README's `ObjectId` and the emitted types the rows read.
fn definitions() -> Vec<String> {
    vec![
        readme_object_id("dart"),
        notation_dart::dart_definition(),
        cartouche_dart::dart_definition(),
        impress_dart::dart_definition(),
        waxwork_dart::dart_definition(),
        countersigned_dart::dart_definition(),
        sigil_dart::dart_definition(),
        sigillum_dart::dart_definition(),
        endorsed_dart::dart_definition(),
        endorsing_dart::dart_definition(),
        cachet_dart::dart_definition(),
        embossed_dart::dart_definition(),
        docketing_dart::dart_definition(),
        stampage_dart::dart_definition(),
        sigiled_dart::dart_definition(),
        sigillary_dart::dart_definition(),
        enveloping_dart::dart_definition(),
        waxing_dart::dart_definition(),
        rewaxing_dart::dart_definition(),
        relaying_dart::dart_definition(),
        wafer_dart::dart_definition(),
        lettering_dart::dart_definition(),
        folding_dart::dart_definition(),
        cartouching_dart::dart_definition(),
        rewaxed_dart::dart_definition(),
        countersealing_dart::dart_definition(),
        latewaxed_dart::dart_definition(),
        waxworking_dart::dart_definition(),
        impressing_dart::dart_definition(),
        relayed_dart::dart_definition(),
        crossed_dart::dart_definition(),
        crossing_dart::dart_definition(),
        bullae_dart::dart_definition(),
        wafered_dart::dart_definition(),
        wafering_dart::dart_definition(),
        enwafered_dart::dart_definition(),
        wafers_dart::dart_definition(),
        impressed_dart::dart_definition(),
        belated_dart::dart_definition(),
        late_sigil_dart::dart_definition(),
        late_cartouche_dart::dart_definition(),
    ]
}

/// Each payload of [`rows`] is read by the Dart read where the row states a client reads it, and
/// written back as serde writes it, and refused where it states none does.
#[test]
fn an_object_id_is_read_and_written_back_by_dart_as_serde_reads_it() {
    let mut main = String::from("void main() {\n");
    let mut expected = String::new();
    for (at, row) in rows().iter().enumerate() {
        let _ = writeln!(
            main,
            "  told('{at}', () => {}.fromJson(jsonDecode(r'''{}''')).toJson());",
            row.read_as, row.json
        );
        let _ = writeln!(
            expected,
            "{at}: {}",
            printed(row.dart, row.read_as, row.json)
        );
    }
    main.push_str("}\n");
    let source = module(&definitions(), &[TOLD], &main);
    let Some((ran, printed)) = dart("object-ids", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not serde's");
}

/// The analyzer passes the README's `ObjectId` beside every emitted type the rows read: a holder
/// of one under a key, in a list, as a type argument, under a brand, as a map's value, flattened
/// and beside a tag.
#[test]
fn the_readme_object_id_has_no_issue_beside_the_emitted_types() {
    let source = module(
        &definitions(),
        &[],
        &format!(
            "void main() => \
             print(jsonEncode(Endorsed.fromJson(jsonDecode(r'''{}''')).toJson()));\n",
            rows()[0].json
        ),
    );
    let Some((clean, said)) = dart("object-ids-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}

/// The Dart read hands a flattened `ObjectId`, and a brand and an alias over one, the entry of its
/// `$oid` key alone, and looks for an optional one by that key.
#[test]
fn the_dart_text_hands_a_flattened_object_id_its_key_alone() {
    let only = "(switch (json) { final whole => { for (final key in const ['\\$oid']) if \
                (whole.containsKey(key)) key: whole[key] } })";
    for (written, held) in [
        (
            docketing_dart::dart_definition(),
            format!("oid: ObjectId.fromJson({only}),"),
        ),
        (
            sigiled_dart::dart_definition(),
            format!("mark: Sigil.fromJsonUnjudged({only}),"),
        ),
        (
            sigillary_dart::dart_definition(),
            format!("oid: SigillumType.fromJsonUnjudged({only}),"),
        ),
        (
            stampage_dart::dart_definition(),
            "oid: ((switch (({...json}..remove('id'))) { final whole => { for (final key in \
             const ['\\$oid']) if (whole.containsKey(key)) key: whole[key] } }).keys.any(const \
             ['\\$oid'].contains) ? "
                .to_owned(),
        ),
    ] {
        assert!(written.contains(&held), "no `{held}` in: {written}");
    }
}

/// A holder hands a type that reads its type parameter only flattened a converter that hands an
/// `ObjectId`, and a brand and an alias over one, the entry of its `$oid` key alone, through a type
/// that hands the parameter on as well. A type that also reads it under a key takes that converter
/// for its flattened reads beside the one that hands all it is handed, and a type that reads it
/// under a key alone takes the second alone.
#[test]
fn the_dart_text_hands_an_object_id_a_type_parameter_flattens_its_key_alone() {
    let only = |read: &str| {
        format!(
            "(e) => {read}((switch (e) {{ final whole => {{ for (final key in const ['\\$oid']) \
             if (whole.containsKey(key)) key: whole[key] }} }}))"
        )
    };
    for (written, held) in [
        (
            lettering_dart::dart_definition(),
            format!(
                "Enveloping.fromJsonUnjudged(json['envelope'], {})",
                only("ObjectId.fromJson")
            ),
        ),
        (
            relayed_dart::dart_definition(),
            format!(
                "Relaying.fromJsonUnjudged(json['relay'], {})",
                only("ObjectId.fromJson")
            ),
        ),
        (
            bullae_dart::dart_definition(),
            format!(
                "Enveloping.fromJsonUnjudged(json['branded'], {})",
                only("Sigil.fromJson")
            ),
        ),
        (
            bullae_dart::dart_definition(),
            format!(
                "Enveloping.fromJsonUnjudged(json['aliased'], {})",
                only("SigillumType.fromJson")
            ),
        ),
        (
            endorsed_dart::dart_definition(),
            "Countersigned.fromJsonUnjudged(json['carried'], (e) => ObjectId.fromJson(e))"
                .to_owned(),
        ),
        (
            crossing_dart::dart_definition(),
            format!(
                "Crossed.fromJsonUnjudged(json['crossed'], (e) => ObjectId.fromJson(e), \
                 heldFromFlattenedJson: {})",
                only("ObjectId.fromJson")
            ),
        ),
        (
            crossed_dart::dart_definition(),
            "factory Crossed.fromJsonUnjudged(Map<String, dynamic> json, Held Function(dynamic) \
             heldFromJson, {Held Function(dynamic)? heldFromFlattenedJson}) => Crossed(held: \
             (heldFromFlattenedJson ?? heldFromJson)(({...json}..remove('spare'))),remark: \
             Notation.fromJsonUnjudged(json),spare: heldFromJson(json['spare']),);"
                .to_owned(),
        ),
        (
            enveloping_dart::dart_definition(),
            "factory Enveloping.fromJsonUnjudged(Map<String, dynamic> json, Held Function(dynamic) \
             heldFromJson) => Enveloping(held: heldFromJson(json),"
                .to_owned(),
        ),
    ] {
        assert!(written.contains(&held), "no `{held}` in: {written}");
    }
}

/// A type that reads its type parameter flattened under an `Option` takes the keys of what fills
/// it, and reads no value where the object holds none of them, and the filling's read where it
/// holds one. A holder hands the keys of a filling it knows, an `ObjectId`'s and a struct's or an
/// enum's written above it, a type that hands its parameter on hands the keys it is handed, and a
/// holder of a filling declared below it hands none, which reads no value wherever the read fails.
#[test]
fn the_dart_text_looks_a_flattened_option_of_a_type_parameter_up_by_the_keys_of_its_filling() {
    for (written, held) in [
        (
            waxing_dart::dart_definition(),
            "factory Waxing.fromJsonUnjudged(Map<String, dynamic> json, Held Function(dynamic) \
             heldFromJson, {List<String>? heldKeys}) => Waxing(held: (heldKeys == null ? (() { \
             try { return heldFromJson(json); } catch (_) { return null; } })() : \
             (json.keys.any(heldKeys.contains) ? heldFromJson(json) : null)),"
                .to_owned(),
        ),
        (
            folding_dart::dart_definition(),
            "heldKeys: const ['\\$oid'])".to_owned(),
        ),
        (
            cartouching_dart::dart_definition(),
            "Waxing.fromJsonUnjudged(json['waxed'], (e) => Cartouche.fromJson(e), heldKeys: \
             const ['wax'])"
                .to_owned(),
        ),
        (
            countersealing_dart::dart_definition(),
            "heldKeys: const ['held'])".to_owned(),
        ),
        (
            waxworking_dart::dart_definition(),
            "heldKeys: const ['kind'])".to_owned(),
        ),
        (
            rewaxing_dart::dart_definition(),
            "Waxing.fromJsonUnjudged(json['waxing'], (e) => heldFromJson(e), heldKeys: heldKeys)"
                .to_owned(),
        ),
        (
            rewaxed_dart::dart_definition(),
            "Rewaxing.fromJsonUnjudged(json['rewaxed'], (e) => Cartouche.fromJson(e), heldKeys: \
             const ['wax'])"
                .to_owned(),
        ),
    ] {
        assert!(written.contains(&held), "no `{held}` in: {written}");
    }
    let late = latewaxed_dart::dart_definition();
    assert!(
        !late.contains("heldKeys"),
        "a filling declared below its holder is handed keys: {late}"
    );
}
