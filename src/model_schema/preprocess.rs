//! `preprocess`: the functions a type and its members name, each called on a payload before the
//! value is read from it.
//!
//! Every call is written beside the type, inside the read the macro writes for it, so the name is
//! read where the type is declared.

use core::cell::RefCell;

use proc_macro2::{Ident, TokenStream};
#[cfg(feature = "serde")]
use quote::quote;
use quote::quote_spanned;
use syn::ext::IdentExt as _;
use syn::{Attribute, Field, Item, ItemType};

use super::checks::located;
#[cfg(feature = "serde")]
use super::judged_read::unclaimed_parameter;
use super::{guard_failure_output, prefixed_guard_message, written_spelling};
use crate::features::model_schema_prop::{FunctionName, parse_model_schema_prop_attributes};
use crate::features::serde::read_bounds::borrows;
use crate::features::serde::{derives_deserialize, parse_serde_key_omission, read_hook};
use crate::utils::to_snake_case;

/// What the item being expanded names: on the type, and on each member serde reads.
#[derive(Clone)]
pub struct Named {
    /// The functions each member names, in the order the members are declared.
    pub members: Vec<OfMember>,
    /// The functions the type names, in the order written: each takes the whole payload.
    pub own: Vec<FunctionName>,
}

/// The functions one member names, in the order written, and the member's place.
#[derive(Clone)]
pub struct OfMember {
    pub functions: Vec<FunctionName>,
    /// The member's place among those of its struct, or of its variant.
    pub index: usize,
    pub variant: Option<String>,
}

thread_local! {
    /// What the item being expanded names, read by the parts of its expansion that are written
    /// apart: its read, and its recovering decode.
    static NAMED: RefCell<Named> = const {
        RefCell::new(Named {
            members: Vec::new(),
            own: Vec::new(),
        })
    };
}

impl Named {
    /// Whether the item names a function, on itself or on a member.
    pub const fn any(&self) -> bool {
        !self.own.is_empty() || !self.members.is_empty()
    }

    /// The functions the member at `index` of `variant`, or of the struct, names.
    pub fn of_member(&self, variant: Option<&str>, index: usize) -> &[FunctionName] {
        self.members
            .iter()
            .find(|member| member.index == index && member.variant.as_deref() == variant)
            .map_or(&[], |member| member.functions.as_slice())
    }

    /// The name Rust calls each function under.
    pub fn rust_names(&self) -> Vec<String> {
        self.members
            .iter()
            .flat_map(|member| &member.functions)
            .chain(&self.own)
            .map(|function| to_snake_case(&function.name))
            .collect()
    }
}

/// The members of `item`, each with its place and the variant that holds it.
fn members(item: &Item) -> Vec<(Option<&syn::Variant>, usize, &Field)> {
    if let Item::Struct(item_struct) = item {
        item_struct
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| (None, index, field))
            .collect()
    } else if let Item::Enum(item_enum) = item {
        item_enum
            .variants
            .iter()
            .flat_map(|variant| {
                variant
                    .fields
                    .iter()
                    .enumerate()
                    .map(move |(index, field)| (Some(variant), index, field))
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// The attributes of `item` itself.
fn container_attrs(item: &Item) -> &[Attribute] {
    if let Item::Struct(item_struct) = item {
        &item_struct.attrs
    } else if let Item::Enum(item_enum) = item {
        &item_enum.attrs
    } else {
        &[]
    }
}

/// Records what `item` names, `own` on the type itself, for the read the macro writes for it. An
/// item that derives no `Deserialize` is never read, and a member serde does not read neither: no
/// call is written for either.
pub fn record(item: &Item, own: &[FunctionName]) {
    let named = if derives_deserialize(container_attrs(item)) {
        Named {
            members: members(item)
                .into_iter()
                .filter(|(_, _, field)| !parse_serde_key_omission(&field.attrs).skips_deserializing)
                .filter_map(|(variant, index, field)| {
                    let functions = parse_model_schema_prop_attributes(&field.attrs).preprocess;
                    (!functions.is_empty()).then(|| OfMember {
                        functions,
                        index,
                        variant: variant.map(|held| held.ident.to_string()),
                    })
                })
                .collect(),
            own: own.to_vec(),
        }
    } else {
        Named {
            members: Vec::new(),
            own: Vec::new(),
        }
    };
    NAMED.with(|held| *held.borrow_mut() = named);
}

/// What the item being expanded names.
pub fn named() -> Named {
    NAMED.with(|held| held.borrow().clone())
}

/// `payload` as `functions` correct it: each called on the answer of the one before, in the order
/// written, under the snake case of its name. The call is located at the name as it is written, so
/// a function nothing declares is reported at the attribute.
///
/// The run below names a function nothing declares. A `compile_fail` doctest asserts only that
/// some error was raised, so the run was compiled standalone as an ordinary test file, and the
/// text under it is the only error it earned, verbatim:
///
/// ```rust,compile_fail
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(preprocess = ["withoutFragile"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Parcel {
///     pub label: String,
/// }
///
/// fn main() {}
/// ```
///
/// ```text
/// error[E0425]: cannot find function `without_fragile` in this scope
///  --> tests/zz_probe.rs:4:30
///   |
/// 4 | #[model_schema(preprocess = ["withoutFragile"])]
///   |                              ^^^^^^^^^^^^^^^^ not found in this scope
///   |
///   = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
pub fn corrected(functions: &[FunctionName], payload: TokenStream) -> TokenStream {
    functions.iter().fold(payload, |held, function| {
        let at = located(function);
        let called = Ident::new(&to_snake_case(&function.name), at);
        quote_spanned! { at => #called(#held) }
    })
}

/// The function a walker reads a member that names `functions` with: it reads the payload, calls
/// each function on it, and reads the member through `read` from what they answer. Neither name
/// it declares is one of the functions.
#[cfg(feature = "serde")]
pub fn corrected_reader(functions: &[FunctionName], read: &TokenStream) -> TokenStream {
    let called: Vec<String> = functions
        .iter()
        .map(|function| to_snake_case(&function.name))
        .collect();
    let (reader, payload) = (
        unclaimed_parameter("reader", &called),
        unclaimed_parameter("payload", &called),
    );
    let answered = corrected(functions, quote! { #payload });
    quote! {
        |#reader| <::serde_json::Value as ::serde::Deserialize>::deserialize(#reader)
            .and_then(|#payload| #read(#answered).map_err(::serde::de::Error::custom))
    }
}

/// How a refusal names the member at `index`: by its name, or by its place, behind its variant.
pub fn member_label(variant: Option<&syn::Variant>, index: usize, field: &Field) -> String {
    let member = field.ident.as_ref().map_or_else(
        || format!("slot {index}"),
        |ident| format!("field `{}`", ident.unraw()),
    );
    variant.map_or_else(
        || member.clone(),
        |held| format!("{member} of variant `{}`", held.ident),
    )
}

/// The functions `named` lists, as a refusal names them.
fn listed(named: &[FunctionName]) -> String {
    named
        .iter()
        .map(|function| format!("`{}`", to_snake_case(&function.name)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The refusal of a member that names a function and is read through one of its author's own.
fn hooked_refusal(label: &str, named: &[FunctionName], key: &str, path: &str) -> String {
    let functions = listed(named);
    format!(
        "{label}: `preprocess` cannot be written beside `#[serde({key} = \"{path}\")]`: serde \
         reads a member through one function, and `#[model_schema()]` reads this one through a \
         function of its own, which calls {functions}. Call {functions} from `{path}`, or take \
         `{key}` off the member."
    )
}

/// The refusal of a function named where what is read borrows from the payload.
fn borrowed_refusal(label: &str, on: &str, field: &Field) -> String {
    let written = written_spelling(&field.ty);
    format!(
        "{label}: `preprocess` cannot be written on {on} that borrows: the member is \
         `{written}`, and a function answers a payload of its own, which nothing read from it \
         can borrow from. Write the member as an owned type, or take the `preprocess` off."
    )
}

/// What keeps a `preprocess` from being called by the read of `item`, which names `own` on the
/// type: a member read through a function of its author's own, and a value that borrows from the
/// payload. Nothing for an item that derives no `Deserialize`, which is never read.
pub fn refusals(item: &Item, own: &[FunctionName]) -> Vec<syn::Error> {
    if !derives_deserialize(container_attrs(item)) {
        return Vec::new();
    }
    let mut refused: Vec<syn::Error> = Vec::new();
    for (variant, index, field) in members(item) {
        if parse_serde_key_omission(&field.attrs).skips_deserializing {
            continue;
        }
        let label = member_label(variant, index, field);
        let of_variant = variant.map(|held| held.attrs.as_slice());
        let named = parse_model_schema_prop_attributes(&field.attrs).preprocess;
        if let Some(first) = named.first() {
            if let Some((key, path)) = read_hook(&field.attrs) {
                refused.push(syn::Error::new(
                    first.written_at,
                    hooked_refusal(&label, &named, key, &path.value()),
                ));
            } else if borrows(field, of_variant) {
                refused.push(syn::Error::new(
                    first.written_at,
                    borrowed_refusal(&label, "a member", field),
                ));
            } else {
                // The member is read through a function the macro writes.
            }
        } else if let Some(first) = own.first()
            && borrows(field, of_variant)
        {
            refused.push(syn::Error::new(
                first.written_at,
                borrowed_refusal(&label, "a type", field),
            ));
        } else {
            // Nothing is named that this member's read would call.
        }
    }
    refused
}

/// The refusal of a `preprocess` written on a type alias: an alias is another name for its
/// target, and has no read of its own to call a function from.
pub fn alias_refusal_output(
    item_type: &ItemType,
    functions: &[FunctionName],
) -> Option<TokenStream> {
    let first = functions.first()?;
    let name = &item_type.ident;
    let target = written_spelling(&item_type.ty);
    let refusal = format!(
        "`preprocess` cannot be written on a type alias: `{name}` is another name for \
         `{target}`, so it has no read of its own to call `{}` from. Write the `preprocess` on \
         `{target}`, or on a field that holds a `{name}`, in `#[model_schema_prop(...)]`.",
        first.name
    );
    let error = syn::Error::new(
        first.written_at,
        prefixed_guard_message(&format!("type `{name}`: {refusal}")),
    )
    .to_compile_error();
    guard_failure_output(item_type, Some(name), &[error])
}

#[cfg(test)]
mod tests;
