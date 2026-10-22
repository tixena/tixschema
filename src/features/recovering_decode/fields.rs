//! The typed MongoDB paths a type that lists `"mongodb"` in `enable` gets under the `mongodb`
//! feature: `MongoFields`, `MONGO_FIELDS` and one function per field. A type that lists
//! `"decode_with"` alone gets none.

use core::iter::{once, repeat_with};

use proc_macro2::{Group, Ident, Spacing, Span, TokenStream, TokenTree};
use quote::{ToTokens as _, format_ident, quote};
use syn::ext::IdentExt as _;
use syn::punctuated::Punctuated;
use syn::{
    Field, Fields, FieldsNamed, FieldsUnnamed, GenericParam, Generics, ItemEnum, ItemStruct, Token,
    Type, TypeParamBound, Variant, WherePredicate, parse_quote,
};

use super::aliases::PRIMITIVES;
use super::enums::{TagArm, WrittenVariant, tag_arms, written_variants};
use super::{
    Step, TypedPaths, bound_parameters, hooked_leaf, starts_at_a_parameter, type_arguments,
    unclaimed, unclaimed_parameter, value_slot, written_names,
};
use crate::features::serde::{
    Tagging, has_serde_skip_serializing, parse_serde_field_attributes, parse_serde_field_hooks,
    parse_serde_key_omission, parse_written_renames,
};
use crate::field_type::{
    FieldDefType, get_field_def, is_map_wrapper, is_refused_sequence_wrapper, is_sequence_wrapper,
    is_transparent_wrapper,
};
use crate::model_schema::{helper_name_stem, written_spelling};
use crate::rename_rule::resolve_rename_rule;
use crate::utils::{Declared, declared, ident_schema_module_name, written_type};

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

/// What builds one member's path, which each scope that builds one writes from where it sits.
enum Built {
    /// A struct of `mongo_members`: one that holds no path where `slots` is `None`, and the
    /// tuple of what builds each slot otherwise.
    Held {
        named: Ident,
        slots: Option<Vec<Self>>,
    },
    /// A path kind at `key`, whose values the function `writer` of the type's own hands to the
    /// field's hook.
    Hooked {
        key: String,
        kind: Ident,
        writer: Ident,
    },
    /// `Nested` at `key`.
    Nested { key: String },
    /// A path kind at `key`, whose values their type's own `Serialize` writes.
    Plain { key: String, kind: Ident },
    /// `()`: the slot of a tuple that has no path.
    Skipped,
}

impl Built {
    /// The expression, as the type's own scope writes it.
    fn written(&self, own: &Own<'_>) -> TokenStream {
        let kinds = own.kinds(0);
        let path = |key: &String| {
            if key.is_empty() {
                quote! { #kinds MongoPath::ROOT }
            } else {
                quote! { #kinds MongoPath::at(#key) }
            }
        };
        match self {
            Self::Held { named, slots: None } => {
                quote! { #kinds mongo_members::#named { row: ::core::marker::PhantomData } }
            }
            Self::Held {
                named,
                slots: Some(slots),
            } => {
                let each = slots.iter().map(|slot| slot.written(own));
                quote! { #kinds mongo_members::#named(#(#each),*) }
            }
            Self::Hooked { key, kind, writer } => {
                let (at, itself) = (path(key), own.itself(0));
                quote! { #kinds #kind::hooked(#at, <#itself>::#writer) }
            }
            Self::Nested { key } => {
                let at = path(key);
                quote! { #kinds Nested::at(#at) }
            }
            Self::Plain { key, kind } => {
                let at = path(key);
                quote! { #kinds #kind::plain(#at) }
            }
            Self::Skipped => quote! { () },
        }
    }
}

/// The paths of one type, as they are gathered: what is written beside the type, what
/// `mongo_members` holds under the names each struct takes there, and what the type's `impl`
/// gains beside the const that holds them.
struct Emitter<'item> {
    beside: Vec<TokenStream>,
    held: Vec<(Vec<String>, TokenStream)>,
    own: Own<'item>,
    writers: Vec<(String, TokenStream)>,
    /// Every name the type's item writes.
    written: &'item [String],
}

impl<'item> Emitter<'item> {
    /// The arms of an enum serde writes under `tag`: one function per arm of its struct of paths,
    /// for the `impl` beside the type, where the model an arm holds is named as its author wrote
    /// it, and `Arm` for its schema module. An arm whose function would be no identifier has
    /// neither.
    fn arms(&self, tag: &str, arms: &[TagArm<'_>], put: &Put<'_>) -> (TokenStream, TokenStream) {
        let own = &self.own;
        let (kinds, root) = (own.kinds(0), &own.root);
        let place = if let Put::Under(content) = put {
            quote! { #kinds MongoPath::at(#content) }
        } else {
            quote! { #kinds MongoPath::ROOT }
        };
        let named: Vec<(Ident, &TagArm<'_>)> = arms
            .iter()
            .filter_map(|arm| Some((function_ident(&arm.function)?, arm)))
            .collect();
        if named.is_empty() {
            return (TokenStream::new(), TokenStream::new());
        }
        let functions = named.iter().map(|(function, arm)| {
            let value = &arm.value;
            let held = arm_model(arm.variant).map(|ty| own.naming_itself(ty.to_token_stream()));
            let asked = held.as_ref().map_or_else(
                || "the tag alone".to_owned(),
                |model| {
                    format!(
                        "the tag is asked, and `and` takes conditions on the `{}` it holds",
                        written_spelling(model)
                    )
                },
            );
            let told = format!("Rows of the arm serde writes as `{value}`: {asked}.");
            let model = held.unwrap_or_else(|| quote! { () });
            quote! {
                #[doc = #told]
                #[must_use]
                pub const fn #function(&self) -> #kinds ArmFilter<#root, #model> {
                    #kinds ArmFilter::of(#tag, #value, #place)
                }
            }
        });
        let told = format!(
            "The arms of `{}`, one per variant serde writes.",
            own.name.unraw()
        );
        let each = named.iter().map(|(_function, arm)| {
            let mut name = arm.variant.ident.clone();
            name.set_span(Span::call_site());
            name
        });
        (
            quote! { #(#functions)* },
            quote! {
                #[doc = #told]
                #[derive(Clone, Copy, Debug, Eq, PartialEq)]
                #[non_exhaustive]
                pub enum Arm {
                    #(#each,)*
                }
            },
        )
    }

    /// What one variant holds, where serde writes it, as one member. `None` for a variant with no
    /// path of its own.
    fn content(&mut self, variant: &Variant, of: &OfVariant<'_>) -> Option<Member> {
        let place = Place {
            key: match of.put {
                Put::Beside | Put::InPlace => None,
                Put::Under(content) => Some((*content).to_owned()),
                Put::UnderItsName => Some(of.name.to_owned()),
            },
            under: String::new(),
        };
        let below = place.joined();
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
                self.held(of.rust_name, &of.told(), Members::Named(members))
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
                self.held(of.rust_name, &of.told(), Members::Positional(members))
            }
        }
    }

    /// What the paths add: `declared` beside the structs `mongo_members` holds, and on the type
    /// the const `built` is the value of and the functions hooked paths write through.
    /// `one_value` says the struct of paths is one path, whose type the type's `impl` writes.
    fn emitted(self, declared: &TokenStream, built: &TokenStream, one_value: bool) -> TypedPaths {
        let (module, arguments) = (&self.own.module, self.own.arguments());
        let as_the_row = if one_value {
            quote! { , #module::Field<Self, Self> }
        } else {
            TokenStream::new()
        };
        let members = (!self.held.is_empty()).then(|| {
            let held = self.held.iter().map(|(_claimed, written)| written);
            quote! {
                /// The structs of paths a function of `MongoFields` answers: one per variant, and
                /// per tuple, with paths of its own.
                pub mod mongo_members {
                    use super::*;

                    #(#held)*
                }
            }
        });
        let (beside, writers) = (
            &self.beside,
            self.writers.iter().map(|(_named, written)| written),
        );
        TypedPaths {
            beside: quote! { #(#beside)* },
            module_items: quote! {
                #declared
                #members
            },
            type_items: quote! {
                /// The typed MongoDB paths of this type, as the row a filter or an update is over.
                pub const MONGO_FIELDS: #module::MongoFields<Self #arguments #as_the_row> = #built;

                #(#writers)*
            },
        }
    }

    /// What a flattened field holds: the type it is typed with writes its keys among those of
    /// what flattens it, at `under`, and `nest` reaches them there. `None` for any field but one
    /// serde reads as a struct or an enum `#[model_schema]` was written on above, or as an
    /// `Option` of one, and for one serde writes as one value, which has no key to put there.
    fn flattened(&self, field: &Field, under: &str, read: bool) -> Option<Member> {
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
        (!model.one_value).then(|| {
            let place = Place {
                key: None,
                under: under.to_owned(),
            };
            Member {
                built: Built::Nested {
                    key: place.joined(),
                },
                shown: Shown::Nested {
                    model: model.ty.to_token_stream(),
                },
                told: format!(
                    "What the model serde writes {} holds: `nest` reaches its paths.",
                    place.told()
                ),
            }
        })
    }

    /// A struct of `members` in `mongo_members`, named after `base` and told as `told`, as the
    /// one member that answers it. `None` where no member is a path.
    fn held(&mut self, base: &str, told: &str, members: Members) -> Option<Member> {
        if members.is_empty() {
            return None;
        }
        let own = &self.own;
        // A struct named as a prelude type or as a parameter would be read in its place by every
        // member beside it that names one.
        let mut taken: Vec<String> = self
            .held
            .iter()
            .flat_map(|(named, _)| named.iter().cloned())
            .chain(PRELUDE.map(str::to_owned))
            .chain(own.parameters.iter().cloned())
            .collect();
        let name = Ident::new(&unclaimed(base, &taken), Span::call_site());
        let mut claimed = vec![name.to_string()];
        let (arguments, written, slots, slotted) = match members {
            // A function may name the type itself, to reach a hook: every parameter is declared.
            Members::Named(functions) => {
                let (written, beside) =
                    of_functions(&name, &functions, told, own, None, IN_MEMBERS);
                self.beside.extend(beside);
                (own.arguments(), written, None, None)
            }
            Members::Positional(slots) => {
                // The trait takes no name the item writes, so no variant's struct moves for it.
                taken.extend(claimed.iter().chain(self.written).cloned());
                let slotted = Ident::new(
                    &unclaimed(&format!("{name}Slots"), &taken),
                    Span::call_site(),
                );
                claimed.push(slotted.to_string());
                let used = own.used(&slots);
                let (written, beside) =
                    of_slots(&name, &slotted, &slots, Some(&used), told, own, IN_MEMBERS);
                self.beside.push(beside);
                let built = slots.into_iter().map(|slot| slot.built).collect();
                (
                    own.arguments_among(&used),
                    written,
                    Some(built),
                    Some(slotted),
                )
            }
        };
        self.held.push((claimed, written));
        Some(Member {
            built: Built::Held {
                named: name.clone(),
                slots,
            },
            shown: Shown::Held {
                arguments,
                named: name,
                slotted,
            },
            told: told.to_owned(),
        })
    }

    /// The path of a field written through a hook of its own: the value is handed to the hook as
    /// the field holds it, which under an `Option` is `Some`.
    fn hooked(&mut self, ty: &Type, place: &Place, stem: &str, write: &TokenStream) -> Member {
        let module = &self.own.module;
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
        let writer = Ident::new(&named, Span::call_site());
        let told = format!("What `{stem}` holds, as its own serde hook writes it.");
        self.writers.push((
            named,
            quote! {
                #[doc = #told]
                fn #writer(value: #value) -> ::core::result::Result<bson::Bson, #module::WriteError> {
                    #write(#handed, #module::BsonIntegers(bson::Serializer::new()))
                }
            },
        ));
        Member {
            built: Built::Hooked {
                key: place.joined(),
                kind: Ident::new(of_kind, Span::call_site()),
                writer,
            },
            shown: Shown::Kind {
                kind: Ident::new(of_kind, Span::call_site()),
                nested: reached_through(value),
                value: value.to_token_stream(),
            },
            told: format!("The path of {}.", place.told()),
        }
    }

    /// One member per field of `named` that serde writes, under the field's own name: the key
    /// serde writes it at under `under`, cased by `rename_all` where the field names none, or
    /// what a flattened one holds at `under` itself.
    fn keyed(
        &mut self,
        named: &FieldsNamed,
        rename_all: Option<&str>,
        under: &str,
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
                self.flattened(field, under, field_read)
            } else {
                let renamed = parse_written_renames(&field.attrs).rename;
                let place = Place {
                    key: Some(renamed.unwrap_or_else(|| {
                        resolve_rename_rule(rename_all).apply_to_field(&written)
                    })),
                    under: under.to_owned(),
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

    /// `MongoFields` for `members`, with the `is_{variant}` methods an enum asks by, and what the
    /// type's own const holds as it. Named members are functions of a struct that holds no path.
    /// The slots of a tuple are public members; a tuple with a parameter no slot names keeps its
    /// marker beside them, in a struct of `mongo_members` it dereferences to.
    fn laid_out(
        &mut self,
        members: Members,
        asked: Option<&TokenStream>,
    ) -> (TokenStream, TokenStream) {
        let named = Ident::new("MongoFields", Span::call_site());
        let slots = match members {
            Members::Named(functions) => {
                let own = &self.own;
                let module = &own.module;
                let told = format!("{} It holds no path.", own.told());
                let (declared, beside) =
                    of_functions(&named, &functions, &told, own, asked, IN_MODULE);
                self.beside.extend(beside);
                return (
                    declared,
                    quote! { #module::MongoFields { row: ::core::marker::PhantomData } },
                );
            }
            Members::Positional(slots) => slots,
        };
        let used = self.own.used(&slots);
        let unused: Vec<Ident> = self
            .own
            .type_parameters
            .iter()
            .filter(|parameter| !used.contains(parameter))
            .map(|parameter| Ident::new(parameter, Span::call_site()))
            .collect();
        let none = slots
            .iter()
            .all(|slot| matches!(slot.shown, Shown::Skipped));
        if unused.is_empty() && !none {
            let own = &self.own;
            let (module, slotted) = (&own.module, Ident::new("MongoSlots", Span::call_site()));
            let each = slots.iter().map(|slot| slot.built.written(own));
            let (declared, beside) =
                of_slots(&named, &slotted, &slots, None, &own.told(), own, IN_MODULE);
            self.beside.push(beside);
            return (declared, quote! { #module::MongoFields(#(#each),*) });
        }
        let told = format!(
            "The paths `MongoFields` holds for `{}`, which it dereferences to.",
            self.own.name.unraw()
        );
        let inner = self.held("Members", &told, Members::Positional(slots));
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
        self.own.keeping(inner.as_ref(), &marked)
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
        // A type is a model only where its walker is: a hook reads the value whole, and a value
        // serde never reads back is walked by nothing.
        self.own.typed(&field.ty, place, read && hooked.is_none())
    }

    fn new(name: &'item Ident, generics: &'item Generics, written: &'item [String]) -> Self {
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
            beside: Vec::new(),
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
            written,
        }
    }

    /// What a variant's one slot holds. Under a key of its own it is a member like any field.
    /// With none, the keys of a flagged type serde writes under keys sit at the enum's own
    /// level, where `nest` reaches them, and any other value is the enum's own where serde
    /// writes it bare.
    fn only_slot(&mut self, only: &Field, place: &Place, of: &OfVariant<'_>) -> Option<Member> {
        if has_serde_skip_serializing(&only.attrs) {
            return None;
        }
        let read = of.read && !parse_serde_key_omission(&only.attrs).skips_deserializing;
        if place.key.is_none() {
            if let Some(fields) = self.flattened(only, &place.under, read) {
                return Some(fields);
            }
            if matches!(of.put, Put::Beside) {
                return None;
            }
        }
        self.member(only, place, of.stem, read)
    }

    /// One member per slot of `slots`, at the position serde writes it under `under`. A slot
    /// with no path keeps its place as `()`, so a member's position is its slot's.
    fn positional(
        &mut self,
        slots: &FieldsUnnamed,
        under: &str,
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
                under: under.to_owned(),
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
    List(&'ty Type),
    Map,
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

/// One path, or one struct of paths, as a member of the struct that answers it.
struct Member {
    built: Built,
    shown: Shown,
    /// What the function that answers it is told as.
    told: String,
}

impl Member {
    const fn skipped() -> Self {
        Self {
            built: Built::Skipped,
            shown: Shown::Skipped,
            told: String::new(),
        }
    }
}

/// The members of one struct of paths: each under the name of the function that answers it,
/// or by position.
enum Members {
    Named(Vec<(Ident, Member)>),
    Positional(Vec<Member>),
}

impl Members {
    /// Whether none of them is a path.
    fn is_empty(&self) -> bool {
        match self {
            Self::Named(members) => members.is_empty(),
            Self::Positional(members) => members
                .iter()
                .all(|member| matches!(member.shown, Shown::Skipped)),
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

    /// The parameters a struct of paths declares in a module below the type: the row type, then
    /// the type's own, or those of them `used` names, with no bound but `?Sized`. A trait of the
    /// author's is not reached from there where the type is declared in a function.
    fn bare(&self, used: Option<&[String]>) -> Generics {
        let mut declared = Generics::default();
        let root = &self.root;
        declared.params.push(parse_quote!(#root));
        for parameter in &self.generics.params {
            let (named, bare): (&Ident, GenericParam) = match parameter {
                GenericParam::Const(constant) => {
                    let (ident, ty) = (&constant.ident, &constant.ty);
                    (ident, parse_quote!(const #ident: #ty))
                }
                GenericParam::Type(ty) => {
                    let ident = &ty.ident;
                    let unsized_bound = ty.bounds.iter().any(|bound| {
                        matches!(bound, TypeParamBound::Trait(trait_bound) if trait_bound.maybe.is_some())
                    });
                    let bare = if unsized_bound {
                        parse_quote!(#ident: ?::core::marker::Sized)
                    } else {
                        parse_quote!(#ident)
                    };
                    (ident, bare)
                }
                GenericParam::Lifetime(_) => continue,
            };
            if used.is_none_or(|names| names.contains(&named.unraw().to_string())) {
                declared.params.push(bare);
            }
        }
        declared
    }

    /// The parameters an `impl` beside the type declares: the row type, then the type's own with
    /// the bounds their author wrote, or those of them `used` names, under every bound that names
    /// no parameter it leaves out.
    fn declared(&self, used: Option<&[String]>) -> Generics {
        let kept = |named: &String| used.is_none_or(|names| names.contains(named));
        // A bound that names a parameter the `impl` does not declare is none of its own.
        let names_only_kept = |tokens: TokenStream| {
            written_names(tokens)
                .iter()
                .all(|written| !self.parameters.contains(written) || kept(written))
        };
        let kept_bounds = |bounds: &Punctuated<TypeParamBound, Token![+]>| {
            bounds
                .iter()
                .filter(|bound| names_only_kept(bound.to_token_stream()))
                .cloned()
                .collect::<Punctuated<_, _>>()
        };
        let mut declared = Generics::default();
        let root = &self.root;
        declared.params.push(parse_quote!(#root));
        for parameter in &self.generics.params {
            let (named, written) = match parameter {
                GenericParam::Const(constant) => (&constant.ident, parameter.clone()),
                GenericParam::Type(ty) => {
                    let mut held = ty.clone();
                    held.bounds = kept_bounds(&ty.bounds);
                    if held.bounds.is_empty() {
                        held.colon_token = None;
                    }
                    (&ty.ident, GenericParam::Type(held))
                }
                GenericParam::Lifetime(_) => continue,
            };
            if kept(&named.unraw().to_string()) {
                let scoped = self.naming_itself(written.to_token_stream());
                declared.params.push(syn::parse2(scoped).unwrap_or(written));
            }
        }
        let bounds = self
            .generics
            .where_clause
            .iter()
            .flat_map(|clause| &clause.predicates);
        for predicate in bounds {
            // A predicate on a type that names a left-out parameter cannot be written at all, and
            // one whose every bound names such a parameter is left with nothing to declare.
            let kept_predicate = if let WherePredicate::Type(typed) = predicate {
                let (lifetimes, bounded_ty) = (&typed.lifetimes, &typed.bounded_ty);
                let mut held = typed.clone();
                held.bounds = kept_bounds(&typed.bounds);
                let emptied = held.bounds.is_empty() && !typed.bounds.is_empty();
                (names_only_kept(quote! { #lifetimes #bounded_ty }) && !emptied)
                    .then_some(WherePredicate::Type(held))
            } else {
                names_only_kept(predicate.to_token_stream()).then(|| predicate.clone())
            };
            if let Some(written) = kept_predicate {
                let scoped = self.naming_itself(written.to_token_stream());
                declared
                    .make_where_clause()
                    .predicates
                    .push(syn::parse2::<WherePredicate>(scoped).unwrap_or(written));
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

    /// `MongoFields` of a tuple keeping what a consumer does not read: a marker of the `marked`
    /// parameters no slot names, beside `inner`, the struct of its slots, which it dereferences
    /// to. With it, what the type's own const holds as it.
    fn keeping(&self, inner: Option<&Member>, marked: &[Ident]) -> (TokenStream, TokenStream) {
        let (module, mut generics) = (&self.module, self.bare(None));
        if let Some(Shown::Held {
            arguments,
            named: _named,
            slotted: Some(slotted),
        }) = inner.map(|member| &member.shown)
        {
            let root = &self.root;
            generics
                .make_where_clause()
                .predicates
                .push(parse_quote!((): self::mongo_members::#slotted<#root #arguments>));
        }
        let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
        let members = inner.map(|member| member.shown.written(self, IN_MODULE));
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
                #members_field
                #marker_field
            }

            #shown

            impl #impl_generics MongoFields #type_generics #where_clause {
                /// The struct of paths, from what the type's own const builds for it.
                #[must_use]
                pub const fn built(#members_handed) -> Self {
                    Self { #members_kept #marker_kept }
                }
            }

            #reached
        };
        let built = inner.map(|member| member.built.written(self));
        (declared, quote! { #module::MongoFields::built(#built) })
    }

    /// What names one of the schema module's own items a module `depth` below the type, and
    /// beside the type at no depth: through a path, so that a parameter of the type's own of the
    /// same name does not stand in for one.
    fn kinds(&self, depth: usize) -> TokenStream {
        if depth == 0 {
            let module = &self.module;
            quote! { #module:: }
        } else if depth == IN_MODULE {
            quote! { self:: }
        } else {
            supers(depth.saturating_sub(IN_MODULE))
        }
    }

    /// `tokens`, a type written beside the flagged type, with `Self` written as the type itself:
    /// what an `impl` of another type beside it reads.
    fn naming_itself(&self, tokens: TokenStream) -> TokenStream {
        let mut named: Vec<TokenTree> = Vec::new();
        let mut rest = tokens.into_iter().peekable();
        while let Some(token) = rest.next() {
            match token {
                TokenTree::Group(group) => {
                    let mut inner =
                        Group::new(group.delimiter(), self.naming_itself(group.stream()));
                    inner.set_span(group.span());
                    named.push(TokenTree::Group(inner));
                }
                TokenTree::Ident(ident) if ident == "Self" => {
                    let leads = matches!(
                        rest.peek(),
                        Some(TokenTree::Punct(mark))
                            if mark.as_char() == ':' && mark.spacing() == Spacing::Joint
                    );
                    named.extend(self.renamed(&ident, leads, 0));
                }
                other @ (TokenTree::Ident(_) | TokenTree::Punct(_) | TokenTree::Literal(_)) => {
                    named.push(other);
                }
            }
        }
        named.into_iter().collect()
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
    /// writes the same. At no depth they stand but for `Self`, which an `impl` of another type
    /// beside the flagged one would read as that other type.
    fn scoped(&self, tokens: TokenStream, depth: usize) -> TokenStream {
        if depth == 0 {
            return self.naming_itself(tokens);
        }
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
    /// a struct or an enum `#[model_schema]` was written on above, with whether serde writes it
    /// as one value. `None` for any other: the type itself, one declared below, an alias, a type
    /// of another crate.
    fn seen_model<'ty>(&self, ty: &'ty Type) -> Option<Seen<'ty>> {
        let (inner, _) = through_wrappers(ty);
        let Type::Path(named) = inner else {
            return None;
        };
        let name = named.path.segments.last()?.ident.unraw().to_string();
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
            one_value,
            ty: inner,
        })
    }

    /// What the struct of paths is told as.
    fn told(&self) -> String {
        format!(
            "The typed MongoDB paths of `{}`, each under no key but the type's own.",
            self.name.unraw()
        )
    }

    /// The member of a value of type `ty` that serde writes at `place`, by what the type is:
    /// `None` for a map, whose keys are data. `below` says a flagged type serde writes under
    /// keys is a model, where one is otherwise one whole value.
    ///
    /// A list of a flagged type serde writes as one value is a list of plain values, matched
    /// through its elements. This builds:
    ///
    /// ```rust
    /// # extern crate bson2 as bson;
    /// use serde::{Deserialize, Serialize};
    /// use tixschema::model_schema;
    ///
    /// tixschema::decode_with_foundation!();
    /// tixschema::mongodb_foundation!();
    ///
    /// #[model_schema(enable = ["decode_with", "mongodb"])]
    /// #[derive(Debug, Deserialize, Serialize)]
    /// #[serde(transparent)]
    /// pub struct Score(pub u32);
    ///
    /// #[model_schema(enable = ["decode_with", "mongodb"])]
    /// #[derive(Debug, Deserialize, Serialize)]
    /// pub struct Game {
    ///     pub best: Score,
    ///     pub scores: Vec<Score>,
    /// }
    ///
    /// fn main() -> Result<(), game_schema::WriteError> {
    ///     let f = Game::MONGO_FIELDS;
    ///     let _ = f.best().gt(Score(80))?;
    ///     let _ = f.scores().elem_match(f.scores().element().gt(Score(5))?);
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
    /// tixschema::decode_with_foundation!();
    /// tixschema::mongodb_foundation!();
    ///
    /// #[model_schema(enable = ["decode_with", "mongodb"])]
    /// #[derive(Debug, Deserialize, Serialize)]
    /// #[serde(transparent)]
    /// pub struct Score(pub u32);
    ///
    /// #[model_schema(enable = ["decode_with", "mongodb"])]
    /// #[derive(Debug, Deserialize, Serialize)]
    /// pub struct Game {
    ///     pub best: Score,
    ///     pub scores: Vec<Score>,
    /// }
    ///
    /// fn main() -> Result<(), game_schema::WriteError> {
    ///     let f = Game::MONGO_FIELDS;
    ///     let _ = f.best().gt(Score(80))?;
    ///     let _ = f.scores().elem_match(Score::MONGO_FIELDS.gt(Score(5))?);
    ///     Ok(())
    /// }
    /// ```
    ///
    /// ```text
    /// error[E0308]: `?` operator has incompatible types
    ///   --> tests/zz_probe.rs:23:35
    ///    |
    /// 23 |     let _ = f.scores().elem_match(Score::MONGO_FIELDS.gt(Score(5))?);
    ///    |                                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `Element<Score>`, found `Filter<Score>`
    ///    |
    ///    = note: `?` operator cannot convert from `__tixschema_mongodb::Filter<Score>` to `__tixschema_mongodb::Element<Score>`
    ///    = note: expected struct `__tixschema_mongodb::Element<Score>`
    ///               found struct `__tixschema_mongodb::Filter<Score>`
    ///
    /// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
    /// ```
    fn typed(&self, ty: &Type, place: &Place, below: bool) -> Option<Member> {
        let (kinds, value) = taken(ty)?;
        // A value serde writes whole is a plain one, with every comparison its own: a filter
        // over the paths of one has no key to write an operator under.
        let model = self
            .seen_model(value)
            .is_some_and(|seen| below && !seen.one_value);
        let [plain, typed] = kinds;
        let kind = Ident::new(if model { typed } else { plain }, Span::call_site());
        Some(Member {
            built: Built::Plain {
                key: place.joined(),
                kind: kind.clone(),
            },
            shown: Shown::Kind {
                kind,
                nested: reached_through(value),
                value: value.to_token_stream(),
            },
            told: format!("The path of {}.", place.told()),
        })
    }

    /// Those of the type's own parameters the types of `members` name.
    fn used(&self, members: &[Member]) -> Vec<String> {
        let written: Vec<String> = members
            .iter()
            .flat_map(|member| written_names(member.shown.written(self, IN_MODULE)))
            .collect();
        self.parameters
            .iter()
            .filter(|parameter| written.contains(parameter))
            .cloned()
            .collect()
    }

    /// `MongoFields` of a type serde writes as one value, and what builds it: the path of that
    /// value, at the empty key, which it dereferences to. `text` adds `$regex`, for a value serde
    /// writes as text. The path is a parameter the type's own `impl` writes, so the module names
    /// no type of its author's and builds where `super` does not reach the type: in a function.
    fn whole(&self, text: bool) -> (TokenStream, TokenStream) {
        let (module, root, whole, value) = (&self.module, &self.root, &self.whole, &self.value);
        let arguments = self.arguments();
        let own = self.bare(None);
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
                        self::Field::<#root, ::std::string::String, ::std::string::String>::plain(self.whole.held_path())
                            .regex(pattern, options)
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
            quote! { #module::MongoFields::built(#module::Field::plain(#module::MongoPath::ROOT)) },
        )
    }

    /// The parameters an `impl` of a struct of functions beside the type declares, under what the
    /// type's own `impl` of its paths is under: `Serialize` for itself and for each of its type
    /// parameters, which a path writes a value with.
    fn writing(&self) -> Generics {
        let mut bounded = self.declared(None);
        if !self.type_parameters.is_empty() {
            let itself = self.itself(0);
            bounded
                .make_where_clause()
                .predicates
                .push(parse_quote!(#itself: ::serde::Serialize));
            bound_parameters(
                &mut bounded,
                &parse_quote!(::serde::Serialize),
                Some(&self.root),
            );
        }
        bounded
    }
}

/// Where serde writes a value: the key of what holds it, relative to the type the paths are of,
/// and its own key there, or none for a value written in place of what holds it.
struct Place {
    key: Option<String>,
    under: String,
}

impl Place {
    /// The key of the value, relative to the type the paths are of: empty for the type itself.
    fn joined(&self) -> String {
        match &self.key {
            None => self.under.clone(),
            Some(key) if self.under.is_empty() => key.clone(),
            Some(key) => format!("{}.{key}", self.under),
        }
    }

    /// The place, as the doc of a function names it.
    fn told(&self) -> String {
        let key = self.joined();
        if key.is_empty() {
            "the value itself".to_owned()
        } else {
            format!("`{key}`")
        }
    }
}

/// A flagged type a member is typed with, as the type that holds the member writes it.
struct Seen<'ty> {
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
    /// A struct of `mongo_members`: its name, what it is written with after the row type, and
    /// the trait its slots are read through where it is a tuple, which naming it asks to hold.
    Held {
        arguments: TokenStream,
        named: Ident,
        slotted: Option<Ident>,
    },
    /// One of the module's path kinds over a value of the author's: `Field<Root, V>`, and
    /// `Field<Root, V, N>` where the value wraps the model `nest` reaches.
    Kind {
        kind: Ident,
        nested: Option<TokenStream>,
        value: TokenStream,
    },
    /// `Nested<Root, N>`, over a flagged model the author wrote.
    Nested { model: TokenStream },
    /// No path: the slot of a tuple that has none.
    Skipped,
}

impl Shown {
    /// The type, as a module `depth` below the flagged type writes it.
    fn written(&self, own: &Own<'_>, depth: usize) -> TokenStream {
        let (root, kinds) = (&own.root, own.kinds(depth));
        match self {
            Self::Held {
                arguments,
                named,
                slotted: _slotted,
            } => {
                if depth == IN_MEMBERS {
                    quote! { self::#named<#root #arguments> }
                } else {
                    quote! { #kinds mongo_members::#named<#root #arguments> }
                }
            }
            Self::Kind {
                kind,
                nested,
                value,
            } => {
                let of = own.scoped(value.clone(), depth);
                let reached = nested.as_ref().map(|model| {
                    let written = own.scoped(model.clone(), depth);
                    quote! { , #written }
                });
                quote! { #kinds #kind<#root, #of #reached> }
            }
            Self::Nested { model } => {
                let of = own.scoped(model.clone(), depth);
                quote! { #kinds Nested<#root, #of> }
            }
            Self::Skipped => quote! { () },
        }
    }
}

/// The typed paths of an enum: one function per variant that holds a value serde writes, and one
/// `is_{variant}` per variant where serde writes what names it. A plain enum is one value.
///
/// A path typed with a plain enum takes a value of the enum and nothing else. The run below
/// hands one the text serde writes for a variant. A `compile_fail` doctest asserts only that some
/// error was raised, so it was compiled standalone as an ordinary test file, and the text under
/// it is the only error it earned, verbatim:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "kebab-case")]
/// pub enum InvoiceStatus {
///     Draft,
///     PastDue,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub status: InvoiceStatus,
/// }
///
/// fn main() {
///     let _ = Invoice::MONGO_FIELDS.status().eq("past-due");
/// }
/// ```
///
/// ```text
/// error[E0308]: mismatched types
///   --> tests/zz_probe.rs:23:47
///    |
/// 23 |     let _ = Invoice::MONGO_FIELDS.status().eq("past-due");
///    |                                            -- ^^^^^^^^^^ expected `InvoiceStatus`, found `&str`
///    |                                            |
///    |                                            arguments to this method are incorrect
///    |
/// note: method defined here
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
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
    let fields_rule = parse_written_renames(&item.attrs).rename_all_fields;
    let mut holding: Vec<(String, Member)> = Vec::new();
    let mut asks: Vec<Ask> = Vec::new();
    for WrittenVariant {
        name,
        stem,
        variant,
    } in written_variants(item)
    {
        let rust_name = variant.ident.unraw().to_string();
        let of = OfVariant {
            fields_rule: fields_rule.as_deref(),
            name: &name,
            put: &put,
            read: !parse_serde_key_omission(&variant.attrs).skips_deserializing,
            rust_name: &rust_name,
            stem: &stem,
        };
        if let Some(member) = emitter.content(variant, &of) {
            holding.push((stem.clone(), member));
        }
        asks.push(Ask {
            holds: writes_an_object(variant),
            name,
            stem,
        });
    }
    let asked = named.map(|by| asked(&by, &asks, &emitter.own));
    // An `is_{variant}` keeps its name, and the function of a variant that meets one is numbered.
    let mut taken: Vec<String> = if asked.is_some() {
        asks.iter().map(|ask| format!("is_{}", ask.stem)).collect()
    } else {
        Vec::new()
    };
    let mut members: Vec<(Ident, Member)> = Vec::new();
    for (stem, member) in holding {
        let name = unclaimed(&stem, &taken);
        members.push((member_ident(&name), member));
        taken.push(name);
    }
    let (arm_functions, arm_enum) = tag_arms(item)
        .map(|(tag, arms)| emitter.arms(&tag, &arms, &put))
        .unzip();
    let more =
        (asked.is_some() || arm_functions.is_some()).then(|| quote! { #asked #arm_functions });
    let (mut declared, built) = emitter.laid_out(Members::Named(members), more.as_ref());
    declared.extend(arm_enum);
    emitter.emitted(&declared, &built, false)
}

/// The typed paths of a struct: one function per field serde writes, one member per slot of a
/// tuple, and one value for a struct serde writes as the value it holds, or as nothing.
///
/// A path takes a value of its field's own type, and is a value a `const` item holds. `nest`
/// writes what is asked of a nested type's own paths under the field that holds it. This builds:
///
/// ```rust
/// # extern crate bson2 as bson;
/// use std::collections::HashMap;
///
/// use bson::doc;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "camelCase")]
/// pub struct Address {
///     pub city: String,
///     pub postal_code: String,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "camelCase")]
/// pub struct Customer {
///     pub address: Address,
///     pub open_invoices: u32,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "kebab-case")]
/// pub enum InvoiceStatus {
///     Draft,
///     PastDue,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
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
/// const TOTAL: invoice_schema::Field<Invoice, f64> = Invoice::MONGO_FIELDS.total();
///
/// fn main() -> Result<(), invoice_schema::WriteError> {
///     let (f, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
///     let few_open = f.customer().nest(customer.open_invoices().lte(3)?);
///     let _ = f
///         .status()
///         .eq(InvoiceStatus::PastDue)?
///         .or(TOTAL.gt(1000.0)?)
///         .and(f.tags().contains("export".to_owned())?.or(few_open))
///         .or(f.paid_at().exists(true).negated());
///     let in_city = Address::MONGO_FIELDS.city().eq("Santo Domingo".to_owned())?;
///     let _ = f.customer().nest(customer.address().nest(in_city));
///     let _ = f
///         .number()
///         .eq("INV-0042".to_owned())?
///         .and(invoice_schema::Filter::<Invoice>::raw(doc! { "details.costCenter": "CC-7" }));
///     Ok(())
/// }
/// ```
///
/// Each run below declares one such type and asks of its paths what they refuse. A
/// `compile_fail` doctest asserts only that some error was raised, so each was compiled
/// standalone as an ordinary test file, and the text under it is the only error it earned,
/// verbatim.
///
/// A field the type does not declare, which is a function its struct of paths does not have:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Customer {
///     pub name: String,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub customer: Customer,
///     pub total: f64,
/// }
///
/// fn main() {
///     let _ = Invoice::MONGO_FIELDS.amount().gt(1000.0);
/// }
/// ```
///
/// ```text
/// error[E0599]: no method named `amount` found for struct `invoice_schema::MongoFields<Root>` in the current scope
///   --> tests/zz_probe.rs:22:35
///    |
/// 14 | #[model_schema(enable = ["decode_with", "mongodb"])]
///    | ---------------------------------------------------- method `amount` not found for this struct
/// ...
/// 22 |     let _ = Invoice::MONGO_FIELDS.amount().gt(1000.0);
///    |                                   ^^^^^^ method not found in `invoice_schema::MongoFields<Invoice>`
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// `$unset` on a field every row holds:
///
/// ```rust,compile_fail
/// # extern crate bson2 as bson;
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub number: String,
/// }
///
/// fn main() {
///     let _ = Invoice::MONGO_FIELDS.number().unset();
/// }
/// ```
///
/// ```text
/// error[E0599]: no method named `unset` found for struct `__tixschema_mongodb::Field<Root, V, N>` in the current scope
///   --> tests/zz_probe.rs:15:44
///    |
///  6 | tixschema::mongodb_foundation!();
///    | -------------------------------- method `unset` not found for this struct
/// ...
/// 15 |     let _ = Invoice::MONGO_FIELDS.number().unset();
///    |                                            ^^^^^
///    |
/// help: there is a method `set` with a similar name, but with different arguments
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
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
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(rename_all = "camelCase")]
/// pub struct Invoice {
///     pub open_invoices: u32,
/// }
///
/// fn main() {
///     let _ = Invoice::MONGO_FIELDS.open_invoices().lte("3");
/// }
/// ```
///
/// ```text
/// error[E0308]: mismatched types
///   --> tests/zz_probe.rs:16:55
///    |
/// 16 |     let _ = Invoice::MONGO_FIELDS.open_invoices().lte("3");
///    |                                                   --- ^^^ expected `u32`, found `&str`
///    |                                                   |
///    |                                                   arguments to this method are incorrect
///    |
/// note: method defined here
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
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
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub total: f64,
/// }
///
/// fn main() {
///     let _ = Invoice::MONGO_FIELDS.total().gt("1000");
/// }
/// ```
///
/// ```text
/// error[E0308]: mismatched types
///   --> tests/zz_probe.rs:15:46
///    |
/// 15 |     let _ = Invoice::MONGO_FIELDS.total().gt("1000");
///    |                                           -- ^^^^^^ expected `f64`, found `&str`
///    |                                           |
///    |                                           arguments to this method are incorrect
///    |
/// note: method defined here
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
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
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub tags: Vec<String>,
/// }
///
/// fn main() -> Result<(), invoice_schema::WriteError> {
///     let _ = Invoice::MONGO_FIELDS.tags().gt("export".to_owned())?;
///     Ok(())
/// }
/// ```
///
/// ```text
/// error[E0599]: `ListField<Invoice, std::string::String>` is not an iterator
///   --> tests/zz_probe.rs:15:42
///    |
///  6 | tixschema::mongodb_foundation!();
///    | -------------------------------- method `gt` not found for this struct because it doesn't satisfy `ListField<Invoice, std::string::String>: Iterator`
/// ...
/// 15 |     let _ = Invoice::MONGO_FIELDS.tags().gt("export".to_owned())?;
///    |                                          ^^ `ListField<Invoice, std::string::String>` is not an iterator
///    |
///    = note: the following trait bounds were not satisfied:
///            `ListField<Invoice, std::string::String>: Iterator`
///            which is required by `&mut ListField<Invoice, std::string::String>: Iterator`
/// note: the trait `Iterator` must be implemented
///   --> $RUST/core/src/iter/traits/iterator.rs:42:1
///    |
/// 42 | pub const trait Iterator {
///    | ^^^^^^^^^^^^^^^^^^^^^^^^
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
/// tixschema::decode_with_foundation!();
/// tixschema::mongodb_foundation!();
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Invoice {
///     pub total: f64,
/// }
///
/// #[model_schema(enable = ["decode_with", "mongodb"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Order {
///     pub placed: bool,
/// }
///
/// fn main() -> Result<(), invoice_schema::WriteError> {
///     let invoice = Invoice::MONGO_FIELDS;
///     let _ = invoice.total().gt(1000.0)?.and(Order::MONGO_FIELDS.placed().eq(true)?);
///     Ok(())
/// }
/// ```
///
/// ```text
/// error[E0277]: the trait bound `__tixschema_mongodb::Filter<Order>: AsRef<PhantomData<Invoice>>` is not satisfied
///   --> tests/zz_probe.rs:22:45
///    |
/// 22 |     let _ = invoice.total().gt(1000.0)?.and(Order::MONGO_FIELDS.placed().eq(true)?);
///    |                                         --- ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ unsatisfied trait bound
///    |                                         |
///    |                                         required by a bound introduced by this call
///    |
/// help: the trait `AsRef<PhantomData<Invoice>>` is not implemented for `__tixschema_mongodb::Filter<Order>`
///       but trait `AsRef<PhantomData<Order>>` is implemented for it
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///    = help: for that trait implementation, expected `Order`, found `Invoice`
/// note: required by a bound in `__tixschema_mongodb::Filter::<Root>::and`
///   --> tests/zz_probe.rs:6:1
///    |
///  6 | tixschema::mongodb_foundation!();
///    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ required by this bound in `Filter::<Root>::and`
///    = note: this error originates in the macro `tixschema::mongodb_foundation` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
pub fn struct_paths(item: &ItemStruct, written: &[String]) -> TypedPaths {
    let mut emitter = Emitter::new(&item.ident, &item.generics, written);
    let (declared, built, one_value) = match (value_slot(item), &item.fields) {
        (Some(slot), _) => {
            let (declared, built) = emitter.own.whole(is_text(slot));
            (declared, built, true)
        }
        (None, Fields::Named(named)) => {
            let rule = parse_written_renames(&item.attrs).rename_all;
            let members = emitter.keyed(named, rule.as_deref(), "", None, true);
            let (declared, built) = emitter.laid_out(Members::Named(members), None);
            (declared, built, false)
        }
        (None, Fields::Unit) => {
            let (declared, built) = emitter.own.whole(false);
            (declared, built, true)
        }
        (None, Fields::Unnamed(slots)) => {
            let members = emitter.positional(slots, "", None, true);
            let (declared, built) = emitter.laid_out(Members::Positional(members), None);
            (declared, built, false)
        }
    };
    emitter.emitted(&declared, &built, one_value)
}

/// The model the arm of `variant` holds: what a path kind over its one slot names as the model
/// `nest` reaches. `None` for a variant with fields of its own, with several slots, with nothing
/// or with one slot serde never writes, and for a map.
fn arm_model(variant: &Variant) -> Option<&Type> {
    let Fields::Unnamed(slots) = &variant.fields else {
        return None;
    };
    let mut written = slots.unnamed.iter();
    let (Some(only), None) = (written.next(), written.next()) else {
        return None;
    };
    if has_serde_skip_serializing(&only.attrs) {
        return None;
    }
    let (_kinds, value) = taken(&only.ty)?;
    Some(through_wrappers(value).0)
}

/// The `is_{variant}` methods of an enum's struct of paths, by what names a variant in the form
/// serde writes the enum in: each reads the key that names it as the enum's own level writes it.
fn asked(by: &Named<'_>, asks: &[Ask], own: &Own<'_>) -> TokenStream {
    let (root, kinds) = (&own.root, own.kinds(0));
    let each = asks.iter().map(|ask| {
        let named = format_ident!("is_{}", ask.stem);
        let told = format!(
            "Rows whose value is the variant serde writes as `{}`.",
            ask.name
        );
        let name = &ask.name;
        let body = match by {
            Named::ByItsKey if ask.holds => {
                quote! { #kinds Filter::held(#name, "$exists", bson::Bson::Boolean(true)) }
            }
            Named::ByItsKey => {
                quote! { #kinds Filter::held("", "$eq", bson::Bson::String(#name.to_owned())) }
            }
            Named::ByTag(tag) => {
                quote! { #kinds Filter::held(#tag, "$eq", bson::Bson::String(#name.to_owned())) }
            }
        };
        quote! {
            #[doc = #told]
            #[must_use]
            pub fn #named(&self) -> #kinds Filter<#root> {
                #body
            }
        }
    });
    quote! { #(#each)* }
}

/// Whether the tokens end on `::`, so that the name after them continues a path.
fn continues_a_path(tokens: &[TokenTree]) -> bool {
    let [.., TokenTree::Punct(first), TokenTree::Punct(second)] = tokens else {
        return false;
    };
    first.as_char() == ':' && first.spacing() == Spacing::Joint && second.as_char() == ':'
}

/// `name` as the name of a function, at the call site's span. `None` where it is no identifier,
/// as under a tag key whose snake case starts with a digit.
fn function_ident(name: &str) -> Option<Ident> {
    syn::parse_str::<Ident>(name)
        .is_ok()
        .then(|| format_ident!("{name}"))
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
        (map, _, Some(_)) if is_map_wrapper(map) => Held::Map,
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

/// A struct `named` that holds no path, told as `told`, as the module `depth` below the type
/// declares it, and the `impl` beside the type with one function per member of `members` that
/// answers the member's path, and `more` beside them. It carries no function to be built with,
/// whose name a member's could meet: its one member is a public marker, which whoever builds the
/// struct writes.
fn of_functions(
    named: &Ident,
    members: &[(Ident, Member)],
    told: &str,
    own: &Own<'_>,
    more: Option<&TokenStream>,
    depth: usize,
) -> (TokenStream, Option<TokenStream>) {
    let generics = own.bare(None);
    let marked = generics.type_params().map(|parameter| &parameter.ident);
    let shown = shown_as(named, &generics);
    let beside = (!members.is_empty() || more.is_some()).then(|| {
        let functions = members.iter().map(|(name, member)| {
            let (of, answered, built) = (
                &member.told,
                member.shown.written(own, 0),
                member.built.written(own),
            );
            quote! {
                #[doc = #of]
                #[must_use]
                pub const fn #name(&self) -> #answered {
                    #built
                }
            }
        });
        let writing = own.writing();
        let (impl_generics, type_generics, bounds) = writing.split_for_impl();
        let (kinds, within) = (own.kinds(0), within(depth));
        quote! {
            impl #impl_generics #kinds #within #named #type_generics #bounds {
                #(#functions)*

                #more
            }
        }
    });
    let declared = quote! {
        #[doc = #told]
        #[non_exhaustive]
        pub struct #named #generics {
            /// The row the paths are of, and what the type is written with: no path is held.
            pub row: ::core::marker::PhantomData<fn() -> (#(#marked,)*)>,
        }

        #shown
    };
    (declared, beside)
}

/// A tuple struct `named` whose every member is one of `slots`, each one public, under the
/// parameters `used` names, told as `told`, as the module `depth` below the type declares it.
/// Each slot's type is read through `slotted`, a trait the module declares, and with it is the
/// `impl` of that trait beside the type, where every name the slots' types write resolves.
fn of_slots(
    named: &Ident,
    slotted: &Ident,
    slots: &[Member],
    used: Option<&[String]>,
    told: &str,
    own: &Own<'_>,
    depth: usize,
) -> (TokenStream, TokenStream) {
    let declared = own.bare(used);
    let (_, arguments, _) = declared.split_for_impl();
    let filled = quote! { self::#slotted #arguments };
    let mut generics = declared.clone();
    // The impl beside the type may be under the author's bounds, which the struct cannot name.
    generics
        .make_where_clause()
        .predicates
        .push(parse_quote!((): #filled));
    let where_clause = &generics.where_clause;
    let shown = shown_as(named, &generics);
    let positions: Vec<Ident> = (0..slots.len())
        .map(|position| format_ident!("Slot{position}"))
        .collect();
    let each = positions
        .iter()
        .map(|position| quote! { <() as #filled>::#position });
    let told_each = (0..slots.len()).map(|position| format!("The path of slot {position}."));
    let told_trait = format!(
        "The type of each slot of `{}`, written beside the type where every name it holds resolves.",
        named.unraw()
    );
    let module_items = quote! {
        #[doc = #told_trait]
        pub trait #slotted #declared {
            #(
                #[doc = #told_each]
                type #positions;
            )*
        }

        #[doc = #told]
        #[non_exhaustive]
        pub struct #named #generics (#(pub #each),*) #where_clause;

        #shown
    };
    let beside_generics = own.declared(used);
    let (impl_generics, type_generics, bounds) = beside_generics.split_for_impl();
    let (kinds, within) = (own.kinds(0), within(depth));
    let values = slots.iter().map(|slot| slot.shown.written(own, 0));
    let beside = quote! {
        impl #impl_generics #kinds #within #slotted #type_generics for () #bounds {
            #(type #positions = #values;)*
        }
    };
    (module_items, beside)
}

/// The model `nest` reaches under a value of type `value`, where that is not `value` itself:
/// what the wrappers serde writes as the value they hold are around.
fn reached_through(value: &Type) -> Option<TokenStream> {
    let (inner, wrapped) = through_wrappers(value);
    wrapped.then(|| inner.to_token_stream())
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

/// What an operator over a value of type `ty` takes, with the kinds a path over it is where the
/// value is a plain one and where it is a model: what a list or an `Option` holds, or the value
/// itself. `None` for a map, whose keys are data.
fn taken(ty: &Type) -> Option<([&'static str; 2], &Type)> {
    match held(ty) {
        Held::List(inner) => Some((["ListField", "ModelList"], inner)),
        Held::Map => None,
        Held::Optional(inner) => (!matches!(held(inner), Held::Map))
            .then_some((["OptionalField", "OptionalModel"], inner)),
        Held::Value => Some((["Field", "Model"], ty)),
    }
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

/// The module of a struct of paths `depth` below the type, after the path of the schema module:
/// `mongo_members::`, or nothing for the schema module itself.
fn within(depth: usize) -> Option<TokenStream> {
    (depth == IN_MEMBERS).then(|| quote! { mongo_members:: })
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
