//! `check`: the calls of the functions a type and its members name, as `validate()` writes them.
//!
//! Every call is written beside the type, in the type's own `impl`, so the name is read where the
//! type is declared. A type declared inside a function body names a function declared there, which
//! no path from another module reaches.

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{Field, ItemType};

use super::{
    MemberPlace, guard_failure_output, nested_under_fn, prefixed_guard_message, sole_type_argument,
    wrap_binding, written_spelling,
};
use crate::features::model_schema_prop::{CheckName, parse_model_schema_prop_attributes};
use crate::field_type::is_ownership_wrapper;
use crate::utils::{to_snake_case, written_type};

/// One step from a member to the value its check takes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckStep {
    /// An ownership wrapper or a reference, which derefs to the value.
    Held,
    /// An `Option`, which holds the value or nothing to check.
    Present,
}

/// The checks an item names, as its `validate()` is written from them.
pub struct ItemChecks<'args> {
    /// Whether the item names a check, on itself or on a member. Each bound of such an item imports
    /// the schema module for itself, so a check is called where that import is not in scope.
    pub named: bool,
    /// The checks written on the type, in the order written.
    pub own: &'args [CheckName],
}

impl<'args> ItemChecks<'args> {
    /// The checks `own` written on a type, beside whether one of its `members` names any.
    pub fn of<'member, M>(own: &'args [CheckName], mut members: M) -> Self
    where
        M: Iterator<Item = &'member Field>,
    {
        Self {
            named: !own.is_empty() || members.any(names_a_check),
            own,
        }
    }

    /// The calls of the type's own checks, each on the whole value, its messages as it writes them.
    pub fn own_calls(&self, sink: &str) -> Vec<TokenStream> {
        self.own
            .iter()
            .map(|check| {
                let at = located(check);
                call(check, &quote_spanned! { at => self }, None, sink)
            })
            .collect()
    }
}

/// Whether `member` carries a `check` in its `#[model_schema_prop(...)]`.
fn names_a_check(member: &Field) -> bool {
    !parse_model_schema_prop_attributes(&member.attrs)
        .check
        .is_empty()
}

/// Where a check's name is written, with the macro's own hygiene: what the compiler says of the
/// call is said at the attribute, and a consumer's lints judge the call as generated.
fn located(check: &CheckName) -> Span {
    check.written_at.resolved_at(Span::call_site())
}

/// The call of one check on `subject`, the snake case of its name as Rust writes a function's.
/// `sink` takes each message it answers, under `reported` where the subject has a name.
///
/// The run below names a function nothing declares. A `compile_fail` doctest asserts only that
/// some error was raised, so each run here was compiled standalone as an ordinary test file, and
/// the text under it is the only error it earned, verbatim:
///
/// ```rust,compile_fail
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(check = ["withinParcel"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Parcel {
///     pub label: String,
/// }
///
/// fn main() {}
/// ```
///
/// ```text
/// error[E0425]: cannot find function `within_parcel` in this scope
///  --> tests/zz_probe.rs:4:25
///   |
/// 4 | #[model_schema(check = ["withinParcel"])]
///   |                         ^^^^^^^^^^^^^^ help: try calling `within_parcel` as a method: `self.within_parcel()`
///   |
///   = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
///
/// The run below asks `PartialEq` of a parameter the type declares no bound on:
///
/// ```rust,compile_fail
/// use serde::{Deserialize, Serialize};
/// use tixschema::model_schema;
///
/// #[model_schema(default_types(CodeType = String), check = ["withoutRepeats"])]
/// #[derive(Debug, Deserialize, Serialize)]
/// pub struct Crate<CodeType> {
///     pub codes: Vec<CodeType>,
/// }
///
/// fn without_repeats<CodeType>(value: &Crate<CodeType>) -> Result<(), Vec<String>>
/// where
///     CodeType: PartialEq,
/// {
///     for (index, code) in value.codes.iter().enumerate() {
///         if value.codes[..index].contains(code) {
///             return Err(vec![format!("'codes.{index}': a code is listed once")]);
///         }
///     }
///     Ok(())
/// }
///
/// fn main() {}
/// ```
///
/// ```text
/// error[E0277]: can't compare `CodeType` with `CodeType`
///   --> tests/zz_probe.rs:4:59
///    |
///  4 | #[model_schema(default_types(CodeType = String), check = ["withoutRepeats"])]
///    |                                                           ^^^^^^^^^^^^^^^^ no implementation for `CodeType == CodeType`
///    |
/// note: required by a bound in `without_repeats`
///   --> tests/zz_probe.rs:12:15
///    |
/// 10 | fn without_repeats<CodeType>(value: &Crate<CodeType>) -> Result<(), Vec<String>>
///    |    --------------- required by a bound in this function
/// 11 | where
/// 12 |     CodeType: PartialEq,
///    |               ^^^^^^^^^ required by this bound in `without_repeats`
///    = note: this error originates in the attribute macro `model_schema` (in Nightly builds, run with -Z macro-backtrace for more info)
/// help: consider restricting type parameter `CodeType` with trait `PartialEq`
///    |
///  6 | pub struct Crate<CodeType: std::cmp::PartialEq> {
///    |                          +++++++++++++++++++++
///
/// error: could not compile `tixschema` (test "zz_probe") due to 1 previous error
/// ```
pub fn call(
    check: &CheckName,
    subject: &TokenStream,
    reported: Option<&str>,
    sink: &str,
) -> TokenStream {
    let at = located(check);
    let function = Ident::new(&to_snake_case(&check.name), at);
    let taken = Ident::new(sink, at);
    let Some(named) = reported else {
        return quote_spanned! { at =>
            if let ::core::result::Result::Err(reported) = #function(#subject) {
                #taken.extend(reported);
            }
        };
    };
    let naming = nested_under_fn();
    quote_spanned! { at =>
        if let ::core::result::Result::Err(reported) = #function(#subject) {
            #naming
            #taken.extend(reported.iter().map(|violation| nested_under(#named, violation)));
        }
    }
}

/// The steps from a member written as `ty` to the value its check takes, outermost first: the
/// reach a bound has through `Option` and the transparent wrappers, ending on whatever the member
/// holds under them, a list among them.
pub fn reach(ty: &syn::Type) -> Vec<CheckStep> {
    let mut steps = Vec::new();
    let mut current = ty;
    loop {
        current = written_type(current);
        if let syn::Type::Reference(borrowed) = current {
            steps.push(CheckStep::Held);
            current = &borrowed.elem;
            continue;
        }
        let syn::Type::Path(named) = current else {
            return steps;
        };
        let Some(last) = named.path.segments.last() else {
            return steps;
        };
        let syn::PathArguments::AngleBracketed(held) = &last.arguments else {
            return steps;
        };
        let wrapper = last.ident.to_string();
        let step = if wrapper == "Option" {
            CheckStep::Present
        } else if is_ownership_wrapper(&wrapper) {
            CheckStep::Held
        } else {
            return steps;
        };
        let Some(inner) = sole_type_argument(held) else {
            return steps;
        };
        steps.push(step);
        current = inner;
    }
}

/// `leaf` at the end of `steps`, read from `value`: each step binds what it reaches under the next
/// depth's name, and `leaf` reads the last.
fn reached(steps: &[CheckStep], value: &Ident, depth: usize, leaf: &TokenStream) -> TokenStream {
    let Some((step, rest)) = steps.split_first() else {
        return leaf.clone();
    };
    let next = wrap_binding(depth);
    let inner = reached(rest, &next, depth.saturating_add(1), leaf);
    match step {
        CheckStep::Held => quote! {
            let #next = &**#value;
            #inner
        },
        CheckStep::Present => quote! {
            if let ::core::option::Option::Some(#next) = #value {
                #inner
            }
        },
    }
}

/// What the checks a member names add to `validate()`: one call per check, in the order written,
/// on the member's own value, or `None` for a member that names none.
pub fn member_calls(
    place: &MemberPlace,
    ty: &syn::Type,
    checks: &[CheckName],
    sink: &str,
) -> Option<TokenStream> {
    if checks.is_empty() {
        return None;
    }
    let steps = reach(ty);
    let reported = place.reported.as_deref();
    if steps.is_empty() {
        let calls = checks
            .iter()
            .map(|check| call(check, &place.checked, reported, sink));
        return Some(quote! { #(#calls)* });
    }
    let head = wrap_binding(0);
    let value = wrap_binding(steps.len());
    let calls = checks
        .iter()
        .map(|check| call(check, &quote! { #value }, reported, sink));
    let walk = reached(&steps, &head, 1, &quote! { #(#calls)* });
    let checked = &place.checked;
    Some(quote! {
        {
            let #head = #checked;
            #walk
        }
    })
}

/// The refusal of a `check` written on a type alias: an alias is another name for its target, and
/// has no `impl` of its own to call a function from.
pub fn alias_refusal_output(item_type: &ItemType, checks: &[CheckName]) -> Option<TokenStream> {
    let first = checks.first()?;
    let name = &item_type.ident;
    let target = written_spelling(&item_type.ty);
    let refusal = format!(
        "`check` cannot be written on a type alias: `{name}` is another name for `{target}`, so \
         it has no `impl` of its own to call `{}` from. Write the check on `{target}`, or on a \
         field that holds a `{name}`, in `#[model_schema_prop(...)]`.",
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
