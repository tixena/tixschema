//! Serde integration feature module: attribute parsing and field name transformation.
//!
//! Most of it is gated on the "serde" feature: renaming, tagging, and the guards are all things
//! the feature buys. The omission walk at the top is not — whether a key reaches the serialized
//! object is a fact about the wire under every toggle, and every build emits surfaces that claim
//! to describe that wire.

pub mod read_bounds;

#[cfg(any(
    feature = "zod",
    feature = "jsonschema",
    feature = "dart",
    feature = "swift",
    feature = "kotlin",
    all(feature = "serde", feature = "typescript")
))]
pub mod defaults;

#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
use core::iter::once;

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
use crate::features::model_schema_prop::FunctionName;
#[cfg(feature = "serde")]
use crate::field_type::is_plain_enum;
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
use crate::rename_rule::RenameRule;
#[cfg(feature = "serde")]
use crate::rename_rule::resolve_rename_rule;
use crate::utils::written_type;
#[cfg(feature = "serde")]
use proc_macro2::{Delimiter, TokenTree};
use proc_macro2::{Group, Span};
use syn::ext::IdentExt as _;
use syn::meta::ParseNestedMeta;
use syn::token::Paren;
use syn::{Attribute, Field, Fields, Ident, ItemEnum, ItemStruct, LitStr, Token, Type, Variant};
#[cfg(feature = "serde")]
use syn::{Error, Meta};

#[cfg(feature = "serde")]
const CFG_ATTR_SERDE_REJECTION: &str = "cfg_attr-wrapped serde attribute is invisible to \
     model_schema and will be silently ignored by the generator; write #[serde(...)] \
     unconditionally (serde attrs are inert without the serde derive)";

/// What the name of every read hook generated to name a field in its type's refusal starts with.
/// The recovering decode reads past such a hook, which changes a message and never a verdict.
#[cfg(feature = "serde")]
pub const NAMED_READ_HOOK_PREFIX: &str = "deserialize_named_";

/// The container keys that put a conversion of the author's between an item and what serde reads
/// or writes for it.
const CONVERSION_KEYS: [&str; 4] = ["remote", "from", "try_from", "into"];

/// What a field's serde attributes say about the key it writes. Read in every build, unlike the
/// rest of this module, since one walk answers all three questions together and the guard that
/// polices their combination needs them side by side.
#[derive(Clone, Copy, Debug, Default)]
pub struct SerdeKeyOmission {
    pub defaulted: bool,
    /// A `skip`, `skip_serializing` or `skip_serializing_if` leaves the key out of the output.
    pub omits_key: bool,
    /// A `skip` also stops serde reading the field, which is its own answer to a missing key and
    /// so needs no `default` written beside it. The other two omission spellings still read.
    pub skips_deserializing: bool,
}

impl SerdeKeyOmission {
    /// Whether the attributes take the member out of both of serde's directions at once: nothing
    /// serde writes carries it, and nothing serde reads keeps what a payload put there — the
    /// conjunction, not the word `skip`, since `skip_serializing` + `skip_deserializing` side by
    /// side is that same wire spelled out.
    pub const fn absent_from_wire(self) -> bool {
        self.omits_key && self.skips_deserializing
    }

    /// Whether the attributes take the member out of exactly one of serde's two directions, which
    /// leaves what serde writes and what serde reads two different payloads.
    pub const fn drops_one_direction_only(self) -> bool {
        self.omits_key != self.skips_deserializing
    }
}

/// What a list-form renaming names in each of serde's two directions.
#[cfg(feature = "serde")]
#[derive(Default)]
struct RenameDirections {
    deserialize: Option<String>,
    serialize: Option<String>,
}

/// Metadata for serde attributes applied to a struct or enum.
#[cfg(feature = "serde")]
#[derive(Clone, Debug, Default)]
pub struct SerdeTypeMeta {
    pub cfg_attr_rejection: Option<Error>,
    pub content: Option<String>,
    pub rename_all: Option<String>,
    /// The casing rule the container applies to the members of every struct variant. serde keeps
    /// this apart from `rename_all`, which reaches variant names only.
    pub rename_all_fields: Option<String>,
    pub tag: Option<String>,
    pub untagged: bool,
}

/// The paths a field's serde attributes read and write it through, each as its author wrote it.
#[cfg(feature = "serde")]
#[derive(Default)]
pub struct SerdeFieldHooks {
    pub deserialize_with: Option<LitStr>,
    pub serialize_with: Option<LitStr>,
    pub with: Option<LitStr>,
}

/// Metadata for serde attributes applied to a field. Whether the field's key is omitted is not
/// here: [`parse_serde_key_omission`] answers that in every build, keeping the surfaces from
/// disagreeing across the feature toggle.
#[cfg(feature = "serde")]
#[derive(Clone, Debug, Default)]
pub struct SerdeFieldMeta {
    pub aliases: Vec<String>,
    pub cfg_attr_rejection: Option<Error>,
    pub flatten: bool,
    pub rename: Option<String>,
    pub skip: bool,
}

/// What the renaming keys of a container, a variant or a field name in one of serde's two
/// directions.
#[cfg(feature = "serde")]
#[derive(Default)]
pub struct DirectedRenames {
    pub rename: Option<String>,
    pub rename_all: Option<String>,
    pub rename_all_fields: Option<String>,
}

/// Which of serde's two derives picks the field a `#[serde(transparent)]` struct is the value of.
#[derive(Clone, Copy)]
pub enum Direction {
    /// `Deserialize`: the field it reads that has no `default`.
    Read,
    /// `Serialize`: the field it writes.
    Written,
}

impl Direction {
    /// What a struct is described over: the field serde writes where it derives `Serialize` alone,
    /// and the one it reads otherwise.
    fn of_derives(attrs: &[Attribute]) -> Self {
        if derives_serialize(attrs) && !derives_deserialize(attrs) {
            Self::Written
        } else {
            Self::Read
        }
    }
}

/// Where serde's attributes put one member among those of its container.
struct Placed {
    /// Whether the member's own members join its container's, under no key of its own.
    flattened: bool,
    /// The key a named member is written under. `None` for a slot.
    key: Option<String>,
    /// Whether serde reads the member, which is what gives a slot a position.
    read: bool,
}

/// The form serde writes an enum in, read off its attributes as the schema surfaces read them.
#[cfg(feature = "serde")]
pub enum Tagging {
    Adjacent {
        content: String,
        tag: String,
    },
    External,
    Internal {
        tag: String,
    },
    /// No attribute, and no variant holding a value: the variant's name, as text.
    Plain,
    Untagged,
}

#[cfg(feature = "serde")]
impl Tagging {
    pub fn of(item_enum: &ItemEnum) -> Self {
        let container = parse_serde_type_attributes(&item_enum.attrs);
        if container.untagged {
            return Self::Untagged;
        }
        match (container.tag, container.content) {
            (None, None) if is_plain_enum(item_enum) => Self::Plain,
            (None, None) => Self::External,
            (Some(tag), None) => Self::Internal { tag },
            (tag, Some(content)) => Self::Adjacent {
                content,
                tag: tag.unwrap_or_else(|| "type".to_owned()),
            },
        }
    }
}

/// Walks the serde attributes for exactly the keys [`SerdeKeyOmission`] names, ignoring every
/// other one. Attributes wrapped in a `cfg_attr` are not reached — a predicate a proc macro cannot
/// evaluate is not one this walk may guess at.
pub fn parse_serde_key_omission(attrs: &[Attribute]) -> SerdeKeyOmission {
    let mut omission = SerdeKeyOmission::default();

    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|nested| {
            if nested.path.is_ident("skip") {
                omission.omits_key = true;
                omission.skips_deserializing = true;
            } else if nested.path.is_ident("skip_serializing")
                || nested.path.is_ident("skip_serializing_if")
            {
                omission.omits_key = true;
            } else if nested.path.is_ident("skip_deserializing") {
                omission.skips_deserializing = true;
            } else if nested.path.is_ident("default") {
                omission.defaulted = true;
            } else {
                // Every other serde key is someone else's business.
            }
            consume_unread_value(&nested)?;
            Ok(())
        })
        .unwrap_or_else(|e| {
            log::trace!("Failed to parse serde key-omission attribute: {e}");
        });
    }

    omission
}

/// Whether serde writes and reads the member or the variant through functions of its author's
/// alone: under `with`, or under both `serialize_with` and `deserialize_with`. Read in every build,
/// as [`parse_serde_key_omission`] is.
pub fn written_and_read_by_the_author(attrs: &[Attribute]) -> bool {
    let (mut with, mut serialize_with, mut deserialize_with) = (false, false, false);
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|nested| {
            if nested.path.is_ident("with") {
                with = true;
            } else if nested.path.is_ident("serialize_with") {
                serialize_with = true;
            } else if nested.path.is_ident("deserialize_with") {
                deserialize_with = true;
            } else {
                // Every other serde key is someone else's business.
            }
            consume_unread_value(&nested)?;
            Ok(())
        })
        .unwrap_or_else(|e| {
            log::trace!("Failed to parse serde hook attribute: {e}");
        });
    }
    with || (serialize_with && deserialize_with)
}

/// Consumes the value of a `key = value` or the group of a `key(...)` list the walk had no use
/// for, so an unread value doesn't end the walk early and drop every attribute written after it.
/// The list is consumed whole rather than parsed. `bound(...)` is the one list form worth naming
/// here: it replaces the trait bounds serde's derive writes on its own impls and leaves the JSON
/// byte-identical, so no surface generated from this walk has anything to read out of it.
fn consume_unread_value(nested: &ParseNestedMeta<'_>) -> syn::Result<()> {
    if nested.input.peek(Token![=]) {
        nested.value()?.parse::<syn::Expr>()?;
    } else if nested.input.peek(Paren) {
        nested.input.parse::<Group>()?;
    } else {
        // Neither `= value` nor `(...)`: a bare key like `untagged`, nothing to step over.
    }
    Ok(())
}

/// Whether the field writes a value for itself when its key is missing, in either spelling
/// (`default` and `default = "path"`).
pub fn has_serde_default(attrs: &[Attribute]) -> bool {
    parse_serde_key_omission(attrs).defaulted
}

/// Whether the item is written to be read back at all — whether `Deserialize` is among what it
/// derives, in any spelling of the path.
pub fn derives_deserialize(attrs: &[Attribute]) -> bool {
    derives(attrs, "Deserialize")
}

/// Whether `Serialize` is among what the item derives, in any spelling of the path.
pub fn derives_serialize(attrs: &[Attribute]) -> bool {
    derives(attrs, "Serialize")
}

/// Whether a derive list of the item names a path that ends on `named`.
fn derives(attrs: &[Attribute], named: &str) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("derive"))
        .any(|attr| {
            let mut found = false;
            attr.parse_nested_meta(|nested| {
                if nested
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == named)
                {
                    found = true;
                }
                Ok(())
            })
            .unwrap_or_else(|e| {
                log::trace!("Failed to parse derive list: {e}");
            });
            found
        })
}

/// The conversion of its author's a container reads or writes through, with where it is written:
/// `remote`, `from`, `try_from` or `into`, the first one written.
pub fn serde_conversion(attrs: &[Attribute]) -> Option<(&'static str, Span)> {
    let mut found: Option<(&'static str, Span)> = None;
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|nested| {
            let written = CONVERSION_KEYS
                .into_iter()
                .find(|key| nested.path.is_ident(key));
            if let (Some(key), None) = (written, found) {
                found = Some((
                    key,
                    nested
                        .path
                        .get_ident()
                        .map_or_else(Span::call_site, Ident::span),
                ));
            }
            consume_unread_value(&nested)?;
            Ok(())
        })
        .unwrap_or_else(|e| {
            log::trace!("Failed to parse serde conversion attribute: {e}");
        });
    }
    found
}

/// The function a field is read through, with the key that names it: `deserialize_with`, or
/// `with`, which names the module that holds it. Read in every build: a field takes one.
pub fn read_hook(attrs: &[Attribute]) -> Option<(&'static str, LitStr)> {
    let mut found: Option<(&'static str, LitStr)> = None;
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|nested| {
            let key = ["deserialize_with", "with"]
                .into_iter()
                .find(|key| nested.path.is_ident(key));
            if let Some(named) = key {
                let path: LitStr = nested.value()?.parse()?;
                found.get_or_insert((named, path));
                return Ok(());
            }
            consume_unread_value(&nested)
        })
        .unwrap_or_else(|e| {
            log::trace!("Failed to parse serde read-hook attribute: {e}");
        });
    }
    found
}

/// Whether the field already reads itself through a function of the author's own (`with = "…"` or
/// `deserialize_with = "…"`).
///
/// A field that does is left alone: serde admits one reader per field, so hanging a generated one
/// beside it would replace the author's rather than wrap it.
#[cfg(feature = "serde")]
pub fn has_serde_read_hook(attrs: &[Attribute]) -> bool {
    let mut found = false;
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|nested| {
            if nested.path.is_ident("with") || nested.path.is_ident("deserialize_with") {
                found = true;
            }
            consume_unread_value(&nested)?;
            Ok(())
        })
        .unwrap_or_else(|e| {
            log::trace!("Failed to parse serde read-hook attribute: {e}");
        });
    }
    found
}

/// Whether serde never writes the field: it is under `skip` or `skip_serializing`. One under
/// `skip_serializing_if` is written wherever its predicate lets it be. Read in every build, as
/// [`parse_serde_key_omission`] is: each client and the brand rule read it.
pub fn has_serde_skip_serializing(attrs: &[Attribute]) -> bool {
    let mut found = false;
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|nested| {
            if nested.path.is_ident("skip") || nested.path.is_ident("skip_serializing") {
                found = true;
            }
            consume_unread_value(&nested)?;
            Ok(())
        })
        .unwrap_or_else(|e| {
            log::trace!("Failed to parse serde skip attribute: {e}");
        });
    }
    found
}

/// The key a struct writes its own serde name under, with that name: what `#[serde(tag = "...")]`
/// means on a struct. serde writes the pair ahead of the fields and reads the struct with or
/// without it.
#[cfg(feature = "serde")]
pub fn struct_tag(item_struct: &ItemStruct) -> Option<(String, String)> {
    let key = parse_serde_type_attributes(&item_struct.attrs).tag?;
    let named = parse_serde_field_attributes(&item_struct.attrs)
        .rename
        .unwrap_or_else(|| item_struct.ident.unraw().to_string());
    Some((key, named))
}

/// Without the `serde` feature nothing reads the attribute, so no struct has a tag.
#[cfg(all(
    not(feature = "serde"),
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub const fn struct_tag(_item_struct: &ItemStruct) -> Option<(String, String)> {
    None
}

/// The field a brand is written and read as the value of, or `None` for a struct that is no
/// brand: a `#[serde(transparent)]` tuple struct's one slot, and of any other
/// `#[serde(transparent)]` struct the one [`transparent_field`] finds in the [`Direction`] its
/// derives pick by.
pub fn brand_field(item_struct: &ItemStruct) -> Option<&Field> {
    brand_slot(item_struct).map(|(_, field)| field)
}

/// [`brand_field`] beside its place among the struct's fields.
pub fn brand_slot(item_struct: &ItemStruct) -> Option<(usize, &Field)> {
    if !has_serde_transparent(&item_struct.attrs) {
        return None;
    }
    match &item_struct.fields {
        Fields::Unnamed(slots) if slots.unnamed.len() == 1 => {
            slots.unnamed.first().map(|only| (0, only))
        }
        Fields::Named(_) | Fields::Unnamed(_) => transparent_slot(
            &item_struct.fields,
            Direction::of_derives(&item_struct.attrs),
        ),
        Fields::Unit => None,
    }
}

/// The field serde writes a `#[serde(transparent)]` struct that derives both `Serialize` and
/// `Deserialize` as the value of, then the one it reads it as, each beside its place, where the two
/// are not one field. `None` where they are, and where either derive finds no field, which that
/// derive refuses itself.
pub fn transparent_fields_apart(item_struct: &ItemStruct) -> Option<[(usize, &Field); 2]> {
    if !has_serde_transparent(&item_struct.attrs)
        || !derives_serialize(&item_struct.attrs)
        || !derives_deserialize(&item_struct.attrs)
    {
        return None;
    }
    let written = transparent_slot(&item_struct.fields, Direction::Written)?;
    let read = transparent_slot(&item_struct.fields, Direction::Read)?;
    (written.0 != read.0).then_some([written, read])
}

/// Where serde's attributes put `field`, whose name `rename_all` cases where it has no `rename` of
/// its own.
#[cfg(feature = "serde")]
fn placed(field: &Field, rename_all: Option<&str>) -> Placed {
    let key = field.ident.as_ref().map(|ident| {
        parse_written_renames(&field.attrs)
            .rename
            .unwrap_or_else(|| {
                resolve_rename_rule(rename_all).apply_to_field(&ident.unraw().to_string())
            })
    });
    Placed {
        flattened: parse_serde_field_attributes(&field.attrs).flatten,
        key,
        read: !parse_serde_key_omission(&field.attrs).skips_deserializing,
    }
}

/// Without `serde` no attribute is read: a member is written as it is declared.
#[cfg(not(feature = "serde"))]
fn placed(field: &Field, _rename_all: Option<&str>) -> Placed {
    Placed {
        flattened: false,
        key: field.ident.as_ref().map(|ident| ident.unraw().to_string()),
        read: true,
    }
}

/// The steps serde writes on the way to each member of `fields`, below `prefix`, each beside its
/// place among the ones declared: a named member under its key, a flattened one under none, the
/// only slot under none where `lone_is_the_value`, and any other slot under its position among
/// the slots serde reads. A slot serde does not read has no position.
pub fn member_steps<'item>(
    fields: &'item Fields,
    prefix: &[String],
    rename_all: Option<&str>,
    lone_is_the_value: bool,
) -> Vec<(&'item Field, usize, Vec<String>)> {
    let mut position = 0_usize;
    fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let written = placed(field, rename_all);
            let mut steps = prefix.to_vec();
            if let Some(key) = written.key {
                if !written.flattened {
                    steps.push(key);
                }
            } else if written.read && !lone_is_the_value {
                steps.push(position.to_string());
            } else {
                // The slot is the value serde writes, or one it never does.
            }
            if written.read {
                position = position.saturating_add(1);
            }
            (field, index, steps)
        })
        .collect()
}

/// What serde writes ahead of the members of `variant`: the variant's written name with no tag
/// attribute, the content key under a tag and a content key, and nothing under a tag alone and
/// for an untagged enum.
#[cfg(feature = "serde")]
pub fn variant_steps(item_enum: &ItemEnum, variant: &Variant) -> Vec<String> {
    let tagging = Tagging::of(item_enum);
    if matches!(tagging, Tagging::External) {
        let container = parse_written_renames(&item_enum.attrs);
        vec![
            parse_written_renames(&variant.attrs)
                .rename
                .unwrap_or_else(|| {
                    resolve_rename_rule(container.rename_all.as_deref())
                        .apply_to_variant(&variant.ident.unraw().to_string())
                }),
        ]
    } else if let Tagging::Adjacent { content, tag: _tag } = tagging {
        vec![content]
    } else {
        // serde writes what the variant holds beside the tag, or as the value itself.
        Vec::new()
    }
}

/// Without `serde` no attribute is read: a variant is written under its own name.
#[cfg(not(feature = "serde"))]
pub fn variant_steps(_item_enum: &ItemEnum, variant: &Variant) -> Vec<String> {
    vec![variant.ident.unraw().to_string()]
}

/// The keys serde takes off a holder's object for a struct of `fields`, cased by `rule`, flattened
/// into it: the name each member it reads is read under, and each alias. A struct with a flattened
/// member of its own is read as a map, and one that names a function on itself (`preprocess`) as a
/// value: neither takes a key.
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub fn keys_taken_when_flattened(
    fields: &Fields,
    rule: RenameRule,
    preprocess: &[FunctionName],
) -> Vec<String> {
    let members: Vec<_> = fields
        .iter()
        .map(|field| (field, parse_serde_field_attributes(&field.attrs)))
        .collect();
    if !preprocess.is_empty() || members.iter().any(|(_, meta)| meta.flatten) {
        return Vec::new();
    }
    members
        .into_iter()
        .filter(|(field, _)| !parse_serde_key_omission(&field.attrs).skips_deserializing)
        .filter_map(|(field, meta)| {
            let name = field.ident.as_ref()?.unraw().to_string();
            let read_as = meta.rename.unwrap_or_else(|| rule.apply_to_field(&name));
            Some(once(read_as).chain(meta.aliases))
        })
        .flatten()
        .collect()
}

/// Without `serde` no member is flattened, so none takes a key.
#[cfg(all(
    not(feature = "serde"),
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub const fn keys_taken_when_flattened(
    _fields: &Fields,
    _rule: RenameRule,
    _preprocess: &[FunctionName],
) -> Vec<String> {
    Vec::new()
}

/// The keys serde takes off a holder's object for an enum under a tag and a content key flattened
/// into it, which it reads as a struct of the two: the tag, then the content key. An enum under
/// any other tagging, and one that names a function on itself (`preprocess`), answer none.
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub fn tag_and_content_when_flattened(
    item_enum: &ItemEnum,
    preprocess: &[FunctionName],
) -> Vec<String> {
    if let Tagging::Adjacent { content, tag } = Tagging::of(item_enum)
        && preprocess.is_empty()
    {
        vec![tag, content]
    } else {
        Vec::new()
    }
}

/// Without `serde` no member is flattened, so no enum takes a key.
#[cfg(all(
    not(feature = "serde"),
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub const fn tag_and_content_when_flattened(
    _item_enum: &ItemEnum,
    _preprocess: &[FunctionName],
) -> Vec<String> {
    Vec::new()
}

/// The names serde reads the variant of an externally tagged enum flattened into a holder under,
/// one list per variant it reads: the name the variant is read under, then each alias, which
/// together are its `VARIANTS`. serde takes the first entry of the holder's object, in its order,
/// under one of them. An enum under a tag key, an untagged one and one that names a function on
/// itself (`preprocess`) answer none.
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub fn variant_names_when_flattened(
    item_enum: &ItemEnum,
    preprocess: &[FunctionName],
) -> Vec<Vec<String>> {
    if !preprocess.is_empty()
        || !matches!(Tagging::of(item_enum), Tagging::External | Tagging::Plain)
    {
        return Vec::new();
    }
    let rule = resolve_rename_rule(parse_read_renames(&item_enum.attrs).rename_all.as_deref());
    item_enum
        .variants
        .iter()
        .filter(|variant| !parse_serde_key_omission(&variant.attrs).skips_deserializing)
        .map(|variant| {
            let read_as = parse_read_renames(&variant.attrs)
                .rename
                .unwrap_or_else(|| rule.apply_to_variant(&variant.ident.unraw().to_string()));
            once(read_as)
                .chain(parse_serde_field_attributes(&variant.attrs).aliases)
                .collect()
        })
        .collect()
}

/// Without `serde` no member is flattened, so no enum takes an entry.
#[cfg(all(
    not(feature = "serde"),
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
pub const fn variant_names_when_flattened(
    _item_enum: &ItemEnum,
    _preprocess: &[FunctionName],
) -> Vec<Vec<String>> {
    Vec::new()
}

/// The field serde's derive in `direction` takes a `#[serde(transparent)]` struct as the value of,
/// named or a slot: the one that derive does not skip, that is not written `PhantomData` and, read,
/// that has no `default`. `None` for any other count, which that derive refuses.
pub fn transparent_field(fields: &Fields, direction: Direction) -> Option<&Field> {
    transparent_slot(fields, direction).map(|(_, field)| field)
}

/// [`transparent_field`] beside its place among `fields`.
fn transparent_slot(fields: &Fields, direction: Direction) -> Option<(usize, &Field)> {
    let mut taken = fields.iter().enumerate().filter(|(_, field)| {
        let marker = matches!(
            written_type(&field.ty),
            Type::Path(written)
                if written.path.segments.last().is_some_and(|last| last.ident == "PhantomData")
        );
        !marker
            && match direction {
                Direction::Read => {
                    let omission = parse_serde_key_omission(&field.attrs);
                    !omission.skips_deserializing && !omission.defaulted
                }
                Direction::Written => !has_serde_skip_serializing(&field.attrs),
            }
    });
    if let (Some(only), None) = (taken.next(), taken.next()) {
        Some(only)
    } else {
        None
    }
}

/// Whether the container is `#[serde(transparent)]`, wherever among its serde attributes the key
/// is written.
pub fn has_serde_transparent(attrs: &[Attribute]) -> bool {
    let mut found = false;
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|nested| {
            if nested.path.is_ident("transparent") {
                found = true;
            }
            consume_unread_value(&nested)?;
            Ok(())
        })
        .unwrap_or_else(|e| {
            log::trace!("Failed to parse serde transparent attribute: {e}");
        });
    }
    found
}

/// Reads the `with`, `deserialize_with` and `serialize_with` a field carries, tixschema's own
/// included: the attributes are read as serde's derive will read them.
#[cfg(feature = "serde")]
pub fn parse_serde_field_hooks(attrs: &[Attribute]) -> SerdeFieldHooks {
    let mut hooks = SerdeFieldHooks::default();
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|nested| {
            let slot = if nested.path.is_ident("with") {
                &mut hooks.with
            } else if nested.path.is_ident("deserialize_with") {
                &mut hooks.deserialize_with
            } else if nested.path.is_ident("serialize_with") {
                &mut hooks.serialize_with
            } else {
                return consume_unread_value(&nested);
            };
            *slot = Some(nested.value()?.parse()?);
            Ok(())
        })
        .unwrap_or_else(|e| {
            log::trace!("Failed to parse serde hook attribute: {e}");
        });
    }
    hooks
}

/// Reads the `serialize` and `deserialize` sub-keys out of a list-form renaming, stepping past any
/// other sub-key the same way the outer walk does.
#[cfg(feature = "serde")]
fn parse_rename_directions(nested: &ParseNestedMeta<'_>) -> syn::Result<RenameDirections> {
    let mut directions = RenameDirections::default();
    nested.parse_nested_meta(|inner| {
        if inner.path.is_ident("serialize") {
            directions.serialize = Some(inner.value()?.parse::<LitStr>()?.value());
        } else if inner.path.is_ident("deserialize") {
            directions.deserialize = Some(inner.value()?.parse::<LitStr>()?.value());
        } else {
            consume_unread_value(&inner)?;
        }
        Ok(())
    })?;
    Ok(directions)
}

/// The one name both of serde's directions agree on, or the refusal a pair naming two earns. A
/// direction the list leaves out keeps the name it would otherwise have had, so writing one
/// direction alone splits the two apart as writing two different values does.
#[cfg(feature = "serde")]
fn agreed_rename(
    key: &str,
    path: &syn::Path,
    directions: &RenameDirections,
) -> Result<Option<String>, Error> {
    match (&directions.serialize, &directions.deserialize) {
        (Some(serialize), Some(deserialize)) if serialize == deserialize => {
            Ok(Some(serialize.clone()))
        }
        (Some(serialize), Some(deserialize)) => Err(Error::new_spanned(
            path,
            format!(
                "serde `{key}` names `{serialize}` when serializing and `{deserialize}` when \
                 deserializing, so the payload serde writes is not one serde reads. A schema and \
                 an emitted client describe one key per member, so neither can be written for \
                 the pair. Write both directions at one value, or use the single-value spelling \
                 `{key} = \"...\"`."
            ),
        )),
        (Some(serialize), None) => Err(one_direction_only(key, path, serialize, "serializing")),
        (None, Some(deserialize)) => {
            Err(one_direction_only(key, path, deserialize, "deserializing"))
        }
        (None, None) => Ok(None),
    }
}

/// The refusal a list form writing one of serde's two directions and not the other earns.
#[cfg(feature = "serde")]
fn one_direction_only(key: &str, path: &syn::Path, written: &str, direction: &str) -> Error {
    Error::new_spanned(
        path,
        format!(
            "serde `{key}` names `{written}` when {direction} only, and leaves the other \
             direction at the name it would otherwise use, so the payload serde writes is not one \
             serde reads. A schema and an emitted client describe one key per member, so neither \
             can be written for the pair. Write both directions at one value, or use the \
             single-value spelling `{key} = \"...\"`."
        ),
    )
}

/// The name a renaming key carries, in either spelling: the value of `key = "..."`, or the one name
/// a list form's two directions agree on. A list form the directions disagree over names nothing a
/// surface could render, and is answered by [`rename_direction_rejection`].
#[cfg(feature = "serde")]
fn read_renaming(nested: &ParseNestedMeta<'_>, key: &str) -> syn::Result<Option<String>> {
    if nested.input.peek(Token![=]) {
        let lit: LitStr = nested.value()?.parse()?;
        Ok(Some(lit.value()))
    } else if nested.input.peek(Paren) {
        let directions = parse_rename_directions(nested)?;
        Ok(agreed_rename(key, &nested.path, &directions).ok().flatten())
    } else {
        Ok(None)
    }
}

/// Reads each renaming key for what it names in the direction `side` takes out of a list form. A
/// key written `key = "..."` names that in both.
#[cfg(feature = "serde")]
fn parse_directed_renames(
    attrs: &[Attribute],
    side: fn(RenameDirections) -> Option<String>,
) -> DirectedRenames {
    let mut renames = DirectedRenames::default();

    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|nested| {
            let slot = if nested.path.is_ident("rename") {
                &mut renames.rename
            } else if nested.path.is_ident("rename_all") {
                &mut renames.rename_all
            } else if nested.path.is_ident("rename_all_fields") {
                &mut renames.rename_all_fields
            } else {
                return consume_unread_value(&nested);
            };
            if nested.input.peek(Token![=]) {
                *slot = Some(nested.value()?.parse::<LitStr>()?.value());
            } else if nested.input.peek(Paren)
                && let Some(named) = side(parse_rename_directions(&nested)?)
            {
                *slot = Some(named);
            } else {
                // A list that leaves the direction out leaves its name as it was.
            }
            Ok(())
        })
        .unwrap_or_else(|e| {
            log::trace!("Failed to parse serde rename attribute: {e}");
        });
    }

    renames
}

/// Reads each renaming key for what it names when serde reads, which is the key a value is read
/// from whatever the same key names for writing. A list form counts by its `deserialize` side.
#[cfg(feature = "serde")]
pub fn parse_read_renames(attrs: &[Attribute]) -> DirectedRenames {
    parse_directed_renames(attrs, |directions| directions.deserialize)
}

/// Reads each renaming key for what it names when serde writes, which is the key a stored value
/// is under whatever the same key names for reading. A list form counts by its `serialize` side.
#[cfg(feature = "serde")]
pub fn parse_written_renames(attrs: &[Attribute]) -> DirectedRenames {
    parse_directed_renames(attrs, |directions| directions.serialize)
}

/// The refusal the item's own list-form renaming earns when its two directions do not name one key,
/// or `None` when every renaming written here names exactly one.
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
pub fn rename_direction_rejection(attrs: &[Attribute]) -> Option<Error> {
    let mut rejection: Option<Error> = None;

    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|nested| {
            let renaming = if nested.path.is_ident("rename") {
                Some("rename")
            } else if nested.path.is_ident("rename_all") {
                Some("rename_all")
            } else if nested.path.is_ident("rename_all_fields") {
                Some("rename_all_fields")
            } else {
                None
            };
            if let Some(key) = renaming
                && nested.input.peek(Paren)
            {
                let directions = parse_rename_directions(&nested)?;
                if let Err(refusal) = agreed_rename(key, &nested.path, &directions)
                    && rejection.is_none()
                {
                    rejection = Some(refusal);
                }
                return Ok(());
            }
            consume_unread_value(&nested)?;
            Ok(())
        })
        .unwrap_or_else(|e| {
            log::trace!("Failed to parse serde rename attribute: {e}");
        });
    }

    rejection
}

/// The rejection for a `cfg_attr` carrying a `serde(...)` attribute, or `None` for every other
/// `cfg_attr` — gated derives and gated docs stay legal.
#[cfg(feature = "serde")]
fn cfg_attr_serde_rejection(attr: &Attribute) -> Option<Error> {
    let Meta::List(list) = &attr.meta else {
        return None;
    };
    // Only the top level of the payload is scanned: a `serde` naming the cfg predicate is a string
    // literal, and one naming a nested predicate sits inside a group.
    let mut previous_is_serde = false;
    for token in list.tokens.clone() {
        if previous_is_serde
            && matches!(&token, TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis)
        {
            return Some(Error::new_spanned(attr, CFG_ATTR_SERDE_REJECTION));
        }
        previous_is_serde = matches!(&token, TokenTree::Ident(ident) if ident == "serde");
    }
    None
}

/// Parses serde attributes from a struct or enum.
#[cfg(feature = "serde")]
pub fn parse_serde_type_attributes(attrs: &[Attribute]) -> SerdeTypeMeta {
    let mut meta = SerdeTypeMeta::default();

    for attr in attrs {
        if attr.path().is_ident("cfg_attr") && meta.cfg_attr_rejection.is_none() {
            meta.cfg_attr_rejection = cfg_attr_serde_rejection(attr);
        } else if attr.path().is_ident("serde") {
            attr.parse_nested_meta(|nested| {
                if nested.path.is_ident("tag") {
                    let value = nested.value()?;
                    let lit: LitStr = value.parse()?;
                    meta.tag = Some(lit.value());
                } else if nested.path.is_ident("content") {
                    let value = nested.value()?;
                    let lit: LitStr = value.parse()?;
                    meta.content = Some(lit.value());
                }
                // Handle `rename_all = "value"` and `rename_all(serialize = "...",
                // deserialize = "...")`
                else if nested.path.is_ident("rename_all") {
                    meta.rename_all = read_renaming(&nested, "rename_all")?;
                } else if nested.path.is_ident("rename_all_fields") {
                    meta.rename_all_fields = read_renaming(&nested, "rename_all_fields")?;
                } else if nested.path.is_ident("untagged") {
                    meta.untagged = true;
                } else {
                    // Ignore other serde attributes.
                }
                consume_unread_value(&nested)?;
                Ok(())
            })
            .unwrap_or_else(|e| {
                log::trace!("Failed to parse serde type attribute: {e}");
            });
        } else {
            // Ignore attributes that are neither serde nor a cfg_attr wrapper.
        }
    }

    meta
}

/// Parses serde attributes from a field.
#[cfg(feature = "serde")]
pub fn parse_serde_field_attributes(attrs: &[Attribute]) -> SerdeFieldMeta {
    let mut meta = SerdeFieldMeta::default();

    for attr in attrs {
        if attr.path().is_ident("cfg_attr") && meta.cfg_attr_rejection.is_none() {
            meta.cfg_attr_rejection = cfg_attr_serde_rejection(attr);
        } else if attr.path().is_ident("serde") {
            attr.parse_nested_meta(|nested| {
                // Handle `rename = "value"` and `rename(serialize = "...", deserialize = "...")`
                if nested.path.is_ident("rename") {
                    meta.rename = read_renaming(&nested, "rename")?;
                } else if nested.path.is_ident("skip")
                    || nested.path.is_ident("skip_serializing")
                    || nested.path.is_ident("skip_serializing_if")
                    || nested.path.is_ident("skip_deserializing")
                {
                    meta.skip = true;
                } else if nested.path.is_ident("flatten") {
                    meta.flatten = true;
                }
                // Handle `alias = "value"`, which a field may carry more than once
                else if nested.path.is_ident("alias") {
                    let value: LitStr = nested.value()?.parse()?;
                    meta.aliases.push(value.value());
                    return Ok(());
                } else {
                    // Ignore other serde attributes.
                }
                consume_unread_value(&nested)?;
                Ok(())
            })
            .unwrap_or_else(|e| {
                log::trace!("Failed to parse serde field attribute: {e}");
            });
        } else {
            // Ignore attributes that are neither serde nor a cfg_attr wrapper.
        }
    }

    meta
}

/// Applies serde `rename_all` transformation to a field name.
#[cfg(all(test, feature = "serde"))]
pub fn apply_rename_all(field_name: &str, rename_all: Option<&str>) -> String {
    resolve_rename_rule(rename_all).apply_to_field(field_name)
}

/// Get the final field name after applying serde transformations.
#[cfg(all(test, feature = "serde"))]
pub fn get_final_field_name(
    original_name: &str,
    field_meta: &SerdeFieldMeta,
    type_meta: &SerdeTypeMeta,
) -> String {
    if let Some(rename) = &field_meta.rename {
        return rename.clone();
    }

    // Otherwise apply rename_all transformation
    apply_rename_all(original_name, type_meta.rename_all.as_deref())
}

#[cfg(test)]
mod key_omission_tests;

#[cfg(test)]
mod steps_tests;

#[cfg(test)]
#[cfg(feature = "serde")]
mod tests;
