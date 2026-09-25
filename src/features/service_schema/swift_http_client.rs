//! The Swift `http_rest` client: one transport seam the app implements, one `async` method per
//! operation, mirroring the Dart client's request grammar statement for statement.
//!
//! # A caller reads the outcome; one-way still throws
//!
//! A reply operation answers `Result<Success, Failure>` and never throws; a one-way operation
//! answers `Void` and throws only the fault-only `{Service}Refusal`, having no reply arm to carry
//! a fault through otherwise — the same outcome shape the TypeScript clients report through.
//!
//! # Swift needs two helpers Dart gets for free
//!
//! Dart's `Uri.encodeComponent` and `List<String>.join('&')` have no Foundation equivalent that
//! matches them exactly, so this module writes `{fnPrefix}PercentEncode`/`{fnPrefix}QueryText`
//! once per service — the RFC 3986 unreserved set plus `-_.!~*'()`, the same characters
//! `Uri.encodeComponent` leaves unescaped.
//!
//! # A message property is reached in Swift's own spelling
//!
//! A generated message's Swift property is always `RenameRule::CamelCase.apply_to_field` of the
//! raw Rust field name — the same spelling an `http(...)` path placeholder or a bodyless method's
//! own field name is written in — regardless of any serde rename on the field, since Swift's own
//! `Codable` synthesis carries the wire spelling through a separate `CodingKeys` enum instead.
//!
//! # A branded newtype is a value wrapper, not a bare scalar
//!
//! Swift's own `Codable` synthesis has no union type, so a branded newtype (`ConversationId`)
//! publishes a struct with one `value` property rather than TypeScript's intersection brand. A
//! placeholder or header reading a sibling type's value therefore reads `.value`, never the
//! sibling type itself.

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

use super::swift_type::swift_typename_of;

/// The Swift type a `body = "stream"` operation's own success answers with: a nullable
/// `contentRange` paired with the body as an `AsyncThrowingStream<Data, Error>` — mirrors the
/// Dart client's own `STREAMED_ANSWER_DART_TYPE`.
const STREAMED_ANSWER_SWIFT_TYPE: &str =
    "(contentRange: String?, body: AsyncThrowingStream<Data, Error>)";

pub fn emit(service: &ServiceDef) -> Vec<String> {
    let named = service.ident.to_string();
    let fn_prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let has_stream = service_declares_a_stream(service);
    let has_multipart = service_declares_multipart(service);
    let mut published = vec![
        request_struct(&named, has_multipart),
        response_struct(&named, has_stream),
        transport_protocol(&named),
        fault_alias(&named),
    ];
    for operation in &service.operations {
        if let Some(failure) = failure_enum(&named, operation) {
            published.push(failure);
        }
    }
    if has_one_way(service) {
        published.push(refusal_struct(&named));
    }
    published.push(client_struct(service, &named, &fn_prefix, has_multipart));
    published.extend(fault_helpers(service, &named, &fn_prefix));
    published
}

fn has_one_way(service: &ServiceDef) -> bool {
    service
        .operations
        .iter()
        .any(|operation| matches!(operation.outcome, OperationOutcome::OneWay))
}

// -------------------------------------------------------------------------------------------
// The seam: request in, response out, one transport protocol a hosting app implements.
// -------------------------------------------------------------------------------------------

fn request_struct(named: &str, has_multipart: bool) -> String {
    let (parts_field, parts_param, parts_assign) = if has_multipart {
        (
            "\n  public let parts: [(String, any Sendable)]",
            ", parts: [(String, any Sendable)]",
            " self.parts = parts;",
        )
    } else {
        ("", "", "")
    };
    format!(
        "/// One `{named}` call, in plain terms: nothing here names the library that finally\n\
         /// carries it.\n\
         public struct {named}HttpRequest: Sendable {{\n  \
         public let method: String\n  \
         public let path: String\n  \
         public let query: String\n  \
         public let headers: [(String, String)]\n  \
         public let body: Data{parts_field}\n\n  \
         public init(method: String, path: String, query: String, headers: [(String, String)], \
         body: Data{parts_param}) {{\n    \
         self.method = method; self.path = path; self.query = query; self.headers = headers; \
         self.body = body;{parts_assign}\n  \
         }}\n\
         }}"
    )
}

fn response_struct(named: &str, has_stream: bool) -> String {
    let (stream_field, stream_param, stream_assign) = if has_stream {
        (
            "\n  public let bodyStream: AsyncThrowingStream<Data, Error>",
            ", bodyStream: AsyncThrowingStream<Data, Error>",
            " self.bodyStream = bodyStream;",
        )
    } else {
        ("", "", "")
    };
    format!(
        "/// What a `{named}` transport answers `send` with.\n\
         public struct {named}HttpResponse: Sendable {{\n  \
         public let status: Int\n  \
         public let headers: [(String, String)]\n  \
         public let body: Data{stream_field}\n\n  \
         public init(status: Int, headers: [(String, String)], body: Data{stream_param}) {{\n    \
         self.status = status; self.headers = headers; self.body = body;{stream_assign}\n  \
         }}\n\
         }}"
    )
}

fn transport_protocol(named: &str) -> String {
    format!(
        "/// What binds a `{named}` Swift client to a real HTTP stack. Nothing here names one.\n\
         public protocol {named}HttpTransport: Sendable {{\n  \
         func send(_ request: {named}HttpRequest) async throws -> {named}HttpResponse\n\
         }}"
    )
}

// -------------------------------------------------------------------------------------------
// The failure a reply operation answers with, and the refusal a one-way operation throws.
// -------------------------------------------------------------------------------------------

/// What one reply operation's `Result<Success, Failure>` names as `Failure`: the declared error,
/// or a fault. `None` for a one-way operation, which has no reply arm to carry either in.
fn failure_name(named: &str, operation: &OperationDef) -> Option<String> {
    match &operation.outcome {
        OperationOutcome::OneWay => None,
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => Some(format!(
            "{named}{}Failure",
            RenameRule::PascalCase.apply_to_field(&operation.ident.to_string())
        )),
    }
}

/// `Result`'s own `Failure` parameter requires `Error`, so every failure conforms to it beside
/// `Sendable` — the compiled spike's own shape, minus `Equatable`, which the generated message
/// and fault types this enum carries do not themselves conform to.
fn failure_enum(named: &str, operation: &OperationDef) -> Option<String> {
    let OperationOutcome::Reply {
        error,
        success: _success,
    } = &operation.outcome
    else {
        return None;
    };
    let failure = failure_name(named, operation)?;
    let fields = fault_fields_typescript_name(named);
    let error_ty = swift_typename_of(error);
    Some(format!(
        "public enum {failure}: Error, Sendable {{\n  \
         case declared({error_ty})\n  \
         case fault({fields})\n\
         }}"
    ))
}

/// `{Named}Fault`, the shared fault type both this client and `swift_ws_client()` answer a
/// defect with — declared unconditionally, since `swift_ws_client()`'s own helpers name it
/// whether or not this client also publishes.
fn fault_alias(named: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!("public typealias {named}Fault = {fields}")
}

fn refusal_struct(named: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// What a one-way `{named}` `http_rest` method throws when it cannot answer its declared\n\
         /// status. A one-way operation declares no error, so there is nothing else to throw.\n\
         public struct {named}Refusal: Error, Sendable {{\n  \
         public let fault: {fields}\n\
         }}"
    )
}

// -------------------------------------------------------------------------------------------
// The client: one struct, one initializer, one method per operation.
// -------------------------------------------------------------------------------------------

fn client_struct(
    service: &ServiceDef,
    named: &str,
    fn_prefix: &str,
    has_multipart: bool,
) -> String {
    let methods = service
        .operations
        .iter()
        .map(|operation| method(named, fn_prefix, operation, has_multipart))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "/// A `{named}` caller over `http_rest`.\n\
         public struct {named}HttpClient: Sendable {{\n  \
         private let transport: any {named}HttpTransport\n\n  \
         public init(transport: any {named}HttpTransport) {{ self.transport = transport }}\n\n\
         {methods}\n\
         }}"
    )
}

/// One argument per `header_in` binding, then one per `part` binding, after the message —
/// mirrors the Dart client's own `method_params`: the raw Rust identifier, never re-cased,
/// since these are function arguments rather than message properties.
fn method_params(operation: &OperationDef, shape: &HttpShape) -> String {
    let mut params = vec![format!("_ req: {}", message_swift_typename(operation))];
    for header in &shape.header_in {
        params.push(format!(
            "{}: {}",
            header.parameter,
            swift_typename_of(&header.ty)
        ));
    }
    for part in &shape.multipart_parts {
        params.push(format!(
            "{}: {}",
            part.parameter,
            swift_typename_of(&part.ty)
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

fn stream_success_swift_type(shape: &HttpShape, success: &Type) -> String {
    if shape.header_out.is_empty() {
        return STREAMED_ANSWER_SWIFT_TYPE.to_owned();
    }
    let elements: Vec<&Type> = tuple_elements(success).into_iter().flatten().collect();
    let mut parts = vec![STREAMED_ANSWER_SWIFT_TYPE.to_owned()];
    parts.extend(elements.iter().skip(1).map(|ty| swift_typename_of(ty)));
    format!("({})", parts.join(", "))
}

/// A `body = "bytes"` operation's own success type: the raw bytes as `Data` (never the generic
/// `swift_typename_of` rendering of `Vec<u8>`), then its content type, then one element per
/// declared `header_out` entry.
fn bytes_success_swift_type(success: &Type) -> String {
    let elements: Vec<&Type> = tuple_elements(success).into_iter().flatten().collect();
    let mut parts = vec!["Data".to_owned()];
    parts.extend(elements.iter().skip(1).map(|ty| swift_typename_of(ty)));
    format!("({})", parts.join(", "))
}

/// The Swift type `Result<Success, Failure>` names as `Success` — shared with nothing else, since
/// Swift has no separate sealed pair to keep in step with it.
fn swift_success_type(operation: &OperationDef, shape: &HttpShape) -> String {
    match &operation.outcome {
        OperationOutcome::OneWay => "Void".to_owned(),
        OperationOutcome::Reply {
            success,
            error: _error,
        } => match shape.body_kind {
            BodyKind::Bytes => bytes_success_swift_type(success),
            BodyKind::Stream => stream_success_swift_type(shape, success),
            BodyKind::Json | BodyKind::Multipart => {
                if shape.header_out.is_empty() && is_unit_type(success) {
                    "Void".to_owned()
                } else {
                    swift_typename_of(success)
                }
            }
        },
    }
}

fn return_type(named: &str, operation: &OperationDef) -> String {
    match &operation.outcome {
        OperationOutcome::OneWay => "async throws".to_owned(),
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => {
            let shape = HttpShape::of(operation);
            let failure = failure_name(named, operation).unwrap();
            format!(
                "async -> Result<{}, {failure}>",
                swift_success_type(operation, &shape)
            )
        }
    }
}

fn method(named: &str, fn_prefix: &str, operation: &OperationDef, has_multipart: bool) -> String {
    let shape = HttpShape::of(operation);
    let wire = &operation.wire_name;
    let call = &operation.ts_name;
    let params = method_params(operation, &shape);
    let returns = return_type(named, operation);
    let path_build = path_build_stmt(fn_prefix, operation, &shape);
    let query_build = query_build_stmt(operation, &shape);
    let headers_build = header_in_build_stmt(&shape);
    let body_build = body_build_stmt(named, fn_prefix, wire, &operation.outcome, &shape);
    let parts_build = multipart_parts_build_stmt(operation, &shape, has_multipart);
    let method_str = shape.method.name();
    let (send, decode) = match &operation.outcome {
        OperationOutcome::OneWay => (
            send_stmt_one_way(named, fn_prefix, wire, method_str, has_multipart),
            one_way_decode_stmt(named, fn_prefix, &shape, wire),
        ),
        OperationOutcome::Reply { error, success } => (
            send_stmt_reply(named, fn_prefix, wire, method_str, has_multipart),
            reply_decode_stmt(fn_prefix, &shape, wire, error, success),
        ),
    };
    format!(
        "{doc}\n  \
         public func {call}({params}) {returns} {{\n\
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

// -------------------------------------------------------------------------------------------
// Building the request from the validated message.
// -------------------------------------------------------------------------------------------

/// The Swift expression that renders one path placeholder's value: the field it names (a message
/// the macro declared, or an author's own struct), or the whole message where that message
/// already is a wire scalar — mirrors the Dart client's own `placeholder_value_dart_expr`.
fn placeholder_value_swift_expr(
    operation: &OperationDef,
    shape: &HttpShape,
    placeholder: &str,
) -> String {
    let accessor = RenameRule::CamelCase.apply_to_field(placeholder);
    match &operation.inputs {
        OperationInputs::Empty => format!("\"\\(req.{accessor})\""),
        OperationInputs::Generated(fields) => fields
            .iter()
            .find(|(field, _)| *field == placeholder)
            .map_or_else(
                || format!("\"\\(req.{accessor})\""),
                |(_, ty)| swift_wire_text(ty, &format!("req.{accessor}")),
            ),
        OperationInputs::Named(declared) => {
            if shape.placeholder_names().len() == 1 && is_scalar_named_type(declared) {
                swift_wire_text(declared, "req")
            } else {
                format!("\"\\(req.{accessor})\"")
            }
        }
    }
}

fn path_build_stmt(fn_prefix: &str, operation: &OperationDef, shape: &HttpShape) -> String {
    let mut stmt = String::from("    var path = \"\"\n");
    for segment in &shape.path {
        match segment {
            PathSegment::Literal(text) => {
                let _ = writeln!(stmt, "    path += \"{}\"", swift_escape(text));
            }
            PathSegment::Placeholder(name) => {
                let value = placeholder_value_swift_expr(operation, shape, name);
                let _ = writeln!(stmt, "    path += {fn_prefix}PercentEncode({value})");
            }
        }
    }
    stmt
}

fn query_build_stmt(operation: &OperationDef, shape: &HttpShape) -> String {
    if shape.method.carries_a_body() {
        return "    let query: [(String, String)] = []\n".to_owned();
    }
    let fields = match &operation.inputs {
        // `Empty` sends no field. A bodyless `Named` message is always the one scalar the path
        // binds whole (refused at parse time otherwise), reading off the placeholder rather than
        // the query.
        OperationInputs::Empty | OperationInputs::Named(_) => {
            return "    let query: [(String, String)] = []\n".to_owned();
        }
        OperationInputs::Generated(fields) => fields,
    };
    let placeholders = shape.placeholder_names();
    let mut pushes = String::new();
    for (field, ty) in fields {
        let field_name = field.to_string();
        if placeholders.contains(&field_name) {
            continue;
        }
        let key = wire_key(field);
        let accessor = RenameRule::CamelCase.apply_to_field(&field_name);
        let inner = option_inner(ty).unwrap_or(ty);
        let rendered = swift_wire_text(inner, "value");
        let _ = write!(
            pushes,
            "    if let value = req.{accessor} {{\n      \
             query.append((\"{key}\", {rendered}))\n    \
             }}\n"
        );
    }
    if pushes.is_empty() {
        return "    let query: [(String, String)] = []\n".to_owned();
    }
    format!("    var query: [(String, String)] = []\n{pushes}")
}

/// Builds the outgoing header list, one entry per `header_in` binding — except a `nil` optional
/// binding, which is added nowhere rather than as the empty string [`swift_wire_text`] renders
/// for a present-but-empty value.
fn header_in_build_stmt(shape: &HttpShape) -> String {
    if shape.header_in.is_empty() {
        return "    let headers: [(String, String)] = []\n".to_owned();
    }
    let mut stmt = String::from("    var headers: [(String, String)] = []\n");
    for header in &shape.header_in {
        let name = &header.name;
        let parameter = &header.parameter;
        if let Some(inner) = option_inner(&header.ty) {
            let text = swift_wire_text(inner, &parameter.to_string());
            let _ = writeln!(
                stmt,
                "    if let {parameter} = {parameter} {{\n      \
                 headers.append((\"{name}\", {text}))\n    \
                 }}"
            );
        } else {
            let text = swift_wire_text(&header.ty, &parameter.to_string());
            let _ = writeln!(stmt, "    headers.append((\"{name}\", {text}))");
        }
    }
    stmt
}

fn body_build_stmt(
    named: &str,
    fn_prefix: &str,
    wire: &str,
    outcome: &OperationOutcome,
    shape: &HttpShape,
) -> String {
    match shape.body_kind {
        BodyKind::Multipart => "    let body = Data()\n".to_owned(),
        BodyKind::Bytes | BodyKind::Json | BodyKind::Stream => {
            if shape.method.carries_a_body() {
                json_encode_body_stmt(named, fn_prefix, wire, outcome)
            } else {
                "    let body = Data()\n".to_owned()
            }
        }
    }
}

fn json_encode_body_stmt(
    named: &str,
    fn_prefix: &str,
    wire: &str,
    outcome: &OperationOutcome,
) -> String {
    let recover = encode_failure_recovery(named, fn_prefix, wire, outcome);
    format!(
        "    let body: Data\n    \
         do {{\n      \
         body = try JSONEncoder().encode(req)\n    \
         }} catch {{\n      \
{recover}\
         }}\n"
    )
}

fn encode_failure_recovery(
    named: &str,
    fn_prefix: &str,
    wire: &str,
    outcome: &OperationOutcome,
) -> String {
    let fault = format!("{fn_prefix}UndeserializablePayload(\"{wire}\", \"\\(error)\")");
    match outcome {
        OperationOutcome::OneWay => {
            format!("      throw {named}Refusal(fault: {fault})\n")
        }
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => {
            format!("      return .failure(.fault({fault}))\n")
        }
    }
}

/// The `parts` a `body = "multipart"` method sends: one text entry per carried `Generated` field
/// not otherwise placeholder-bound, then one per declared `part` binding, passed through
/// untouched. Every other body kind on a multipart-declaring service still builds an empty one.
fn multipart_parts_build_stmt(
    operation: &OperationDef,
    shape: &HttpShape,
    has_multipart: bool,
) -> String {
    if !has_multipart {
        return String::new();
    }
    if !matches!(shape.body_kind, BodyKind::Multipart) {
        return "    let parts: [(String, any Sendable)] = []\n".to_owned();
    }
    let placeholders = shape.placeholder_names();
    let mut stmt = String::from("    var parts: [(String, any Sendable)] = []\n");
    if let OperationInputs::Generated(fields) = &operation.inputs {
        for (field, ty) in fields {
            let field_name = field.to_string();
            if placeholders.contains(&field_name) {
                continue;
            }
            let key = wire_key(field);
            let accessor = RenameRule::CamelCase.apply_to_field(&field_name);
            if let Some(inner) = option_inner(ty) {
                let text = swift_wire_text(inner, "value");
                let _ = write!(
                    stmt,
                    "    if let value = req.{accessor} {{\n      \
                     parts.append((\"{key}\", {text}))\n    \
                     }}\n"
                );
            } else {
                let text = swift_wire_text(ty, &format!("req.{accessor}"));
                let _ = writeln!(stmt, "    parts.append((\"{key}\", {text}))");
            }
        }
    }
    for part in &shape.multipart_parts {
        let name = &part.name;
        let parameter = &part.parameter;
        let _ = writeln!(stmt, "    parts.append((\"{name}\", {parameter}))");
    }
    stmt
}

// -------------------------------------------------------------------------------------------
// Sending, and decoding the answer by status.
// -------------------------------------------------------------------------------------------

fn send_expr(named: &str, fn_prefix: &str, method_str: &str, has_multipart: bool) -> String {
    let parts_arg = if has_multipart { ", parts: parts" } else { "" };
    format!(
        "try await transport.send(\n      \
         {named}HttpRequest(method: \"{method_str}\", path: path, query: {fn_prefix}QueryText(query), \
         headers: headers, body: body{parts_arg})\n    \
         )"
    )
}

fn send_stmt_one_way(
    named: &str,
    fn_prefix: &str,
    wire: &str,
    method_str: &str,
    has_multipart: bool,
) -> String {
    let send = send_expr(named, fn_prefix, method_str, has_multipart);
    format!(
        "    let response: {named}HttpResponse\n    \
         do {{\n      \
         response = {send}\n    \
         }} catch {{\n      \
         throw {named}Refusal(fault: {fn_prefix}TransportFailure(\"{wire}\", \"\\(error)\"))\n    \
         }}\n"
    )
}

fn send_stmt_reply(
    named: &str,
    fn_prefix: &str,
    wire: &str,
    method_str: &str,
    has_multipart: bool,
) -> String {
    let send = send_expr(named, fn_prefix, method_str, has_multipart);
    format!(
        "    let response: {named}HttpResponse\n    \
         do {{\n      \
         response = {send}\n    \
         }} catch {{\n      \
         return .failure(.fault({fn_prefix}TransportFailure(\"{wire}\", \"\\(error)\")))\n    \
         }}\n"
    )
}

fn one_way_decode_stmt(named: &str, fn_prefix: &str, shape: &HttpShape, wire: &str) -> String {
    let ok_status = shape.ok_status;
    format!(
        "    let status = response.status\n    \
         if status == {ok_status} {{ return }}\n    \
         if status == 400 || status == 404 || status == 500 {{\n      \
         throw {named}Refusal(fault: {fn_prefix}FaultFromBody(\"{wire}\", response.body))\n    \
         }}\n    \
         throw {named}Refusal(\n      \
         fault: {fn_prefix}UndeserializablePayload(\n        \
         \"{wire}\", \"an unexpected status (\\(status)) answered\"\n      \
         )\n    \
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
    fn_prefix: &str,
    shape: &HttpShape,
    wire: &str,
    error: &Type,
    success: &Type,
) -> String {
    if matches!(shape.body_kind, BodyKind::Stream) {
        return stream_reply_decode_stmt(fn_prefix, shape, wire, error, success);
    }
    let ok_status = shape.ok_status;
    let error_condition = error_condition_expr(shape);
    let error_ty = swift_typename_of(error);
    let success_block = success_decode_block(fn_prefix, wire, shape, success);
    format!(
        "    let status = response.status\n    \
         if status == {ok_status} {{\n{success_block}    }}\n    \
         if {error_condition} {{\n      \
         let declared: {error_ty}\n      \
         do {{\n        \
         declared = try JSONDecoder().decode({error_ty}.self, from: response.body)\n      \
         }} catch {{\n        \
         return .failure(.fault(\n          \
         {fn_prefix}UndeserializablePayload(\"{wire}\", \"\\(error)\")\n        \
         ))\n      \
         }}\n      \
         return .failure(.declared(declared))\n    \
         }}\n    \
         if status == 400 || status == 404 || status == 500 {{\n      \
         return .failure(.fault({fn_prefix}FaultFromBody(\"{wire}\", response.body)))\n    \
         }}\n    \
         return .failure(.fault(\n      \
         {fn_prefix}UndeserializablePayload(\n        \
         \"{wire}\", \"an unexpected status (\\(status)) answered\"\n      \
         )\n    \
         ))\n"
    )
}

/// A `body = "stream"` operation's own decode: `206` answers the streamed value with its own
/// `contentRange` read back; the declared `ok_status` answers the same shape with `contentRange`
/// left `nil`; every other status falls through the same ladder every other kind answers through.
fn stream_reply_decode_stmt(
    fn_prefix: &str,
    shape: &HttpShape,
    wire: &str,
    error: &Type,
    success: &Type,
) -> String {
    let ok_status = shape.ok_status;
    let error_condition = error_condition_expr(shape);
    let error_ty = swift_typename_of(error);
    let partial = stream_success_arm(fn_prefix, wire, shape, success, true);
    let full = stream_success_arm(fn_prefix, wire, shape, success, false);
    format!(
        "    let status = response.status\n    \
         if status == 206 {{\n{partial}    }}\n    \
         if status == {ok_status} {{\n{full}    }}\n    \
         if {error_condition} {{\n      \
         let declared: {error_ty}\n      \
         do {{\n        \
         declared = try JSONDecoder().decode({error_ty}.self, from: response.body)\n      \
         }} catch {{\n        \
         return .failure(.fault(\n          \
         {fn_prefix}UndeserializablePayload(\"{wire}\", \"\\(error)\")\n        \
         ))\n      \
         }}\n      \
         return .failure(.declared(declared))\n    \
         }}\n    \
         if status == 400 || status == 404 || status == 500 {{\n      \
         return .failure(.fault({fn_prefix}FaultFromBody(\"{wire}\", response.body)))\n    \
         }}\n    \
         return .failure(.fault(\n      \
         {fn_prefix}UndeserializablePayload(\n        \
         \"{wire}\", \"an unexpected status (\\(status)) answered\"\n      \
         )\n    \
         ))\n"
    )
}

/// One status arm of [`stream_reply_decode_stmt`]: `contentRange` read back off the response for
/// a `206` partial answer, left `nil` for the declared `ok_status`'s whole-body answer, then every
/// declared `header_out` element read back exactly as the bytes and JSON paths do.
fn stream_success_arm(
    fn_prefix: &str,
    wire: &str,
    shape: &HttpShape,
    success: &Type,
    partial: bool,
) -> String {
    let mut stmt = if partial {
        format!(
            "      let contentRange = {fn_prefix}FindHeader(response.headers, \"content-range\") ?? \"\"\n"
        )
    } else {
        "      let contentRange: String? = nil\n".to_owned()
    };
    stmt.push_str("      let answer = (contentRange: contentRange, body: response.bodyStream)\n");
    if shape.header_out.is_empty() {
        stmt.push_str("      return .success(answer)\n");
        return stmt;
    }
    let elements: Vec<&Type> = tuple_elements(success).into_iter().flatten().collect();
    let (header_stmts, header_idents) = header_out_read_stmts(fn_prefix, wire, shape, &elements, 1);
    stmt.push_str(&header_stmts);
    let _ = writeln!(
        stmt,
        "      return .success((answer, {}))",
        header_idents.join(", ")
    );
    stmt
}

/// Reads every declared `header_out` element back off the response headers, in declaration order,
/// skipping `body_elements` positions in `elements` to reach the first header slot — 1 for the
/// ordinary JSON and streamed shapes, 2 for `body = "bytes"`. Shared so no copy can drift.
fn header_out_read_stmts(
    fn_prefix: &str,
    wire: &str,
    shape: &HttpShape,
    elements: &[&Type],
    body_elements: usize,
) -> (String, Vec<String>) {
    let mut stmt = String::new();
    let mut idents = Vec::new();
    for (index, (name, element_ty)) in shape
        .header_out
        .iter()
        .zip(elements.iter().skip(body_elements))
        .enumerate()
    {
        let ident = format!("headerOut{index}");
        let decode = swift_header_out_decode(element_ty, &format!("rawHeaderOut{index}"));
        let _ = write!(
            stmt,
            "      guard let rawHeaderOut{index} = {fn_prefix}FindHeader(response.headers, \"{name}\") \
             else {{\n        \
             return .failure(.fault(\n          \
             {fn_prefix}UndeserializablePayload(\"{wire}\", \"a declared response header was missing\")\n        \
             ))\n      \
             }}\n      \
             let {ident} = {decode}\n",
        );
        idents.push(ident);
    }
    (stmt, idents)
}

/// What one operation's method returns once its status has already matched `ok_status`: bytes and
/// content type, nothing for a no-payload reply, the decoded body alone, or the body plus every
/// `header_out` element. `body = "stream"` never reaches here — it has two success statuses.
fn success_decode_block(fn_prefix: &str, wire: &str, shape: &HttpShape, success: &Type) -> String {
    if matches!(shape.body_kind, BodyKind::Bytes) {
        return bytes_success_decode_block(fn_prefix, wire, shape, success);
    }
    if shape.header_out.is_empty() {
        if is_unit_type(success) {
            return "      return .success(())\n".to_owned();
        }
        let success_ty = swift_typename_of(success);
        return format!(
            "      let value: {success_ty}\n      \
             do {{\n        \
             value = try JSONDecoder().decode({success_ty}.self, from: response.body)\n      \
             }} catch {{\n        \
             return .failure(.fault(\n          \
             {fn_prefix}UndeserializablePayload(\"{wire}\", \"\\(error)\")\n        \
             ))\n      \
             }}\n      \
             return .success(value)\n"
        );
    }
    // `header_out`'s own arity check guarantees `success` is a tuple of exactly this many
    // elements, so the lookups below never miss.
    let elements: Vec<&Type> = tuple_elements(success).into_iter().flatten().collect();
    let body_ty = elements
        .first()
        .map_or_else(|| swift_typename_of(success), |ty| swift_typename_of(ty));
    let mut stmt = format!(
        "      let value: {body_ty}\n      \
         do {{\n        \
         value = try JSONDecoder().decode({body_ty}.self, from: response.body)\n      \
         }} catch {{\n        \
         return .failure(.fault(\n          \
         {fn_prefix}UndeserializablePayload(\"{wire}\", \"\\(error)\")\n        \
         ))\n      \
         }}\n"
    );
    let (header_stmts, header_idents) = header_out_read_stmts(fn_prefix, wire, shape, &elements, 1);
    stmt.push_str(&header_stmts);
    let _ = writeln!(
        stmt,
        "      return .success((value, {}))",
        header_idents.join(", ")
    );
    stmt
}

/// A `body = "bytes"` operation's own success decode: the raw response body and its content type,
/// then every declared `header_out` element read back off the response's own headers.
fn bytes_success_decode_block(
    fn_prefix: &str,
    wire: &str,
    shape: &HttpShape,
    success: &Type,
) -> String {
    let mut stmt = format!(
        "      let contentType = {fn_prefix}FindHeader(response.headers, \"content-type\") ?? \"\"\n"
    );
    if shape.header_out.is_empty() {
        stmt.push_str("      return .success((response.body, contentType))\n");
        return stmt;
    }
    let elements: Vec<&Type> = tuple_elements(success).into_iter().flatten().collect();
    let (header_stmts, header_idents) = header_out_read_stmts(fn_prefix, wire, shape, &elements, 2);
    stmt.push_str(&header_stmts);
    let _ = writeln!(
        stmt,
        "      return .success((response.body, contentType, {}))",
        header_idents.join(", ")
    );
    stmt
}

// -------------------------------------------------------------------------------------------
// The fault helpers every method reaches for.
// -------------------------------------------------------------------------------------------

fn fault_helpers(service: &ServiceDef, named: &str, fn_prefix: &str) -> Vec<String> {
    let mut helpers = vec![percent_encode_fn(fn_prefix), query_text_fn(fn_prefix)];
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

/// Read through [`HttpShape::of`], the shape every method above is built from, so the gate and
/// the methods cannot answer differently for an operation with no `http(...)` group.
fn reads_a_response_header(service: &ServiceDef) -> bool {
    service.operations.iter().any(|operation| {
        let shape = HttpShape::of(operation);
        !shape.header_out.is_empty()
            || matches!(shape.body_kind, BodyKind::Bytes | BodyKind::Stream)
    })
}

/// Percent-encodes exactly the characters `Uri.encodeComponent` leaves unescaped on the Dart
/// side: the RFC 3986 unreserved set plus `-_.!~*'()`.
fn percent_encode_fn(fn_prefix: &str) -> String {
    format!(
        "/// Percent-encodes `text` the same way the Dart client's `Uri.encodeComponent` does.\n\
         func {fn_prefix}PercentEncode(_ text: String) -> String {{\n  \
         var allowed = CharacterSet.alphanumerics\n  \
         allowed.insert(charactersIn: \"-_.!~*'()\")\n  \
         return text.addingPercentEncoding(withAllowedCharacters: allowed) ?? text\n\
         }}"
    )
}

fn query_text_fn(fn_prefix: &str) -> String {
    format!(
        "/// Joins percent-encoded query pairs the way the Dart client's own `queryParts` are\n\
         /// joined, keys left as written — a wire key this crate composes is always URL-safe.\n\
         func {fn_prefix}QueryText(_ pairs: [(String, String)]) -> String {{\n  \
         pairs.map {{ \"\\($0.0)=\\({fn_prefix}PercentEncode($0.1))\" }}.joined(separator: \"&\")\n\
         }}"
    )
}

/// Reads one response header back case-insensitively, the way HTTP headers are read.
fn find_header_fn(fn_prefix: &str) -> String {
    format!(
        "/// Reads one response header back case-insensitively, the way HTTP headers are read.\n\
         func {fn_prefix}FindHeader(_ headers: [(String, String)], _ name: String) -> String? {{\n  \
         for header in headers where header.0.lowercased() == name {{\n    \
         return header.1\n  \
         }}\n  \
         return nil\n\
         }}"
    )
}

fn transport_failure_fn(named: &str, fn_prefix: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    let kind = format!("{named}FaultKind");
    format!(
        "/// The fault a `{named}` HTTP client answers with when the transport could not carry a\n\
         /// call: the request never went out, or the response never came back.\n\
         func {fn_prefix}TransportFailure(_ operation: String, _ detail: String) -> {fields} {{\n  \
         {fields}(detail: detail, field: nil, kind: {kind}.transportFailure, operation: operation)\n\
         }}"
    )
}

fn undeserializable_payload_fn(named: &str, fn_prefix: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    let kind = format!("{named}FaultKind");
    format!(
        "/// The fault a `{named}` HTTP client answers with when a response will not become the\n\
         /// answer its status promised: a body that will not decode, a declared header that\n\
         /// never arrived, or a status this client did not expect.\n\
         func {fn_prefix}UndeserializablePayload(_ operation: String, _ detail: String) -> {fields} {{\n  \
         {fields}(detail: detail, field: nil, kind: {kind}.undeserializablePayload, operation: operation)\n\
         }}"
    )
}

fn fault_from_body_fn(named: &str, fn_prefix: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// Reads a fixed fault status's own body back into a `{fields}`, through the same\n\
         /// `Codable` conformance every other surface reads it with. A body that does not decode\n\
         /// as the fixed fault shape is itself a defect, answered as one.\n\
         func {fn_prefix}FaultFromBody(_ operation: String, _ body: Data) -> {fields} {{\n  \
         if let decoded = try? JSONDecoder().decode({fields}.self, from: body) {{\n    \
         return decoded\n  \
         }}\n  \
         return {fn_prefix}UndeserializablePayload(\n    \
         operation, \"a fault body did not match the expected shape\"\n  \
         )\n\
         }}"
    )
}

// -------------------------------------------------------------------------------------------
// Small, Swift-flavored value rendering.
// -------------------------------------------------------------------------------------------

/// The message's Swift type: the type the operation named, or the one the macro declared for an
/// operation that named none — mirrors the Dart client's own `message_dart_typename`.
fn message_swift_typename(operation: &OperationDef) -> String {
    match &operation.inputs {
        OperationInputs::Named(declared) => swift_typename_of(declared),
        OperationInputs::Empty | OperationInputs::Generated(_) => {
            operation.generated_message_ident().map_or_else(
                || "Never".to_owned(),
                |ident| {
                    let named: Type = syn::parse_quote! { #ident };
                    swift_typename_of(&named)
                },
            )
        }
    }
}

/// Whether `ty` is a reference to another `#[model_schema()]` item rather than a primitive this
/// crate itself renders — every one reachable here is a branded newtype, `is_scalar_named_type`
/// being what let it stand in for the whole message in the first place.
fn is_sibling_type(ty: &Type) -> bool {
    matches!(
        get_field_def("value", ty, "").field_type,
        FieldDefType::SiblingType(_, _)
    )
}

/// The Swift expression that renders `ty`'s value at `expr` as URL- or header-safe text: an
/// `Option<T>` reads as the empty string when absent, a `Vec<T>` joins its elements' own text
/// with a comma, and a branded sibling type is read through its own `.value` first.
fn swift_wire_text(ty: &Type, expr: &str) -> String {
    if let Some(inner) = option_inner(ty) {
        let rendered = swift_wire_text(inner, "unwrapped");
        return format!("(({expr}).map {{ unwrapped in {rendered} }} ?? \"\")");
    }
    if let Some(inner) = vec_inner(ty) {
        let element = swift_wire_text(inner, "element");
        return format!("(({expr}).map {{ element in {element} }}.joined(separator: \",\"))");
    }
    if is_sibling_type(ty) {
        return format!("\"\\(({expr}).value)\"");
    }
    format!("\"\\({expr})\"")
}

/// The expression that reads one `header_out` element's declared type back off `raw`, a
/// non-optional `String` already checked non-missing. A value this crate cannot parse back falls
/// back to a fixed default rather than throwing: a reply method's return type carries no `throws`.
fn swift_header_out_decode(ty: &Type, raw: &str) -> String {
    let base = option_inner(ty).unwrap_or(ty);
    if let Some(inner) = vec_inner(base) {
        let element = swift_header_out_decode(inner, "piece");
        return format!(
            "{raw}.split(separator: \",\").map {{ piece -> String in String(piece) }}\n        \
             .map {{ piece in {element} }}"
        );
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
        | FieldDefType::Isize => format!("(Int({raw}) ?? 0)"),
        FieldDefType::F32 | FieldDefType::F64 => format!("(Double({raw}) ?? 0)"),
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

/// Escapes a path literal for a double-quoted Swift string: a backslash and a double quote both
/// need escaping so a literal segment cannot open `\(...)` interpolation or close the literal
/// early; a path template is ASCII by construction, so nothing else does.
fn swift_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(ch, '\\' | '"') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}
