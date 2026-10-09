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
/// All three constructors — the dispatcher's two and the client's one — end this way, so the
/// assertion the seal costs is written once, here, rather than at each site. `members` is the
/// object's own lines, indented as they appear.
#[cfg(feature = "zod")]
pub fn minted(service: &str, members: &str) -> String {
    let fields = fault_fields_typescript_name(service);
    format!(
        "  const built: {fields} = {{\n\
         {members}\n  \
         }};\n  \
         return built as {service}Fault;"
    )
}

/// The symbol the brand is keyed on. `declare const` rather than a value, so nothing is emitted for
/// it and no fault gains a key on the wire; `unique symbol` so the property it keys is one no other
/// declaration can spell.
fn seal(service: &str) -> String {
    let sealed = seal_name(service);
    format!(
        "/**\n \
         * The brand on `{service}Fault`, declared here and exported from nowhere.\n \
         *\n \
         * A module that cannot name this symbol cannot write the property it keys, and a value \
         without\n \
         * that property is not a `{service}Fault`. It is `declare const` rather than a value: \
         nothing\n \
         * is emitted for it, and a fault carries no extra key on the wire.\n \
         */\n\
         declare const {sealed}: unique symbol;"
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
