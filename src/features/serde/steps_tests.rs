//! The steps serde writes on the way to a member, for each place a member is written at. The two
//! functions are compiled in every build, so their tests are too.

use syn::ext::IdentExt as _;

use super::{member_steps, variant_steps};

/// The steps of each member of `item`, under the casing `rename_all` names.
fn of_struct(item: &syn::ItemStruct, rename_all: Option<&str>) -> Vec<Vec<String>> {
    let lone = matches!(&item.fields, syn::Fields::Unnamed(slots) if slots.unnamed.len() == 1);
    member_steps(&item.fields, &[], rename_all, lone)
        .into_iter()
        .map(|(_, _, steps)| steps)
        .collect()
}

/// The steps of each member of the variant `named` of `item`, below what serde writes ahead of
/// them, under the casing `rename_all` names.
fn of_variant(item: &syn::ItemEnum, named: &str, rename_all: Option<&str>) -> Vec<Vec<String>> {
    let variant = item
        .variants
        .iter()
        .find(|variant| variant.ident.unraw() == named)
        .unwrap();
    let lone = matches!(&variant.fields, syn::Fields::Unnamed(slots) if slots.unnamed.len() == 1);
    member_steps(
        &variant.fields,
        &variant_steps(item, variant),
        rename_all,
        lone,
    )
    .into_iter()
    .map(|(_, _, steps)| steps)
    .collect()
}

fn steps(written: &[&[&str]]) -> Vec<Vec<String>> {
    written
        .iter()
        .map(|member| member.iter().map(|&step| step.to_owned()).collect())
        .collect()
}

/// An enum with a variant of each shape that holds a value, under no attribute.
#[cfg(feature = "serde")]
fn freight() -> syn::ItemEnum {
    syn::parse_quote! {
        enum Freight {
            Boxed(Parcel),
            Held,
            Loose { freight_label: String },
            Split(String, Parcel),
        }
    }
}

#[test]
fn a_slot_among_several_is_at_its_position_and_the_only_slot_is_the_value() {
    let docket: syn::ItemStruct = syn::parse_quote! {
        struct Docket(String, u32);
    };
    assert_eq!(of_struct(&docket, None), steps(&[&["0"], &["1"]]));
    let sealed: syn::ItemStruct = syn::parse_quote! {
        struct Sealed(Parcel);
    };
    assert_eq!(of_struct(&sealed, None), steps(&[&[]]));
}

#[test]
fn each_member_is_listed_beside_its_place_among_the_ones_declared() {
    let docket: syn::ItemStruct = syn::parse_quote! {
        struct Docket(String, u32);
    };
    let placed: Vec<usize> = member_steps(&docket.fields, &[], None, false)
        .into_iter()
        .map(|(_, index, _)| index)
        .collect();
    assert_eq!(placed, [0, 1]);
}

#[cfg(feature = "serde")]
#[test]
fn a_named_field_is_under_the_key_serde_writes() {
    let parcel: syn::ItemStruct = syn::parse_quote! {
        struct Parcel {
            item_label: String,
            #[serde(rename = "codes")]
            listed_item_codes: Vec<String>,
            r#type: String,
        }
    };
    assert_eq!(
        of_struct(&parcel, Some("camelCase")),
        steps(&[&["itemLabel"], &["codes"], &["type"]])
    );
    assert_eq!(
        of_struct(&parcel, None),
        steps(&[&["item_label"], &["codes"], &["type"]])
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_flattened_member_adds_no_step() {
    let manifest: syn::ItemStruct = syn::parse_quote! {
        struct Manifest {
            first_parcel: Parcel,
            #[serde(flatten)]
            starting_base: Base,
        }
    };
    assert_eq!(of_struct(&manifest, None), steps(&[&["first_parcel"], &[]]));
    let dispatch: syn::ItemEnum = syn::parse_quote! {
        enum Dispatch {
            ByRoad {
                #[serde(flatten)]
                starting_base: Base,
            },
        }
    };
    assert_eq!(of_variant(&dispatch, "ByRoad", None), steps(&[&["ByRoad"]]));
}

#[cfg(feature = "serde")]
#[test]
fn a_slot_serde_does_not_read_has_no_position_and_the_next_one_takes_it() {
    let measured: syn::ItemStruct = syn::parse_quote! {
        struct Measured(#[serde(skip)] u8, String, u32);
    };
    assert_eq!(of_struct(&measured, None), steps(&[&[], &["0"], &["1"]]));
    let stamped: syn::ItemStruct = syn::parse_quote! {
        struct Stamped {
            #[serde(skip_deserializing)]
            stamped_at: String,
        }
    };
    assert_eq!(of_struct(&stamped, None), steps(&[&["stamped_at"]]));
}

#[cfg(feature = "serde")]
#[test]
fn a_variant_with_no_tag_attribute_is_under_its_name() {
    let freight = freight();
    assert_eq!(of_variant(&freight, "Boxed", None), steps(&[&["Boxed"]]));
    assert_eq!(
        of_variant(&freight, "Loose", Some("camelCase")),
        steps(&[&["Loose", "freightLabel"]])
    );
    assert_eq!(
        of_variant(&freight, "Split", None),
        steps(&[&["Split", "0"], &["Split", "1"]])
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_variant_under_a_tag_and_a_content_key_is_under_the_content_key() {
    let mut haul = freight();
    haul.attrs
        .push(syn::parse_quote! { #[serde(tag = "kind", content = "held")] });
    assert_eq!(of_variant(&haul, "Boxed", None), steps(&[&["held"]]));
    assert_eq!(
        of_variant(&haul, "Loose", Some("camelCase")),
        steps(&[&["held", "freightLabel"]])
    );
    assert_eq!(
        of_variant(&haul, "Split", None),
        steps(&[&["held", "0"], &["held", "1"]])
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_variant_under_a_tag_alone_and_an_untagged_one_add_no_step() {
    for attribute in [
        syn::parse_quote! { #[serde(tag = "kind")] },
        syn::parse_quote! { #[serde(untagged)] },
    ] {
        let mut lift = freight();
        lift.attrs.push(attribute);
        assert_eq!(of_variant(&lift, "Boxed", None), steps(&[&[]]));
        assert_eq!(
            of_variant(&lift, "Loose", Some("camelCase")),
            steps(&[&["freightLabel"]])
        );
        assert_eq!(of_variant(&lift, "Split", None), steps(&[&["0"], &["1"]]));
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_variants_step_is_the_name_serde_writes_it_under() {
    let mooring: syn::ItemEnum = syn::parse_quote! {
        #[serde(rename_all = "snake_case")]
        enum Mooring {
            #[serde(rename = "atAnchor")]
            Anchored { anchor_code: String },
            DryDock(Parcel),
            r#Final(Parcel),
        }
    };
    assert_eq!(
        of_variant(&mooring, "Anchored", None),
        steps(&[&["atAnchor", "anchor_code"]])
    );
    assert_eq!(
        of_variant(&mooring, "DryDock", None),
        steps(&[&["dry_dock"]])
    );
    assert_eq!(of_variant(&mooring, "Final", None), steps(&[&["final"]]));
}

#[cfg(feature = "serde")]
#[test]
fn an_enum_no_variant_of_which_holds_a_value_writes_nothing_ahead_of_a_member() {
    let grade: syn::ItemEnum = syn::parse_quote! {
        enum Grade {
            Express,
            Standard,
        }
    };
    let express = grade.variants.first().unwrap();
    assert_eq!(variant_steps(&grade, express), Vec::<String>::new());
}

/// Without `serde` no attribute is read: a variant is under its own name and a member is as it
/// is declared.
#[cfg(not(feature = "serde"))]
#[test]
fn a_member_is_as_it_is_declared_and_a_variant_under_its_own_name() {
    let waybill: syn::ItemStruct = syn::parse_quote! {
        struct Waybill {
            #[serde(flatten)]
            carriage: Carriage,
            #[serde(rename = "codes")]
            listed_codes: Vec<String>,
        }
    };
    assert_eq!(
        of_struct(&waybill, Some("camelCase")),
        steps(&[&["carriage"], &["listed_codes"]])
    );
    let measured: syn::ItemStruct = syn::parse_quote! {
        struct Measured(#[serde(skip)] u8, String);
    };
    assert_eq!(of_struct(&measured, None), steps(&[&["0"], &["1"]]));
    let dispatch: syn::ItemEnum = syn::parse_quote! {
        #[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
        enum Dispatch {
            ByRoad { road_stops: Vec<String> },
            #[serde(rename = "sea")]
            BySea(String, Vec<String>),
            Carried(String),
        }
    };
    assert_eq!(
        of_variant(&dispatch, "ByRoad", None),
        steps(&[&["ByRoad", "road_stops"]])
    );
    assert_eq!(
        of_variant(&dispatch, "BySea", None),
        steps(&[&["BySea", "0"], &["BySea", "1"]])
    );
    assert_eq!(
        of_variant(&dispatch, "Carried", None),
        steps(&[&["Carried"]])
    );
}
