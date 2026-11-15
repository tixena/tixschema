//! The refusal class each Kotlin type declares in its own text, held to what kotlinc accepts in one
//! file of a struct, an enum, a generic type, a brand and an alias, and to what the run prints:
//! `validate()`, `of(..)` and a read throw `{Name}Refusal`, whose `issues` are what Rust's
//! `validate()` answers of the same value, and a refusal of shape keeps its
//! `SerializationException`.

#![cfg(feature = "kotlin")]

use super::refusal_class_models::{
    BROKEN_CRIB, BROKEN_HAULAGE, BROKEN_LABEL, BROKEN_SKID, UNREAD_SKID, counted_issues,
    crib_issues, crib_kotlin, haulage_kotlin, label_issues, loose_issues, skep_label_kotlin,
    skid_issues, skid_kotlin, told, truss_kotlin,
};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// The cases, each printing the class it was refused with and each issue.
fn main() -> String {
    format!(
        r#"
fun report(name: String, refusal: String, issues: List<String>) {{
    println("$name: $refusal")
    for (issue in issues) println("    $issue")
}}

fun main() {{
    try {{ SkidDraft(label = "", parcelCount = 200u).validate() }} catch (refusal: SkidRefusal) {{ report("skid validate", "SkidRefusal", refusal.issues); println(refusal) }}
    try {{ Skid.of(label = "", parcelCount = 200u) }} catch (refusal: SkidRefusal) {{ report("skid of", "SkidRefusal", refusal.issues) }}
    try {{ Json.decodeFromString<Skid>("""{BROKEN_SKID}""") }} catch (refusal: SkidRefusal) {{ report("skid read", "SkidRefusal", refusal.issues) }}
    try {{ HaulageLoose(label = "").validate() }} catch (refusal: HaulageRefusal) {{ report("haulage validate", "HaulageRefusal", refusal.issues) }}
    try {{ Json.decodeFromString<Haulage>("""{BROKEN_HAULAGE}""") }} catch (refusal: HaulageRefusal) {{ report("haulage read", "HaulageRefusal", refusal.issues) }}
    try {{ CribDraft<Skid>(items = emptyList(), title = "").validate() }} catch (refusal: CribRefusal) {{ report("crib validate", "CribRefusal", refusal.issues) }}
    try {{ Json.decodeFromString(serializer<Crib<Skid>>(), """{BROKEN_CRIB}""") }} catch (refusal: CribRefusal) {{ report("crib read", "CribRefusal", refusal.issues) }}
    try {{ SkepLabel.of("a") }} catch (refusal: SkepLabelRefusal) {{ report("label of", "SkepLabelRefusal", refusal.issues) }}
    try {{ Json.decodeFromString<SkepLabel>("""{BROKEN_LABEL}""") }} catch (refusal: SkepLabelRefusal) {{ report("label read", "SkepLabelRefusal", refusal.issues) }}
    try {{ Json.decodeFromString<Truss>("""{BROKEN_SKID}""") }} catch (refusal: SkidRefusal) {{ report("truss read", "SkidRefusal", refusal.issues) }}
    try {{
        Json.decodeFromString<Skid>("""{UNREAD_SKID}""")
        println("skid unread: read")
    }} catch (refusal: SkidRefusal) {{
        println("skid unread: SkidRefusal")
    }} catch (refused: SerializationException) {{
        println("skid unread: SerializationException")
    }}
}}
"#
    )
}

#[test]
fn each_type_throws_its_own_refusal_holding_what_rust_answers() {
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        skid_kotlin::kotlin_definition(),
        haulage_kotlin::kotlin_definition(),
        crib_kotlin::kotlin_definition(),
        skep_label_kotlin::kotlin_definition(),
        truss_kotlin::kotlin_definition(),
        main(),
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
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
        "skid unread: SerializationException\n".to_owned(),
    ]
    .concat();
    assert_eq!(printed, expected);
}
