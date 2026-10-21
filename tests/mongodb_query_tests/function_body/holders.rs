//! Holders of a type declared in the same function body: each path writes the key the same pair
//! writes at module level.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::shown;

#[test]
fn a_model_an_option_of_one_and_a_list_of_one_build_inside_a_function() {
    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct InnerA {
        pub name: String,
    }

    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct OuterA {
        pub inner: InnerA,
        pub many: Vec<InnerA>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub maybe: Option<InnerA>,
    }

    let outer = OuterA::MONGO_FIELDS;
    let named = || InnerA::MONGO_FIELDS.name().eq("kept".to_owned()).unwrap();
    assert_eq!(
        shown(outer.inner().nest(named())),
        r#"{ "inner.name": { "$eq": "kept" } }"#
    );
    assert_eq!(
        shown(outer.maybe().nest(named())),
        r#"{ "maybe.name": { "$eq": "kept" } }"#
    );
    assert_eq!(
        shown(outer.many().elem_match(named())),
        r#"{ "many": { "$elemMatch": { "name": { "$eq": "kept" } } } }"#
    );
    let inner = InnerA {
        name: "kept".to_owned(),
    };
    assert_eq!(
        shown(outer.inner().set(inner).unwrap()),
        r#"{ "$set": { "inner": { "name": "kept" } } }"#
    );
}

#[test]
fn a_tuple_struct_of_a_model_builds_inside_a_function() {
    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct InnerB {
        pub name: String,
    }

    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct PairB(pub InnerB, pub u32);

    let pair = PairB::MONGO_FIELDS;
    let named = InnerB::MONGO_FIELDS.name().eq("kept".to_owned()).unwrap();
    assert_eq!(
        shown(pair.0.nest(named)),
        r#"{ "0.name": { "$eq": "kept" } }"#
    );
    assert_eq!(
        shown(pair.1.gt(3_u32).unwrap()),
        r#"{ "1": { "$gt": Int64(3) } }"#
    );
}

#[test]
fn an_externally_tagged_enum_of_models_builds_inside_a_function() {
    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct InnerC {
        pub name: String,
    }

    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub enum HeldC {
        Named { inner: InnerC },
        One(InnerC),
        Slots(InnerC, u32),
    }

    let held = HeldC::MONGO_FIELDS;
    let named = || InnerC::MONGO_FIELDS.name().eq("kept".to_owned()).unwrap();
    assert_eq!(
        shown(held.named().inner().nest(named())),
        r#"{ "Named.inner.name": { "$eq": "kept" } }"#
    );
    assert_eq!(
        shown(held.slots().0.nest(named())),
        r#"{ "Slots.0.name": { "$eq": "kept" } }"#
    );
    assert_eq!(
        shown(held.slots().1.lt(9_u32).unwrap()),
        r#"{ "Slots.1": { "$lt": Int64(9) } }"#
    );
    assert_eq!(
        shown(held.one().nest(named())),
        r#"{ "One.name": { "$eq": "kept" } }"#
    );
    assert_eq!(shown(held.is_one()), r#"{ "One": { "$exists": true } }"#);
    let inner = InnerC {
        name: "kept".to_owned(),
    };
    assert_eq!(
        shown(held.one().set(inner).unwrap()),
        r#"{ "$set": { "One": { "name": "kept" } } }"#
    );
}

#[test]
fn an_internally_tagged_enum_of_models_builds_inside_a_function() {
    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct InnerD {
        pub name: String,
    }

    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(tag = "kind")]
    pub enum HeldD {
        Named { inner: InnerD },
        One(InnerD),
    }

    let held = HeldD::MONGO_FIELDS;
    let named = || InnerD::MONGO_FIELDS.name().eq("kept".to_owned()).unwrap();
    assert_eq!(
        shown(held.one().nest(named())),
        r#"{ "name": { "$eq": "kept" } }"#
    );
    assert_eq!(
        shown(held.named().inner().nest(named())),
        r#"{ "inner.name": { "$eq": "kept" } }"#
    );
    assert_eq!(shown(held.is_named()), r#"{ "kind": { "$eq": "Named" } }"#);
    assert_eq!(
        shown(held.kind__one().and(named())),
        r#"{ "$and": [{ "kind": { "$eq": "One" } }, { "name": { "$eq": "kept" } }] }"#
    );
}

#[test]
fn a_flattened_model_builds_inside_a_function() {
    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct InnerE {
        pub name: String,
    }

    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct OuterE {
        pub code: String,
        #[serde(flatten)]
        pub inner: InnerE,
    }

    let outer = OuterE::MONGO_FIELDS;
    let named = InnerE::MONGO_FIELDS.name().eq("kept".to_owned()).unwrap();
    assert_eq!(
        shown(outer.inner().nest(named)),
        r#"{ "name": { "$eq": "kept" } }"#
    );
    assert_eq!(
        shown(outer.code().eq("C-1".to_owned()).unwrap()),
        r#"{ "code": { "$eq": "C-1" } }"#
    );
}

/// A model declared below its holder is one whole value there, which `nest` still reaches.
#[test]
fn a_model_declared_below_its_holder_builds_inside_a_function() {
    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct OuterF {
        pub inner: InnerF,
    }

    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct InnerF {
        pub name: String,
    }

    let outer = OuterF::MONGO_FIELDS;
    let inner = InnerF {
        name: "kept".to_owned(),
    };
    assert_eq!(
        shown(outer.inner().eq(inner).unwrap()),
        r#"{ "inner": { "$eq": { "name": "kept" } } }"#
    );
    let named = InnerF::MONGO_FIELDS.name().eq("kept".to_owned()).unwrap();
    assert_eq!(
        shown(outer.inner().nest(named)),
        r#"{ "inner.name": { "$eq": "kept" } }"#
    );
}

#[test]
fn a_type_with_no_model_schema_held_inside_a_function_builds() {
    #[derive(Debug, Deserialize, Serialize)]
    pub struct PlainG {
        pub name: String,
    }

    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct OuterG {
        pub plain: PlainG,
    }

    let plain = PlainG {
        name: "kept".to_owned(),
    };
    assert_eq!(
        shown(OuterG::MONGO_FIELDS.plain().eq(plain).unwrap()),
        r#"{ "plain": { "$eq": { "name": "kept" } } }"#
    );
}

/// The slot's type asks the bound of `InnerH` to be well formed: the struct of paths declares
/// that its trait is filled, and the type's own scope fills it under that bound.
#[test]
fn a_tuple_slot_of_a_model_bound_on_a_trait_declared_inside_the_function_builds() {
    pub trait Coded {}
    impl Coded for String {}

    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct InnerH<C: Coded> {
        pub code: C,
    }

    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct PairH<C: Coded>(pub InnerH<C>, pub u32);

    let pair = PairH::<String>::MONGO_FIELDS;
    let coded = InnerH::<String>::MONGO_FIELDS
        .code()
        .eq("C-1".to_owned())
        .unwrap();
    assert_eq!(
        shown(pair.0.nest(coded)),
        r#"{ "0.code": { "$eq": "C-1" } }"#
    );
    assert_eq!(
        shown(pair.1.eq(4_u32).unwrap()),
        r#"{ "1": { "$eq": Int64(4) } }"#
    );
}
