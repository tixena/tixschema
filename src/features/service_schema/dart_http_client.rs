//! The Dart `http_rest` client: one service-agnostic transport seam, and one method per operation
//! that builds a plain-terms request from the operation's own message and decodes the answer by
//! status.
//!
//! # The seam is structural, not just service-agnostic
//!
//! The request and response the seam carries are Dart 3 records — `({String method, ...})` in,
//! `({int status, ...})` out — rather than named classes. A record is Dart's own structural type,
//! so every service's `send` reads the exact same anonymous shape; the only per-service name is
//! the abstract `{Service}HttpTransport` interface itself, kept apart only so two services in one
//! library do not both declare it. Nothing here names an HTTP package; the hand-written
//! implementation of this interface lives with the Flutter workspace.
//!
//! # A caller throws, rather than narrows a result
//!
//! The TypeScript half answers every reply with a `{ ok, value | error }` envelope, because a
//! union is how TypeScript tells success from failure. Dart's own idiom for a `Future` is a thrown
//! exception, so a reply operation here answers `Future<Success>` directly and throws a
//! `{Service}HttpError<Declared>` — the declared error, or the fault behind `isServiceFault` —
//! instead. A one-way operation answers `Future<void>` and throws the fault-only
//! `{Service}HttpRefusal`, exactly as its TypeScript sibling throws rather than returns one.
//!
//! # The fault is the same generated type every other surface answers faults through
//!
//! `{Service}FaultFields`/`{Service}FaultKind` already carry `#[model_schema()]` (declared in
//! [`crate::service_schema::support`]) and already publish their own Dart class and enum through
//! the ordinary [`crate::features::dart`] dispatch, with a working `fromJson`/`toJson` this module
//! never has to re-derive. Reusing them here is what keeps a fault's shape from drifting between
//! languages; nothing is invented beside them.
//!
//! # No outbound validation
//!
//! The TypeScript and Rust clients each parse a message against its own schema before a byte goes
//! out, because a JavaScript object or a hand-built `serde_json::Value` can be malformed even
//! though it is typed. A Dart message is a real class with `required` constructor parameters, so
//! the equivalent malformed value cannot be constructed in the first place — there is no separate
//! check to run.
//!
//! # `BodyKind` is matched exhaustively on purpose
//!
//! `BodyKind` carries `Json` and `Bytes` today; the streamed kind has no grammar yet
//! ([`crate::service_schema::parse`] — `BodyKind`'s own doc comment), so no operation can reach
//! this module carrying one. Every match on `BodyKind` here is exhaustive rather than defaulted, so
//! the moment a third variant lands, the compiler — not a silently wrong client — is what stops
//! this module until it is taught the new kind.

use crate::features::dart::dart_typename;
use crate::field_type::{FieldDefType, get_field_def};
use crate::rename_rule::RenameRule;
use crate::service_schema::parse::{
    BodyKind, DEFAULT_BINDING_ERROR_STATUS, HttpShape, OperationDef, OperationInputs,
    OperationOutcome, PathSegment, ServiceDef, is_unit_type, option_inner, tuple_elements,
    vec_inner, wire_key,
};
use crate::service_schema::support::fault_fields_typescript_name;
use core::fmt::Write as _;
use syn::Type;

pub fn emit(service: &ServiceDef) -> Vec<String> {
    let named = service.ident.to_string();
    let fn_prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let mut published = vec![transport_seam(&named), error_class(&named)];
    if has_one_way(service) {
        published.push(refusal_class(&named));
    }
    published.push(client_class(service));
    published.extend(fault_helpers(&named, &fn_prefix));
    published
}

fn has_one_way(service: &ServiceDef) -> bool {
    service
        .operations
        .iter()
        .any(|operation| matches!(operation.outcome, OperationOutcome::OneWay))
}

// ---------------------------------------------------------------------------------------------
// The seam: an abstract, per-service interface over one structural request/response record pair.
// ---------------------------------------------------------------------------------------------

fn transport_seam(named: &str) -> String {
    format!(
        "/// What binds a `{named}` Dart client to a real HTTP stack.\n\
         ///\n\
         /// The request and response are Dart records, not named classes: every service's own\n\
         /// `send` reads the exact same anonymous shape, so one hand-written implementation over\n\
         /// any HTTP stack satisfies every service's interface.\n\
         abstract class {named}HttpTransport {{\n  \
         Future<({{int status, List<(String, String)> headers, List<int> body}})> send(\n    \
         ({{String method, String path, String query, List<(String, String)> headers, List<int> body}}) request,\n  \
         );\n\
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// The two exceptions a client throws: the declared error or a fault, and (one-way only) a fault
// with nowhere else to be returned.
// ---------------------------------------------------------------------------------------------

fn error_class(named: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// What a `{named}` `http_rest` client throws for a request-and-reply operation: the\n\
         /// error the operation declared, or a fault it never declared.\n\
         class {named}HttpError<E> implements Exception {{\n  \
         {named}HttpError.declared(E declared)\n    \
         : error = declared,\n      \
         fault = null;\n  \
         {named}HttpError.fault({fields} reported)\n    \
         : error = null,\n      \
         fault = reported;\n  \
         final E? error;\n  \
         final {fields}? fault;\n  \
         bool get isServiceFault => fault != null;\n  \
         @override\n  \
         String toString() => isServiceFault\n      \
         ? '{named}HttpError: service fault ${{fault!.kind}} in `${{fault!.operation}}`: ${{fault!.detail}}'\n      \
         : '{named}HttpError: $error';\n\
         }}"
    )
}

fn refusal_class(named: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// What a one-way `{named}` `http_rest` method throws when it cannot answer its declared\n\
         /// status. A one-way operation declares no error, so there is nothing else to throw.\n\
         class {named}HttpRefusal implements Exception {{\n  \
         {named}HttpRefusal(this.fault);\n  \
         final {fields} fault;\n  \
         @override\n  \
         String toString() =>\n      \
         '{named}HttpRefusal: ${{fault.kind}} in `${{fault.operation}}`: ${{fault.detail}}';\n\
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// The client: one class, one constructor, one method per operation.
// ---------------------------------------------------------------------------------------------

fn client_class(service: &ServiceDef) -> String {
    let named = service.ident.to_string();
    let fn_prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let methods = service
        .operations
        .iter()
        .map(|operation| method(&named, &fn_prefix, operation))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "/// A `{named}` caller over `http_rest`.\n\
         class {named}HttpClient {{\n  \
         {named}HttpClient(this._transport);\n  \
         final {named}HttpTransport _transport;\n\n\
         {methods}\n\
         }}"
    )
}

/// The parameter list a method takes: the message first, then one argument per `header_in`
/// binding, in declaration order — the raw Rust identifier, spelled exactly as the rest of this
/// crate's Dart output spells a field, never re-cased.
fn method_params(operation: &OperationDef, shape: &HttpShape) -> String {
    let mut params = vec![format!("{} req", message_dart_typename(operation))];
    for header in &shape.header_in {
        params.push(format!("{} {}", dart_type_of(&header.ty), header.parameter));
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

fn return_type(operation: &OperationDef, shape: &HttpShape) -> String {
    match &operation.outcome {
        OperationOutcome::OneWay => "Future<void>".to_owned(),
        OperationOutcome::Reply { success, .. } => {
            if matches!(shape.body_kind, BodyKind::Bytes) {
                format!("Future<{}>", dart_type_of(success))
            } else if shape.header_out.is_empty() && is_unit_type(success) {
                "Future<void>".to_owned()
            } else {
                format!("Future<{}>", dart_type_of(success))
            }
        }
    }
}

fn method(named: &str, fn_prefix: &str, operation: &OperationDef) -> String {
    let shape = HttpShape::of(operation);
    let wire = &operation.wire_name;
    let call = &operation.ts_name;
    let params = method_params(operation, &shape);
    let returns = return_type(operation, &shape);
    let path_build = path_build_stmt(operation, &shape);
    let query_build = query_build_stmt(operation, &shape);
    let headers_build = header_in_build_stmt(&shape);
    let body_build = body_build_stmt(&shape);
    let method_str = shape.method.name();
    let (send, decode) = match &operation.outcome {
        OperationOutcome::OneWay => (
            send_stmt_one_way(named, fn_prefix, wire, method_str),
            one_way_decode_stmt(named, fn_prefix, &shape, wire),
        ),
        OperationOutcome::Reply { error, success } => (
            send_stmt_reply(named, fn_prefix, wire, method_str, &dart_type_of(error)),
            reply_decode_stmt(named, fn_prefix, &shape, wire, error, success),
        ),
    };
    format!(
        "{doc}\n  \
         {returns} {call}({params}) async {{\n\
{path_build}\
{query_build}\
{headers_build}\
{body_build}\
{send}\
{decode}\
  }}",
        doc = method_doc(operation, &shape),
    )
}

// ---------------------------------------------------------------------------------------------
// Building the request from the validated message.
// ---------------------------------------------------------------------------------------------

/// The value one path placeholder reads off `req`: one of its own fields under its own written
/// spelling for a generated or a multi-placeholder named message, or the whole message where a
/// named message answers to exactly one placeholder — mirrors the Rust and TypeScript clients'
/// own `client_placeholder_value`/`placeholder_value_expr`.
fn placeholder_value_dart_expr(
    operation: &OperationDef,
    shape: &HttpShape,
    placeholder: &str,
) -> String {
    match &operation.inputs {
        OperationInputs::Empty => format!("'${{req.{placeholder}}}'"),
        OperationInputs::Generated(fields) => fields
            .iter()
            .find(|(field, _)| field == placeholder)
            .map_or_else(
                || format!("'${{req.{placeholder}}}'"),
                |(_, ty)| dart_wire_text(ty, &format!("req.{placeholder}")),
            ),
        OperationInputs::Named(declared) => {
            if shape.placeholder_names().len() == 1 {
                dart_wire_text(declared, "req")
            } else {
                format!("'${{req.{placeholder}}}'")
            }
        }
    }
}

fn path_build_stmt(operation: &OperationDef, shape: &HttpShape) -> String {
    let mut stmt = String::from("    var path = '';\n");
    for segment in &shape.path {
        match segment {
            PathSegment::Literal(text) => {
                let _ = writeln!(stmt, "    path += '{}';", dart_escape(text));
            }
            PathSegment::Placeholder(name) => {
                let value = placeholder_value_dart_expr(operation, shape, name);
                let _ = writeln!(stmt, "    path += Uri.encodeComponent({value});");
            }
        }
    }
    stmt
}

/// The query string a bodyless method's own unbound fields build — only a `Generated` message
/// carries query fields, mirroring the Rust and TypeScript clients: every field a bodyless
/// `Named` message carries has to be exposed through a path placeholder instead.
fn query_build_stmt(operation: &OperationDef, shape: &HttpShape) -> String {
    let OperationInputs::Generated(fields) = &operation.inputs else {
        return "    const query = '';\n".to_owned();
    };
    if shape.method.carries_a_body() {
        return "    const query = '';\n".to_owned();
    }
    let placeholders = shape.placeholder_names();
    let mut pushes = String::new();
    for (field, ty) in fields {
        let field_name = field.to_string();
        if placeholders.contains(&field_name) {
            continue;
        }
        let key = wire_key(field);
        // A bodyless method's own field, unbound to a placeholder, is always `Option<...>` — a
        // required field with nowhere else to go is refused at parse time.
        let inner = option_inner(ty).unwrap_or(ty);
        let rendered = dart_wire_text(inner, "value");
        let _ = write!(
            pushes,
            "    {{\n      final value = req.{field_name};\n      if (value != null) {{\n        \
             queryParts.add('{key}=' + Uri.encodeComponent({rendered}));\n      \
             }}\n    }}\n"
        );
    }
    if pushes.is_empty() {
        return "    const query = '';\n".to_owned();
    }
    format!("    final queryParts = <String>[];\n{pushes}    final query = queryParts.join('&');\n")
}

/// Builds the outgoing header list, one entry per `header_in` binding — except a `null` optional
/// binding, which is added nowhere rather than as the empty string [`dart_wire_text`] renders for
/// it. A header the request never meant to carry is omitted, not sent empty. Mirrors the Rust and
/// TypeScript clients' own `header_in_build_stmts`/`header_in_build_stmt`.
fn header_in_build_stmt(shape: &HttpShape) -> String {
    if shape.header_in.is_empty() {
        return "    const headers = <(String, String)>[];\n".to_owned();
    }
    let mut stmt = String::from("    final headers = <(String, String)>[];\n");
    for header in &shape.header_in {
        let name = &header.name;
        let parameter = &header.parameter;
        if let Some(inner) = option_inner(&header.ty) {
            let text = dart_wire_text(inner, &format!("{parameter}!"));
            let _ = writeln!(
                stmt,
                "    if ({parameter} != null) {{\n      headers.add(('{name}', {text}));\n    }}"
            );
        } else {
            let text = dart_wire_text(&header.ty, &parameter.to_string());
            let _ = writeln!(stmt, "    headers.add(('{name}', {text}));");
        }
    }
    stmt
}

fn body_build_stmt(shape: &HttpShape) -> String {
    if shape.method.carries_a_body() {
        "    final body = utf8.encode(jsonEncode(req.toJson()));\n".to_owned()
    } else {
        "    const body = <int>[];\n".to_owned()
    }
}

// ---------------------------------------------------------------------------------------------
// Sending, and decoding the answer by status.
// ---------------------------------------------------------------------------------------------

fn send_expr(method_str: &str) -> String {
    format!(
        "await _transport.send((method: '{method_str}', path: path, query: query, headers: headers, body: body))"
    )
}

fn send_stmt_one_way(named: &str, fn_prefix: &str, wire: &str, method_str: &str) -> String {
    format!(
        "    late final ({{int status, List<(String, String)> headers, List<int> body}}) response;\n    \
         try {{\n      \
         response = {send};\n    \
         }} catch (uncarried) {{\n      \
         throw {named}HttpRefusal(_{fn_prefix}HttpTransportFailure('{wire}', '$uncarried'));\n    \
         }}\n",
        send = send_expr(method_str),
    )
}

fn send_stmt_reply(
    named: &str,
    fn_prefix: &str,
    wire: &str,
    method_str: &str,
    error_ty: &str,
) -> String {
    format!(
        "    late final ({{int status, List<(String, String)> headers, List<int> body}}) response;\n    \
         try {{\n      \
         response = {send};\n    \
         }} catch (uncarried) {{\n      \
         throw {named}HttpError<{error_ty}>.fault(_{fn_prefix}HttpTransportFailure('{wire}', '$uncarried'));\n    \
         }}\n",
        send = send_expr(method_str),
    )
}

fn one_way_decode_stmt(named: &str, fn_prefix: &str, shape: &HttpShape, wire: &str) -> String {
    let ok_status = shape.ok_status;
    format!(
        "    final status = response.status;\n    \
         if (status == {ok_status}) {{\n      return;\n    }}\n    \
         if (status == 400 || status == 404 || status == 500) {{\n      \
         throw {named}HttpRefusal(_{fn_prefix}HttpFaultFromBody('{wire}', response.body));\n    \
         }}\n    \
         throw {named}HttpRefusal(\n      \
         _{fn_prefix}HttpUndeserializablePayload('{wire}', 'an unexpected status ($status) answered'),\n    \
         );\n"
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
    named: &str,
    fn_prefix: &str,
    shape: &HttpShape,
    wire: &str,
    error: &Type,
    success: &Type,
) -> String {
    let ok_status = shape.ok_status;
    let error_condition = error_condition_expr(shape);
    let error_ty = dart_type_of(error);
    let success_block = success_decode_block(named, fn_prefix, wire, shape, &error_ty, success);
    format!(
        "    final status = response.status;\n    \
         if (status == {ok_status}) {{\n{success_block}    }}\n    \
         if ({error_condition}) {{\n      \
         late final {error_ty} declared;\n      \
         try {{\n        \
         declared = {error_ty}.fromJson(jsonDecode(utf8.decode(response.body)));\n      \
         }} catch (rejected) {{\n        \
         throw {named}HttpError<{error_ty}>.fault(\n          \
         _{fn_prefix}HttpUndeserializablePayload('{wire}', '$rejected'),\n        \
         );\n      \
         }}\n      \
         throw {named}HttpError<{error_ty}>.declared(declared);\n    \
         }}\n    \
         if (status == 400 || status == 404 || status == 500) {{\n      \
         throw {named}HttpError<{error_ty}>.fault(_{fn_prefix}HttpFaultFromBody('{wire}', response.body));\n    \
         }}\n    \
         throw {named}HttpError<{error_ty}>.fault(\n      \
         _{fn_prefix}HttpUndeserializablePayload('{wire}', 'an unexpected status ($status) answered'),\n    \
         );\n"
    )
}

/// What one operation's method returns once its status has already matched `ok_status`: the byte
/// list and content type for a `body = \"bytes\"` operation, nothing for a no-payload reply, the
/// decoded body alone, or the decoded body plus every `header_out` element read back off the
/// response's own headers.
fn success_decode_block(
    named: &str,
    fn_prefix: &str,
    wire: &str,
    shape: &HttpShape,
    error_ty: &str,
    success: &Type,
) -> String {
    if matches!(shape.body_kind, BodyKind::Bytes) {
        return "      final contentType = _findHeader(response.headers, 'content-type') ?? '';\n      \
                return (response.body, contentType);\n"
            .to_owned();
    }
    if shape.header_out.is_empty() {
        if is_unit_type(success) {
            return "      return;\n".to_owned();
        }
        let success_ty = dart_type_of(success);
        return format!(
            "      late final {success_ty} value;\n      \
             try {{\n        \
             value = {success_ty}.fromJson(jsonDecode(utf8.decode(response.body)));\n      \
             }} catch (rejected) {{\n        \
             throw {named}HttpError<{error_ty}>.fault(\n          \
             _{fn_prefix}HttpUndeserializablePayload('{wire}', '$rejected'),\n        \
             );\n      \
             }}\n      \
             return value;\n"
        );
    }
    // `header_out`'s own arity check guarantees `success` is a tuple of exactly this many
    // elements, so the lookups below never miss.
    let elements: Vec<&Type> = tuple_elements(success).into_iter().flatten().collect();
    let body_ty = elements
        .first()
        .map_or_else(|| dart_type_of(success), |ty| dart_type_of(ty));
    let mut stmt = format!(
        "      late final {body_ty} value;\n      \
         try {{\n        \
         value = {body_ty}.fromJson(jsonDecode(utf8.decode(response.body)));\n      \
         }} catch (rejected) {{\n        \
         throw {named}HttpError<{error_ty}>.fault(\n          \
         _{fn_prefix}HttpUndeserializablePayload('{wire}', '$rejected'),\n        \
         );\n      \
         }}\n"
    );
    let mut header_idents = Vec::new();
    for (index, (name, element_ty)) in shape
        .header_out
        .iter()
        .zip(elements.iter().skip(1))
        .enumerate()
    {
        let raw_ident = format!("rawHeaderOut{index}");
        let ident = format!("headerOut{index}");
        let decode = dart_header_out_decode(element_ty, &raw_ident);
        let _ = write!(
            stmt,
            "      final {raw_ident} = _findHeader(response.headers, '{name}');\n      \
             if ({raw_ident} == null) {{\n        \
             throw {named}HttpError<{error_ty}>.fault(\n          \
             _{fn_prefix}HttpUndeserializablePayload('{wire}', 'a declared response header was missing'),\n        \
             );\n      \
             }}\n      \
             final {ident} = {decode};\n"
        );
        header_idents.push(ident);
    }
    let _ = writeln!(stmt, "      return (value, {});", header_idents.join(", "));
    stmt
}

// ---------------------------------------------------------------------------------------------
// The fault helpers every method reaches for.
// ---------------------------------------------------------------------------------------------

fn fault_helpers(named: &str, fn_prefix: &str) -> Vec<String> {
    vec![
        find_header_fn(),
        transport_failure_fn(named, fn_prefix),
        undeserializable_payload_fn(named, fn_prefix),
        fault_from_body_fn(named, fn_prefix),
    ]
}

/// Reads one response header back case-insensitively, the way HTTP headers are read — Dart's
/// core `Iterable` carries no `firstWhereOrNull` of its own.
fn find_header_fn() -> String {
    "/// Reads one response header back case-insensitively, the way HTTP headers are read.\n\
     String? _findHeader(List<(String, String)> headers, String name) {\n  \
     for (final header in headers) {\n    \
     if (header.$1.toLowerCase() == name) return header.$2;\n  \
     }\n  \
     return null;\n\
     }"
    .to_owned()
}

fn transport_failure_fn(named: &str, fn_prefix: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// The fault a `{named}` HTTP client answers with when the transport could not carry a\n\
         /// call: the request never went out, or the response never came back.\n\
         {fields} _{fn_prefix}HttpTransportFailure(String operation, String detail) =>\n    \
         {fields}(\n      \
         detail: detail,\n      \
         kind: {named}FaultKind.transportFailure,\n      \
         operation: operation,\n    \
         );"
    )
}

fn undeserializable_payload_fn(named: &str, fn_prefix: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// The fault a `{named}` HTTP client answers with when a response will not become the\n\
         /// answer its status promised: a body that will not parse, a declared header that never\n\
         /// arrived, or a status this client did not expect.\n\
         {fields} _{fn_prefix}HttpUndeserializablePayload(String operation, String detail) =>\n    \
         {fields}(\n      \
         detail: detail,\n      \
         kind: {named}FaultKind.undeserializablePayload,\n      \
         operation: operation,\n    \
         );"
    )
}

fn fault_from_body_fn(named: &str, fn_prefix: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "/// Reads a fixed fault status's own body back into a `{fields}`, through the same\n\
         /// `fromJson` every other surface reads it with. A body that does not read as the fixed\n\
         /// fault shape is itself a defect, answered as one.\n\
         {fields} _{fn_prefix}HttpFaultFromBody(String operation, List<int> body) {{\n  \
         try {{\n    \
         return {fields}.fromJson(jsonDecode(utf8.decode(body)));\n  \
         }} catch (_) {{\n    \
         return _{fn_prefix}HttpUndeserializablePayload(\n      \
         operation,\n      \
         'a fault body did not match the expected shape',\n    \
         );\n  \
         }}\n\
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// Small, Dart-flavored value rendering — kept apart from `features::dart` itself, which carries no
// HTTP-shaped knowledge at all; every other surface's own rendering stays exactly as it was.
// ---------------------------------------------------------------------------------------------

/// The message's Dart type: the type the operation named, or the one the macro declared for an
/// operation that named none — mirrors `message::typename` (the TypeScript half), through the
/// same `FieldDef` walk every reference to a type goes through.
fn message_dart_typename(operation: &OperationDef) -> String {
    match &operation.inputs {
        OperationInputs::Named(declared) => dart_type_of(declared),
        OperationInputs::Empty | OperationInputs::Generated(_) => {
            operation.generated_message_ident().map_or_else(
                || "dynamic".to_owned(),
                |ident| {
                    let named: Type = syn::parse_quote! { #ident };
                    dart_type_of(&named)
                },
            )
        }
    }
}

/// `ty`'s own Dart type name, read through the same `FieldDef` walk every field's type goes
/// through — so a reference to a sibling `#[model_schema()]` type resolves to its published Dart
/// name exactly as it would inside an ordinary field.
fn dart_type_of(ty: &Type) -> String {
    dart_typename(&get_field_def("value", ty, ""))
}

/// Whether `ty` is a reference to another `#[model_schema()]` item (an author's own type, or one
/// this crate generated) rather than a primitive this crate itself renders — the one distinction
/// [`dart_wire_text`] needs to decide whether a value must be read through its own `toJson()`
/// before it can be embedded as text.
fn is_sibling_type(ty: &Type) -> bool {
    matches!(
        get_field_def("value", ty, "").field_type,
        FieldDefType::SiblingType(_, _)
    )
}

/// The Dart expression that renders `ty`'s value at `expr` as URL- or header-safe text: an
/// `Option<T>` reads as the empty string when absent, a `Vec<T>` joins its elements' own text with
/// a comma, and a sibling type is read through its own `toJson()` first — Dart's string
/// interpolation would otherwise call `Object`'s default `toString()` on the class instance rather
/// than on the value it wraps. A `String`, a `bool` and a number all interpolate correctly as
/// themselves, which is what lets everything else fall through to plain interpolation.
fn dart_wire_text(ty: &Type, expr: &str) -> String {
    if let Some(inner) = option_inner(ty) {
        let non_null = format!("{expr}!");
        let rendered = dart_wire_text(inner, &non_null);
        return format!("({expr} == null ? '' : {rendered})");
    }
    if let Some(inner) = vec_inner(ty) {
        let element = dart_wire_text(inner, "e");
        return format!("({expr}).map((e) => {element}).join(\",\")");
    }
    if is_sibling_type(ty) {
        return format!("'${{({expr}).toJson()}}'");
    }
    format!("'${{{expr}}}'")
}

/// The expression that reads one `header_out` element's declared type back off `raw` — a
/// `String?` expression already checked non-null — mirroring the coercion the Rust and TypeScript
/// clients perform on the way back from a response header.
fn dart_header_out_decode(ty: &Type, raw: &str) -> String {
    let base = option_inner(ty).unwrap_or(ty);
    if let Some(inner) = vec_inner(base) {
        let element = dart_header_out_decode(inner, "piece");
        return format!("({raw}).split(\",\").map((piece) => {element}).toList()");
    }
    match get_field_def("value", base, "").field_type {
        FieldDefType::Boolean => format!("({raw} == 'true')"),
        FieldDefType::U8
        | FieldDefType::U16
        | FieldDefType::U32
        | FieldDefType::U64
        | FieldDefType::I8
        | FieldDefType::I16
        | FieldDefType::I32
        | FieldDefType::I64
        | FieldDefType::Usize
        | FieldDefType::Isize => format!("int.parse({raw})"),
        FieldDefType::F32 | FieldDefType::F64 => format!("double.parse({raw})"),
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

/// Escapes a path literal for a single-quoted Dart string: a backslash, a single quote and a
/// dollar sign (which Dart reads as interpolation even inside a single-quoted literal) all need
/// escaping; a path template is ASCII by construction, so nothing else does.
fn dart_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(ch, '\\' | '\'' | '$') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}
