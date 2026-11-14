//! The valid type: the declared name of every model type but a brand and an enum with no check
//! that serde writes as a variant name. The declared item is
//! emitted as `{Name}Draft`, which code builds and changes, and `{Name}` wraps a draft that passed
//! `validate()` in a private field.
//!
//! The wrapper is one generic struct in a hidden module nested in the type's schema module, where
//! no other module reaches its field, and the author's module names it through a type alias that
//! fills it with the draft. It holds no name the author's module declares, so a type declared in
//! a function body is wrapped as one declared beside it.

use crate::utils::is_rule_less;
use core::cell::RefCell;

use proc_macro2::{Group, Ident, Literal, TokenStream, TokenTree};
use quote::{ToTokens as _, format_ident, quote};
use syn::ext::IdentExt as _;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::{Attribute, Generics, Path, Token, Visibility};

use super::refusal_from_violations;
use super::unbounded;
use super::{LiteralLeaf, literal_shape, sole_type_argument};
use crate::features::model_schema_prop::{LiteralValue, parse_model_schema_prop_attributes};
#[cfg(all(feature = "zod", feature = "serde"))]
use crate::features::serde::parse_serde_type_attributes;
use crate::features::serde::{consume_unread_value, derives_deserialize, derives_serialize};
#[cfg(feature = "zod")]
use crate::field_type::get_field_def;
use crate::field_type::held_literal;
#[cfg(any(feature = "typescript", all(feature = "serde", feature = "zod")))]
use crate::utils::record_drafted_type;
use crate::utils::unclaimed_parameter;
use crate::utils::written_type;
use crate::utils::{record_undrafted_enum, written_names};

/// The most parameters a `new` takes unless the author asks for it with `new = true`, which is
/// where clippy's `too_many_arguments` starts to fire.
const MOST_PARAMETERS: usize = 7;

/// Opens, and closes, the text of a surface that a type with no rule alone has.
pub const SETTLE_OPEN: char = '\u{e000}';
pub const SETTLE_CLOSE: char = '\u{e001}';

thread_local! {
    static DECLARED: RefCell<Declared> = RefCell::new(Declared::default());
}

/// What the item being expanded declares of what the valid type repeats: read before the
/// expansion strips what it writes itself.
#[derive(Clone, Default)]
pub struct Declared {
    /// Whether the type names a check of its own, on itself.
    pub checked: bool,
    pub conditional: Vec<(TokenStream, Vec<Path>)>,
    /// The fields of a struct, which the valid type's `new` takes.
    pub constructor: Constructor,
    pub debugs: Gate,
    pub derives: Vec<Path>,
    pub deserializes: Gate,
    /// Whether a value of the type can be an array buffer view whole, which no freeze takes.
    #[cfg(feature = "zod")]
    pub holds_bytes_whole: bool,
    pub name: Option<Ident>,
    pub origin: Origin,
    pub serializes: Gate,
    /// Whether the item is a struct with no braces and no parameter.
    #[cfg(feature = "typescript")]
    pub unit: bool,
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
    /// An enum serde writes as one of its variant names, with no check of its own: every value of
    /// its draft would be valid, so it is declared as written.
    Plain,
    /// A type a service writes for itself, declared as written.
    Service,
}

/// The values of a type that no rule can refuse, which the valid type holds as constants.
#[derive(Clone, Default)]
pub enum UnitValues {
    #[default]
    None,
    /// The one value of a unit struct.
    Struct,
    /// The unit variants of an enum, each with the `cfg` attributes it is declared under.
    Variants(Vec<(Vec<Attribute>, Ident)>),
}

/// One field of a struct, as the valid type's `new` takes it.
#[derive(Clone)]
pub struct Slot {
    /// The `cfg` attributes the field is declared under, which its parameter and its value repeat.
    pub cfgs: Vec<String>,
    /// The value `new` writes for a field a `literal` fixes, which is then no parameter. This and
    /// `ty` are text: the thread-local that holds them outlives the expansion's token handles.
    pub fill: Option<String>,
    /// The field's name, or `None` for a tuple field.
    pub member: Option<String>,
    pub ty: String,
}

/// Who decides whether a struct's valid type has a `new`.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum NewBy {
    /// `new = false`: the author writes it, or wants none.
    Author,
    /// Nothing written: the macro writes it for a struct of at most seven parameters.
    #[default]
    Default,
    /// `new = true`: the macro writes it however many parameters it takes.
    Written,
}

/// The value a `literal` fixes in a field whose own type holds it directly, which no client's
/// constructor takes as a parameter.
pub enum Fixed {
    Bool(bool),
    /// A number as its digits, and the Rust number type that holds it.
    Number(String, &'static str),
    Text(String),
}

/// The fields `new` builds the draft from.
#[derive(Clone, Default)]
pub enum Constructor {
    Named(Vec<Slot>),
    #[default]
    None,
    Tuple(Vec<Slot>),
    Unit,
}

/// What the valid type of one declaration is written from.
pub struct ValidType<'parts> {
    pub conditional: Vec<(TokenStream, Vec<Path>)>,
    pub constructor: Constructor,
    pub debugs: Gate,
    pub derives: Vec<Path>,
    pub deserializes: Gate,
    pub draft: Ident,
    pub generics: &'parts Generics,
    pub issues: &'parts TokenStream,
    pub module_ident: &'parts Ident,
    pub name: &'parts Ident,
    pub reads: &'parts TokenStream,
    /// Whether the type declares no rule of its own, so a draft becomes valid with no judgement.
    pub rule_less: bool,
    pub serializes: Gate,
    pub units: UnitValues,
    pub vis: &'parts Visibility,
}

/// A declaration that may hold values no rule can refuse.
pub trait UnitShaped {
    fn unit_values(&self) -> UnitValues;
}

impl UnitShaped for syn::ItemStruct {
    fn unit_values(&self) -> UnitValues {
        if matches!(self.fields, syn::Fields::Unit) && self.generics.params.is_empty() {
            UnitValues::Struct
        } else {
            UnitValues::None
        }
    }
}

impl UnitShaped for syn::ItemEnum {
    fn unit_values(&self) -> UnitValues {
        if !self.generics.params.is_empty() {
            return UnitValues::None;
        }
        let units: Vec<(Vec<Attribute>, Ident)> = self
            .variants
            .iter()
            .filter(|variant| matches!(variant.fields, syn::Fields::Unit))
            .map(|variant| {
                let cfgs = variant
                    .attrs
                    .iter()
                    .filter(|attr| attr.path().is_ident("cfg"))
                    .cloned()
                    .collect();
                (cfgs, variant.ident.clone())
            })
            .collect();
        if units.is_empty() {
            UnitValues::None
        } else {
            UnitValues::Variants(units)
        }
    }
}

impl Constructor {
    /// How many parameters `new` takes: every field a `literal` does not fix.
    fn parameters(&self) -> usize {
        match self {
            Self::Named(slots) | Self::Tuple(slots) => {
                slots.iter().filter(|slot| slot.fill.is_none()).count()
            }
            Self::None | Self::Unit => 0,
        }
    }
}

impl ValidType<'_> {
    /// The valid value of each unit variant, and of a unit struct, as a constant named like it.
    fn constants(&self) -> Option<TokenStream> {
        let (name, draft, vis) = (self.name, &self.draft, self.vis);
        let reason = "named like its variant, so it reads and matches as the variant";
        match &self.units {
            UnitValues::None => None,
            UnitValues::Struct => {
                let doc = format!("The one value of `{name}`.");
                let expected = (!is_upper_case(name)).then(|| {
                    quote! { #[expect(non_upper_case_globals, reason = #reason)] }
                });
                Some(quote! {
                    #[doc = #doc]
                    #expected
                    #vis const #name: #name = <#name>::model_schema_wrap_unjudged(#draft);
                })
            }
            UnitValues::Variants(variants) => {
                let constants = variants.iter().map(|(cfgs, variant)| {
                    let doc = format!("The `{variant}` value of `{name}`.");
                    quote! {
                        #(#cfgs)*
                        #[doc = #doc]
                        pub const #variant: Self = Self::model_schema_wrap_unjudged(#draft::#variant);
                    }
                });
                let expected = variants
                    .iter()
                    .any(|(_, variant)| !is_upper_case(variant))
                    .then(|| quote! { #[expect(non_upper_case_globals, reason = #reason)] });
                Some(quote! {
                    #expected
                    impl #name {
                        #(#constants)*
                    }
                })
            }
        }
    }

    /// `new`, which builds the draft from its fields and judges it as `validate()` does.
    fn constructor(&self) -> Option<TokenStream> {
        let (name, draft) = (self.name, &self.draft);
        let (_, type_generics, _) = self.generics.split_for_impl();
        let valid_generics = self.valid_generics();
        let (impl_generics, _, where_clause) = valid_generics.split_for_impl();
        let (slots, named) = match &self.constructor {
            Constructor::None => return None,
            Constructor::Unit => (&[][..], None),
            Constructor::Named(slots) => (slots.as_slice(), Some(true)),
            Constructor::Tuple(slots) => (slots.as_slice(), Some(false)),
        };
        let one_slot = slots.len() == 1;
        let bindings: Vec<TokenStream> = slots
            .iter()
            .enumerate()
            .map(|(index, slot)| match &slot.member {
                Some(member) => text_tokens(member),
                None if one_slot => quote! { value },
                None => format_ident!("value{}", index).to_token_stream(),
            })
            .collect();
        let parameters = slots
            .iter()
            .zip(&bindings)
            .filter(|(slot, _)| slot.fill.is_none())
            .map(|(slot, binding)| {
                let ty = text_tokens(&slot.ty);
                let cfgs = slot.cfgs.iter().map(|cfg| text_tokens(cfg));
                quote! { #(#cfgs)* #binding: #ty }
            });
        let values = slots.iter().zip(&bindings).map(|(slot, binding)| {
            slot.fill
                .as_deref()
                .map_or_else(|| binding.clone(), text_tokens)
        });
        let built = match named {
            None => quote! { #draft },
            Some(true) => {
                let fields = slots.iter().zip(&bindings).map(|(slot, binding)| {
                    let cfgs = slot.cfgs.iter().map(|cfg| text_tokens(cfg));
                    let value = slot.fill.as_deref().map_or_else(
                        || binding.clone(),
                        |text| {
                            let value = text_tokens(text);
                            quote! { #binding: #value }
                        },
                    );
                    quote! { #(#cfgs)* #value }
                });
                quote! { #draft { #(#fields),* } }
            }
            Some(false) => quote! { #draft(#(#values),*) },
        };
        let wide = (self.constructor.parameters() > MOST_PARAMETERS)
            .then(|| quote! { #[allow(clippy::too_many_arguments)] });
        let doc = format!(
            "Builds a valid `{name}` from its fields, judged as `{draft}::validate()` judges them."
        );
        Some(quote! {
            impl #impl_generics #name #type_generics #where_clause {
                #[doc = #doc]
                #wide
                pub fn new(#(#parameters),*) -> ::core::result::Result<#name #type_generics, ::std::vec::Vec<::std::string::String>> {
                    #built.validate()
                }
            }
        })
    }

    /// The conversion from a draft that no rule judges.
    fn conversion(&self) -> Option<TokenStream> {
        let (name, draft) = (self.name, &self.draft);
        let (impl_generics, type_generics, where_clause) = self.generics.split_for_impl();
        self.rule_less.then(|| {
            quote! {
                impl #impl_generics ::core::convert::From<#draft #type_generics> for #name #type_generics #where_clause {
                    fn from(draft: #draft #type_generics) -> Self {
                        Self::model_schema_wrap_unjudged(draft)
                    }
                }
            }
        })
    }

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
        let (conversion, constants, constructor) =
            (self.conversion(), self.constants(), self.constructor());
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

            #conversion

            #constants

            #constructor

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
                    pub const fn model_schema_wrap_unjudged(draft: ModelSchemaDraft) -> Self {
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

/// Whether `item` declares no rule of its own. A rule is a constraint in a `model_schema_prop`, a
/// check on the type or on a member, a string check on the type, or a literal; a brand has no
/// draft. A member typed with another type is not a rule: that type judges itself.
pub fn declares_no_rule(item: &syn::Item, checks: bool, string_check: bool) -> bool {
    if checks {
        return false;
    }
    let ruled = |fields: &syn::Fields| {
        fields.iter().any(|field| {
            let meta = parse_model_schema_prop_attributes(&field.attrs);
            !meta.check.is_empty()
                || meta.literal.is_some()
                || meta.min_length.is_some()
                || meta.max_length.is_some()
                || meta.min_items.is_some()
                || meta.max_items.is_some()
                || meta.minimum.is_some()
                || meta.maximum.is_some()
                || meta.pattern.is_some()
                || meta.enum_pattern.is_some()
        })
    };
    if let syn::Item::Struct(item_struct) = item {
        return !string_check && !ruled(&item_struct.fields);
    }
    if let syn::Item::Enum(item_enum) = item {
        // A string check holds the names of an enum of unit variants, which expansion has
        // already measured.
        let holds_values = item_enum
            .variants
            .iter()
            .any(|variant| !matches!(variant.fields, syn::Fields::Unit));
        let is_ruled = item_enum
            .variants
            .iter()
            .any(|variant| ruled(&variant.fields));
        return !(string_check && holds_values || is_ruled);
    }
    true
}

/// `tokens` with the text between [`SETTLE_OPEN`] and [`SETTLE_CLOSE`] in each string literal kept
/// where the item declares no rule, and left out where it declares one.
pub fn settle(tokens: TokenStream) -> TokenStream {
    let rule_less = is_rule_less();
    settled(tokens, rule_less)
}

fn settled(tokens: TokenStream, rule_less: bool) -> TokenStream {
    tokens
        .into_iter()
        .map(|token| match token {
            TokenTree::Group(group) => {
                let mut rebuilt = Group::new(group.delimiter(), settled(group.stream(), rule_less));
                rebuilt.set_span(group.span());
                TokenTree::Group(rebuilt)
            }
            TokenTree::Literal(literal) => settled_literal(literal, rule_less),
            kept @ (TokenTree::Ident(_) | TokenTree::Punct(_)) => kept,
        })
        .collect()
}

fn settled_literal(literal: Literal, rule_less: bool) -> TokenTree {
    let span = literal.span();
    let Ok(text) = syn::parse2::<syn::LitStr>(TokenTree::Literal(literal.clone()).into()) else {
        return TokenTree::Literal(literal);
    };
    let value = text.value();
    if !value.contains(SETTLE_OPEN) {
        return TokenTree::Literal(literal);
    }
    let mut written = String::new();
    let mut inside = false;
    for character in value.chars() {
        match character {
            SETTLE_OPEN => inside = true,
            SETTLE_CLOSE => inside = false,
            other if !inside || rule_less => written.push(other),
            _ => {}
        }
    }
    let mut rebuilt = Literal::string(&written);
    rebuilt.set_span(span);
    TokenTree::Literal(rebuilt)
}

/// Whether rustc takes `name` as an upper case name for a constant: it holds no lower case letter.
fn is_upper_case(name: &Ident) -> bool {
    !name.unraw().to_string().chars().any(char::is_lowercase)
}

/// The fields of `item`, read before the expansion strips the `literal` keys that fix some.
fn constructor_of(item: &syn::Item, new_by: NewBy) -> Constructor {
    let syn::Item::Struct(item_struct) = item else {
        return Constructor::None;
    };
    if new_by == NewBy::Author {
        return Constructor::None;
    }
    let slot = |field: &syn::Field| Slot {
        cfgs: field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("cfg"))
            .map(|attr| attr.to_token_stream().to_string())
            .collect(),
        member: field.ident.as_ref().map(ToString::to_string),
        ty: field.ty.to_token_stream().to_string(),
        fill: literal_fill(field),
    };
    let constructor = match &item_struct.fields {
        syn::Fields::Named(fields) => Constructor::Named(fields.named.iter().map(slot).collect()),
        syn::Fields::Unnamed(fields) => {
            let slots: Vec<Slot> = fields.unnamed.iter().map(slot).collect();
            if slots.iter().any(|held| !held.cfgs.is_empty()) {
                return Constructor::None;
            }
            Constructor::Tuple(slots)
        }
        syn::Fields::Unit => Constructor::Unit,
    };
    if new_by == NewBy::Default && constructor.parameters() > MOST_PARAMETERS {
        return Constructor::None;
    }
    constructor
}

/// What the `literal` of `field` fixes, where the field's type holds one value of it: the value
/// itself, or behind a pointer, a `Cow` or a `'static` reference. An `Option`, a list or a set
/// holds none or several, so its literal stays a parameter.
pub fn fixed_value(field: &syn::Field) -> Option<Fixed> {
    let literal = parse_model_schema_prop_attributes(&field.attrs).literal?;
    let shape = literal_shape(&field.ty)?;
    let fixed = match (&shape.leaf, &literal) {
        (LiteralLeaf::Bool, LiteralValue::Bool(value)) => Some(Fixed::Bool(*value)),
        (LiteralLeaf::Number(number), LiteralValue::Integer(_) | LiteralValue::Number(_)) => {
            Some(Fixed::Number(held_literal(number, &literal)?, number))
        }
        (LiteralLeaf::Path | LiteralLeaf::Str, LiteralValue::Str(text)) => {
            Some(Fixed::Text(text.clone()))
        }
        _ => None,
    }?;
    held_fill(&field.ty, &fixed, false)?;
    Some(fixed)
}

/// The value of a field a `literal` fixes, as Rust writes it.
fn literal_fill(field: &syn::Field) -> Option<String> {
    held_fill(&field.ty, &fixed_value(field)?, false)
}

/// The value of type `ty` that holds `fixed`, or `None` where no one value of it does. A `bare`
/// value is the argument of a conversion, so it names its own type.
fn held_fill(ty: &syn::Type, fixed: &Fixed, bare: bool) -> Option<String> {
    if let syn::Type::Reference(reference) = written_type(ty) {
        if reference.mutability.is_some()
            || reference
                .lifetime
                .as_ref()
                .is_none_or(|held| held.ident != "static")
        {
            return None;
        }
        return match (fixed, plain_ident(&reference.elem)?.as_str()) {
            (Fixed::Text(text), "str") => Some(format!("{text:?}")),
            (Fixed::Bool(_), "bool") => Some(format!("&{}", scalar(fixed)?)),
            (Fixed::Number(_, rust), name) if name == *rust => Some(format!("&{}", scalar(fixed)?)),
            (Fixed::Bool(_) | Fixed::Number(..) | Fixed::Text(_), _) => None,
        };
    }
    if let syn::Type::Path(path) = written_type(ty) {
        let segment = path.path.segments.last()?;
        return match &segment.arguments {
            syn::PathArguments::None => match fixed {
                Fixed::Text(text) if !bare => {
                    Some(format!("::core::convert::Into::into({text:?})"))
                }
                Fixed::Text(text) => match segment.ident.to_string().as_str() {
                    "String" => Some(format!("::std::string::String::from({text:?})")),
                    "PathBuf" => Some(format!("::std::path::PathBuf::from({text:?})")),
                    _ => None,
                },
                Fixed::Bool(_) | Fixed::Number(..) => scalar(fixed),
            },
            syn::PathArguments::AngleBracketed(args) if !bare => {
                let inner = written_type(sole_type_argument(args)?);
                let name = segment.ident.to_string();
                match (name.as_str(), fixed, plain_ident(inner).as_deref()) {
                    ("Box" | "Rc" | "Arc", Fixed::Text(text), Some("str")) => {
                        Some(format!("::core::convert::Into::into({text:?})"))
                    }
                    ("Box" | "Rc" | "Arc", Fixed::Text(text), Some("Path")) => Some(format!(
                        "::core::convert::Into::into(::std::path::PathBuf::from({text:?}))"
                    )),
                    ("Cow", Fixed::Text(text), Some("str")) => {
                        Some(format!("::std::borrow::Cow::Borrowed({text:?})"))
                    }
                    (_, _, Some("Path")) => None,
                    ("Box" | "Rc" | "Arc", _, _) => Some(format!(
                        "::core::convert::Into::into({})",
                        held_fill(inner, fixed, true)?
                    )),
                    ("Cow", _, _) => Some(format!(
                        "::std::borrow::Cow::Owned({})",
                        held_fill(inner, fixed, true)?
                    )),
                    _ => None,
                }
            }
            syn::PathArguments::AngleBracketed(_) | syn::PathArguments::Parenthesized(_) => None,
        };
    }
    None
}

#[cfg(test)]
mod tests;

/// A boolean or a number as Rust writes it, which holds its own type.
fn scalar(fixed: &Fixed) -> Option<String> {
    match fixed {
        Fixed::Bool(value) => Some(value.to_string()),
        Fixed::Number(digits, rust) => Some(format!("{digits}{rust}")),
        Fixed::Text(_) => None,
    }
}

/// The name of `ty` where it is a path of one segment with no arguments.
fn plain_ident(ty: &syn::Type) -> Option<String> {
    let syn::Type::Path(path) = written_type(ty) else {
        return None;
    };
    let segment = path.path.segments.last()?;
    matches!(segment.arguments, syn::PathArguments::None).then(|| segment.ident.to_string())
}

/// The tokens `text` spells, for a name or a type held as text.
fn text_tokens(text: &str) -> TokenStream {
    text.parse().unwrap_or_default()
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

/// Whether the declaration being expanded is declared as written, with no draft: a type a service
/// writes for itself, or an enum with no check that serde writes as a variant name.
pub fn is_undrafted() -> bool {
    DECLARED.with(|held| matches!(held.borrow().origin, Origin::Service | Origin::Plain))
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
pub fn record(item: &syn::Item, undrafted: bool, checked: bool, new_by: NewBy) {
    let origin = if undrafted {
        Origin::Service
    } else if matches!(item, syn::Item::Struct(item_struct) if super::is_branded_newtype(item_struct))
    {
        Origin::Brand
    } else if matches!(item, syn::Item::Enum(item_enum) if super::writes_a_string_union(item_enum))
        && !checked
    {
        Origin::Plain
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
        constructor: constructor_of(item, new_by),
        debugs: gate_of(attrs, "Debug"),
        derives: wrapper_derives(attrs),
        deserializes: gate_of(attrs, "Deserialize"),
        #[cfg(feature = "zod")]
        holds_bytes_whole: holds_bytes_whole(item),
        name,
        origin,
        serializes: gate_of(attrs, "Serialize"),
        checked,
        #[cfg(feature = "typescript")]
        unit: matches!(item, syn::Item::Struct(item_struct) if item_struct.generics.params.is_empty() && matches!(item_struct.fields, syn::Fields::Unit)),
    };
    if let Some(declared_name) = &declared.name {
        record_undrafted_enum(&declared_name.to_string(), declared.origin == Origin::Plain);
    }
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
