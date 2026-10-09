//! The `{Service}{Operation}Result` sealed class pair: Dart's own construct for the two-armed
//! outcome every client in every language now returns from a request-and-reply call.

use super::dart_http_client::{carries_no_value, dart_success_type, dart_type_of};
use super::result::result_name;
use crate::service_schema::parse::{HttpShape, OperationDef, OperationOutcome, ServiceDef};
use crate::service_schema::support::fault_fields_typescript_name;

pub fn emit(service: &ServiceDef) -> Vec<String> {
    let named = service.ident.to_string();
    service
        .operations
        .iter()
        .filter_map(|operation| result_pair(&named, operation))
        .collect()
}

fn result_pair(named: &str, operation: &OperationDef) -> Option<String> {
    let OperationOutcome::Reply {
        error,
        success: _success,
    } = &operation.outcome
    else {
        return None;
    };
    let published = result_name(named, operation)?;
    let shape = HttpShape::of(operation);
    let ok_member = if carries_no_value(operation, &shape) {
        format!("final class {published}Ok extends {published} {{\n  const {published}Ok();\n}}")
    } else {
        let value = dart_success_type(operation, &shape);
        format!(
            "final class {published}Ok extends {published} {{\n  \
             const {published}Ok(this.value);\n  final {value} value;\n}}"
        )
    };
    let failure = dart_type_of(error);
    let fault = fault_fields_typescript_name(named);
    let ident = &operation.ident;
    Some(format!(
        "/// What `{ident}` answers: the success, the error the operation declared, or a fault it \
         never\n\
         /// declared.\n\
         sealed class {published} {{\n  const {published}();\n}}\n\n\
         {ok_member}\n\n\
         final class {published}Operation extends {published} {{\n  \
         const {published}Operation(this.error);\n  final {failure} error;\n}}\n\n\
         final class {published}Fault extends {published} {{\n  \
         const {published}Fault(this.fault);\n  final {fault} fault;\n}}"
    ))
}
