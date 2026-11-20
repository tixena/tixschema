//! The walks of `#[serde(...)]` attributes the surfaces and the read beside serde's derive ask.

use super::{
    SerdeKeyOmission, brand_slot, carries_untagged, derive_reads_a_default, derived_key_omission,
    derived_serde_attributes, has_serde_default, has_serde_skip_serializing, has_serde_transparent,
    parse_serde_key_omission, read_hook, serde_attributes, serde_conversion, tag_written_alone,
    variant_members_read, variant_on_the_wire, written_and_read_by_the_author,
};

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

#[test]
fn test_skip_takes_a_member_and_a_variant_off_the_wire_with_the_serde_feature() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct S {
            #[serde(skip)]
            note: Option<String>,
        }
    };
    let skipped: syn::Variant = syn::parse_quote! {
        #[serde(skip)]
        Hidden
    };

    assert!(omission(&item).absent_from_wire());
    assert!(!variant_on_the_wire(&skipped));
    assert!(!variant_members_read(&skipped));
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

#[test]
fn test_serde_attributes_yields_the_serde_attributes_with_the_feature_alone() {
    let item: syn::ItemStruct = syn::parse_quote! {
        #[derive(Debug)]
        #[serde(transparent)]
        #[doc = "A login."]
        #[serde(crate = "serde")]
        struct Username(String);
    };
    assert_eq!(
        serde_attributes(&item.attrs).count(),
        if true { 2 } else { 0 }
    );
    assert_eq!(derived_serde_attributes(&item.attrs).count(), 2);
}

#[test]
fn test_written_and_read_by_the_author_is_read_with_the_feature_alone() {
    let item: syn::ItemStruct = syn::parse_quote! {
        struct S {
            #[serde(with = "as_hex")]
            bytes: Vec<u8>,
        }
    };
    assert!(written_and_read_by_the_author(field_attrs(&item)));
}

#[test]
fn test_tag_written_alone_is_read_with_the_feature_alone() {
    let item: syn::ItemEnum = syn::parse_quote! {
        #[serde(tag = "kind")]
        enum E { Named { label: String } }
    };
    assert_eq!(
        tag_written_alone(&item.attrs),
        true.then(|| "kind".to_owned())
    );
}

#[test]
fn test_skip_serializing_is_read_with_the_feature_alone() {
    let written: [syn::ItemStruct; 2] = [
        syn::parse_quote! {
            struct S {
                #[serde(skip)]
                draft: String,
            }
        },
        syn::parse_quote! {
            struct S {
                #[serde(skip_serializing)]
                draft: String,
            }
        },
    ];
    for item in &written {
        assert!(
            has_serde_skip_serializing(field_attrs(item)),
            "{}",
            quote::quote!(#item)
        );
    }
}

/// A `#[serde(transparent)]` struct is a brand.
#[test]
fn test_transparent_is_read() {
    let item: syn::ItemStruct = syn::parse_quote! {
        #[serde(transparent)]
        struct Username(String);
    };
    assert!(has_serde_transparent(&item.attrs));
    assert!(brand_slot(&item).is_some());
}

/// The read the macro writes beside serde's derive reads what that derive reads, in every build.
#[test]
fn test_the_read_beside_serde_derive_reads_its_attributes_in_every_build() {
    let hooked: syn::ItemStruct = syn::parse_quote! {
        struct S {
            #[serde(default, deserialize_with = "read_note")]
            note: Option<String>,
        }
    };
    let skipped: syn::ItemStruct = syn::parse_quote! {
        struct S {
            #[serde(skip_deserializing)]
            note: String,
        }
    };
    let converted: syn::ItemStruct = syn::parse_quote! {
        #[serde(from = "RawNote")]
        struct S { note: String }
    };
    assert_eq!(
        read_hook(field_attrs(&hooked)).map(|(key, path)| (key, path.value())),
        Some(("deserialize_with", "read_note".to_owned()))
    );
    assert!(derive_reads_a_default(field_attrs(&hooked)));
    assert!(derived_key_omission(field_attrs(&skipped)).skips_deserializing);
    assert_eq!(
        serde_conversion(&converted.attrs).map(|(key, _)| key),
        Some("from")
    );
}
