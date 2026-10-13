//! The read of a type that publishes `validate()`: its own unjudged read, followed by its
//! `validate()`.
//!
//! serde's derive writes the unjudged read on a copy of the item, declared inside
//! `model_schema_read_unjudged` under `#[serde(remote = "...")]`. The functions that derive writes
//! are then functions of the copy, and the type has none named `deserialize`: one beside the
//! `Deserialize` the macro writes would answer `Type::deserialize(..)` with the unjudged read. A
//! holder reads the models it holds through their unjudged read, so the whole value is judged
//! once, by the type the caller asked for.
//!
//! A type that names a `preprocess`, on itself or on a member, has the same read written for it:
//! each function is called on the payload inside the unjudged read, ahead of what it corrects.

#[cfg(feature = "serde")]
use core::cell::RefCell;
use core::mem;
use core::slice;

use proc_macro2::{Group, Ident, Span, TokenStream, TokenTree};
use quote::{ToTokens as _, quote};
use syn::ext::IdentExt as _;
use syn::punctuated::Punctuated;
use syn::{Attribute, DeriveInput, Field, Fields, ItemEnum, ItemStruct, LitStr, Token};

use super::preprocess::{self, Named};
#[cfg(feature = "serde")]
use super::{
    ConstraintLeaf, ConstraintWrap, HeldEnd, constrained_shape, helper_name_stem,
    joined_violations_fn, list_hook_header, map_value_argument,
    names_what_the_module_cannot_repeat, needs_injected_default, nested_under_fn, reads_through,
    reports_a_bound_fn,
};
use super::{prefixed_guard_message, refusal_from_violations, sole_type_argument};
use crate::features::model_schema_prop::FunctionName;
#[cfg(feature = "serde")]
use crate::features::serde::NAMED_READ_HOOK_PREFIX;
#[cfg(feature = "serde")]
use crate::features::serde::has_serde_read_hook;
#[cfg(all(
    feature = "serde",
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema"))
))]
use crate::features::serde::has_serde_transparent;
#[cfg(feature = "serde")]
use crate::features::serde::parse_serde_key_omission;
use crate::features::serde::read_bounds::{ReadBounds, read_bounds, written_lifetimes};
use crate::features::serde::{
    derives_deserialize, derives_serialize, has_serde_default, read_hook, serde_conversion,
};
#[cfg(feature = "serde")]
use crate::field_type::is_map_wrapper;
use crate::field_type::is_ownership_wrapper;
use crate::utils::{to_snake_case, written_names, written_type};

/// The read the recovering decode of a type lists from, which every type under the flag has.
#[cfg(feature = "serde")]
const LISTED_READ: &str = "decode_with_read_unjudged";

/// The name of the trait that answers the unjudged read of a type that has none of its own.
#[cfg(feature = "serde")]
const FALLBACK: &str = "ReadUnjudged";

/// The name of the type a hook reads a model as, to read it unjudged.
#[cfg(feature = "serde")]
const HELD: &str = "Unjudged";

/// What a brand's own bound answers of a value, the checks the brand names apart. A member of an
/// untagged enum that holds the brand is held to it as it is read, so the bound chooses the
/// variant, and so is the value a recovering read lists the brand from.
#[cfg(feature = "serde")]
pub const BOUND_ALONE: &str = "model_schema_validate_bound";

/// The name of the trait that answers [`BOUND_ALONE`] for a type that carries no such bound.
#[cfg(feature = "serde")]
const CHOSEN_BY_SHAPE: &str = "ChosenByShape";

/// What the name of the read of a corrected payload ends on, in place of [`UNJUDGED`].
const CORRECTED: &str = "_corrected";

/// The name of the type a member's hook reads, where it reads whatever type the member declares.
const READ: &str = "Read";

/// The name of the copy of an item serde's derive writes the unjudged read on.
const REMOTE: &str = "ModelSchemaRemote";

/// What the name of an unjudged read ends on.
const UNJUDGED: &str = "_unjudged";

/// The read of a type that publishes `validate()`, before its `validate()`.
pub const UNJUDGED_READ: &str = "model_schema_read_unjudged";

/// What [`judged`] answers for an item whose read the macro writes.
enum Judged {
    Refused(TokenStream),
    Written(TokenStream),
}

/// What the hooks recorded for the item being expanded add to its read: the functions, and the
/// attributes that hang each on its member of the copy.
#[derive(Default)]
struct Hooked {
    functions: TokenStream,
    hung: Vec<HungOn>,
}

/// The attributes that hang a hook on the member at `index` of `variant`, or of the struct.
struct HungOn {
    attrs: Vec<Attribute>,
    index: usize,
    /// Whether the hook is read in place of the one the member's own attributes name.
    replaces: bool,
    variant: Option<String>,
}

/// What a read calls of the functions `preprocess` names, and the names it declares to call
/// them: none is a name the item writes, or one of the functions.
struct Corrects {
    /// The name Rust calls each function under.
    called: Vec<String>,
    /// What holds the payload the functions answered.
    corrected: Ident,
    /// The parameter a function of the read takes its deserializer under.
    deserializer: Ident,
    named: Named,
    /// What holds the payload as it was read.
    payload: Ident,
}

/// The hook one member of the item being expanded reads the models it holds through. It is held
/// as text: the item's read is written after its members are walked, and a token outlives no
/// expansion.
#[cfg(feature = "serde")]
struct MemberHook {
    /// The member's value, built back from `held_0`, what `read_as` was read into.
    back: String,
    /// Whether the member's read chooses the variant of an untagged enum.
    choosing: bool,
    /// Whether a missing key is read as the member's default, as it is with no hook.
    defaulted: bool,
    /// The name of the type `read_as` writes where the member's type writes the model.
    held_as: String,
    /// The member's place among those of its struct, or of its variant.
    index: usize,
    /// The model the member holds.
    leaf: String,
    name: String,
    /// The member's type with `held_as` written where it writes the model.
    read_as: String,
    /// The key a refusal raised below is answered under, where it is answered under one.
    renamed_under: Option<String>,
    ty: String,
    variant: Option<String>,
}

/// Where a member sits in its item, as its hook is written and hung.
#[cfg(feature = "serde")]
pub(super) struct HookPlace<'item> {
    /// Whether the member's read chooses the variant of an untagged enum: a brand it holds is
    /// then held to its own bound as it is read.
    pub choosing: bool,
    /// Whether the container answers a missing key itself, with `#[serde(default)]`.
    pub container_defaulted: bool,
    pub index: usize,
    /// The key a refusal raised below the member is answered under, where it is answered under
    /// one.
    pub renamed_under: Option<&'item str>,
    pub type_parameters: &'item [String],
    pub variant: Option<&'item str>,
    /// Whether serde reads the member under a key of its own, so that a missing one has a meaning.
    pub written_under_a_key: bool,
}

/// The read of a member's type with every model it holds read unjudged.
#[cfg(feature = "serde")]
struct Reached {
    /// The member's value, built back from `held_0`, what `read_as` was read into.
    back: TokenStream,
    leaf: syn::Type,
    /// The member's type with the name it was asked for written where it writes `leaf`.
    read_as: syn::Type,
    /// The wrappers `leaf` is held under, outermost first.
    wraps: Vec<ConstraintWrap>,
}

#[cfg(feature = "serde")]
thread_local! {
    /// The hooks of the item being expanded, in the order its members were walked.
    static MEMBER_HOOKS: RefCell<Vec<MemberHook>> = const { RefCell::new(Vec::new()) };
}

impl Corrects {
    /// The name the read declares a hook under: `named`, numbered where a function is called
    /// under it.
    fn hook_name(&self, named: &str) -> Ident {
        unclaimed_parameter(named, &self.called)
    }

    /// What the item being expanded names, with names for what a read of it declares: none of
    /// `written`, the names the item writes.
    fn of(written: &[String]) -> Self {
        let named = preprocess::named();
        let called = named.rust_names();
        let mut taken = written.to_vec();
        taken.extend(called.iter().cloned());
        Self {
            corrected: unclaimed_parameter("corrected", &taken),
            deserializer: unclaimed_parameter("deserializer", &called),
            payload: unclaimed_parameter("payload", &taken),
            called,
            named,
        }
    }

    /// The statements that read the payload from the deserializer and bind what `functions`
    /// answer for it.
    fn statements(&self, functions: &[FunctionName]) -> TokenStream {
        let (deserializer, payload, corrected) =
            (&self.deserializer, &self.payload, &self.corrected);
        let answered = preprocess::corrected(functions, quote! { #payload });
        quote! {
            let #payload = <::serde_json::Value as serde::Deserialize>::deserialize(#deserializer)?;
            let #corrected = #answered;
        }
    }
}

#[cfg(feature = "serde")]
impl MemberHook {
    /// The function the member is read through: its type with each model it holds read through
    /// that model's `unjudged` read, from the payload the member's own functions corrected where
    /// it names any.
    fn function(&self, unjudged: &Ident, reader: &Ident, corrects: &Corrects) -> TokenStream {
        let name = corrects.hook_name(&self.name);
        let held_as = Ident::new(&self.held_as, Span::call_site());
        let [ty, leaf, read_as, back] = [&self.ty, &self.leaf, &self.read_as, &self.back]
            .map(|written| written.parse::<TokenStream>().unwrap_or_default());
        let held_leaf = self.held_leaf(&leaf, &held_as, unjudged);
        let renamed = self.renamed_under.as_deref().map(|key| {
            let renaming = named_refusal(key);
            quote! { .map_err(#renaming) }
        });
        let functions = corrects
            .named
            .of_member(self.variant.as_deref(), self.index);
        if functions.is_empty() {
            return quote! {
                fn #name<'de, #reader>(deserializer: #reader) -> ::core::result::Result<#ty, #reader::Error>
                where
                    #reader: serde::Deserializer<'de>,
                {
                    struct #held_as(#leaf);
                    impl<'de> serde::Deserialize<'de> for #held_as {
                        fn deserialize<#reader>(deserializer: #reader) -> ::core::result::Result<Self, #reader::Error>
                        where
                            #reader: serde::Deserializer<'de>,
                        {
                            #held_leaf
                        }
                    }
                    let held_0 = <#read_as as serde::Deserialize>::deserialize(deserializer) #renamed ?;
                    ::core::result::Result::Ok(#back)
                }
            };
        }
        let (deserializer, corrected) = (&corrects.deserializer, &corrects.corrected);
        let correction = corrects.statements(functions);
        let told = told_by(reader);
        quote! {
            fn #name<'de, #reader>(#deserializer: #reader) -> ::core::result::Result<#ty, #reader::Error>
            where
                #reader: serde::Deserializer<'de>,
            {
                struct #held_as(#leaf);
                impl<'de> serde::Deserialize<'de> for #held_as {
                    fn deserialize<#reader>(deserializer: #reader) -> ::core::result::Result<Self, #reader::Error>
                    where
                        #reader: serde::Deserializer<'de>,
                    {
                        #held_leaf
                    }
                }
                #correction
                let held_0 = <#read_as as serde::Deserialize>::deserialize(#corrected) #told #renamed ?;
                ::core::result::Result::Ok(#back)
            }
        }
    }

    /// What answers `leaf`, the model the member holds, as `held_as`: its `unjudged` read, held
    /// to the bound a brand carries where the member chooses a variant.
    fn held_leaf(&self, leaf: &TokenStream, held_as: &Ident, unjudged: &Ident) -> TokenStream {
        if !self.choosing {
            return quote! { <#leaf>::#unjudged(deserializer).map(#held_as) };
        }
        let bound_alone = Ident::new(BOUND_ALONE, Span::call_site());
        let refusal = refusal_from_violations();
        quote! {
            let held = <#leaf>::#unjudged(deserializer)?;
            let chosen = &held;
            chosen.#bound_alone().map_err(#refusal)?;
            ::core::result::Result::Ok(#held_as(held))
        }
    }
}

/// Forgets every hook recorded: an item's read holds only that item's own.
#[cfg(feature = "serde")]
pub fn forget_member_hooks() {
    MEMBER_HOOKS.with(|held| held.borrow_mut().clear());
}

/// What turns the refusal of a corrected payload, which is that payload's own, into the one the
/// deserializer of type `reader` answers.
fn told_by(reader: &Ident) -> TokenStream {
    quote! { .map_err(<#reader::Error as serde::de::Error>::custom) }
}

/// The name of the read of a payload the type's own functions have corrected, beside the unjudged
/// read `unjudged`.
fn corrected_read(unjudged: &Ident) -> Ident {
    let named = unjudged.to_string();
    let stem = named.strip_suffix(UNJUDGED).unwrap_or(&named);
    Ident::new(&format!("{stem}{CORRECTED}"), Span::call_site())
}

/// The binding a level of [`Reached::back`] holds its value under.
#[cfg(feature = "serde")]
fn held_binding(depth: usize) -> Ident {
    Ident::new(&format!("held_{depth}"), Span::call_site())
}

/// `ty` with its type argument at `position` written `argument`.
#[cfg(feature = "serde")]
fn with_argument(ty: &syn::TypePath, position: usize, argument: &syn::Type) -> syn::Type {
    let mut written = ty.clone();
    if let Some(last) = written.path.segments.last_mut()
        && let syn::PathArguments::AngleBracketed(held) = &mut last.arguments
        && let Some(replaced) = held
            .args
            .iter_mut()
            .filter_map(|listed| match listed {
                syn::GenericArgument::Type(of) => Some(of),
                syn::GenericArgument::Lifetime(_)
                | syn::GenericArgument::Const(_)
                | syn::GenericArgument::AssocType(_)
                | syn::GenericArgument::AssocConst(_)
                | syn::GenericArgument::Constraint(_)
                | _ => None,
            })
            .nth(position)
    {
        *replaced = argument.clone();
    }
    syn::Type::Path(written)
}

/// How a value of type `ty` is read with the model it holds unjudged, the model read as the type
/// `held_as`: a declared type itself, or one under `Option`, `Vec`, `Box` and the values of a map.
/// `None` for any other type.
#[cfg(feature = "serde")]
fn reached(ty: &syn::Type, depth: usize, held_as: &Ident) -> Option<Reached> {
    let written = written_type(ty);
    let syn::Type::Path(named) = written else {
        return None;
    };
    let last = named.path.segments.last()?;
    let held = held_binding(depth);
    let itself = || Reached {
        back: quote! { #held.0 },
        leaf: written.clone(),
        read_as: syn::parse_quote! { #held_as },
        wraps: Vec::new(),
    };
    let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return matches!(last.arguments, syn::PathArguments::None).then(itself);
    };
    let wrapper = last.ident.to_string();
    let (wrap, position, held_type) = if is_map_wrapper(&wrapper) {
        (ConstraintWrap::Map, 1, map_value_argument(arguments)?)
    } else {
        let wrap = match wrapper.as_str() {
            "Box" => ConstraintWrap::Transparent,
            "Option" => ConstraintWrap::Optional,
            "Vec" => ConstraintWrap::Sequence,
            // A declared type written with arguments is the model itself.
            _ => return (!reads_through(&wrapper)).then(itself),
        };
        (wrap, 0, sole_type_argument(arguments)?)
    };
    let below = reached(held_type, depth.saturating_add(1), held_as)?;
    let (inner, value) = (held_binding(depth.saturating_add(1)), &below.back);
    let back = match wrap {
        ConstraintWrap::Map => {
            let key = Ident::new(&format!("key_{depth}"), Span::call_site());
            quote! { #held.into_iter().map(|(#key, #inner)| (#key, #value)).collect::<#written>() }
        }
        ConstraintWrap::Optional => quote! { #held.map(|#inner| #value) },
        ConstraintWrap::Sequence => {
            quote! { #held.into_iter().map(|#inner| #value).collect::<#written>() }
        }
        ConstraintWrap::Borrowed | ConstraintWrap::Transparent => {
            quote! { <#written>::new({ let #inner = *#held; #value }) }
        }
    };
    let mut wraps = vec![wrap];
    wraps.extend(below.wraps);
    Some(Reached {
        back,
        leaf: below.leaf,
        read_as: with_argument(named, position, &below.read_as),
        wraps,
    })
}

/// What answers a refusal raised below the member with the key `named` written in front of each
/// of this crate's own bound sentences it carries.
#[cfg(feature = "serde")]
pub fn named_refusal(named: &str) -> TokenStream {
    let naming = nested_under_fn();
    let recogniser = reports_a_bound_fn();
    let splitter = joined_violations_fn();
    quote! {
        |refused| {
            #naming
            #recogniser
            #splitter
            let reported = refused.to_string();
            let violations = joined_violations(&reported);
            if !violations.iter().all(|violation| reports_a_bound(violation)) {
                return refused;
            }
            let named: ::std::vec::Vec<::std::string::String> = violations
                .iter()
                .map(|violation| nested_under(#named, violation))
                .collect();
            serde::de::Error::custom(named.join("; "))
        }
    }
}

/// The hook a member reads the models it holds unjudged through, recorded for the read of the
/// item being expanded: the member's type is read with each model under `Option`, `Vec`, `Box`
/// and a map's values read through its unjudged read. `shape` is the walk `validate()` makes of
/// the same member, which is what judges what the hook leaves unjudged. Answers whether the
/// member was given one.
#[cfg(feature = "serde")]
pub(super) fn hook_member(field: &Field, shape: &super::HeldShape, place: &HookPlace<'_>) -> bool {
    if parse_serde_key_omission(&field.attrs).skips_deserializing
        || has_serde_read_hook(&field.attrs)
        || names_what_the_module_cannot_repeat(&field.ty, place.type_parameters)
        || !matches!(shape.end, HeldEnd::Declared)
    {
        return false;
    }
    // The hook writes the member's type beside the type it reads the model as.
    let held_as = unclaimed_parameter(HELD, &written_names(field.ty.to_token_stream()));
    let Some(read) = reached(&field.ty, 0, &held_as) else {
        return false;
    };
    // The walk and the read are of one type, and reach the model through the same wrappers.
    if read.wraps.len() != shape.wraps.len()
        || read
            .wraps
            .iter()
            .zip(&shape.wraps)
            .any(|(read_through, walked)| read_through != walked)
    {
        return false;
    }
    let declared_as = field.ident.as_ref().map_or_else(
        || place.index.to_string(),
        |member| member.unraw().to_string(),
    );
    let name = format!(
        "read_unjudged_{}",
        helper_name_stem(&declared_as, place.variant)
    );
    let defaulted = place.written_under_a_key
        && !place.container_defaulted
        && needs_injected_default(&shape.wraps, has_serde_default(&field.attrs));
    MEMBER_HOOKS.with(|held| {
        held.borrow_mut().push(MemberHook {
            back: read.back.to_string(),
            choosing: place.choosing,
            defaulted,
            held_as: held_as.to_string(),
            index: place.index,
            leaf: read.leaf.to_token_stream().to_string(),
            name,
            read_as: read.read_as.to_token_stream().to_string(),
            renamed_under: place.renamed_under.map(str::to_owned),
            ty: field.ty.to_token_stream().to_string(),
            variant: place.variant.map(str::to_owned),
        });
    });
    true
}

/// The paths `attr` lists, where it is a derive list.
fn derive_list(attr: &Attribute) -> Option<Punctuated<syn::Path, Token![,]>> {
    if !attr.path().is_ident("derive") {
        return None;
    }
    attr.parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
        .ok()
}

fn ends_on(path: &syn::Path, named: &str) -> bool {
    path.segments.last().is_some_and(|last| last.ident == named)
}

/// The path the item derives `Deserialize` by, as its author wrote it.
fn deserialize_derive(attrs: &[Attribute]) -> Option<syn::Path> {
    attrs
        .iter()
        .filter_map(derive_list)
        .flatten()
        .find(|path| ends_on(path, "Deserialize"))
}

/// `attrs` with `Deserialize` taken off every derive list, and a list left empty taken off whole.
fn without_deserialize(attrs: Vec<Attribute>) -> Vec<Attribute> {
    attrs
        .into_iter()
        .filter_map(|attr| {
            let Some(listed) = derive_list(&attr) else {
                return Some(attr);
            };
            if !listed.iter().any(|path| ends_on(path, "Deserialize")) {
                return Some(attr);
            }
            let kept: Vec<&syn::Path> = listed
                .iter()
                .filter(|path| !ends_on(path, "Deserialize"))
                .collect();
            (!kept.is_empty()).then(|| syn::parse_quote! { #[derive(#(#kept),*)] })
        })
        .collect()
}

fn without_serde(attrs: &mut Vec<Attribute>) {
    attrs.retain(|attr| !attr.path().is_ident("serde"));
}

/// Takes the item's own `Deserialize` off it. Where it derives no `Serialize` either, no derive
/// is left to read its serde attributes, and they are taken off with it.
fn leave_the_read_to_the_macro<'item, F>(attrs: &mut Vec<Attribute>, fields: F)
where
    F: Iterator<Item = &'item mut Field>,
{
    *attrs = without_deserialize(mem::take(attrs));
    if derives_serialize(attrs) {
        return;
    }
    without_serde(attrs);
    for field in fields {
        without_serde(&mut field.attrs);
    }
}

/// `tokens` with every span the macro's own, so a copy of what the author wrote is judged as
/// generated code is.
fn generated(tokens: TokenStream) -> TokenStream {
    tokens
        .into_iter()
        .map(|token| {
            let TokenTree::Group(group) = &token else {
                let mut own = token;
                own.set_span(Span::call_site());
                return own;
            };
            let mut inner = Group::new(group.delimiter(), generated(group.stream()));
            inner.set_span(Span::call_site());
            TokenTree::Group(inner)
        })
        .collect()
}

/// `tokens` with `named` written wherever they write `Self`, which a copy of the item would read
/// as itself.
fn naming_the_type(tokens: TokenStream, named: &TokenStream) -> TokenStream {
    tokens
        .into_iter()
        .flat_map(|token| match token {
            TokenTree::Group(group) => {
                let mut inner =
                    Group::new(group.delimiter(), naming_the_type(group.stream(), named));
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

/// What a copy of the item keeps of a declaration's attributes: what serde's derive reads, and
/// what takes a member out of a build.
fn kept_by_the_copy(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("serde") || attr.path().is_ident("cfg"))
        .cloned()
        .collect()
}

/// The attributes that hang the hook `named` on a member of the copy, with the `default` that
/// reads a missing key as it is read with no hook where `defaulted`.
fn hook_attrs(named: &Ident, defaulted: bool) -> Vec<Attribute> {
    let path = LitStr::new(&named.to_string(), Span::call_site());
    let mut attrs: Vec<Attribute> = vec![syn::parse_quote! { #[serde(deserialize_with = #path)] }];
    if defaulted {
        attrs.push(syn::parse_quote! { #[serde(default)] });
    }
    attrs
}

/// Whether `attr` names the function a member is read through.
fn names_a_read_hook(attr: &Attribute) -> bool {
    read_hook(slice::from_ref(attr)).is_some()
}

/// The members of the copy as serde's derive reads them: each with the attributes it keeps, and
/// with the hook hung on its place in `variant`.
fn copied_fields(fields: &mut Fields, variant: Option<&str>, hung: &[HungOn]) {
    for (index, field) in fields.iter_mut().enumerate() {
        field.attrs = kept_by_the_copy(&field.attrs);
        field.vis = syn::Visibility::Inherited;
        let hooked = hung
            .iter()
            .find(|hook| hook.index == index && hook.variant.as_deref() == variant);
        // serde takes one function per member, and the hook calls the one it is read in place of.
        if hooked.is_some_and(|hook| hook.replaces) {
            field.attrs.retain(|attr| !names_a_read_hook(attr));
        }
        field
            .attrs
            .extend(hooked.into_iter().flat_map(|hook| hook.attrs.clone()));
    }
}

/// The copy of the item serde's derive writes the unjudged read on: its members under the name
/// `copied_as`, with `remote` naming the item, the read's hooks, and the bounds of the read where
/// the item's own attributes write none.
fn remote_definition(
    input: &DeriveInput,
    copied_as: &Ident,
    hung: &[HungOn],
    bounds: &ReadBounds,
    derive: &syn::Path,
) -> TokenStream {
    let name = &input.ident;
    let (_, type_generics, _) = input.generics.split_for_impl();
    let named = quote! { #name #type_generics };

    let mut copy = input.clone();
    copy.ident = copied_as.clone();
    copy.vis = syn::Visibility::Inherited;
    copy.attrs = kept_by_the_copy(&input.attrs);
    match &mut copy.data {
        syn::Data::Struct(held) => copied_fields(&mut held.fields, None, hung),
        syn::Data::Enum(held) => {
            for variant in &mut held.variants {
                variant.attrs = kept_by_the_copy(&variant.attrs);
                variant.discriminant = None;
                let written = variant.ident.to_string();
                copied_fields(&mut variant.fields, Some(&written), hung);
            }
        }
        syn::Data::Union(_) => {}
    }

    let remote = LitStr::new(&name.to_string(), Span::call_site());
    // serde's derive holds the copy, and not the item, to `Default` for a container `default`,
    // so the bounds are written out for it wherever the item leaves them to the derive.
    let bound = (!bounds.written_by_container).then(|| {
        let predicates = bounds
            .predicates
            .iter()
            .map(|predicate| naming_the_type(predicate.to_token_stream(), &named));
        let written = LitStr::new(&quote! { #(#predicates),* }.to_string(), Span::call_site());
        quote! { #[serde(bound(deserialize = #written))] }
    });
    let declared = naming_the_type(copy.to_token_stream(), &named);
    generated(quote! {
        #[derive(#derive)]
        #[serde(remote = #remote)]
        #bound
        #declared
    })
}

/// What the `unjudged` read of a type answers where the type has none of its own: serde's read
/// of it. An inherent function is found ahead of a trait's, so a type that has one answers with
/// its own.
#[cfg(feature = "serde")]
fn held_fallback(named: &Ident, unjudged: &Ident) -> TokenStream {
    quote! {
        trait #named<'de>: serde::Deserialize<'de> {
            fn #unjudged<D>(deserializer: D) -> ::core::result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                <Self as serde::Deserialize<'de>>::deserialize(deserializer)
            }
        }
        impl<'de, T: serde::Deserialize<'de>> #named<'de> for T {}
    }
}

/// What a type answers of [`BOUND_ALONE`] where it carries no such bound: nothing, so its shape
/// alone chooses the variant. An inherent method is found ahead of the one the trait gives a
/// reference, so a brand that carries a bound answers with its own.
#[cfg(feature = "serde")]
fn chosen_by_shape(named: &Ident) -> TokenStream {
    let bound_alone = Ident::new(BOUND_ALONE, Span::call_site());
    quote! {
        trait #named {
            fn #bound_alone(&self) -> ::core::result::Result<(), ::std::vec::Vec<::std::string::String>> {
                ::core::result::Result::Ok(())
            }
        }
        impl<T: ?::core::marker::Sized> #named for &T {}
    }
}

/// A name for a type parameter of a function written for an item: `base`, numbered where one of
/// `written`, the names the item writes, is that name. The parameter would hide a type of its
/// name wherever the function writes one, and be refused beside a parameter of the type's own.
pub(super) fn unclaimed_parameter(base: &str, written: &[String]) -> Ident {
    let mut named = base.to_owned();
    let mut tried: usize = 1;
    while written.contains(&named) {
        tried = tried.saturating_add(1);
        named = format!("{base}{tried}");
    }
    Ident::new(&named, Span::call_site())
}

/// The name of the deserializer's type in every function written for `input`.
fn reader_parameter(input: &DeriveInput) -> Ident {
    unclaimed_parameter("D", &written_names(input.to_token_stream()))
}

/// What the hooks recorded for the item being expanded add to its read, each model read through
/// its own `unjudged` read, from the payload the member's own functions corrected.
#[cfg(feature = "serde")]
fn unjudged_hooks(
    unjudged: &Ident,
    reader: &Ident,
    written: &[String],
    corrects: &Corrects,
) -> Hooked {
    MEMBER_HOOKS.with(|held| {
        let hooks = held.borrow();
        if hooks.is_empty() {
            return Hooked::default();
        }
        let fallback = held_fallback(&unclaimed_parameter(FALLBACK, written), unjudged);
        let chosen = hooks
            .iter()
            .any(|hook| hook.choosing)
            .then(|| chosen_by_shape(&unclaimed_parameter(CHOSEN_BY_SHAPE, written)));
        let functions = hooks
            .iter()
            .map(|hook| hook.function(unjudged, reader, corrects));
        Hooked {
            functions: quote! { #fallback #chosen #(#functions)* },
            hung: hooks
                .iter()
                .map(|hook| HungOn {
                    attrs: hook_attrs(&corrects.hook_name(&hook.name), hook.defaulted),
                    index: hook.index,
                    replaces: false,
                    variant: hook.variant.clone(),
                })
                .collect(),
        }
    })
}

/// No hook is recorded in a build that reads no serde attribute: a model below judges itself.
#[cfg(not(feature = "serde"))]
fn unjudged_hooks(
    _unjudged: &Ident,
    _reader: &Ident,
    _written: &[String],
    _corrects: &Corrects,
) -> Hooked {
    Hooked::default()
}

/// The members of `input`, each with its place and the name of the variant that holds it.
fn members_of(input: &DeriveInput) -> Vec<(Option<String>, usize, &Field)> {
    match &input.data {
        syn::Data::Struct(held) => held
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| (None, index, field))
            .collect(),
        syn::Data::Enum(held) => held
            .variants
            .iter()
            .flat_map(|variant| {
                let named = variant.ident.to_string();
                variant
                    .fields
                    .iter()
                    .enumerate()
                    .map(move |(index, field)| (Some(named.clone()), index, field))
            })
            .collect(),
        syn::Data::Union(_) => Vec::new(),
    }
}

/// Whether a missing key of `field` is read as `None` where no hook is hung on it: a named field
/// written as an `Option`, itself or under an ownership wrapper.
fn reads_a_missing_key(field: &Field) -> bool {
    if field.ident.is_none() {
        return false;
    }
    let mut current = &field.ty;
    loop {
        let syn::Type::Path(named) = written_type(current) else {
            return false;
        };
        let Some(last) = named.path.segments.last() else {
            return false;
        };
        if last.ident == "Option" {
            return true;
        }
        let syn::PathArguments::AngleBracketed(held) = &last.arguments else {
            return false;
        };
        if !is_ownership_wrapper(&last.ident.to_string()) {
            return false;
        }
        let Some(inner) = sole_type_argument(held) else {
            return false;
        };
        current = inner;
    }
}

/// What the hook of a list held to a count is declared with, where `path` names that hook of
/// `field`: its type parameters, the type it reads and the bounds it holds that type to.
#[cfg(feature = "serde")]
fn counted_list_header(
    field: &Field,
    key: &str,
    path: &str,
) -> Option<(TokenStream, TokenStream, TokenStream)> {
    let renames = path
        .rsplit("::")
        .next()
        .is_some_and(|last| last.starts_with(NAMED_READ_HOOK_PREFIX));
    if key != "deserialize_with" || renames {
        return None;
    }
    let shape = constrained_shape(&field.ty)?;
    if !matches!(shape.leaf, ConstraintLeaf::List) {
        return None;
    }
    let header = list_hook_header(&shape.wraps)?;
    let (holders, read, held_to, counted) = (
        &header.holders,
        &header.read,
        &header.held_to,
        &header.counted,
    );
    Some((
        quote! { #(#holders,)* L },
        read.clone(),
        quote! { #read: serde::Deserialize<'de>, #(#held_to)* #counted },
    ))
}

/// No list is held to a count in a build that reads no serde attribute.
#[cfg(not(feature = "serde"))]
const fn counted_list_header(
    _field: &Field,
    _key: &str,
    _path: &str,
) -> Option<(TokenStream, TokenStream, TokenStream)> {
    None
}

/// The hook a member that names `functions` is read through where the macro hangs no other on
/// the copy: it reads the payload, calls the functions, and reads the member from what they
/// answer. A member the macro already reads through a function of the type's schema module, or
/// through a module serde names, is read through that one, from the corrected payload.
fn corrected_hook(
    field: &Field,
    named: &Ident,
    functions: &[FunctionName],
    reader: &Ident,
    written: &[String],
    corrects: &Corrects,
) -> TokenStream {
    let (deserializer, corrected) = (&corrects.deserializer, &corrects.corrected);
    let correction = corrects.statements(functions);
    let told = told_by(reader);
    let Some((key, path)) = read_hook(&field.attrs) else {
        let read = unclaimed_parameter(READ, written);
        return quote! {
            fn #named<'de, #reader, #read>(#deserializer: #reader) -> ::core::result::Result<#read, #reader::Error>
            where
                #reader: serde::Deserializer<'de>,
                #read: serde::Deserialize<'de>,
            {
                #correction
                <#read as serde::Deserialize>::deserialize(#corrected) #told
            }
        };
    };
    let through = path.parse::<syn::ExprPath>().map_or_else(
        |_| TokenStream::new(),
        |function| {
            if key == "with" {
                quote! { #function::deserialize }
            } else {
                quote! { #function }
            }
        },
    );
    let (parameters, read, held_to) = counted_list_header(field, key, &path.value())
        .unwrap_or_else(|| {
            let mut lifetimes: Vec<syn::Lifetime> = Vec::new();
            written_lifetimes(&field.ty, &mut lifetimes);
            let mut declared: Vec<syn::Lifetime> = Vec::new();
            for lifetime in lifetimes {
                if lifetime.ident != "static" && !declared.contains(&lifetime) {
                    declared.push(lifetime);
                }
            }
            let ty = &field.ty;
            (
                quote! { #(#declared,)* },
                quote! { #ty },
                TokenStream::new(),
            )
        });
    quote! {
        fn #named<'de, #reader, #parameters>(#deserializer: #reader) -> ::core::result::Result<#read, #reader::Error>
        where
            #reader: serde::Deserializer<'de>,
            #held_to
        {
            #correction
            #through(#corrected) #told
        }
    }
}

/// What the hooks of the item's members add to its read: each model a member holds read through
/// its own `unjudged` read, and each member that names a `preprocess` read from the payload its
/// functions corrected.
fn hooked(
    input: &DeriveInput,
    unjudged: &Ident,
    reader: &Ident,
    written: &[String],
    corrects: &Corrects,
) -> Hooked {
    let mut hooked = unjudged_hooks(unjudged, reader, written, corrects);
    let container_defaulted = has_serde_default(&input.attrs);
    for (variant, index, field) in members_of(input) {
        let functions = corrects.named.of_member(variant.as_deref(), index);
        let taken = hooked
            .hung
            .iter()
            .any(|hung| hung.index == index && hung.variant == variant);
        if functions.is_empty() || taken {
            continue;
        }
        let stem = field
            .ident
            .as_ref()
            .map_or_else(|| index.to_string(), |member| member.unraw().to_string());
        let placed = variant.as_ref().map_or_else(
            || stem.clone(),
            |held| format!("{}_{stem}", to_snake_case(held)),
        );
        let named = corrects.hook_name(&format!("read_corrected_{placed}"));
        let chained = read_hook(&field.attrs).is_some();
        // A hook the copy keeps brings the `default` it was hung with.
        let defaulted = !chained
            && !container_defaulted
            && !has_serde_default(&field.attrs)
            && reads_a_missing_key(field);
        hooked.functions.extend(corrected_hook(
            field, &named, functions, reader, written, corrects,
        ));
        hooked.hung.push(HungOn {
            attrs: hook_attrs(&named, defaulted),
            index,
            replaces: chained,
            variant,
        });
    }
    hooked
}

/// The read alone of `input`, a function of the type under the name `unjudged`: serde's derive on
/// a copy of the item, with each model a member holds read through that model's own `unjudged`
/// read. It is written under the bounds serde's derive writes on the impl it would have written,
/// which are answered beside it, or under none of its own where `asked_by_the_impl`: a bound
/// asked twice, by the impl and by the function, is one the compiler cannot choose between.
///
/// Where the type names a `preprocess` of its own, the function reads the payload, calls each
/// function on it, and hands what they answer to a second function, written beside it, which
/// reads the copy: the read of a payload already corrected.
fn unjudged_read(
    input: &DeriveInput,
    derive: &syn::Path,
    unjudged: &Ident,
    doc: &str,
    asked_by_the_impl: bool,
) -> (TokenStream, ReadBounds) {
    let bounds = read_bounds(input);
    // Each name the function declares is one the item does not write: the copy writes the item's
    // own types beside them.
    let written = written_names(input.to_token_stream());
    let reader = unclaimed_parameter("D", &written);
    let corrects = Corrects::of(&written);
    let Hooked { functions, hung } = hooked(input, unjudged, &reader, &written, &corrects);
    let remote_name = unclaimed_parameter(REMOTE, &written);
    let remote = remote_definition(input, &remote_name, &hung, &bounds, derive);
    let (_, type_generics, _) = input.generics.split_for_impl();
    let predicates: &[syn::WherePredicate] = if asked_by_the_impl {
        &[]
    } else {
        &bounds.predicates
    };
    // serde's derive reads at `'static` an item that borrows for it, and at a `'de` that outlives
    // every other lifetime the item borrows for.
    let (de, read_parameters) = if bounds.borrowed.is_some() {
        (quote! { 'de }, quote! { <'de, #reader> })
    } else {
        (quote! { 'static }, quote! { <#reader> })
    };
    let borrowing = outlived(&bounds);
    let borrowed_for: Vec<&syn::WherePredicate> = borrowing.iter().collect();
    let own = &corrects.named.own;
    // A hook is declared inside the function that reads the copy, where that function's own
    // parameter is in scope: it goes by a name no function a hook calls has.
    let (deserializer, corrected) = (&corrects.deserializer, &corrects.corrected);
    if own.is_empty() {
        let read = quote! {
            #[doc = #doc]
            pub fn #unjudged #read_parameters (#deserializer: #reader) -> ::core::result::Result<Self, #reader::Error>
            where
                #reader: serde::Deserializer<#de>,
                #(#borrowed_for,)*
                #(#predicates,)*
            {
                #functions
                #remote
                <#remote_name #type_generics>::deserialize(#deserializer)
            }
        };
        return (read, bounds);
    }
    let correction = corrects.statements(own);
    let told = told_by(&reader);
    let of_corrected = corrected_read(unjudged);
    let read = quote! {
        #[doc = #doc]
        pub fn #unjudged #read_parameters (#deserializer: #reader) -> ::core::result::Result<Self, #reader::Error>
        where
            #reader: serde::Deserializer<#de>,
            #(#borrowed_for,)*
            #(#predicates,)*
        {
            #correction
            Self::#of_corrected(#corrected) #told
        }

        /// The read of a payload the functions `preprocess` names on the type have corrected.
        pub fn #of_corrected #read_parameters (#deserializer: #reader) -> ::core::result::Result<Self, #reader::Error>
        where
            #reader: serde::Deserializer<#de>,
            #(#borrowed_for,)*
            #(#predicates,)*
        {
            #functions
            #remote
            <#remote_name #type_generics>::deserialize(#deserializer)
        }
    };
    (read, bounds)
}

/// `'de: 'a + 'b` for the lifetimes the read borrows for, where it borrows for any.
fn outlived(bounds: &ReadBounds) -> Option<syn::WherePredicate> {
    bounds
        .borrowed
        .as_ref()
        .filter(|borrowed| !borrowed.is_empty())
        .map(|borrowed| syn::parse_quote! { 'de: #(#borrowed)+* })
}

/// `model_schema_read_unjudged` and the `Deserialize` that calls it, for an item that derives
/// `Deserialize` by `derive`: with `validate()` run after it where the item `publishes_validate`.
fn written_read(input: &DeriveInput, derive: &syn::Path, publishes_validate: bool) -> TokenStream {
    let name = &input.ident;
    let unjudged = Ident::new(UNJUDGED_READ, Span::call_site());
    let (read, bounds) = unjudged_read(
        input,
        derive,
        &unjudged,
        " The read alone, for a holder that judges the whole value once.",
        false,
    );
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let reader = reader_parameter(input);
    let refusal = refusal_from_violations();

    let mut read_at = input.generics.clone();
    let de = if bounds.borrowed.is_some() {
        read_at.params.insert(0, syn::parse_quote! { 'de });
        quote! { 'de }
    } else {
        quote! { 'static }
    };
    let read_where = read_at.make_where_clause();
    read_where.predicates.extend(outlived(&bounds));
    read_where.predicates.extend(bounds.predicates);
    let (read_impl_generics, _, read_where_clause) = read_at.split_for_impl();
    let judged = if publishes_validate {
        quote! {
            let read = Self::#unjudged(deserializer)?;
            read.validate().map_err(#refusal)?;
            ::core::result::Result::Ok(read)
        }
    } else {
        quote! { Self::#unjudged(deserializer) }
    };
    quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #read
        }

        impl #read_impl_generics serde::Deserialize<#de> for #name #type_generics #read_where_clause {
            fn deserialize<#reader>(deserializer: #reader) -> ::core::result::Result<Self, #reader::Error>
            where
                #reader: serde::Deserializer<#de>,
            {
                #judged
            }
        }
    }
}

/// What the unjudged read of the unit struct `name` is written as where the struct names a
/// `preprocess`: `model_schema_read_unjudged`, which calls each function on the payload and reads
/// what they answer through the function it is answered beside, and that function's name.
#[cfg(feature = "serde")]
pub fn unit_struct_correction(name: &Ident) -> Option<(TokenStream, Ident)> {
    let corrects = Corrects::of(&[name.to_string()]);
    let own = &corrects.named.own;
    if own.is_empty() {
        return None;
    }
    let unjudged = Ident::new(UNJUDGED_READ, Span::call_site());
    let of_corrected = corrected_read(&unjudged);
    let (deserializer, corrected) = (&corrects.deserializer, &corrects.corrected);
    let correction = corrects.statements(own);
    Some((
        quote! {
            /// The read alone, for a holder that judges the whole value once.
            pub fn #unjudged<'de, D: ::serde::Deserializer<'de>>(#deserializer: D) -> ::core::result::Result<Self, D::Error> {
                #correction
                Self::#of_corrected(#corrected).map_err(<D::Error as ::serde::de::Error>::custom)
            }
        },
        of_corrected,
    ))
}

/// The refusal of a conversion of the author's own on an item whose read the macro writes: one
/// that publishes `validate()`, or one that `corrects` its payload with a `preprocess`.
fn conversion_refusal(
    input: &DeriveInput,
    key: &str,
    written_at: Span,
    corrects: bool,
) -> TokenStream {
    let name = &input.ident;
    let refusal = if corrects {
        format!(
            "type `{name}`: `#[serde({key} = ...)]` puts a conversion of its author's between \
             the type and what serde reads or writes for it, and `{name}` names a `preprocess`, \
             so `#[model_schema()]` writes its `Deserialize`: each function called on the \
             payload, then the read of its members. It cannot put that read behind a conversion \
             it does not write. Take `{key}` off the type and convert where the value is read, \
             or take the `preprocess` off the type and its members."
        )
    } else {
        format!(
            "type `{name}`: `#[serde({key} = ...)]` puts a conversion of its author's between \
             the type and what serde reads or writes for it, and `{name}` publishes \
             `validate()`, so `#[model_schema()]` writes its `Deserialize`: the read of its \
             members, then `validate()`. It cannot put that judgement behind a conversion it \
             does not write. Take `{key}` off the type and convert where the value is read, or \
             take off the bounds and the checks that make the type publish `validate()`."
        )
    };
    syn::Error::new(written_at, prefixed_guard_message(&refusal)).to_compile_error()
}

/// What an item that derives `Deserialize` gains where it publishes `validate()` or names a
/// `preprocess`, on itself or on a member, or the refusal of a conversion written on it. `None`
/// for every other item, which is left as it is.
fn judged(input: &DeriveInput, publishes_validate: bool) -> Option<Judged> {
    let corrects = preprocess::named().any();
    if !(publishes_validate || corrects) || !derives_deserialize(&input.attrs) {
        return None;
    }
    let derive = deserialize_derive(&input.attrs)?;
    Some(match serde_conversion(&input.attrs) {
        Some((key, written_at)) => {
            Judged::Refused(conversion_refusal(input, key, written_at, corrects))
        }
        None => Judged::Written(written_read(input, &derive, publishes_validate)),
    })
}

/// What a struct's read gains where it derives `Deserialize` and publishes `validate()` or names
/// a `preprocess`, taking the struct's own `Deserialize` off it. Nothing for any other struct.
pub fn of_struct(item: &mut ItemStruct, publishes_validate: bool) -> TokenStream {
    match judged(&DeriveInput::from(item.clone()), publishes_validate) {
        Some(Judged::Written(read)) => {
            leave_the_read_to_the_macro(&mut item.attrs, item.fields.iter_mut());
            read
        }
        Some(Judged::Refused(refusal)) => refusal,
        None => TokenStream::new(),
    }
}

/// [`of_struct`] for an enum.
pub fn of_enum(item: &mut ItemEnum, publishes_validate: bool) -> TokenStream {
    match judged(&DeriveInput::from(item.clone()), publishes_validate) {
        Some(Judged::Written(read)) => {
            if !derives_serialize(&item.attrs) {
                for variant in &mut item.variants {
                    without_serde(&mut variant.attrs);
                }
            }
            leave_the_read_to_the_macro(
                &mut item.attrs,
                item.variants
                    .iter_mut()
                    .flat_map(|variant| variant.fields.iter_mut()),
            );
            read
        }
        Some(Judged::Refused(refusal)) => refusal,
        None => TokenStream::new(),
    }
}

/// `decode_with_read_unjudged`, the read the recovering decode of `input` lists from, where a
/// schema surface is on: the type's own unjudged read where it publishes `validate()`, and
/// serde's where it publishes none. Every holder of a model publishes `validate()` there, so that
/// read holds nothing judged.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
pub fn listed_read(input: &DeriveInput) -> TokenStream {
    delegated_listed_read(input, false)
}

/// [`listed_read`] of a brand that carries a bound, which no walk lists: the brand's unjudged
/// read, refused where [`BOUND_ALONE`] answers a violation of the value read. The checks the
/// brand names are listed beside it, from the value this read answers.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
pub fn bounded_listed_read(input: &DeriveInput) -> TokenStream {
    delegated_listed_read(input, true)
}

/// `decode_with_read_unjudged`, the read the recovering decode of `input` lists from, in a build
/// with no schema surface. A holder that names no check publishes no `validate()` there, and its
/// own read judges the models it holds: so the item's members are read here, on a copy of the
/// item, with every model below read through its own `decode_with_read_unjudged`.
#[cfg(all(
    feature = "serde",
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema"))
))]
pub fn listed_read(input: &DeriveInput) -> TokenStream {
    let own_members = deserialize_derive(&input.attrs)
        .filter(|_| serde_conversion(&input.attrs).is_none() && !is_a_brand(input));
    let Some(derive) = own_members else {
        return delegated_listed_read(input, false);
    };
    let unjudged = Ident::new(LISTED_READ, Span::call_site());
    unjudged_read(
        input,
        &derive,
        &unjudged,
        " The read alone, with every model below read unjudged, for the recovering read to list \
         from.",
        // The recovering read's own `impl` asks what serde's derive asks to read the type.
        true,
    )
    .0
}

/// Whether serde writes the item as the value of one of its fields, which a read of its own
/// holds to its bound.
#[cfg(all(
    feature = "serde",
    not(any(feature = "typescript", feature = "zod", feature = "jsonschema"))
))]
fn is_a_brand(input: &DeriveInput) -> bool {
    has_serde_transparent(&input.attrs)
}

/// `decode_with_read_unjudged` of a type whose own unjudged read is the one to list from, refused
/// where the type is `bounded` and [`BOUND_ALONE`] answers a violation of the value read.
#[cfg(feature = "serde")]
fn delegated_listed_read(input: &DeriveInput, bounded: bool) -> TokenStream {
    let (listed, unjudged) = (
        Ident::new(LISTED_READ, Span::call_site()),
        Ident::new(UNJUDGED_READ, Span::call_site()),
    );
    let reader = reader_parameter(input);
    let fallback = held_fallback(&Ident::new(FALLBACK, Span::call_site()), &unjudged);
    let (bound_alone, refusal) = (
        Ident::new(BOUND_ALONE, Span::call_site()),
        refusal_from_violations(),
    );
    let answered = |read: TokenStream| {
        if !bounded {
            return read;
        }
        quote! {
            let read = #read?;
            read.#bound_alone().map_err(#refusal)?;
            ::core::result::Result::Ok(read)
        }
    };
    let of_unjudged = answered(quote! { <Self>::#unjudged(deserializer) });
    // A type that names a `preprocess` of its own has the read of a corrected payload beside its
    // unjudged read, and the recovering read lists from it once it has called the functions.
    let of_corrected = (!preprocess::named().own.is_empty()).then(|| {
        let (listed_corrected, corrected) = (corrected_read(&listed), corrected_read(&unjudged));
        let read = answered(quote! { Self::#corrected(deserializer) });
        quote! {
            /// The read of a payload the functions `preprocess` names on the type have corrected,
            /// for the recovering read to list from.
            pub fn #listed_corrected<'de, #reader>(deserializer: #reader) -> ::core::result::Result<Self, #reader::Error>
            where
                #reader: serde::Deserializer<'de>,
            {
                #read
            }
        }
    });
    quote! {
        /// The read alone, for the recovering read to list from: the type's unjudged read where
        /// it publishes `validate()`, and serde's where it publishes none.
        pub fn #listed<'de, #reader>(deserializer: #reader) -> ::core::result::Result<Self, #reader::Error>
        where
            #reader: serde::Deserializer<'de>,
        {
            #fallback
            #of_unjudged
        }

        #of_corrected
    }
}

/// The name of the read the recovering decode lists from: a type's unjudged read, which reads
/// what it holds unjudged wherever a schema surface is on.
#[cfg(feature = "serde")]
fn listed() -> Ident {
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    let named = UNJUDGED_READ;
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    let named = LISTED_READ;
    Ident::new(named, Span::call_site())
}

/// A function that reads a `ty` with every model it holds read unjudged, for the recovering
/// decode to hold a value it lists the items of to a bound of its own. `None` where a hook would
/// read no model in `ty`, or `ty` names what an item nested in a function cannot.
#[cfg(feature = "serde")]
pub fn listed_reader(ty: &syn::Type, type_parameters: &[String]) -> Option<TokenStream> {
    if names_what_the_module_cannot_repeat(ty, type_parameters) {
        return None;
    }
    let written = written_names(ty.to_token_stream());
    let held_as = unclaimed_parameter(HELD, &written);
    let read = reached(ty, 0, &held_as)?;
    let unjudged = listed();
    let fallback = held_fallback(&unclaimed_parameter(FALLBACK, &written), &unjudged);
    let (leaf, read_as, back) = (&read.leaf, &read.read_as, &read.back);
    let reader_type = unclaimed_parameter("D", &written);
    Some(quote! {
        |reader| {
            #fallback
            struct #held_as(#leaf);
            impl<'de> serde::Deserialize<'de> for #held_as {
                fn deserialize<#reader_type>(deserializer: #reader_type) -> ::core::result::Result<Self, #reader_type::Error>
                where
                    #reader_type: serde::Deserializer<'de>,
                {
                    <#leaf>::#unjudged(deserializer).map(#held_as)
                }
            }
            <#read_as as serde::Deserialize>::deserialize(reader).map(|held_0| #back)
        }
    })
}

#[cfg(test)]
mod tests;
