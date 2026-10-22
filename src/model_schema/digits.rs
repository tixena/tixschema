//! The string of decimal digits a `u64` and an `i64` travel as on the JSON wire.
//!
//! [`members`] reads which members of an item hold one where the macro writes that string: the
//! value itself, and what an `Option`, a `Box`, an `Rc`, an `Arc`, a `Cow`, a `Cell`, a list, a
//! set, an array, a map's value and a tuple hold of it, and what a lock the member's own type is
//! guards, in a member whose type names no type parameter, lifetime or `Self` of the item. Every
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
use syn::{Attribute, Field, Fields, GenericArgument, PathArguments, Type};

#[cfg(feature = "serde")]
use core::cell::RefCell;
#[cfg(feature = "serde")]
use core::iter::once;

#[cfg(feature = "serde")]
use super::judged_read::unclaimed_parameter;
#[cfg(feature = "serde")]
use crate::features::model_schema_prop::parse_model_schema_prop_attributes;
#[cfg(feature = "serde")]
use crate::features::serde::{
    derives_deserialize, derives_serialize, has_serde_default, has_serde_skip_serializing,
    parse_serde_field_attributes, parse_serde_field_hooks, parse_serde_key_omission,
    serde_conversions,
};
#[cfg(feature = "serde")]
use crate::field_type::{is_map_wrapper, is_ownership_wrapper, is_sequence_wrapper};
#[cfg(feature = "serde")]
use crate::utils::{to_snake_case, written_names, written_type};

/// The name of the newtype a digit string is written inside where the serializer is
/// human-readable, which a writer of rows reads as the integer it holds.
#[cfg(feature = "serde")]
pub const DIGITS_NEWTYPE: &str = "$__tixschema_digits";

/// The attribute [`marked`] puts on a member whose integers are written as digit strings.
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
const MARK: &str = "model_schema_digits";

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
    /// An `Option`.
    Optional(Box<Self>),
    /// A `Box`, an `Rc` or an `Arc`, written as the whole level, of what it holds.
    Owned(Type, Box<Self>),
    /// What holds no such integer, written as its own type writes it.
    Plain(Type),
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

/// A level above a member's shape, which the value is taken through before it is written.
#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
pub enum Top {
    /// A lock.
    Locked(Type, Guard),
    /// A `Box`, an `Rc` or an `Arc`.
    Owned(Type),
}

/// How a member holds the integers the macro writes as digit strings: the levels down to its last
/// lock, outermost first, and the shape below them.
#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
pub struct Held {
    pub shape: Shape,
    pub top: Vec<Top>,
}

/// What a type reaches of the integers the macro writes as digit strings.
#[cfg(feature = "serde")]
enum Reach {
    /// Some, held as the shape says.
    Held(Shape),
    /// Some, under the levels down to a lock: only a member's own type takes one, never what a
    /// list, a map, an `Option` or a tuple holds.
    Locked(Vec<Top>, Shape),
    /// None.
    Plain,
    /// Some, under a level no function the macro writes takes apart: a reference, and a lock
    /// below another level.
    Refused,
}

/// One member whose integers the macro writes as digit strings.
#[cfg(feature = "serde")]
#[derive(Clone, Debug)]
pub struct DigitMember {
    pub held: Held,
    /// The member's place among the ones declared.
    pub index: usize,
    /// Whether serde reads the member.
    pub reads: bool,
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

/// The names the functions written for a member declare, none of them a name its type writes.
#[cfg(feature = "serde")]
struct Names {
    digits: Ident,
    entries: Ident,
    items: Ident,
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

#[cfg(feature = "serde")]
thread_local! {
    /// The members of the item being expanded whose integers are written as digit strings.
    static MEMBERS: RefCell<Vec<DigitMember>> = const { RefCell::new(Vec::new()) };
    /// The functions [`hang_on_struct`] and [`hang_on_enum`] wrote for the item being expanded.
    static WRITTEN: RefCell<TokenStream> = RefCell::new(TokenStream::new());
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

/// The members of `item` whose integers the macro writes as digit strings, each with the
/// directions serde writes and reads it in. An item whose derives serde reads none of, or that
/// serde writes or reads through a conversion, has none in that direction.
#[cfg(feature = "serde")]
pub fn members(item: &Item) -> Vec<DigitMember> {
    let (attrs, generics) = if let Item::Struct(item_struct) = item {
        (&item_struct.attrs, &item_struct.generics)
    } else if let Item::Enum(item_enum) = item {
        (&item_enum.attrs, &item_enum.generics)
    } else {
        return Vec::new();
    };
    let conversions = serde_conversions(attrs);
    let converted = |key: &str| conversions.contains(&key) || conversions.contains(&"remote");
    let container = Directions {
        reads: derives_deserialize(attrs) && !converted("from") && !converted("try_from"),
        writes: derives_serialize(attrs) && !converted("into"),
    };
    if !container.reads && !container.writes {
        return Vec::new();
    }
    let declared: Vec<String> = generics
        .params
        .iter()
        .filter_map(|parameter| match parameter {
            syn::GenericParam::Type(type_param) => Some(type_param.ident.to_string()),
            syn::GenericParam::Const(const_param) => Some(const_param.ident.to_string()),
            syn::GenericParam::Lifetime(_) => None,
        })
        .collect();
    let mut found: Vec<DigitMember> = Vec::new();
    if let Item::Struct(item_struct) = item {
        found.extend(fields_members(
            &item_struct.fields,
            None,
            container,
            &declared,
        ));
    } else if let Item::Enum(item_enum) = item {
        for variant in &item_enum.variants {
            if author_hooked(&variant.attrs) {
                continue;
            }
            let omission = parse_serde_key_omission(&variant.attrs);
            let directions = Directions {
                reads: container.reads && !omission.skips_deserializing,
                writes: container.writes && !has_serde_skip_serializing(&variant.attrs),
            };
            found.extend(fields_members(
                &variant.fields,
                Some(&variant.ident.to_string()),
                directions,
                &declared,
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

/// The members of `fields` whose integers are written as digit strings, in `variant`.
#[cfg(feature = "serde")]
fn fields_members(
    fields: &Fields,
    variant: Option<&str>,
    container: Directions,
    declared: &[String],
) -> Vec<DigitMember> {
    fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| {
            let directions = member_directions(field, container)?;
            let held = match reach(&field.ty) {
                Reach::Held(shape) => Held {
                    shape,
                    top: Vec::new(),
                },
                Reach::Locked(top, shape) => Held { shape, top },
                Reach::Plain | Reach::Refused => return None,
            };
            if names_what_a_hook_cannot(field.ty.to_token_stream(), declared) {
                return None;
            }
            Some(DigitMember {
                held,
                index,
                reads: directions.reads,
                variant: variant.map(ToOwned::to_owned),
                writes: directions.writes,
            })
        })
        .collect()
}

/// The directions serde writes and reads `field` in through the functions the macro hangs on it,
/// and `None` for a member its author reads or writes through functions of their own, a
/// flattened one, one serde neither writes nor reads, and one under a `literal` or an `as`, whose
/// author states what the surfaces describe.
#[cfg(feature = "serde")]
fn member_directions(field: &Field, container: Directions) -> Option<Directions> {
    let stated = parse_model_schema_prop_attributes(&field.attrs);
    if author_hooked(&field.attrs)
        || parse_serde_field_attributes(&field.attrs).flatten
        || stated.literal.is_some()
        || stated.as_type.is_some()
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

/// Whether `tokens` name one of `declared`, `Self` or a lifetime, none of which a function written
/// beside the item can name.
#[cfg(feature = "serde")]
fn names_what_a_hook_cannot(tokens: TokenStream, declared: &[String]) -> bool {
    let mut after_quote = false;
    tokens.into_iter().any(|token| {
        let quoted = after_quote;
        after_quote = false;
        match token {
            TokenTree::Group(group) => names_what_a_hook_cannot(group.stream(), declared),
            TokenTree::Ident(ident) => {
                let name = ident.to_string();
                (quoted && name != "static") || name == "Self" || declared.contains(&name)
            }
            TokenTree::Punct(punct) => {
                after_quote = punct.as_char() == '\'';
                false
            }
            TokenTree::Literal(_) => false,
        }
    })
}

/// What `ty` reaches of the integers the macro writes as digit strings.
#[cfg(feature = "serde")]
fn reach(ty: &Type) -> Reach {
    let written = written_type(ty);
    if let Type::Paren(paren) = written {
        reach(&paren.elem)
    } else if let Type::Path(path) = written
        && path.qself.is_none()
    {
        reach_path(written, &path.path)
    } else if let Type::Array(array) = written {
        wrapped(reach(&array.elem), |held| {
            Shape::Array(written.clone(), Box::new(held))
        })
    } else if let Type::Slice(slice) = written {
        wrapped(reach(&slice.elem), |held| Shape::Slice(Box::new(held)))
    } else if let Type::Tuple(tuple) = written {
        let positions: Vec<Reach> = tuple.elems.iter().map(reach).collect();
        if positions
            .iter()
            .any(|position| matches!(position, Reach::Refused | Reach::Locked(_, _)))
        {
            Reach::Refused
        } else if positions
            .iter()
            .any(|position| matches!(position, Reach::Held(_)))
        {
            Reach::Held(Shape::Tuple(
                positions
                    .into_iter()
                    .zip(&tuple.elems)
                    .map(|(position, elem)| match position {
                        Reach::Held(held) => held,
                        Reach::Plain | Reach::Refused | Reach::Locked(_, _) => {
                            Shape::Plain(elem.clone())
                        }
                    })
                    .collect(),
            ))
        } else {
            Reach::Plain
        }
    } else if let Type::Reference(reference) = written {
        if matches!(reach(&reference.elem), Reach::Plain) {
            Reach::Plain
        } else {
            Reach::Refused
        }
    } else {
        Reach::Plain
    }
}

/// What a path reaches of the integers the macro writes as digit strings: one of the two itself,
/// or one a std wrapper, list, set or map holds, as the field walk reads each of them.
#[cfg(feature = "serde")]
fn reach_path(written: &Type, path: &syn::Path) -> Reach {
    let Some(segment) = path.segments.last() else {
        return Reach::Plain;
    };
    let name = segment.ident.to_string();
    let arguments: Vec<&Type> = match &segment.arguments {
        PathArguments::AngleBracketed(angled) => angled
            .args
            .iter()
            .filter_map(|argument| {
                if let GenericArgument::Type(held) = argument {
                    Some(held)
                } else {
                    None
                }
            })
            .collect(),
        PathArguments::None => {
            return match name.as_str() {
                "u64" => Reach::Held(Shape::Digits(Width::Unsigned)),
                "i64" => Reach::Held(Shape::Digits(Width::Signed)),
                _ => Reach::Plain,
            };
        }
        PathArguments::Parenthesized(_) => return Reach::Plain,
    };
    let level = || written.clone();
    if let Some(lock) = Guard::named(&name)
        && let [held] = arguments.as_slice()
    {
        return match reach(held) {
            Reach::Held(shape) => Reach::Locked(vec![Top::Locked(level(), lock)], shape),
            Reach::Locked(top, shape) => {
                Reach::Locked(once(Top::Locked(level(), lock)).chain(top).collect(), shape)
            }
            Reach::Plain => Reach::Plain,
            Reach::Refused => Reach::Refused,
        };
    }
    match arguments.as_slice() {
        [held] if name == "Option" => {
            wrapped(reach(held), |inner| Shape::Optional(Box::new(inner)))
        }
        [held, ..] if name == "Vec" || is_sequence_wrapper(&name) => {
            wrapped(reach(held), |inner| Shape::Listed(level(), Box::new(inner)))
        }
        [_, value, ..] if is_map_wrapper(&name) => {
            wrapped(reach(value), |inner| Shape::Keyed(level(), Box::new(inner)))
        }
        [held] if is_ownership_wrapper(&name) && name != "Cow" => match reach(held) {
            Reach::Locked(top, shape) => {
                Reach::Locked(once(Top::Owned(level())).chain(top).collect(), shape)
            }
            Reach::Held(shape) => Reach::Held(Shape::Owned(level(), Box::new(shape))),
            Reach::Plain => Reach::Plain,
            Reach::Refused => Reach::Refused,
        },
        [held] if name == "Cow" => wrapped(reach(held), |inner| Shape::Cow(Box::new(inner))),
        [held] if name == "Cell" => {
            wrapped(reach(held), |inner| Shape::Celled(level(), Box::new(inner)))
        }

        _ => Reach::Plain,
    }
}

/// `reached` held one level further down, as `level` holds it.
#[cfg(feature = "serde")]
fn wrapped<F>(reached: Reach, level: F) -> Reach
where
    F: FnOnce(Shape) -> Shape,
{
    match reached {
        Reach::Held(held) => Reach::Held(level(held)),
        Reach::Plain => Reach::Plain,
        Reach::Refused | Reach::Locked(_, _) => Reach::Refused,
    }
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
        if member.writes {
            let named = format_ident!("{owner}_model_schema_write_{stem}");
            let path = syn::LitStr::new(&named.to_string(), Span::call_site());
            field
                .attrs
                .push(syn::parse_quote! { #[serde(serialize_with = #path)] });
            functions.extend(write_function(&named, &field.ty, &member.held));
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
            // serde's derive reads a missing key of an `Option` as `None` only where it calls no
            // function of its author's.
            if field.ident.is_some()
                && optional(&member.held.shape)
                && !has_serde_default(&field.attrs)
            {
                field.attrs.push(syn::parse_quote! { #[serde(default)] });
            }
            functions.extend(read_function(&named, &field.ty, &member.held));
        }
        WRITTEN.with(|written| written.borrow_mut().extend(functions));
    }
}

/// Whether the outermost level of `shape` that serde writes is an `Option`.
#[cfg(feature = "serde")]
fn optional(shape: &Shape) -> bool {
    match shape {
        Shape::Optional(_) => true,
        Shape::Owned(_, held) | Shape::Celled(_, held) | Shape::Cow(held) => optional(held),
        Shape::Digits(_)
        | Shape::Plain(_)
        | Shape::Slice(_)
        | Shape::Listed(_, _)
        | Shape::Array(_, _)
        | Shape::Keyed(_, _)
        | Shape::Tuple(_) => false,
    }
}

/// The function serde's derive writes the member of type `ty` through: each integer as its digit
/// string where the serializer is human-readable, inside the newtype [`DIGITS_NEWTYPE`] names, and
/// as the integer where it is not.
#[cfg(feature = "serde")]
fn write_function(function: &Ident, ty: &Type, held: &Held) -> TokenStream {
    let names = Names::of(ty);
    let used = Used::of(&held.shape);
    let library = write_library(&names, &used);
    let (serializer, value) = (&names.serializer, &names.value);
    let (taken, reached) = taken_through(&held.top, serializer);
    let written = written(&held.shape, &reached, 1, &names);
    // Generic over what serde hands it, which a `&Vec<_>` parameter would be linted for.
    quote! {
        fn #function<#value, #serializer>(held: &#value, serializer: #serializer) -> ::core::result::Result<#serializer::Ok, #serializer::Error>
        where
            #value: ::core::borrow::Borrow<#ty>,
            #serializer: ::serde::Serializer,
        {
            #library
            let value: &#ty = ::core::borrow::Borrow::borrow(held);
            #taken
            ::serde::Serialize::serialize(&#written, serializer)
        }
    }
}

/// The statements that take `value` through the levels of `top`, each lock taken as serde's own
/// impl takes it and refused in its words, and the reference to what the last one guards.
#[cfg(feature = "serde")]
fn taken_through(top: &[Top], serializer: &Ident) -> (TokenStream, TokenStream) {
    let mut held = quote! { value };
    let mut taken = TokenStream::new();
    for (at, level) in top.iter().enumerate() {
        if let Top::Locked(_, lock) = level {
            let (guard, take, refusal) =
                (format_ident!("taken_{at}"), lock.taken(), lock.refusal());
            let receiver = receiver(&held);
            taken.extend(quote! {
                let #guard = match #receiver.#take() {
                    ::core::result::Result::Ok(#guard) => #guard,
                    ::core::result::Result::Err(_) => {
                        return ::core::result::Result::Err(<#serializer::Error as ::serde::ser::Error>::custom(#refusal));
                    }
                };
            });
            held = quote! { &*#guard };
        } else {
            held = quote! { &**#held };
        }
    }
    (taken, held)
}

/// The types a written function writes through: the digit string of each width `used` holds, and
/// the list and the map written from an iterator.
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
    library
}

/// The value serde writes for `held`, a reference to a value of `shape`, with each integer written
/// through the digit string. `depth` numbers the closures' bindings.
#[cfg(feature = "serde")]
fn written(shape: &Shape, held: &TokenStream, depth: usize, names: &Names) -> TokenStream {
    let binding = format_ident!("held_{depth}");
    let below = |inner: &Shape| written(inner, &quote! { #binding }, depth + 1, names);
    let receiving = receiver(held);
    match shape {
        Shape::Digits(_) => {
            let digits = &names.digits;
            quote! { #digits(*#receiving) }
        }
        Shape::Plain(_) => held.clone(),
        Shape::Optional(inner) => {
            let each = below(inner);
            quote! { #receiving.as_ref().map(|#binding| #each) }
        }
        Shape::Owned(_, inner) | Shape::Cow(inner) => {
            written(inner, &quote! { &**#held }, depth, names)
        }
        Shape::Celled(_, inner) => written(inner, &quote! { &#receiving.get() }, depth, names),
        Shape::Slice(inner) | Shape::Listed(_, inner) => {
            let (items, each) = (&names.items, below(inner));
            quote! { #items(#receiving.iter().map(|#binding| #each)) }
        }
        Shape::Array(_, inner) => {
            let each = below(inner);
            quote! { #receiving.each_ref().map(|#binding| #each) }
        }
        Shape::Keyed(_, inner) => {
            let (entries, each) = (&names.entries, below(inner));
            quote! { #entries(#receiving.iter().map(|(key, #binding)| (key, #each))) }
        }
        Shape::Tuple(positions) => {
            let each = positions.iter().enumerate().map(|(at, position)| {
                let index = syn::Index::from(at);
                written(position, &quote! { &#receiving.#index }, depth + 1, names)
            });
            quote! { (#(#each,)*) }
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

/// The function serde's derive reads the member of type `ty` through: each integer from its digit
/// string or from a number.
#[cfg(feature = "serde")]
fn read_function(function: &Ident, ty: &Type, held: &Held) -> TokenStream {
    let names = Names::of(ty);
    let reader = &names.reader;
    let read = read_expression(held, &quote! { deserializer }, &names);
    quote! {
        fn #function<'de, #reader>(deserializer: #reader) -> ::core::result::Result<#ty, #reader::Error>
        where
            #reader: ::serde::Deserializer<'de>,
        {
            ::core::result::Result::Ok(#read)
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
    let built =
        held.top
            .iter()
            .rev()
            .fold(converted(shape, &quote! { held }, 1), |built, above| {
                let (Top::Owned(level) | Top::Locked(level, _)) = above;
                quote! { <#level as ::core::convert::From<_>>::from(#built) }
            });
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
        Shape::Owned(_, inner) | Shape::Celled(_, inner) | Shape::Cow(inner) => {
            adapter(inner, names)
        }
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
        Shape::Owned(level, inner) | Shape::Celled(level, inner) => {
            let built = converted(inner, held, depth);
            quote! { <#level as ::core::convert::From<_>>::from(#built) }
        }
        Shape::Cow(inner) => {
            let built = converted(inner, held, depth);
            quote! { ::std::borrow::Cow::Owned(#built) }
        }
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
    }
}

#[cfg(test)]
#[cfg(feature = "serde")]
mod tests;
