//! The Kotlin `http_rest` client: request/response data classes, a transport seam the hosting
//! application implements, one sealed result per reply operation, and one `suspend` method per
//! operation that builds a plain-terms request from the operation's own message and decodes the
//! answer by status — mirroring [`super::dart_http_client`] statement for statement, since the URL,
//! query and header rules it builds on were proven by execution on Dart.
//!
//! # A caller reads the outcome; one-way still throws
//!
//! A reply method answers `{Service}{Operation}Result` — a sealed interface of `Ok`/`Declared`/
//! `Fault` nested inside it — and never throws for a declared error or a fault. A one-way operation
//! still answers plainly (`Unit`) and throws the fault-only `{Service}Refusal`, having no reply arm
//! to carry a fault through.
//!
//! # The fault is the same generated type every other surface answers faults through
//!
//! `{Service}FaultFields`/`{Service}FaultKind` already carry `#[model_schema()]`, so their Kotlin
//! comes from the ordinary [`crate::features::kotlin`] dispatch — this module reuses them rather
//! than inventing a fault shape of its own.
//!
//! # Naming
//!
//! Every private helper here is prefixed with the service's own lower-camel name, mirroring the
//! Dart client's own `_{fn_prefix}Http...` convention — Kotlin has no per-file privacy narrower
//! than a `private` modifier, but two services vendored into one bundle would still collide on an
//! unprefixed top-level name.

use super::result::result_name;
use crate::features::kotlin::kotlin_typename;
use crate::field_type::{FieldDefType, get_field_def};
use crate::rename_rule::RenameRule;
use crate::service_schema::parse::{
    BodyKind, DEFAULT_BINDING_ERROR_STATUS, HttpShape, OperationDef, OperationInputs,
    OperationOutcome, PathSegment, ServiceDef, is_scalar_named_type, is_unit_type, option_inner,
    service_declares_a_stream, service_declares_multipart, tuple_elements, vec_inner, wire_key,
};
use crate::service_schema::support::fault_fields_typescript_name;
use core::fmt::Write as _;
use syn::Type;

/// One reply operation's own success shape: the Kotlin type `Ok`'s `value` carries, and any
/// auxiliary `data class` declaration that type needs published ahead of the sealed result —
/// `body = "bytes"` and `body = "stream"` each answer more than the bare success type, and a
/// declared `header_out` composes onto whichever of the two applies.
struct SuccessShape {
    aux_declaration: Option<String>,
    type_name: String,
}

/// One declared `header_out` entry, once matched against `success`'s own tuple elements: the
/// Kotlin property name this module invents for it (`headerOut0`, ...), the response header's own
/// wire name to read it back under, and its declared type.
struct HeaderOutField {
    kotlin_prop: String,
    ty: Type,
    wire_name: String,
}

pub fn emit(service: &ServiceDef) -> Vec<String> {
    let named = service.ident.to_string();
    let fn_prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let has_stream = service_declares_a_stream(service);
    let has_multipart = service_declares_multipart(service);
    let mut published = vec![
        request_class(&named, has_multipart),
        response_class(&named, has_stream),
        transport_interface(&named),
    ];
    published.extend(result_interfaces(service, &named));
    if has_one_way(service) {
        published.push(refusal_class(&named));
    }
    published.push(client_class(service, &named, &fn_prefix, has_multipart));
    published.extend(fault_helpers(service, &named, &fn_prefix));
    published
}

fn has_one_way(service: &ServiceDef) -> bool {
    service
        .operations
        .iter()
        .any(|operation| matches!(operation.outcome, OperationOutcome::OneWay))
}

// ---------------------------------------------------------------------------------------------
// The seam: request, response, transport.
// ---------------------------------------------------------------------------------------------

fn request_class(named: &str, has_multipart: bool) -> String {
    let parts_field = if has_multipart {
        ",\n  val parts: List<Pair<String, Any?>> = emptyList()"
    } else {
        ""
    };
    format!(
        "/// One `{named}` call in plain terms: what a hand-written `{named}HttpTransport`\n\
         /// carries to a real HTTP stack. Names no networking library.\n\
         data class {named}HttpRequest(\n  \
         val method: String,\n  \
         val path: String,\n  \
         val query: String,\n  \
         val headers: List<Pair<String, String>>,\n  \
         val body: ByteArray{parts_field},\n\
         )"
    )
}

fn response_class(named: &str, has_stream: bool) -> String {
    let stream_field = if has_stream {
        ",\n  val bodyStream: Flow<ByteArray> = emptyFlow()"
    } else {
        ""
    };
    format!(
        "/// One `{named}` answer in plain terms.\n\
         data class {named}HttpResponse(\n  \
         val status: Int,\n  \
         val headers: List<Pair<String, String>>,\n  \
         val body: ByteArray{stream_field},\n\
         )"
    )
}

fn transport_interface(named: &str) -> String {
    format!(
        "/// What binds a `{named}` Kotlin client to a real HTTP stack. A hand-written\n\
         /// implementation over any HTTP client satisfies it; nothing here names one.\n\
         interface {named}HttpTransport {{\n  \
         suspend fun send(request: {named}HttpRequest): {named}HttpResponse\n\
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// One sealed result per reply operation: `Ok`/`Declared`/`Fault`, nested so a caller narrows on
// `is {Result}.Ok` exactly as it narrows on any other sealed hierarchy.
// ---------------------------------------------------------------------------------------------

fn result_interfaces(service: &ServiceDef, named: &str) -> Vec<String> {
    service
        .operations
        .iter()
        .filter_map(|operation| result_interface(named, operation))
        .collect()
}

/// Whether a reply operation's success carries nothing at all: the same condition that earns the
/// unit `data object Ok` member instead of a `data class Ok(val value: ...)`.
fn carries_no_value(operation: &OperationDef, shape: &HttpShape) -> bool {
    let OperationOutcome::Reply {
        success,
        error: _error,
    } = &operation.outcome
    else {
        return false;
    };
    matches!(shape.body_kind, BodyKind::Json | BodyKind::Multipart)
        && shape.header_out.is_empty()
        && is_unit_type(success)
}

fn header_out_fields(
    shape: &HttpShape,
    success: &Type,
    body_elements: usize,
) -> Vec<HeaderOutField> {
    let elements: Vec<&Type> = tuple_elements(success).into_iter().flatten().collect();
    shape
        .header_out
        .iter()
        .zip(elements.iter().skip(body_elements))
        .enumerate()
        .map(|(index, (name, element_ty))| HeaderOutField {
            kotlin_prop: format!("headerOut{index}"),
            ty: (*element_ty).clone(),
            wire_name: name.clone(),
        })
        .collect()
}

/// `success`'s own body portion: the first element of its tuple where `header_out` composed one
/// onto it, or `success` itself where it carries no such composition — mirrors the Dart client's
/// own `elements.first()` fallback in `success_decode_block`/`bytes_success_decode_block`.
fn body_element_type(success: &Type) -> Type {
    tuple_elements(success)
        .and_then(|elements| elements.first())
        .cloned()
        .unwrap_or_else(|| success.clone())
}

fn success_shape(published: &str, shape: &HttpShape, success: &Type) -> SuccessShape {
    match shape.body_kind {
        BodyKind::Stream => {
            let extra = header_out_fields(shape, success, 1);
            let type_name = format!("{published}Streamed");
            let mut params = vec![
                "val contentRange: String?".to_owned(),
                "val body: Flow<ByteArray>".to_owned(),
            ];
            for field in &extra {
                params.push(format!(
                    "val {}: {}",
                    field.kotlin_prop,
                    kotlin_type_of(&field.ty)
                ));
            }
            SuccessShape {
                aux_declaration: Some(format!("data class {type_name}({})", params.join(", "))),
                type_name,
            }
        }
        BodyKind::Bytes => {
            let extra = header_out_fields(shape, success, 2);
            let type_name = format!("{published}Bytes");
            let mut params = vec![
                "val body: ByteArray".to_owned(),
                "val contentType: String".to_owned(),
            ];
            for field in &extra {
                params.push(format!(
                    "val {}: {}",
                    field.kotlin_prop,
                    kotlin_type_of(&field.ty)
                ));
            }
            SuccessShape {
                aux_declaration: Some(format!("data class {type_name}({})", params.join(", "))),
                type_name,
            }
        }
        BodyKind::Json | BodyKind::Multipart => {
            if shape.header_out.is_empty() {
                return SuccessShape {
                    aux_declaration: None,
                    type_name: kotlin_type_of(success),
                };
            }
            let extra = header_out_fields(shape, success, 1);
            let type_name = format!("{published}Value");
            let mut params = vec![format!(
                "val value: {}",
                kotlin_type_of(&body_element_type(success))
            )];
            for field in &extra {
                params.push(format!(
                    "val {}: {}",
                    field.kotlin_prop,
                    kotlin_type_of(&field.ty)
                ));
            }
            SuccessShape {
                aux_declaration: Some(format!("data class {type_name}({})", params.join(", "))),
                type_name,
            }
        }
    }
}

fn result_interface(named: &str, operation: &OperationDef) -> Option<String> {
    let OperationOutcome::Reply { error, success } = &operation.outcome else {
        return None;
    };
    let published = result_name(named, operation)?;
    let shape = HttpShape::of(operation);
    let error_ty = kotlin_type_of(error);
    let fault = fault_fields_typescript_name(named);
    let (aux, ok_member) = if carries_no_value(operation, &shape) {
        (String::new(), format!("  data object Ok : {published}"))
    } else {
        let shape_info = success_shape(&published, &shape, success);
        let aux = shape_info
            .aux_declaration
            .map(|declared| format!("{declared}\n"))
            .unwrap_or_default();
        (
            aux,
            format!(
                "  data class Ok(val value: {}) : {published}",
                shape_info.type_name
            ),
        )
    };
    Some(format!(
        "/// What `{}` on `{named}` answers: the success, the error the operation declared, or a\n\
         /// fault it never declared.\n\
         {aux}sealed interface {published} {{\n\
         {ok_member}\n  \
         data class Declared(val error: {error_ty}) : {published}\n  \
         data class Fault(val fault: {fault}) : {published}\n\
         }}",
        operation.ident,
    ))
}

// ---------------------------------------------------------------------------------------------
// The one exception a client still throws: a one-way method's own fault.
// ---------------------------------------------------------------------------------------------

fn refusal_class(named: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// What a one-way `{named}` `http_rest` method throws when it cannot answer its\n\
         /// declared status. A one-way operation declares no error, so there is nothing else to\n\
         /// throw.\n\
         class {named}Refusal(val fault: {fields}) : Exception(fault.detail)"
    )
}

// ---------------------------------------------------------------------------------------------
// The client: one class, one constructor, one `suspend` method per operation.
// ---------------------------------------------------------------------------------------------

fn client_class(service: &ServiceDef, named: &str, fn_prefix: &str, has_multipart: bool) -> String {
    let methods = service
        .operations
        .iter()
        .map(|operation| method(named, fn_prefix, operation, has_multipart))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "/// A `{named}` caller over `http_rest`.\n\
         class {named}HttpClient(private val transport: {named}HttpTransport) {{\n\
         {methods}\n\
         }}"
    )
}

fn method_params(operation: &OperationDef, shape: &HttpShape) -> String {
    let mut params = vec![format!("req: {}", message_kotlin_typename(operation))];
    for header in &shape.header_in {
        params.push(format!(
            "{}: {}",
            kotlin_property(&header.parameter.to_string()),
            kotlin_type_of(&header.ty)
        ));
    }
    for part in &shape.multipart_parts {
        params.push(format!(
            "{}: {}",
            kotlin_property(&part.parameter.to_string()),
            kotlin_type_of(&part.ty)
        ));
    }
    params.join(", ")
}

fn method_doc(operation: &OperationDef, shape: &HttpShape) -> String {
    format!(
        "  /// Calls `{}` over `{} {}`.",
        operation.wire_name,
        shape.method.name(),
        shape.path_template()
    )
}

fn return_type(named: &str, operation: &OperationDef) -> String {
    if matches!(operation.outcome, OperationOutcome::OneWay) {
        return String::new();
    }
    result_name(named, operation).unwrap()
}

fn method(named: &str, fn_prefix: &str, operation: &OperationDef, has_multipart: bool) -> String {
    let shape = HttpShape::of(operation);
    let wire = &operation.wire_name;
    let call = &operation.ts_name;
    let params = method_params(operation, &shape);
    let path_build = path_build_stmt(operation, &shape, fn_prefix);
    let query_build = query_build_stmt(operation, &shape, fn_prefix);
    let headers_build = header_in_build_stmt(&shape);
    let body_build = body_build_stmt(&shape, &message_kotlin_typename(operation));
    let parts_build = multipart_parts_build_stmt(operation, &shape, has_multipart);
    let method_str = shape.method.name();
    let (signature, send, decode) = match &operation.outcome {
        OperationOutcome::OneWay => (
            format!("  suspend fun {call}({params})"),
            send_stmt_one_way(named, fn_prefix, wire, method_str, has_multipart),
            one_way_decode_stmt(named, fn_prefix, &shape, wire),
        ),
        OperationOutcome::Reply { error, success } => {
            let result = return_type(named, operation);
            (
                format!("  suspend fun {call}({params}): {result}"),
                send_stmt_reply(named, &result, fn_prefix, wire, method_str, has_multipart),
                reply_decode_stmt(&result, fn_prefix, &shape, wire, error, success),
            )
        }
    };
    format!(
        "{doc}\n\
         {signature} {{\n\
{path_build}\
{query_build}\
{headers_build}\
{body_build}\
{parts_build}\
{send}\
{decode}\
  }}",
        doc = method_doc(operation, &shape),
    )
}

// ---------------------------------------------------------------------------------------------
// Building the request from the validated message.
// ---------------------------------------------------------------------------------------------

fn placeholder_value_kotlin_expr(
    operation: &OperationDef,
    shape: &HttpShape,
    placeholder: &str,
) -> String {
    let prop = kotlin_property(placeholder);
    match &operation.inputs {
        OperationInputs::Empty => format!("\"${{req.{prop}}}\""),
        OperationInputs::Generated(fields) => fields
            .iter()
            .find(|(field, _)| field == placeholder)
            .map_or_else(
                || format!("\"${{req.{prop}}}\""),
                |(_, ty)| kotlin_wire_text(ty, &format!("req.{prop}"), false),
            ),
        OperationInputs::Named(declared) => {
            if shape.placeholder_names().len() == 1 && is_scalar_named_type(declared) {
                kotlin_wire_text(declared, "req", true)
            } else {
                format!("\"${{req.{prop}}}\"")
            }
        }
    }
}

fn path_build_stmt(operation: &OperationDef, shape: &HttpShape, fn_prefix: &str) -> String {
    let mut stmt = String::from("    var path = \"\"\n");
    for segment in &shape.path {
        match segment {
            PathSegment::Literal(text) => {
                let _ = writeln!(stmt, "    path += \"{}\"", kotlin_escape(text));
            }
            PathSegment::Placeholder(name) => {
                let value = placeholder_value_kotlin_expr(operation, shape, name);
                let _ = writeln!(stmt, "    path += {fn_prefix}HttpPercentEncode({value})");
            }
        }
    }
    stmt
}

fn query_build_stmt(operation: &OperationDef, shape: &HttpShape, fn_prefix: &str) -> String {
    if shape.method.carries_a_body() {
        return "    val query = \"\"\n".to_owned();
    }
    let fields = match &operation.inputs {
        // `Empty` sends no field. A bodyless `Named` message is always the one scalar the path
        // binds whole (refused at parse time otherwise), reading off the placeholder rather than
        // the query.
        OperationInputs::Empty | OperationInputs::Named(_) => {
            return "    val query = \"\"\n".to_owned();
        }
        OperationInputs::Generated(fields) => fields,
    };
    let placeholders = shape.placeholder_names();
    let mut pushes = String::new();
    let mut any = false;
    for (field, ty) in fields {
        let field_name = field.to_string();
        if placeholders.contains(&field_name) {
            continue;
        }
        any = true;
        let key = wire_key(field);
        let prop = kotlin_property(&field_name);
        let inner = option_inner(ty).unwrap_or(ty);
        let rendered = kotlin_wire_text(inner, "value", true);
        let _ = write!(
            pushes,
            "    req.{prop}?.let {{ value ->\n      \
             queryParts.add(\"{key}=\" + {fn_prefix}HttpPercentEncode({rendered}))\n    \
             }}\n"
        );
    }
    if !any {
        return "    val query = \"\"\n".to_owned();
    }
    format!(
        "    val queryParts = mutableListOf<String>()\n{pushes}    val query = queryParts.joinToString(\"&\")\n"
    )
}

fn header_in_build_stmt(shape: &HttpShape) -> String {
    if shape.header_in.is_empty() {
        return "    val headers = emptyList<Pair<String, String>>()\n".to_owned();
    }
    let mut stmt = String::from("    val headers = mutableListOf<Pair<String, String>>()\n");
    for header in &shape.header_in {
        let name = &header.name;
        let prop = kotlin_property(&header.parameter.to_string());
        if let Some(inner) = option_inner(&header.ty) {
            let text = kotlin_wire_text(inner, &prop, true);
            let _ = writeln!(
                stmt,
                "    {prop}?.let {{ headers.add(\"{name}\" to {text}) }}"
            );
        } else {
            let text = kotlin_wire_text(&header.ty, &prop, true);
            let _ = writeln!(stmt, "    headers.add(\"{name}\" to {text})");
        }
    }
    stmt
}

fn body_build_stmt(shape: &HttpShape, req_type: &str) -> String {
    match shape.body_kind {
        BodyKind::Multipart => "    val body = ByteArray(0)\n".to_owned(),
        BodyKind::Bytes | BodyKind::Json | BodyKind::Stream => {
            if shape.method.carries_a_body() {
                format!(
                    "    val body = Json.encodeToString(serializer<{req_type}>(), req).encodeToByteArray()\n"
                )
            } else {
                "    val body = ByteArray(0)\n".to_owned()
            }
        }
    }
}

fn multipart_parts_build_stmt(
    operation: &OperationDef,
    shape: &HttpShape,
    has_multipart: bool,
) -> String {
    if !has_multipart {
        return String::new();
    }
    if !matches!(shape.body_kind, BodyKind::Multipart) {
        return "    val parts = emptyList<Pair<String, Any?>>()\n".to_owned();
    }
    let placeholders = shape.placeholder_names();
    let mut stmt = String::from("    val parts = mutableListOf<Pair<String, Any?>>()\n");
    if let OperationInputs::Generated(fields) = &operation.inputs {
        for (field, ty) in fields {
            let field_name = field.to_string();
            if placeholders.contains(&field_name) {
                continue;
            }
            let key = wire_key(field);
            let prop = kotlin_property(&field_name);
            if let Some(inner) = option_inner(ty) {
                let text = kotlin_wire_text(inner, "it", true);
                let _ = writeln!(
                    stmt,
                    "    req.{prop}?.let {{ parts.add(\"{key}\" to {text}) }}"
                );
            } else {
                let text = kotlin_wire_text(ty, &format!("req.{prop}"), false);
                let _ = writeln!(stmt, "    parts.add(\"{key}\" to {text})");
            }
        }
    }
    for part in &shape.multipart_parts {
        let name = &part.name;
        let prop = kotlin_property(&part.parameter.to_string());
        let _ = writeln!(stmt, "    parts.add(\"{name}\" to {prop})");
    }
    stmt
}

// ---------------------------------------------------------------------------------------------
// Sending, and decoding the answer by status.
// ---------------------------------------------------------------------------------------------

fn send_expr(named: &str, method_str: &str, has_multipart: bool) -> String {
    let parts_arg = if has_multipart { ", parts = parts" } else { "" };
    format!(
        "transport.send({named}HttpRequest(method = \"{method_str}\", path = path, query = query, headers = headers, body = body{parts_arg}))"
    )
}

fn send_stmt_one_way(
    named: &str,
    fn_prefix: &str,
    wire: &str,
    method_str: &str,
    has_multipart: bool,
) -> String {
    format!(
        "    val response = try {{\n      {send}\n    }} catch (thrown: Throwable) {{\n      \
         throw {named}Refusal({fn_prefix}HttpTransportFailure(\"{wire}\", thrown.toString()))\n    \
         }}\n",
        send = send_expr(named, method_str, has_multipart),
    )
}

fn send_stmt_reply(
    named: &str,
    result: &str,
    fn_prefix: &str,
    wire: &str,
    method_str: &str,
    has_multipart: bool,
) -> String {
    format!(
        "    val response = try {{\n      {send}\n    }} catch (thrown: Throwable) {{\n      \
         return {result}.Fault({fn_prefix}HttpTransportFailure(\"{wire}\", thrown.toString()))\n    \
         }}\n",
        send = send_expr(named, method_str, has_multipart),
    )
}

fn one_way_decode_stmt(named: &str, fn_prefix: &str, shape: &HttpShape, wire: &str) -> String {
    let ok_status = shape.ok_status;
    format!(
        "    val status = response.status\n    \
         if (status == {ok_status}) {{\n      return\n    }}\n    \
         if (status == 400 || status == 404 || status == 500) {{\n      \
         throw {named}Refusal({fn_prefix}HttpFaultFromBody(\"{wire}\", response))\n    \
         }}\n    \
         throw {named}Refusal(\n      \
         {fn_prefix}HttpUndeserializablePayload(\"{wire}\", \"an unexpected status ($status) answered\"),\n    \
         )\n"
    )
}

fn error_condition_expr(shape: &HttpShape) -> String {
    if shape.error_status.is_empty() {
        format!("status == {DEFAULT_BINDING_ERROR_STATUS}")
    } else {
        shape
            .error_status
            .iter()
            .map(|(_, code)| format!("status == {code}"))
            .collect::<Vec<_>>()
            .join(" || ")
    }
}

fn reply_decode_stmt(
    result: &str,
    fn_prefix: &str,
    shape: &HttpShape,
    wire: &str,
    error: &Type,
    success: &Type,
) -> String {
    if matches!(shape.body_kind, BodyKind::Stream) {
        return stream_reply_decode_stmt(result, fn_prefix, shape, wire, error, success);
    }
    let ok_status = shape.ok_status;
    let error_condition = error_condition_expr(shape);
    let error_ty = kotlin_type_of(error);
    let success_block = success_decode_block(result, fn_prefix, wire, shape, success);
    format!(
        "    val status = response.status\n    \
         if (status == {ok_status}) {{\n{success_block}    }}\n    \
         if ({error_condition}) {{\n      \
         return try {{\n        \
         {result}.Declared(Json.decodeFromString(serializer<{error_ty}>(), response.body.decodeToString()))\n      \
         }} catch (rejected: Throwable) {{\n        \
         {result}.Fault({fn_prefix}HttpUndeserializablePayload(\"{wire}\", rejected.toString()))\n      \
         }}\n    \
         }}\n    \
         if (status == 400 || status == 404 || status == 500) {{\n      \
         return {result}.Fault({fn_prefix}HttpFaultFromBody(\"{wire}\", response))\n    \
         }}\n    \
         return {result}.Fault(\n      \
         {fn_prefix}HttpUndeserializablePayload(\"{wire}\", \"an unexpected status ($status) answered\"),\n    \
         )\n"
    )
}

fn stream_reply_decode_stmt(
    result: &str,
    fn_prefix: &str,
    shape: &HttpShape,
    wire: &str,
    error: &Type,
    success: &Type,
) -> String {
    let ok_status = shape.ok_status;
    let error_condition = error_condition_expr(shape);
    let error_ty = kotlin_type_of(error);
    let partial = stream_success_arm(result, fn_prefix, wire, shape, success, true);
    let full = stream_success_arm(result, fn_prefix, wire, shape, success, false);
    format!(
        "    val status = response.status\n    \
         if (status == 206) {{\n{partial}    }}\n    \
         if (status == {ok_status}) {{\n{full}    }}\n    \
         if ({error_condition}) {{\n      \
         return try {{\n        \
         {result}.Declared(Json.decodeFromString(serializer<{error_ty}>(), response.body.decodeToString()))\n      \
         }} catch (rejected: Throwable) {{\n        \
         {result}.Fault({fn_prefix}HttpUndeserializablePayload(\"{wire}\", rejected.toString()))\n      \
         }}\n    \
         }}\n    \
         if (status == 400 || status == 404 || status == 500) {{\n      \
         return {result}.Fault({fn_prefix}HttpFaultFromBody(\"{wire}\", response))\n    \
         }}\n    \
         return {result}.Fault(\n      \
         {fn_prefix}HttpUndeserializablePayload(\"{wire}\", \"an unexpected status ($status) answered\"),\n    \
         )\n"
    )
}

fn stream_success_arm(
    result: &str,
    fn_prefix: &str,
    wire: &str,
    shape: &HttpShape,
    success: &Type,
    partial: bool,
) -> String {
    let mut stmt = if partial {
        format!(
            "      val contentRange = {}(response.headers, \"content-range\")\n",
            find_header_call(fn_prefix)
        )
    } else {
        "      val contentRange: String? = null\n".to_owned()
    };
    let extra = header_out_fields(shape, success, 1);
    if extra.is_empty() {
        let _ = writeln!(
            stmt,
            "      return {result}.Ok({result}Streamed(contentRange, response.bodyStream))"
        );
        return stmt;
    }
    let (header_stmts, header_idents) = header_out_read_stmts(result, fn_prefix, wire, &extra);
    stmt.push_str(&header_stmts);
    let _ = writeln!(
        stmt,
        "      return {result}.Ok({result}Streamed(contentRange, response.bodyStream, {}))",
        header_idents.join(", ")
    );
    stmt
}

fn header_out_read_stmts(
    result: &str,
    fn_prefix: &str,
    wire: &str,
    extra: &[HeaderOutField],
) -> (String, Vec<String>) {
    let mut stmt = String::new();
    let mut idents = Vec::new();
    for field in extra {
        let raw_ident = format!("raw{}", capitalize(&field.kotlin_prop));
        let decode = kotlin_header_out_decode(&field.ty, &raw_ident);
        let _ = write!(
            stmt,
            "      val {raw_ident} = {find_header}(response.headers, \"{header_name}\")\n      \
             ?: return {result}.Fault(\n        \
             {fn_prefix}HttpUndeserializablePayload(\"{wire}\", \"a declared response header was missing\"),\n      \
             )\n      \
             val {prop} = {decode}\n",
            find_header = find_header_call(fn_prefix),
            header_name = field.wire_name,
            prop = field.kotlin_prop,
        );
        idents.push(field.kotlin_prop.clone());
    }
    (stmt, idents)
}

/// `raw{Camel}` for `header_out_read_stmts`'s own raw-value local, ahead of the decoded property
/// (`headerOut0` -> `rawHeaderOut0`).
fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        let mut capitalized: String = first.to_uppercase().collect();
        capitalized.push_str(chars.as_str());
        capitalized
    })
}

fn success_decode_block(
    result: &str,
    fn_prefix: &str,
    wire: &str,
    shape: &HttpShape,
    success: &Type,
) -> String {
    if matches!(shape.body_kind, BodyKind::Bytes) {
        return bytes_success_decode_block(result, fn_prefix, wire, shape, success);
    }
    if shape.header_out.is_empty() {
        if is_unit_type(success) {
            return format!("      return {result}.Ok\n");
        }
        let success_ty = kotlin_type_of(success);
        return format!(
            "      return try {{\n        \
             {result}.Ok(Json.decodeFromString(serializer<{success_ty}>(), response.body.decodeToString()))\n      \
             }} catch (rejected: Throwable) {{\n        \
             {result}.Fault({fn_prefix}HttpUndeserializablePayload(\"{wire}\", rejected.toString()))\n      \
             }}\n"
        );
    }
    let extra = header_out_fields(shape, success, 1);
    let success_ty = kotlin_type_of(&body_element_type(success));
    let mut stmt = format!(
        "      val value = try {{\n        \
         Json.decodeFromString(serializer<{success_ty}>(), response.body.decodeToString())\n      \
         }} catch (rejected: Throwable) {{\n        \
         return {result}.Fault({fn_prefix}HttpUndeserializablePayload(\"{wire}\", rejected.toString()))\n      \
         }}\n"
    );
    let (header_stmts, header_idents) = header_out_read_stmts(result, fn_prefix, wire, &extra);
    stmt.push_str(&header_stmts);
    let _ = writeln!(
        stmt,
        "      return {result}.Ok({result}Value(value, {}))",
        header_idents.join(", ")
    );
    stmt
}

fn bytes_success_decode_block(
    result: &str,
    fn_prefix: &str,
    wire: &str,
    shape: &HttpShape,
    success: &Type,
) -> String {
    let mut stmt = format!(
        "      val contentType = {}(response.headers, \"content-type\") ?: \"\"\n",
        find_header_call(fn_prefix)
    );
    let extra = header_out_fields(shape, success, 2);
    if extra.is_empty() {
        let _ = writeln!(
            stmt,
            "      return {result}.Ok({result}Bytes(response.body, contentType))"
        );
        return stmt;
    }
    let (header_stmts, header_idents) = header_out_read_stmts(result, fn_prefix, wire, &extra);
    stmt.push_str(&header_stmts);
    let _ = writeln!(
        stmt,
        "      return {result}.Ok({result}Bytes(response.body, contentType, {}))",
        header_idents.join(", ")
    );
    stmt
}

// ---------------------------------------------------------------------------------------------
// The fault helpers every method reaches for.
// ---------------------------------------------------------------------------------------------

fn fault_helpers(service: &ServiceDef, named: &str, fn_prefix: &str) -> Vec<String> {
    let mut helpers = vec![percent_encode_fn(fn_prefix)];
    if reads_a_response_header(service) {
        helpers.push(find_header_fn(fn_prefix));
    }
    helpers.extend([
        transport_failure_fn(named, fn_prefix),
        undeserializable_payload_fn(named, fn_prefix),
        fault_from_body_fn(named, fn_prefix),
    ]);
    helpers
}

fn reads_a_response_header(service: &ServiceDef) -> bool {
    service.operations.iter().any(|operation| {
        let shape = HttpShape::of(operation);
        !shape.header_out.is_empty()
            || matches!(shape.body_kind, BodyKind::Bytes | BodyKind::Stream)
    })
}

/// The RFC 3986 unreserved-character percent-encoder every path and query value is written
/// through — the emitted code writes it for itself rather than reaching for `java.net.URLEncoder`
/// (which encodes a space as `+`, the wrong rule for a URI component).
fn percent_encode_fn(fn_prefix: &str) -> String {
    format!(
        "/// Percent-encodes `value` for a URL path segment or query value: every byte but the\n\
         /// unreserved set (`A-Za-z0-9-._~`) is written `%XX`.\n\
         private fun {fn_prefix}HttpPercentEncode(value: String): String {{\n  \
         val builder = StringBuilder()\n  \
         for (byte in value.encodeToByteArray()) {{\n    \
         val ch = byte.toInt().toChar()\n    \
         if (ch.isLetterOrDigit() && ch.code < 128 || ch == '-' || ch == '.' || ch == '_' || ch == '~') {{\n      \
         builder.append(ch)\n    \
         }} else {{\n      \
         builder.append('%')\n      \
         builder.append(String.format(\"%02X\", byte.toInt() and 0xFF))\n    \
         }}\n  \
         }}\n  \
         return builder.toString()\n\
         }}"
    )
}

fn find_header_fn(fn_prefix: &str) -> String {
    format!(
        "/// Reads one response header back case-insensitively, the way HTTP headers are read.\n\
         private fun {}(headers: List<Pair<String, String>>, name: String): String? {{\n  \
         for (header in headers) {{\n    \
         if (header.first.lowercase() == name) return header.second\n  \
         }}\n  \
         return null\n\
         }}",
        find_header_call(fn_prefix)
    )
}

fn find_header_call(fn_prefix: &str) -> String {
    format!("{fn_prefix}HttpFindHeader")
}

fn transport_failure_fn(named: &str, fn_prefix: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// The fault a `{named}` HTTP client answers with when the transport could not carry a\n\
         /// call: the request never went out, or the response never came back.\n\
         private fun {fn_prefix}HttpTransportFailure(operation: String, detail: String): {fields} =\n  \
         {fields}(\n    \
         detail = detail,\n    \
         kind = {named}FaultKind.TransportFailure,\n    \
         operation = operation,\n  \
         )"
    )
}

fn undeserializable_payload_fn(named: &str, fn_prefix: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// The fault a `{named}` HTTP client answers with when a response will not become the\n\
         /// answer its status promised: a body that will not parse, a declared header that never\n\
         /// arrived, or a status this client did not expect.\n\
         private fun {fn_prefix}HttpUndeserializablePayload(operation: String, detail: String): {fields} =\n  \
         {fields}(\n    \
         detail = detail,\n    \
         kind = {named}FaultKind.UndeserializablePayload,\n    \
         operation = operation,\n  \
         )"
    )
}

fn fault_from_body_fn(named: &str, fn_prefix: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// Reads a fixed fault status's own body back into a `{fields}`, through the same JSON\n\
         /// codec every other surface reads it with. A body that does not read as the fixed fault\n\
         /// shape is itself a defect, answered as one.\n\
         private fun {fn_prefix}HttpFaultFromBody(operation: String, response: {named}HttpResponse): {fields} =\n  \
         try {{\n    \
         Json.decodeFromString(serializer<{fields}>(), response.body.decodeToString())\n  \
         }} catch (rejected: Throwable) {{\n    \
         {fn_prefix}HttpUndeserializablePayload(\n      \
         operation,\n      \
         \"a fault body did not match the expected shape\",\n    \
         )\n  \
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// Small, Kotlin-flavored value rendering — kept apart from `features::kotlin` itself, which
// carries no HTTP-shaped knowledge at all.
// ---------------------------------------------------------------------------------------------

/// `raw`'s own Kotlin property spelling: `conversation_id` -> `conversationId`. Every reference
/// this module writes to a field or an argument goes through this, since `features::kotlin`
/// camel-cases every property it declares.
fn kotlin_property(raw: &str) -> String {
    RenameRule::CamelCase.apply_to_field(raw)
}

fn message_kotlin_typename(operation: &OperationDef) -> String {
    match &operation.inputs {
        OperationInputs::Named(declared) => kotlin_type_of(declared),
        OperationInputs::Empty | OperationInputs::Generated(_) => {
            operation.generated_message_ident().map_or_else(
                || "Unit".to_owned(),
                |ident| {
                    let named: Type = syn::parse_quote! { #ident };
                    kotlin_type_of(&named)
                },
            )
        }
    }
}

/// `ty`'s own Kotlin type name, read through the same [`crate::field_type::FieldDef`] walk every
/// field's type goes through — so a reference to a sibling `#[model_schema()]` type resolves to
/// its published Kotlin name exactly as it would inside an ordinary field.
fn kotlin_type_of(ty: &Type) -> String {
    kotlin_typename(&get_field_def("value", ty, ""))
}

/// Whether `ty` is a reference to another `#[model_schema()]` item — the one distinction
/// [`kotlin_wire_text`] needs to decide whether a value must be read through its own JSON codec
/// before it can be embedded as text.
fn is_sibling_type(ty: &Type) -> bool {
    matches!(
        get_field_def("value", ty, "").field_type,
        FieldDefType::SiblingType(_, _)
    )
}

/// The Kotlin expression that renders `ty`'s value at `expr` as URL- or header-safe text: an
/// `Option<T>` reads as the empty string when absent, a `List<T>` joins with a comma, and a
/// sibling type is read through the JSON codec first. `promoted` marks `expr` already null-checked.
fn kotlin_wire_text(ty: &Type, expr: &str, promoted: bool) -> String {
    if let Some(inner) = option_inner(ty) {
        let narrowed = if promoted {
            expr.to_owned()
        } else {
            format!("{expr}!!")
        };
        let rendered = kotlin_wire_text(inner, &narrowed, promoted);
        return format!("(if ({expr} == null) \"\" else {rendered})");
    }
    if let Some(inner) = vec_inner(ty) {
        let element = kotlin_wire_text(inner, "it", true);
        return format!("({expr}).joinToString(\",\") {{ {element} }}");
    }
    if is_sibling_type(ty) {
        // A scalar sibling type serializes to a bare JSON primitive; reading `.jsonPrimitive.content`
        // back off it gives the unquoted wire text a path or query value needs.
        let element_ty = kotlin_type_of(ty);
        return format!(
            "Json.encodeToJsonElement(serializer<{element_ty}>(), {expr}).jsonPrimitive.content"
        );
    }
    format!("\"${{{expr}}}\"")
}

/// The expression that reads one `header_out` element's declared type back off `raw` — a
/// non-null `String` expression already checked — mirroring the coercion the Rust, TypeScript and
/// Dart clients perform on the way back from a response header.
fn kotlin_header_out_decode(ty: &Type, raw: &str) -> String {
    let base = option_inner(ty).unwrap_or(ty);
    if let Some(inner) = vec_inner(base) {
        let element = kotlin_header_out_decode(inner, "piece");
        return format!("({raw}).split(\",\").map {{ piece -> {element} }}");
    }
    match get_field_def("value", base, "").field_type {
        FieldDefType::Boolean => format!("({raw} == \"true\")"),
        FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Usize
        | FieldDefType::Isize => format!("({raw}).toLong()"),
        FieldDefType::F32 | FieldDefType::F64 => format!("({raw}).toDouble()"),
        FieldDefType::BooleanLiteral(_)
        | FieldDefType::Char
        | FieldDefType::Map(_, _)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::SiblingType(_, _)
        | FieldDefType::String
        | FieldDefType::StringLiteral(_)
        | FieldDefType::Tuple(_)
        | FieldDefType::TypeParam(_)
        | FieldDefType::Unknown => raw.to_owned(),
        #[cfg(feature = "object_id")]
        FieldDefType::ObjectId => raw.to_owned(),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate
        | FieldDefType::NaiveTime
        | FieldDefType::NaiveDateTime
        | FieldDefType::DateTime => raw.to_owned(),
    }
}

/// Escapes a path literal for a double-quoted Kotlin string: a backslash, a double quote and a
/// dollar sign (which Kotlin reads as interpolation even inside a plain string literal) all need
/// escaping; a path template is ASCII by construction, so nothing else does.
fn kotlin_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(ch, '\\' | '"' | '$') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}
