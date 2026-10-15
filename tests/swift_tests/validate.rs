//! What the Swift backend writes for `validate()`: the method every emitted type has, in the text
//! of each shape. What the method answers is run under a Swift toolchain in
//! `service_schema_emitted_client_tests/run_swift_validate.rs`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use crate::tests::{
    adjacent_tagged_swift, all_widths_swift, by_bool_swift, by_enum_swift, by_u32_swift,
    correlation_id_swift, envelope_swift, external_tagged_swift, flattened_extra_swift,
    internal_tagged_swift, reserved_member_swift, reserved_members_swift, status_swift,
    tuple_point_swift, untagged_swift, user_data_swift, window_request_swift, with_flatten_swift,
    with_nullable_swift,
};

/// What every emitted type with no rule declares.
const NO_RULE: &str = "public func validate() -> [String] { [] }";

/// The parcel of the shared table: a bound, a check on a field and a check on the type.
#[model_schema(check = ["withinParcel"])]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub fragile_item_codes: Vec<String>,
    #[model_schema_prop(check = ["withoutDuplicates"])]
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A model held in a field, a list, a map and an optional.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub first_parcel: Parcel,
    pub parcel_list: Vec<Parcel>,
    pub parcels_by_dock: HashMap<String, Parcel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_parcel: Option<Parcel>,
}

/// A member named after the method every emitted type has, and one named after a local it writes.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inspection {
    #[model_schema_prop(minLength = 1)]
    pub errors: Vec<String>,
    pub validate: String,
}

#[model_schema(minLength = 2, maxLength = 8, pattern = "^[a-z]+$")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Slug(pub String);

/// A brand over a brand, with a bound of its own.
#[model_schema(maxLength = 6)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SealedSlug(pub Slug);

/// A brand with a parameter, whose bound holds the declared default alone.
#[model_schema(minLength = 2, default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Marker<IdType>(pub IdType);

/// A range on a float and on an unsigned integer no value of which is below zero.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reading {
    #[model_schema_prop(minimum = 0, maximum = 120)]
    pub age: u32,
    #[model_schema_prop(minimum = 0.5)]
    pub level: f64,
    #[model_schema_prop(minimum = 1)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<u8>,
}

#[model_schema()]
pub type Heading = String;

/// A bound on a field typed with a brand declared above, with an alias of a string, and with a
/// brand declared below.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Routing {
    #[model_schema_prop(maxLength = 3)]
    pub seen_code: Slug,
    #[model_schema_prop(minLength = 2)]
    pub titled: Heading,
    #[model_schema_prop(minLength = 2)]
    pub unseen_code: LaterSlug,
}

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LaterSlug(pub String);

/// An enum with no tag attribute: a slot, a named member, a slot of several and no member.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all_fields = "camelCase")]
pub enum Freight {
    Boxed(#[model_schema_prop(minLength = 1)] String),
    Loose {
        #[model_schema_prop(minLength = 1)]
        freight_label: String,
    },
    Split(
        #[model_schema_prop(minLength = 1)] String,
        #[model_schema_prop(minimum = 1)] u32,
    ),
    Unladen,
}

/// An enum whose own pattern is held beside the one a field writes, and alone by a slot.
#[model_schema(pattern = "^[a-z]+$")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Pier {
    Named {
        #[model_schema_prop(pattern = "^q")]
        quay: String,
    },
    Tagged(String),
}

/// A struct serde writes as the value of its one slot.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sealed(#[model_schema_prop(minLength = 1)] pub String);

fn within_parcel(value: &Parcel) -> Result<(), Vec<String>> {
    let outside: Vec<String> = value
        .fragile_item_codes
        .iter()
        .enumerate()
        .filter(|(_, code)| !value.item_code_list.contains(code))
        .map(|(index, code)| {
            format!("'fragileItemCodes.{index}': {code} is outside of itemCodeList")
        })
        .collect();
    if outside.is_empty() {
        Ok(())
    } else {
        Err(outside)
    }
}

fn without_duplicates(value: &[String]) -> Result<(), Vec<String>> {
    for (index, code) in value.iter().enumerate() {
        if value[..index].contains(code) {
            return Err(vec![format!("duplicate item code {code}")]);
        }
    }
    Ok(())
}

/// The bounds, then each field's check, then the type's check, with the helper a place is written
/// by. The read is written ahead of them, and `encode(to:)` stays synthesized.
#[test]
fn a_struct_runs_its_bounds_its_fields_checks_then_its_own() {
    assert_eq!(
        parcel_swift::swift_definition(),
        concat!(
            "public struct Parcel: Codable, Sendable { public let fragileItemCodes: [String]; ",
            "public let itemCodeList: [String]; public let label: String; ",
            "enum CodingKeys: String, CodingKey { case fragileItemCodes; case itemCodeList; ",
            "case label;  }; ",
            "public init(fragileItemCodes: [String], itemCodeList: [String], label: String) ",
            "{ self.fragileItemCodes = fragileItemCodes; self.itemCodeList = itemCodeList; ",
            "self.label = label;  }; ",
            "public init(swiftSchemaUnjudged decoder: Decoder) throws ",
            "{ let container = try decoder.container(keyedBy: CodingKeys.self); ",
            "self.fragileItemCodes = try container.decode([String].self, forKey: .fragileItemCodes); ",
            "self.itemCodeList = try container.decode([String].self, forKey: .itemCodeList); ",
            "self.label = try container.decode(String.self, forKey: .label) }; ",
            "public init(from decoder: Decoder) throws { ",
            "try self.init(swiftSchemaUnjudged: decoder); let violations = validate(); ",
            "if !violations.isEmpty { throw DecodingError.dataCorrupted(DecodingError.Context(",
            "codingPath: decoder.codingPath, ",
            r#"debugDescription: violations.joined(separator: "; "))) } }; "#,
            "public func validate() -> [String] { var errors: [String] = []; ",
            "errors += withoutDuplicates(itemCodeList).map ",
            r#"{ Parcel.swiftSchemaUnder("itemCodeList", $0) }; "#,
            "if label.unicodeScalars.count < 1 { errors.append(",
            r#""'label': too short: minimum length is 1, got \(label.unicodeScalars.count)") }; "#,
            "errors += withinParcel(self); return errors }; ",
            "private static func swiftSchemaUnder(_ place: String, _ violation: String) -> String ",
            r#"{ if violation.hasPrefix("'"), "#,
            r#"let end = violation.dropFirst().firstIndex(of: "'") { "#,
            r#"return "'\(place).\(violation[violation.index(after: violation.startIndex)..<end])'"#,
            r#"\(violation[violation.index(after: end)...])" }; "#,
            r#"return "'\(place)': \(violation)" } }; "#,
        )
    );
}

#[test]
fn a_holder_runs_what_it_holds_under_the_place_it_holds_it_at() {
    let swift = manifest_swift::swift_definition();
    for written in [
        r#"errors += firstParcel.validate().map { Manifest.swiftSchemaUnder("firstParcel", $0) }"#,
        concat!(
            "for (index, held) in parcelList.enumerated() { errors += held.validate().map ",
            r#"{ Manifest.swiftSchemaUnder("parcelList.\(index)", $0) } }"#,
        ),
        concat!(
            "for (key, held) in parcelsByDock.sorted(by: { $0.key < $1.key }) ",
            "{ errors += held.validate().map ",
            r#"{ Manifest.swiftSchemaUnder("parcelsByDock.\(key)", $0) } }"#,
        ),
        concat!(
            "if let held = spareParcel { errors += held.validate().map ",
            r#"{ Manifest.swiftSchemaUnder("spareParcel", $0) } }"#,
        ),
    ] {
        assert!(swift.contains(written), "{written} is not in: {swift}");
        assert!(
            include_str!("../../README.md").contains(written),
            "the README no longer shows: {written}"
        );
    }
}

/// One `validate()` per struct and enum the emitter writes, a wrapper of a map's keys aside: it is
/// the form a map is read and written in, and no value a client holds.
#[test]
fn every_emitted_type_has_validate() {
    for swift in [
        adjacent_tagged_swift::swift_definition(),
        all_widths_swift::swift_definition(),
        by_bool_swift::swift_definition(),
        by_enum_swift::swift_definition(),
        by_u32_swift::swift_definition(),
        correlation_id_swift::swift_definition(),
        envelope_swift::swift_definition(),
        external_tagged_swift::swift_definition(),
        flattened_extra_swift::swift_definition(),
        internal_tagged_swift::swift_definition(),
        reserved_member_swift::swift_definition(),
        reserved_members_swift::swift_definition(),
        status_swift::swift_definition(),
        tuple_point_swift::swift_definition(),
        untagged_swift::swift_definition(),
        user_data_swift::swift_definition(),
        window_request_swift::swift_definition(),
        with_flatten_swift::swift_definition(),
        with_nullable_swift::swift_definition(),
    ] {
        let declared = swift.matches("public struct ").count()
            + swift.matches("public enum ").count()
            - swift.matches("KeyedMap: Codable").count();
        assert_eq!(
            swift.matches("public func validate() -> [String]").count(),
            declared,
            "got: {swift}"
        );
        assert_eq!(
            swift.matches(NO_RULE).count(),
            declared,
            "a type with no rule, which holds none that has one, answers an empty list: {swift}"
        );
    }
}

#[test]
fn a_type_with_no_rule_keeps_the_encode_it_had() {
    let swift = window_request_swift::swift_definition();
    assert!(swift.contains(NO_RULE), "got: {swift}");
    assert!(!swift.contains("func encode(to encoder:"), "got: {swift}");
    let ruled = parcel_swift::swift_definition();
    assert!(ruled.contains("init(from decoder:"), "got: {ruled}");
    assert!(!ruled.contains("func encode(to encoder:"), "got: {ruled}");
}

#[test]
fn a_member_named_after_the_method_or_a_local_of_it_keeps_its_key() {
    let swift = inspection_swift::swift_definition();
    for written in [
        "public let errors: [String]; public let validate_: String; ",
        r#"case errors; case validate_ = "validate"; "#,
        "if self.errors.count < 1 { errors.append(",
        r#""'errors': too short: minimum length is 1, got \(self.errors.count)") }"#,
    ] {
        assert!(swift.contains(written), "{written} is not in: {swift}");
    }
    assert_eq!(
        serde_json::to_value(Inspection {
            errors: Vec::new(),
            validate: "v".to_owned(),
        })
        .unwrap(),
        serde_json::json!({ "errors": [], "validate": "v" })
    );
}

/// A brand's bounds are on the value it wraps, under no place, and a length is counted in
/// Unicode code points. A `$` is written `\z`, which ends the text where the `regex` crate's does.
#[test]
fn a_brand_runs_its_bounds_on_the_value_it_wraps() {
    let swift = slug_swift::swift_definition();
    for written in [
        concat!(
            "if value.unicodeScalars.count < 2 { errors.append(",
            r#""too short: minimum length is 2, got \(value.unicodeScalars.count)") }"#,
        ),
        concat!(
            "if value.unicodeScalars.count > 8 { errors.append(",
            r#""too long: maximum length is 8, got \(value.unicodeScalars.count)") }"#,
        ),
        concat!(
            r#"if value.range(of: "^[a-z]+\\z", options: .regularExpression) == nil "#,
            r#"{ errors.append("does not match pattern '^[a-z]+$'") }"#,
        ),
        "public var swiftSchemaBoundText: String { value }",
    ] {
        assert!(swift.contains(written), "{written} is not in: {swift}");
    }
    assert!(!swift.contains("swiftSchemaUnder"), "got: {swift}");
}

/// A brand over a brand answers its text through the brand it holds, and measures its own bound
/// on that text.
#[test]
fn a_brand_over_a_brand_answers_its_text_through_the_brand_it_holds() {
    let swift = sealed_slug_swift::swift_definition();
    for written in [
        "public let value: Slug; ",
        concat!(
            "if value.swiftSchemaBoundText.unicodeScalars.count > 6 { errors.append(",
            r#""too long: maximum length is 6, got "#,
            r#"\(value.swiftSchemaBoundText.unicodeScalars.count)") }"#,
        ),
        "public var swiftSchemaBoundText: String { value.swiftSchemaBoundText }",
    ] {
        assert!(swift.contains(written), "{written} is not in: {swift}");
    }
}

#[test]
fn a_brand_with_a_parameter_runs_its_bounds_at_its_declared_default() {
    let swift = marker_swift::swift_definition();
    let written = concat!(
        "if let held = value as? String { if held.unicodeScalars.count < 2 { errors.append(",
        r#""too short: minimum length is 2, got \(held.unicodeScalars.count)") } }"#,
    );
    assert!(swift.contains(written), "{written} is not in: {swift}");
}

/// A whole float is quoted back without `.0`, and a bound no value of the type can break is not
/// written: a comparison Swift knows to be false is a warning there.
#[test]
fn a_range_is_written_where_a_value_can_break_it() {
    let swift = reading_swift::swift_definition();
    for written in [
        r#"if age > 120 { errors.append("'age': too large: maximum is 120, got \(age)") }"#,
        concat!(
            "if level < 0.5 { errors.append(",
            r#""'level': too small: minimum is 0.5, got \(Reading.swiftSchemaPlain(level))") }"#,
        ),
        concat!(
            "if let held = spare { if held < 1 { errors.append(",
            r#""'spare': too small: minimum is 1, got \(held)") } }"#,
        ),
        concat!(
            "private static func swiftSchemaPlain<Number: BinaryFloatingPoint>",
            "(_ value: Number) -> String",
        ),
    ] {
        assert!(swift.contains(written), "{written} is not in: {swift}");
    }
    assert!(!swift.contains("age < 0"), "got: {swift}");
}

/// A brand is measured through the member its wrapper answers its text with, declared above the
/// field or below it, and an alias of a string declared above as the string it is.
#[test]
fn a_bound_on_a_named_type_measures_the_string_it_holds() {
    let swift = routing_swift::swift_definition();
    for written in [
        concat!(
            "if seenCode.swiftSchemaBoundText.unicodeScalars.count > 3 ",
            "{ errors.append(\"'seenCode': too long",
        ),
        "if titled.unicodeScalars.count < 2 { errors.append(\"'titled': too short",
        concat!(
            "if unseenCode.swiftSchemaBoundText.unicodeScalars.count < 2 ",
            "{ errors.append(\"'unseenCode': too short",
        ),
    ] {
        assert!(swift.contains(written), "{written} is not in: {swift}");
    }
}

/// A member of a variant is named by the steps serde writes on the way to it: the variant's name
/// around its payload, then the member's key or its position.
#[test]
fn an_enum_runs_the_rules_of_the_case_it_holds() {
    let swift = freight_swift::swift_definition();
    for written in [
        concat!(
            "public struct FreightLoosePayload: Codable, Sendable { public let freightLabel: ",
            "String; enum CodingKeys: String, CodingKey { case freightLabel;  }; ",
            "public init(freightLabel: String) { self.freightLabel = freightLabel;  }; ",
        ),
        concat!(
            "public func validate() -> [String] { var errors: [String] = []; ",
            "if freightLabel.unicodeScalars.count < 1 { errors.append(\"'freightLabel': too short",
        ),
        concat!(
            "switch self { case .boxed(let payload): if payload.unicodeScalars.count < 1 ",
            "{ errors.append(\"'Boxed': too short: minimum length is 1, ",
            r#"got \(payload.unicodeScalars.count)") }; "#,
            "case .loose(let payload): errors += payload.validate().map ",
            r#"{ Freight.swiftSchemaUnder("Loose", $0) }; "#,
            "case .split(let payload): errors += payload.validate().map ",
            r#"{ Freight.swiftSchemaUnder("Split", $0) }; default: break }"#,
        ),
        "if slot0.unicodeScalars.count < 1 { errors.append(\"'0': too short",
        r#"if slot1 < 1 { errors.append("'1': too small: minimum is 1, got \(slot1)") }"#,
    ] {
        assert!(swift.contains(written), "{written} is not in: {swift}");
    }
}

/// An enum's string check is the same check written on each field it reaches: a field with a
/// pattern of its own is held to both, its own first.
#[test]
fn an_enums_string_check_reaches_the_text_its_variants_hold() {
    let swift = pier_swift::swift_definition();
    for written in [
        concat!(
            r#"if quay.range(of: "^q", options: .regularExpression) == nil "#,
            r#"{ errors.append("'quay': does not match pattern '^q'") }; "#,
            r#"if quay.range(of: "^[a-z]+\\z", options: .regularExpression) == nil "#,
            r#"{ errors.append("'quay': does not match pattern '^[a-z]+$'") }"#,
        ),
        concat!(
            r#"case .tagged(let payload): if payload.range(of: "^[a-z]+\\z", "#,
            r#"options: .regularExpression) == nil "#,
            r#"{ errors.append("'Tagged': does not match pattern '^[a-z]+$'") }"#,
        ),
    ] {
        assert!(swift.contains(written), "{written} is not in: {swift}");
    }
}

#[test]
fn the_one_slot_of_a_struct_is_reported_under_no_step() {
    let swift = sealed_swift::swift_definition();
    let written = concat!(
        "if value.unicodeScalars.count < 1 { errors.append(",
        r#""too short: minimum length is 1, got \(value.unicodeScalars.count)") }"#,
    );
    assert!(swift.contains(written), "{written} is not in: {swift}");
}

#[test]
fn every_validated_fixture_is_built_and_written_by_serde() {
    let parcel = Parcel {
        fragile_item_codes: Vec::new(),
        item_code_list: vec!["x".to_owned()],
        label: "A".to_owned(),
    };
    let manifest = Manifest {
        first_parcel: parcel.clone(),
        parcel_list: vec![parcel.clone()],
        parcels_by_dock: HashMap::from([("north".to_owned(), parcel.clone())]),
        spare_parcel: Some(parcel),
    };
    assert_eq!(
        serde_json::to_value(&manifest).unwrap()["parcelsByDock"]["north"]["label"],
        "A"
    );
    assert_eq!(serde_json::to_value(Marker("ab".to_owned())).unwrap(), "ab");
    assert_eq!(
        serde_json::to_value(Reading {
            age: 4,
            level: 1.5,
            spare: None,
        })
        .unwrap(),
        serde_json::json!({ "age": 4_u32, "level": 1.5_f64 })
    );
    assert_eq!(
        serde_json::to_value(Routing {
            seen_code: Slug("ab".to_owned()),
            titled: Heading::from("ti"),
            unseen_code: LaterSlug("ab".to_owned()),
        })
        .unwrap(),
        serde_json::json!({ "seenCode": "ab", "titled": "ti", "unseenCode": "ab" })
    );
    assert_eq!(
        serde_json::to_value([
            Freight::Boxed("b".to_owned()),
            Freight::Loose {
                freight_label: "l".to_owned(),
            },
            Freight::Split("s".to_owned(), 1),
            Freight::Unladen,
        ])
        .unwrap(),
        serde_json::json!([
            { "Boxed": "b" },
            { "Loose": { "freightLabel": "l" } },
            { "Split": ["s", 1_u32] },
            "Unladen",
        ])
    );
    assert_eq!(
        serde_json::to_value([
            Pier::Named {
                quay: "q".to_owned(),
            },
            Pier::Tagged("t".to_owned()),
        ])
        .unwrap(),
        serde_json::json!([{ "Named": { "quay": "q" } }, { "Tagged": "t" }])
    );
    assert_eq!(serde_json::to_value(Sealed("s".to_owned())).unwrap(), "s");
    assert_eq!(
        serde_json::to_value(SealedSlug(Slug("ab".to_owned()))).unwrap(),
        "ab"
    );
}
