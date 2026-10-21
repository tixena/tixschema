//! Flagged types declared inside a function body, where a module written beside a type reaches
//! none of the function's items: every path writes the key the same shape writes at module level.

// The JSON Schema of a holder names the schema module of the type it holds.
#[cfg(not(feature = "jsonschema"))]
mod holders;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::shown;

#[test]
fn a_generic_struct_declared_inside_a_function_builds() {
    #[model_schema(enable = ["decode_with", "mongodb"], default_types(T = String))]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Wrapped<T> {
        pub count: u32,
        pub value: T,
    }

    let wrapped = Wrapped::<u32>::MONGO_FIELDS;
    assert_eq!(
        shown(wrapped.value().gt(5_u32).unwrap()),
        r#"{ "value": { "$gt": Int64(5) } }"#
    );
    assert_eq!(
        shown(wrapped.count().eq(2_u32).unwrap()),
        r#"{ "count": { "$eq": Int64(2) } }"#
    );
}

#[test]
fn a_struct_bound_on_a_trait_declared_inside_the_function_builds() {
    pub trait Coded {}
    impl Coded for String {}

    #[model_schema(enable = ["decode_with", "mongodb"], default_types(C = String))]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Tagged<C: Coded> {
        pub code: C,
        pub count: u32,
    }

    let tagged = Tagged::<String>::MONGO_FIELDS;
    assert_eq!(
        shown(tagged.code().eq("C-1".to_owned()).unwrap()),
        r#"{ "code": { "$eq": "C-1" } }"#
    );
    assert_eq!(
        shown(tagged.count().eq(3_u32).unwrap()),
        r#"{ "count": { "$eq": Int64(3) } }"#
    );
}

/// Each slot's type is read through a trait the type's own scope fills, under the type's bounds.
#[test]
fn a_bounded_tuple_struct_and_an_enum_of_slots_declared_inside_a_function_build() {
    pub trait Coded {}
    impl Coded for String {}

    #[model_schema(enable = ["decode_with", "mongodb"], default_types(C = String))]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Couple<C: Coded>(pub C, pub u32);

    #[model_schema(enable = ["decode_with", "mongodb"], default_types(C = String))]
    #[derive(Debug, Deserialize, Serialize)]
    pub enum Slotted<C: Coded> {
        Lone(C),
        Pair(C, u32),
        PairSlots(u32, u32),
    }

    let couple = Couple::<String>::MONGO_FIELDS;
    assert_eq!(
        shown(couple.0.eq("C-1".to_owned()).unwrap()),
        r#"{ "0": { "$eq": "C-1" } }"#
    );
    assert_eq!(
        shown(couple.1.gte(2_u32).unwrap()),
        r#"{ "1": { "$gte": Int64(2) } }"#
    );
    let slotted = Slotted::<String>::MONGO_FIELDS;
    assert_eq!(
        shown(slotted.pair().0.eq("C-1".to_owned()).unwrap()),
        r#"{ "Pair.0": { "$eq": "C-1" } }"#
    );
    assert_eq!(
        shown(slotted.pair_slots().1.lt(3_u32).unwrap()),
        r#"{ "PairSlots.1": { "$lt": Int64(3) } }"#
    );
    assert_eq!(
        shown(slotted.lone().eq("C-1".to_owned()).unwrap()),
        r#"{ "Lone": { "$eq": "C-1" } }"#
    );
}
