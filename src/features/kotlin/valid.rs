//! The valid type of each model type the emitter writes: `{Name}Draft` is the class code builds
//! and changes, and `{Name}` wraps a draft that passed `validate()` in a private constructor.

use core::cell::Cell;
use core::fmt::Write as _;

use super::validate::Rules;
use super::{KotlinArgs, kotlin_property_name, kotlin_serializer_param_name, read};

thread_local! {
    /// Whether the item being expanded has a draft.
    static DRAFTED: Cell<bool> = const { Cell::new(true) };
}

const PLAIN_ISSUES: &str = "fun validationIssues(): List<String> = draft.validationIssues(); ";

/// The parameters a keyed `validationIssues` takes, and what it asks of the draft: `variants` are
/// the subclasses of an enum's draft that take the serializers.
fn keyed_issues(keys: &[String], variants: &[String]) -> (String, String) {
    let taken = keys
        .iter()
        .map(|key| format!("{}: KSerializer<{key}>?", kotlin_serializer_param_name(key)))
        .collect::<Vec<_>>()
        .join(", ");
    let names = keys
        .iter()
        .map(|key| kotlin_serializer_param_name(key))
        .collect::<Vec<_>>()
        .join(", ");
    let asked = if variants.is_empty() {
        format!("draft.validationIssues({names})")
    } else {
        let arms = variants
            .iter()
            .map(|variant| format!("is {variant} -> draft.validationIssues({names})"))
            .collect::<Vec<_>>()
            .join("; ");
        format!("when (draft) {{ {arms}; else -> draft.validationIssues() }}")
    };
    (taken, asked)
}

/// The members through which a valid type answers what its draft reports, which every holder asks
/// of a value it holds. `keys` are the type parameters whose serializers the draft takes.
pub fn forwarded_issues(keys: &[String]) -> String {
    if keys.is_empty() {
        return PLAIN_ISSUES.to_owned();
    }
    let (taken, asked) = keyed_issues(keys, &[]);
    format!("{PLAIN_ISSUES}fun validationIssues({taken}): List<String> = {asked}; ")
}

/// [`forwarded_issues`] for the valid type of an enum, which declares its parameters `out` as its
/// draft does: a member cannot take a serializer of a parameter in that position, so the keyed
/// one is an extension, private to the file the emitted types are joined into.
pub fn enum_forwarded_issues() -> String {
    PLAIN_ISSUES.to_owned()
}

/// The extension of the valid enum `export_name` that answers what its draft reports given the
/// serializers of `keys`, where there are any.
pub fn keyed_extension(
    export_name: &str,
    generic_params: &str,
    keys: &[String],
    variants: &[String],
) -> String {
    if keys.is_empty() {
        return String::new();
    }
    let (taken, asked) = keyed_issues(keys, variants);
    format!(
        " private fun {generic_params} {export_name}{generic_params}.validationIssues({taken}): \
         List<String> = {asked}"
    )
}

/// Whether the item being expanded has a draft: a type a service writes for itself has none.
pub fn drafted() -> bool {
    DRAFTED.with(Cell::get)
}

/// Starts the expansion of the item `args` describes.
pub fn enter(args: &KotlinArgs<'_>) {
    DRAFTED.with(|drafted| drafted.set(!args.undrafted));
}

/// The name of the class that holds the members of the model type `export_name`: its draft, or
/// the type itself where it has none.
pub fn draft_name(export_name: &str) -> String {
    if drafted() {
        format!("{export_name}Draft")
    } else {
        export_name.to_owned()
    }
}

/// The `validate()` the draft carries: its rules, then the valid type.
pub fn validate_method(export_name: &str, generic_params: &str) -> String {
    format!(
        "fun validate(): {export_name}{generic_params} {{ val violations = validationIssues(); \
         if (violations.isNotEmpty()) throw {}; return {export_name}.checked(this) }}",
        read::JUDGED
    )
}

/// The members that forward to the draft each property it holds, as `declared` types them.
pub fn forwarded_properties(declared: &[(&str, String)]) -> String {
    declared
        .iter()
        .fold(String::new(), |mut members, (rust_name, ty)| {
            let property = kotlin_property_name(rust_name);
            let _ = write!(members, "val {property}: {ty} get() = draft.{property}; ");
            members
        })
}

/// The valid class of `export_name`, over the draft `draft`. `declared` is the type parameters
/// as the class declares them and `generic_params` as a use names them; `members` are the
/// forwarding ones.
pub fn valid_class(
    export_name: &str,
    draft: &str,
    (declared, generic_params): (&str, &str),
    type_parameters: &[String],
    members: &str,
) -> String {
    let checked_params = if type_parameters.is_empty() {
        String::new()
    } else {
        format!("<{}> ", type_parameters.join(", "))
    };
    format!(
        "@ConsistentCopyVisibility @Serializable(with = {export_name}Serializer::class) data class \
         {export_name}{declared} private constructor(val draft: {draft}{generic_params}) {{ \
         {members}companion object {{ internal fun {checked_params}checked(draft: \
         {draft}{generic_params}): {export_name}{generic_params} = {export_name}(draft) }} }}"
    )
}

/// The members of a model type's body that carry its rules: `validationIssues()` and `validate()`
/// where it has a draft, and `validate()` alone where it has none.
pub fn body(rules: &mut Rules<'_>, export_name: &str, generic_params: &str) -> String {
    if drafted() {
        format!(
            "{} {}",
            rules.issues(""),
            validate_method(export_name, generic_params)
        )
    } else {
        rules.written("")
    }
}

/// The valid class of a model type and its unjudged serializer, or nothing where it has no
/// draft. `composed` says the draft's own serializer binds each type parameter to `Any`.
pub fn pair(
    export_name: &str,
    draft: &str,
    generics: (&str, &str),
    type_parameters: &[String],
    (members, composed): (&str, bool),
) -> String {
    if !drafted() {
        return String::new();
    }
    format!(
        "{} {}",
        valid_class(export_name, draft, generics, type_parameters, members),
        read::valid_unjudged(export_name, draft, generics.1, type_parameters, composed)
    )
}
