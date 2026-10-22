//! The omission walk is compiled in every build, so its tests are too — nothing here may reach
//! anything the `serde` feature gates.

use super::{SerdeKeyOmission, carries_untagged, has_serde_default, parse_serde_key_omission};

fn field_attrs(item: &syn::ItemStruct) -> &[syn::Attribute] {
    &item.fields.iter().next().unwrap().attrs
}

fn omission(item: &syn::ItemStruct) -> SerdeKeyOmission {
    parse_serde_key_omission(field_attrs(item))
}

#[test]
fn test_key_omission_set_by_serialization_skips() {
    let skip: syn::ItemStruct = syn::parse_quote! {
        struct S {
            #[serde(skip)]
            note: Option<String>,
        }
    };
    let skip_serializing: syn::ItemStruct = syn::parse_quote! {
        struct S {
            #[serde(skip_serializing)]
            note: Option<String>,
        }
    };
    let skip_serializing_if: syn::ItemStruct = syn::parse_quote! {
        struct S {
            #[serde(skip_serializing_if = "Option::is_none")]
            note: Option<String>,
        }
    };
    assert!(omission(&skip).omits_key);
    assert!(omission(&skip).skips_deserializing);
    assert!(omission(&skip_serializing).omits_key);
    assert!(!omission(&skip_serializing).skips_deserializing);
    assert!(omission(&skip_serializing_if).omits_key);
    assert!(!omission(&skip_serializing_if).skips_deserializing);
}

#[test]
fn test_key_omission_default_is_false() {
    let omission = SerdeKeyOmission::default();
    assert!(!omission.omits_key);
    assert!(!omission.skips_deserializing);
    assert!(!omission.defaulted);
}

/// A field's default is read in either spelling, since either one answers for a missing key.
#[test]
fn test_has_serde_default_reads_either_spelling() {
    let bare: syn::ItemStruct = syn::parse_quote! {
        struct S {
            #[serde(default)]
            note: Option<String>,
        }
    };
    let named: syn::ItemStruct = syn::parse_quote! {
        struct S {
            #[serde(default = "make_note")]
            note: Option<String>,
        }
    };
    let neither: syn::ItemStruct = syn::parse_quote! {
        struct S {
            #[serde(skip_serializing_if = "Option::is_none")]
            note: Option<String>,
        }
    };
    assert!(has_serde_default(field_attrs(&bare)));
    assert!(has_serde_default(field_attrs(&named)));
    assert!(!has_serde_default(field_attrs(&neither)));
}

#[test]
fn test_key_omission_not_set_by_skip_deserializing() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct S {
            #[serde(skip_deserializing)]
            note: Option<String>,
        }
    };
    assert!(!omission(&item).omits_key);
    assert!(omission(&item).skips_deserializing);
}

/// The walk reads every attribute in the list, wherever it sits.
#[test]
fn test_omission_keys_after_an_unread_value_are_still_read() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct S {
            #[serde(skip_serializing_if = "Option::is_none", default)]
            note: Option<String>,
        }
    };
    assert!(
        omission(&item).omits_key,
        "skip_serializing_if is the attribute read first"
    );
    assert!(
        omission(&item).defaulted,
        "default is written after an unread value"
    );
}

#[test]
fn test_skip_after_a_list_form_rename_is_still_read() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct S {
            #[serde(rename(serialize = "ser", deserialize = "de"), skip)]
            foo: String,
        }
    };
    assert!(
        omission(&item).omits_key,
        "skip is written after a list-form rename(...)"
    );
    assert!(omission(&item).skips_deserializing);
}

/// serde reads `untagged` wherever the list writes it, after a key with a value included.
#[test]
fn test_untagged_is_read_in_any_place_of_the_list() {
    let written: [syn::ItemEnum; 4] = [
        syn::parse_quote! {
            #[serde(untagged, rename_all = "camelCase")]
            enum E { FirstOne(String) }
        },
        syn::parse_quote! {
            #[serde(rename_all = "camelCase", untagged)]
            enum E { FirstOne(String) }
        },
        syn::parse_quote! {
            #[serde(rename_all(serialize = "camelCase", deserialize = "camelCase"), untagged)]
            enum E { FirstOne(String) }
        },
        syn::parse_quote! {
            #[serde(expecting = "a widget error", bound = "T: Clone", untagged)]
            enum E<T> { FirstOne(T) }
        },
    ];
    for item in &written {
        assert!(
            carries_untagged(&item.attrs),
            "untagged is read in any place: {}",
            quote::quote!(#item)
        );
    }
}

#[test]
fn test_a_list_without_the_untagged_key_is_not_untagged() {
    let tagged: syn::ItemEnum = syn::parse_quote! {
        #[serde(rename_all = "camelCase", tag = "untagged")]
        enum E { FirstOne(String) }
    };
    assert!(
        !carries_untagged(&tagged.attrs),
        "a value that reads `untagged` is no key"
    );
}
