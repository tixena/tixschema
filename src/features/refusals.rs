//! The refusal class each Dart, Swift and Kotlin type that judges a value declares in its own text,
//! `{Name}Refusal`, which `validate()`, `of(..)` and the type's judged read throw with the list of
//! broken rules, and the classes a service surface reads that list off.

#[cfg(any(
    feature = "kotlin",
    feature = "swift",
    all(feature = "serde", feature = "dart")
))]
use crate::field_type::{FieldDef, FieldDefType, is_sequence_wrapper};

/// The types the service macro itself writes into the module of the service `service`, for an
/// `mcp` operation to name: none is a model, and none judges a value, so none declares a refusal.
/// A type of a consumer's, whatever it is called, is none of them.
#[cfg(feature = "serde")]
pub fn service_types(service: &str) -> Vec<String> {
    [
        "McpContent",
        "McpPromptMessage",
        "McpResourcePage",
        "McpTask",
        "McpTaskState",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain([format!("{service}McpResourcePage")])
    .collect()
}

/// Whether `name`, written as a path or not, is one of `written`.
#[cfg(any(
    feature = "kotlin",
    all(feature = "serde", any(feature = "dart", feature = "swift"))
))]
fn is_written(name: &str, written: &[String]) -> bool {
    name.rsplit("::")
        .next()
        .is_some_and(|last| written.iter().any(|declared| declared == last))
}

/// The refusal class of the type published as `export_name`, without its type parameters.
pub fn refusal_name(export_name: &str) -> String {
    format!("{export_name}Refusal")
}

/// Every refusal class a read of `def` throws where the value it reads, or one a list, a set, a
/// map, an option, a tuple or a `Result` holds, breaks a rule: `refusal_of` names the class of a
/// declared type by its Rust ident, which every declared type declares whatever its place. A
/// generic type judges what fills its parameters itself, so its arguments add none. A type in
/// `written`, which the service macro wrote, names none.
#[cfg(any(
    feature = "kotlin",
    all(feature = "serde", any(feature = "dart", feature = "swift"))
))]
pub fn reached(
    def: &FieldDef,
    written: &[String],
    refusal_of: &dyn Fn(&str) -> String,
) -> Vec<String> {
    let mut found = Vec::new();
    collect(def, written, refusal_of, &mut found);
    found
}

/// The refusal class of the declared type an alias of `target` is, where `target` is one bare named
/// type: a read of the alias throws what a read of that type throws.
#[cfg(any(feature = "swift", feature = "kotlin"))]
pub fn aliased(target: &FieldDef, refusal_of: &dyn Fn(&str) -> String) -> Option<String> {
    if let FieldDefType::SiblingType(name, _) = &target.field_type
        && !target.is_list()
        && !target.is_optional()
        && !is_sequence_wrapper(name)
    {
        Some(refusal_of(name))
    } else {
        None
    }
}

#[cfg(any(
    feature = "kotlin",
    all(feature = "serde", any(feature = "dart", feature = "swift"))
))]
fn collect(
    def: &FieldDef,
    written: &[String],
    refusal_of: &dyn Fn(&str) -> String,
    found: &mut Vec<String>,
) {
    let held: Vec<&FieldDef> = if let FieldDefType::SiblingType(name, generics) = &def.field_type {
        if let [element] = generics.as_slice()
            && is_sequence_wrapper(name)
        {
            vec![element]
        } else if is_written(name, written) {
            Vec::new()
        } else {
            let refusal = refusal_of(name);
            if !found.contains(&refusal) {
                found.push(refusal);
            }
            Vec::new()
        }
    } else if let FieldDefType::Map(key, value) | FieldDefType::Result(key, value) = &def.field_type
    {
        vec![key.as_ref(), value.as_ref()]
    } else if let FieldDefType::Tuple(elements) = &def.field_type {
        elements.iter().collect()
    } else {
        Vec::new()
    };
    for inner in held {
        collect(inner, written, refusal_of, found);
    }
}
