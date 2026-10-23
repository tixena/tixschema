//! The members of a struct, and of the variants of an enum, that serde reads as their default where
//! the payload leaves the key out. The value of a default is known to Rust alone, so it is built
//! where the type is declared, and each surface writes it into its text as JSON when the text is
//! built: a placeholder at expansion, replaced by the run of the function that answers the text.
//! A member of a variant whose default no target but Rust can read is refused.

#[cfg(feature = "serde")]
use crate::features::model_schema_prop::parse_model_schema_prop_attributes;
use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemEnum;
#[cfg(feature = "serde")]
use syn::ext::IdentExt as _;
#[cfg(feature = "serde")]
use {
    super::{
        CONVERSION_KEYS, consume_unread_value, derives_deserialize, derives_serialize,
        parse_serde_field_attributes, parse_serde_field_hooks, parse_serde_key_omission,
    },
    crate::field_type::{get_field_def, is_map_wrapper, is_sequence_wrapper},
    proc_macro2::TokenTree,
    quote::ToTokens as _,
    syn::{Attribute, ExprPath, Field, Generics, LitStr, Path, Variant},
};

/// Why no target but Rust reads the default of a flattened member.
#[cfg(feature = "serde")]
const FLATTENED: &str = "`#[serde(default)]` on a flattened field is read only by Rust. Remove \
                         the default, or the flatten.";

/// Why no target but Rust reads the default of a member of an enum whose defaults have no JSON.
#[cfg(feature = "serde")]
const UNWRITTEN: &str = "`#[serde(default)]` needs the enum to derive `Serialize`, with no `into` \
                         or `remote`, so the default can be written for the other targets.";

/// What serde's read of an enum comes to for the members of its variants.
#[cfg(feature = "serde")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum EnumRead {
    /// serde never reads the members: the enum derives no `Deserialize`, or reads itself through
    /// `from` or `try_from`.
    NoMember,
    /// serde reads the members, and no JSON of a default can be written: the enum derives no
    /// `Serialize`, or writes itself through `into` or `remote`.
    Unwritten,
    /// serde reads and writes the members.
    Written,
}

/// What a `default` attribute names: the type's own `Default`, or a function.
#[cfg(feature = "serde")]
enum Written {
    Bare,
    Function(ExprPath),
}

/// The members of one struct, or of the variants of one enum, serde reads as their default where
/// the payload leaves the key out, each with the JSON of its default. A member whose default has
/// no JSON is not among them: one of a type that does not derive both `Serialize` and
/// `Deserialize`, one whose key serde leaves out of what it writes, a flattened one, an `Option`,
/// one whose type names a parameter of the type, a lifetime or `Self`, and one under the
/// container's default where the struct has a parameter. A list, a set or a map under the bare
/// `default` is among them whatever its type names: its default is the empty collection at every
/// filling, `[]` or `{}`.
#[cfg(any(
    feature = "zod",
    feature = "jsonschema",
    feature = "dart",
    feature = "swift",
    feature = "kotlin"
))]
pub struct SerdeDefaults {
    container: Option<TokenStream>,
    /// Each member read as its default: the variant it is a member of, `None` for a struct's, the
    /// name serde reads, and the expression that builds the JSON of the default. One index runs
    /// across every variant, so no two members share a placeholder.
    members: Vec<(Option<String>, String, TokenStream)>,
}

#[cfg(any(
    feature = "zod",
    feature = "jsonschema",
    feature = "dart",
    feature = "swift",
    feature = "kotlin"
))]
impl SerdeDefaults {
    /// The placeholder a client writes for the default of `field` of `variant`, `None` for a
    /// struct's.
    #[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
    pub fn at(&self, variant: Option<&str>, field: &str) -> Option<String> {
        self.index(variant, field).map(placeholder)
    }

    /// What the module a text is answered from declares to be handed the defaults by: nothing
    /// where the type has none.
    pub fn holder(&self) -> TokenStream {
        if self.members.is_empty() {
            return TokenStream::new();
        }
        quote! {
            #[non_exhaustive]
            pub struct SerdeDefaults;
        }
    }

    /// Where the default of the member serde reads as `field`, of `variant` or of a struct where
    /// that is `None`, sits among the type's, or `None` for a member serde reads no default for.
    pub fn index(&self, variant: Option<&str>, field: &str) -> Option<usize> {
        self.members
            .iter()
            .position(|(of, name, _)| of.as_deref() == variant && name == field)
    }

    /// The defaults of a type that has none.
    pub const fn none() -> Self {
        Self {
            container: None,
            members: Vec::new(),
        }
    }

    /// Without `serde` no attribute is read, and no member is read as its default.
    #[cfg(not(feature = "serde"))]
    pub const fn of(_item: &syn::ItemStruct) -> Self {
        Self::none()
    }

    /// The defaults serde reads for the members of `item`, read off its attributes as written.
    #[cfg(feature = "serde")]
    pub fn of(item: &syn::ItemStruct) -> Self {
        let syn::Fields::Named(named) = &item.fields else {
            return Self::none();
        };
        if !derives_serialize(&item.attrs)
            || !derives_deserialize(&item.attrs)
            || !conversions(&item.attrs).is_empty()
            || super::has_serde_transparent(&item.attrs)
        {
            return Self::none();
        }
        let parameters = declared(&item.generics);
        let ident = &item.ident;
        let whole = written_default(&item.attrs)
            .filter(|_| parameters.is_empty())
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
                || get_field_def(&name, &field.ty, "").is_optional()
            {
                continue;
            }
            let named_parameter = first_named(field.ty.to_token_stream(), &parameters).is_some();
            let ty = &field.ty;
            let held = match written_default(&field.attrs) {
                Some(Written::Bare) if named_parameter => {
                    if let Some(empty) = empty_collection(ty, &field.attrs) {
                        members.push((None, name, empty));
                    }
                    continue;
                }
                _ if named_parameter => continue,
                Some(written) => held_default(ty, written),
                None if whole.is_some() => {
                    reads_the_whole = true;
                    quote! { &serde_default_whole.#member }
                }
                None => continue,
            };
            members.push((None, name, serialized(&field.attrs, &held)));
        }
        let container = whole
            .filter(|_| reads_the_whole)
            .map(|built| quote! { let serde_default_whole: #ident = #built; });
        Self { container, members }
    }

    /// Without `serde` no attribute is read, and no member is read as its default.
    #[cfg(not(feature = "serde"))]
    pub const fn of_enum(_item: &ItemEnum) -> Self {
        Self::none()
    }

    /// The defaults serde reads for the members of the variants of `item`, read off its
    /// attributes as written: each member [`default_refusals`] does not refuse whose own `default`
    /// has JSON. A variant takes no `default` of its own.
    #[cfg(feature = "serde")]
    pub fn of_enum(item: &ItemEnum) -> Self {
        let mut members = Vec::new();
        if EnumRead::of(&item.attrs) == EnumRead::Written {
            for (variant, field) in read_members(item) {
                if let Some(Ok(value)) = reach(field, &item.generics) {
                    members.push((Some(variant.ident.to_string()), field_name(field), value));
                }
            }
        }
        Self {
            container: None,
            members,
        }
    }

    /// The function the holder `module` declares answers the defaults by, written beside the
    /// type, where every name its author wrote resolves: each placeholder and the JSON of its
    /// default, in the order of the members.
    pub fn values_impl(&self, module: &proc_macro2::Ident) -> TokenStream {
        if self.members.is_empty() {
            return TokenStream::new();
        }
        let count = proc_macro2::Literal::usize_unsuffixed(self.members.len());
        let container = &self.container;
        let entries = self
            .members
            .iter()
            .enumerate()
            .map(|(index, (_, _, value))| {
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

#[cfg(feature = "serde")]
impl EnumRead {
    /// What serde's derive reads and writes of an enum with the container attributes `attrs`.
    fn of(attrs: &[Attribute]) -> Self {
        let written = conversions(attrs);
        if !derives_deserialize(attrs)
            || written.iter().any(|key| ["from", "try_from"].contains(key))
        {
            Self::NoMember
        } else if !derives_serialize(attrs)
            || written.iter().any(|key| ["into", "remote"].contains(key))
        {
            Self::Unwritten
        } else {
            Self::Written
        }
    }
}

/// Every conversion of its author's a container reads or writes through, in the order written:
/// `remote`, `from`, `try_from` and `into`.
#[cfg(feature = "serde")]
fn conversions(attrs: &[Attribute]) -> Vec<&'static str> {
    let mut found = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|nested| {
            found.extend(
                CONVERSION_KEYS
                    .into_iter()
                    .filter(|key| nested.path.is_ident(key)),
            );
            consume_unread_value(&nested)
        })
        .unwrap_or_else(|e| {
            log::trace!("Failed to parse serde conversion attribute: {e}");
        });
    }
    found
}

/// The names of the parameters `generics` declares: each type, lifetime and const.
#[cfg(feature = "serde")]
fn declared(generics: &Generics) -> Vec<String> {
    generics
        .params
        .iter()
        .map(|parameter| match parameter {
            syn::GenericParam::Type(held) => held.ident.to_string(),
            syn::GenericParam::Lifetime(held) => held.lifetime.ident.to_string(),
            syn::GenericParam::Const(held) => held.ident.to_string(),
        })
        .collect()
}

/// The `compile_error!` tokens of every member of a variant of `item` under a `default` of its own
/// that no target but Rust reads as serde does, each spanned on the member. A member serde never
/// reads the key of, an `Option` and a member of an enum serde never reads are refused nothing.
#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
pub fn default_refusals(item: &ItemEnum) -> Vec<TokenStream> {
    let read = EnumRead::of(&item.attrs);
    if read == EnumRead::NoMember {
        return Vec::new();
    }
    read_members(item)
        .filter_map(|(variant, field)| {
            let reason = match reach(field, &item.generics)? {
                _ if read == EnumRead::Unwritten => UNWRITTEN.to_owned(),
                Err(reason) => reason,
                Ok(_) => return None,
            };
            let named = field.ident.as_ref()?;
            let message = format!(
                "model_schema: type `{}`: variant `{}`, field `{}`: {reason}",
                item.ident,
                variant.ident,
                named.unraw()
            );
            Some(syn::Error::new(named.span(), message).to_compile_error())
        })
        .collect()
}

/// The JSON of the empty collection a field of type `ty` defaults to, for a list, a set or a map
/// written as such: the same at every filling of the type's parameters. `None` for any other type,
/// and for a field serde writes through a function of its own, whose JSON of the empty value only
/// that function knows.
#[cfg(feature = "serde")]
fn empty_collection(ty: &syn::Type, attrs: &[Attribute]) -> Option<TokenStream> {
    let hooks = parse_serde_field_hooks(attrs);
    if hooks.serialize_with.is_some() || hooks.with.is_some() {
        return None;
    }
    let syn::Type::Path(path) = ty else {
        return None;
    };
    let last = path.path.segments.last()?.ident.to_string();
    if is_sequence_wrapper(&last) {
        Some(quote! { ::serde_json::Value::Array(::std::vec::Vec::new()) })
    } else if is_map_wrapper(&last) {
        Some(quote! { ::serde_json::Value::Object(::serde_json::Map::new()) })
    } else {
        None
    }
}

/// The name serde reads `field` under, before any renaming.
#[cfg(feature = "serde")]
fn field_name(field: &Field) -> String {
    field
        .ident
        .as_ref()
        .map_or_default(|ident| ident.unraw().to_string())
}

/// The first of `Self`, a lifetime other than `'static` and the parameters `declared` that `tokens`
/// name, as written.
#[cfg(feature = "serde")]
fn first_named(tokens: TokenStream, declared: &[String]) -> Option<String> {
    let mut after_a_quote = false;
    tokens.into_iter().find_map(|token| {
        let quoted = after_a_quote;
        after_a_quote = false;
        match token {
            TokenTree::Group(group) => first_named(group.stream(), declared),
            TokenTree::Ident(ident) => {
                let name = ident.to_string();
                if quoted {
                    (name != "static").then(|| format!("'{name}"))
                } else {
                    (name == "Self" || declared.contains(&name)).then_some(name)
                }
            }
            TokenTree::Punct(punct) => {
                after_a_quote = punct.as_char() == '\'';
                None
            }
            TokenTree::Literal(_) => None,
        }
    })
}

/// The expression that borrows the default `written` builds for a member of type `ty`.
#[cfg(feature = "serde")]
fn held_default(ty: &syn::Type, written: Written) -> TokenStream {
    match written {
        Written::Bare => quote! { &<#ty as ::core::default::Default>::default() },
        Written::Function(path) => {
            quote! { &{ let serde_default_value: #ty = #path(); serde_default_value } }
        }
    }
}

/// Whether `text` holds the placeholder of a default.
#[cfg(feature = "zod")]
pub fn holds_a_placeholder(text: &str) -> bool {
    text.contains("\u{1}serde-default-")
}

/// The key that leaves a member out of what serde writes, `skip_serializing_if` or
/// `skip_serializing`, the first one written.
#[cfg(feature = "serde")]
fn key_left_out_by(attrs: &[Attribute]) -> &'static str {
    let mut found = None;
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|nested| {
            for key in ["skip_serializing_if", "skip_serializing"] {
                if found.is_none() && nested.path.is_ident(key) {
                    found = Some(key);
                }
            }
            consume_unread_value(&nested)
        })
        .unwrap_or_else(|e| {
            log::trace!("Failed to parse serde key-omission attribute: {e}");
        });
    }
    found.unwrap_or("skip_serializing_if")
}

/// Why no target but Rust reads the default of a member whose type names `named`, of a type that
/// declares `generics`.
#[cfg(feature = "serde")]
fn named_reason(named: &str, generics: &Generics) -> String {
    let said = if named == "Self" {
        "the type names `Self`, and the JSON of the default is built outside the enum, where \
         `Self` names nothing of it. Write the enum's name in place of `Self`"
            .to_owned()
    } else if named.starts_with('\'') {
        format!(
            "the type names the lifetime `{named}`, and the JSON of the default is built outside \
             the enum, where no lifetime of it is in scope. Give the field a type that names no \
             lifetime"
        )
    } else if generics.const_params().any(|held| held.ident == named) {
        format!(
            "the type names the const parameter `{named}`, whose default has one JSON for each \
             value that fills it. Give the field a type that names no parameter"
        )
    } else {
        format!(
            "the type names the type parameter `{named}`, whose default has one JSON for each \
             type that fills it. Give the field a type that names no parameter"
        )
    };
    format!("`#[serde(default)]` is read only by Rust here: {said}, or remove the default.")
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

/// What the targets make of the `default` `field` writes on itself, the enum declaring
/// `generics`: the JSON they read the key the payload leaves out as, or why Rust alone reads it.
/// `None` where it writes none, or the targets read the key as serde does with no default in hand:
/// serde never reads it, or the member is an `Option`, flattened or not, whose absence every
/// target reads as `None`.
#[cfg(feature = "serde")]
fn reach(field: &Field, generics: &Generics) -> Option<Result<TokenStream, String>> {
    let written = written_default(&field.attrs)?;
    let omission = parse_serde_key_omission(&field.attrs);
    if omission.skips_deserializing {
        return None;
    }
    if get_field_def(&field_name(field), &field.ty, "").stands_for_an_option() {
        return None;
    }
    if parse_serde_field_attributes(&field.attrs).flatten {
        return Some(Err(FLATTENED.to_owned()));
    }
    if omission.omits_key {
        return Some(Err(format!(
            "`#[serde(default)]` beside `{}` is read only by Rust on a field that is not an \
             `Option`. Make the field an `Option`, or remove one of the two.",
            key_left_out_by(&field.attrs)
        )));
    }
    let ty = &field.ty;
    let Some(named) = first_named(ty.to_token_stream(), &declared(generics)) else {
        return Some(Ok(serialized(&field.attrs, &held_default(ty, written))));
    };
    if matches!(written, Written::Bare)
        && let Some(empty) = empty_collection(ty, &field.attrs)
    {
        return Some(Ok(empty));
    }
    Some(Err(named_reason(&named, generics)))
}

/// Each named member of a variant of `item` that serde reads member by member, with its variant:
/// a variant serde never reads, or reads through a function of its author's, is left out.
#[cfg(feature = "serde")]
fn read_members(item: &ItemEnum) -> impl Iterator<Item = (&Variant, &Field)> {
    item.variants
        .iter()
        .filter(|variant| {
            let hooks = parse_serde_field_hooks(&variant.attrs);
            !parse_serde_key_omission(&variant.attrs).skips_deserializing
                && hooks.deserialize_with.is_none()
                && hooks.with.is_none()
        })
        .flat_map(|variant| {
            variant
                .fields
                .iter()
                .filter(|field| field.ident.is_some())
                .map(move |field| (variant, field))
        })
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
    } else if parse_model_schema_prop_attributes(attrs).as_number {
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
        let written_char = proc_macro2::Literal::character(read);
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

#[cfg(test)]
#[cfg(all(
    feature = "serde",
    any(
        feature = "zod",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
mod tests;
