//! The Swift `http_rest` client: one transport seam the app implements, one `async` method per
//! operation, mirroring the Dart client's request grammar statement for statement.

use crate::features::swift::{forms, swift_field_member, swift_member, swift_quoted};
use crate::features::text_forms::TextForm;
use crate::field_type::{FieldDefType, get_field_def};
use crate::rename_rule::RenameRule;
use crate::service_schema::parse::{
    BodyKind, DEFAULT_BINDING_ERROR_STATUS, HttpShape, OperationDef, OperationInputs,
    OperationOutcome, PathSegment, ServiceDef, is_scalar_named_type, is_unit_type, option_inner,
    path_reads_a_named_field, service_declares_a_stream, service_declares_multipart,
    tuple_elements, vec_inner, wire_key, written,
};
use crate::service_schema::support::fault_fields_typescript_name;
use core::fmt::Write as _;
use syn::{Ident, Type};

use super::swift_type::swift_typename_of;
use super::{on_one_line, text_types};

/// The locals and parameters an emitted method writes around an operation's own argument: the
/// REST client's and the `ws_rpc` client's.
const TAKEN_BY_A_METHOD: [&str; 24] = [
    "answer",
    "body",
    "contentRange",
    "contentType",
    "declared",
    "declaredHead",
    "decoded",
    "error",
    "errorHeaderValues",
    "fault",
    "headerValues",
    "headers",
    "ok",
    "parts",
    "path",
    "probed",
    "query",
    "raw",
    "rendered",
    "req",
    "response",
    "status",
    "transport",
    "value",
];

/// The Swift type a `body = "stream"` operation's success answers with: a nullable `contentRange`
/// and the `contentType`, paired with the body as an `AsyncThrowingStream<Data, Error>`.
const STREAMED_ANSWER_SWIFT_TYPE: &str =
    "(contentRange: String?, contentType: String, body: AsyncThrowingStream<Data, Error>)";

/// One text a statement writes: the expression of its value, and whether that expression throws,
/// the refusal of a value no form serde reads holds.
struct SwiftText {
    expr: String,
    throws: bool,
}

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
            RenameRule::PascalCase.apply_to_field(&written(&operation.ident))
        )),
    }
}

/// `Result`'s own `Failure` parameter requires `Error`, so every failure conforms to it beside
/// `Sendable`. Not `Equatable`: the generated message and fault types this enum carries do not
/// conform to it.
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
    let forms = forms::helpers(&text_forms(service), false);
    let texts = if forms.is_empty() {
        String::new()
    } else {
        format!("  {}\n\n", forms.trim_end())
    };
    format!(
        "/// A `{named}` caller over `http_rest`.\n\
         public struct {named}HttpClient: Sendable {{\n  \
         private let transport: any {named}HttpTransport\n\n  \
         public init(transport: any {named}HttpTransport) {{ self.transport = transport }}\n\n\
         {texts}{methods}\n\
         }}"
    )
}

/// The Swift name of the method that calls `operation`.
pub(super) fn swift_call(operation: &OperationDef) -> String {
    swift_member(&operation.ts_name)
}

/// The name an emitted method reads an operation's own argument by: the Rust identifier, never
/// re-cased, since it is a function argument rather than a message property, and moved off a
/// name the method writes itself, which Swift would read it as without a word.
pub(super) fn swift_parameter(parameter: &Ident) -> String {
    let name = written(parameter);
    if TAKEN_BY_A_METHOD.contains(&name.as_str()) {
        format!("{name}_")
    } else {
        swift_member(&name)
    }
}

/// An operation's own argument as a method declares it: the label a caller writes, which never
/// moves, and after it the name the method reads it by where that one did.
pub(super) fn swift_parameter_declared(parameter: &Ident) -> String {
    let label = swift_member(&written(parameter));
    let read_by = swift_parameter(parameter);
    if label == read_by {
        label
    } else {
        format!("{label} {read_by}")
    }
}

/// One argument per `header_in` binding, then one per `part` binding, after the message —
/// mirrors the Dart client's own `method_params`.
fn method_params(operation: &OperationDef, shape: &HttpShape) -> String {
    let mut params = vec![format!("_ req: {}", message_swift_typename(operation))];
    for header in &shape.header_in {
        params.push(format!(
            "{}: {}",
            swift_parameter_declared(&header.parameter),
            swift_typename_of(&header.ty)
        ));
    }
    for part in &shape.multipart_parts {
        params.push(format!(
            "{}: {}",
            swift_parameter_declared(&part.parameter),
            swift_typename_of(&part.ty)
        ));
    }
    params.join(", ")
}

fn method_doc(operation: &OperationDef, shape: &HttpShape) -> String {
    format!(
        "  /// Calls `{}` over `{} {}`.",
        on_one_line(&operation.wire_name),
        shape.method.name(),
        on_one_line(&shape.path_template())
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
    let wire = &swift_quoted(&operation.wire_name);
    let call = swift_call(operation);
    let params = method_params(operation, &shape);
    let returns = return_type(named, operation);
    let recover = encode_failure_recovery(named, fn_prefix, wire, &operation.outcome);
    let path_build = path_build_stmt(fn_prefix, &recover, operation, &shape);
    let query_build = query_build_stmt(fn_prefix, &recover, operation, &shape);
    let headers_build = header_in_build_stmt(named, fn_prefix, &recover, operation, &shape);
    let body_build = body_build_stmt(named, fn_prefix, wire, operation, &shape);
    let parts_build =
        multipart_parts_build_stmt(fn_prefix, &recover, operation, &shape, has_multipart);
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

/// The Swift expression that renders one path placeholder's value: the field it names (a message
/// the macro declared, or an author's own struct), or the whole message where that message
/// already is a wire scalar — mirrors the Dart client's own `placeholder_value_dart_expr`.
fn placeholder_value_swift_expr(
    fn_prefix: &str,
    operation: &OperationDef,
    shape: &HttpShape,
    placeholder: &str,
) -> SwiftText {
    let accessor = swift_field_member(placeholder);
    let interpolated = || SwiftText {
        expr: format!("\"\\(req.{accessor})\""),
        throws: false,
    };
    match &operation.inputs {
        OperationInputs::Empty => interpolated(),
        OperationInputs::Generated(fields) => fields
            .iter()
            .find(|(field, _)| written(field) == placeholder)
            .map_or_else(interpolated, |(_, ty)| {
                swift_wire_text(fn_prefix, ty, &format!("req.{accessor}"))
            }),
        OperationInputs::Named(declared) => {
            if shape.placeholder_names().len() == 1 && is_scalar_named_type(declared) {
                swift_wire_text(fn_prefix, declared, "req")
            } else {
                SwiftText {
                    expr: format!("try {fn_prefix}WireText(req.{accessor})"),
                    throws: true,
                }
            }
        }
    }
}

/// The statements at `margin` that bind `rendered` to `text`, answering `recover` where its
/// expression throws.
fn bound_text(text: &SwiftText, recover: &str, margin: &str) -> String {
    if !text.throws {
        return format!("{margin}let rendered = {}\n", text.expr);
    }
    format!(
        "{margin}let rendered: String\n\
         {margin}do {{\n\
         {margin}  rendered = {}\n\
         {margin}}} catch {{\n\
         {margin}  {recover}\n\
         {margin}}}\n",
        text.expr
    )
}

fn path_build_stmt(
    fn_prefix: &str,
    recover: &str,
    operation: &OperationDef,
    shape: &HttpShape,
) -> String {
    let mut stmt = String::from("    var path = \"\"\n");
    for segment in &shape.path {
        match segment {
            PathSegment::Literal(text) => {
                let _ = writeln!(stmt, "    path += {}", swift_quoted(text));
            }
            PathSegment::Placeholder(name) => {
                let value = placeholder_value_swift_expr(fn_prefix, operation, shape, name);
                if value.throws {
                    let _ = write!(
                        stmt,
                        "    do {{\n{}      path += {fn_prefix}PercentEncode(rendered)\n    }}\n",
                        bound_text(&value, recover, "      ")
                    );
                } else {
                    let _ = writeln!(stmt, "    path += {fn_prefix}PercentEncode({})", value.expr);
                }
            }
        }
    }
    stmt
}

fn query_build_stmt(
    fn_prefix: &str,
    recover: &str,
    operation: &OperationDef,
    shape: &HttpShape,
) -> String {
    if shape.method.carries_a_body() {
        return "    let query: [(String, String)] = []\n".to_owned();
    }
    let fields = match &operation.inputs {
        // `Empty` sends no field.
        OperationInputs::Empty | OperationInputs::Named(_) => {
            return "    let query: [(String, String)] = []\n".to_owned();
        }
        OperationInputs::Generated(fields) => fields,
    };
    let placeholders = shape.placeholder_names();
    let mut pushes = String::new();
    for (field, ty) in fields {
        let field_name = written(field);
        if placeholders.contains(&field_name) {
            continue;
        }
        let key = wire_key(field);
        let accessor = swift_field_member(&field_name);
        let inner = option_inner(ty).unwrap_or(ty);
        let rendered = swift_wire_text(fn_prefix, inner, "value");
        let _ = write!(
            pushes,
            "    if let value = req.{accessor} {{\n{}      \
             query.append((\"{key}\", rendered))\n    \
             }}\n",
            bound_text(&rendered, recover, "      ")
        );
    }
    if pushes.is_empty() {
        return "    let query: [(String, String)] = []\n".to_owned();
    }
    format!("    var query: [(String, String)] = []\n{pushes}")
}

/// Builds the outgoing header list, one entry per `header_in` binding - a `nil` optional binding
/// is added nowhere, and every rendered value is checked before it is appended.
fn header_in_build_stmt(
    named: &str,
    fn_prefix: &str,
    recover: &str,
    operation: &OperationDef,
    shape: &HttpShape,
) -> String {
    if shape.header_in.is_empty() {
        return "    let headers: [(String, String)] = []\n".to_owned();
    }
    let mut stmt = String::from("    var headers: [(String, String)] = []\n");
    for header in &shape.header_in {
        let name = swift_quoted(&header.name);
        let parameter = swift_parameter(&header.parameter);
        let fault_expr = format!(
            "{fn_prefix}HttpOutboundFault({}, {name}, \"a header value contains a \
             character illegal in an HTTP header\")",
            swift_quoted(&operation.wire_name),
        );
        let refusal = outbound_refusal_stmt(named, operation, &fault_expr);
        let inner = option_inner(&header.ty);
        let text = swift_wire_text(fn_prefix, inner.unwrap_or(&header.ty), &parameter);
        let opened = if inner.is_some() {
            format!("if let {parameter} = {parameter}")
        } else {
            "do".to_owned()
        };
        let _ = writeln!(
            stmt,
            "    {opened} {{\n{}      \
             if !{fn_prefix}LegalHeaderValue(rendered) {{\n        {refusal}\n      \
             }}\n      \
             headers.append(({name}, rendered))\n    \
             }}",
            bound_text(&text, recover, "      ")
        );
    }
    stmt
}

/// What a `header_in` value failing the safety check answers, before the transport is ever
/// reached — mirrors [`encode_failure_recovery`]'s own `OneWay`/`Reply` split.
fn outbound_refusal_stmt(named: &str, operation: &OperationDef, fault_expr: &str) -> String {
    match &operation.outcome {
        OperationOutcome::OneWay => format!("throw {named}Refusal(fault: {fault_expr})"),
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => format!("return .failure(.fault({fault_expr}))"),
    }
}

fn body_build_stmt(
    named: &str,
    fn_prefix: &str,
    wire: &str,
    operation: &OperationDef,
    shape: &HttpShape,
) -> String {
    if encodes_a_body(shape) {
        json_encode_body_stmt(named, fn_prefix, wire, operation)
    } else {
        "    let body = Data()\n".to_owned()
    }
}

/// Whether a method writes its message as a JSON body: every method that carries a body, but a
/// multipart one, whose message rides in its parts.
const fn encodes_a_body(shape: &HttpShape) -> bool {
    shape.method.carries_a_body() && !matches!(shape.body_kind, BodyKind::Multipart)
}

/// The message as a JSON body, through [`dated_encoder_fn`]'s encoder, a `char` or a date the
/// message is in the form serde writes.
fn json_encode_body_stmt(
    named: &str,
    fn_prefix: &str,
    wire: &str,
    operation: &OperationDef,
) -> String {
    let recover = encode_failure_recovery(named, fn_prefix, wire, &operation.outcome);
    let sent = match &operation.inputs {
        OperationInputs::Named(declared) => swift_sent_value(declared, "req"),
        OperationInputs::Empty | OperationInputs::Generated(_) => None,
    };
    format!(
        "    let body: Data\n    \
         do {{\n      \
         body = try {fn_prefix}DatedEncoder().encode({})\n    \
         }} catch {{\n      \
         {recover}\n    \
         }}\n",
        sent.as_deref().unwrap_or("req")
    )
}

/// The expression that writes `expr`, a value of `ty` Swift holds a `char` or a date of as a
/// `String`, in the form serde writes, under an `Option` and a list alike; `None` where `ty`
/// reaches no such text. It throws for a text no rewrite turns into its form.
pub(super) fn swift_sent_value(ty: &Type, expr: &str) -> Option<String> {
    if let Some(inner) = option_inner(ty) {
        let held = swift_sent_value(inner, "held")?;
        return Some(format!("try ({expr}).map {{ held in {held} }}"));
    }
    if let Some(inner) = vec_inner(ty) {
        let element = swift_sent_value(inner, "element")?;
        return Some(format!("try ({expr}).map {{ element in {element} }}"));
    }
    forms::held_as_text(&get_field_def("value", ty, ""))
        .map(|form| forms::sent_call_at(form, expr, "[]"))
}

/// What a method answers for a value it cannot write, before the transport is reached: a message
/// its encoder refuses, and a text no form serde reads holds.
fn encode_failure_recovery(
    named: &str,
    fn_prefix: &str,
    wire: &str,
    outcome: &OperationOutcome,
) -> String {
    let fault = format!("{fn_prefix}UndeserializablePayload({wire}, \"\\(error)\")");
    match outcome {
        OperationOutcome::OneWay => format!("throw {named}Refusal(fault: {fault})"),
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => format!("return .failure(.fault({fault}))"),
    }
}

/// The `parts` a `body = "multipart"` method sends: one text entry per carried `Generated` field
/// not otherwise placeholder-bound, then one per declared `part` binding, passed through
/// untouched. Every other body kind on a multipart-declaring service still builds an empty one.
fn multipart_parts_build_stmt(
    fn_prefix: &str,
    recover: &str,
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
            let field_name = written(field);
            if placeholders.contains(&field_name) {
                continue;
            }
            let key = wire_key(field);
            let accessor = swift_field_member(&field_name);
            if let Some(inner) = option_inner(ty) {
                let text = swift_wire_text(fn_prefix, inner, "value");
                let _ = write!(
                    stmt,
                    "    if let value = req.{accessor} {{\n{}      \
                     parts.append((\"{key}\", rendered))\n    \
                     }}\n",
                    bound_text(&text, recover, "      ")
                );
            } else {
                let text = swift_wire_text(fn_prefix, ty, &format!("req.{accessor}"));
                if text.throws {
                    let _ = write!(
                        stmt,
                        "    do {{\n{}      parts.append((\"{key}\", rendered))\n    }}\n",
                        bound_text(&text, recover, "      ")
                    );
                } else {
                    let _ = writeln!(stmt, "    parts.append((\"{key}\", {}))", text.expr);
                }
            }
        }
    }
    for part in &shape.multipart_parts {
        let name = swift_quoted(&part.name);
        let parameter = swift_parameter(&part.parameter);
        let _ = writeln!(stmt, "    parts.append(({name}, {parameter}))");
    }
    stmt
}

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
         throw {named}Refusal(fault: {fn_prefix}TransportFailure({wire}, \"\\(error)\"))\n    \
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
         return .failure(.fault({fn_prefix}TransportFailure({wire}, \"\\(error)\")))\n    \
         }}\n"
    )
}

fn one_way_decode_stmt(named: &str, fn_prefix: &str, shape: &HttpShape, wire: &str) -> String {
    let ok_status = shape.ok_status;
    format!(
        "    let status = response.status\n    \
         if status == {ok_status} {{ return }}\n    \
         if status == 400 || status == 404 || status == 500 {{\n      \
         throw {named}Refusal(fault: {fn_prefix}FaultFromBody({wire}, response.body))\n    \
         }}\n    \
         throw {named}Refusal(\n      \
         fault: {fn_prefix}UndeserializablePayload(\n        \
         {wire}, \"an unexpected status (\\(status)) answered\"\n      \
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
    let success_block = success_decode_block(fn_prefix, wire, shape, success);
    let error_block = error_decode_block(fn_prefix, wire, shape, error);
    format!(
        "    let status = response.status\n    \
         if status == {ok_status} {{\n{success_block}    }}\n    \
         if {error_condition} {{\n{error_block}    \
         }}\n    \
         if status == 400 || status == 404 || status == 500 {{\n      \
         return .failure(.fault({fn_prefix}FaultFromBody({wire}, response.body)))\n    \
         }}\n    \
         return .failure(.fault(\n      \
         {fn_prefix}UndeserializablePayload(\n        \
         {wire}, \"an unexpected status (\\(status)) answered\"\n      \
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
    let partial = stream_success_arm(fn_prefix, wire, shape, success, true);
    let full = stream_success_arm(fn_prefix, wire, shape, success, false);
    let error_block = error_decode_block(fn_prefix, wire, shape, error);
    format!(
        "    let status = response.status\n    \
         if status == 206 {{\n{partial}    }}\n    \
         if status == {ok_status} {{\n{full}    }}\n    \
         if {error_condition} {{\n{error_block}    \
         }}\n    \
         if status == 400 || status == 404 || status == 500 {{\n      \
         return .failure(.fault({fn_prefix}FaultFromBody({wire}, response.body)))\n    \
         }}\n    \
         return .failure(.fault(\n      \
         {fn_prefix}UndeserializablePayload(\n        \
         {wire}, \"an unexpected status (\\(status)) answered\"\n      \
         )\n    \
         ))\n"
    )
}

/// One status arm of [`stream_reply_decode_stmt`]: `contentRange` read back off the response for
/// a `206` partial answer, left `nil` for the declared `ok_status`'s whole-body answer, both with
/// `contentType` read back the way a bytes reply reads it, then every declared `header_out` element
/// read back exactly as the bytes and JSON paths do.
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
    let _ = write!(
        stmt,
        "      let contentType = {fn_prefix}FindHeader(response.headers, \"content-type\") \
         ?? \"\"\n      \
         let answer = (contentRange: contentRange, contentType: contentType, body: \
         response.bodyStream)\n"
    );
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
    header_value_read_stmts(
        fn_prefix,
        wire,
        &shape.header_out,
        elements,
        body_elements,
        "headerOut",
    )
}

/// [`header_out_read_stmts`]'s own general form, shared with the error side: `ident_prefix`
/// names the locals apart so an operation declaring both never binds two under one name. A
/// `Boolean` or numeric element (bare or the item type of a `Vec`) reads through
/// [`fallible_header_read_stmt`] instead, since a present value there can still fail to decode.
fn header_value_read_stmts(
    fn_prefix: &str,
    wire: &str,
    names: &[String],
    elements: &[&Type],
    body_elements: usize,
    ident_prefix: &str,
) -> (String, Vec<String>) {
    let mut stmt = String::new();
    let mut idents = Vec::new();
    let capitalized_prefix = RenameRule::PascalCase.apply_to_field(ident_prefix);
    for (index, (header, element_ty)) in names
        .iter()
        .zip(elements.iter().skip(body_elements))
        .enumerate()
    {
        let name = &swift_quoted(header);
        let ident = format!("{ident_prefix}{index}");
        let raw_ident = format!("raw{capitalized_prefix}{index}");
        let bare_ty = option_inner(element_ty).unwrap_or(element_ty);
        if header_read_is_fallible(bare_ty) {
            let _ = write!(
                stmt,
                "{}",
                fallible_header_read_stmt(fn_prefix, wire, name, &ident, &raw_ident, element_ty)
            );
        } else if option_inner(element_ty).is_some() {
            let decode = swift_header_out_decode(fn_prefix, element_ty, "raw");
            let _ = write!(
                stmt,
                "      let {raw_ident} = {fn_prefix}FindHeader(response.headers, {name})\n      \
                 let {ident} = {raw_ident}.map {{ raw in {decode} }}\n",
            );
        } else {
            let decode = swift_header_out_decode(fn_prefix, element_ty, &raw_ident);
            let _ = write!(
                stmt,
                "      guard let {raw_ident} = {fn_prefix}FindHeader(response.headers, {name}) \
                 else {{\n        \
                 return .failure(.fault(\n          \
                 {fn_prefix}UndeserializablePayload({wire}, \"a declared response header was missing\")\n        \
                 ))\n      \
                 }}\n      \
                 let {ident} = {decode}\n",
            );
        }
        idents.push(ident);
    }
    (stmt, idents)
}

/// Whether `bare_ty` (an element's own type, its outer `Option` already stripped) can fail to
/// decode from header text — every `Boolean`, numeric and declared kind, bare or as a `Vec`'s own
/// item type.
fn header_read_is_fallible(bare_ty: &Type) -> bool {
    let leaf = vec_inner(bare_ty).unwrap_or(bare_ty);
    let read = get_field_def("value", leaf, "");
    #[cfg(feature = "chrono")]
    if matches!(read.field_type, FieldDefType::DateTime) {
        return true;
    }
    matches!(
        read.field_type,
        FieldDefType::Boolean
            | FieldDefType::U8
            | FieldDefType::U16
            | FieldDefType::U32
            | FieldDefType::U64
            | FieldDefType::U64Digits
            | FieldDefType::I8
            | FieldDefType::I16
            | FieldDefType::I32
            | FieldDefType::I64
            | FieldDefType::I64Digits
            | FieldDefType::Usize
            | FieldDefType::Isize
            | FieldDefType::F32
            | FieldDefType::F64
            | FieldDefType::SiblingType(_, _)
    )
}

/// One `Boolean`/numeric `header_out` (or `error_header_out`) element: a present value that will
/// not become the declared type answers the same `UndeserializablePayload` fault a missing
/// required header does, rather than the silent `0`/`false` an earlier build fell back to.
fn fallible_header_read_stmt(
    fn_prefix: &str,
    wire: &str,
    name: &str,
    ident: &str,
    raw_ident: &str,
    element_ty: &Type,
) -> String {
    let bad_type_fault = format!(
        "return .failure(.fault(\n          \
         {fn_prefix}UndeserializablePayload({wire}, \"a response header did not match its \
         declared type\")\n        \
         ))"
    );
    let bare_ty = option_inner(element_ty).unwrap_or(element_ty);
    if option_inner(element_ty).is_some() {
        let full_ty = swift_typename_of(element_ty);
        let assign =
            fallible_assign_stmt(fn_prefix, ident, bare_ty, raw_ident, &bad_type_fault, false);
        format!(
            "      let {raw_ident} = {fn_prefix}FindHeader(response.headers, {name})\n      \
             let {ident}: {full_ty}\n      \
             if let {raw_ident} {{\n\
{assign}      \
             }} else {{\n        \
             {ident} = nil\n      \
             }}\n"
        )
    } else {
        let assign =
            fallible_assign_stmt(fn_prefix, ident, bare_ty, raw_ident, &bad_type_fault, true);
        format!(
            "      guard let {raw_ident} = {fn_prefix}FindHeader(response.headers, {name}) \
             else {{\n        \
             return .failure(.fault(\n          \
             {fn_prefix}UndeserializablePayload({wire}, \"a declared response header was \
             missing\")\n        \
             ))\n      \
             }}\n\
{assign}"
        )
    }
}

/// The statements that give `{ident}` its value once `raw_ident` is known to hold the header's
/// own text: a scalar `guard let`, or a `Vec` whose every piece decodes before any is trusted,
/// faulting if one is `nil`. `declare` says whether this is `{ident}`'s own first appearance (a
/// required element) or an assignment into an already-declared `Optional` (an optional one).
fn fallible_assign_stmt(
    fn_prefix: &str,
    ident: &str,
    bare_ty: &Type,
    raw_ident: &str,
    fault_stmt: &str,
    declare: bool,
) -> String {
    match vec_inner(bare_ty) {
        Some(item_ty) => {
            let keyword = if declare { "let " } else { "" };
            let item = swift_typename_of(item_ty);
            let item_expr = swift_header_out_decode(fn_prefix, item_ty, "piece");
            format!(
                "      let {ident}Pieces = {raw_ident}.split(separator: \",\").map {{ piece -> \
                 String in String(piece) }}\n      \
                 let {ident}Decoded = {ident}Pieces.map {{ piece -> {item}? in {item_expr} \
                 }}\n      \
                 guard {ident}Decoded.allSatisfy({{ $0 != nil }}) else {{\n        \
                 {fault_stmt}\n      \
                 }}\n      \
                 {keyword}{ident} = {ident}Decoded.map {{ $0! }}\n"
            )
        }
        None if declare => {
            let expr = swift_header_out_decode(fn_prefix, bare_ty, raw_ident);
            format!("      guard let {ident} = {expr} else {{\n        {fault_stmt}\n      }}\n")
        }
        None => {
            let expr = swift_header_out_decode(fn_prefix, bare_ty, raw_ident);
            format!(
                "      guard let {ident}Decoded = {expr} else {{\n        {fault_stmt}\n      \
                 }}\n      \
                 {ident} = {ident}Decoded\n"
            )
        }
    }
}

/// The declared-error read every reply-decoding arm shares: the error's own head off the body,
/// plus each `error_header_out` element off its own response header where any were declared.
fn error_decode_block(fn_prefix: &str, wire: &str, shape: &HttpShape, error: &Type) -> String {
    if shape.error_header_out.is_empty() {
        let error_ty = swift_typename_of(error);
        return format!(
            "      let declared: {error_ty}\n      \
             do {{\n        \
             declared = try {fn_prefix}DatedDecoder().decode({error_ty}.self, from: \
             response.body)\n      \
             }} catch {{\n        \
             return .failure(.fault(\n          \
             {fn_prefix}UndeserializablePayload({wire}, \"\\(error)\")\n        \
             ))\n      \
             }}\n      \
             return .failure(.declared(declared))\n"
        );
    }
    let elements: Vec<&Type> = tuple_elements(error).into_iter().flatten().collect();
    let head_ty = elements
        .first()
        .map_or_else(|| swift_typename_of(error), |ty| swift_typename_of(ty));
    let mut stmt = format!(
        "      let declaredHead: {head_ty}\n      \
         do {{\n        \
         declaredHead = try {fn_prefix}DatedDecoder().decode({head_ty}.self, from: \
         response.body)\n      \
         }} catch {{\n        \
         return .failure(.fault(\n          \
         {fn_prefix}UndeserializablePayload({wire}, \"\\(error)\")\n        \
         ))\n      \
         }}\n"
    );
    let (header_stmts, header_idents) = header_value_read_stmts(
        fn_prefix,
        wire,
        &shape.error_header_out,
        &elements,
        1,
        "errorHeaderOut",
    );
    stmt.push_str(&header_stmts);
    let _ = writeln!(
        stmt,
        "      return .failure(.declared((declaredHead, {})))",
        header_idents.join(", ")
    );
    stmt
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
             value = try {fn_prefix}DatedDecoder().decode({success_ty}.self, from: \
             response.body)\n      \
             }} catch {{\n        \
             return .failure(.fault(\n          \
             {fn_prefix}UndeserializablePayload({wire}, \"\\(error)\")\n        \
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
         value = try {fn_prefix}DatedDecoder().decode({body_ty}.self, from: response.body)\n      \
         }} catch {{\n        \
         return .failure(.fault(\n          \
         {fn_prefix}UndeserializablePayload({wire}, \"\\(error)\")\n        \
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

fn fault_helpers(service: &ServiceDef, named: &str, fn_prefix: &str) -> Vec<String> {
    let mut helpers = vec![percent_encode_fn(fn_prefix), query_text_fn(fn_prefix)];
    if reads_a_response_header(service) {
        helpers.push(find_header_fn(fn_prefix));
    }
    let writes_text = service.operations.iter().any(|operation| {
        path_reads_a_named_field(operation)
            || text_types(operation).iter().any(writes_through_wire_text)
    });
    if writes_text {
        helpers.push(wire_text_fn(fn_prefix));
    }
    if writes_text
        || service
            .operations
            .iter()
            .any(|operation| encodes_a_body(&HttpShape::of(operation)))
    {
        helpers.push(dated_encoder_fn(fn_prefix));
    }
    if service
        .operations
        .iter()
        .any(|operation| !matches!(operation.outcome, OperationOutcome::OneWay))
    {
        helpers.push(dated_decoder_fn(fn_prefix));
    }
    helpers.extend([
        transport_failure_fn(named, fn_prefix),
        undeserializable_payload_fn(named, fn_prefix),
        fault_from_body_fn(named, fn_prefix),
    ]);
    if declares_header_in(service) {
        helpers.push(legal_header_value_fn(fn_prefix));
        helpers.push(outbound_fault_fn(named, fn_prefix));
    }
    helpers
}

/// Read through [`HttpShape::of`], the shape every method above is built from, so the gate and
/// the methods cannot answer differently for an operation with no `http(...)` group.
fn reads_a_response_header(service: &ServiceDef) -> bool {
    service.operations.iter().any(|operation| {
        let shape = HttpShape::of(operation);
        !shape.header_out.is_empty()
            || !shape.error_header_out.is_empty()
            || matches!(shape.body_kind, BodyKind::Bytes | BodyKind::Stream)
    })
}

/// Whether the service declares an operation carrying at least one `header_in` binding, which is
/// what needs the outbound header-safety checker.
fn declares_header_in(service: &ServiceDef) -> bool {
    service
        .operations
        .iter()
        .any(|operation| !HttpShape::of(operation).header_in.is_empty())
}

/// Whether every byte of `value` is legal as an HTTP header value: visible ASCII (`0x21`-`0x7E`),
/// a space, or a tab.
fn legal_header_value_fn(fn_prefix: &str) -> String {
    format!(
        "/// Whether every byte of `value` is legal as an HTTP header value: visible ASCII\n\
         /// (`0x21`-`0x7E`), a space, or a tab.\n\
         func {fn_prefix}LegalHeaderValue(_ value: String) -> Bool {{\n  \
         for scalar in value.unicodeScalars {{\n    \
         let code = scalar.value\n    \
         if code != 0x09 && code != 0x20 && (code < 0x21 || code > 0x7e) {{ return false }}\n  \
         }}\n  \
         return true\n\
         }}"
    )
}

/// The fault a `header_in` value that fails [`legal_header_value_fn`]'s own check answers with,
/// before the transport is ever reached.
fn outbound_fault_fn(named: &str, fn_prefix: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    let kind = format!("{named}FaultKind");
    format!(
        "/// The fault a `{named}` HTTP client answers with when a `header_in` value fails its\n\
         /// own safety check, before the transport is ever reached.\n\
         func {fn_prefix}HttpOutboundFault(_ operation: String, _ field: String, _ detail: \
         String) -> {fields} {{\n  \
         {fields}(detail: detail, field: field, kind: {kind}.failedValidation, operation: \
         operation)\n\
         }}"
    )
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

/// A value as the text a path segment, a query, a header or a part carries, as the Rust client
/// writes it: a string as itself, anything else as its JSON, through [`dated_encoder_fn`]'s
/// encoder. Throws for a value the encoder refuses, a `Double` NaN among them.
fn wire_text_fn(fn_prefix: &str) -> String {
    format!(
        "func {fn_prefix}WireText<Value: Encodable>(_ value: Value) throws -> String {{\n  \
         let encoder = {fn_prefix}DatedEncoder()\n  \
         encoder.outputFormatting = [.withoutEscapingSlashes]\n  \
         let data = try encoder.encode(value)\n  \
         if let text = try? JSONDecoder().decode(String.self, from: data) {{\n    \
         return text\n  \
         }}\n  \
         return String(decoding: data, as: UTF8.self)\n\
         }}"
    )
}

/// The encoder a message and a value written as text go through: a `Date` as the RFC 3339 text
/// with its fraction the model codec writes, where `JSONEncoder` alone writes a number.
fn dated_encoder_fn(fn_prefix: &str) -> String {
    format!(
        "/// A `JSONEncoder` that writes a `Date` as the RFC 3339 text serde reads.\n\
         func {fn_prefix}DatedEncoder() -> JSONEncoder {{\n  \
         let encoder = JSONEncoder()\n  \
         encoder.dateEncodingStrategy = .custom {{ date, encoder in\n    \
         let formatter = ISO8601DateFormatter()\n    \
         formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]\n    \
         var container = encoder.singleValueContainer()\n    \
         try container.encode(formatter.string(from: date))\n  \
         }}\n  \
         return encoder\n\
         }}"
    )
}

/// The decoder a reply goes through: a `Date` from the RFC 3339 text serde writes, with a fraction
/// or with none, where `JSONDecoder` alone reads a number.
fn dated_decoder_fn(fn_prefix: &str) -> String {
    format!(
        "/// A `JSONDecoder` that reads a `Date` from the RFC 3339 text serde writes.\n\
         func {fn_prefix}DatedDecoder() -> JSONDecoder {{\n  \
         let decoder = JSONDecoder()\n  \
         decoder.dateDecodingStrategy = .custom {{ decoder in\n    \
         let container = try decoder.singleValueContainer()\n    \
         let text = try container.decode(String.self)\n    \
         let fractional = ISO8601DateFormatter()\n    \
         fractional.formatOptions = [.withInternetDateTime, .withFractionalSeconds]\n    \
         let whole = ISO8601DateFormatter()\n    \
         whole.formatOptions = [.withInternetDateTime]\n    \
         guard let date = fractional.date(from: text) ?? whole.date(from: text) else {{\n      \
         throw DecodingError.dataCorruptedError(in: container, debugDescription: \"expected a \
         date and time with its zone, found \\(text)\")\n    \
         }}\n    \
         return date\n  \
         }}\n  \
         return decoder\n\
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

/// The Swift expression that renders `ty`'s value at `expr` as URL- or header-safe text: an
/// `Option<T>` reads as the empty string when absent, a `Vec<T>` joins its elements' own text
/// with a comma, a `String`, a `Bool` and an integer interpolate as the text serde reads, a
/// `char` and a date Swift holds as a `String` are written in the form serde writes, and any
/// other value is written through [`wire_text_fn`]'s encoder, as it is in a body. The two last
/// throw for a value no form serde reads holds.
fn swift_wire_text(fn_prefix: &str, ty: &Type, expr: &str) -> SwiftText {
    if let Some(inner) = option_inner(ty) {
        let rendered = swift_wire_text(fn_prefix, inner, "unwrapped");
        let tried = if rendered.throws { "try " } else { "" };
        return SwiftText {
            expr: format!(
                "({tried}({expr}).map {{ unwrapped in {} }} ?? \"\")",
                rendered.expr
            ),
            throws: rendered.throws,
        };
    }
    if let Some(inner) = vec_inner(ty) {
        let element = swift_wire_text(fn_prefix, inner, "element");
        let tried = if element.throws { "try " } else { "" };
        return SwiftText {
            expr: format!(
                "({tried}({expr}).map {{ element in {} }}.joined(separator: \",\"))",
                element.expr
            ),
            throws: element.throws,
        };
    }
    let field = get_field_def("value", ty, "");
    if let Some(form) = forms::held_as_text(&field) {
        return SwiftText {
            expr: forms::sent_call_at(form, expr, "[]"),
            throws: true,
        };
    }
    if interpolates(&field.field_type) {
        return SwiftText {
            expr: format!("\"\\({expr})\""),
            throws: false,
        };
    }
    SwiftText {
        expr: format!("try {fn_prefix}WireText({expr})"),
        throws: true,
    }
}

/// Whether Swift's string interpolation writes a value of `field_type` as the text serde reads:
/// a `String`, a `Bool` and an integer.
const fn interpolates(field_type: &FieldDefType) -> bool {
    matches!(
        field_type,
        FieldDefType::String
            | FieldDefType::StringLiteral(_)
            | FieldDefType::Boolean
            | FieldDefType::BooleanLiteral(_)
            | FieldDefType::U8
            | FieldDefType::U16
            | FieldDefType::U32
            | FieldDefType::U64
            | FieldDefType::I8
            | FieldDefType::I16
            | FieldDefType::I32
            | FieldDefType::I64
            | FieldDefType::Usize
            | FieldDefType::Isize
    )
}

/// Whether [`swift_wire_text`] writes a value of `ty` through [`wire_text_fn`].
fn writes_through_wire_text(ty: &Type) -> bool {
    let base = option_inner(ty).unwrap_or(ty);
    if let Some(inner) = vec_inner(base) {
        return writes_through_wire_text(inner);
    }
    let field = get_field_def("value", base, "");
    forms::held_as_text(&field).is_none() && !interpolates(&field.field_type)
}

/// The forms of the texts the service's client writes as a `String` Swift holds, as text or in a
/// message written whole as the body, once each, in the order its operations first write them.
fn text_forms(service: &ServiceDef) -> Vec<TextForm> {
    let mut found = Vec::new();
    for operation in &service.operations {
        let mut written = text_types(operation);
        if let OperationInputs::Named(declared) = &operation.inputs
            && encodes_a_body(&HttpShape::of(operation))
        {
            written.push(declared.as_ref().clone());
        }
        for ty in written {
            let mut base = &ty;
            while let Some(inner) = option_inner(base).or_else(|| vec_inner(base)) {
                base = inner;
            }
            if let Some(form) = forms::held_as_text(&get_field_def("value", base, ""))
                && !found.contains(&form)
            {
                found.push(form);
            }
        }
    }
    found
}

/// The expression that reads one `header_out` element's declared type back off `raw`, a
/// non-optional `String` already checked non-missing. `Boolean` and every numeric kind answer an
/// `Optional`, `nil` on a value that will not become the declared type — read through
/// [`header_read_is_fallible`] before a scalar (non-`Vec`) slot of one is ever reached this way.
/// Every other kind still answers the value directly, since it cannot fail to decode.
fn swift_header_out_decode(fn_prefix: &str, ty: &Type, raw: &str) -> String {
    let base = option_inner(ty).unwrap_or(ty);
    if let Some(inner) = vec_inner(base) {
        let element = swift_header_out_decode(fn_prefix, inner, "piece");
        return format!(
            "{raw}.split(separator: \",\").map {{ piece -> String in String(piece) }}\n        \
             .map {{ piece in {element} }}"
        );
    }
    match get_field_def("value", base, "").field_type {
        FieldDefType::Boolean => {
            format!("({raw} == \"true\" ? true : ({raw} == \"false\" ? false : nil))")
        }
        FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::U64Digits
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::I64Digits
        | FieldDefType::Usize
        | FieldDefType::Isize
        | FieldDefType::F32
        | FieldDefType::F64 => format!("{}({raw})", swift_typename_of(base)),
        // The header text read as a JSON string, as the Rust client reads it, through the type's
        // own `Decodable` conformance and a `Date` as the model codec reads one; one that refuses
        // it is a malformed header.
        FieldDefType::SiblingType(_, _) => format!(
            "(try? {fn_prefix}DatedDecoder().decode({}.self, from: JSONEncoder().encode({raw})))",
            swift_typename_of(base)
        ),
        #[cfg(feature = "chrono")]
        FieldDefType::DateTime => format!(
            "(try? {fn_prefix}DatedDecoder().decode(Date.self, from: JSONEncoder().encode({raw})))"
        ),
        FieldDefType::BooleanLiteral(_)
        | FieldDefType::Char
        | FieldDefType::Map(_, _)
        | FieldDefType::NumberLiteral(_)
        | FieldDefType::Range(_, _)
        | FieldDefType::Result(_, _)
        | FieldDefType::String
        | FieldDefType::StringLiteral(_)
        | FieldDefType::Tuple(_)
        | FieldDefType::TypeParam(_)
        | FieldDefType::Unknown => raw.to_owned(),
        #[cfg(feature = "mongodb")]
        FieldDefType::ObjectId => raw.to_owned(),
        #[cfg(feature = "chrono")]
        FieldDefType::NaiveDate | FieldDefType::NaiveTime | FieldDefType::NaiveDateTime => {
            raw.to_owned()
        }
    }
}
