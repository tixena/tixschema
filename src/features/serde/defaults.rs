//! The members of a struct serde reads as their default where the payload leaves the key out.
//! The value of a default is known to Rust alone, so it is built where the struct is declared,
//! and each surface writes it into its text as JSON when the text is built: a placeholder at
//! expansion, replaced by the run of the function that answers the text.

use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::ItemStruct;
#[cfg(feature = "serde")]
use {
    super::{
        consume_unread_value, derives_deserialize, derives_serialize, has_serde_transparent,
        parse_serde_field_attributes, parse_serde_field_hooks, parse_serde_key_omission,
        serde_conversion,
    },
    crate::features::model_schema_prop::parse_model_schema_prop_attributes,
    crate::field_type::get_field_def,
    proc_macro2::TokenTree,
    quote::ToTokens as _,
    syn::ext::IdentExt as _,
    syn::{Attribute, ExprPath, Fields, LitStr, Path},
};

/// What a `default` attribute names: the type's own `Default`, or a function.
#[cfg(feature = "serde")]
enum Written {
    Bare,
    Function(ExprPath),
}

/// The members of one struct serde reads as their default where the payload leaves the key out,
/// each with the JSON of its default. A member whose default has no JSON is not among them: one
/// of a type that does not derive both `Serialize` and `Deserialize`, one whose key serde leaves
/// out of what it writes, a flattened one, an `Option`, one whose type names a parameter of the
/// struct, a lifetime or `Self`, and one under the container's default where the struct has a
/// parameter.
pub struct SerdeDefaults {
    container: Option<TokenStream>,
    /// Each member read as its default, by the name serde reads, and the expression that builds
    /// the JSON of the default.
    members: Vec<(String, TokenStream)>,
}

impl SerdeDefaults {
    /// The placeholder a client writes for the default of `field`.
    #[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
    pub fn at(&self, field: &str) -> Option<String> {
        self.index(field).map(placeholder)
    }

    /// What the module a text is answered from declares to be handed the defaults by: nothing
    /// where the struct has none.
    pub fn holder(&self) -> TokenStream {
        if self.members.is_empty() {
            return TokenStream::new();
        }
        quote! {
            #[non_exhaustive]
            pub struct SerdeDefaults;
        }
    }

    /// Where the default of the member serde reads as `field` sits among the struct's, or `None`
    /// for a member serde reads no default for.
    pub fn index(&self, field: &str) -> Option<usize> {
        self.members.iter().position(|(member, _)| member == field)
    }

    /// The defaults of a struct that has none.
    pub const fn none() -> Self {
        Self {
            container: None,
            members: Vec::new(),
        }
    }

    /// Without `serde` no attribute is read, and no member is read as its default.
    #[cfg(not(feature = "serde"))]
    pub const fn of(_item: &ItemStruct) -> Self {
        Self::none()
    }

    /// The defaults serde reads for the members of `item`, read off its attributes as written.
    #[cfg(feature = "serde")]
    pub fn of(item: &ItemStruct) -> Self {
        let Fields::Named(named) = &item.fields else {
            return Self::none();
        };
        if !derives_serialize(&item.attrs)
            || !derives_deserialize(&item.attrs)
            || serde_conversion(&item.attrs).is_some()
            || has_serde_transparent(&item.attrs)
        {
            return Self::none();
        }
        let declared: Vec<String> = item
            .generics
            .params
            .iter()
            .map(|parameter| match parameter {
                syn::GenericParam::Type(held) => held.ident.to_string(),
                syn::GenericParam::Lifetime(held) => held.lifetime.ident.to_string(),
                syn::GenericParam::Const(held) => held.ident.to_string(),
            })
            .collect();
        let ident = &item.ident;
        let whole = written_default(&item.attrs)
            .filter(|_| declared.is_empty())
            .map(|written| match written {
                Written::Bare => quote! { <#ident as ::core::default::Default>::default() },
                Written::Function(path) => quote! { #path() },
            });
        let mut members = Vec::new();
        let mut reads_the_whole = false;
        for field in &named.named {
            let Some(member) = field.ident.as_ref() else {
                continue;
            };
            let omission = parse_serde_key_omission(&field.attrs);
            let name = member.unraw().to_string();
            if omission.omits_key
                || omission.skips_deserializing
                || parse_serde_field_attributes(&field.attrs).flatten
                || names_any(field.ty.to_token_stream(), &declared)
                || get_field_def(&name, &field.ty, "").is_optional()
            {
                continue;
            }
            let ty = &field.ty;
            let held = match written_default(&field.attrs) {
                Some(Written::Bare) => quote! { &<#ty as ::core::default::Default>::default() },
                Some(Written::Function(path)) => {
                    quote! { &{ let serde_default_value: #ty = #path(); serde_default_value } }
                }
                None if whole.is_some() => {
                    reads_the_whole = true;
                    quote! { &serde_default_whole.#member }
                }
                None => continue,
            };
            members.push((name, serialized(&field.attrs, &held)));
        }
        let container = whole
            .filter(|_| reads_the_whole)
            .map(|built| quote! { let serde_default_whole: #ident = #built; });
        Self { container, members }
    }

    /// The function the holder `module` declares answers the defaults by, written beside the
    /// struct, where every name its author wrote resolves: each placeholder and the JSON of its
    /// default, in the order of the members.
    #[cfg(any(
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    ))]
    pub fn values_impl(&self, module: &proc_macro2::Ident) -> TokenStream {
        if self.members.is_empty() {
            return TokenStream::new();
        }
        let count = Literal::usize_unsuffixed(self.members.len());
        let container = &self.container;
        let entries = self.members.iter().enumerate().map(|(index, (_, value))| {
            let at = placeholder(index);
            quote! { (#at, #value) }
        });
        quote! {
            impl #module::SerdeDefaults {
                pub fn literals() -> [(&'static str, ::serde_json::Value); #count] {
                    #container
                    [#(#entries),*]
                }
            }
        }
    }
}

/// Whether `text` holds the placeholder of a default.
#[cfg(feature = "zod")]
pub fn holds_a_placeholder(text: &str) -> bool {
    text.contains("\u{1}serde-default-")
}

/// Whether `tokens` name `Self`, a lifetime other than `'static`, or one of `declared`.
#[cfg(feature = "serde")]
fn names_any(tokens: TokenStream, declared: &[String]) -> bool {
    let mut after_a_quote = false;
    tokens.into_iter().any(|token| {
        let quoted = after_a_quote;
        after_a_quote = false;
        match token {
            TokenTree::Group(group) => names_any(group.stream(), declared),
            TokenTree::Ident(ident) => {
                let name = ident.to_string();
                (quoted && name != "static")
                    || (!quoted && (name == "Self" || declared.contains(&name)))
            }
            TokenTree::Punct(punct) => {
                after_a_quote = punct.as_char() == '\'';
                false
            }
            TokenTree::Literal(_) => false,
        }
    })
}

/// The text a surface writes where the default of the member `index` goes, replaced by its JSON
/// when the text is built. Two control characters keep it from meeting anything an author writes.
#[cfg(any(
    feature = "zod",
    feature = "jsonschema",
    feature = "dart",
    feature = "swift",
    feature = "kotlin"
))]
pub fn placeholder(index: usize) -> String {
    format!("\u{1}serde-default-{index}\u{1}")
}

/// The JSON of the value `held` borrows, written through the function serde writes the member
/// with: `serialize_with`, the module `with` names, the one `as_number` hangs on a date, or the
/// value's own `Serialize`.
#[cfg(feature = "serde")]
fn serialized(attrs: &[Attribute], held: &TokenStream) -> TokenStream {
    let hooks = parse_serde_field_hooks(attrs);
    let through: Option<Path> = if let Some(written) = hooks.serialize_with {
        written.parse().ok()
    } else if let Some(written) = hooks.with {
        written
            .parse::<Path>()
            .ok()
            .map(|module| syn::parse_quote! { #module::serialize })
    } else if hooks.deserialize_with.is_none()
        && parse_model_schema_prop_attributes(attrs).as_number
    {
        Some(syn::parse_quote! { chrono::serde::ts_milliseconds::serialize })
    } else {
        None
    };
    through.map_or_else(
        || quote! { ::serde_json::to_value(#held).unwrap() },
        |function| quote! { #function(#held, ::serde_json::value::Serializer).unwrap() },
    )
}

/// The statements that write into `written`, a `String` the text is held in, the JSON of each
/// default the holder `holder` answers in place of its placeholder, each character of `escaped`
/// replaced by its escape.
#[cfg(any(
    feature = "zod",
    feature = "dart",
    feature = "swift",
    feature = "kotlin"
))]
pub fn substituted(
    holder: &TokenStream,
    written: &proc_macro2::Ident,
    escaped: &[(char, &str)],
) -> TokenStream {
    let replaced = escaped.iter().map(|&(read, escape)| {
        let written_char = Literal::character(read);
        quote! { .replace(#written_char, #escape) }
    });
    quote! {
        for (serde_default_at, serde_default_value) in #holder::literals() {
            #written = #written.replace(
                serde_default_at,
                &serde_default_value.to_string() #(#replaced)*,
            );
        }
    }
}

/// The `default` written in `attrs`, the last one read, as serde's derive reads it.
#[cfg(feature = "serde")]
fn written_default(attrs: &[Attribute]) -> Option<Written> {
    let mut found = None;
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|nested| {
            if nested.path.is_ident("default") {
                found = Some(if nested.input.peek(syn::Token![=]) {
                    Written::Function(nested.value()?.parse::<LitStr>()?.parse()?)
                } else {
                    Written::Bare
                });
                return Ok(());
            }
            consume_unread_value(&nested)
        })
        .unwrap_or_else(|e| {
            log::trace!("Failed to parse serde default attribute: {e}");
        });
    }
    found
}
