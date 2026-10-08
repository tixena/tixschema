//! A field typed with an alias `#[model_schema]` was written on above, walked as the type the
//! alias names rather than read whole.
//!
//! The alias's tokens are written where the alias is declared, and a name among them may mean
//! nothing where the field is. So the type the walker is given is written out again: a standard
//! type by its full path, and every other type under an alias of its own, in the walked type's
//! schema module, that reaches it from the field's type (`<Counts as IntoIterator>::Item`) and
//! keeps the name the walker reads it by.

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::ext::IdentExt as _;
use syn::{Expr, Fields, GenericArgument, PathArguments, Type, TypePath, parse_quote};

use crate::utils::{Declared, declared, written_type};

/// How deep an alias of an alias is followed.
const DEPTH: usize = 8;

/// The types that mean the same wherever they are written.
pub const PRIMITIVES: [&str; 17] = [
    "bool", "char", "f32", "f64", "i128", "i16", "i32", "i64", "i8", "isize", "str", "u128", "u16",
    "u32", "u64", "u8", "usize",
];

/// What one walked type's fields reach through the aliases they are typed with.
pub struct Reach {
    /// Whether an alias of a map was reached, whose entries `EntryOf` names the halves of.
    entries: bool,
    /// One check per field written out again: that it still names the type the field declares.
    guards: Vec<TokenStream>,
    module: Ident,
    /// One module per type reached from a field's own type, holding the alias that names it.
    modules: Vec<TokenStream>,
}

impl Reach {
    /// The type one entry's `half` is, of the map `map` names.
    fn entry_half(&mut self, map: &TokenStream, half: &str) -> TokenStream {
        self.entries = true;
        let named = Ident::new(half, Span::call_site());
        quote! { <<#map as ::core::iter::IntoIterator>::Item as super::EntryOf>::#named }
    }

    /// Writes each of `fields` that is typed with an alias seen above as the type it names.
    pub fn fields(&mut self, fields: &mut Fields) {
        for field in fields {
            if let Some(named) = self.through(&field.ty) {
                let written = &field.ty;
                self.guards.push(quote! {
                    const _: fn(#written) -> #named = |held| held;
                });
                field.ty = named;
            }
        }
    }

    /// The checks, written beside the type: a field written out again names the type it declares.
    pub fn guards(&self) -> TokenStream {
        let guards = &self.guards;
        quote! { #(#guards)* }
    }

    /// What reaching the fields' types adds to the schema module.
    pub fn module_items(&self) -> TokenStream {
        let modules = &self.modules;
        let entry = self.entries.then(entry_of_items);
        quote! {
            #entry
            #(#modules)*
        }
    }

    /// `shape`, the type an alias names, written so it means the same beside the walked type:
    /// `held` names it from inside a module of the schema module. The answer says whether it is
    /// one type reached as it is, rather than one walked inside.
    fn named(&mut self, shape: &Type, held: &TokenStream, depth: usize) -> (Type, bool) {
        let written = written_type(shape);
        if let Type::Paren(inside) = written {
            self.named(&inside.elem, held, depth)
        } else if let Type::Array(array) = written
            && matches!(array.len, Expr::Lit(_))
        {
            let (item, _) = self.named(&array.elem, &item_of(held), depth);
            let length = &array.len;
            (parse_quote! { [#item; #length] }, false)
        } else if let Type::Tuple(tuple) = written
            && !tuple.elems.is_empty()
            && tuple.elems.iter().all(stands_anywhere)
        {
            let positions = tuple
                .elems
                .iter()
                .map(|position| self.named(position, held, depth).0);
            (parse_quote! { (#(#positions,)*) }, false)
        } else if let Type::Path(path) = written
            && path.qself.is_none()
        {
            self.named_path(path, held, depth)
        } else {
            (self.reached(held, &format_ident!("Held")), true)
        }
    }

    fn named_path(&mut self, path: &TypePath, held: &TokenStream, depth: usize) -> (Type, bool) {
        let Some(last) = path.path.segments.last() else {
            return (self.reached(held, &format_ident!("Held")), true);
        };
        let name = last.ident.unraw().to_string();
        let Some(arguments) = type_arguments(&last.arguments) else {
            return (self.reached(held, &last.ident), true);
        };
        if arguments.is_empty() {
            if PRIMITIVES.contains(&name.as_str()) {
                let ident = &last.ident;
                return (parse_quote! { #ident }, false);
            }
            if name == "String" {
                return (parse_quote! { ::std::string::String }, false);
            }
            if depth < DEPTH
                && let Some(Declared::Alias(stands_for)) = declared(&name)
                && let Ok(aliased) = syn::parse_str::<Type>(&stands_for)
            {
                return self.named(&aliased, held, depth.saturating_add(1));
            }
        }
        if let ([inside], Some(holder)) = (arguments.as_slice(), holder_of_one(&name)) {
            let reaches = if matches!(name.as_str(), "Arc" | "Box" | "Rc") {
                quote! { <#held as ::core::ops::Deref>::Target }
            } else {
                item_of(held)
            };
            let (item, _) = self.named(inside, &reaches, depth);
            return (parse_quote! { #holder<#item> }, false);
        }
        if let ([key, value], "BTreeMap" | "HashMap") = (arguments.as_slice(), name.as_str()) {
            let (key_half, value_half) =
                (self.entry_half(held, "Key"), self.entry_half(held, "Value"));
            let (keyed, _) = self.named(key, &key_half, depth);
            let (valued, _) = self.named(value, &value_half, depth);
            let map = &last.ident;
            return (
                parse_quote! { ::std::collections::#map<#keyed, #valued> },
                false,
            );
        }
        (self.reached(held, &last.ident), true)
    }

    pub fn new(module_name: &str) -> Self {
        Self {
            entries: false,
            guards: Vec::new(),
            module: Ident::new(module_name, Span::call_site()),
            modules: Vec::new(),
        }
    }

    /// An alias named `name` of the type `held` names, in a module of its own, and its path from
    /// beside the walked type. The walker reads a type by its last name, which the alias keeps.
    fn reached(&mut self, held: &TokenStream, name: &Ident) -> Type {
        let (module, at) = (
            &self.module,
            format_ident!("decode_with_at{}", self.modules.len()),
        );
        self.modules.push(quote! {
            pub mod #at {
                pub type #name = #held;
            }
        });
        parse_quote! { #module::#at::#name }
    }

    /// `ty` as the type it names, where it is an alias seen above of a type the walker walks
    /// inside: a list, a map, an `Option`, a tuple. `None` leaves the field as it is written.
    fn through(&mut self, ty: &Type) -> Option<Type> {
        let Type::Path(path) = written_type(ty) else {
            return None;
        };
        let last = path.path.segments.last()?;
        if path.qself.is_some() || !last.arguments.is_none() {
            return None;
        }
        let Some(Declared::Alias(stands_for)) = declared(&last.ident.unraw().to_string()) else {
            return None;
        };
        let aliased = syn::parse_str::<Type>(&stands_for).ok()?;
        let (kept_modules, kept_entries) = (self.modules.len(), self.entries);
        let (walked, reached) = self.named(&aliased, &from_two_modules_down(path), 0);
        if reached {
            self.modules.truncate(kept_modules);
            self.entries = kept_entries;
            return None;
        }
        Some(walked)
    }
}

/// `EntryOf`: the key and the value of one entry of a map.
pub fn entry_of_items() -> TokenStream {
    quote! {
        /// The key and the value of one entry of a map, by which a field typed with an alias of a
        /// map names what the map holds.
        pub trait EntryOf {
            type Key;
            type Value;
        }

        impl<K, V> EntryOf for (K, V) {
            type Key = K;
            type Value = V;
        }
    }
}

/// The path of the type `path` names beside the walked type, as a module two below reads it.
fn from_two_modules_down(path: &TypePath) -> TokenStream {
    let named = &path.path;
    let first = named
        .segments
        .first()
        .map(|segment| segment.ident.to_string());
    if named.leading_colon.is_some() || first.as_deref() == Some("crate") {
        quote! { #named }
    } else if first.as_deref() == Some("self") {
        let rest = named.segments.iter().skip(1);
        quote! { super::super #(::#rest)* }
    } else {
        quote! { super::super::#named }
    }
}

/// The full path of the standard type `name` that holds one type, written with one argument.
fn holder_of_one(name: &str) -> Option<TokenStream> {
    let ident = Ident::new(name, Span::call_site());
    match name {
        "Arc" => Some(quote! { ::std::sync::Arc }),
        "BTreeSet" | "BinaryHeap" | "HashSet" | "VecDeque" => {
            Some(quote! { ::std::collections::#ident })
        }
        "Box" => Some(quote! { ::std::boxed::Box }),
        "Option" => Some(quote! { ::core::option::Option }),
        "Rc" => Some(quote! { ::std::rc::Rc }),
        "Vec" => Some(quote! { ::std::vec::Vec }),
        _ => None,
    }
}

/// The type of one item of the list, or of what the `Option`, `held` names.
fn item_of(held: &TokenStream) -> TokenStream {
    quote! { <#held as ::core::iter::IntoIterator>::Item }
}

/// Whether `ty` is written with standard names alone, which mean the same anywhere.
fn stands_anywhere(ty: &Type) -> bool {
    let written = written_type(ty);
    if let Type::Paren(inside) = written {
        stands_anywhere(&inside.elem)
    } else if let Type::Array(array) = written {
        matches!(array.len, Expr::Lit(_)) && stands_anywhere(&array.elem)
    } else if let Type::Tuple(tuple) = written {
        tuple.elems.iter().all(stands_anywhere)
    } else if let Type::Path(path) = written
        && path.qself.is_none()
        && let Some(last) = path.path.segments.last()
        && let Some(arguments) = type_arguments(&last.arguments)
    {
        let name = last.ident.unraw().to_string();
        match arguments.as_slice() {
            [] => name == "String" || PRIMITIVES.contains(&name.as_str()),
            [inside] => holder_of_one(&name).is_some() && stands_anywhere(inside),
            [key, value] => {
                matches!(name.as_str(), "BTreeMap" | "HashMap")
                    && stands_anywhere(key)
                    && stands_anywhere(value)
            }
            _ => false,
        }
    } else {
        false
    }
}

/// The types `arguments` are, or `None` where one of them is not a type.
fn type_arguments(arguments: &PathArguments) -> Option<Vec<&Type>> {
    if arguments.is_none() {
        Some(Vec::new())
    } else if let PathArguments::AngleBracketed(written) = arguments {
        written
            .args
            .iter()
            .map(|argument| {
                if let GenericArgument::Type(ty) = argument {
                    Some(ty)
                } else {
                    None
                }
            })
            .collect()
    } else {
        None
    }
}
