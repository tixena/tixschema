//! serde reads each row of [`ROWS`] as the table says, and Zod joins the variants of each enum of
//! [`WRITTEN_AS`] with `z.discriminatedUnion` exactly where the variants are told apart by a
//! `literal` on one key.

use super::literal_models::{
    AliasedPanel, BothOptional, Calculation, Kind, ModelBelow, OmitsAsModel, OmitsBesideOption,
    OmitsFirst, OmitsLast, OmitsTwice, ROWS, SameLiteral, UnmarkedKey, Valued, serde_reads,
};

/// Each enum the rule is held to, and the Zod text it is written as.
pub const WRITTEN_AS: [(&str, Union); 11] = [
    ("Calculation", Union::Discriminated),
    ("BothOptional", Union::Plain),
    ("UnmarkedKey", Union::Plain),
    ("ModelBelow", Union::Plain),
    ("SameLiteral", Union::Plain),
    ("OmitsAsModel", Union::Discriminated),
    ("OmitsLast", Union::Discriminated),
    ("OmitsFirst", Union::Plain),
    ("OmitsBesideOption", Union::Plain),
    ("OmitsTwice", Union::Plain),
    ("Valued", Union::Discriminated),
];

/// How an enum's variants are joined in Zod.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Union {
    Discriminated,
    Plain,
}

/// The Zod text of the type `name`, or the words that say there is none.
fn zod_of(name: &str) -> String {
    match name {
        "BothOptional" => BothOptional::zod_schema(),
        "Calculation" => Calculation::zod_schema(),
        "Kind" => Kind::zod_schema(),
        "ModelBelow" => ModelBelow::zod_schema(),
        "OmitsAsModel" => OmitsAsModel::zod_schema(),
        "OmitsBesideOption" => OmitsBesideOption::zod_schema(),
        "OmitsFirst" => OmitsFirst::zod_schema(),
        "OmitsLast" => OmitsLast::zod_schema(),
        "OmitsTwice" => OmitsTwice::zod_schema(),
        "SameLiteral" => SameLiteral::zod_schema(),
        "UnmarkedKey" => UnmarkedKey::zod_schema(),
        "Valued" => Valued::<String>::zod_schema(),
        other => format!("no model {other}"),
    }
}

#[test]
fn serde_reads_each_row_as_the_table_says() {
    for row in &ROWS {
        assert_eq!(
            serde_reads(row.read_as, row.json),
            Ok(row.serde_reads),
            "serde answers {} read as {} otherwise",
            row.json,
            row.read_as
        );
    }
}

#[test]
fn zod_joins_the_variants_by_their_literal_exactly_where_the_rule_holds() {
    for (name, union) in WRITTEN_AS {
        let written = zod_of(name);
        let discriminated = written.contains("z.discriminatedUnion(\"isCalculated\"");
        assert_eq!(
            discriminated,
            union == Union::Discriminated,
            "{name} is written as:\n{written}"
        );
    }
}

/// An object that flattens the enum joins its discriminated union as the one operand it is, in
/// place of one chain per variant.
#[test]
fn a_flattened_discriminated_union_is_one_operand() {
    let written = Kind::zod_schema();
    assert!(
        written.contains(".and(z.lazy(() => Calculation$Schema))") && !written.contains("path"),
        "Kind is written as:\n{written}"
    );
}

/// An object that flattens an alias of the enum joins it as one operand too.
#[test]
fn a_flattened_alias_of_a_discriminated_union_is_one_operand() {
    let written = AliasedPanel::zod_schema();
    assert!(
        !written.contains("path"),
        "AliasedPanel is written as:\n{written}"
    );
}
