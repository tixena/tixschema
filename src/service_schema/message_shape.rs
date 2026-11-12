//! A message that is a list, a map or an optional of a model: read element by element with no
//! judgement, then validated element by element.

#[cfg(all(feature = "typescript", feature = "zod"))]
use super::parse::{OperationDef, OperationInputs};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{GenericArgument, PathArguments, Type};

/// The collection a layer of a message is.
enum Kind {
    Map,
    Option,
    Vec,
}

/// A message type seen as the collections around the type each element is read at, outermost
/// first.
pub struct Shape {
    below: Option<Box<Self>>,
    element: Type,
    kind: Kind,
    original: Type,
}

impl Shape {
    #[cfg(all(feature = "typescript", feature = "zod"))]
    /// How many lists and maps wrap the model: the leading steps of a path into the message that
    /// are an index or a key, and no field of a model.
    pub fn depth(&self) -> usize {
        let beneath = self.below.as_ref().map_or(0, |layer| layer.depth());
        match self.kind {
            Kind::Option => beneath,
            Kind::Map | Kind::Vec => 1 + beneath,
        }
    }

    /// The type each collection is read as, with `held` where an element sits.
    pub fn held_type(&self, held: &Type) -> Type {
        let placed = self
            .below
            .as_ref()
            .map_or_else(|| held.clone(), |layer| layer.held_type(held));
        let mut replaced = self.original.clone();
        if let Type::Path(path) = &mut replaced
            && let Some(last) = path.path.segments.last_mut()
            && let PathArguments::AngleBracketed(arguments) = &mut last.arguments
            && let Some(GenericArgument::Type(slot)) = arguments.args.last_mut()
        {
            *slot = placed;
        }
        replaced
    }

    /// The innermost type, the one a model's own read is asked of.
    pub fn leaf(&self) -> &Type {
        self.below
            .as_ref()
            .map_or(&self.element, |layer| layer.leaf())
    }

    /// The shape of `message`, or `None` when it is not a list, a map or an optional.
    pub fn of(message: &Type) -> Option<Self> {
        let (kind, element) = collection_of(message)?;
        Some(Self {
            below: Self::of(element).map(Box::new),
            element: element.clone(),
            kind,
            original: message.clone(),
        })
    }

    /// The expression turning a value read as [`Self::held_type`] into the message.
    pub fn unwrapped(&self, value: &TokenStream) -> TokenStream {
        let element = quote! { item };
        let turned = self
            .below
            .as_ref()
            .map_or_else(|| quote! { item.0 }, |layer| layer.unwrapped(&element));
        let original = &self.original;
        match self.kind {
            Kind::Option => quote! { #value.map(|item| #turned) },
            Kind::Vec => quote! {
                #value.into_iter().map(|item| #turned).collect::<#original>()
            },
            Kind::Map => quote! {
                #value.into_iter().map(|(key, item)| (key, #turned)).collect::<#original>()
            },
        }
    }

    /// The statements that validate every element of `value`, a reference to the collection,
    /// pushing what each reports onto `violations`.
    pub fn validated(&self, value: &TokenStream) -> TokenStream {
        let element = quote! { item };
        let walked = self.below.as_ref().map_or_else(
            || {
                quote! {
                    violations.extend(item.validation_issues());
                }
            },
            |layer| layer.validated(&element),
        );
        match self.kind {
            Kind::Option => quote! {
                if let ::core::option::Option::Some(item) = #value.as_ref() {
                    #walked
                }
            },
            Kind::Vec => quote! {
                for item in #value.iter() {
                    #walked
                }
            },
            Kind::Map => quote! {
                for item in #value.values() {
                    #walked
                }
            },
        }
    }
}

#[cfg(all(feature = "typescript", feature = "zod"))]
/// How many lists and maps wrap the model of `operation`'s message.
pub fn collection_depth(operation: &OperationDef) -> usize {
    match &operation.inputs {
        OperationInputs::Named(declared) => Shape::of(declared).map_or(0, |shape| shape.depth()),
        OperationInputs::Empty | OperationInputs::Generated(_) => 0,
    }
}

/// The collection `held` is and the type it holds: a `Vec`, an `Option` or a map keyed by a
/// string.
fn collection_of(held: &Type) -> Option<(Kind, &Type)> {
    let Type::Path(path) = held else {
        return None;
    };
    if path.qself.is_some() {
        return None;
    }
    let last = path.path.segments.last()?;
    let PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return None;
    };
    let (kind, wanted) = match last.ident.to_string().as_str() {
        "Option" => (Kind::Option, 1),
        "Vec" => (Kind::Vec, 1),
        "HashMap" | "BTreeMap" => (Kind::Map, 2),
        _ => return None,
    };
    if arguments.args.len() != wanted {
        return None;
    }
    if let GenericArgument::Type(element) = arguments.args.last()? {
        Some((kind, element))
    } else {
        None
    }
}
