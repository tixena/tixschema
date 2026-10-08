//! The typed MongoDB paths `#[model_schema(decode_with)]` adds under `mongodb`.
//!
//! A flagged type gets `MongoFields<Root>` in its `{type}_schema` module: one path per key serde
//! writes for the type, under the name of the field that writes it, in a row of `Root`. On the
//! type, `mongo_fields_under` builds that struct under the keys leading to a value of the type,
//! and `MONGO_FIELDS` is that struct for the type as the row itself. A field typed with a struct
//! or an enum `#[model_schema]` was written on above holds that type's own struct, built by that
//! type's own function under the field's key, so no type writes another's keys. A field of any
//! other type is one whole value. The module reads the types its author wrote through `super`.
//!
//! A key is the name serde writes: a renaming written as a list counts by its `serialize` side,
//! in every build. A type serde writes as one value is one path, `MongoFields<Root, Whole>`,
//! whose type the type's own `impl` and whoever holds the struct write, so that its module names
//! no type of its author's. A list of such a type is a list of plain values.

use core::iter::{once, repeat_with};

use proc_macro2::{Group, Ident, Spacing, Span, TokenStream, TokenTree};
use quote::{ToTokens as _, format_ident, quote};
use syn::ext::IdentExt as _;
use syn::{
    Field, Fields, FieldsNamed, FieldsUnnamed, GenericParam, Generics, ItemEnum, ItemStruct,
    PathArguments, PathSegment, Type, Variant, WherePredicate, parse_quote,
};

use super::aliases::PRIMITIVES;
use super::enums::Tagging;
use super::{
    Step, TypedPaths, hooked_leaf, starts_at_a_parameter, type_arguments, unclaimed_parameter,
    value_slot, written_names,
};
use crate::features::serde::{
    has_serde_skip_serializing, parse_serde_field_attributes, parse_serde_field_hooks,
    parse_serde_key_omission, parse_written_renames,
};
use crate::field_type::{
    FieldDefType, get_field_def, is_refused_sequence_wrapper, is_sequence_wrapper,
    is_transparent_wrapper,
};
use crate::model_schema::helper_name_stem;
use crate::rename_rule::resolve_rename_rule;
use crate::utils::{Declared, declared, ident_schema_module_name, to_snake_case, written_type};

/// How many modules below the type its members module is.
const IN_MEMBERS: usize = 2;

/// How many modules below the type its schema module is.
const IN_MODULE: usize = 1;

/// The words a type is written with that name nothing.
const KEYWORDS: [&str; 15] = [
    "_", "as", "const", "dyn", "extern", "false", "fn", "for", "impl", "in", "mut", "ref", "true",
    "unsafe", "where",
];

/// The types and traits the standard prelude brings into every module, none of which is a member
/// of the module above.
const PRELUDE: [&str; 39] = [
    "AsMut",
    "AsRef",
    "AsyncFn",
    "AsyncFnMut",
    "AsyncFnOnce",
    "Box",
    "Clone",
    "Copy",
    "Default",
    "DoubleEndedIterator",
    "Drop",
    "Eq",
    "ExactSizeIterator",
    "Extend",
    "Fn",
    "FnMut",
    "FnOnce",
    "From",
    "FromIterator",
    "Future",
    "Into",
    "IntoFuture",
    "IntoIterator",
    "Iterator",
    "Option",
    "Ord",
    "PartialEq",
    "PartialOrd",
    "Result",
    "Send",
    "Sized",
    "String",
    "Sync",
    "ToOwned",
    "ToString",
    "TryFrom",
    "TryInto",
    "Unpin",
    "Vec",
];

/// One variant as a filter asks for it.
struct Ask {
    /// serde writes the variant as an object under its name, where it writes one that holds
    /// nothing as that name alone.
    holds: bool,
    /// The name serde writes the variant under.
    name: String,
    stem: String,
}

/// What an enum's struct of paths keeps beside its members: the path its `is_{variant}` methods
/// read, and those methods.
struct Asked {
    methods: TokenStream,
    /// The path, as the type's own function builds it.
    path: TokenStream,
}

/// The paths of one type, as they are gathered: what `mongo_members` holds and what the type's
/// `impl` gains beside the function that builds them.
struct Emitter<'item> {
    /// The structs `mongo_members` holds, each under its name.
    held: Vec<(String, TokenStream)>,
    own: Own<'item>,
    /// The functions hooked paths write their values through, each under its name.
    writers: Vec<(String, TokenStream)>,
}

impl<'item> Emitter<'item> {
    /// What one variant holds, where serde writes it, as one member. `None` for a variant with no
    /// path of its own.
    fn content(&mut self, variant: &Variant, of: &OfVariant<'_>) -> Option<Member> {
        let place = Place {
            key: match of.put {
                Put::Beside | Put::InPlace => None,
                Put::Under(content) => Some((*content).to_owned()),
                Put::UnderItsName => Some(of.name.to_owned()),
            },
            prefix: quote! { prefix },
        };
        let below = place.below(&self.own.module);
        match &variant.fields {
            Fields::Named(named) => {
                let rule = parse_written_renames(&variant.attrs).rename_all;
                let members = self.keyed(
                    named,
                    rule.as_deref().or(of.fields_rule),
                    &below,
                    Some(of.stem),
                    of.read,
                );
                self.held(of.rust_name, &of.told(), &Members::Named(members))
            }
            Fields::Unit => None,
            Fields::Unnamed(slots) => {
                let mut written = slots.unnamed.iter();
                if let (Some(only), None) = (written.next(), written.next()) {
                    return self.only_slot(only, &place, of);
                }
                // serde's derive refuses several slots under a tag written beside them.
                if matches!(of.put, Put::Beside) {
                    return None;
                }
                let members = self.positional(slots, &below, Some(of.stem), of.read);
                self.held(of.rust_name, &of.told(), &Members::Positional(members))
            }
        }
    }

    /// What the paths add: `declared` beside the structs `mongo_members` holds, and on the type
    /// the const, the function answering `built`, and the functions hooked paths write through.
    /// `one_value` says the struct of paths is one path, whose type the type's `impl` writes.
    fn emitted(self, declared: &TokenStream, built: &TokenStream, one_value: bool) -> TypedPaths {
        let (module, root, arguments) = (&self.own.module, &self.own.root, self.own.arguments());
        let (as_the_row, as_asked) = if one_value {
            (
                quote! { , #module::Field<Self, Self> },
                quote! { , #module::Field<#root, Self> },
            )
        } else {
            (TokenStream::new(), TokenStream::new())
        };
        let members = (!self.held.is_empty()).then(|| {
            let held = self.held.iter().map(|(_named, written)| written);
            quote! {
                /// The structs of paths `MongoFields` holds: one per variant, and per tuple, with
                /// paths of its own.
                pub mod mongo_members {
                    use super::*;

                    #(#held)*
                }
            }
        });
        // A type with no key of its own reads nothing of what leads to it.
        let prefix = if written_names(built.clone()).contains(&"prefix".to_owned()) {
            quote! { prefix }
        } else {
            quote! { _ }
        };
        let writers = self.writers.iter().map(|(_named, written)| written);
        TypedPaths {
            module_items: quote! {
                #declared
                #members
            },
            type_items: quote! {
                /// The typed MongoDB paths of this type, as the row a filter or an update is over.
                pub const MONGO_FIELDS: #module::MongoFields<Self #arguments #as_the_row> =
                    Self::mongo_fields_under(#module::MongoPath::ROOT);

                /// The typed MongoDB paths of a value of this type held under the keys of `prefix`,
                /// in a row of the type the paths are asked as.
                #[must_use]
                pub const fn mongo_fields_under<#root>(
                    #prefix: [::core::option::Option<&'static str>; 8],
                ) -> #module::MongoFields<#root #arguments #as_asked> {
                    #built
                }

                #(#writers)*
            },
        }
    }

    /// The paths of a flattened field, which are those of the type it is typed with under the
    /// keys leading to the type that flattens it. `None` for any field but one serde reads as a
    /// struct or an enum `#[model_schema]` was written on above, or as an `Option` of one, and
    /// for one serde writes as one value, which has no key to put there.
    fn flattened(&self, field: &Field, prefix: &TokenStream, read: bool) -> Option<Member> {
        let hooks = parse_serde_field_hooks(&field.attrs);
        let hooked = hooked_leaf(
            &field.ty,
            &hooks,
            &self.own.module_name,
            &self.own.type_parameters,
        );
        if !read || hooked.is_some() {
            return None;
        }
        let model = match held(&field.ty) {
            Held::Optional(inner) if matches!(held(inner), Held::Value) => {
                self.own.seen_model(inner)
            }
            Held::Value => self.own.seen_model(&field.ty),
            Held::List(_) | Held::Map | Held::Optional(_) => None,
        }?;
        (!model.one_value).then(|| paths_of(&model, prefix))
    }

    /// A struct of `members` in `mongo_members`, named after `base` and told as `told`, as the
    /// one member that holds it. `None` where no member is a path.
    fn held(&mut self, base: &str, told: &str, members: &Members) -> Option<Member> {
        if members.is_empty() {
            return None;
        }
        let own = &self.own;
        let used = own.used(members);
        let generics = own.declared(IN_MEMBERS, Some(&used));
        // A struct named as a prelude type or as a parameter would be read in its place by every
        // member beside it that names one.
        let taken: Vec<String> = self
            .held
            .iter()
            .map(|(named, _)| named.clone())
            .chain(PRELUDE.map(str::to_owned))
            .chain(own.parameters.iter().cloned())
            .collect();
        let name = Ident::new(&unclaimed(base, &taken), Span::call_site());
        let arguments = own.arguments_among(&used);
        let (module, built) = (&own.module, members.built());
        let written = all_public(&name, members, &generics, told, own, IN_MEMBERS);
        self.held.push((name.to_string(), written));
        Some(Member {
            shown: Shown::Held {
                arguments,
                named: name.clone(),
            },
            value: quote! { #module::mongo_members::#name #built },
        })
    }

    /// The path of a field written through a hook of its own: the value is handed to the hook as
    /// the field holds it, which under an `Option` is `Some`.
    fn hooked(&mut self, ty: &Type, place: &Place, stem: &str, write: &TokenStream) -> Member {
        let module = &self.own.module;
        let path = place.path(module);
        let (of_kind, value, handed) = match held(ty) {
            Held::Optional(inner) if !through_wrappers(ty).1 => (
                "OptionalField",
                inner,
                quote! { &::core::option::Option::Some(value) },
            ),
            Held::List(_) | Held::Map | Held::Optional(_) | Held::Value => {
                ("Field", ty, quote! { &value })
            }
        };
        let taken: Vec<String> = self
            .writers
            .iter()
            .map(|(named, _)| named.clone())
            .collect();
        let named = unclaimed(&format!("mongo_write_{stem}"), &taken);
        let (writer, kind) = (
            Ident::new(&named, Span::call_site()),
            Ident::new(of_kind, Span::call_site()),
        );
        let told = format!("What `{stem}` holds, as its own serde hook writes it.");
        self.writers.push((
            named,
            quote! {
                #[doc = #told]
                fn #writer(value: #value) -> ::core::result::Result<bson::Bson, #module::WriteError> {
                    #write(#handed, bson::Serializer::new())
                }
            },
        ));
        Member {
            shown: Shown::Kind {
                fields: None,
                kind: kind.clone(),
                value: value.to_token_stream(),
            },
            value: quote! { #module::#kind::hooked(#path, Self::#writer) },
        }
    }

    /// One member per field of `named` that serde writes, under the field's own name: the key
    /// serde writes it at under `prefix`, cased by `rename_all` where the field names none, or
    /// what a flattened one holds under `prefix` itself.
    fn keyed(
        &mut self,
        named: &FieldsNamed,
        rename_all: Option<&str>,
        prefix: &TokenStream,
        variant: Option<&str>,
        read: bool,
    ) -> Vec<(Ident, Member)> {
        let mut members: Vec<(Ident, Member)> = Vec::new();
        for field in &named.named {
            let Some(ident) = &field.ident else {
                continue;
            };
            if has_serde_skip_serializing(&field.attrs) {
                continue;
            }
            let field_read = read && !parse_serde_key_omission(&field.attrs).skips_deserializing;
            let meta = parse_serde_field_attributes(&field.attrs);
            let written = ident.unraw().to_string();
            let member = if meta.flatten {
                self.flattened(field, prefix, field_read)
            } else {
                let renamed = parse_written_renames(&field.attrs).rename;
                let place = Place {
                    key: Some(renamed.unwrap_or_else(|| {
                        resolve_rename_rule(rename_all).apply_to_field(&written)
                    })),
                    prefix: prefix.clone(),
                };
                self.member(
                    field,
                    &place,
                    &helper_name_stem(&written, variant),
                    field_read,
                )
            };
            if let Some(found) = member {
                let mut name = ident.clone();
                name.set_span(Span::call_site());
                members.push((name, found));
            }
        }
        members
    }

    /// `MongoFields` holding `members`, with what an enum keeps beside them, and what the type's
    /// own function builds it with. Members are public and what the struct keeps is not, so a
    /// struct with both holds its members in one of `mongo_members` and dereferences to it.
    fn laid_out(&mut self, members: &Members, asked: Option<&Asked>) -> (TokenStream, TokenStream) {
        let used = self.own.used(members);
        let unused: Vec<Ident> = self
            .own
            .type_parameters
            .iter()
            .filter(|parameter| !used.contains(parameter))
            .map(|parameter| Ident::new(parameter, Span::call_site()))
            .collect();
        if asked.is_none() && unused.is_empty() && !members.is_empty() {
            let (own, built) = (&self.own, members.built());
            let module = &own.module;
            let generics = own.declared(IN_MODULE, None);
            let named = Ident::new("MongoFields", Span::call_site());
            return (
                all_public(&named, members, &generics, &own.told(), own, IN_MODULE),
                quote! { #module::MongoFields #built },
            );
        }
        let told = format!(
            "The paths `MongoFields` holds for `{}`, which it dereferences to.",
            self.own.name.unraw()
        );
        let inner = self.held("Members", &told, members);
        let marked = if inner.is_some() {
            unused
        } else {
            let own = &self.own;
            once(own.root.clone())
                .chain(
                    own.type_parameters
                        .iter()
                        .map(|parameter| Ident::new(parameter, Span::call_site())),
                )
                .collect()
        };
        self.own.keeping(inner.as_ref(), asked, &marked)
    }

    /// The path of one field or slot serde writes at `place`: through its own hook where it has
    /// one, and by its type where it has none. `read` says serde reads the value back.
    fn member(&mut self, field: &Field, place: &Place, stem: &str, read: bool) -> Option<Member> {
        let hooks = parse_serde_field_hooks(&field.attrs);
        let hooked = hooked_leaf(
            &field.ty,
            &hooks,
            &self.own.module_name,
            &self.own.type_parameters,
        );
        if let Some(Step::Leaf(whole)) = &hooked
            && let Some(write) = &whole.write
        {
            return Some(self.hooked(&field.ty, place, stem, write));
        }
        // A type is asked for its own paths only where its walker is: a hook reads the value
        // whole, and a value serde never reads back is walked by nothing.
        self.own.typed(&field.ty, place, read && hooked.is_none())
    }

    fn new(name: &'item Ident, generics: &'item Generics, written: &[String]) -> Self {
        let module_name = ident_schema_module_name(&name.to_string());
        let named = |kept: fn(&GenericParam) -> Option<&Ident>| -> Vec<String> {
            generics
                .params
                .iter()
                .filter_map(kept)
                .map(|parameter| parameter.unraw().to_string())
                .collect()
        };
        Self {
            held: Vec::new(),
            own: Own {
                generics,
                module: Ident::new(&module_name, Span::call_site()),
                module_name,
                name,
                parameters: named(|parameter| match parameter {
                    GenericParam::Const(constant) => Some(&constant.ident),
                    GenericParam::Type(ty) => Some(&ty.ident),
                    GenericParam::Lifetime(_) => None,
                }),
                root: unclaimed_parameter("Root", written),
                type_parameters: named(|parameter| match parameter {
                    GenericParam::Type(ty) => Some(&ty.ident),
                    GenericParam::Const(_) | GenericParam::Lifetime(_) => None,
                }),
                value: unclaimed_parameter("Value", written),
                whole: unclaimed_parameter("Whole", written),
            },
            writers: Vec::new(),
        }
    }

    /// What a variant's one slot holds. Under a key of its own it is a member like any field.
    /// With none, the paths of a flagged type serde writes under keys sit at the enum's own
    /// level, and any other value is the enum's own where serde writes it bare.
    fn only_slot(&mut self, only: &Field, place: &Place, of: &OfVariant<'_>) -> Option<Member> {
        if has_serde_skip_serializing(&only.attrs) {
            return None;
        }
        let read = of.read && !parse_serde_key_omission(&only.attrs).skips_deserializing;
        if place.key.is_none() {
            if let Some(fields) = self.flattened(only, &place.prefix, read) {
                return Some(fields);
            }
            if matches!(of.put, Put::Beside) {
                return None;
            }
        }
        self.member(only, place, of.stem, read)
    }

    /// One member per slot of `slots`, at the position serde writes it under `prefix`. A slot
    /// with no path keeps its place as `()`, so a member's position is its slot's.
    fn positional(
        &mut self,
        slots: &FieldsUnnamed,
        prefix: &TokenStream,
        variant: Option<&str>,
        read: bool,
    ) -> Vec<Member> {
        let mut position = 0_usize;
        let mut members: Vec<Member> = Vec::new();
        for (index, slot) in slots.unnamed.iter().enumerate() {
            if has_serde_skip_serializing(&slot.attrs) {
                members.push(Member::skipped());
                continue;
            }
            let place = Place {
                key: Some(position.to_string()),
                prefix: prefix.clone(),
            };
            position = position.saturating_add(1);
            let slot_read = read && !parse_serde_key_omission(&slot.attrs).skips_deserializing;
            let stem = helper_name_stem(&index.to_string(), variant);
            members.push(
                self.member(slot, &place, &stem, slot_read)
                    .unwrap_or_else(Member::skipped),
            );
        }
        members
    }
}

/// How serde writes a value of a type, as far as its path goes.
enum Held<'ty> {
    /// A list of what it holds.
    List(&'ty Type),
    /// A map, whose keys are data.
    Map,
    /// What it holds, or nothing.
    Optional(&'ty Type),
    Value,
}

/// What names the variant of an enum, by the form serde writes the enum in.
enum Named<'tagging> {
    /// A key of the object the enum is written as, or the variant's name as text where it holds
    /// nothing.
    ByItsKey,
    /// The text under this key.
    ByTag(&'tagging str),
}

/// One path, or one struct of paths, as a member of the struct that holds it.
struct Member {
    shown: Shown,
    /// What the type's own function builds it with.
    value: TokenStream,
}

impl Member {
    fn skipped() -> Self {
        Self {
            shown: Shown::Skipped,
            value: quote! { () },
        }
    }
}

/// The members of one struct of paths: under the names of the fields that write them, or by
/// position.
enum Members {
    Named(Vec<(Ident, Member)>),
    Positional(Vec<Member>),
}

impl Members {
    /// What the type's own function writes after the struct's name to build it.
    fn built(&self) -> TokenStream {
        match self {
            Self::Named(members) => {
                let each = members.iter().map(|(name, member)| {
                    let value = &member.value;
                    quote! { #name: #value }
                });
                quote! { { #(#each),* } }
            }
            Self::Positional(members) => {
                let each = members.iter().map(|member| &member.value);
                quote! { (#(#each),*) }
            }
        }
    }

    /// The members as a module `depth` below the type declares them.
    fn declared(&self, own: &Own<'_>, depth: usize) -> TokenStream {
        match self {
            Self::Named(members) => {
                let each = members.iter().map(|(name, member)| {
                    let written = member.shown.written(own, depth);
                    quote! { pub #name: #written }
                });
                quote! { #(#each),* }
            }
            Self::Positional(members) => {
                let each = members.iter().map(|member| {
                    let written = member.shown.written(own, depth);
                    quote! { pub #written }
                });
                quote! { #(#each),* }
            }
        }
    }

    /// Whether none of them is a path.
    fn is_empty(&self) -> bool {
        match self {
            Self::Named(members) => members.is_empty(),
            Self::Positional(members) => members
                .iter()
                .all(|member| matches!(member.shown, Shown::Skipped)),
        }
    }

    fn shown(&self) -> Vec<&Shown> {
        match self {
            Self::Named(members) => members
                .iter()
                .map(|(_name, member)| &member.shown)
                .collect(),
            Self::Positional(members) => members.iter().map(|member| &member.shown).collect(),
        }
    }
}

/// What the emitter reads of the variant a member is written for.
struct OfVariant<'variant> {
    /// The casing the enum gives the fields of every variant that writes none of its own.
    fields_rule: Option<&'variant str>,
    /// The name serde writes the variant under.
    name: &'variant str,
    put: &'variant Put<'variant>,
    /// serde reads a value as the variant.
    read: bool,
    rust_name: &'variant str,
    stem: &'variant str,
}

impl OfVariant<'_> {
    /// What the struct of the variant's own paths is told as.
    fn told(&self) -> String {
        format!(
            "The typed MongoDB paths of what the variant `{}` holds.",
            self.rust_name
        )
    }
}

/// The type paths are written for, as its module and its `impl` name it.
struct Own<'item> {
    generics: &'item Generics,
    module: Ident,
    module_name: String,
    name: &'item Ident,
    /// The type's own type and const parameters, by name.
    parameters: Vec<String>,
    /// The parameter every path carries the row type as, under a name the item does not write.
    root: Ident,
    /// The type's own type parameters, by name.
    type_parameters: Vec<String>,
    /// The parameter `regex` reads the value of a path of text as, named as `root` is.
    value: Ident,
    /// The parameter a type serde writes as one value carries its path as, named as `root` is.
    whole: Ident,
}

impl Own<'_> {
    /// The type's own parameters as arguments written after the row type.
    fn arguments(&self) -> TokenStream {
        self.arguments_among(&self.parameters)
    }

    /// Those of the type's own parameters `kept` names, as arguments written after the row type.
    fn arguments_among(&self, kept: &[String]) -> TokenStream {
        let each = self
            .parameters
            .iter()
            .filter(|parameter| kept.contains(parameter))
            .map(|parameter| Ident::new(parameter, Span::call_site()));
        quote! { #(, #each)* }
    }

    /// The parameters a struct of paths declares a module `depth` below the type: the row type,
    /// then the type's own with their bounds, or those of them `used` names.
    fn declared(&self, depth: usize, used: Option<&[String]>) -> Generics {
        let kept = |named: &String| used.is_none_or(|names| names.contains(named));
        let mut declared = Generics::default();
        let root = &self.root;
        declared.params.push(parse_quote!(#root));
        for parameter in &self.generics.params {
            let named = match parameter {
                GenericParam::Const(constant) => &constant.ident,
                GenericParam::Type(ty) => &ty.ident,
                GenericParam::Lifetime(_) => continue,
            };
            if kept(&named.unraw().to_string()) {
                let scoped = self.scoped(parameter.to_token_stream(), depth);
                declared
                    .params
                    .push(syn::parse2(scoped).unwrap_or_else(|_| parameter.clone()));
            }
        }
        let bounds = self
            .generics
            .where_clause
            .iter()
            .flat_map(|clause| &clause.predicates);
        for predicate in bounds {
            // A bound that names a parameter the struct does not declare is none of its own.
            let names_only_kept = written_names(predicate.to_token_stream())
                .iter()
                .all(|written| !self.parameters.contains(written) || kept(written));
            if names_only_kept {
                let scoped = self.scoped(predicate.to_token_stream(), depth);
                declared.make_where_clause().predicates.push(
                    syn::parse2::<WherePredicate>(scoped).unwrap_or_else(|_| predicate.clone()),
                );
            }
        }
        declared
    }

    /// The type itself, as a module `depth` below it names it.
    fn itself(&self, depth: usize) -> TokenStream {
        let (above, name) = (supers(depth), self.name);
        let (_, type_generics, _) = self.generics.split_for_impl();
        quote! { #above #name #type_generics }
    }

    /// `MongoFields` keeping what a consumer does not read: the path `asked` reads, a marker of
    /// the `marked` parameters nothing else names, and `inner`, the struct of its members, which
    /// it dereferences to. With it, what the type's own function builds it with.
    fn keeping(
        &self,
        inner: Option<&Member>,
        asked: Option<&Asked>,
        marked: &[Ident],
    ) -> (TokenStream, TokenStream) {
        let (module, generics) = (&self.module, self.declared(IN_MODULE, None));
        let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
        let members = inner.map(|member| member.shown.written(self, IN_MODULE));
        let (asked_field, asked_handed, asked_kept) = if asked.is_some() {
            (
                quote! { asked: self::MongoPath, },
                quote! { asked: self::MongoPath, },
                quote! { asked, },
            )
        } else {
            (TokenStream::new(), TokenStream::new(), TokenStream::new())
        };
        let (members_field, members_handed, members_kept) =
            members.as_ref().map_or_else(Default::default, |held| {
                (
                    quote! { members: #held, },
                    quote! { members: #held, },
                    quote! { members, },
                )
            });
        let (marker_field, marker_kept) = if marked.is_empty() {
            (TokenStream::new(), TokenStream::new())
        } else {
            (
                quote! { unused: ::core::marker::PhantomData<fn() -> (#(#marked,)*)>, },
                quote! { unused: ::core::marker::PhantomData, },
            )
        };
        let methods = asked.map(|kept| &kept.methods);
        let reached = members.as_ref().map(|held| {
            quote! {
                impl #impl_generics ::core::ops::Deref for MongoFields #type_generics #where_clause {
                    type Target = #held;

                    fn deref(&self) -> &Self::Target {
                        &self.members
                    }
                }
            }
        });
        let (told, shown) = (
            self.told(),
            shown_as(&Ident::new("MongoFields", Span::call_site()), &generics),
        );
        let declared = quote! {
            #[doc = #told]
            #[non_exhaustive]
            pub struct MongoFields #generics #where_clause {
                #asked_field
                #members_field
                #marker_field
            }

            #shown

            impl #impl_generics MongoFields #type_generics #where_clause {
                /// The struct of paths, from what the type's own function builds for it.
                #[must_use]
                pub const fn built(#asked_handed #members_handed) -> Self {
                    Self { #asked_kept #members_kept #marker_kept }
                }

                #methods
            }

            #reached
        };
        let path = asked.map(|kept| {
            let read = &kept.path;
            quote! { #read, }
        });
        let built = inner.map(|member| &member.value);
        (
            declared,
            quote! { #module::MongoFields::built(#path #built) },
        )
    }

    /// What names `ident` a module `depth` below the type, where it starts a path that goes on
    /// when `leads`. A name alone is a member of the module above, unless it is a keyword, a
    /// primitive, a prelude name or a parameter; a longer path is read as its author wrote it.
    fn renamed(&self, ident: &Ident, leads: bool, depth: usize) -> TokenStream {
        let name = ident.to_string();
        let above = supers(depth);
        if name == "Self" {
            let itself = self.itself(depth);
            return if leads {
                quote! { <#itself> }
            } else {
                itself
            };
        }
        if leads {
            return match name.as_str() {
                "self" => {
                    let nearer = supers(depth.saturating_sub(1));
                    quote! { #nearer super }
                }
                "super" => quote! { #above #ident },
                _ => ident.to_token_stream(),
            };
        }
        let stands = KEYWORDS.contains(&name.as_str())
            || PRIMITIVES.contains(&name.as_str())
            || PRELUDE.contains(&name.as_str())
            || self.parameters.contains(&ident.unraw().to_string());
        if stands {
            ident.to_token_stream()
        } else {
            quote! { #above #ident }
        }
    }

    /// `tokens`, a type or a bound written beside the flagged type, as a module `depth` below it
    /// writes the same.
    fn scoped(&self, tokens: TokenStream, depth: usize) -> TokenStream {
        let mut scoped: Vec<TokenTree> = Vec::new();
        let mut rest = tokens.into_iter().peekable();
        while let Some(token) = rest.next() {
            match token {
                TokenTree::Group(group) => {
                    let mut inner =
                        Group::new(group.delimiter(), self.scoped(group.stream(), depth));
                    inner.set_span(group.span());
                    scoped.push(TokenTree::Group(inner));
                }
                TokenTree::Ident(ident) => {
                    let (before, next) = (scoped.last(), rest.peek());
                    // An associated type is bound where arguments are listed: `<Item = T>`.
                    let bound = matches!(next, Some(TokenTree::Punct(mark)) if mark.as_char() == '=')
                        && matches!(before, Some(TokenTree::Punct(mark)) if matches!(mark.as_char(), '<' | ','));
                    // A macro's name, a lifetime, the name of an associated type being bound, and
                    // every name of a path after its first are read as they stand.
                    let stands = bound
                        || continues_a_path(&scoped)
                        || matches!(before, Some(TokenTree::Punct(tick)) if tick.as_char() == '\'')
                        || matches!(next, Some(TokenTree::Punct(mark)) if mark.as_char() == '!');
                    if stands {
                        scoped.push(TokenTree::Ident(ident));
                    } else {
                        let leads = matches!(
                            next,
                            Some(TokenTree::Punct(mark))
                                if mark.as_char() == ':' && mark.spacing() == Spacing::Joint
                        );
                        scoped.extend(self.renamed(&ident, leads, depth));
                    }
                }
                other @ (TokenTree::Punct(_) | TokenTree::Literal(_)) => scoped.push(other),
            }
        }
        scoped.into_iter().collect()
    }

    /// The type `ty` is, through the wrappers serde writes as the value they hold, where that is
    /// a struct or an enum `#[model_schema]` was written on above, whose module holds a struct of
    /// paths, with whether serde writes it as one value. `None` for any other: the type itself,
    /// one declared below, an alias, a type of another crate.
    fn seen_model<'ty>(&self, ty: &'ty Type) -> Option<Seen<'ty>> {
        let (inner, _) = through_wrappers(ty);
        let Type::Path(named) = inner else {
            return None;
        };
        let last = named.path.segments.last()?;
        let name = last.ident.unraw().to_string();
        let itself = name == "Self" || self.name.unraw() == name;
        let def = get_field_def("", inner, "");
        let sibling = def.array_depth == 0
            && !def.is_optional()
            && matches!(def.field_type, FieldDefType::SiblingType(_, _));
        let reached = named.qself.is_none()
            && !itself
            && sibling
            && !starts_at_a_parameter(named, &self.type_parameters);
        let one_value = match declared(&name) {
            Some(Declared::Model) => false,
            Some(Declared::OneValue) => true,
            Some(Declared::Alias(_)) | None => return None,
        };
        reached.then_some(Seen {
            last,
            one_value,
            ty: inner,
        })
    }

    /// What the struct of paths is told as.
    fn told(&self) -> String {
        format!(
            "The typed MongoDB paths of `{}`, under whatever leads to it in a row of `{}`.",
            self.name.unraw(),
            self.root
        )
    }

    /// The member of a value of type `ty` that serde writes at `place`, by what the type is:
    /// `None` for a map, whose keys are data. `below` says a flagged type is asked for its own
    /// paths, where one is otherwise one whole value.
    ///
    /// A list of a flagged type serde writes as one value is a list of plain values, matched
    /// through its elements. This builds:
    ///
    /// ```rust
    /// # extern crate bson2 as bson;
    /// use serde::{Deserialize, Serialize};
    /// use tixschema::model_schema;
    ///
    /// #[model_schema(decode_with)]
    /// #[derive(Debug, Deserialize, Serialize)]
    /// #[serde(transparent)]
    /// pub struct Score(pub u32);
    ///
    /// #[model_schema(decode_with)]
    /// #[derive(Debug, Deserialize, Serialize)]
    /// pub struct Game {
    ///     pub best: Score,
    ///     pub scores: Vec<Score>,
    /// }
    ///
    /// fn main() -> Result<(), game_schema::WriteError> {
    ///     let f = &Game::MONGO_FIELDS;
    ///     let _ = f.best.gt(Score(80))?;
    ///     let _ = f.scores.elem_match(f.scores.element().gt(Score(5))?);
    ///     Ok(())
    /// }
    /// ```
    ///
    /// The run below is that one with the elements held to a filter over the rows of their own
    /// type, as a list of models takes one, and nothing else changed: such a filter has no key to
    /// write its operator under. A `compile_fail` doctest asserts only that some error was
    /// raised, so it was compiled standalone as an ordinary test file, and the text under it is
    /// the only error it earned, verbatim:
    ///
    /// ```rust,compile_fail
    /// # extern crate bson2 as bson;
    /// use serde::{Deserialize, Serialize};
    /// use tixschema::model_schema;
    ///
    /// #[model_schema(decode_with)]
    /// #[derive(Debug, Deserialize, Serialize)]
    /// #[serde(transparent)]
    /// pub struct Score(pub u32);
    ///
    /// #[model_schema(decode_with)]
    /// #[derive(Debug, Deserialize, Serialize)]
    /// pub struct Game {
    ///     pub best: Score,
    ///     pub scores: Vec<Score>,
    /// }
    ///
    /// fn main() -> Result<(), game_schema::WriteError> {
    ///     let f = &Game::MONGO_FIELDS;
    ///     let _ = f.best.gt(Score(80))?;
    ///     let _ = f.scores.elem_match(Score::MONGO_FIELDS.gt(Score(5))?);
    ///     Ok(())
    /// }
    /// ```
    ///
    /// ```text
    /// error[E0308]: `?` operator has incompatible types
    ///   --> tests/zz_probe.rs:20:33
    ///    |
    /// 20 |     let _ = f.scores.elem_match(Score::MONGO_FIELDS.gt(Score(5))?);
    ///    |                                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `Element<Score>`, found `Filter<Score>`
    ///    |
    ///    = note: `?` operator cannot convert from `score_schema::Filter<Score>` to `game_schema::Element<Score>`
    ///    = note: expected struct `game_schema::Element<Score>`
    ///               found struct `score_schema::Filter<Score>`
    ///
    /// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
    /// ```
    fn typed(&self, ty: &Type, place: &Place, below: bool) -> Option<Member> {
        let (kinds, value, listed) = match held(ty) {
            Held::List(inner) => (["ListField", "ModelList"], inner, true),
            Held::Map => return None,
            Held::Optional(inner) => {
                if matches!(held(inner), Held::Map) {
                    return None;
                }
                (["OptionalField", "OptionalModel"], inner, false)
            }
            Held::Value => (["Field", "Model"], ty, false),
        };
        let module = &self.module;
        let path = place.path(module);
        // The elements of a list of values serde writes whole are plain values: a filter over
        // the paths of one would have no key to write an operator under.
        let model = self
            .seen_model(value)
            .filter(|seen| below && !(listed && seen.one_value));
        let [plain, typed] = kinds.map(|kind| Ident::new(kind, Span::call_site()));
        Some(model.map_or_else(
            || Member {
                shown: Shown::Kind {
                    fields: None,
                    kind: plain.clone(),
                    value: value.to_token_stream(),
                },
                value: quote! { #module::#plain::plain(#path) },
            },
            |seen| {
                let fields = paths_of(&seen, &place.below(module));
                let built = &fields.value;
                Member {
                    value: quote! { #module::#typed::plain(#path, #built) },
                    shown: Shown::Kind {
                        fields: Some(Box::new(fields.shown)),
                        kind: typed.clone(),
                        value: value.to_token_stream(),
                    },
                }
            },
        ))
    }

    /// Those of the type's own parameters the types of `members` name.
    fn used(&self, members: &Members) -> Vec<String> {
        let written: Vec<String> = members
            .shown()
            .into_iter()
            .flat_map(|shown| written_names(shown.written(self, IN_MODULE)))
            .collect();
        self.parameters
            .iter()
            .filter(|parameter| written.contains(parameter))
            .cloned()
            .collect()
    }

    /// `MongoFields` of a type serde writes as one value, and what builds it: the path of that
    /// value, which it dereferences to. `text` adds `$regex`, for a value serde writes as text.
    /// The path is a parameter the type's own `impl` and every holder write, so the module names
    /// no type of its author's and builds where `super` does not reach the type: in a function.
    fn whole(&self, text: bool) -> (TokenStream, TokenStream) {
        let (module, root, whole, value) = (&self.module, &self.root, &self.whole, &self.value);
        let arguments = self.arguments();
        let own = self.declared(IN_MODULE, None);
        let mut generics = own.clone();
        generics.params.push(parse_quote!(#whole));
        let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
        let marked = once(root.clone()).chain(
            self.type_parameters
                .iter()
                .map(|parameter| Ident::new(parameter, Span::call_site())),
        );
        let matched = text.then(|| {
            let mut over_text = own.clone();
            over_text.params.push(parse_quote!(#value));
            let (text_generics, _, _) = over_text.split_for_impl();
            quote! {
                impl #text_generics MongoFields<#root #arguments, self::Field<#root, #value>> #where_clause {
                    /// `$regex` with `$options`: rows whose text at this path matches `pattern`.
                    #[must_use]
                    pub fn regex(&self, pattern: &str, options: &str) -> self::Filter<#root> {
                        self::Field::<#root, ::std::string::String>::plain(self.whole.path).regex(pattern, options)
                    }
                }
            }
        });
        let (told, shown) = (
            self.told(),
            shown_as(&Ident::new("MongoFields", Span::call_site()), &generics),
        );
        let declared = quote! {
            #[doc = #told]
            #[non_exhaustive]
            pub struct MongoFields #generics #where_clause {
                unused: ::core::marker::PhantomData<fn() -> (#(#marked,)*)>,
                whole: #whole,
            }

            #shown

            impl #impl_generics MongoFields #type_generics #where_clause {
                /// The struct of paths, from the path of the one value the type is written as.
                #[must_use]
                pub const fn built(whole: #whole) -> Self {
                    Self { unused: ::core::marker::PhantomData, whole }
                }
            }

            impl #impl_generics ::core::ops::Deref for MongoFields #type_generics #where_clause {
                type Target = #whole;

                fn deref(&self) -> &Self::Target {
                    &self.whole
                }
            }

            #matched
        };
        (
            declared,
            quote! { #module::MongoFields::built(#module::Field::plain(#module::MongoPath::at(prefix))) },
        )
    }
}

/// Where serde writes a value: the keys leading to what holds it, and its own key there, or none
/// for a value written in place of what holds it.
struct Place {
    key: Option<String>,
    /// The keys, as the expression the type's own function reads them from.
    prefix: TokenStream,
}

impl Place {
    /// The keys leading to what the value itself holds.
    fn below(&self, module: &Ident) -> TokenStream {
        if self.key.is_some() {
            let path = self.path(module);
            quote! { #path.segments }
        } else {
            self.prefix.clone()
        }
    }

    /// The path of the value.
    fn path(&self, module: &Ident) -> TokenStream {
        let prefix = &self.prefix;
        self.key.as_ref().map_or_else(
            || quote! { #module::MongoPath::at(#prefix) },
            |key| quote! { #module::MongoPath::under(#prefix, #key) },
        )
    }
}

/// A flagged type a member is typed with, as the type that holds the member writes it.
struct Seen<'ty> {
    /// The last name of its path, which its module is named after.
    last: &'ty PathSegment,
    /// serde writes the type as one value, so its struct of paths is one path.
    one_value: bool,
    ty: &'ty Type,
}

/// Where the form an enum is written in puts what one of its variants holds.
enum Put<'tagging> {
    /// Among the enum's own keys, beside the one that names the variant: serde refuses a value
    /// there that writes no key.
    Beside,
    /// Where the enum itself is: nothing names the variant.
    InPlace,
    /// Under this key of the enum's own.
    Under(&'tagging str),
    /// Under the name serde writes the variant as.
    UnderItsName,
}

/// The type of one member, which each module that declares it writes from where it sits.
enum Shown {
    /// A struct of `mongo_members`: its name, and what it is written with after the row type.
    Held {
        arguments: TokenStream,
        named: Ident,
    },
    /// One of the module's path kinds over a value of the author's: `Field<Root, V>`, and with
    /// the struct of paths of a flagged type `Model<Root, M, F>`.
    Kind {
        fields: Option<Box<Self>>,
        kind: Ident,
        value: TokenStream,
    },
    /// Another flagged type's own struct of paths: its module, what the type is written with,
    /// and for a type serde writes as one value the type itself, which its one path is over.
    Nested {
        arguments: TokenStream,
        module: Ident,
        whole: Option<TokenStream>,
    },
    /// No path: the slot of a tuple that has none.
    Skipped,
}

impl Shown {
    /// The type, as a module `depth` below the flagged type writes it.
    fn written(&self, own: &Own<'_>, depth: usize) -> TokenStream {
        let root = &own.root;
        match self {
            Self::Held { arguments, named } => {
                if depth == IN_MODULE {
                    quote! { self::mongo_members::#named<#root #arguments> }
                } else {
                    quote! { self::#named<#root #arguments> }
                }
            }
            Self::Kind {
                fields,
                kind,
                value,
            } => {
                // The path kinds are the schema module's own, named through a path so that a
                // parameter of the type's own of the same name does not stand in for one.
                let kinds = if depth == IN_MODULE {
                    quote! { self:: }
                } else {
                    supers(depth.saturating_sub(IN_MODULE))
                };
                let of = own.scoped(value.clone(), depth);
                let nested = fields.as_ref().map(|held| {
                    let written = held.written(own, depth);
                    quote! { , #written }
                });
                quote! { #kinds #kind<#root, #of #nested> }
            }
            Self::Nested {
                arguments,
                module,
                whole,
            } => {
                let above = supers(depth);
                let with = own.scoped(arguments.clone(), depth);
                // The path of a type written as one value is a parameter of its struct of paths,
                // which names no type of its author's: whoever holds the struct writes it.
                let path = whole.as_ref().map(|value| {
                    let of = own.scoped(value.clone(), depth);
                    quote! { , #above #module::Field<#root, #of> }
                });
                quote! { #above #module::MongoFields<#root #with #path> }
            }
            Self::Skipped => quote! { () },
        }
    }
}

/// The typed paths of an enum: one member per variant that holds a value serde writes, and one
/// `is_{variant}` per variant where serde writes what names it. A plain enum is one value.
///
/// A member typed with a plain enum takes a value of the enum and nothing else. The run below
/// hands one the text serde writes for a variant. A `compile_fail` doctest asserts only that some
/// error was raised, so it was compiled standalone as an ordinary test file, and the text under
/// it is the only error it earned, verbatim:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "kebab-case")]
/// pub enum InvoiceStatus {
///     Draft,
///     PastDue,
/// }
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub status: InvoiceStatus,
/// }
///
/// fn main() {
///     let _ = Invoice::MONGO_FIELDS.status.eq("past-due");
/// }
/// ```
///
/// ```text
/// error[E0308]: mismatched types
///   --> tests/zz_probe.rs:20:45
///    |
/// 20 |     let _ = Invoice::MONGO_FIELDS.status.eq("past-due");
///    |                                          -- ^^^^^^^^^^ expected `InvoiceStatus`, found `&str`
///    |                                          |
///    |                                          arguments to this method are incorrect
///    |
/// note: method defined here
///   --> tests/zz_probe.rs:13:1
///    |
/// 13 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
pub fn enum_paths(item: &ItemEnum, written: &[String]) -> TypedPaths {
    let mut emitter = Emitter::new(&item.ident, &item.generics, written);
    let tagging = Tagging::of(item);
    let (put, named) = match &tagging {
        Tagging::Adjacent { content, tag } => (Put::Under(content), Some(Named::ByTag(tag))),
        Tagging::External => (Put::UnderItsName, Some(Named::ByItsKey)),
        Tagging::Internal { tag } => (Put::Beside, Some(Named::ByTag(tag))),
        Tagging::Plain => {
            let (declared, built) = emitter.own.whole(false);
            return emitter.emitted(&declared, &built, true);
        }
        Tagging::Untagged => (Put::InPlace, None),
    };
    let container = parse_written_renames(&item.attrs);
    let mut members: Vec<(Ident, Member)> = Vec::new();
    let mut asks: Vec<Ask> = Vec::new();
    for variant in &item.variants {
        if has_serde_skip_serializing(&variant.attrs) {
            continue;
        }
        let rust_name = variant.ident.unraw().to_string();
        let name = parse_written_renames(&variant.attrs)
            .rename
            .unwrap_or_else(|| {
                resolve_rename_rule(container.rename_all.as_deref()).apply_to_variant(&rust_name)
            });
        let taken: Vec<String> = asks.iter().map(|ask| ask.stem.clone()).collect();
        let stem = unclaimed(&to_snake_case(&rust_name), &taken);
        let of = OfVariant {
            fields_rule: container.rename_all_fields.as_deref(),
            name: &name,
            put: &put,
            read: !parse_serde_key_omission(&variant.attrs).skips_deserializing,
            rust_name: &rust_name,
            stem: &stem,
        };
        if let Some(member) = emitter.content(variant, &of) {
            members.push((member_ident(&stem), member));
        }
        asks.push(Ask {
            holds: writes_an_object(variant),
            name,
            stem,
        });
    }
    let asked = named.map(|by| asked(&by, &asks, &emitter.own));
    let (declared, built) = emitter.laid_out(&Members::Named(members), asked.as_ref());
    emitter.emitted(&declared, &built, false)
}

/// The typed paths of a struct: one member per field or slot serde writes, and one value for a
/// struct serde writes as the value it holds, or as nothing.
///
/// A path takes a value of its field's own type, and a field typed with a flagged type declared
/// above reaches that type's paths. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use std::collections::HashMap;
///
/// use bson::doc;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "camelCase")]
/// pub struct Address {
///     pub city: String,
///     pub postal_code: String,
/// }
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "camelCase")]
/// pub struct Customer {
///     pub address: Address,
///     pub open_invoices: u32,
/// }
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "kebab-case")]
/// pub enum InvoiceStatus {
///     Draft,
///     PastDue,
/// }
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "camelCase")]
/// pub struct Invoice {
///     pub customer: Customer,
///     pub details: HashMap<String, String>,
///     pub number: String,
///     #[serde(default, skip_serializing_if = "Option::is_none")]
///     pub paid_at: Option<String>,
///     pub status: InvoiceStatus,
///     pub tags: Vec<String>,
///     pub total: f64,
/// }
///
/// fn main() -> Result<(), invoice_schema::WriteError> {
///     let f = &Invoice::MONGO_FIELDS;
///     let _ = f
///         .status
///         .eq(InvoiceStatus::PastDue)?
///         .or(f.total.gt(1000.0)?)
///         .and(f.tags.contains("export".to_owned())?.or(f.customer.open_invoices.lte(3)?))
///         .or(f.paid_at.exists(true).negated());
///     let _ = f.customer.address.city.eq("Santo Domingo".to_owned())?;
///     let _ = f
///         .number
///         .eq("INV-0042".to_owned())?
///         .and(invoice_schema::Filter::<Invoice>::raw(doc! { "details.costCenter": "CC-7" }));
///     Ok(())
/// }
/// ```
///
/// Each run below declares one such type and asks of a generated path what its kind refuses. A
/// `compile_fail` doctest asserts only that some error was raised, so each was compiled
/// standalone as an ordinary test file, and the text under it is the only error it earned,
/// verbatim.
///
/// `$unset` on a field every row holds:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub number: String,
/// }
///
/// fn main() {
///     let _ = Invoice::MONGO_FIELDS.number.unset();
/// }
/// ```
///
/// ```text
/// error[E0599]: no method named `unset` found for struct `invoice_schema::Field<Root, V>` in the current scope
///   --> tests/zz_probe.rs:12:42
///    |
///  5 | #[model_schema(decode_with)]
///    | ---------------------------- method `unset` not found for this struct
/// ...
/// 12 |     let _ = Invoice::MONGO_FIELDS.number.unset();
///    |                                          ^^^^^
///    |
/// help: there is a method `set` with a similar name, but with different arguments
///   --> tests/zz_probe.rs:5:1
///    |
///  5 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// Text for a `u32` field:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "camelCase")]
/// pub struct Invoice {
///     pub open_invoices: u32,
/// }
///
/// fn main() {
///     let _ = Invoice::MONGO_FIELDS.open_invoices.lte("3");
/// }
/// ```
///
/// ```text
/// error[E0308]: mismatched types
///   --> tests/zz_probe.rs:13:53
///    |
/// 13 |     let _ = Invoice::MONGO_FIELDS.open_invoices.lte("3");
///    |                                                 --- ^^^ expected `u32`, found `&str`
///    |                                                 |
///    |                                                 arguments to this method are incorrect
///    |
/// note: method defined here
///   --> tests/zz_probe.rs:5:1
///    |
///  5 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// Text for an `f64` field:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub total: f64,
/// }
///
/// fn main() {
///     let _ = Invoice::MONGO_FIELDS.total.gt("1000");
/// }
/// ```
///
/// ```text
/// error[E0308]: mismatched types
///   --> tests/zz_probe.rs:12:44
///    |
/// 12 |     let _ = Invoice::MONGO_FIELDS.total.gt("1000");
///    |                                         -- ^^^^^^ expected `f64`, found `&str`
///    |                                         |
///    |                                         arguments to this method are incorrect
///    |
/// note: method defined here
///   --> tests/zz_probe.rs:5:1
///    |
///  5 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// `$gt` on a list, which is matched through its elements. The text under it is verbatim but for
/// one note, which quotes the standard library's own `Iterator` at a path of the machine that
/// compiled it: the second `...` stands for that note.
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub tags: Vec<String>,
/// }
///
/// fn main() -> Result<(), invoice_schema::WriteError> {
///     let _ = Invoice::MONGO_FIELDS.tags.gt("export".to_owned())?;
///     Ok(())
/// }
/// ```
///
/// ```text
/// error[E0599]: `ListField<Invoice, std::string::String>` is not an iterator
///   --> tests/zz_probe.rs:12:40
///    |
///  5 | #[model_schema(decode_with)]
///    | ---------------------------- method `gt` not found for this struct because it doesn't satisfy `ListField<Invoice, std::string::String>: Iterator`
/// ...
/// 12 |     let _ = Invoice::MONGO_FIELDS.tags.gt("export".to_owned())?;
///    |                                        ^^ `ListField<Invoice, std::string::String>` is not an iterator
///    |
///    = note: the following trait bounds were not satisfied:
///            `ListField<Invoice, std::string::String>: Iterator`
///            which is required by `&mut ListField<Invoice, std::string::String>: Iterator`
/// ...
///    = help: items from traits can only be used if the trait is implemented and in scope
///    = note: the following traits define an item `gt`, perhaps you need to implement one of them:
///            candidate #1: `Iterator`
///            candidate #2: `PartialOrd`
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// A filter over the rows of another type, handed to `and`:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub total: f64,
/// }
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Order {
///     pub placed: bool,
/// }
///
/// fn main() -> Result<(), invoice_schema::WriteError> {
///     let invoice = &Invoice::MONGO_FIELDS;
///     let _ = invoice.total.gt(1000.0)?.and(Order::MONGO_FIELDS.placed.eq(true)?);
///     Ok(())
/// }
/// ```
///
/// ```text
/// error[E0277]: the trait bound `order_schema::Filter<Order>: AsRef<PhantomData<Invoice>>` is not satisfied
///   --> tests/zz_probe.rs:19:43
///    |
/// 19 |     let _ = invoice.total.gt(1000.0)?.and(Order::MONGO_FIELDS.placed.eq(true)?);
///    |                                       --- ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ unsatisfied trait bound
///    |                                       |
///    |                                       required by a bound introduced by this call
///    |
/// help: the trait `AsRef<PhantomData<Invoice>>` is not implemented for `order_schema::Filter<Order>`
///       but trait `AsRef<PhantomData<Order>>` is implemented for it
///   --> tests/zz_probe.rs:11:1
///    |
/// 11 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = help: for that trait implementation, expected `Order`, found `Invoice`
/// note: required by a bound in `invoice_schema::Filter::<Root>::and`
///   --> tests/zz_probe.rs:5:1
///    |
///  5 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `Filter::<Root>::and`
///    = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
pub fn struct_paths(item: &ItemStruct, written: &[String]) -> TypedPaths {
    let mut emitter = Emitter::new(&item.ident, &item.generics, written);
    let prefix = quote! { prefix };
    let (declared, built, one_value) = match (value_slot(item), &item.fields) {
        (Some(slot), _) => {
            let (declared, built) = emitter.own.whole(is_text(slot));
            (declared, built, true)
        }
        (None, Fields::Named(named)) => {
            let rule = parse_written_renames(&item.attrs).rename_all;
            let members = emitter.keyed(named, rule.as_deref(), &prefix, None, true);
            let (declared, built) = emitter.laid_out(&Members::Named(members), None);
            (declared, built, false)
        }
        (None, Fields::Unit) => {
            let (declared, built) = emitter.own.whole(false);
            (declared, built, true)
        }
        (None, Fields::Unnamed(slots)) => {
            let members = emitter.positional(slots, &prefix, None, true);
            let (declared, built) = emitter.laid_out(&Members::Positional(members), None);
            (declared, built, false)
        }
    };
    emitter.emitted(&declared, &built, one_value)
}

/// A struct that holds nothing but `members`, each one public, under the parameters `generics`
/// declares, told as `told`, as a module `depth` below the type declares it.
fn all_public(
    named: &Ident,
    members: &Members,
    generics: &Generics,
    told: &str,
    own: &Own<'_>,
    depth: usize,
) -> TokenStream {
    let where_clause = &generics.where_clause;
    let (declared, shown) = (members.declared(own, depth), shown_as(named, generics));
    match members {
        Members::Named(_) => quote! {
            #[doc = #told]
            #[non_exhaustive]
            pub struct #named #generics #where_clause {
                #declared
            }

            #shown
        },
        Members::Positional(_) => quote! {
            #[doc = #told]
            #[non_exhaustive]
            pub struct #named #generics (#declared) #where_clause;

            #shown
        },
    }
}

/// What an enum's struct of paths keeps to answer `is_{variant}`, by what names a variant in the
/// form serde writes the enum in.
fn asked(by: &Named<'_>, asks: &[Ask], own: &Own<'_>) -> Asked {
    let (module, root) = (&own.module, &own.root);
    let method = |ask: &Ask, body: &TokenStream| {
        let named = format_ident!("is_{}", ask.stem);
        let told = format!(
            "Rows whose value at this path is the variant serde writes as `{}`.",
            ask.name
        );
        quote! {
            #[doc = #told]
            #[must_use]
            pub fn #named(&self) -> self::Filter<#root> {
                #body
            }
        }
    };
    match by {
        Named::ByItsKey => {
            let each = asks.iter().map(|ask| {
                let name = &ask.name;
                if ask.holds {
                    method(ask, &quote! { self.keyed(#name) })
                } else {
                    method(
                        ask,
                        &quote! { self::Filter::held(self.asked.key(), "$eq", bson::Bson::String(#name.to_owned())) },
                    )
                }
            });
            let keyed = asks.iter().any(|ask| ask.holds).then(|| {
                quote! {
                    fn keyed(&self, variant: &str) -> self::Filter<#root> {
                        let mut key = self.asked.key();
                        if !key.is_empty() {
                            key.push('.');
                        }
                        key.push_str(variant);
                        self::Filter::held(key, "$exists", bson::Bson::Boolean(true))
                    }
                }
            });
            Asked {
                methods: quote! {
                    #keyed
                    #(#each)*
                },
                path: quote! { #module::MongoPath::at(prefix) },
            }
        }
        Named::ByTag(tag) => {
            let each = asks.iter().map(|ask| {
                let name = &ask.name;
                method(ask, &quote! { self.tagged(#name) })
            });
            let tagged = (!asks.is_empty()).then(|| {
                quote! {
                    fn tagged(&self, variant: &str) -> self::Filter<#root> {
                        self::Filter::held(self.asked.key(), "$eq", bson::Bson::String(variant.to_owned()))
                    }
                }
            });
            Asked {
                methods: quote! {
                    #tagged
                    #(#each)*
                },
                path: quote! { #module::MongoPath::under(prefix, #tag) },
            }
        }
    }
}

/// Whether the tokens end on `::`, so that the name after them continues a path.
fn continues_a_path(tokens: &[TokenTree]) -> bool {
    let [.., TokenTree::Punct(first), TokenTree::Punct(second)] = tokens else {
        return false;
    };
    first.as_char() == ':' && first.spacing() == Spacing::Joint && second.as_char() == ':'
}

/// How serde writes a value of type `ty`, read through the wrappers it writes as the value they
/// hold, as the walker reads the same type.
fn held(ty: &Type) -> Held<'_> {
    let written = written_type(ty);
    if let Type::Array(array) = written {
        return Held::List(&array.elem);
    }
    if let Type::Slice(slice) = written {
        return Held::List(&slice.elem);
    }
    let Type::Path(named) = written else {
        return Held::Value;
    };
    let Some(last) = named.path.segments.last() else {
        return Held::Value;
    };
    let name = last.ident.to_string();
    let mut arguments = type_arguments(&last.arguments);
    match (name.as_str(), arguments.next(), arguments.next()) {
        ("Option", Some(inner), _) => Held::Optional(inner),
        (sequence, Some(inner), _)
            if is_sequence_wrapper(sequence) || is_refused_sequence_wrapper(sequence) =>
        {
            Held::List(inner)
        }
        ("BTreeMap" | "HashMap", _, Some(_)) => Held::Map,
        (wrapper, Some(inner), _) if is_transparent_wrapper(wrapper) => held(inner),
        _ => Held::Value,
    }
}

/// Whether serde writes what `slot` holds as text, with no hook of the slot's writing it.
fn is_text(slot: &Field) -> bool {
    let hooks = parse_serde_field_hooks(&slot.attrs);
    let def = get_field_def("", &slot.ty, "");
    hooks.with.is_none()
        && hooks.serialize_with.is_none()
        && def.array_depth == 0
        && !def.is_optional()
        && matches!(def.field_type, FieldDefType::String)
}

/// `name` as a member's name: written raw where it is a keyword, and with an underscore after it
/// where a keyword has no raw form.
fn member_ident(name: &str) -> Ident {
    if matches!(name, "_" | "crate" | "self" | "super") {
        format_ident!("{name}_")
    } else if syn::parse_str::<Ident>(name).is_ok() {
        Ident::new(name, Span::call_site())
    } else {
        Ident::new_raw(name, Span::call_site())
    }
}

/// The struct of paths of the flagged type `seen`, built by that type's own function under the
/// keys of `prefix`.
///
/// A path kind reaches the nested type's paths by dereferencing, which a `const` item cannot do.
/// A member of the row itself is held in one, and a path below a nested model is read in a
/// function. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Address {
///     pub city: String,
/// }
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Customer {
///     pub address: Address,
/// }
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub customer: Customer,
///     pub total: f64,
/// }
///
/// const TOTAL: invoice_schema::Field<Invoice, f64> = Invoice::MONGO_FIELDS.total;
///
/// fn main() {
///     let city = &Invoice::MONGO_FIELDS.customer.address.city;
///     let _ = (TOTAL.segments(), city.segments());
/// }
/// ```
///
/// The run below is that one with the path below the nested model held in a `const` item too,
/// and nothing else changed. A `compile_fail` doctest asserts only that some error was raised, so
/// it was compiled standalone as an ordinary test file, and the text under it is the only two
/// errors it earned, verbatim:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Address {
///     pub city: String,
/// }
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Customer {
///     pub address: Address,
/// }
///
/// #[model_schema(decode_with)]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub customer: Customer,
///     pub total: f64,
/// }
///
/// const TOTAL: invoice_schema::Field<Invoice, f64> = Invoice::MONGO_FIELDS.total;
/// const CITY: address_schema::Field<Invoice, String> = Invoice::MONGO_FIELDS.customer.address.city;
///
/// fn main() {
///     let _ = (TOTAL.segments(), CITY.segments());
/// }
/// ```
///
/// ```text
/// error[E0015]: cannot perform non-const deref coercion on `invoice_schema::Model<Invoice, Customer, customer_schema::MongoFields<Invoice>>` in constants
///   --> tests/zz_probe.rs:25:54
///    |
/// 25 | const CITY: address_schema::Field<Invoice, String> = Invoice::MONGO_FIELDS.customer.address.city;
///    |                                                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    |
///    = note: attempting to deref into `customer_schema::MongoFields<Invoice>`
/// note: deref defined here
///   --> tests/zz_probe.rs:17:1
///    |
/// 17 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
/// note: impl defined here, but it is not `const`
///   --> tests/zz_probe.rs:17:1
///    |
/// 17 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = note: calls in constants are limited to constant functions, tuple structs and tuple variants
///    = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error[E0015]: cannot perform non-const deref coercion on `customer_schema::Model<Invoice, Address, address_schema::MongoFields<Invoice>>` in constants
///   --> tests/zz_probe.rs:25:54
///    |
/// 25 | const CITY: address_schema::Field<Invoice, String> = Invoice::MONGO_FIELDS.customer.address.city;
///    |                                                      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    |
///    = note: attempting to deref into `address_schema::MongoFields<Invoice>`
/// note: deref defined here
///   --> tests/zz_probe.rs:11:1
///    |
/// 11 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
/// note: impl defined here, but it is not `const`
///   --> tests/zz_probe.rs:11:1
///    |
/// 11 | #[model_schema(decode_with)]
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = note: calls in constants are limited to constant functions, tuple structs and tuple variants
///    = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 2 previous errors
/// ```
fn paths_of(seen: &Seen<'_>, prefix: &TokenStream) -> Member {
    let arguments = if let PathArguments::AngleBracketed(angled) = &seen.last.arguments {
        let each = angled.args.iter();
        quote! { #(, #each)* }
    } else {
        TokenStream::new()
    };
    let module = ident_schema_module_name(&seen.last.ident.unraw().to_string());
    let model = seen.ty;
    Member {
        shown: Shown::Nested {
            arguments,
            // Spanned on the type's name, so a module that is not in scope beside the type that
            // holds the member is reported at the member's own field.
            module: Ident::new(&module, seen.last.ident.span()),
            whole: seen.one_value.then(|| model.to_token_stream()),
        },
        value: quote! { <#model>::mongo_fields_under(#prefix) },
    }
}

/// `Debug` for the struct of paths `named`, declared under `generics`: its name alone. A derive
/// would ask `Debug` of every type its author wrote a field with, which the flag does not.
fn shown_as(named: &Ident, generics: &Generics) -> TokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let name = named.unraw().to_string();
    quote! {
        impl #impl_generics ::core::fmt::Debug for #named #type_generics #where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(#name).finish_non_exhaustive()
            }
        }
    }
}

/// `super::`, once per module between where a name is written and the type's own scope.
fn supers(depth: usize) -> TokenStream {
    let each = repeat_with(|| quote! { super:: }).take(depth);
    quote! { #(#each)* }
}

/// `ty` read through the wrappers serde writes as the value they hold, and whether it had any.
fn through_wrappers(ty: &Type) -> (&Type, bool) {
    let mut inner = written_type(ty);
    let mut wrapped = false;
    loop {
        let Type::Path(named) = inner else {
            return (inner, wrapped);
        };
        let Some(last) = named.path.segments.last() else {
            return (inner, wrapped);
        };
        let held_by_wrapper = if is_transparent_wrapper(&last.ident.to_string()) {
            type_arguments(&last.arguments).next()
        } else {
            None
        };
        let Some(held_type) = held_by_wrapper else {
            return (inner, wrapped);
        };
        inner = written_type(held_type);
        wrapped = true;
    }
}

/// `base`, numbered where `taken` already holds it.
fn unclaimed(base: &str, taken: &[String]) -> String {
    let mut name = base.to_owned();
    let mut number = 1_u32;
    while taken.contains(&name) {
        number = number.saturating_add(1);
        name = format!("{base}{number}");
    }
    name
}

/// Whether serde writes `variant` as an object under its name. One that holds nothing is written
/// as its name alone, and so is one whose single slot serde never writes.
fn writes_an_object(variant: &Variant) -> bool {
    match &variant.fields {
        Fields::Named(_) => true,
        Fields::Unit => false,
        Fields::Unnamed(slots) => {
            let mut written = slots.unnamed.iter();
            !matches!(
                (written.next(), written.next()),
                (Some(only), None) if has_serde_skip_serializing(&only.attrs)
            )
        }
    }
}
