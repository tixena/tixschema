//! The seal on the published fault: the two declarations that turn a structural object type into
//! one only the generated code can write, and the form every generated constructor mints through.

use crate::rename_rule::RenameRule;
use crate::service_schema::parse::ServiceDef;
use crate::service_schema::support::fault_fields_typescript_name;

pub fn emit(service: &ServiceDef) -> Vec<String> {
    let named = service.ident.to_string();
    vec![seal(&named), sealed_fault(&named)]
}

/// The body of a generated fault constructor: build the fields the Rust declaration published, then
/// seal them.
///
/// Every fault the generated code makes ends this way, so the seal is put on in one place.
/// `members` is the object's own lines, indented as they appear.
#[cfg(feature = "zod")]
pub fn minted(service: &str, members: &str) -> String {
    let fields = fault_fields_typescript_name(service);
    let sealed = seal_name(service);
    format!(
        "  const built: {fields} = {{\n\
         {members}\n  \
         }};\n  \
         const fault: {service}Fault = {{ ...built, [{sealed}]: true }};\n  \
         return fault;"
    )
}

/// The TypeScript expression of what the fault held under `fault` says, for the message of the
/// error it is thrown on: its detail, or the list of every problem a failed validation found,
/// each as the validator words it, as the Rust `Display` of the fault writes it.
#[cfg(feature = "zod")]
pub fn said(fault: &str) -> String {
    format!(
        "{fault}.detail ?? `[${{({fault}.issues ?? []).map((issue) => JSON.stringify(issue.field \
         === undefined ? issue.detail : `'${{issue.field}}': ${{issue.detail}}`)).join(\", \")}}]`"
    )
}

/// The symbol the brand is keyed on: a value, so the generated code puts the brand on with no cast,
/// and a symbol, so `JSON.stringify` writes no key for it; `unique symbol` so the property it keys
/// is one no other declaration can spell.
fn seal(service: &str) -> String {
    let sealed = seal_name(service);
    format!(
        "/**\n \
         * The brand on `{service}Fault`, declared here and exported from nowhere.\n \
         *\n \
         * A module that cannot name this symbol cannot write the property it keys, and a value \
         without\n \
         * that property is not a `{service}Fault`. A symbol key is never written by \
         `JSON.stringify`,\n \
         * so a fault carries no extra key on the wire.\n \
         */\n\
         const {sealed}: unique symbol = Symbol(\"{sealed}\");"
    )
}

/// What a caller names, and what only the generated client and dispatcher can build: the fields the
/// Rust declaration published, plus the brand.
fn sealed_fault(service: &str) -> String {
    let fields = fault_fields_typescript_name(service);
    let sealed = seal_name(service);
    format!(
        "/**\n \
         * A failure `{service}` never declared, as a caller reads it.\n \
         *\n \
         * The fields come from the same Rust declaration the dispatcher and the client build \
         faults\n \
         * from. The brand is what an implementation cannot write: it is keyed on a symbol this\n \
         * bundle declares and exports nowhere, so an object literal under this annotation does \
         not\n \
         * compile, and neither does a structurally-equal value assigned into this position.\n \
         *\n \
         * Reading one is unaffected. Narrow on `isServiceFault`, then read `kind`, `detail`, \
         `field`\n \
         * and `operation`, or switch over `kind` exhaustively — the brand has no runtime value \
         and\n \
         * never reaches the wire.\n \
         */\n\
         export type {service}Fault = {fields} & {{\n  \
         readonly [{sealed}]: true;\n\
         }};"
    )
}

/// The symbol's own name, carrying the service for the reason every published name does: a bundle
/// is one flat file, and ten services would otherwise declare one symbol ten times over.
fn seal_name(service: &str) -> String {
    format!(
        "{}FaultSeal",
        RenameRule::CamelCase.apply_to_variant(service)
    )
}
