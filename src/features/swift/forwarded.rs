//! The functions a type calls that one of its members shares a name with. Swift reads a bare name
//! in a type as the member ahead of the module's function, so the type calls a private forwarder
//! declared ahead of it, which names the function where no member does.

use core::cell::RefCell;

use syn::Item;
use syn::ext::IdentExt as _;

use super::{
    SwiftTypeArgs, bounded_generic_params, held_at, swift_member, type_parameters_in_scope,
};
use crate::features::model_schema_prop::FunctionName;
use crate::rename_rule::RenameRule;
use crate::utils::compute_item_export_name;

/// The members of the type being emitted a function may share a name with, and what its
/// forwarders are named and typed from.
#[derive(Default)]
struct Context {
    export_name: String,
    generic_params: Vec<String>,
    members: Vec<String>,
}

thread_local! {
    static CONTEXT: RefCell<Context> = RefCell::new(Context::default());

    /// Each forwarder the type being emitted calls, by name, with its declaration.
    static DECLARED: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
}

/// Every name a member of the type `item` may be written as: what the emitter declares on any
/// type, and each field and variant as written, in camel case, or moved off a name Swift will not
/// take.
pub(super) fn record(item: &Item, args: &SwiftTypeArgs<'_>) {
    let (idents, variants, generics) = if let Item::Struct(item_struct) = item {
        (
            item_struct
                .fields
                .iter()
                .filter_map(|field| field.ident.clone())
                .collect::<Vec<_>>(),
            Vec::new(),
            &item_struct.generics,
        )
    } else if let Item::Enum(item_enum) = item {
        (
            item_enum
                .variants
                .iter()
                .flat_map(|variant| variant.fields.iter())
                .filter_map(|field| field.ident.clone())
                .collect(),
            item_enum
                .variants
                .iter()
                .map(|variant| variant.ident.clone())
                .collect(),
            &item_enum.generics,
        )
    } else {
        return;
    };
    let mut members: Vec<String> = ["encode", "validate", "value"]
        .iter()
        .map(|&name| name.to_owned())
        .collect();
    for ident in idents.iter().chain(&variants) {
        let written = ident.unraw().to_string();
        let camel = RenameRule::CamelCase.apply_to_field(&written);
        let lowered = written
            .chars()
            .next()
            .map(|first| {
                format!(
                    "{}{}",
                    first.to_lowercase(),
                    written.get(first.len_utf8()..).unwrap_or_default()
                )
            })
            .unwrap_or_default();
        for name in [written, camel, lowered] {
            members.push(format!("{name}_"));
            members.push(name);
        }
    }
    CONTEXT.with(|held| {
        *held.borrow_mut() = Context {
            export_name: compute_item_export_name(args.type_name, args.name_override),
            generic_params: type_parameters_in_scope(generics),
            members,
        };
    });
}

/// Forgets the type being emitted and what it declared.
pub(super) fn clear() {
    CONTEXT.with(|held| *held.borrow_mut() = Context::default());
    DECLARED.with(|held| held.borrow_mut().clear());
}

/// The forwarders the type being emitted called, as the declarations written ahead of it.
pub(super) fn take() -> String {
    DECLARED
        .with(RefCell::take)
        .into_iter()
        .fold(String::new(), |mut declared, (_, text)| {
            declared.push_str(&text);
            declared.push('\n');
            declared
        })
}

/// The name the type being emitted calls the check `function` by, on a value of `value_type`, or
/// on the whole type where there is none.
pub(super) fn check(function: &FunctionName, value_type: Option<&dyn Fn() -> String>) -> String {
    forwarded(function, "Check", value_type)
}

/// The name the type being emitted calls the `preprocess` function `function` by.
pub(super) fn preprocess(function: &FunctionName) -> String {
    forwarded(function, "Preprocess", None)
}

fn forwarded(
    function: &FunctionName,
    kind: &str,
    value_type: Option<&dyn Fn() -> String>,
) -> String {
    let written = swift_member(&function.name);
    CONTEXT.with(|held| {
        let context = held.borrow();
        if !context.members.contains(&function.name) {
            return written;
        }
        let item = &context.export_name;
        let forwarder = format!(
            "_{}{kind}{}",
            lower_first(item),
            upper_first(&function.name)
        );
        let declaration = if kind == "Check" {
            let whole = held_at(item, &bounded_generic_params(&context.generic_params));
            let ty = value_type.map_or(whole, |typed| typed());
            let reached: Vec<String> = context
                .generic_params
                .iter()
                .filter(|parameter| names(&ty, parameter))
                .cloned()
                .collect();
            format!(
                "private func {forwarder}{}(_ swiftSchemaChecked: {ty}) -> [String] {{ {written}(swiftSchemaChecked) }}",
                bounded_generic_params(&reached)
            )
        } else {
            format!(
                "private func {forwarder}(_ swiftSchemaPayload: Any) -> Any {{ {written}(swiftSchemaPayload) }}"
            )
        };
        DECLARED.with(|recorded| {
            let mut forwarders = recorded.borrow_mut();
            if forwarders.iter().all(|(name, _)| *name != forwarder) {
                forwarders.push((forwarder.clone(), declaration));
            }
        });
        forwarder
    })
}

fn lower_first(name: &str) -> String {
    let mut characters = name.chars();
    characters.next().map_or_else(String::new, |first| {
        format!("{}{}", first.to_lowercase(), characters.as_str())
    })
}

fn upper_first(name: &str) -> String {
    let mut characters = name.chars();
    characters.next().map_or_else(String::new, |first| {
        format!("{}{}", first.to_uppercase(), characters.as_str())
    })
}

/// Whether `text` writes `name` as a whole identifier.
fn names(text: &str, name: &str) -> bool {
    text.split(|character: char| !character.is_alphanumeric() && character != '_')
        .any(|word| word == name)
}
