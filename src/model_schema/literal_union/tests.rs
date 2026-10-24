use super::{Literal, Member, Plan, Shape, plan};

fn held(key: &str, value: &str, optional: bool) -> Member {
    Member {
        key: key.to_owned(),
        literal: Some(Literal {
            optional,
            value: value.to_owned(),
        }),
    }
}

fn bare(key: &str) -> Member {
    Member {
        key: key.to_owned(),
        literal: None,
    }
}

fn planned(shapes: &[Shape]) -> Option<Plan> {
    plan(&shapes.iter().collect::<Vec<_>>())
}

fn on(key: &str, omitting: Option<usize>) -> Plan {
    Plan {
        key: key.to_owned(),
        omitting,
    }
}

#[test]
fn every_variant_holding_a_distinct_literal_is_told_apart() {
    let shapes = [
        Shape::Object(vec![held("kind", "string \"a\"", false)]),
        Shape::Object(vec![held("kind", "string \"b\"", false)]),
    ];
    assert_eq!(planned(&shapes), Some(on("kind", None)));
}

#[test]
fn one_variant_may_hold_its_literal_in_an_option() {
    let shapes = [
        Shape::Object(vec![held("on", "boolean true", false)]),
        Shape::Object(vec![held("on", "boolean false", true)]),
    ];
    assert_eq!(planned(&shapes), Some(on("on", None)));
}

#[test]
fn two_variants_in_an_option_are_not_told_apart() {
    let shapes = [
        Shape::Object(vec![held("on", "boolean true", true)]),
        Shape::Object(vec![held("on", "boolean false", true)]),
    ];
    assert_eq!(planned(&shapes), None);
}

#[test]
fn a_literal_two_variants_share_tells_nothing_apart() {
    let shapes = [
        Shape::Object(vec![held("on", "boolean true", false)]),
        Shape::Object(vec![held("on", "boolean true", false)]),
    ];
    assert_eq!(planned(&shapes), None);
}

#[test]
fn a_variant_holding_the_key_under_no_literal_is_not_told_apart() {
    let shapes = [
        Shape::Object(vec![held("on", "boolean true", false)]),
        Shape::Object(vec![bare("on")]),
    ];
    assert_eq!(planned(&shapes), None);
}

#[test]
fn the_last_variant_may_omit_the_key() {
    let shapes = [
        Shape::Object(vec![held("on", "boolean true", false)]),
        Shape::Object(vec![bare("label")]),
    ];
    assert_eq!(planned(&shapes), Some(on("on", Some(1))));
}

#[test]
fn a_variant_that_omits_the_key_before_another_is_not_told_apart() {
    let shapes = [
        Shape::Object(vec![bare("label")]),
        Shape::Object(vec![held("on", "boolean true", false)]),
    ];
    assert_eq!(planned(&shapes), None);
}

#[test]
fn an_omitting_variant_beside_an_option_is_not_told_apart() {
    let shapes = [
        Shape::Object(vec![held("on", "boolean true", false)]),
        Shape::Object(vec![held("on", "boolean false", true)]),
        Shape::Object(vec![bare("label")]),
    ];
    assert_eq!(planned(&shapes), None);
}

#[test]
fn two_variants_that_omit_the_key_are_not_told_apart() {
    let shapes = [
        Shape::Object(vec![held("on", "boolean true", false)]),
        Shape::Object(vec![bare("label")]),
        Shape::Object(vec![bare("note")]),
    ];
    assert_eq!(planned(&shapes), None);
}

#[test]
fn a_variant_no_key_reads_is_not_told_apart() {
    let shapes = [
        Shape::Object(vec![held("on", "boolean true", false)]),
        Shape::Unreadable,
    ];
    assert_eq!(planned(&shapes), None);
}

#[test]
fn the_first_key_every_variant_holds_a_literal_on_is_the_key() {
    let shapes = [
        Shape::Object(vec![
            held("version", "number 1", false),
            held("kind", "string \"a\"", false),
        ]),
        Shape::Object(vec![
            held("version", "number 1", false),
            held("kind", "string \"b\"", false),
        ]),
    ];
    assert_eq!(planned(&shapes), Some(on("kind", None)));
}
