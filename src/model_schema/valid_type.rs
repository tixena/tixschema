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
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::{Attribute, Generics, Path, Token, Visibility};

use super::judged_read::unclaimed_parameter;
use super::refusal_from_violations;
use super::unbounded;
#[cfg(all(feature = "zod", feature = "serde"))]
use crate::features::serde::parse_serde_type_attributes;
use crate::features::serde::{consume_unread_value, derives_deserialize, derives_serialize};
#[cfg(feature = "zod")]
use crate::field_type::get_field_def;
#[cfg(any(feature = "typescript", all(feature = "serde", feature = "zod")))]
use crate::utils::record_drafted_type;
use crate::utils::written_names;

thread_local! {
    static DECLARED: RefCell<Declared> = RefCell::new(Declared::default());
}

/// What the item being expanded declares of what the valid type repeats: read before the
/// expansion strips what it writes itself.
#[derive(Clone, Default)]
pub struct Declared {
    pub conditional: Vec<(TokenStream, Vec<Path>)>,
    pub debugs: Gate,
    pub derives: Vec<Path>,
    pub deserializes: Gate,
    /// Whether a value of the type can be an array buffer view whole, which no freeze takes.
    #[cfg(feature = "zod")]
    pub holds_bytes_whole: bool,
    pub name: Option<Ident>,
    pub origin: Origin,
    pub serializes: Gate,
}

/// Whether, and under which `cfg`, an item derives a trait the valid type writes itself.
#[derive(Clone, Default)]
pub enum Gate {
    Always,
    #[default]
    Never,
    When(TokenStream),
}

impl Gate {
    /// `tokens` under the `cfg` this gate names, or nothing where it is `Never`.
    fn guard(&self, tokens: TokenStream) -> Option<TokenStream> {
        match self {
            Self::Never => None,
            Self::Always => Some(tokens),
            Self::When(condition) => Some(quote! { #[cfg(#condition)] #tokens }),
        }
    }

    /// This gate, or `Never` where `refused`.
    pub fn unless(self, refused: bool) -> Self {
        if refused { Self::Never } else { self }
    }
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
    pub conditional: Vec<(TokenStream, Vec<Path>)>,
    pub debugs: Gate,
    pub derives: Vec<Path>,
    pub deserializes: Gate,
    pub draft: Ident,
    pub generics: &'parts Generics,
    pub issues: &'parts TokenStream,
    pub module_ident: &'parts Ident,
    pub name: &'parts Ident,
    pub reads: &'parts TokenStream,
    pub serializes: Gate,
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
        let (writes, read) = (
            self.serializes.guard(self.write()),
            self.deserializes.guard(self.read()),
        );
        let valid_generics = self.valid_generics();
        let (valid_impl_generics, _, valid_where) = valid_generics.split_for_impl();
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

            impl #valid_impl_generics #name #type_generics #valid_where {
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
        let conditional = self.conditional.iter().map(|(condition, paths)| {
            quote! { #[cfg_attr(#condition, derive(#(#paths),*))] }
        });
        let doc = format!("A `{}Draft` that passed `validate()`.", self.name);
        let (draft_text, name_text) = (self.draft.to_string(), self.name.to_string());
        let debug = self.debugs.guard(quote! {
            impl<ModelSchemaDraft: ::core::fmt::Debug> ::core::fmt::Debug for #name<ModelSchemaDraft> {
                fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let shown = if formatter.alternate() {
                        ::std::format!("{:#?}", self.0)
                    } else {
                        ::std::format!("{:?}", self.0)
                    };
                    match shown.strip_prefix(#draft_text) {
                        ::core::option::Option::Some(rest) => {
                            formatter.write_str(#name_text)?;
                            formatter.write_str(rest)
                        }
                        ::core::option::Option::None => formatter.write_str(&shown),
                    }
                }
            }
        });
        quote! {
            #[doc(hidden)]
            pub mod #sealed {
                #[doc = #doc]
                #derive
                #(#conditional)*
                pub struct #name<ModelSchemaDraft>(ModelSchemaDraft);

                #debug

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
    fn read(&self) -> TokenStream {
        let (name, draft) = (self.name, &self.draft);
        let valid_generics = self.valid_generics();
        let (impl_generics, type_generics, where_clause) = valid_generics.split_for_impl();
        let written = written_names(quote!(#impl_generics #type_generics #where_clause));
        let reader = unclaimed_parameter("D", &written);
        let refusal = refusal_from_violations();
        let mut held = valid_generics.clone();
        held.make_where_clause()
            .predicates
            .push(syn::parse_quote! { #draft #type_generics: ::serde::Deserialize<'de> });
        let mut read_at = held.clone();
        read_at.params.insert(0, syn::parse_quote! { 'de });
        let (read_impl_generics, _, read_where_clause) = read_at.split_for_impl();
        let relabelled = self.relabelled();
        quote! {
          const _: () = {
            #relabelled
            fn relabel<E: ::serde::de::Error>(error: E) -> E {
                let text = ::std::string::ToString::to_string(&error);
                match relabelled(&text) {
                    ::core::option::Option::Some(relabelled) => E::custom(relabelled),
                    ::core::option::Option::None => error,
                }
            }

            impl #impl_generics #name #type_generics #where_clause {
                /// The read alone, for a holder that judges the whole value once.
                pub fn model_schema_read_unjudged<'de, #reader>(deserializer: #reader) -> ::core::result::Result<Self, #reader::Error>
                where
                    #reader: ::serde::Deserializer<'de>,
                    #draft #type_generics: ::serde::Deserialize<'de>,
                {
                    <#draft #type_generics as ::serde::Deserialize<'de>>::deserialize(deserializer)
                        .map(Self::model_schema_wrap_unjudged)
                        .map_err(relabel)
                }
            }

            impl #read_impl_generics ::serde::Deserialize<'de> for #name #type_generics #read_where_clause {
                fn deserialize<#reader>(deserializer: #reader) -> ::core::result::Result<Self, #reader::Error>
                where
                    #reader: ::serde::Deserializer<'de>,
                {
                    let read = <#draft #type_generics as ::serde::Deserialize<'de>>::deserialize(deserializer).map_err(relabel)?;
                    let found = read.validation_issues();
                    if found.is_empty() {
                        ::core::result::Result::Ok(Self::model_schema_wrap_unjudged(read))
                    } else {
                        ::core::result::Result::Err((#refusal)(found))
                    }
                }
            }
          };
        }
    }

    /// The function that words a serde message with the declared name where it names the draft.
    fn relabelled(&self) -> TokenStream {
        let (draft_text, declared_text) = (self.draft.to_string(), self.name.to_string());
        quote! {
            fn relabelled(text: &str) -> ::core::option::Option<::std::string::String> {
                let (draft_name, declared_name) = (#draft_text, #declared_text);
                let is_word = |character: char| character.is_alphanumeric() || character == '_';
                let mut relabelled = ::std::string::String::new();
                let mut rest = text;
                while let ::core::option::Option::Some(at) = rest.find(draft_name) {
                    let end = at + draft_name.len();
                    relabelled.push_str(&rest[..at]);
                    let whole = !relabelled.chars().next_back().is_some_and(is_word)
                        && !rest[end..].chars().next().is_some_and(is_word);
                    relabelled.push_str(if whole { declared_name } else { draft_name });
                    rest = &rest[end..];
                }
                relabelled.push_str(rest);
                (relabelled != text).then_some(relabelled)
            }
        }
    }

    /// The generics of the declaration as an impl on the valid type reads them: a bound written on
    /// `Self` is one of the draft.
    fn valid_generics(&self) -> Generics {
        let (_, type_generics, _) = self.generics.split_for_impl();
        let draft = &self.draft;
        let named = quote! { #draft #type_generics };
        let mut bounded = self.generics.clone();
        if let Some(clause) = bounded.where_clause.take() {
            let rewritten = valid_for_self(clause.to_token_stream(), &named);
            bounded.where_clause = syn::parse2(rewritten).ok();
        }
        bounded
    }

    /// The write of the valid type: the draft's.
    fn write(&self) -> TokenStream {
        let (name, draft) = (self.name, &self.draft);
        let (_, type_generics, _) = self.generics.split_for_impl();
        let mut written = self.valid_generics();
        written
            .make_where_clause()
            .predicates
            .push(syn::parse_quote! { #draft #type_generics: ::serde::Serialize });
        let (impl_generics, _, where_clause) = written.split_for_impl();
        let relabelled = self.relabelled();
        quote! {
            const _: () = {
                #relabelled
                fn relabel<E: ::serde::ser::Error>(error: E) -> E {
                    let text = ::std::string::ToString::to_string(&error);
                    match relabelled(&text) {
                        ::core::option::Option::Some(relabelled) => E::custom(relabelled),
                        ::core::option::Option::None => error,
                    }
                }

                impl #impl_generics ::serde::Serialize for #name #type_generics #where_clause {
                    fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error> {
                        <#draft #type_generics as ::serde::Serialize>::serialize(self, serializer).map_err(relabel)
                    }
                }
            };
        }
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
    let mut derives: Vec<Path> = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("derive"))
        .filter_map(|attr| {
            attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
                .ok()
        })
        .flatten()
        .filter(|path| {
            path.segments.last().is_some_and(|segment| {
                !["Serialize", "Deserialize", "Default", "Debug"]
                    .iter()
                    .any(|left_out| segment.ident == left_out)
            })
        })
        .collect();
    equal_whole(&mut derives);
    derives
}

/// `Eq` beside a `PartialEq` the wrapper derives: the wrapper is `Eq` wherever its draft is, and
/// derived alone `PartialEq` would be `clippy::derive_partial_eq_without_eq` on it.
fn equal_whole(derives: &mut Vec<Path>) {
    let names = |wanted: &str, held: &[Path]| {
        held.iter().any(|path| {
            path.segments
                .last()
                .is_some_and(|segment| segment.ident == wanted)
        })
    };
    if names("PartialEq", derives) && !names("Eq", derives) {
        derives.push(syn::parse_quote!(Eq));
    }
}

/// The derives written as `#[cfg_attr(condition, derive(..))]` the wrapper repeats, under the same
/// condition.
fn conditional_derives(attrs: &[Attribute]) -> Vec<(TokenStream, Vec<Path>)> {
    cfg_derives(attrs)
        .into_iter()
        .filter_map(|(condition, paths)| {
            let mut kept: Vec<Path> = paths.into_iter().filter(|path| !left_out(path)).collect();
            equal_whole(&mut kept);
            (!kept.is_empty()).then_some((condition, kept))
        })
        .collect()
}

/// Whether `attrs` derive `named`, unconditionally.
fn derives_named(attrs: &[Attribute], named: &str) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("derive"))
        .filter_map(|attr| {
            attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
                .ok()
        })
        .flatten()
        .any(|path| {
            path.segments
                .last()
                .is_some_and(|segment| segment.ident == named)
        })
}

/// Whether the wrapper leaves `path` to be written by hand or not at all.
fn left_out(path: &Path) -> bool {
    path.segments.last().is_some_and(|segment| {
        ["Serialize", "Deserialize", "Default", "Debug"]
            .iter()
            .any(|named| segment.ident == named)
    })
}

/// Every `#[cfg_attr(condition, derive(..))]` of `attrs`, with its condition and its paths.
fn cfg_derives(attrs: &[Attribute]) -> Vec<(TokenStream, Vec<Path>)> {
    let mut found = Vec::new();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("cfg_attr")) {
        let parsed = attr.parse_args_with(|input: ParseStream<'_>| {
            let condition: syn::Meta = input.parse()?;
            input.parse::<Token![,]>()?;
            let rest = Punctuated::<syn::Meta, Token![,]>::parse_terminated(input)?;
            Ok((condition, rest))
        });
        let Ok((condition, rest)) = parsed else {
            continue;
        };
        for meta in rest {
            if let syn::Meta::List(list) = meta
                && list.path.is_ident("derive")
                && let Ok(paths) =
                    list.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
            {
                found.push((quote! { #condition }, paths.into_iter().collect()));
            }
        }
    }
    found
}

/// Where `attrs` derive the trait `named`: always, under a `cfg_attr`, or never.
fn gate_of(attrs: &[Attribute], named: &str) -> Gate {
    let always = match named {
        "Serialize" => derives_serialize(attrs),
        "Deserialize" => derives_deserialize(attrs),
        _ => derives_named(attrs, named),
    };
    if always {
        return Gate::Always;
    }
    cfg_derives(attrs)
        .into_iter()
        .find(|(_, paths)| {
            paths
                .iter()
                .any(|path| path.segments.last().is_some_and(|last| last.ident == named))
        })
        .map_or(Gate::Never, |(condition, _)| Gate::When(condition))
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
        conditional: conditional_derives(attrs),
        debugs: gate_of(attrs, "Debug"),
        derives: wrapper_derives(attrs),
        deserializes: gate_of(attrs, "Deserialize"),
        #[cfg(feature = "zod")]
        holds_bytes_whole: holds_bytes_whole(item),
        name,
        origin,
        serializes: gate_of(attrs, "Serialize"),
    };
    #[cfg(any(feature = "typescript", all(feature = "serde", feature = "zod")))]
    if let Some(declared_name) = &declared.name {
        record_drafted_type(
            &declared_name.to_string(),
            declared.origin == Origin::Declared,
        );
    }
    DECLARED.with(|held| *held.borrow_mut() = declared);
}

/// Whether a value of `item` is its bytes, which `Object.freeze` refuses: a struct of one slot of
/// bytes, and an untagged enum with a variant of one slot of bytes.
#[cfg(feature = "zod")]
fn holds_bytes_whole(item: &syn::Item) -> bool {
    let bytes = |fields: &syn::Fields| match fields {
        syn::Fields::Unnamed(slots) if slots.unnamed.len() == 1 => slots
            .unnamed
            .first()
            .is_some_and(|slot| get_field_def("_slot", &slot.ty, "").writes_bytes()),
        syn::Fields::Named(_) | syn::Fields::Unnamed(_) | syn::Fields::Unit => false,
    };
    match item {
        syn::Item::Struct(item_struct) => bytes(&item_struct.fields),
        #[cfg(feature = "serde")]
        syn::Item::Enum(item_enum) => {
            parse_serde_type_attributes(&item_enum.attrs).untagged
                && item_enum
                    .variants
                    .iter()
                    .any(|variant| bytes(&variant.fields))
        }
        _ => false,
    }
}

/// Whether the valid type's schema freezes the value it parses.
#[cfg(feature = "zod")]
pub fn freezes() -> bool {
    DECLARED.with(|held| !held.borrow().holds_bytes_whole)
}

/// What the item being expanded declares.
pub fn declared() -> Declared {
    DECLARED.with(|held| held.borrow().clone())
}

/// Names the draft as the declaration was, in the messages serde writes for it: the draft is
/// declared under another ident, and a message that names the type names what the author wrote.
pub fn named_as_declared(attrs: &mut Vec<Attribute>, declared_name: &Ident) {
    if is_undrafted() || renames_the_container(attrs) {
        return;
    }
    let written = declared_name.unraw().to_string();
    if derives_serialize(attrs) || derives_deserialize(attrs) {
        attrs.push(syn::parse_quote! { #[serde(rename = #written)] });
        return;
    }
    let gated = [gate_of(attrs, "Serialize"), gate_of(attrs, "Deserialize")]
        .into_iter()
        .find_map(|gate| match gate {
            Gate::When(condition) => Some(condition),
            Gate::Always | Gate::Never => None,
        });
    if let Some(condition) = gated {
        attrs.push(syn::parse_quote! { #[cfg_attr(#condition, serde(rename = #written))] });
    }
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
