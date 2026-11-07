//! `needs_wrapper()`: whether serde writes a value of a model type as something other than a JSON
//! object, so that a tool's arguments object, which must be one, holds it under a key.
//!
//! A type that names another reads the answer recorded when that one was expanded, and takes the
//! answer that is always valid, `true`, for one it has not met or one that writes what fills its
//! type parameter.

use super::{
    FieldDef, FieldDefType, Item, Tagging, VariantKind, get_field_def, sibling_schema_module_ident,
    type_parameters_in_scope, variant_on_the_wire, variant_wire_kind, written_as_its_slot,
};
use crate::utils::{record_needs_wrapper, recorded_needs_wrapper};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Type};

/// What a type is, as far as its wire shape goes.
enum Shape {
    /// A model type, named without type arguments or with them.
    Model {
        arguments: bool,
        name: String,
        span: proc_macro2::Span,
    },
    /// The answer is known from the type alone.
    Settled(bool),
}

/// The `needs_wrapper()` items an item earns: one on the type, except for an alias, which may
/// stand for a type that is no model type, and one on its schema module's `Schema`.
pub fn items(item: &Item) -> TokenStream {
    let (name, generics) = match item {
        Item::Struct(declared) => (&declared.ident, &declared.generics),
        Item::Enum(declared) => (&declared.ident, &declared.generics),
        Item::Type(declared) => (&declared.ident, &declared.generics),
        Item::Const(_)
        | Item::ExternCrate(_)
        | Item::Fn(_)
        | Item::ForeignMod(_)
        | Item::Impl(_)
        | Item::Macro(_)
        | Item::Mod(_)
        | Item::Static(_)
        | Item::Trait(_)
        | Item::TraitAlias(_)
        | Item::Union(_)
        | Item::Use(_)
        | Item::Verbatim(_)
        | _ => return TokenStream::new(),
    };
    let wrapped = of_item(item);
    let schema_module = sibling_schema_module_ident(&name.to_string(), name.span());
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let type_impl = (!matches!(item, Item::Type(_))).then(|| {
        quote! {
            impl #impl_generics #name #type_generics #where_clause {
                pub const fn needs_wrapper() -> bool {
                    #wrapped
                }
            }
        }
    });
    quote! {
        #type_impl

        impl #schema_module::Schema {
            pub const fn needs_wrapper() -> bool {
                #wrapped
            }
        }
    }
}

/// Records the answer for `item`, for the types that name it.
pub fn record(item: &Item) {
    let name = match item {
        Item::Struct(declared) => &declared.ident,
        Item::Enum(declared) => &declared.ident,
        Item::Type(declared) => &declared.ident,
        Item::Const(_)
        | Item::ExternCrate(_)
        | Item::Fn(_)
        | Item::ForeignMod(_)
        | Item::Impl(_)
        | Item::Macro(_)
        | Item::Mod(_)
        | Item::Static(_)
        | Item::Trait(_)
        | Item::TraitAlias(_)
        | Item::Union(_)
        | Item::Use(_)
        | Item::Verbatim(_)
        | _ => return,
    };
    record_needs_wrapper(&name.to_string(), of_item(item));
}

/// The expression a service reads for a message type: a model type answers for itself, and the
/// rest is decided from the type alone.
pub fn of_type(ty: &Type) -> TokenStream {
    match shape_of(ty, &[]) {
        Shape::Settled(wrapped) => quote! { #wrapped },
        Shape::Model {
            arguments: false,
            name,
            span,
        } => {
            let module = sibling_schema_module_ident(&name, span);
            quote! { #module::Schema::needs_wrapper() }
        }
        Shape::Model {
            arguments: true,
            name: _name,
            span: _span,
        } => {
            let named = peeled(ty);
            quote! { <#named>::needs_wrapper() }
        }
    }
}

fn of_item(item: &Item) -> bool {
    match item {
        Item::Struct(declared) => {
            let parameters = type_parameters_in_scope(&declared.generics);
            written_as_its_slot(declared).map_or_else(
                || !matches!(declared.fields, Fields::Named(_)),
                |slot| of_slot(&slot.ty, &parameters),
            )
        }
        Item::Enum(declared) => of_enum(declared, &type_parameters_in_scope(&declared.generics)),
        Item::Type(declared) => {
            of_slot(&declared.ty, &type_parameters_in_scope(&declared.generics))
        }
        Item::Const(_)
        | Item::ExternCrate(_)
        | Item::Fn(_)
        | Item::ForeignMod(_)
        | Item::Impl(_)
        | Item::Macro(_)
        | Item::Mod(_)
        | Item::Static(_)
        | Item::Trait(_)
        | Item::TraitAlias(_)
        | Item::Union(_)
        | Item::Use(_)
        | Item::Verbatim(_)
        | _ => true,
    }
}

fn of_enum(declared: &syn::ItemEnum, parameters: &[String]) -> bool {
    let tagging = Tagging::of(declared);
    if matches!(tagging, Tagging::Plain) {
        true
    } else if matches!(tagging, Tagging::External) {
        declared.variants.iter().any(|variant| {
            variant_on_the_wire(variant) && matches!(variant_wire_kind(variant), VariantKind::Unit)
        })
    } else if matches!(tagging, Tagging::Untagged) {
        declared
            .variants
            .iter()
            .filter(|variant| variant_on_the_wire(variant))
            .any(|variant| match variant_wire_kind(variant) {
                VariantKind::Named => false,
                VariantKind::TupleSingle => variant
                    .fields
                    .iter()
                    .next()
                    .is_none_or(|slot| of_slot(&slot.ty, parameters)),
                VariantKind::TupleMultiple | VariantKind::Unit => true,
            })
    } else {
        false
    }
}

/// Whether a value of `ty`, the slot an item is written as, is no object.
fn of_slot(ty: &Type, parameters: &[String]) -> bool {
    match shape_of(ty, parameters) {
        Shape::Settled(wrapped) => wrapped,
        Shape::Model {
            arguments: _arguments,
            name,
            span: _span,
        } => recorded_needs_wrapper(&name).unwrap_or(true),
    }
}

fn shape_of(ty: &Type, parameters: &[String]) -> Shape {
    let mut def = get_field_def("value", ty, "");
    def.erase_type_parameters(parameters);
    shape_of_field(&def)
}

fn shape_of_field(def: &FieldDef) -> Shape {
    if def.is_optional() || def.array_depth > 0 {
        return Shape::Settled(true);
    }
    match &def.field_type {
        FieldDefType::Map(..) | FieldDefType::Range(..) | FieldDefType::Result(..) => {
            Shape::Settled(false)
        }
        FieldDefType::SiblingType(name, arguments) => Shape::Model {
            arguments: !arguments.is_empty(),
            name: name.clone(),
            span: def.type_span,
        },
        FieldDefType::Boolean
        | FieldDefType::BooleanLiteral(_)
        | FieldDefType::Char
        | FieldDefType::F32
        | FieldDefType::F64
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::I8
        | FieldDefType::Isize
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::String
        | FieldDefType::StringLiteral(_)
        | FieldDefType::Tuple(_)
        | FieldDefType::TypeParam(_)
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::U8
        | FieldDefType::Unknown
        | FieldDefType::Usize => Shape::Settled(true),
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime
        | FieldDefType::NaiveDate
        | FieldDefType::NaiveDateTime
        | FieldDefType::NaiveTime => Shape::Settled(true),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => Shape::Settled(true),
    }
}

/// `ty` without the references and the transparent pointers around it.
fn peeled(ty: &Type) -> &Type {
    match ty {
        Type::Reference(reference) => peeled(&reference.elem),
        Type::Paren(parenthesized) => peeled(&parenthesized.elem),
        Type::Group(grouped) => peeled(&grouped.elem),
        Type::Path(path) => {
            let Some(last) = path.path.segments.last() else {
                return ty;
            };
            let pointer = matches!(last.ident.to_string().as_str(), "Box" | "Rc" | "Arc");
            let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
                return ty;
            };
            match (pointer, arguments.args.first()) {
                (true, Some(syn::GenericArgument::Type(inner))) => peeled(inner),
                _ => ty,
            }
        }
        Type::Array(_)
        | Type::FnPtr(_)
        | Type::ImplTrait(_)
        | Type::Infer(_)
        | Type::Macro(_)
        | Type::Never(_)
        | Type::Ptr(_)
        | Type::Slice(_)
        | Type::TraitObject(_)
        | Type::Tuple(_)
        | Type::Verbatim(_)
        | _ => ty,
    }
}
