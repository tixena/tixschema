//! The refusal class each Dart type declares in its own text, held to what the analyzer accepts in
//! one file of a struct, an enum, a generic type, a brand and an alias, and to what the run prints:
//! `validate()`, `of(..)` and `fromJson` throw `{Name}Refusal`, whose `issues` are what Rust's
//! `validate()` answers of the same value, and a refusal of shape keeps its `FormatException`.

#![cfg(feature = "dart")]

use super::refusal_class_models::{
    BROKEN_CRIB, BROKEN_HAULAGE, BROKEN_LABEL, BROKEN_SKID, UNREAD_SKID, counted_issues, crib_dart,
    crib_issues, haulage_dart, label_issues, loose_issues, skep_label_dart, skid_dart, skid_issues,
    told, truss_dart,
};
use super::run_dart_validate::{dart, module};

/// What prints a case: its name, the class it was refused with and each issue.
const REPORT: &str = "
void report(String name, String refusal, List<String> issues) {
  print('$name: $refusal');
  for (final issue in issues) {
    print('    $issue');
  }
}

void unread(String name, Object Function() read) {
  try {
    read();
    print('$name: read');
  } on SkidRefusal {
    print('$name: SkidRefusal');
  } on FormatException {
    print('$name: FormatException');
  }
}
";

/// The five texts in one file, the reports, and the cases.
fn source() -> String {
    let main = format!(
        "void main() {{
  try {{ SkidDraft(label: '', parcel_count: 200).validate(); }} on SkidRefusal catch (refusal) {{ report('skid validate', 'SkidRefusal', refusal.issues); print(refusal); }}
  try {{ Skid.of(label: '', parcel_count: 200); }} on SkidRefusal catch (refusal) {{ report('skid of', 'SkidRefusal', refusal.issues); }}
  try {{ Skid.fromJson(jsonDecode('{BROKEN_SKID}') as Map<String, dynamic>); }} on SkidRefusal catch (refusal) {{ report('skid read', 'SkidRefusal', refusal.issues); }}
  try {{ HaulageLoose(label: '').validate(); }} on HaulageRefusal catch (refusal) {{ report('haulage validate', 'HaulageRefusal', refusal.issues); }}
  try {{ Haulage.fromJson(jsonDecode('{BROKEN_HAULAGE}')); }} on HaulageRefusal catch (refusal) {{ report('haulage read', 'HaulageRefusal', refusal.issues); }}
  try {{ CribDraft<Skid>(items: [], title: '').validate(); }} on CribRefusal catch (refusal) {{ report('crib validate', 'CribRefusal', refusal.issues); }}
  try {{ Crib<Skid>.fromJson(jsonDecode('{BROKEN_CRIB}') as Map<String, dynamic>, (e) => Skid.fromJson(e as Map<String, dynamic>)); }} on CribRefusal catch (refusal) {{ report('crib read', 'CribRefusal', refusal.issues); }}
  try {{ SkepLabel.of('a'); }} on SkepLabelRefusal catch (refusal) {{ report('label of', 'SkepLabelRefusal', refusal.issues); }}
  try {{ SkepLabel.fromJson(jsonDecode('{BROKEN_LABEL}')); }} on SkepLabelRefusal catch (refusal) {{ report('label read', 'SkepLabelRefusal', refusal.issues); }}
  try {{ Truss.fromJson(jsonDecode('{BROKEN_SKID}')); }} on TrussTypeRefusal catch (refusal) {{ report('truss read', 'TrussTypeRefusal', refusal.issues); }}
  unread('skid unread', () => Skid.fromJson(jsonDecode('{UNREAD_SKID}') as Map<String, dynamic>));
}}"
    );
    module(
        &[
            skid_dart::dart_definition(),
            haulage_dart::dart_definition(),
            crib_dart::dart_definition(),
            skep_label_dart::dart_definition(),
            truss_dart::dart_definition(),
        ],
        &[REPORT],
        &main,
    )
}

#[test]
fn a_file_of_every_kind_of_type_declares_each_refusal_once() {
    let Some((clean, said)) = dart("refusal-classes-analyze", &["analyze"], &source()) else {
        return;
    };
    assert!(clean, "{said}");
}

#[test]
fn each_type_throws_its_own_refusal_holding_what_rust_answers() {
    let Some((ran, printed)) = dart("refusal-classes", &["run"], &source()) else {
        return;
    };
    assert!(ran, "{printed}");
    let skid = skid_issues();
    let expected = [
        told("skid validate", "SkidRefusal", &skid),
        format!("SkidRefusal: [{}]\n", skid.join(", ")),
        told("skid of", "SkidRefusal", &skid),
        told("skid read", "SkidRefusal", &skid),
        told("haulage validate", "HaulageRefusal", &loose_issues()),
        told("haulage read", "HaulageRefusal", &counted_issues()),
        told("crib validate", "CribRefusal", &crib_issues()),
        told("crib read", "CribRefusal", &crib_issues()),
        told("label of", "SkepLabelRefusal", &label_issues()),
        told("label read", "SkepLabelRefusal", &label_issues()),
        told("truss read", "TrussTypeRefusal", &skid),
        "skid unread: FormatException\n".to_owned(),
    ]
    .concat();
    assert_eq!(printed, expected);
}
