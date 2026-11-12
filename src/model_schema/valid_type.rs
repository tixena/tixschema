//! The valid type: the declared name of every model type but a brand. The declared item is
//! emitted as `{Name}Draft`, which code builds and changes, and `{Name}` wraps a draft that passed
//! `validate()` in a private field.
//!
//! The wrapper is one generic struct in a hidden module nested in the type's schema module, where
//! no other module reaches its field, and the author's module names it through a type alias that
//! fills it with the draft. It holds no name the author's module declares, so a type declared in
//! a function body is wrapped as one declared beside it.

use core::cell::RefCell;

use proc_macro2::{Group, Ident, TokenStream, TokenTree};
use quote::{ToTokens as _, format_ident, quote};
use syn::ext::IdentExt as _;
use syn::punctuated::Punctuated;
use syn::{Attribute, Generics, Path, Token, Visibility};

use super::judged_read::unclaimed_parameter;
use super::{refusal_from_violations, unbounded};
use crate::features::serde::{consume_unread_value, derives_deserialize, derives_serialize};
use crate::utils::written_names;

thread_local! {
    static DECLARED: RefCell<Declared> = RefCell::new(Declared::default());
}

/// What the item being expanded declares of what the valid type repeats: read before the
/// expansion strips what it writes itself.
#[derive(Clone, Default)]
pub struct Declared {
    pub derives: Vec<Path>,
    pub deserializes: bool,
    pub name: Option<Ident>,
    pub origin: Origin,
    pub serializes: bool,
}

/// Who writes the type being expanded, which decides whether it has a draft.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Origin {
    /// A brand, which is built through `new` and has no draft.
    Brand,
    /// Any other struct or enum an author declares.
    #[default]
    Declared,
    /// A type a service writes for itself, declared as written.
    Service,
}

/// What the valid type of one declaration is written from.
pub struct ValidType<'parts> {
    pub derives: Vec<Path>,
    pub deserializes: bool,
    pub draft: Ident,
    pub generics: &'parts Generics,
    pub issues: &'parts TokenStream,
    pub module_ident: &'parts Ident,
    pub name: &'parts Ident,
    pub reads: &'parts TokenStream,
    pub serializes: bool,
    pub vis: &'parts Visibility,
}

impl ValidType<'_> {
    /// The name the author's module gives the valid type, and what the draft and the valid type
    /// do beside it.
    pub fn items(&self) -> TokenStream {
        let (name, draft) = (self.name, &self.draft);
        let (impl_generics, type_generics, where_clause) = self.generics.split_for_impl();
        let (vis, issues, reads) = (self.vis, self.issues, self.reads);
        let module_ident = self.module_ident;
        let sealed = sealed_ident(module_ident);
        let alias_generics = unbounded(self.generics);
        let writes = self.serializes.then(|| self.write());
        let read = self.deserializes.then(|| self.read());
        quote! {
            #vis type #name #alias_generics = #module_ident::#sealed::#name<#draft #type_generics>;

            impl #impl_generics #draft #type_generics #where_clause {
                #issues

                /// Judges the draft, and answers the valid value it makes or every rule it
                /// breaks.
                pub fn validate(self) -> ::core::result::Result<#name #type_generics, ::std::vec::Vec<::std::string::String>> {
                    let found = self.validation_issues();
                    if found.is_empty() {
                        ::core::result::Result::Ok(<#name #type_generics>::model_schema_wrap_unjudged(self))
                    } else {
                        ::core::result::Result::Err(found)
                    }
                }
            }

            impl #impl_generics ::core::convert::From<#name #type_generics> for #draft #type_generics #where_clause {
                fn from(valid: #name #type_generics) -> Self {
                    valid.into_draft()
                }
            }

            impl #impl_generics #name #type_generics #where_clause {
                /// What the draft's `validate()` would refuse, listed again.
                pub fn validation_issues(&self) -> ::std::vec::Vec<::std::string::String> {
                    <#draft #type_generics>::validation_issues(self)
                }
            }

            #writes

            #read

            #reads
        }
    }

    /// The hidden module the wrapper is written in, which goes inside the schema module.
    pub fn module(&self) -> TokenStream {
        let name = self.name;
        let sealed = sealed_ident(self.module_ident);
        let derives = &self.derives;
        let derive = (!derives.is_empty()).then(|| quote! { #[derive(#(#derives),*)] });
        let doc = format!("A `{}Draft` that passed `validate()`.", self.name);
        quote! {
            #[doc(hidden)]
            pub mod #sealed {
                #[doc = #doc]
                #derive
                pub struct #name<ModelSchemaDraft>(ModelSchemaDraft);

                impl<ModelSchemaDraft> ::core::ops::Deref for #name<ModelSchemaDraft> {
                    type Target = ModelSchemaDraft;

                    fn deref(&self) -> &ModelSchemaDraft {
                        &self.0
                    }
                }

                impl<ModelSchemaDraft> #name<ModelSchemaDraft> {
                    /// Gives the draft back, consuming the valid value.
                    pub fn into_draft(self) -> ModelSchemaDraft {
                        self.0
                    }

                    /// Wraps a draft with no judgement of its own, for the reads that hand the
                    /// whole value to be judged once.
                    pub fn model_schema_wrap_unjudged(draft: ModelSchemaDraft) -> Self {
                        Self(draft)
                    }
                }
            }
        }
    }

    /// The read of the valid type: the draft's own read, judged, and without the judgement for a
    /// holder, which judges the whole value once.
    #[cfg(feature = "serde")]
    fn read(&self) -> TokenStream {
        let (name, draft) = (self.name, &self.draft);
        let (impl_generics, type_generics, where_clause) = self.generics.split_for_impl();
        let written = written_names(quote!(#impl_generics #type_generics #where_clause));
        let reader = unclaimed_parameter("D", &written);
        let refusal = refusal_from_violations();
        let mut held = self.generics.clone();
        held.make_where_clause()
            .predicates
            .push(syn::parse_quote! { #draft #type_generics: ::serde::Deserialize<'de> });
        let mut read_at = held.clone();
        read_at.params.insert(0, syn::parse_quote! { 'de });
        let (read_impl_generics, _, read_where_clause) = read_at.split_for_impl();
        quote! {
            impl #impl_generics #name #type_generics #where_clause {
                /// The read alone, for a holder that judges the whole value once.
                pub fn model_schema_read_unjudged<'de, #reader>(deserializer: #reader) -> ::core::result::Result<Self, #reader::Error>
                where
                    #reader: ::serde::Deserializer<'de>,
                    #draft #type_generics: ::serde::Deserialize<'de>,
                {
                    <#draft #type_generics as ::serde::Deserialize<'de>>::deserialize(deserializer)
                        .map(Self::model_schema_wrap_unjudged)
                }
            }

            impl #read_impl_generics ::serde::Deserialize<'de> for #name #type_generics #read_where_clause {
                fn deserialize<#reader>(deserializer: #reader) -> ::core::result::Result<Self, #reader::Error>
                where
                    #reader: ::serde::Deserializer<'de>,
                {
                    let read = <#draft #type_generics as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                    let found = read.validation_issues();
                    if found.is_empty() {
                        ::core::result::Result::Ok(Self::model_schema_wrap_unjudged(read))
                    } else {
                        ::core::result::Result::Err((#refusal)(found))
                    }
                }
            }
        }
    }

    #[cfg(not(feature = "serde"))]
    fn read(&self) -> TokenStream {
        TokenStream::new()
    }

    /// The write of the valid type: the draft's.
    #[cfg(feature = "serde")]
    fn write(&self) -> TokenStream {
        let (name, draft) = (self.name, &self.draft);
        let (_, type_generics, _) = self.generics.split_for_impl();
        let mut written = self.generics.clone();
        written
            .make_where_clause()
            .predicates
            .push(syn::parse_quote! { #draft #type_generics: ::serde::Serialize });
        let (impl_generics, _, where_clause) = written.split_for_impl();
        quote! {
            impl #impl_generics ::serde::Serialize for #name #type_generics #where_clause {
                fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error> {
                    <#draft #type_generics as ::serde::Serialize>::serialize(self, serializer)
                }
            }
        }
    }

    #[cfg(not(feature = "serde"))]
    fn write(&self) -> TokenStream {
        TokenStream::new()
    }
}

/// The ident the declared item is emitted under.
pub fn draft_ident(name: &Ident) -> Ident {
    if !has_draft() {
        return name.clone();
    }
    format_ident!("{}Draft", name, span = name.span())
}

/// The name the declaration being expanded was written under, where `draft` is its draft's.
pub fn declared_name(draft: &Ident) -> Ident {
    DECLARED
        .with(|held| held.borrow().name.clone())
        .filter(|declared_as| !is_undrafted() && draft_ident(declared_as) == *draft)
        .unwrap_or_else(|| draft.clone())
}

/// Whether the declaration being expanded has a draft: it is no brand and no type a service
/// writes for itself.
pub fn has_draft() -> bool {
    DECLARED.with(|held| held.borrow().origin == Origin::Declared)
}

/// Whether the declaration being expanded is a type a service writes for itself, which has no
/// draft: it is declared as written, with the `validate()` it publishes.
pub fn is_undrafted() -> bool {
    DECLARED.with(|held| held.borrow().origin == Origin::Service)
}

/// The derives of `attrs` the wrapper repeats: every one but those that write or read a value
/// with no check, or build one from nothing.
pub fn wrapper_derives(attrs: &[Attribute]) -> Vec<Path> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("derive"))
        .filter_map(|attr| {
            attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
                .ok()
        })
        .flatten()
        .filter(|path| {
            path.segments.last().is_some_and(|segment| {
                !["Serialize", "Deserialize", "Default"]
                    .iter()
                    .any(|left_out| segment.ident == left_out)
            })
        })
        .collect()
}

/// Records what `item` declares, for the expansion of its shape.
pub fn record(item: &syn::Item, undrafted: bool) {
    let origin = if undrafted {
        Origin::Service
    } else if matches!(item, syn::Item::Struct(item_struct) if super::is_branded_newtype(item_struct))
    {
        Origin::Brand
    } else {
        Origin::Declared
    };
    let (attrs, name): (&[Attribute], Option<Ident>) = if let syn::Item::Struct(item_struct) = item
    {
        (&item_struct.attrs, Some(item_struct.ident.clone()))
    } else if let syn::Item::Enum(item_enum) = item {
        (&item_enum.attrs, Some(item_enum.ident.clone()))
    } else {
        (&[], None)
    };
    let declared = Declared {
        derives: wrapper_derives(attrs),
        deserializes: derives_deserialize(attrs),
        name,
        origin,
        serializes: derives_serialize(attrs),
    };
    DECLARED.with(|held| *held.borrow_mut() = declared);
}

/// What the item being expanded declares.
pub fn declared() -> Declared {
    DECLARED.with(|held| held.borrow().clone())
}

/// Names the draft as the declaration was, in the messages serde writes for it: the draft is
/// declared under another ident, and a message that names the type names what the author wrote.
pub fn named_as_declared(attrs: &mut Vec<Attribute>, declared_name: &Ident) {
    if is_undrafted() {
        return;
    }
    let derived = derives_serialize(attrs) || derives_deserialize(attrs);
    if !derived || renames_the_container(attrs) {
        return;
    }
    let written = declared_name.unraw().to_string();
    attrs.push(syn::parse_quote! { #[serde(rename = #written)] });
}

/// Whether a `#[serde(...)]` of `attrs` names the container under a key of its own.
fn renames_the_container(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("serde"))
        .any(|attr| {
            let mut renamed = false;
            let walked = attr.parse_nested_meta(|nested| {
                renamed |= nested.path.is_ident("rename");
                consume_unread_value(&nested)
            });
            walked.is_err() || renamed
        })
}

/// The methods a type with no draft has: the rules it breaks, and `validate()` where it has rules.
pub fn undrafted_methods(issues: &TokenStream, publishes: bool) -> TokenStream {
    let validate = publishes.then(|| {
        quote! {
            /// Validates all constrained fields and returns all validation errors.
            pub fn validate(&self) -> ::core::result::Result<(), ::std::vec::Vec<::std::string::String>> {
                let found = self.validation_issues();
                if found.is_empty() {
                    ::core::result::Result::Ok(())
                } else {
                    ::core::result::Result::Err(found)
                }
            }
        }
    });
    quote! {
        #issues

        #validate
    }
}

/// The ident of the hidden module of the schema module `module_ident`.
fn sealed_ident(module_ident: &Ident) -> Ident {
    format_ident!("__tixschema_valid_{}", module_ident)
}

/// Writes the valid type where a member of the draft writes `Self`: a member that holds the type
/// holds the valid one, whatever the draft is called.
pub fn self_names_the_valid_type<'field, Fields>(fields: Fields, name: &Ident, generics: &Generics)
where
    Fields: Iterator<Item = &'field mut syn::Field>,
{
    if is_undrafted() {
        return;
    }
    let (_, type_generics, _) = generics.split_for_impl();
    let named = quote! { #name #type_generics };
    for field in fields {
        let written = field.ty.to_token_stream();
        let rewritten = valid_for_self(written, &named);
        if let Ok(parsed) = syn::parse2::<syn::Type>(rewritten) {
            field.ty = parsed;
        }
    }
}

/// `tokens` with `named` written wherever they write `Self`.
fn valid_for_self(tokens: TokenStream, named: &TokenStream) -> TokenStream {
    tokens
        .into_iter()
        .flat_map(|token| match token {
            TokenTree::Group(group) => {
                let mut inner =
                    Group::new(group.delimiter(), valid_for_self(group.stream(), named));
                inner.set_span(group.span());
                TokenStream::from(TokenTree::Group(inner))
            }
            TokenTree::Ident(ident) if ident == "Self" => named.clone(),
            other @ (TokenTree::Ident(_) | TokenTree::Literal(_) | TokenTree::Punct(_)) => {
                TokenStream::from(other)
            }
        })
        .collect()
}
