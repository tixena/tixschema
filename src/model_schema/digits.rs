//! The string of decimal digits a `u64` and an `i64` travel as on the JSON wire.
//!
//! [`members`] reads which members of an item hold one where the macro writes that string: the
//! value itself, and what an `Option`, a `Box`, an `Rc`, an `Arc`, a `Cow`, a `Cell`, a lock, a
//! reference, a list, a set, an array, a map's value, a tuple, a range's bound and a `Result`'s
//! arm hold of it, through an alias declared above, beside the item's own parameters. Every
//! surface describes those integers as the string, and [`hang_on_struct`] and [`hang_on_enum`]
//! hand serde's derive the functions that write and read it.

use proc_macro2::TokenStream;
#[cfg(feature = "serde")]
use proc_macro2::{Ident, Span, TokenTree};
#[cfg(feature = "serde")]
use quote::{ToTokens as _, format_ident, quote};
use syn::Item;

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
use crate::field_type::FieldDef;
#[cfg(feature = "serde")]
use syn::{
    Attribute, DeriveInput, Field, Fields, GenericArgument, GenericParam, Generics, Lifetime,
    PathArguments, Type, TypePath, WherePredicate,
};

#[cfg(feature = "serde")]
use core::cell::RefCell;

#[cfg(feature = "serde")]
use super::judged_read::{naming_the_type, unclaimed_parameter};
#[cfg(feature = "serde")]
use crate::features::model_schema_prop::parse_model_schema_prop_attributes;
#[cfg(feature = "serde")]
use crate::features::serde::read_bounds::{
    BoundSides, asked_by_type, bound_sides, read_bounds, write_bounds,
};
#[cfg(feature = "serde")]
use crate::features::serde::{
    derives_deserialize, derives_serialize, has_serde_default, has_serde_skip_serializing,
    parse_serde_field_attributes, parse_serde_field_hooks, parse_serde_key_omission,
    serde_conversions,
};
#[cfg(feature = "serde")]
use crate::field_type::{
    RangeKind, is_map_wrapper, is_ownership_wrapper, is_sequence_wrapper, written_std_range,
    written_std_result,
};
#[cfg(feature = "serde")]
use crate::utils::{Declared, declared, to_snake_case, written_names, written_type};

/// The name of the newtype a digit string is written inside where the serializer is
/// human-readable, which a writer of rows reads as the integer it holds.
#[cfg(feature = "serde")]
pub const DIGITS_NEWTYPE: &str = "$__tixschema_digits";

/// The attribute [`marked`] puts on a member whose integers are written as digit strings.
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
const MARK: &str = "model_schema_digits";

/// The lifetime a lock written below another level is borrowed for while it is written.
#[cfg(feature = "serde")]
const HELD_LIFETIME: &str = "'model_schema_held";

/// How many aliases deep a name is followed to the type it stands for.
#[cfg(feature = "serde")]
const ALIAS_DEPTH: u8 = 16;

/// Which of the two integers a digit string holds.
#[cfg(feature = "serde")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Width {
    /// An `i64`.
    Signed,
    /// A `u64`.
    Unsigned,
}

#[cfg(feature = "serde")]
impl Width {
    /// The method of `serde::Deserializer` that reads the integer.
    fn deserialize(self) -> Ident {
        match self {
            Self::Signed => format_ident!("deserialize_i64"),
            Self::Unsigned => format_ident!("deserialize_u64"),
        }
    }

    /// What a refusal says the reader expected.
    const fn expecting(self) -> &'static str {
        match self {
            Self::Signed => "an i64 as a string of its decimal digits",
            Self::Unsigned => "a u64 as a string of its decimal digits",
        }
    }

    /// The Rust type.
    fn rust(self) -> TokenStream {
        match self {
            Self::Signed => quote! { i64 },
            Self::Unsigned => quote! { u64 },
        }
    }

    /// The method of `serde::Serializer` that writes the integer.
    fn serialize(self) -> Ident {
        match self {
            Self::Signed => format_ident!("serialize_i64"),
            Self::Unsigned => format_ident!("serialize_u64"),
        }
    }
}

/// How a member's type holds the integers the macro writes as digit strings, level by level.
#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
pub enum Shape {
    /// An array of a fixed length, written as the whole level.
    Array(Type, Box<Self>),
    /// A reference, written as the value it borrows. No value is read into one.
    Borrowed(Box<Self>),
    /// A `Cell`, written as the whole level, of the value it holds.
    Celled(Type, Box<Self>),
    /// A `Cow` of what it holds.
    Cow(Box<Self>),
    /// A `u64` or an `i64`.
    Digits(Width),
    /// A map, written as the whole level, of its values.
    Keyed(Type, Box<Self>),
    /// A list or a set, written as the whole level.
    Listed(Type, Box<Self>),
    /// A lock, taken as serde's own impl takes it before what it guards is written.
    Locked(Type, Guard, Box<Self>),
    /// An `Option`.
    Optional(Box<Self>),
    /// A `Box`, an `Rc` or an `Arc`, written as the whole level, of what it holds.
    Owned(Type, Box<Self>),
    /// What holds no such integer, written as its own type writes it.
    Plain(Type),
    /// A std range, of its bound.
    Ranged(RangeKind, Box<Self>),
    /// A `Result`, of its `Ok` arm and its `Err` arm.
    Resulted(Box<Self>, Box<Self>),
    /// The slice a `Box`, an `Rc` or an `Arc` holds.
    Slice(Box<Self>),
    /// A tuple, position by position.
    Tuple(Vec<Self>),
}

/// A lock serde takes before it writes the value it guards.
#[cfg(feature = "serde")]
#[derive(Clone, Copy, Debug)]
pub enum Guard {
    /// A `Mutex`.
    Mutex,
    /// A `RefCell`.
    RefCell,
    /// An `RwLock`.
    RwLock,
}

#[cfg(feature = "serde")]
impl Guard {
    /// The name of the std type.
    const fn name(self) -> &'static str {
        match self {
            Self::Mutex => "Mutex",
            Self::RefCell => "RefCell",
            Self::RwLock => "RwLock",
        }
    }

    /// The lock a std type of that name is.
    fn named(name: &str) -> Option<Self> {
        [Self::Mutex, Self::RefCell, Self::RwLock]
            .into_iter()
            .find(|guard| guard.name() == name)
    }

    /// What serde's own impl answers where the lock is not taken.
    const fn refusal(self) -> &'static str {
        match self {
            Self::Mutex | Self::RwLock => "lock poison error while serializing",
            Self::RefCell => "already mutably borrowed",
        }
    }

    /// The method that takes the lock, as serde's own impl takes it.
    fn taken(self) -> Ident {
        match self {
            Self::Mutex => format_ident!("lock"),
            Self::RefCell => format_ident!("try_borrow"),
            Self::RwLock => format_ident!("read"),
        }
    }
}

/// How a member holds the integers the macro writes as digit strings.
#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
pub struct Held {
    pub shape: Shape,
}

/// One bound the functions written for a member may declare, with the item's parameters it names.
/// Kept as text: a token stream outlives no expansion, and the record outlives the item's.
#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
struct Predicate {
    names: Vec<String>,
    text: String,
}

#[cfg(feature = "serde")]
impl Predicate {
    /// The bound as a `where` clause writes it.
    fn tokens(&self) -> TokenStream {
        self.text.parse().unwrap_or_default()
    }
}

/// The parameters of the item a member's type names, which the functions written for it declare,
/// and the bounds each of them is written under.
#[cfg(feature = "serde")]
#[derive(Clone, Debug, Default)]
pub struct Scope {
    /// The lifetimes `'de` outlives in the read, as serde's derive reads the item.
    borrowed: Vec<Lifetime>,
    consts: Vec<(Ident, Type)>,
    lifetimes: Vec<Lifetime>,
    /// The bounds of the function that reads the member.
    read: Vec<Predicate>,
    types: Vec<Ident>,
    /// The bounds of the function that writes the member.
    written: Vec<Predicate>,
}

#[cfg(feature = "serde")]
impl Scope {
    /// The parameters as a function declares them, each followed by a comma.
    fn declared(&self) -> TokenStream {
        let lifetimes = &self.lifetimes;
        let types = &self.types;
        let consts = self
            .consts
            .iter()
            .map(|(ident, ty)| quote! { const #ident: #ty });
        quote! { #(#lifetimes,)* #(#types,)* #(#consts,)* }
    }

    /// Whether the member names no parameter of the item.
    pub const fn is_empty(&self) -> bool {
        self.lifetimes.is_empty() && self.types.is_empty() && self.consts.is_empty()
    }

    /// The scope of `generics` that `ty` names, each bound kept where every parameter it names is
    /// one of them.
    fn of(ty: &Type, generics: &ItemGenerics, bounds: &Bounds) -> Self {
        let mut named = Vec::new();
        generics.named_in(ty.to_token_stream(), &mut named);
        let whole = Self {
            borrowed: bounds.borrowed.clone(),
            consts: generics.consts.clone(),
            lifetimes: generics.lifetimes.clone(),
            read: bounds.read.clone(),
            types: generics.types.clone(),
            written: bounds.written.clone(),
        };
        whole.within(&named)
    }

    /// The parameters as a type is written with them, each followed by a comma.
    fn used(&self) -> TokenStream {
        let lifetimes = &self.lifetimes;
        let types = &self.types;
        let consts = self.consts.iter().map(|(ident, _)| ident);
        quote! { #(#lifetimes,)* #(#types,)* #(#consts,)* }
    }

    /// This scope held to the parameters `named`.
    fn within(&self, named: &[String]) -> Self {
        let kept = |predicates: &[Predicate]| -> Vec<Predicate> {
            predicates
                .iter()
                .filter(|predicate| {
                    !predicate.names.is_empty()
                        && predicate.names.iter().all(|name| named.contains(name))
                })
                .cloned()
                .collect()
        };
        Self {
            borrowed: self
                .borrowed
                .iter()
                .filter(|lifetime| named.contains(&lifetime.to_string()))
                .cloned()
                .collect(),
            consts: self
                .consts
                .iter()
                .filter(|(ident, _)| named.contains(&ident.to_string()))
                .cloned()
                .collect(),
            lifetimes: self
                .lifetimes
                .iter()
                .filter(|lifetime| named.contains(&lifetime.to_string()))
                .cloned()
                .collect(),
            read: kept(&self.read),
            types: self
                .types
                .iter()
                .filter(|ident| named.contains(&ident.to_string()))
                .cloned()
                .collect(),
            written: kept(&self.written),
        }
    }
}

/// The parameters an item declares, and its type as `Self` stands for it.
#[cfg(feature = "serde")]
struct ItemGenerics {
    consts: Vec<(Ident, Type)>,
    lifetimes: Vec<Lifetime>,
    named: TokenStream,
    types: Vec<Ident>,
}

#[cfg(feature = "serde")]
impl ItemGenerics {
    /// Whether `name` is one of the item's type or const parameters.
    fn declares(&self, name: &str) -> bool {
        self.types.iter().any(|ident| ident == name)
            || self.consts.iter().any(|(ident, _)| ident == name)
    }

    /// Collects into `named` each parameter of the item `tokens` write, once: a lifetime as `'a`.
    fn named_in(&self, tokens: TokenStream, named: &mut Vec<String>) {
        let mut quoted = false;
        for token in tokens {
            match token {
                TokenTree::Group(group) => self.named_in(group.stream(), named),
                TokenTree::Punct(mark) => {
                    quoted = mark.as_char() == '\'';
                    continue;
                }
                TokenTree::Ident(word) => {
                    let text = if quoted {
                        format!("'{word}")
                    } else {
                        word.to_string()
                    };
                    let declared = if quoted {
                        self.lifetimes
                            .iter()
                            .any(|lifetime| lifetime.to_string() == text)
                    } else {
                        self.declares(&text)
                    };
                    if declared && !named.contains(&text) {
                        named.push(text);
                    }
                }
                TokenTree::Literal(_) => {}
            }
            quoted = false;
        }
    }

    /// The parameters of an item that declares none.
    fn none() -> Self {
        Self {
            consts: Vec::new(),
            lifetimes: Vec::new(),
            named: TokenStream::new(),
            types: Vec::new(),
        }
    }

    fn of(ident: &Ident, generics: &Generics) -> Self {
        let (_, type_generics, _) = generics.split_for_impl();
        Self {
            consts: generics
                .const_params()
                .map(|constant| (constant.ident.clone(), constant.ty.clone()))
                .collect(),
            lifetimes: generics
                .lifetimes()
                .map(|declared| declared.lifetime.clone())
                .collect(),
            named: quote! { #ident #type_generics },
            types: generics
                .type_params()
                .map(|declared| declared.ident.clone())
                .collect(),
        }
    }

    /// `predicate` split into one bound each, with `Self` written out and the parameters each
    /// names.
    fn split(&self, predicate: &WherePredicate) -> Vec<Predicate> {
        let written: Vec<TokenStream> = match predicate {
            WherePredicate::Type(typed) => {
                let (binder, bounded) = (&typed.lifetimes, &typed.bounded_ty);
                typed
                    .bounds
                    .iter()
                    .map(|bound| quote! { #binder #bounded: #bound })
                    .collect()
            }
            WherePredicate::Lifetime(outlives) => {
                let lifetime = &outlives.lifetime;
                outlives
                    .bounds
                    .iter()
                    .map(|bound| quote! { #lifetime: #bound })
                    .collect()
            }
            _ => vec![predicate.to_token_stream()],
        };
        written
            .into_iter()
            .map(|asked| {
                let self_named = naming_the_type(asked, &self.named);
                let mut names = Vec::new();
                self.named_in(self_named.clone(), &mut names);
                Predicate {
                    names,
                    text: self_named.to_string(),
                }
            })
            .collect()
    }
}

/// The bounds every function written for a member of an item may declare: the item's own, and
/// what serde's derive writes on each impl.
#[cfg(feature = "serde")]
struct Bounds {
    borrowed: Vec<Lifetime>,
    read: Vec<Predicate>,
    written: Vec<Predicate>,
}

#[cfg(feature = "serde")]
impl Bounds {
    fn of(input: &DeriveInput, generics: &ItemGenerics) -> Self {
        let declared = &input.generics;
        let mut own: Vec<WherePredicate> = Vec::new();
        for parameter in &declared.params {
            match parameter {
                GenericParam::Type(typed) if !typed.bounds.is_empty() => {
                    let (ident, bounds) = (&typed.ident, &typed.bounds);
                    own.push(syn::parse_quote! { #ident: #bounds });
                }
                GenericParam::Lifetime(lived) if !lived.bounds.is_empty() => {
                    let (lifetime, bounds) = (&lived.lifetime, &lived.bounds);
                    own.push(syn::parse_quote! { #lifetime: #bounds });
                }
                GenericParam::Type(_) | GenericParam::Lifetime(_) | GenericParam::Const(_) => {}
            }
        }
        own.extend(
            declared
                .where_clause
                .iter()
                .flat_map(|clause| clause.predicates.iter().cloned()),
        );
        let read = read_bounds(input);
        let split = |predicates: &[WherePredicate]| -> Vec<Predicate> {
            own.iter()
                .chain(predicates)
                .flat_map(|predicate| generics.split(predicate))
                .collect()
        };
        Self {
            borrowed: read.borrowed.unwrap_or_default(),
            read: split(&read.predicates),
            written: split(&write_bounds(input)),
        }
    }
}

/// What one member's integers are written as digit strings through.
#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
pub struct DigitMember {
    /// The predicates serde's derive would have asked of the member, which it asks of a member
    /// read or written by a function of its own only where a `bound` says so: the read's, then
    /// the write's, as text.
    pub bound: (String, String),
    pub held: Held,
    /// The member's place among the ones declared.
    pub index: usize,
    /// Whether serde reads the member.
    pub reads: bool,
    /// The parameters of the item the member's type names.
    pub scope: Scope,
    /// The member's type, with `Self` written out.
    pub ty: Type,
    /// The variant the member is declared in, by its Rust name, or `None` for a struct's own.
    pub variant: Option<String>,
    /// Whether serde writes the member.
    pub writes: bool,
}

/// Whether serde writes and reads a container's members.
#[cfg(feature = "serde")]
#[derive(Clone, Copy)]
struct Directions {
    reads: bool,
    writes: bool,
}

/// What a container says of every member: the sides of the `bound` it writes, its parameters and
/// the bounds of the functions written.
#[cfg(feature = "serde")]
struct Container<'item> {
    bounds: Bounds,
    declared: &'item Generics,
    generics: ItemGenerics,
    sides: BoundSides,
}

/// The names the functions written for a member declare, none of them a name its type writes.
#[cfg(feature = "serde")]
struct Names {
    digits: Ident,
    entries: Ident,
    items: Ident,
    locked: Ident,
    ranged: Ident,
    reader: Ident,
    serializer: Ident,
    text: Ident,
    value: Ident,
}

#[cfg(feature = "serde")]
impl Names {
    fn of(ty: &Type) -> Self {
        let written = written_names(ty.to_token_stream());
        Self {
            digits: unclaimed_parameter("ModelSchemaDigits", &written),
            entries: unclaimed_parameter("ModelSchemaEntries", &written),
            items: unclaimed_parameter("ModelSchemaItems", &written),
            locked: unclaimed_parameter("ModelSchemaLocked", &written),
            ranged: unclaimed_parameter("ModelSchemaRange", &written),
            reader: unclaimed_parameter("D", &written),
            serializer: unclaimed_parameter("S", &written),
            text: unclaimed_parameter("ModelSchemaText", &written),
            value: unclaimed_parameter("V", &written),
        }
    }
}

/// What a shape uses of the types a written function declares.
#[cfg(feature = "serde")]
#[derive(Default)]
struct Used {
    entries: bool,
    items: bool,
    ranged: bool,
    widths: Vec<Width>,
}

#[cfg(feature = "serde")]
impl Used {
    fn of(shape: &Shape) -> Self {
        let mut used = Self::default();
        used.read(shape);
        used
    }

    fn read(&mut self, shape: &Shape) {
        match shape {
            Shape::Digits(width) => {
                if !self.widths.contains(width) {
                    self.widths.push(*width);
                }
            }
            Shape::Plain(_) => {}
            Shape::Optional(held)
            | Shape::Owned(_, held)
            | Shape::Celled(_, held)
            | Shape::Cow(held)
            | Shape::Borrowed(held)
            | Shape::Locked(_, _, held)
            | Shape::Array(_, held) => {
                self.read(held);
            }
            Shape::Slice(held) | Shape::Listed(_, held) => {
                self.items = true;
                self.read(held);
            }
            Shape::Keyed(_, held) => {
                self.entries = true;
                self.read(held);
            }
            Shape::Ranged(_, held) => {
                self.ranged = true;
                self.read(held);
            }
            Shape::Resulted(ok, err) => {
                self.read(ok);
                self.read(err);
            }
            Shape::Tuple(positions) => {
                for position in positions {
                    self.read(position);
                }
            }
        }
    }

    /// The widths a shape holds.
    fn widths(&self) -> impl Iterator<Item = Width> {
        self.widths.iter().copied()
    }
}

/// What builds the value serde writes for a member, and the types it declares for each lock it
/// takes below another level.
#[cfg(feature = "serde")]
struct Writer<'written> {
    /// How many lock types are declared.
    count: usize,
    names: &'written Names,
    scope: &'written Scope,
    sites: TokenStream,
}

#[cfg(feature = "serde")]
impl Writer<'_> {
    /// A type written for the lock `level`, which takes the lock as serde's own impl takes it,
    /// refuses in its words where it is not taken, and writes what it guards; and that type of
    /// `held`.
    fn locked(
        &mut self,
        level: &Type,
        lock: Guard,
        inner: &Shape,
        held: &TokenStream,
    ) -> TokenStream {
        let site = format_ident!("{}{}", self.names.locked, self.count);
        self.count += 1;
        let mut named = Vec::new();
        let generics = ItemGenerics {
            consts: self.scope.consts.clone(),
            lifetimes: self.scope.lifetimes.clone(),
            named: TokenStream::new(),
            types: self.scope.types.clone(),
        };
        generics.named_in(level.to_token_stream(), &mut named);
        let within = self.scope.within(&named);
        let (declared, used) = (within.declared(), within.used());
        let bounds = within.written.iter().map(Predicate::tokens);
        let lifetime = Lifetime::new(HELD_LIFETIME, Span::call_site());
        let serializer = &self.names.serializer;
        let (take, refusal) = (lock.taken(), lock.refusal());
        let guarded = self.written(inner, &quote! { &*taken }, 1);
        self.sites.extend(quote! {
            struct #site<#lifetime, #declared>(&#lifetime #level);
            impl<#lifetime, #declared> ::serde::Serialize for #site<#lifetime, #used>
            where
                #(#bounds,)*
            {
                fn serialize<#serializer>(&self, serializer: #serializer) -> ::core::result::Result<#serializer::Ok, #serializer::Error>
                where
                    #serializer: ::serde::Serializer,
                {
                    let taken = match self.0.#take() {
                        ::core::result::Result::Ok(taken) => taken,
                        ::core::result::Result::Err(_) => {
                            return ::core::result::Result::Err(<#serializer::Error as ::serde::ser::Error>::custom(#refusal));
                        }
                    };
                    ::serde::Serialize::serialize(&#guarded, serializer)
                }
            }
        });
        quote! { #site(#held) }
    }

    /// The value serde writes for `held`, a reference to a value of `shape`, with each integer
    /// written through the digit string. `depth` numbers the closures' bindings.
    fn written(&mut self, shape: &Shape, held: &TokenStream, depth: usize) -> TokenStream {
        let binding = format_ident!("held_{depth}");
        let receiving = receiver(held);
        match shape {
            Shape::Digits(_) => {
                let digits = &self.names.digits;
                quote! { #digits(*#receiving) }
            }
            Shape::Plain(_) => held.clone(),
            Shape::Optional(inner) => {
                let each = self.written(inner, &quote! { #binding }, depth + 1);
                quote! { #receiving.as_ref().map(|#binding| #each) }
            }
            Shape::Owned(_, inner) | Shape::Cow(inner) | Shape::Borrowed(inner) => {
                self.written(inner, &quote! { &**#held }, depth)
            }
            Shape::Celled(_, inner) => self.written(inner, &quote! { &#receiving.get() }, depth),
            Shape::Slice(inner) | Shape::Listed(_, inner) => {
                let each = self.written(inner, &quote! { #binding }, depth + 1);
                let items = &self.names.items;
                quote! { #items(#receiving.iter().map(|#binding| #each)) }
            }
            Shape::Array(_, inner) => {
                let each = self.written(inner, &quote! { #binding }, depth + 1);
                quote! { #receiving.each_ref().map(|#binding| #each) }
            }
            Shape::Keyed(_, inner) => {
                let each = self.written(inner, &quote! { #binding }, depth + 1);
                let entries = &self.names.entries;
                quote! { #entries(#receiving.iter().map(|(key, #binding)| (key, #each))) }
            }
            Shape::Tuple(positions) => {
                let each: Vec<TokenStream> = positions
                    .iter()
                    .enumerate()
                    .map(|(at, position)| {
                        let index = syn::Index::from(at);
                        self.written(position, &quote! { &#receiving.#index }, depth + 1)
                    })
                    .collect();
                quote! { (#(#each,)*) }
            }
            Shape::Ranged(kind, bound) => {
                let (start_at, end_at) = match kind {
                    RangeKind::HalfOpen => (
                        Some(quote! { &#receiving.start }),
                        Some(quote! { &#receiving.end }),
                    ),
                    RangeKind::Inclusive => (
                        Some(quote! { #receiving.start() }),
                        Some(quote! { #receiving.end() }),
                    ),
                    RangeKind::From => (Some(quote! { &#receiving.start }), None),
                    RangeKind::To => (None, Some(quote! { &#receiving.end })),
                };
                let mut side = |written: Option<TokenStream>| {
                    written.map_or_else(
                        || quote! { ::core::option::Option::<()>::None },
                        |at| {
                            let each = self.written(bound, &at, depth + 1);
                            quote! { ::core::option::Option::Some(#each) }
                        },
                    )
                };
                let (start, end) = (side(start_at), side(end_at));
                let (ranged, name) = (&self.names.ranged, kind.name());
                quote! { #ranged(#name, #start, #end) }
            }
            Shape::Resulted(ok_shape, err_shape) => {
                let ok = self.written(ok_shape, &quote! { #binding }, depth + 1);
                let err = self.written(err_shape, &quote! { #binding }, depth + 1);
                quote! { #receiving.as_ref().map(|#binding| #ok).map_err(|#binding| #err) }
            }
            Shape::Locked(level, lock, inner) => self.locked(level, *lock, inner, held),
        }
    }
}

#[cfg(feature = "serde")]
thread_local! {
    /// The members of the item being expanded whose integers are written as digit strings.
    static MEMBERS: RefCell<Vec<DigitMember>> = const { RefCell::new(Vec::new()) };
    /// The functions [`hang_on_struct`] and [`hang_on_enum`] wrote for the item being expanded.
    static WRITTEN: RefCell<TokenStream> = RefCell::new(TokenStream::new());
    /// The check each member's read holds the value to, by variant and index, as text: a token
    /// outlives no expansion.
    static CHECKED: RefCell<Vec<(Option<String>, usize, String)>> = const { RefCell::new(Vec::new()) };
}

/// Records the members of `item` whose integers are written as digit strings, for the expansion
/// of `item` to read.
#[cfg(feature = "serde")]
pub fn record(item: &Item) {
    let read = members(item);
    MEMBERS.with(|recorded| *recorded.borrow_mut() = read);
    WRITTEN.with(|written| {
        written.borrow_mut().clone_from(&TokenStream::new());
    });
    CHECKED.with(|checked| checked.borrow_mut().clear());
}

/// Records `check`, a closure over a reference to the member declared at `index` in `variant`,
/// for the function that reads the member's digits to call on the value it read: an untagged
/// member's bound, where the member's type names the item's own parameters.
#[cfg(feature = "serde")]
pub fn read_and_check(variant: Option<&str>, index: usize, check: &TokenStream) {
    CHECKED.with(|checked| {
        checked
            .borrow_mut()
            .push((variant.map(ToOwned::to_owned), index, check.to_string()));
    });
}

/// The check [`read_and_check`] recorded for the member declared at `index` in `variant`.
#[cfg(feature = "serde")]
fn checked(variant: Option<&str>, index: usize) -> Option<TokenStream> {
    CHECKED.with(|checked| {
        checked
            .borrow()
            .iter()
            .find(|(held, at, _)| *at == index && held.as_deref() == variant)
            .and_then(|(_, _, check)| check.parse().ok())
    })
}

/// Nothing is recorded without `serde`, which no digit string is written without.
#[cfg(not(feature = "serde"))]
pub const fn record(_item: &Item) {}

/// Whether the member declared at `index`, in `variant` or in a struct where it is `None`, of the
/// item being expanded holds an integer written as a digit string.
#[cfg(feature = "serde")]
pub fn written_as_digits(variant: Option<&str>, index: usize) -> bool {
    recorded(variant, index).is_some()
}

/// No integer is written as a digit string without `serde`.
#[cfg(not(feature = "serde"))]
pub const fn written_as_digits(_variant: Option<&str>, _index: usize) -> bool {
    false
}

/// Whether a member of the struct being expanded holds an integer written as a digit string: the
/// value a brand writes, where the struct is one.
#[cfg(feature = "serde")]
pub fn written_by_the_struct() -> bool {
    MEMBERS.with(|members| {
        members
            .borrow()
            .iter()
            .any(|member| member.variant.is_none())
    })
}

/// No integer is written as a digit string without `serde`.
#[cfg(not(feature = "serde"))]
pub const fn written_by_the_struct() -> bool {
    false
}

/// The recorded member declared at `index` in `variant`.
#[cfg(feature = "serde")]
pub fn recorded(variant: Option<&str>, index: usize) -> Option<DigitMember> {
    MEMBERS.with(|members| {
        members
            .borrow()
            .iter()
            .find(|member| member.index == index && member.variant.as_deref() == variant)
            .cloned()
    })
}

/// The functions written for the item being expanded, taken out of the registry.
#[cfg(feature = "serde")]
pub fn take_written() -> TokenStream {
    WRITTEN.with(RefCell::take)
}

/// Nothing is written without `serde`.
#[cfg(not(feature = "serde"))]
pub fn take_written() -> TokenStream {
    TokenStream::new()
}

/// The type a name declared above as an alias of no parameter stands for, where the integers it
/// holds are written as digit strings.
#[cfg(feature = "serde")]
pub fn alias_written_as_digits(name: &str) -> Option<Type> {
    let target = alias_target(name)?;
    reach(&target, &ItemGenerics::none(), 0).map(|_| target)
}

/// The type `name`, declared above as an alias of no parameter, names.
#[cfg(feature = "serde")]
fn alias_target(name: &str) -> Option<Type> {
    if let Some(Declared::Alias(written)) = declared(name) {
        syn::parse_str(&written).ok()
    } else {
        None
    }
}

/// The members of `item` whose integers the macro writes as digit strings, each with the
/// directions serde writes and reads it in. An item whose derives serde reads none of, or that
/// serde writes or reads through a conversion, has none in that direction.
#[cfg(feature = "serde")]
pub fn members(item: &Item) -> Vec<DigitMember> {
    let (attrs, generics, ident) = if let Item::Struct(item_struct) = item {
        (
            &item_struct.attrs,
            &item_struct.generics,
            &item_struct.ident,
        )
    } else if let Item::Enum(item_enum) = item {
        (&item_enum.attrs, &item_enum.generics, &item_enum.ident)
    } else {
        return Vec::new();
    };
    let conversions = serde_conversions(attrs);
    let converted = |key: &str| conversions.contains(&key) || conversions.contains(&"remote");
    let directions = Directions {
        reads: derives_deserialize(attrs) && !converted("from") && !converted("try_from"),
        writes: derives_serialize(attrs) && !converted("into"),
    };
    if !directions.reads && !directions.writes {
        return Vec::new();
    }
    let Ok(input) = syn::parse2::<DeriveInput>(item.to_token_stream()) else {
        return Vec::new();
    };
    let item_generics = ItemGenerics::of(ident, generics);
    let container = Container {
        bounds: Bounds::of(&input, &item_generics),
        declared: generics,
        generics: item_generics,
        sides: bound_sides(attrs),
    };
    let mut found: Vec<DigitMember> = Vec::new();
    if let Item::Struct(item_struct) = item {
        found.extend(fields_members(
            &item_struct.fields,
            None,
            directions,
            &container,
        ));
    } else if let Item::Enum(item_enum) = item {
        for variant in &item_enum.variants {
            if author_hooked(&variant.attrs) {
                continue;
            }
            let omission = parse_serde_key_omission(&variant.attrs);
            let held = Directions {
                reads: directions.reads && !omission.skips_deserializing,
                writes: directions.writes && !has_serde_skip_serializing(&variant.attrs),
            };
            found.extend(fields_members(
                &variant.fields,
                Some((&variant.ident.to_string(), &variant.attrs)),
                held,
                &container,
            ));
        }
    } else {
        // Only a struct and an enum hold members.
    }
    found
}

/// `item`, the item being expanded, with each member [`record`] holds marked for [`member_def`],
/// which reads the mark on any copy an emitter makes of the member. Nothing is marked without
/// `serde`.
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub fn marked(item: &Item) -> Item {
    let mut marked = item.clone();
    if let Item::Struct(item_struct) = &mut marked {
        mark_fields(None, &mut item_struct.fields);
    } else if let Item::Enum(item_enum) = &mut marked {
        for variant in &mut item_enum.variants {
            let named = variant.ident.to_string();
            mark_fields(Some(&named), &mut variant.fields);
        }
    } else {
        // Only a struct and an enum hold members.
    }
    marked
}

/// Nothing is marked without `serde`.
#[cfg(all(
    not(feature = "serde"),
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub fn marked(item: &Item) -> Item {
    item.clone()
}

/// Marks each member of `fields`, in `variant`, that [`record`] holds.
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
fn mark_fields(variant: Option<&str>, fields: &mut Fields) {
    let mark = Ident::new(MARK, Span::call_site());
    for (index, field) in fields.iter_mut().enumerate() {
        if written_as_digits(variant, index) {
            field.attrs.push(syn::parse_quote! { #[#mark] });
        }
    }
}

/// `def`, the field of the member `field`, with each integer it writes itself read as its digit
/// string where [`marked`] marked the member.
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
pub fn member_def(field: &syn::Field, def: FieldDef) -> FieldDef {
    if field.attrs.iter().any(|attr| attr.path().is_ident(MARK)) {
        def.with_digit_leaves()
    } else {
        def
    }
}

/// The members of `fields` whose integers are written as digit strings, in `variant`, its name
/// and its attributes, or in a struct where it is `None`.
#[cfg(feature = "serde")]
fn fields_members(
    fields: &Fields,
    variant: Option<(&String, &Vec<Attribute>)>,
    held: Directions,
    container: &Container<'_>,
) -> Vec<DigitMember> {
    let variant_sides = variant.map(|(_, attrs)| bound_sides(attrs));
    fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| {
            let directions = member_directions(field, held)?;
            let ty: Type = syn::parse2(naming_the_type(
                field.ty.to_token_stream(),
                &container.generics.named,
            ))
            .ok()?;
            let shape = reach(&ty, &container.generics, 0)?;
            // No value is read into a reference: serde's derive refuses such a member itself.
            let reads = directions.reads && !borrows(&shape);
            if !reads && !directions.writes {
                return None;
            }
            let own = bound_sides(&field.attrs);
            let left_to_inference = |side: fn(&BoundSides) -> bool| {
                !side(&own) && !variant_sides.as_ref().is_some_and(side) && !side(&container.sides)
            };
            let asked = |hooked: bool, bound: syn::Path, side: fn(&BoundSides) -> bool| {
                if hooked && left_to_inference(side) {
                    let predicates = asked_by_type(container.declared, &field.ty, &bound);
                    quote! { #(#predicates),* }.to_string()
                } else {
                    String::new()
                }
            };
            let bound = (
                asked(
                    reads,
                    syn::parse_quote! { ::serde::Deserialize<'de> },
                    |sides| sides.deserialize,
                ),
                asked(
                    directions.writes,
                    syn::parse_quote! { ::serde::Serialize },
                    |sides| sides.serialize,
                ),
            );
            Some(DigitMember {
                bound,
                held: Held { shape },
                index,
                reads,
                scope: Scope::of(&ty, &container.generics, &container.bounds),
                ty,
                variant: variant.map(|(named, _)| named.clone()),
                writes: directions.writes,
            })
        })
        .collect()
}

/// Whether a level of `shape` is a reference.
#[cfg(feature = "serde")]
fn borrows(shape: &Shape) -> bool {
    match shape {
        Shape::Borrowed(_) => true,
        Shape::Digits(_) | Shape::Plain(_) => false,
        Shape::Array(_, held)
        | Shape::Celled(_, held)
        | Shape::Cow(held)
        | Shape::Keyed(_, held)
        | Shape::Listed(_, held)
        | Shape::Locked(_, _, held)
        | Shape::Optional(held)
        | Shape::Owned(_, held)
        | Shape::Ranged(_, held)
        | Shape::Slice(held) => borrows(held),
        Shape::Resulted(ok, err) => borrows(ok) || borrows(err),
        Shape::Tuple(positions) => positions.iter().any(borrows),
    }
}

/// The directions serde writes and reads `field` in through the functions the macro hangs on it,
/// and `None` for a member its author reads or writes through functions of their own, a
/// flattened one, one serde neither writes nor reads, and one under an `as`, whose author states
/// what the surfaces describe.
#[cfg(feature = "serde")]
fn member_directions(field: &Field, container: Directions) -> Option<Directions> {
    if author_hooked(&field.attrs)
        || parse_serde_field_attributes(&field.attrs).flatten
        || parse_model_schema_prop_attributes(&field.attrs)
            .as_type
            .is_some()
    {
        return None;
    }
    let directions = Directions {
        reads: container.reads && !parse_serde_key_omission(&field.attrs).skips_deserializing,
        writes: container.writes && !has_serde_skip_serializing(&field.attrs),
    };
    (directions.reads || directions.writes).then_some(directions)
}

/// Whether `attrs` name a function of the author's that serde writes or reads through.
#[cfg(feature = "serde")]
fn author_hooked(attrs: &[Attribute]) -> bool {
    let hooks = parse_serde_field_hooks(attrs);
    hooks.with.is_some() || hooks.serialize_with.is_some() || hooks.deserialize_with.is_some()
}

/// What `ty` reaches of the integers the macro writes as digit strings, `depth` aliases down.
#[cfg(feature = "serde")]
fn reach(ty: &Type, generics: &ItemGenerics, depth: u8) -> Option<Shape> {
    let written = written_type(ty);
    if let Type::Paren(paren) = written {
        reach(&paren.elem, generics, depth)
    } else if let Type::Path(path) = written
        && path.qself.is_none()
    {
        reach_path(written, path, generics, depth)
    } else if let Type::Array(array) = written {
        reach(&array.elem, generics, depth)
            .map(|held| Shape::Array(written.clone(), Box::new(held)))
    } else if let Type::Slice(slice) = written {
        reach(&slice.elem, generics, depth).map(|held| Shape::Slice(Box::new(held)))
    } else if let Type::Tuple(tuple) = written {
        let positions: Vec<Option<Shape>> = tuple
            .elems
            .iter()
            .map(|elem| reach(elem, generics, depth))
            .collect();
        positions.iter().any(Option::is_some).then(|| {
            Shape::Tuple(
                positions
                    .into_iter()
                    .zip(&tuple.elems)
                    .map(|(position, elem)| position.unwrap_or_else(|| Shape::Plain(elem.clone())))
                    .collect(),
            )
        })
    } else if let Type::Reference(reference) = written {
        reach(&reference.elem, generics, depth).map(|held| Shape::Borrowed(Box::new(held)))
    } else {
        None
    }
}

/// What a path reaches of the integers the macro writes as digit strings: one of the two itself,
/// one a std wrapper, list, set, map, range or `Result` holds, as the field walk reads each of
/// them, and what an alias declared above stands for.
#[cfg(feature = "serde")]
fn reach_path(
    written: &Type,
    path: &TypePath,
    generics: &ItemGenerics,
    depth: u8,
) -> Option<Shape> {
    if let Some(kind) = written_std_range(path) {
        let bound = type_arguments(path).into_iter().next()?;
        return reach(bound, generics, depth).map(|held| Shape::Ranged(kind, Box::new(held)));
    }
    if let Some((ok, err)) = written_std_result(path) {
        let (ok_held, err_held) = (reach(ok, generics, depth), reach(err, generics, depth));
        if ok_held.is_none() && err_held.is_none() {
            return None;
        }
        return Some(Shape::Resulted(
            Box::new(ok_held.unwrap_or_else(|| Shape::Plain(ok.clone()))),
            Box::new(err_held.unwrap_or_else(|| Shape::Plain(err.clone()))),
        ));
    }
    let segment = path.path.segments.last()?;
    let name = segment.ident.to_string();
    let arguments = match &segment.arguments {
        PathArguments::AngleBracketed(_) => type_arguments(path),
        PathArguments::None => {
            return match name.as_str() {
                "u64" => Some(Shape::Digits(Width::Unsigned)),
                "i64" => Some(Shape::Digits(Width::Signed)),
                _ if path.path.segments.len() == 1
                    && !generics.declares(&name)
                    && depth < ALIAS_DEPTH =>
                {
                    reach(
                        &alias_target(&name)?,
                        &ItemGenerics::none(),
                        depth.saturating_add(1),
                    )
                }
                _ => None,
            };
        }
        PathArguments::Parenthesized(_) => return None,
    };
    let level = || written.clone();
    let below = |held: &Type| reach(held, generics, depth);
    match arguments.as_slice() {
        [held] if Guard::named(&name).is_some() => {
            let guard = Guard::named(&name)?;
            below(held).map(|inner| Shape::Locked(level(), guard, Box::new(inner)))
        }
        [held] if name == "Option" => below(held).map(|inner| Shape::Optional(Box::new(inner))),
        [held, ..] if name == "Vec" || is_sequence_wrapper(&name) => {
            below(held).map(|inner| Shape::Listed(level(), Box::new(inner)))
        }
        [_, value, ..] if is_map_wrapper(&name) => {
            below(value).map(|inner| Shape::Keyed(level(), Box::new(inner)))
        }
        [held] if is_ownership_wrapper(&name) && name != "Cow" => {
            below(held).map(|inner| Shape::Owned(level(), Box::new(inner)))
        }
        [held] if name == "Cow" => below(held).map(|inner| Shape::Cow(Box::new(inner))),
        [held] if name == "Cell" => {
            below(held).map(|inner| Shape::Celled(level(), Box::new(inner)))
        }
        _ => None,
    }
}

/// The type arguments the last segment of `path` writes.
#[cfg(feature = "serde")]
fn type_arguments(path: &TypePath) -> Vec<&Type> {
    let Some(segment) = path.path.segments.last() else {
        return Vec::new();
    };
    let PathArguments::AngleBracketed(angled) = &segment.arguments else {
        return Vec::new();
    };
    angled
        .args
        .iter()
        .filter_map(|argument| {
            if let GenericArgument::Type(held) = argument {
                Some(held)
            } else {
                None
            }
        })
        .collect()
}

/// Hangs the functions that write and read each member of `item_struct` whose integers are written
/// as digit strings, recorded for the item being expanded, and keeps the functions for
/// [`take_written`].
#[cfg(feature = "serde")]
pub fn hang_on_struct(item_struct: &mut syn::ItemStruct) {
    let owner = to_snake_case(&item_struct.ident.to_string());
    hang_on_fields(&owner, None, &mut item_struct.fields);
}

/// [`hang_on_struct`] for each variant of an enum.
#[cfg(feature = "serde")]
pub fn hang_on_enum(item_enum: &mut syn::ItemEnum) {
    let owner = to_snake_case(&item_enum.ident.to_string());
    for variant in &mut item_enum.variants {
        let named = variant.ident.to_string();
        hang_on_fields(&owner, Some(&named), &mut variant.fields);
    }
}

/// [`hang_on_struct`] for the members of one struct or variant.
#[cfg(feature = "serde")]
fn hang_on_fields(owner: &str, variant: Option<&str>, fields: &mut Fields) {
    for (index, field) in fields.iter_mut().enumerate() {
        let Some(member) = recorded(variant, index) else {
            continue;
        };
        let member_name = field
            .ident
            .as_ref()
            .map_or_else(|| index.to_string(), ToString::to_string);
        let stem = variant.map_or_else(
            || member_name.clone(),
            |held| format!("{}_{member_name}", to_snake_case(held)),
        );
        let mut functions = TokenStream::new();
        let (read_bound, write_bound) = &member.bound;
        if member.writes {
            let named = format_ident!("{owner}_model_schema_write_{stem}");
            let path = syn::LitStr::new(&named.to_string(), Span::call_site());
            field
                .attrs
                .push(syn::parse_quote! { #[serde(serialize_with = #path)] });
            if !write_bound.is_empty() {
                let listed = syn::LitStr::new(write_bound, Span::call_site());
                field
                    .attrs
                    .push(syn::parse_quote! { #[serde(bound(serialize = #listed))] });
            }
            functions.extend(write_function(
                &named,
                (&member.ty, &member.held),
                &member.scope,
            ));
        }
        // An untagged member's bound hook reads the digit string itself.
        if member.reads
            && parse_serde_field_hooks(&field.attrs)
                .deserialize_with
                .is_none()
        {
            let named = format_ident!("{owner}_model_schema_read_{stem}");
            let path = syn::LitStr::new(&named.to_string(), Span::call_site());
            field
                .attrs
                .push(syn::parse_quote! { #[serde(deserialize_with = #path)] });
            if !read_bound.is_empty() {
                let listed = syn::LitStr::new(read_bound, Span::call_site());
                field
                    .attrs
                    .push(syn::parse_quote! { #[serde(bound(deserialize = #listed))] });
            }
            // serde's derive reads a missing key of an `Option` as `None` only where it calls no
            // function of its author's.
            if field.ident.is_some()
                && optional(&member.held.shape)
                && !has_serde_default(&field.attrs)
            {
                field.attrs.push(syn::parse_quote! { #[serde(default)] });
            }
            functions.extend(read_function(
                &named,
                (&member.ty, &member.held),
                &member.scope,
                checked(variant, index).as_ref(),
            ));
        }
        WRITTEN.with(|written| written.borrow_mut().extend(functions));
    }
}

/// Whether the outermost level of `shape` that serde writes is an `Option`.
#[cfg(feature = "serde")]
fn optional(shape: &Shape) -> bool {
    match shape {
        Shape::Optional(_) => true,
        Shape::Owned(_, held)
        | Shape::Celled(_, held)
        | Shape::Cow(held)
        | Shape::Locked(_, _, held)
        | Shape::Borrowed(held) => optional(held),
        Shape::Digits(_)
        | Shape::Plain(_)
        | Shape::Slice(_)
        | Shape::Listed(_, _)
        | Shape::Array(_, _)
        | Shape::Keyed(_, _)
        | Shape::Ranged(_, _)
        | Shape::Resulted(_, _)
        | Shape::Tuple(_) => false,
    }
}

/// The function serde's derive writes a value of `ty`, held as `held`, through: each integer as
/// its digit string where the serializer is human-readable, inside the newtype
/// [`DIGITS_NEWTYPE`] names, and as the integer where it is not. It declares the parameters of
/// `scope`.
#[cfg(feature = "serde")]
fn write_function(function: &Ident, (ty, held): (&Type, &Held), scope: &Scope) -> TokenStream {
    let names = Names::of(ty);
    let used = Used::of(&held.shape);
    let mut writer = Writer {
        count: 0,
        names: &names,
        scope,
        sites: TokenStream::new(),
    };
    let written = writer.written(&held.shape, &quote! { value }, 1);
    let library = write_library(&names, &used);
    let sites = writer.sites;
    let (serializer, value) = (&names.serializer, &names.value);
    let declared = scope.declared();
    let bounds = scope.written.iter().map(Predicate::tokens);
    // Generic over what serde hands it, which a `&Vec<_>` parameter would be linted for.
    quote! {
        fn #function<#declared #value, #serializer>(held: &#value, serializer: #serializer) -> ::core::result::Result<#serializer::Ok, #serializer::Error>
        where
            #value: ::core::borrow::Borrow<#ty>,
            #serializer: ::serde::Serializer,
            #(#bounds,)*
        {
            #library
            #sites
            let value: &#ty = ::core::borrow::Borrow::borrow(held);
            ::serde::Serialize::serialize(&#written, serializer)
        }
    }
}

/// The types a written function writes through: the digit string of each width `used` holds, the
/// list and the map written from an iterator, and a range written from its bounds.
#[cfg(feature = "serde")]
fn write_library(names: &Names, used: &Used) -> TokenStream {
    let (digits, text, items, entries) = (&names.digits, &names.text, &names.items, &names.entries);
    let mut library = TokenStream::new();
    let widths: Vec<Width> = used.widths().collect();
    if !widths.is_empty() {
        library.extend(quote! {
            struct #digits<N>(N);
            struct #text<N>(N);
        });
    }
    for width in widths {
        let (rust, serialize) = (width.rust(), width.serialize());
        library.extend(quote! {
            impl ::serde::Serialize for #digits<#rust> {
                fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
                where
                    S: ::serde::Serializer,
                {
                    if serializer.is_human_readable() {
                        serializer.serialize_newtype_struct(#DIGITS_NEWTYPE, &#text(self.0))
                    } else {
                        serializer.#serialize(self.0)
                    }
                }
            }
            impl ::serde::Serialize for #text<#rust> {
                fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
                where
                    S: ::serde::Serializer,
                {
                    if serializer.is_human_readable() {
                        serializer.collect_str(&self.0)
                    } else {
                        serializer.#serialize(self.0)
                    }
                }
            }
        });
    }
    if used.items {
        library.extend(quote! {
            struct #items<I>(I);
            impl<I> ::serde::Serialize for #items<I>
            where
                I: ::core::iter::Iterator + ::core::clone::Clone,
                I::Item: ::serde::Serialize,
            {
                fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
                where
                    S: ::serde::Serializer,
                {
                    serializer.collect_seq(self.0.clone())
                }
            }
        });
    }
    if used.entries {
        library.extend(quote! {
            struct #entries<I>(I);
            impl<I, K, V> ::serde::Serialize for #entries<I>
            where
                I: ::core::iter::Iterator<Item = (K, V)> + ::core::clone::Clone,
                K: ::serde::Serialize,
                V: ::serde::Serialize,
            {
                fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
                where
                    S: ::serde::Serializer,
                {
                    serializer.collect_map(self.0.clone())
                }
            }
        });
    }
    if used.ranged {
        library.extend(range_library(&names.ranged));
    }
    library
}

/// The type a range is written through, as serde's own impl writes one: a struct of the bounds
/// it holds.
#[cfg(feature = "serde")]
fn range_library(ranged: &Ident) -> TokenStream {
    quote! {
        struct #ranged<A, B>(&'static str, ::core::option::Option<A>, ::core::option::Option<B>);
        impl<A, B> ::serde::Serialize for #ranged<A, B>
        where
            A: ::serde::Serialize,
            B: ::serde::Serialize,
        {
            fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                let held = match (&self.1, &self.2) {
                    (::core::option::Option::Some(_), ::core::option::Option::Some(_)) => 2,
                    (::core::option::Option::None, ::core::option::Option::None) => 0,
                    (::core::option::Option::Some(_), ::core::option::Option::None)
                    | (::core::option::Option::None, ::core::option::Option::Some(_)) => 1,
                };
                let mut state = ::serde::Serializer::serialize_struct(serializer, self.0, held)?;
                if let ::core::option::Option::Some(start) = &self.1 {
                    ::serde::ser::SerializeStruct::serialize_field(&mut state, "start", start)?;
                }
                if let ::core::option::Option::Some(end) = &self.2 {
                    ::serde::ser::SerializeStruct::serialize_field(&mut state, "end", end)?;
                }
                ::serde::ser::SerializeStruct::end(state)
            }
        }
    }
}

/// `held` as the receiver of a method call: in parentheses where it is more than a name.
#[cfg(feature = "serde")]
fn receiver(held: &TokenStream) -> TokenStream {
    if held.clone().into_iter().count() == 1 {
        held.clone()
    } else {
        quote! { (#held) }
    }
}

/// The function serde's derive reads a value of `ty`, held as `held`, through: each integer from
/// its digit string or from a number. It declares the parameters of `scope`.
#[cfg(feature = "serde")]
fn read_function(
    function: &Ident,
    (ty, held): (&Type, &Held),
    scope: &Scope,
    check: Option<&TokenStream>,
) -> TokenStream {
    let names = Names::of(ty);
    let reader = &names.reader;
    let read = read_expression(held, &quote! { deserializer }, &names);
    let declared = scope.declared();
    let borrowed = &scope.borrowed;
    let bounds = scope.read.iter().map(Predicate::tokens);
    let body = check.map_or_else(
        || quote! { ::core::result::Result::Ok(#read) },
        |held_to| {
            quote! {
                let read: #ty = #read;
                let check = #held_to;
                check(&read).map_err(|violations: ::std::vec::Vec<::std::string::String>| {
                    <#reader::Error as ::serde::de::Error>::custom(violations.join("; "))
                })?;
                ::core::result::Result::Ok(read)
            }
        },
    );
    quote! {
        fn #function<'de, #declared #reader>(deserializer: #reader) -> ::core::result::Result<#ty, #reader::Error>
        where
            #reader: ::serde::Deserializer<'de>,
            #('de: #borrowed,)*
            #(#bounds,)*
        {
            #body
        }
    }
}

/// The block that reads a value of `ty`, held as `shape`, from `deserializer`, each integer from
/// its digit string or from a number, inside a function answering `Result<_, D::Error>`: an
/// untagged member's bound hook reads its value through it.
#[cfg(feature = "serde")]
pub fn read_block(held: &Held, ty: &Type, deserializer: &TokenStream) -> TokenStream {
    read_expression(held, deserializer, &Names::of(ty))
}

/// [`read_block`] with the names its types are declared under.
#[cfg(feature = "serde")]
fn read_expression(held: &Held, deserializer: &TokenStream, names: &Names) -> TokenStream {
    let shape = &held.shape;
    let used = Used::of(shape);
    let digits = &names.digits;
    let mut library = TokenStream::new();
    let widths: Vec<Width> = used.widths().collect();
    if !widths.is_empty() {
        library.extend(quote! { struct #digits<N>(N); });
    }
    for width in widths {
        let (rust, deserialize, expecting) = (width.rust(), width.deserialize(), width.expecting());
        let visit_other = match width {
            Width::Unsigned => quote! {
                fn visit_i64<E>(self, value: i64) -> ::core::result::Result<u64, E>
                where
                    E: ::serde::de::Error,
                {
                    <u64 as ::core::convert::TryFrom<i64>>::try_from(value).map_err(|_| {
                        E::invalid_value(::serde::de::Unexpected::Signed(value), &self)
                    })
                }
            },
            Width::Signed => quote! {
                fn visit_u64<E>(self, value: u64) -> ::core::result::Result<i64, E>
                where
                    E: ::serde::de::Error,
                {
                    <i64 as ::core::convert::TryFrom<u64>>::try_from(value).map_err(|_| {
                        E::invalid_value(::serde::de::Unexpected::Unsigned(value), &self)
                    })
                }
            },
        };
        let (own_visit, own_unexpected) = match width {
            Width::Unsigned => (quote! { visit_u64 }, quote! { u64 }),
            Width::Signed => (quote! { visit_i64 }, quote! { i64 }),
        };
        library.extend(quote! {
            impl<'de> ::serde::Deserialize<'de> for #digits<#rust> {
                fn deserialize<D>(deserializer: D) -> ::core::result::Result<Self, D::Error>
                where
                    D: ::serde::Deserializer<'de>,
                {
                    struct Visit;
                    impl<'de> ::serde::de::Visitor<'de> for Visit {
                        type Value = #rust;
                        fn expecting(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                            formatter.write_str(#expecting)
                        }
                        fn #own_visit<E>(self, value: #own_unexpected) -> ::core::result::Result<#rust, E>
                        where
                            E: ::serde::de::Error,
                        {
                            ::core::result::Result::Ok(value)
                        }
                        #visit_other
                        fn visit_str<E>(self, text: &str) -> ::core::result::Result<#rust, E>
                        where
                            E: ::serde::de::Error,
                        {
                            <#rust as ::core::str::FromStr>::from_str(text).map_err(|_| {
                                E::invalid_value(::serde::de::Unexpected::Str(text), &self)
                            })
                        }
                    }
                    let read = if deserializer.is_human_readable() {
                        deserializer.deserialize_any(Visit)
                    } else {
                        deserializer.#deserialize(Visit)
                    };
                    read.map(#digits)
                }
            }
        });
    }
    let adapter = adapter(shape, names);
    let built = converted(shape, &quote! { held }, 1);
    quote! {
        {
            #library
            let held = <#adapter as ::serde::Deserialize>::deserialize(#deserializer)?;
            #built
        }
    }
}

/// The type serde reads a value of `shape` as: the value's own type, with each integer read as
/// its digit string and each level that holds one read as a `Vec` or as the same map.
#[cfg(feature = "serde")]
fn adapter(shape: &Shape, names: &Names) -> TokenStream {
    match shape {
        Shape::Digits(width) => {
            let (digits, rust) = (&names.digits, width.rust());
            quote! { #digits<#rust> }
        }
        Shape::Plain(ty) => quote! { #ty },
        Shape::Optional(inner) => {
            let held = adapter(inner, names);
            quote! { ::core::option::Option<#held> }
        }
        Shape::Owned(_, inner)
        | Shape::Celled(_, inner)
        | Shape::Cow(inner)
        | Shape::Borrowed(inner)
        | Shape::Locked(_, _, inner) => adapter(inner, names),
        Shape::Slice(inner) | Shape::Listed(_, inner) => {
            let held = adapter(inner, names);
            quote! { ::std::vec::Vec<#held> }
        }
        Shape::Array(level, inner) => {
            let held = adapter(inner, names);
            let length = if let Type::Array(array) = written_type(level) {
                array.len.to_token_stream()
            } else {
                TokenStream::new()
            };
            quote! { [#held; #length] }
        }
        Shape::Keyed(level, inner) => with_value(level, &adapter(inner, names)),
        Shape::Tuple(positions) => {
            let each = positions.iter().map(|position| adapter(position, names));
            quote! { (#(#each,)*) }
        }
        Shape::Ranged(kind, bound) => {
            let held = adapter(bound, names);
            let range = Ident::new(kind.name(), Span::call_site());
            quote! { ::core::ops::#range<#held> }
        }
        Shape::Resulted(ok_shape, err_shape) => {
            let (ok, err) = (adapter(ok_shape, names), adapter(err_shape, names));
            quote! { ::core::result::Result<#ok, #err> }
        }
    }
}

/// `level`, a map, with `value` written in place of the type of its values.
#[cfg(feature = "serde")]
fn with_value(level: &Type, value: &TokenStream) -> TokenStream {
    let mut replaced = written_type(level).clone();
    if let Type::Path(path) = &mut replaced
        && let Some(segment) = path.path.segments.last_mut()
        && let PathArguments::AngleBracketed(angled) = &mut segment.arguments
        && let Some(held) = angled
            .args
            .iter_mut()
            .filter(|argument| matches!(argument, GenericArgument::Type(_)))
            .nth(1)
    {
        *held = GenericArgument::Type(syn::Type::Verbatim(value.clone()));
    }
    replaced.to_token_stream()
}

/// The value of `shape` built from `held`, the value [`adapter`] read. `depth` numbers the
/// closures' bindings.
#[cfg(feature = "serde")]
fn converted(shape: &Shape, held: &TokenStream, depth: usize) -> TokenStream {
    let binding = format_ident!("held_{depth}");
    let below = |inner: &Shape| converted(inner, &quote! { #binding }, depth + 1);
    match shape {
        Shape::Digits(_) => quote! { #held.0 },
        Shape::Plain(_) => held.clone(),
        Shape::Optional(inner) | Shape::Array(_, inner) => {
            let each = below(inner);
            quote! { #held.map(|#binding| #each) }
        }
        Shape::Owned(level, inner)
        | Shape::Celled(level, inner)
        | Shape::Locked(level, _, inner) => {
            let built = converted(inner, held, depth);
            quote! { <#level as ::core::convert::From<_>>::from(#built) }
        }
        Shape::Cow(inner) => {
            let built = converted(inner, held, depth);
            quote! { ::std::borrow::Cow::Owned(#built) }
        }
        Shape::Borrowed(inner) => converted(inner, held, depth),
        Shape::Slice(inner) => {
            let each = below(inner);
            quote! { #held.into_iter().map(|#binding| #each).collect::<::std::vec::Vec<_>>() }
        }
        Shape::Listed(level, inner) => {
            let each = below(inner);
            quote! { #held.into_iter().map(|#binding| #each).collect::<#level>() }
        }
        Shape::Keyed(level, inner) => {
            let each = below(inner);
            quote! { #held.into_iter().map(|(key, #binding)| (key, #each)).collect::<#level>() }
        }
        Shape::Tuple(positions) => {
            let bindings: Vec<Ident> = (0..positions.len())
                .map(|at| format_ident!("held_{depth}_{at}"))
                .collect();
            let each = positions
                .iter()
                .zip(&bindings)
                .map(|(position, bound)| converted(position, &quote! { #bound }, depth + 1));
            quote! {
                {
                    let (#(#bindings,)*) = #held;
                    (#(#each,)*)
                }
            }
        }
        Shape::Ranged(kind, bound) => {
            let side = |at: TokenStream| converted(bound, &at, depth + 1);
            match kind {
                RangeKind::HalfOpen => {
                    let (start, end) = (
                        side(quote! { #binding.start }),
                        side(quote! { #binding.end }),
                    );
                    quote! {
                        {
                            let #binding = #held;
                            ::core::ops::Range { start: #start, end: #end }
                        }
                    }
                }
                RangeKind::Inclusive => {
                    let (start_at, end_at) = (
                        format_ident!("held_{depth}_0"),
                        format_ident!("held_{depth}_1"),
                    );
                    let (start, end) = (side(quote! { #start_at }), side(quote! { #end_at }));
                    quote! {
                        {
                            let (#start_at, #end_at) = #held.into_inner();
                            ::core::ops::RangeInclusive::new(#start, #end)
                        }
                    }
                }
                RangeKind::From => {
                    let start = side(quote! { #held.start });
                    quote! { ::core::ops::RangeFrom { start: #start } }
                }
                RangeKind::To => {
                    let end = side(quote! { #held.end });
                    quote! { ::core::ops::RangeTo { end: #end } }
                }
            }
        }
        Shape::Resulted(ok_shape, err_shape) => {
            let (ok, err) = (below(ok_shape), below(err_shape));
            quote! {
                match #held {
                    ::core::result::Result::Ok(#binding) => ::core::result::Result::Ok(#ok),
                    ::core::result::Result::Err(#binding) => ::core::result::Result::Err(#err),
                }
            }
        }
    }
}

/// What a value of `ty` a service carries bare, where no member holds it, holds of the integers
/// written as digit strings.
#[cfg(feature = "serde")]
pub fn service_held(ty: &Type) -> Option<Held> {
    reach(ty, &ItemGenerics::none(), 0).map(|shape| Held { shape })
}

/// The two types a service's bare value of `ty` travels through, named after its text: the one
/// that holds the value, which serde writes and reads, and the one that borrows it, which serde
/// writes.
#[cfg(feature = "serde")]
pub fn service_wires(ty: &Type) -> (Ident, Ident) {
    // FNV-1a: the same text names the same type in every expansion of the service.
    let hashed = ty
        .to_token_stream()
        .to_string()
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
    (
        format_ident!("DigitsWire{hashed:016x}"),
        format_ident!("DigitsWireOf{hashed:016x}"),
    )
}

/// The types [`service_wires`] names for `ty`, held as `held`, and the functions they write and
/// read the value through.
#[cfg(feature = "serde")]
pub fn service_wire_items(ty: &Type, held: &Held) -> TokenStream {
    let (wire, wire_of) = service_wires(ty);
    let lowered = to_snake_case(&wire.to_string());
    let (write, read) = (
        format_ident!("{lowered}_write"),
        format_ident!("{lowered}_read"),
    );
    let scope = Scope::default();
    let written = write_function(&write, (ty, held), &scope);
    let read_back = read_function(&read, (ty, held), &scope, None);
    let doc = format!(
        "A `{}` a service carries bare, each `u64` and `i64` it holds written as the string of its \
         digits and read from it and from a number alike.",
        ty.to_token_stream()
    );
    let doc_of = format!("[`{wire}`], borrowing the value it writes.");
    quote! {
        #[doc = #doc]
        pub struct #wire(pub #ty);

        impl ::serde::Serialize for #wire {
            fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                #write(&self.0, serializer)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for #wire {
            fn deserialize<D>(deserializer: D) -> ::core::result::Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                #read(deserializer).map(Self)
            }
        }

        #[doc = #doc_of]
        pub struct #wire_of<'held>(pub &'held #ty);

        impl ::serde::Serialize for #wire_of<'_> {
            fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                #write(self.0, serializer)
            }
        }

        #written
        #read_back
    }
}

#[cfg(test)]
#[cfg(feature = "serde")]
mod tests;
