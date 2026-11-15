//! The refusal struct each Swift type declares in its own text, held to what the compiler accepts
//! in one file of a struct, an enum, a generic type, a brand and an alias, and to what the run
//! prints: `validate()`, `of(..)`, `init(validating:)` and a read throw `{Name}Refusal`, whose
//! `issues` are what Rust's `validate()` answers of the same value, and a refusal of shape keeps
//! its `DecodingError`.

#![cfg(feature = "swift")]

use super::refusal_class_models::{
    BROKEN_CRIB, BROKEN_HAULAGE, BROKEN_LABEL, BROKEN_SKID, UNREAD_SKID, counted_issues,
    crib_issues, crib_swift, haulage_swift, label_issues, loose_issues, skep_label_swift,
    skid_issues, skid_swift, told, truss_swift,
};
use super::runtime::judged;

const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// The cases, each printing the struct it was refused with and each issue.
fn main() -> String {
    format!(
        r##"
func report(_ name: String, _ refusal: String, _ issues: [String]) {{
    print("\(name): \(refusal)")
    for issue in issues {{
        print("    \(issue)")
    }}
}}
let decoder = JSONDecoder()
do {{ _ = try SkidDraft(label: "", parcelCount: 200).validate() }} catch let refusal as SkidRefusal {{ report("skid validate", "SkidRefusal", refusal.issues); print(refusal) }}
do {{ _ = try Skid.of(label: "", parcelCount: 200) }} catch let refusal as SkidRefusal {{ report("skid of", "SkidRefusal", refusal.issues) }}
do {{ _ = try decoder.decode(Skid.self, from: Data(#"{BROKEN_SKID}"#.utf8)) }} catch let refusal as SkidRefusal {{ report("skid read", "SkidRefusal", refusal.issues) }}
do {{ _ = try HaulageDraft.loose(HaulageLoosePayload(label: "")).validate() }} catch let refusal as HaulageRefusal {{ report("haulage validate", "HaulageRefusal", refusal.issues) }}
do {{ _ = try decoder.decode(Haulage.self, from: Data(#"{BROKEN_HAULAGE}"#.utf8)) }} catch let refusal as HaulageRefusal {{ report("haulage read", "HaulageRefusal", refusal.issues) }}
do {{ _ = try CribDraft<Skid>(items: [], title: "").validate() }} catch let refusal as CribRefusal {{ report("crib validate", "CribRefusal", refusal.issues) }}
do {{ _ = try decoder.decode(Crib<Skid>.self, from: Data(#"{BROKEN_CRIB}"#.utf8)) }} catch let refusal as CribRefusal {{ report("crib read", "CribRefusal", refusal.issues) }}
do {{ _ = try SkepLabel(validating: "a") }} catch let refusal as SkepLabelRefusal {{ report("label of", "SkepLabelRefusal", refusal.issues) }}
do {{ _ = try decoder.decode(SkepLabel.self, from: Data(#"{BROKEN_LABEL}"#.utf8)) }} catch let refusal as SkepLabelRefusal {{ report("label read", "SkepLabelRefusal", refusal.issues) }}
do {{ _ = try decoder.decode(Truss.self, from: Data(#"{BROKEN_SKID}"#.utf8)) }} catch let refusal as SkidRefusal {{ report("truss read", "SkidRefusal", refusal.issues) }}
do {{
    _ = try decoder.decode(Skid.self, from: Data(#"{UNREAD_SKID}"#.utf8))
    print("skid unread: read")
}} catch is SkidRefusal {{
    print("skid unread: SkidRefusal")
}} catch is DecodingError {{
    print("skid unread: DecodingError")
}}
"##
    )
}

#[test]
fn each_type_throws_its_own_refusal_holding_what_rust_answers() {
    let source = [
        "import Foundation".to_owned(),
        skid_swift::swift_definition(),
        haulage_swift::swift_definition(),
        crib_swift::swift_definition(),
        skep_label_swift::swift_definition(),
        truss_swift::swift_definition(),
        main(),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-refusal-classes",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(run.succeeded, "{}\n{source}", run.stderr);
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
        told("truss read", "SkidRefusal", &skid),
        "skid unread: DecodingError\n".to_owned(),
    ]
    .concat();
    assert_eq!(run.stdout, expected);
}
