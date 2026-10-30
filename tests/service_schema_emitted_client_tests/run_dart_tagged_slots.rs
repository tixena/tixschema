//! The lone slot of a newtype variant under a tag alone, read and written back by the emitted Dart
//! types under the Dart VM, against what serde writes: the slot is read from the object without
//! the tag, as serde hands it, so an externally tagged enum there reads its one entry, and is
//! written beside the tag, a unit variant of that enum as an entry of its name over `null`, and an
//! `ObjectId` there as its `$oid` key.

#![cfg(all(feature = "dart", feature = "serde"))]

use core::fmt::Write as _;

#[cfg(feature = "mongodb")]
use super::object_id_runs::readme_object_id;
use super::run_dart_validate::{analyzed, dart, module};
use super::tagged_slot_models::{
    ENTRIES_REFUSED, WRITTEN, ballast_dart, bunkered_dart, cradled_dart, demurrage_dart,
    docketed_dart, dray_dart, drayage_dart, freighted_dart, lighter_dart, lightering_dart,
    palleted_dart, serde_told, shrinkwrapped_dart, stevedore_dart, swaddled_dart, tonnage_dart,
    trussed_dart, waybilled_dart, wharfage_dart,
};
#[cfg(feature = "mongodb")]
use super::tagged_slot_models::{STAMP_REFUSED, STAMPED, imprint_dart, stamping_dart};

/// The function `Docketed::Trimmed` and `Docketed::Pared` name, and what prints a row: the value
/// read, written back, or the class of what the read or the write throws and its message.
const TOLD: &str = "
dynamic lightened(dynamic payload) =>
    payload is Map<String, dynamic> ? ({...payload}..remove('spare')) : payload;

void told(String at, Object? Function() read) {
  String printed;
  try {
    printed = 'read ${jsonEncode(read())}';
  } on FormatException catch (refusal) {
    printed = '${refusal.runtimeType}: ${refusal.message}';
  } catch (refusal) {
    printed = '${refusal.runtimeType}: $refusal';
  }
  print('$at: $printed');
}
";

/// What `told` prints of `json` read as `read_as` and written back.
fn told(at: usize, read_as: &str, json: &str) -> String {
    if ["Docketed", "Stamping", "Swaddled", "Trussed"].contains(&read_as) {
        format!("  told('{at}', () => {read_as}.fromJson(jsonDecode(r'''{json}''')).toJson());\n")
    } else {
        format!(
            "  told('{at}', () => Waybilled.fromJson(jsonDecode(r'''{json}'''), (e) => \
             Tonnage.fromJson(e)).toJson((e) => e.toJson()));\n"
        )
    }
}

/// Each payload serde writes is read by the Dart read and written back as serde wrote it, an
/// externally tagged enum's entry beside the tag among them, a unit variant's over `null` too, and
/// each payload of an externally tagged slot serde refuses is refused.
#[test]
fn a_slot_beside_the_tag_is_read_and_written_back_by_dart_as_serde_writes_it() {
    let mut main = String::from("void main() {\n");
    let mut expected = String::new();
    for (at, (read_as, json)) in WRITTEN.iter().enumerate() {
        main.push_str(&told(at, read_as, json));
        let _ = writeln!(expected, "{at}: {}", serde_told(read_as, json));
    }
    for (at, refused) in (WRITTEN.len()..).zip(&ENTRIES_REFUSED) {
        assert_eq!(
            serde_told("Docketed", refused.json),
            "refused",
            "serde no longer refuses {}",
            refused.json
        );
        main.push_str(&told(at, "Docketed", refused.json));
        let _ = writeln!(expected, "{at}: {}", refused.dart);
    }
    main.push_str("}\n");
    let source = module(
        &[
            tonnage_dart::dart_definition(),
            ballast_dart::dart_definition(),
            wharfage_dart::dart_definition(),
            drayage_dart::dart_definition(),
            lighter_dart::dart_definition(),
            demurrage_dart::dart_definition(),
            bunkered_dart::dart_definition(),
            palleted_dart::dart_definition(),
            shrinkwrapped_dart::dart_definition(),
            trussed_dart::dart_definition(),
            cradled_dart::dart_definition(),
            swaddled_dart::dart_definition(),
            dray_dart::dart_definition(),
            stevedore_dart::dart_definition(),
            freighted_dart::dart_definition(),
            docketed_dart::dart_definition(),
            lightering_dart::dart_definition(),
            waybilled_dart::dart_definition(),
        ],
        &[TOLD],
        &main,
    );
    analyzed("tagged-slots-analyze", &source);
    let Some((ran, printed)) = dart("tagged-slots", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not serde's");
}

/// The Dart read hands a slot beside the tag the object without the tag, whatever the slot holds.
#[test]
fn the_dart_text_hands_the_slot_the_object_without_the_tag() {
    let docketed = docketed_dart::dart_definition();
    for held in [
        "case 'Weighed': return \
         DocketedWeighed(Tonnage.fromJsonUnjudged(({...json}..remove('kind'))));",
        "case 'External': return \
         DocketedExternal(Palleted.fromJsonUnjudged(({...json}..remove('kind'))));",
        "case 'Trimmed': return \
         DocketedTrimmed(Tonnage.fromJsonUnjudged(lightened(({...json}..remove('kind')))));",
    ] {
        assert!(docketed.contains(held), "no `{held}` in: {docketed}");
    }
}

/// The Dart write puts an externally tagged enum's unit variant in a slot beside the tag as an
/// entry of its name over `null`, where the enum's own `toJson` writes the bare name.
#[test]
fn the_dart_text_writes_a_unit_variant_beside_the_tag_as_its_entry() {
    let docketed = docketed_dart::dart_definition();
    let written = "@override dynamic toJson() => { 'kind': 'External', ...(switch \
                   ((value).toJson()) { final String name => {name: null}, final written => \
                   written as Map<String, dynamic> }) };";
    assert!(docketed.contains(written), "no `{written}` in: {docketed}");
}

/// An `ObjectId` in the slot is read from the object without the tag and written back as its
/// `$oid` key beside the tag, as serde writes it, owned, in a `Box` and under a brand, and the tag
/// alone is refused, as serde refuses it.
#[cfg(feature = "mongodb")]
#[test]
fn an_object_id_beside_the_tag_is_read_and_written_back_by_dart_as_serde_writes_it() {
    let mut main = String::from("void main() {\n");
    let mut expected = String::new();
    for (at, json) in STAMPED.iter().enumerate() {
        main.push_str(&told(at, "Stamping", json));
        let _ = writeln!(expected, "{at}: {}", serde_told("Stamping", json));
    }
    assert_eq!(
        serde_told("Stamping", STAMP_REFUSED.json),
        "refused",
        "serde no longer refuses {}",
        STAMP_REFUSED.json
    );
    main.push_str(&told(STAMPED.len(), "Stamping", STAMP_REFUSED.json));
    let _ = writeln!(expected, "{}: {}", STAMPED.len(), STAMP_REFUSED.dart);
    main.push_str("}\n");
    let source = module(
        &[
            readme_object_id("dart"),
            imprint_dart::dart_definition(),
            stamping_dart::dart_definition(),
        ],
        &[TOLD],
        &main,
    );
    analyzed("tagged-object-ids-analyze", &source);
    let Some((ran, printed)) = dart("tagged-object-ids", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not serde's");
}

/// The Dart read hands an `ObjectId` in the slot the object without the tag, and the write spreads
/// its `$oid` key after the tag, uncast: the README's `toJson` answers a `Map<String, dynamic>`.
#[cfg(feature = "mongodb")]
#[test]
fn the_dart_text_reads_and_writes_an_object_id_beside_the_tag() {
    let stamping = stamping_dart::dart_definition();
    for held in [
        "case 'Held': return StampingHeld(ObjectId.fromJson(({...json}..remove('kind'))));",
        "@override dynamic toJson() => { 'kind': 'Held', ...(value).toJson() };",
    ] {
        assert!(stamping.contains(held), "no `{held}` in: {stamping}");
    }
}
