//! Serde integration feature module: attribute parsing and field name transformation.
//!
//! Most of it is gated on the "serde" feature: renaming, tagging, and the guards are all things
//! the feature buys. The omission walk at the top is not — whether a key reaches the serialized
//! object is a fact about the wire under every toggle, and every build emits surfaces that claim
//! to describe that wire.

#[cfg(all(test, feature = "serde"))]
use crate::rename_rule::resolve_rename_rule;
use crate::utils::written_type;
use proc_macro2::Group;
#[cfg(feature = "serde")]
use proc_macro2::{Delimiter, TokenTree};
#[cfg(feature = "serde")]
use syn::ext::IdentExt as _;
use syn::meta::ParseNestedMeta;
use syn::token::Paren;
use syn::{Attribute, Field, Fields, ItemStruct, Token, Type};
#[cfg(feature = "serde")]
use syn::{Error, LitStr, Meta};

#[cfg(feature = "serde")]
const CFG_ATTR_SERDE_REJECTION: &str = "cfg_attr-wrapped serde attribute is invisible to \
     model_schema and will be silently ignored by the generator; write #[serde(...)] \
     unconditionally (serde attrs are inert without the serde derive)";

/// What the name of every read hook generated to name a field in its type's refusal starts with.
/// The recovering decode reads past such a hook, which changes a message and never a verdict.
#[cfg(feature = "serde")]
pub const NAMED_READ_HOOK_PREFIX: &str = "deserialize_named_";

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
#[cfg(feature = "serde")]
pub fn derives_deserialize(attrs: &[Attribute]) -> bool {
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
                    .is_some_and(|segment| segment.ident == "Deserialize")
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
/// `skip_serializing_if` is written wherever its predicate lets it be.
#[cfg(feature = "serde")]
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
/// brand: a `#[serde(transparent)]` tuple struct's one slot, and of a `#[serde(transparent)]`
/// struct with named fields the one [`transparent_field`] finds.
pub fn brand_field(item_struct: &ItemStruct) -> Option<&Field> {
    if !has_serde_transparent(&item_struct.attrs) {
        return None;
    }
    match &item_struct.fields {
        Fields::Named(_) => transparent_field(&item_struct.fields),
        Fields::Unnamed(slots) if slots.unnamed.len() == 1 => slots.unnamed.first(),
        Fields::Unnamed(_) | Fields::Unit => None,
    }
}

/// The field serde's derive reads a `#[serde(transparent)]` struct as the value of, named or a
/// slot: the one it reads that has no `default` and is not written `PhantomData`. `None` for any
/// other count, which that derive refuses.
pub fn transparent_field(fields: &Fields) -> Option<&Field> {
    let mut read = fields.iter().filter(|field| {
        let omission = parse_serde_key_omission(&field.attrs);
        let marker = matches!(
            written_type(&field.ty),
            Type::Path(written)
                if written.path.segments.last().is_some_and(|last| last.ident == "PhantomData")
        );
        !omission.skips_deserializing && !omission.defaulted && !marker
    });
    if let (Some(only), None) = (read.next(), read.next()) {
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
                 deserializing, so the payload serde writes is not one serde reads. A schema \
                 describes one key per member, so no schema can be written for the pair. Write \
                 both directions at one value, or use the single-value spelling `{key} = \"...\"`."
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
             serde reads. A schema describes one key per member, so no schema can be written for \
             the pair. Write both directions at one value, or use the single-value spelling \
             `{key} = \"...\"`."
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
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
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
#[cfg(feature = "serde")]
mod tests;
