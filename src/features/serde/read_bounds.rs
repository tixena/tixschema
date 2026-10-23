//! The bounds serde's derive writes on the `Deserialize` impl of an item, read off the item as
//! that derive reads it.
//!
//! The macro writes that impl itself for a type that publishes `validate()`. A bound wider than
//! serde's refuses a type that builds without it, and a narrower one fails where the impl calls
//! the read serde's derive wrote, so each rule here is the derive's own.

use proc_macro2::Span;
use syn::meta::ParseNestedMeta;
use syn::punctuated::Punctuated;
use syn::token::Paren;
use syn::{
    Attribute, Data, DeriveInput, Field, GenericArgument, Generics, Ident, Lifetime, LitStr,
    PathArguments, ReturnType, Token, Type, TypeParamBound, TypePath, WherePredicate, parse_quote,
};

use super::consume_unread_value;
use crate::utils::written_type;

/// What a `borrow` written on a field or on a newtype variant names.
enum Borrow {
    /// Every lifetime the field's type writes.
    All,
    /// The lifetimes its value lists.
    Named(Vec<Lifetime>),
}

/// The `default` a container or a field writes.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Defaulted {
    /// `default`, which builds the value with `Default`.
    Bare,
    /// None written.
    No,
    /// `default = "path"`, which calls the function it names.
    Path,
}

/// What the serde attributes of a container, a variant or a field say of the read.
struct ReadAttrs {
    borrow: Option<Borrow>,
    bound: Option<Vec<WherePredicate>>,
    defaulted: Defaulted,
    /// `deserialize_with` or `with`: a function of its author's reads the value.
    reads_itself: bool,
    skipped: bool,
}

/// What serde's derive writes on the `Deserialize` impl of an item, beside the item's own
/// generics.
pub struct ReadBounds {
    /// The lifetimes `'de` outlives, or `None` where the item is read at `'static`.
    pub borrowed: Option<Vec<Lifetime>>,
    /// The predicates of the impl's `where` clause, after the item's own.
    pub predicates: Vec<WherePredicate>,
    /// Whether the container writes the bounds of the read itself, with `bound`.
    pub written_by_container: bool,
}

/// One field as the derive reads it, under the variant that holds it where one does.
struct ReadField<'item> {
    attrs: ReadAttrs,
    field: &'item Field,
    variant: Option<&'item ReadAttrs>,
}

/// The type parameters of an item that the types of some of its fields name.
struct NamedParameters<'item> {
    /// Field types written as a path that starts at a parameter: `T::Item`.
    associated: Vec<&'item TypePath>,
    declared: Vec<&'item Ident>,
    named: Vec<&'item Ident>,
}

impl ReadAttrs {
    fn of(attrs: &[Attribute]) -> Self {
        let mut read = Self {
            borrow: None,
            bound: None,
            defaulted: Defaulted::No,
            reads_itself: false,
            skipped: false,
        };
        for attr in attrs {
            if !attr.path().is_ident("serde") {
                continue;
            }
            attr.parse_nested_meta(|nested| read.take(&nested))
                .unwrap_or_else(|e| {
                    log::trace!("Failed to parse serde attribute for its bounds: {e}");
                });
        }
        read
    }

    fn take(&mut self, nested: &ParseNestedMeta<'_>) -> syn::Result<()> {
        if nested.path.is_ident("skip") || nested.path.is_ident("skip_deserializing") {
            self.skipped = true;
        } else if nested.path.is_ident("with") || nested.path.is_ident("deserialize_with") {
            self.reads_itself = true;
        } else if nested.path.is_ident("default") {
            self.defaulted = if nested.input.peek(Token![=]) {
                Defaulted::Path
            } else {
                Defaulted::Bare
            };
        } else if nested.path.is_ident("bound") {
            if let Some(written) = bound_side(nested, "deserialize")? {
                self.bound = Some(predicates_of(&written));
            }
            return Ok(());
        } else if nested.path.is_ident("borrow") {
            self.borrow = Some(if nested.input.peek(Token![=]) {
                let written: LitStr = nested.value()?.parse()?;
                Borrow::Named(lifetimes_of(&written))
            } else {
                Borrow::All
            });
            return Ok(());
        } else {
            // Every other key leaves the bounds as they are.
        }
        consume_unread_value(nested)
    }
}

impl<'item> NamedParameters<'item> {
    fn arguments(&mut self, arguments: &'item PathArguments) {
        match arguments {
            PathArguments::None => {}
            PathArguments::AngleBracketed(written) => {
                for argument in &written.args {
                    match argument {
                        GenericArgument::Type(named) => self.ty(named),
                        GenericArgument::AssocType(bound) => self.ty(&bound.ty),
                        GenericArgument::Lifetime(_)
                        | GenericArgument::Const(_)
                        | GenericArgument::AssocConst(_)
                        | GenericArgument::Constraint(_)
                        | _ => {}
                    }
                }
            }
            PathArguments::Parenthesized(written) => {
                for input in &written.inputs {
                    self.ty(&input.ty);
                }
                self.returned(&written.output);
            }
        }
    }

    fn bounds(&mut self, bounds: &'item Punctuated<TypeParamBound, Token![+]>) {
        for bound in bounds {
            if let TypeParamBound::Trait(of_trait) = bound {
                self.path(&of_trait.path);
            }
        }
    }

    fn field(&mut self, field: &'item Field) {
        self.field_type(&field.ty);
    }

    fn field_type(&mut self, ty: &'item Type) {
        if let Type::Path(named) = written_type(ty)
            && named.path.segments.len() > 1
            && let Some(first) = named.path.segments.first()
            && self.declared.contains(&&first.ident)
        {
            self.associated.push(named);
        }
        self.ty(ty);
    }

    fn of(generics: &'item Generics) -> Self {
        Self {
            associated: Vec::new(),
            declared: generics
                .type_params()
                .map(|parameter| &parameter.ident)
                .collect(),
            named: Vec::new(),
        }
    }

    /// `PhantomData<T>` is read and built whatever `T` is, so it names no parameter.
    fn path(&mut self, path: &'item syn::Path) {
        if path
            .segments
            .last()
            .is_some_and(|last| last.ident == "PhantomData")
        {
            return;
        }
        if path.leading_colon.is_none()
            && let (Some(only), 1) = (path.segments.first(), path.segments.len())
            && let Some(declared) = self.declared.iter().find(|held| ***held == only.ident)
            && !self.named.contains(declared)
        {
            self.named.push(declared);
        }
        for segment in &path.segments {
            self.arguments(&segment.arguments);
        }
    }

    /// One predicate holding each parameter named, in the order declared, and each associated
    /// type, to `bound`.
    fn predicates(&self, generics: &'item Generics, bound: &syn::Path) -> Vec<WherePredicate> {
        let parameters = generics
            .type_params()
            .filter(|parameter| self.named.contains(&&parameter.ident))
            .map(|parameter| {
                let named = &parameter.ident;
                parse_quote! { #named: #bound }
            });
        let associated = self
            .associated
            .iter()
            .map(|named| parse_quote! { #named: #bound });
        parameters.chain(associated).collect()
    }

    fn returned(&mut self, returned: &'item ReturnType) {
        if let ReturnType::Type(_, output) = returned {
            self.ty(output);
        }
    }

    fn ty(&mut self, ty: &'item Type) {
        match ty {
            Type::Array(held) => self.ty(&held.elem),
            Type::FnPtr(function) => {
                for input in &function.inputs {
                    self.ty(&input.ty);
                }
                self.returned(&function.output);
            }
            Type::Group(held) => self.ty(&held.elem),
            Type::ImplTrait(held) => self.bounds(&held.bounds),
            Type::Paren(held) => self.ty(&held.elem),
            Type::Path(named) => {
                if let Some(qualified) = &named.qself {
                    self.ty(&qualified.ty);
                }
                self.path(&named.path);
            }
            Type::Ptr(held) => self.ty(&held.elem),
            Type::Reference(held) => self.ty(&held.elem),
            Type::Slice(held) => self.ty(&held.elem),
            Type::TraitObject(held) => self.bounds(&held.bounds),
            Type::Tuple(held) => {
                for position in &held.elems {
                    self.ty(position);
                }
            }
            // A macro's own tokens name no parameter of the item.
            Type::Infer(_) | Type::Macro(_) | Type::Never(_) | Type::Verbatim(_) | _ => {}
        }
    }
}

/// What the serde attributes of a container, a variant or a field say of the write.
struct WriteAttrs {
    bound: Option<Vec<WherePredicate>>,
    skipped: bool,
    /// `serialize_with` or `with`: a function of its author's writes the value.
    writes_itself: bool,
}

impl WriteAttrs {
    fn of(attrs: &[Attribute]) -> Self {
        let mut write = Self {
            bound: None,
            skipped: false,
            writes_itself: false,
        };
        for attr in attrs {
            if !attr.path().is_ident("serde") {
                continue;
            }
            attr.parse_nested_meta(|nested| write.take(&nested))
                .unwrap_or_else(|e| {
                    log::trace!("Failed to parse serde attribute for its bounds: {e}");
                });
        }
        write
    }

    fn take(&mut self, nested: &ParseNestedMeta<'_>) -> syn::Result<()> {
        if nested.path.is_ident("skip") || nested.path.is_ident("skip_serializing") {
            self.skipped = true;
        } else if nested.path.is_ident("with") || nested.path.is_ident("serialize_with") {
            self.writes_itself = true;
        } else if nested.path.is_ident("bound") {
            if let Some(written) = bound_side(nested, "serialize")? {
                self.bound = Some(predicates_of(&written));
            }
            return Ok(());
        } else {
            // Every other key leaves the bounds as they are.
        }
        consume_unread_value(nested)
    }
}

/// Which sides of a `bound` the attributes of a container, a variant or a field write.
#[derive(Clone, Copy, Default)]
pub struct BoundSides {
    pub deserialize: bool,
    pub serialize: bool,
}

/// The `side` of a `bound`, `serialize` or `deserialize`, in either spelling, or `None` for a list
/// that leaves it out.
fn bound_side(nested: &ParseNestedMeta<'_>, side: &str) -> syn::Result<Option<LitStr>> {
    if nested.input.peek(Token![=]) {
        return nested.value()?.parse().map(Some);
    }
    let mut written: Option<LitStr> = None;
    if nested.input.peek(Paren) {
        nested.parse_nested_meta(|listed| {
            let value: LitStr = listed.value()?.parse()?;
            if listed.path.is_ident(side) {
                written = Some(value);
            }
            Ok(())
        })?;
    }
    Ok(written)
}

/// The predicates a `bound` lists, none where its text is no list of predicates: serde's derive
/// refuses that text itself.
fn predicates_of(written: &LitStr) -> Vec<WherePredicate> {
    written
        .parse_with(Punctuated::<WherePredicate, Token![,]>::parse_terminated)
        .map_or_default(|listed| listed.into_iter().collect())
}

/// The lifetimes a `borrow = "'a + 'b"` lists.
fn lifetimes_of(written: &LitStr) -> Vec<Lifetime> {
    written
        .parse_with(Punctuated::<Lifetime, Token![+]>::parse_terminated)
        .map_or_default(|listed| listed.into_iter().collect())
}

/// Every lifetime a type writes that a value of it could borrow for.
pub fn written_lifetimes(ty: &Type, found: &mut Vec<Lifetime>) {
    match ty {
        Type::Array(held) => written_lifetimes(&held.elem, found),
        Type::Group(held) => written_lifetimes(&held.elem, found),
        Type::Paren(held) => written_lifetimes(&held.elem, found),
        Type::Path(named) => {
            if let Some(qualified) = &named.qself {
                written_lifetimes(&qualified.ty, found);
            }
            for segment in &named.path.segments {
                let PathArguments::AngleBracketed(written) = &segment.arguments else {
                    continue;
                };
                for argument in &written.args {
                    match argument {
                        GenericArgument::Lifetime(lifetime) => found.push(lifetime.clone()),
                        GenericArgument::Type(held) => written_lifetimes(held, found),
                        GenericArgument::AssocType(bound) => written_lifetimes(&bound.ty, found),
                        GenericArgument::Const(_)
                        | GenericArgument::AssocConst(_)
                        | GenericArgument::Constraint(_)
                        | _ => {}
                    }
                }
            }
        }
        Type::Ptr(held) => written_lifetimes(&held.elem, found),
        Type::Reference(held) => {
            found.extend(held.lifetime.clone());
            written_lifetimes(&held.elem, found);
        }
        Type::Slice(held) => written_lifetimes(&held.elem, found),
        Type::Tuple(held) => {
            for position in &held.elems {
                written_lifetimes(position, found);
            }
        }
        Type::FnPtr(_)
        | Type::ImplTrait(_)
        | Type::Infer(_)
        | Type::Macro(_)
        | Type::Never(_)
        | Type::TraitObject(_)
        | Type::Verbatim(_)
        | _ => {}
    }
}

/// Whether a type is written as the one path segment `named`, with no argument.
fn is_primitive(ty: &Type, named: &str) -> bool {
    let Type::Path(path) = written_type(ty) else {
        return false;
    };
    path.qself.is_none()
        && path.path.leading_colon.is_none()
        && path.path.segments.len() == 1
        && path
            .path
            .segments
            .first()
            .is_some_and(|only| only.ident == named && only.arguments.is_none())
}

/// Whether a type is `&str` or `&[u8]`, which serde's derive borrows for with no `borrow` written.
fn is_borrowed_text(ty: &Type) -> bool {
    let Type::Reference(reference) = written_type(ty) else {
        return false;
    };
    if reference.mutability.is_some() {
        return false;
    }
    let bytes =
        matches!(written_type(&reference.elem), Type::Slice(of) if is_primitive(&of.elem, "u8"));
    bytes || is_primitive(&reference.elem, "str")
}

/// The one type argument of a path written `wrapper<..>`, where it is written with exactly
/// `count` arguments.
fn last_argument<'ty>(ty: &'ty Type, wrapper: &str, count: usize) -> Option<&'ty Type> {
    let Type::Path(path) = written_type(ty) else {
        return None;
    };
    let last = path.path.segments.last()?;
    let PathArguments::AngleBracketed(written) = &last.arguments else {
        return None;
    };
    if last.ident != wrapper || written.args.len() != count {
        return None;
    }
    match written.args.last()? {
        GenericArgument::Type(held) => Some(held),
        GenericArgument::Lifetime(_)
        | GenericArgument::Const(_)
        | GenericArgument::AssocType(_)
        | GenericArgument::AssocConst(_)
        | GenericArgument::Constraint(_)
        | _ => None,
    }
}

/// Whether a type is `Cow<'a, str>` or `Cow<'a, [u8]>`, which serde's derive reads through a
/// function of its own where a `borrow` is written.
fn is_borrowing_cow(ty: &Type) -> bool {
    last_argument(ty, "Cow", 2).is_some_and(|held| {
        is_primitive(held, "str")
            || matches!(written_type(held), Type::Slice(of) if is_primitive(&of.elem, "u8"))
    })
}

/// The lifetimes the read of a field borrows for: the ones its `borrow` names, its variant's
/// where it writes none, and those of a `&str` or a `&[u8]`, alone or under an `Option`.
fn borrowed_by(read: &ReadField<'_>) -> Vec<Lifetime> {
    let written = read.attrs.borrow.as_ref().or_else(|| {
        let of_variant = read.variant?;
        of_variant.borrow.as_ref()
    });
    let mut found = Vec::new();
    match written {
        Some(Borrow::Named(lifetimes)) => found.extend(lifetimes.iter().cloned()),
        Some(Borrow::All) => written_lifetimes(&read.field.ty, &mut found),
        None => {
            let implicit = is_borrowed_text(&read.field.ty)
                || last_argument(&read.field.ty, "Option", 1).is_some_and(is_borrowed_text);
            if implicit {
                written_lifetimes(&read.field.ty, &mut found);
            }
        }
    }
    found
}

/// Whether the read of `field` borrows from what it is read from, under a variant that carries
/// the attributes `of_variant`.
pub fn borrows(field: &Field, of_variant: Option<&[Attribute]>) -> bool {
    let variant = of_variant.map(ReadAttrs::of);
    !borrowed_by(&ReadField {
        attrs: ReadAttrs::of(&field.attrs),
        field,
        variant: variant.as_ref(),
    })
    .is_empty()
}

/// Whether the derive reads the field's type with its `Deserialize`, and so asks one of each
/// parameter it names.
fn asks_deserialize(read: &ReadField<'_>, borrows: bool) -> bool {
    let of_variant = read
        .variant
        .is_none_or(|held| !held.skipped && !held.reads_itself && held.bound.is_none());
    // A borrowing `Cow` is read by a function of serde's own.
    let reads_itself = read.attrs.reads_itself || (borrows && is_borrowing_cow(&read.field.ty));
    !read.attrs.skipped && !reads_itself && read.attrs.bound.is_none() && of_variant
}

/// Every field of the item, with what its attributes and its variant's say of the read.
fn read_fields<'item>(
    input: &'item DeriveInput,
    variants: &'item [ReadAttrs],
) -> Vec<ReadField<'item>> {
    match &input.data {
        Data::Struct(held) => held
            .fields
            .iter()
            .map(|field| ReadField {
                attrs: ReadAttrs::of(&field.attrs),
                field,
                variant: None,
            })
            .collect(),
        Data::Enum(held) => held
            .variants
            .iter()
            .zip(variants)
            .flat_map(|(variant, attrs)| {
                variant.fields.iter().map(move |field| ReadField {
                    attrs: ReadAttrs::of(&field.attrs),
                    field,
                    variant: Some(attrs),
                })
            })
            .collect(),
        Data::Union(_) => Vec::new(),
    }
}

/// The bounds serde's derive writes on the `Deserialize` impl of `input`.
pub fn read_bounds(input: &DeriveInput) -> ReadBounds {
    let container = ReadAttrs::of(&input.attrs);
    let variants: Vec<ReadAttrs> = match &input.data {
        Data::Enum(held) => held
            .variants
            .iter()
            .map(|variant| ReadAttrs::of(&variant.attrs))
            .collect(),
        Data::Struct(_) | Data::Union(_) => Vec::new(),
    };
    let fields = read_fields(input, &variants);

    let mut borrowed: Vec<Lifetime> = Vec::new();
    let mut asked = NamedParameters::of(&input.generics);
    let mut defaulted = NamedParameters::of(&input.generics);
    let mut predicates: Vec<WherePredicate> = Vec::new();
    for read in &fields {
        let lifetimes = if read.attrs.skipped {
            Vec::new()
        } else {
            borrowed_by(read)
        };
        if asks_deserialize(read, !lifetimes.is_empty()) {
            asked.field(read.field);
        }
        // A skipped field is built with `Default` where neither it nor its container names a
        // function for it.
        let built_by_default = read.attrs.defaulted == Defaulted::Bare
            || (read.attrs.skipped
                && read.attrs.defaulted == Defaulted::No
                && container.defaulted == Defaulted::No);
        if built_by_default {
            defaulted.field(read.field);
        }
        for lifetime in lifetimes {
            if !borrowed.contains(&lifetime) {
                borrowed.push(lifetime);
            }
        }
        predicates.extend(read.attrs.bound.iter().flatten().cloned());
    }
    predicates.extend(
        variants
            .iter()
            .filter_map(|held| held.bound.clone())
            .flatten(),
    );

    let read_at_static = borrowed.iter().any(|lifetime| lifetime.ident == "static");
    let de = Lifetime::new(
        if read_at_static { "'static" } else { "'de" },
        Span::call_site(),
    );
    let written_by_container = container.bound.is_some();
    if let Some(written) = container.bound {
        predicates.extend(written);
    } else {
        if container.defaulted == Defaulted::Bare {
            predicates.push(parse_quote! { Self: ::core::default::Default });
        }
        predicates
            .extend(asked.predicates(&input.generics, &parse_quote! { serde::Deserialize<#de> }));
        predicates.extend(
            defaulted.predicates(&input.generics, &parse_quote! { ::core::default::Default }),
        );
    }
    ReadBounds {
        borrowed: (!read_at_static).then_some(borrowed),
        predicates,
        written_by_container,
    }
}

/// The predicates serde's derive writes on the `Serialize` impl of `input`, after the item's own:
/// the `bound` of each field and variant, and the container's `bound`, or each parameter a written
/// field names held to `Serialize` where the container writes none.
pub fn write_bounds(input: &DeriveInput) -> Vec<WherePredicate> {
    let container = WriteAttrs::of(&input.attrs);
    let mut asked = NamedParameters::of(&input.generics);
    let mut predicates: Vec<WherePredicate> = Vec::new();
    let fields: Vec<(Option<WriteAttrs>, &Field)> = match &input.data {
        Data::Struct(held) => held.fields.iter().map(|field| (None, field)).collect(),
        Data::Enum(held) => held
            .variants
            .iter()
            .flat_map(|variant| {
                variant
                    .fields
                    .iter()
                    .map(|field| (Some(WriteAttrs::of(&variant.attrs)), field))
            })
            .collect(),
        Data::Union(_) => Vec::new(),
    };
    for (variant, field) in fields {
        let attrs = WriteAttrs::of(&field.attrs);
        let of_variant = variant
            .as_ref()
            .is_none_or(|held| !held.skipped && !held.writes_itself && held.bound.is_none());
        if !attrs.skipped && !attrs.writes_itself && attrs.bound.is_none() && of_variant {
            asked.field(field);
        }
        predicates.extend(attrs.bound.into_iter().flatten());
    }
    if let Data::Enum(held) = &input.data {
        for variant in &held.variants {
            predicates.extend(WriteAttrs::of(&variant.attrs).bound.into_iter().flatten());
        }
    }
    if let Some(written) = container.bound {
        predicates.extend(written);
    } else {
        predicates.extend(asked.predicates(&input.generics, &parse_quote! { serde::Serialize }));
    }
    predicates
}

/// The sides of a `bound` that `attrs` write.
pub fn bound_sides(attrs: &[Attribute]) -> BoundSides {
    BoundSides {
        deserialize: ReadAttrs::of(attrs).bound.is_some(),
        serialize: WriteAttrs::of(attrs).bound.is_some(),
    }
}

/// The predicates serde's derive asks of a field of type `ty` it reads or writes itself: each
/// parameter of `generics` the type names, outside `PhantomData`, and each associated type it
/// starts at, held to `bound`.
pub fn asked_by_type(generics: &Generics, ty: &Type, bound: &syn::Path) -> Vec<WherePredicate> {
    let mut asked = NamedParameters::of(generics);
    asked.field_type(ty);
    asked.predicates(generics, bound)
}

#[cfg(test)]
mod tests;
