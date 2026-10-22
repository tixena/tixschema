//! `()` held by a brand and by a member: serde writes it as `null` and refuses an empty array, and
//! every surface describes it as `null`.

use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

/// A brand over `()`.
#[model_schema(no_display)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UnitBrand(());

/// A member typed `()` beside one typed with the brand.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitHolder {
    pub branded: UnitBrand,
    pub unit: (),
}

#[test]
fn serde_writes_a_unit_as_null_and_refuses_an_empty_array() {
    assert_eq!(serde_json::to_value(UnitBrand(())).unwrap(), json!(null));
    assert_eq!(
        serde_json::from_value::<UnitBrand>(json!(null)).unwrap(),
        UnitBrand(())
    );
    assert_eq!(
        serde_json::from_value::<UnitBrand>(json!([]))
            .unwrap_err()
            .to_string(),
        "invalid type: sequence, expected unit"
    );
    let holder = UnitHolder {
        branded: UnitBrand(()),
        unit: (),
    };
    assert_eq!(
        serde_json::to_value(&holder).unwrap(),
        json!({ "branded": null, "unit": null })
    );
    assert!(
        serde_json::from_value::<UnitHolder>(json!({ "branded": null, "unit": [] })).is_err(),
        "serde refuses an empty array for `()`"
    );
}

#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn typescript_describes_a_unit_as_null() {
    assert_eq!(
        UnitBrand::ts_definition(),
        "export type UnitBrand = null & $brand<\"UnitBrand\">;"
    );
    let holder = UnitHolder::ts_definition();
    assert!(holder.contains("  branded: UnitBrand;\n"), "{holder}");
    assert!(holder.contains("  unit: null;\n"), "{holder}");
}

#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn zod_describes_a_unit_as_null() {
    assert_eq!(
        UnitBrand::zod_schema(),
        "const UnitBrand$RawSchema = z.null().brand<\"UnitBrand\">().meta({\n  description: \
         \"A brand over `()`.\",\n  examples: [],\n});\n\nexport const UnitBrand$Schema: \
         typeof UnitBrand$RawSchema = UnitBrand$RawSchema;"
    );
    let holder = UnitHolder::zod_schema();
    assert!(
        holder.contains("  branded: UnitBrand$Schema,\n"),
        "{holder}"
    );
    assert!(holder.contains("  unit: z.null(),\n"), "{holder}");
}

#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_describes_a_unit_as_null() {
    assert_eq!(UnitBrand::json_schema(), json!({ "type": "null" }));
    assert_eq!(
        UnitHolder::json_schema(),
        json!({
            "type": "object",
            "properties": {
                "branded": { "type": "null" },
                "unit": { "type": "null" }
            },
            "required": ["branded", "unit"],
            "additionalProperties": false
        })
    );
}
