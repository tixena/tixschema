//! The defaults of the members of an enum's variants, and the refusal of each one no target but
//! Rust reads.

use syn::{ItemEnum, parse_quote};

use super::{SerdeDefaults, default_refusals};

/// The message of each refusal `item` earns, in the order of its members.
fn refused(item: &ItemEnum) -> Vec<String> {
    default_refusals(item)
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// One index runs across every variant, keyed by the variant and the member, so two variants'
/// members of one name each have their own.
#[test]
fn one_index_runs_across_the_variants() {
    let defaults = SerdeDefaults::of_enum(&parse_quote! {
        #[derive(Deserialize, Serialize)]
        enum Berthing {
            Adrift,
            Docked {
                #[serde(default)]
                dock: String,
                pier: u32,
            },
            Moored {
                #[serde(default = "north_dock")]
                dock: String,
                #[serde(default)]
                line: String,
            },
        }
    });
    assert_eq!(defaults.index(Some("Docked"), "dock"), Some(0));
    assert_eq!(defaults.index(Some("Moored"), "dock"), Some(1));
    assert_eq!(defaults.index(Some("Moored"), "line"), Some(2));
    assert_eq!(defaults.index(Some("Docked"), "pier"), None);
    assert_eq!(defaults.index(None, "dock"), None);
}

/// A member whose type names a parameter is refused, and has no default, unless it is a list, a
/// set or a map under the bare `default`.
#[test]
fn a_member_that_names_a_parameter_is_refused_unless_it_is_an_empty_collection() {
    let item: ItemEnum = parse_quote! {
        #[derive(Deserialize, Serialize)]
        enum Berthing<T> {
            Docked {
                #[serde(default)]
                extra: T,
                #[serde(default)]
                moored: Vec<T>,
                #[serde(default = "two_berths")]
                berths: Vec<T>,
            },
        }
    };
    let said = refused(&item);
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(
        said[0].contains(
            "field `extra`: `#[serde(default)]` is read only by Rust here: the type names the \
             type parameter `T`"
        ),
        "{said:?}"
    );
    assert!(said[1].contains("field `berths`:"), "{said:?}");
    let defaults = SerdeDefaults::of_enum(&item);
    assert_eq!(defaults.index(Some("Docked"), "moored"), Some(0));
    assert_eq!(defaults.index(Some("Docked"), "extra"), None);
}

/// A const parameter, a lifetime and `Self` are each named in the refusal.
#[test]
fn a_const_parameter_a_lifetime_and_self_are_each_named() {
    let said = refused(&parse_quote! {
        #[derive(Deserialize, Serialize)]
        enum Berthing<'held, const N: usize> {
            Docked {
                #[serde(default)]
                slots: [u8; N],
                #[serde(default)]
                dock: &'held str,
                #[serde(default)]
                next: Box<Self>,
                #[serde(default)]
                queue: Vec<Self>,
            },
        }
    });
    assert_eq!(said.len(), 3, "{said:?}");
    assert!(
        said[0].contains(
            "the type names the const parameter `N`, whose default has one JSON for each value \
             that fills it. Give the field a type that names no parameter, or remove the default."
        ),
        "{said:?}"
    );
    assert!(
        said[1].contains(
            "the type names the lifetime `'held`, and the JSON of the default is built outside \
             the enum, where no lifetime of it is in scope. Give the field a type that names no \
             lifetime, or remove the default."
        ),
        "{said:?}"
    );
    assert!(
        said[2].contains(
            "the type names `Self`, and the JSON of the default is built outside the enum, where \
             `Self` names nothing of it. Write the enum's name in place of `Self`, or remove the \
             default."
        ),
        "{said:?}"
    );
}

/// The refusal of a member serde leaves out of what it writes names the attribute written.
#[test]
fn a_member_left_out_of_what_serde_writes_is_refused_under_the_key_written() {
    let said = refused(&parse_quote! {
        #[derive(Deserialize, Serialize)]
        enum Berthing {
            Docked {
                #[serde(default, skip_serializing)]
                dock: String,
                #[serde(skip_serializing_if = "String::is_empty", default)]
                line: String,
            },
        }
    });
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(
        said[0].contains("beside `skip_serializing` is read"),
        "{said:?}"
    );
    assert!(
        said[1].contains("beside `skip_serializing_if` is read"),
        "{said:?}"
    );
}

/// An enum that writes itself through `into` or `remote` is refused each member under a default,
/// as one that derives no `Serialize` is.
#[test]
fn an_enum_written_through_another_type_is_refused_each_default() {
    for item in [
        parse_quote! {
            #[derive(Deserialize, Serialize)]
            #[serde(into = "Written")]
            enum Berthing { Docked { #[serde(default)] dock: String } }
        },
        parse_quote! {
            #[derive(Deserialize, Serialize)]
            #[serde(remote = "Written")]
            enum Berthing { Docked { #[serde(default)] dock: String } }
        },
    ] {
        let said = refused(&item);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(
            said[0].contains("needs the enum to derive `Serialize`, with no `into` or `remote`"),
            "{said:?}"
        );
        assert_eq!(
            SerdeDefaults::of_enum(&item).index(Some("Docked"), "dock"),
            None
        );
    }
}

/// A member serde never reads the key of, an `Option`, a variant serde never reads or reads
/// through a function, and an enum serde never reads or reads through another type are refused
/// nothing, and hold no default.
#[test]
fn what_serde_reads_no_default_of_is_refused_nothing() {
    for item in [
        parse_quote! {
            #[derive(Deserialize, Serialize)]
            enum Berthing {
                Docked {
                    #[serde(default, skip)]
                    dock: String,
                    #[serde(default, skip_deserializing)]
                    line: String,
                    #[serde(default, skip_serializing_if = "Option::is_none")]
                    pier: Option<u32>,
                    #[serde(flatten, skip_serializing_if = "Option::is_none", default)]
                    mooring: Option<Mooring>,
                },
                #[serde(skip_deserializing)]
                Moored { #[serde(default, skip_serializing_if = "String::is_empty")] dock: String },
                #[serde(deserialize_with = "moored")]
                Towed { #[serde(default, skip_serializing_if = "String::is_empty")] dock: String },
            }
        },
        parse_quote! {
            #[derive(Serialize)]
            enum Berthing { Docked { #[serde(default, skip_serializing)] dock: String } }
        },
        parse_quote! {
            #[derive(Deserialize, Serialize)]
            #[serde(from = "Read")]
            enum Berthing { Docked { #[serde(default, skip_serializing)] dock: String } }
        },
        parse_quote! {
            #[derive(Deserialize, Serialize)]
            #[serde(try_from = "Read")]
            enum Berthing { Docked { #[serde(default, skip_serializing)] dock: String } }
        },
    ] {
        assert_eq!(refused(&item), Vec::<String>::new());
        let defaults = SerdeDefaults::of_enum(&item);
        for (variant, field) in [
            ("Docked", "dock"),
            ("Docked", "line"),
            ("Docked", "pier"),
            ("Docked", "mooring"),
            ("Moored", "dock"),
            ("Towed", "dock"),
        ] {
            assert_eq!(defaults.index(Some(variant), field), None);
        }
    }
}
