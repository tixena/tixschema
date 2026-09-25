//! The TypeScript `http_rest` server: a route table, plain-terms request and response shapes, a
//! fault handler with the Rust transport's own defaults, and one dispatcher that matches, assembles
//! the message the way the Rust `dispatch` does, drives `create{Service}Dispatcher`, and maps the
//! outcome to a status and a body.
//!
//! # Names no web framework
//!
//! The dispatcher takes a whole request and answers a whole response; nothing here names the
//! library that finally carries the call. Binding it to a real listener is the hosting
//! application's own adapter, exactly as [`super::http_client`]'s own transport seam names none
//! either.
//!
//! # The route table is data, not a second entry point
//!
//! The dispatcher does its own path matching, in declaration order, over the same template the
//! table publishes. An application that wants one framework handler per route reads the table; one
//! that wants a single catch-all passes every request straight to the dispatcher. Either way the
//! dispatcher matches again — there is no shortcut that skips it.
//!
//! # A declared error's status, read off the value
//!
//! An operation's `error_status` table maps a Rust variant name to a status, and the value on the
//! wire carries no such name of its own except in the position each enum's own serde form puts it.
//! [`error_status_closure`] reaches for the `{Enum}$Variant` reader that enum's own
//! `#[model_schema]` expansion publishes — never a switch this module writes over the wire shape
//! itself — and is called at all only where the table names more than one distinct status; one
//! status (or none) needs no reader.
//!
//! # A bound header or multipart part reaches the dispatcher, not this module
//!
//! `create{Service}Dispatcher` — [`super::service`]'s own `dispatcher` — is where a `header_in`
//! binding and a `part(...)` binding are read, decoded and refused, the same way it already reads
//! and refuses the message. This module keeps no presence check of its own. What it does keep is
//! the HTTP text form: the dispatcher reads and writes each header value JSON-encoded, as a
//! `ws_rpc` frame and an AMQP message carry it, so each `header_in` value is coerced from its
//! header text before it is handed on, and each header the dispatcher answers is rendered back to
//! header text before it is written.

use super::message;
use super::result::STREAMED_ANSWER_TS_TYPE;
use crate::field_type::get_field_def;
use crate::rename_rule::RenameRule;
use crate::service_schema::parse::{
    BodyKind, DEFAULT_BINDING_ERROR_STATUS, HttpShape, OperationDef, OperationInputs,
    OperationOutcome, PathSegment, ServiceDef, error_declared_type, is_scalar_named_type,
    option_inner, service_declares_a_stream, service_declares_multipart, tuple_elements,
    type_leaf_name, vec_inner, wire_key,
};
use crate::service_schema::support::fault_fields_typescript_name;
use crate::utils::is_recorded_untagged_enum;
use core::fmt::Write as _;
use syn::Type;

/// The three facts every dispatcher-building function below reads off the service, bundled so
/// none of them carries more of its own parameters than a reader can hold at once: the published
/// name, the camelCase prefix its own helpers are named under, and whether it declares a
/// multipart operation at all — which decides whether `request.parts`/`parts` is threaded through
/// beside `request.headers`/`headers`.
struct DispatcherContext<'ctx> {
    has_multipart: bool,
    named: &'ctx str,
    prefix: &'ctx str,
}

pub fn emit(service: &ServiceDef) -> Vec<String> {
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let has_stream = service_declares_a_stream(service);
    let has_multipart = service_declares_multipart(service);
    let mut items = vec![
        route_type(&named),
        route_table(service, &named, &prefix),
        request_type(&named, has_multipart),
        response_type(&named, has_stream),
        fault_handler_type(&named),
        json_fn(&named, &prefix),
        default_fault_handler_fn(&named, &prefix),
        fault_fn(&named, &prefix),
        match_path_fn(&prefix),
        parse_query_fn(&prefix),
        coerce_number_fn(&prefix),
    ];
    if service_writes_a_response_header(service) {
        items.push(legal_response_header_value_fn(&prefix));
    }
    if service.operations.iter().any(|operation| {
        let shape = HttpShape::of(operation);
        !shape.header_out.is_empty() || !shape.error_header_out.is_empty()
    }) {
        items.push(replied_header_fn(&prefix));
    }
    items.push(dispatcher_fn(service, &named, &prefix));
    items
}

/// Whether any operation ever pushes a runtime-computed value onto the response's headers.
/// Gates [`legal_response_header_value_fn`]'s own emission, to avoid dead code in a bundle.
fn service_writes_a_response_header(service: &ServiceDef) -> bool {
    service.operations.iter().any(|operation| {
        let shape = HttpShape::of(operation);
        !shape.header_out.is_empty()
            || !shape.error_header_out.is_empty()
            || matches!(shape.body_kind, BodyKind::Bytes | BodyKind::Stream)
    })
}

/// Whether every character of `value` is legal as an HTTP header value. Named apart from the
/// client's own equivalent so a bundle carrying both declares no name twice.
fn legal_response_header_value_fn(prefix: &str) -> String {
    format!(
        "/** Whether every character of `value` is legal as an HTTP header value: visible ASCII\n \
         * (`0x21`-`0x7E`), a space, or a tab. */\n\
         function {prefix}HttpLegalResponseHeaderValue(value: string): boolean {{\n  \
         for (let index = 0; index < value.length; index += 1) {{\n    \
         const code = value.charCodeAt(index);\n    \
         if (code !== 0x09 && code !== 0x20 && (code < 0x21 || code > 0x7e)) return false;\n  \
         }}\n  \
         return true;\n\
         }}"
    )
}

/// Reads one header the dispatcher answered back off its JSON encoding, `undefined` where it wrote
/// none — an optional header holding `undefined`.
fn replied_header_fn(prefix: &str) -> String {
    format!(
        "/** One header the dispatcher answered, decoded off its JSON encoding. */\n\
         function {prefix}HttpRepliedHeader(\n  \
         replied: ReadonlyArray<readonly [string, string]>,\n  \
         name: string,\n\
         ): unknown {{\n  \
         const carried = replied.find(([candidate]) => candidate === name)?.[1];\n  \
         return carried === undefined ? undefined : JSON.parse(carried);\n\
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// The route table
// ---------------------------------------------------------------------------------------------

fn route_type(named: &str) -> String {
    format!(
        "/** One operation's method, path template and status table, for an adapter that \
         registers a handler per route. */\n\
         export type {named}HttpRoute = {{\n  \
         errorStatuses: ReadonlyArray<number>;\n  \
         method: string;\n  \
         okStatus: number;\n  \
         operation: string;\n  \
         path: string;\n\
         }};"
    )
}

/// Every distinct status a declared error can answer with, in the order its variant was first
/// mapped — empty for a one-way operation or one declaring no `error_status` table.
fn distinct_error_statuses(shape: &HttpShape) -> Vec<u16> {
    let mut seen = Vec::new();
    for (_, code) in &shape.error_status {
        if !seen.contains(code) {
            seen.push(*code);
        }
    }
    seen
}

fn route_row(operation: &OperationDef) -> String {
    let shape = HttpShape::of(operation);
    let codes = distinct_error_statuses(&shape)
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "  {{ errorStatuses: [{codes}], method: \"{}\", okStatus: {}, operation: \"{}\", path: \
         \"{}\" }},",
        shape.method.name(),
        shape.ok_status,
        operation.wire_name,
        shape.path_template(),
    )
}

fn route_table(service: &ServiceDef, named: &str, prefix: &str) -> String {
    let rows = service
        .operations
        .iter()
        .map(route_row)
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "/** The declared route table, in declaration order. `{{field}}` placeholders are \
         written as declared. */\n\
         export const {prefix}HttpRoutes: ReadonlyArray<{named}HttpRoute> = [\n\
         {rows}\n\
         ];"
    )
}

// ---------------------------------------------------------------------------------------------
// The request, response and fault-handler types
// ---------------------------------------------------------------------------------------------

fn request_type(named: &str, has_multipart: bool) -> String {
    let parts_field = if has_multipart {
        "\n  parts: ReadonlyArray<readonly [string, unknown]>;"
    } else {
        ""
    };
    format!(
        "/** One HTTP request in plain terms. The body is undecoded bytes, as the Rust \
         `IncomingRequest` carries it. */\n\
         export type {named}HttpRequest = {{\n  \
         body: Uint8Array;\n  \
         headers: ReadonlyArray<readonly [string, string]>;\n  \
         method: string;{parts_field}\n  \
         path: string;\n  \
         query: string;\n\
         }};"
    )
}

fn response_type(named: &str, has_stream: bool) -> String {
    let body_ty = if has_stream {
        "Uint8Array | ReadableStream<Uint8Array>"
    } else {
        "Uint8Array"
    };
    format!(
        "/** One HTTP response in plain terms, for an adapter to write back however its framework \
         answers. */\n\
         export type {named}HttpResponse = {{\n  \
         body: {body_ty};\n  \
         headers: ReadonlyArray<readonly [string, string]>;\n  \
         status: number;\n\
         }};"
    )
}

fn fault_handler_type(named: &str) -> String {
    format!(
        "/** Decides what one fault answers with. The default: 404 unknown-operation, 500 \
         handler-panic, 400 otherwise, the fault as JSON. */\n\
         export type {named}HttpFaultHandler = (fault: {named}Fault) => {named}HttpResponse;"
    )
}

fn json_fn(named: &str, prefix: &str) -> String {
    format!(
        "function {prefix}HttpJson(status: number, headers: ReadonlyArray<readonly [string, \
         string]>, value: unknown): {named}HttpResponse {{\n  \
         return {{ status, headers: [...headers, [\"content-type\", \"application/json\"]], \
         body: new TextEncoder().encode(JSON.stringify(value)) }};\n\
         }}"
    )
}

fn default_fault_handler_fn(named: &str, prefix: &str) -> String {
    format!(
        "export function {prefix}HttpDefaultFaultHandler(fault: {named}Fault): \
         {named}HttpResponse {{\n  \
         const status = fault.kind === \"unknown-operation\" ? 404 : fault.kind === \
         \"handler-panic\" ? 500 : 400;\n  \
         return {prefix}HttpJson(status, [], fault);\n\
         }}"
    )
}

fn fault_fn(named: &str, prefix: &str) -> String {
    let fields = fault_fields_typescript_name(named);
    format!(
        "function {prefix}HttpFault(kind: {fields}[\"kind\"], operation: string, detail: \
         string, field?: string): {named}Fault {{\n  \
         const built: {fields} = {{ detail, field, kind, operation }};\n  \
         return built as {named}Fault;\n\
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// The three helpers: path matching, query parsing, numeric coercion
// ---------------------------------------------------------------------------------------------

fn match_path_fn(prefix: &str) -> String {
    format!(
        "/** Mirrors the Rust `match_path`: a literal token is stripped as a prefix, a \
         placeholder takes one non-empty segment, and nothing may remain. */\n\
         function {prefix}HttpMatchPath(template: ReadonlyArray<string | null>, path: string): \
         Array<string> | undefined {{\n  \
         let rest = path;\n  \
         const captured: Array<string> = [];\n  \
         for (const token of template) {{\n    \
         if (token !== null) {{\n      \
         if (!rest.startsWith(token)) return undefined;\n      \
         rest = rest.slice(token.length);\n    \
         }} else {{\n      \
         const end = rest.indexOf(\"/\") === -1 ? rest.length : rest.indexOf(\"/\");\n      \
         const value = rest.slice(0, end);\n      \
         if (value === \"\") return undefined;\n      \
         captured.push(value);\n      \
         rest = rest.slice(end);\n    \
         }}\n  \
         }}\n  \
         return rest === \"\" ? captured : undefined;\n\
         }}"
    )
}

fn parse_query_fn(prefix: &str) -> String {
    format!(
        "/** Mirrors the Rust `parse_query`: split on `&`, split each pair once on `=`, \
         percent-decode both halves. */\n\
         function {prefix}HttpParseQuery(raw: string): Map<string, string> {{\n  \
         const parsed = new Map<string, string>();\n  \
         for (const pair of raw.split(\"&\")) {{\n    \
         if (pair === \"\") continue;\n    \
         const at = pair.indexOf(\"=\");\n    \
         const [key, value] = at === -1 ? [pair, \"\"] : [pair.slice(0, at), pair.slice(at + \
         1)];\n    \
         parsed.set(decodeURIComponent(key), decodeURIComponent(value));\n  \
         }}\n  \
         return parsed;\n\
         }}"
    )
}

fn coerce_number_fn(prefix: &str) -> String {
    format!(
        "/** The Rust `decode_expr` coercion for a numeric argument: an integer, else a float, \
         else the text itself. */\n\
         function {prefix}HttpCoerceNumber(raw: string): unknown {{\n  \
         if (/^-?\\d+$/.test(raw)) return Number(raw);\n  \
         const asFloat = Number(raw);\n  \
         return raw.trim() !== \"\" && Number.isFinite(asFloat) ? asFloat : raw;\n\
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// A declared error's status
// ---------------------------------------------------------------------------------------------

/// Turns a declared error into a status: the fixed default with no table, a constant where every
/// mapped variant shares one status, or a switch over the error enum's own `{Enum}$Variant`.
fn error_status_closure(shape: &HttpShape, error_type: &Type) -> String {
    if shape.error_status.is_empty() {
        return format!("() => {DEFAULT_BINDING_ERROR_STATUS}");
    }
    // The parser already refuses a multi-status table on an untagged enum, so what remains
    // maps every variant to one status.
    if is_recorded_untagged_enum(error_type) {
        let single = distinct_error_statuses(shape)
            .first()
            .copied()
            .unwrap_or(DEFAULT_BINDING_ERROR_STATUS);
        return format!("() => {single}");
    }
    let enum_name = type_leaf_name(error_type);
    let arms = shape
        .error_status
        .iter()
        .map(|(variant, code)| format!("            case \"{variant}\": return {code};"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "(error) => {{\n          \
         switch ({enum_name}$Variant(error)) {{\n\
{arms}\n            \
         default: return {DEFAULT_BINDING_ERROR_STATUS};\n          \
         }}\n        \
         }}"
    )
}

// ---------------------------------------------------------------------------------------------
// Message assembly (H8): named, generated, or nothing declared to build from
// ---------------------------------------------------------------------------------------------

/// The statements (if any) and the final expression that build one operation's message, mirroring
/// the Rust dispatcher's own `message_value`.
fn message_build(operation: &OperationDef, shape: &HttpShape, prefix: &str) -> (String, String) {
    let bodied = shape.method.carries_a_body();
    let multipart = matches!(shape.body_kind, BodyKind::Multipart);
    match &operation.inputs {
        OperationInputs::Empty => empty_message_build(operation, bodied, multipart, prefix),
        OperationInputs::Named(named_type) => {
            named_message_build(operation, named_type, &shape.placeholder_names(), prefix)
        }
        OperationInputs::Generated(fields) => generated_message_build(fields, shape, prefix),
    }
}

/// The statements that parse `request.body` as JSON and answer a fault where it will not parse at
/// all — the message *is* the parsed value, mirroring the Rust `from_body_expr`.
fn whole_body_message_stmt(wire: &str, prefix: &str) -> String {
    format!(
        "        let message: unknown;\n        \
         try {{\n          \
         message = JSON.parse(new TextDecoder().decode(request.body));\n        \
         }} catch (rejected) {{\n          \
         return onFault({prefix}HttpFault(\"undeserializable-payload\", \"{wire}\", \
         String(rejected)));\n        \
         }}\n"
    )
}

/// The statements that read `request.body` as JSON and answer with an empty base object rather
/// than a fault where it will not parse or is not itself an object — mirroring the Rust
/// `object_base(true)`, which the caller then inserts fields onto.
fn parsed_body_base_stmt() -> String {
    "        let message: Record<string, unknown>;\n        \
     try {\n          \
     const parsedBody: unknown = JSON.parse(new TextDecoder().decode(request.body));\n          \
     message =\n            \
     typeof parsedBody === \"object\" && parsedBody !== null && !Array.isArray(parsedBody)\n              \
     ? (parsedBody as Record<string, unknown>)\n              \
     : {};\n        \
     } catch {\n          \
     message = {};\n        \
     }\n"
        .to_owned()
}

fn empty_message_build(
    operation: &OperationDef,
    bodied: bool,
    multipart: bool,
    prefix: &str,
) -> (String, String) {
    if bodied && !multipart {
        return (
            whole_body_message_stmt(&operation.wire_name, prefix),
            "message".to_owned(),
        );
    }
    (String::new(), "{}".to_owned())
}

/// A `Named` message, TypeScript side: the whole body, the one placeholder's decoded value when
/// the type is a scalar bound whole, or an object keyed by placeholder.
fn named_message_build(
    operation: &OperationDef,
    named_type: &Type,
    placeholder_names: &[String],
    prefix: &str,
) -> (String, String) {
    if placeholder_names.is_empty() {
        return (
            whole_body_message_stmt(&operation.wire_name, prefix),
            "message".to_owned(),
        );
    }
    if placeholder_names.len() == 1 && is_scalar_named_type(named_type) {
        return (
            String::new(),
            message::decode_ts_expr(named_type, &placeholder_names[0], prefix),
        );
    }
    let mut setup = parsed_body_base_stmt();
    for name in placeholder_names {
        let _ = writeln!(setup, "        message[\"{name}\"] = {name};");
    }
    (setup, "message".to_owned())
}

fn query_field_insert(key: &str, ty: &Type, prefix: &str) -> String {
    let decode = message::decode_ts_expr(ty, "raw", prefix);
    format!(
        "        {{\n          \
         const raw = queryMap.get(\"{key}\");\n          \
         message[\"{key}\"] = raw === undefined ? null : {decode};\n        \
         }}\n"
    )
}

fn multipart_field_insert(key: &str, ty: &Type, prefix: &str) -> String {
    let decode = message::decode_ts_expr(ty, "raw", prefix);
    format!(
        "        {{\n          \
         const found = request.parts.find(([name]) => name === \"{key}\");\n          \
         const raw = found !== undefined && typeof found[1] === \"string\" ? found[1] : \
         undefined;\n          \
         message[\"{key}\"] = raw === undefined ? null : {decode};\n        \
         }}\n"
    )
}

fn generated_message_build(
    fields: &[(syn::Ident, Type)],
    shape: &HttpShape,
    prefix: &str,
) -> (String, String) {
    let bodied = shape.method.carries_a_body();
    let multipart = matches!(shape.body_kind, BodyKind::Multipart);
    let placeholder_names = shape.placeholder_names();
    let reads_query = !bodied
        && !multipart
        && fields
            .iter()
            .any(|(field, _)| !placeholder_names.contains(&field.to_string()));
    let mut setup = if multipart {
        "        const message: Record<string, unknown> = {};\n".to_owned()
    } else if bodied {
        parsed_body_base_stmt()
    } else if reads_query {
        format!(
            "        const queryMap = {prefix}HttpParseQuery(request.query);\n        \
             const message: Record<string, unknown> = {{}};\n"
        )
    } else {
        "        const message: Record<string, unknown> = {};\n".to_owned()
    };
    for (field, ty) in fields {
        let field_name = field.to_string();
        let is_placeholder = placeholder_names.contains(&field_name);
        if !is_placeholder && !multipart && bodied {
            // Neither placeholder- nor part-bound, and the method carries a body: the field is
            // already whatever the parsed JSON base holds under this key.
            continue;
        }
        let key = wire_key(field);
        if is_placeholder {
            let decode = message::decode_ts_expr(ty, &field_name, prefix);
            let _ = writeln!(setup, "        message[\"{key}\"] = {decode};");
        } else if multipart {
            setup.push_str(&multipart_field_insert(&key, ty, prefix));
        } else {
            setup.push_str(&query_field_insert(&key, ty, prefix));
        }
    }
    (setup, "message".to_owned())
}

// ---------------------------------------------------------------------------------------------
// The dispatcher
// ---------------------------------------------------------------------------------------------

fn dispatcher_fn(service: &ServiceDef, named: &str, prefix: &str) -> String {
    let ctx = DispatcherContext {
        named,
        prefix,
        has_multipart: service_declares_multipart(service),
    };
    let answer = answer_fn(&ctx);
    let arms = service
        .operations
        .iter()
        .map(|operation| arm(operation, &ctx))
        .collect::<String>();
    format!(
        "/**\n \
         * Turns an implementation into the function an adapter drives it with: one request in, \
         one\n \
         * response out. Matches the route table in declaration order, assembles the \
         operation's message\n \
         * exactly as the Rust dispatcher does, parses it through the generated dispatcher \
         (which checks\n \
         * it against its schema), and maps the outcome to a status and a body.\n \
         */\n\
         export function create{named}HttpDispatcher<Ctx>(\n  \
         impl: {named}Impl<Ctx>,\n  \
         onFault: {named}HttpFaultHandler = {prefix}HttpDefaultFaultHandler,\n\
         ): (ctx: Ctx, request: {named}HttpRequest) => Promise<{named}HttpResponse> {{\n  \
         const dispatch = create{named}Dispatcher(impl);\n\
         {answer}\n  \
         return async (ctx, request) => {{\n    \
         const {{ method, path }} = request;\n\
{arms}    \
         return onFault({prefix}HttpFault(\"unknown-operation\", `${{method}} ${{path}}`, \"the \
         service answers to no route by that method and path\"));\n  \
         }};\n\
         }}"
    )
}

/// The shared closure every JSON-reply, multipart-reply and one-way arm answers through. Bytes,
/// stream and `header_out` replies build their own response instead — see [`custom_reply_block`].
/// Takes the arm's JSON-encoded `header_in` values and, where the service declares multipart,
/// the request's own `parts`, handing both to the dispatcher unchecked.
fn answer_fn(ctx: &DispatcherContext) -> String {
    let DispatcherContext {
        named,
        prefix,
        has_multipart,
    } = *ctx;
    let (parts_param, parts_arg) = if has_multipart {
        (
            " parts: ReadonlyArray<readonly [string, unknown]>,",
            ", parts",
        )
    } else {
        ("", "")
    };
    format!(
        "  const answer = async (ctx: Ctx, operation: string, payload: unknown, headers: \
         ReadonlyArray<readonly [string, string]>,{parts_param} okStatus: number, errorStatus: \
         (error: unknown) => number) => {{\n    \
         let dispatched: {named}Dispatched | undefined;\n    \
         try {{\n      \
         dispatched = await dispatch(ctx, operation, payload, headers{parts_arg});\n    \
         }} catch (thrown) {{\n      \
         return onFault({prefix}HttpFault(\"handler-panic\", operation, thrown instanceof \
         Error ? thrown.message : String(thrown)));\n    \
         }}\n    \
         if (dispatched === undefined) return {{ status: okStatus, headers: [], body: new \
         Uint8Array() }};\n    \
         const envelope = dispatched.answered as {{ ok: true; value: unknown }} | {{ ok: false; \
         error: unknown }};\n    \
         if (envelope.ok) return {prefix}HttpJson(okStatus, [], envelope.value);\n    \
         const error = envelope.error as {{ isServiceFault?: true; fault?: {named}Fault }};\n    \
         if (typeof error === \"object\" && error !== null && error.isServiceFault === true \
         && error.fault !== undefined) return onFault(error.fault);\n    \
         return {prefix}HttpJson(errorStatus(envelope.error), [], envelope.error);\n  \
         }};"
    )
}

fn path_token_list(path: &[PathSegment]) -> String {
    path.iter()
        .map(|segment| match segment {
            PathSegment::Literal(text) => format!("\"{text}\""),
            PathSegment::Placeholder(_) => "null".to_owned(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// One `if (method === "...") { ... }` arm: comment, path match, placeholder destructure, the
/// assembled message, and the answer. A bound `header_in` or `part(...)` binding is read and
/// refused by the dispatcher itself, not here.
fn arm(operation: &OperationDef, ctx: &DispatcherContext) -> String {
    let prefix = ctx.prefix;
    let shape = HttpShape::of(operation);
    let mut out = arm_comment(operation, &shape);
    let _ = writeln!(out, "    if (method === \"{}\") {{", shape.method.name());
    let _ = writeln!(
        out,
        "      const captured = {prefix}HttpMatchPath([{}], path);",
        path_token_list(&shape.path)
    );
    out.push_str("      if (captured !== undefined) {\n");
    let placeholder_names = shape.placeholder_names();
    if !placeholder_names.is_empty() {
        let _ = writeln!(
            out,
            "        const [{}] = captured;",
            placeholder_names.join(", ")
        );
    }
    let (setup, message_expr) = message_build(operation, &shape, prefix);
    out.push_str(&setup);
    out.push_str(&header_in_encode_stmt(&shape, prefix));
    out.push_str(&arm_body(operation, &shape, &message_expr, ctx));
    out.push_str("      }\n");
    out.push_str("    }\n");
    out
}

/// Each `header_in` value's own header text, coerced the way the Rust `decode_expr` coerces it
/// and JSON-encoded into `headersIn` for the dispatcher to read. A header the request did not
/// carry is left out, for the dispatcher to accept as `undefined` or refuse as missing. Nothing at
/// all for an operation binding none, which hands the dispatcher an empty list.
fn header_in_encode_stmt(shape: &HttpShape, prefix: &str) -> String {
    if shape.header_in.is_empty() {
        return String::new();
    }
    let mut stmt = String::from("        const headersIn: Array<[string, string]> = [];\n");
    for header in &shape.header_in {
        let name = &header.name;
        let text = format!(
            "{}Text",
            RenameRule::CamelCase.apply_to_field(&header.parameter.to_string())
        );
        let lower = name.to_lowercase();
        let decode = message::decode_ts_expr(&header.ty, &text, prefix);
        let _ = write!(
            stmt,
            "        const {text} = request.headers.find(([name]) => name.toLowerCase() === \
             \"{lower}\")?.[1];\n        \
             if ({text} !== undefined) headersIn.push([\"{name}\", JSON.stringify({decode})]);\n"
        );
    }
    stmt
}

/// What an arm hands the dispatcher beside the message: its JSON-encoded `header_in` values and,
/// where the service declares multipart, the request's own parts.
fn dispatch_extras(shape: &HttpShape, has_multipart: bool) -> String {
    let headers = if shape.header_in.is_empty() {
        "[]"
    } else {
        "headersIn"
    };
    let parts = if has_multipart { ", request.parts" } else { "" };
    format!("{headers}{parts}")
}

fn arm_body(
    operation: &OperationDef,
    shape: &HttpShape,
    message_expr: &str,
    ctx: &DispatcherContext,
) -> String {
    let extras = dispatch_extras(shape, ctx.has_multipart);
    let OperationOutcome::Reply { error, success } = &operation.outcome else {
        return format!(
            "        return answer(ctx, \"{}\", {message_expr}, {extras}, {}, () => \
             {DEFAULT_BINDING_ERROR_STATUS});\n",
            operation.wire_name, shape.ok_status,
        );
    };
    let plain_json = shape.header_out.is_empty()
        && shape.error_header_out.is_empty()
        && !matches!(shape.body_kind, BodyKind::Bytes | BodyKind::Stream);
    if plain_json {
        let closure = error_status_closure(shape, error);
        return format!(
            "        return answer(ctx, \"{}\", {message_expr}, {extras}, {}, {closure});\n",
            operation.wire_name, shape.ok_status,
        );
    }
    custom_reply_block(operation, shape, message_expr, error, success, ctx)
}

// ---------------------------------------------------------------------------------------------
// The per-arm comment: which of H8's three assembly rules this operation's message follows
// ---------------------------------------------------------------------------------------------

fn arm_comment(operation: &OperationDef, shape: &HttpShape) -> String {
    let wire = &operation.wire_name;
    let method = shape.method.name();
    let path = shape.path_template();
    let lines = assembly_rule_lines(operation, shape);
    let mut out = String::new();
    for (index, line) in lines.iter().enumerate() {
        if index == 0 {
            let _ = writeln!(out, "    // {wire}: {method} {path} — {line}");
        } else {
            let _ = writeln!(out, "    // {line}");
        }
    }
    out
}

fn assembly_rule_lines(operation: &OperationDef, shape: &HttpShape) -> Vec<String> {
    let bodied = shape.method.carries_a_body();
    let multipart = matches!(shape.body_kind, BodyKind::Multipart);
    match &operation.inputs {
        OperationInputs::Empty => empty_rule_lines(bodied, multipart),
        OperationInputs::Named(named_type) => {
            named_rule_lines(named_type, &shape.placeholder_names())
        }
        OperationInputs::Generated(fields) => {
            generated_rule_lines(fields, shape, bodied, multipart)
        }
    }
}

fn empty_rule_lines(bodied: bool, multipart: bool) -> Vec<String> {
    if bodied && !multipart {
        vec!["the message is the parsed body.".to_owned()]
    } else {
        vec!["no argument beside the context: the message is an empty object.".to_owned()]
    }
}

/// The doc line describing a `Named` message's build rule, for whichever of
/// [`named_message_build`]'s shapes applies.
fn named_rule_lines(named_type: &Type, placeholders: &[String]) -> Vec<String> {
    if placeholders.is_empty() {
        return empty_rule_lines(true, false);
    }
    if placeholders.len() == 1 && is_scalar_named_type(named_type) {
        return vec!["the message IS the one placeholder (a wire scalar).".to_owned()];
    }
    let (word, is_are) = if placeholders.len() == 1 {
        ("placeholder", "is")
    } else {
        ("placeholders", "are")
    };
    vec![format!(
        "an author-declared message: the {word} {is_are} inserted under its written spelling as \
         a string, merged onto the parsed body."
    )]
}

fn generated_rule_lines(
    fields: &[(syn::Ident, Type)],
    shape: &HttpShape,
    bodied: bool,
    multipart: bool,
) -> Vec<String> {
    if multipart {
        return vec![
            "a macro-generated message: placeholder-bound fields come from the path, the rest \
             from the named parts; a `part(...)` binding reaches the implementation through no \
             slot here (mirrors a dropped header)."
                .to_owned(),
        ];
    }
    if bodied {
        return vec![
            "a macro-generated message: placeholder-bound fields come from the path, the rest \
             from the parsed JSON body."
                .to_owned(),
        ];
    }
    let placeholders = shape.placeholder_names();
    if fields
        .iter()
        .all(|(field, _)| placeholders.contains(&field.to_string()))
    {
        return vec![
            "a macro-generated message: every field comes from its own placeholder.".to_owned(),
        ];
    }
    vec![
        "a macro-generated message: placeholder-bound fields come from the path, the rest from \
         the query string, each with its own coercion."
            .to_owned(),
    ]
}

// ---------------------------------------------------------------------------------------------
// Bytes, stream and header_out replies: shapes `answer` cannot express
// ---------------------------------------------------------------------------------------------

/// The statements shared by every custom reply: dispatch, catch a panic, and answer a declared
/// error. Ends with `envelope.value` ready to read on the success path, which the caller writes on,
/// and `replied` holding the headers the dispatcher answered where the operation declares any.
fn dispatch_envelope_preamble(
    wire: &str,
    message_expr: &str,
    body_ty: &str,
    shape: &HttpShape,
    error: &Type,
    ctx: &DispatcherContext,
) -> String {
    let DispatcherContext {
        named,
        prefix,
        has_multipart,
    } = *ctx;
    let extras = dispatch_extras(shape, has_multipart);
    let replied = if shape.header_out.is_empty() && shape.error_header_out.is_empty() {
        ""
    } else {
        "        const replied = dispatched === undefined ? [] : dispatched.headers;\n"
    };
    let declared_error_stmt = declared_error_response_stmt(wire, shape, error, ctx);
    format!(
        "        let dispatched: {named}Dispatched | undefined;\n        \
         try {{\n          \
         dispatched = await dispatch(ctx, \"{wire}\", {message_expr}, {extras});\n        \
         }} catch (thrown) {{\n          \
         return onFault({prefix}HttpFault(\"handler-panic\", \"{wire}\", thrown instanceof \
         Error ? thrown.message : String(thrown)));\n        \
         }}\n        \
         const envelope = dispatched?.answered as {{ ok: true; value: {body_ty} }} | {{ ok: \
         false; error: unknown }};\n\
{replied}        \
         if (!envelope.ok) {{\n          \
         const error = envelope.error as {{ isServiceFault?: true; fault?: {named}Fault \
         }};\n          \
         if (typeof error === \"object\" && error !== null && error.isServiceFault === true \
         && error.fault !== undefined) return onFault(error.fault);\n\
{declared_error_stmt}        \
         }}\n"
    )
}

/// The declared-error response [`dispatch_envelope_preamble`] answers with: the mapped status
/// with the error as bare JSON, and a checked response header per `error_header_out` entry the
/// dispatcher answered.
fn declared_error_response_stmt(
    wire: &str,
    shape: &HttpShape,
    error: &Type,
    ctx: &DispatcherContext,
) -> String {
    let error_head = error_declared_type(shape.error_header_out.len(), error);
    let closure = error_status_closure(shape, error_head);
    if shape.error_header_out.is_empty() {
        return format!(
            "          return {}HttpJson(({closure})(envelope.error), [], envelope.error);\n",
            ctx.prefix
        );
    }
    let idents = error_header_out_idents(shape);
    let head_ty = get_field_def("error", error_head, "").typescript_typename();
    let types = message::header_types(shape.error_header_out.len(), error);
    let mut stmt = format!(
        "          const declaredError = envelope.error as {head_ty};\n\
{reads}          \
         const headers: Array<[string, string]> = [];\n",
        reads = replied_header_reads(
            ctx.prefix,
            &shape.error_header_out,
            &idents,
            &types,
            "          "
        ),
    );
    for push in checked_header_pushes(ctx.prefix, wire, &shape.error_header_out, &idents, &types) {
        let _ = writeln!(stmt, "          {push}");
    }
    let _ = writeln!(
        stmt,
        "          return {}HttpJson(({closure})(declaredError), headers, declaredError);",
        ctx.prefix,
    );
    stmt
}

/// One local per declared header, read back off `replied` and typed as the header's own element.
fn replied_header_reads(
    prefix: &str,
    names: &[String],
    idents: &[String],
    types: &[&Type],
    indent: &str,
) -> String {
    let mut stmt = String::new();
    for ((name, ident), ty) in names.iter().zip(idents).zip(types) {
        let element_ty = get_field_def("value", ty, "").typescript_typename();
        let _ = writeln!(
            stmt,
            "{indent}const {ident} = {prefix}HttpRepliedHeader(replied, \"{name}\") as \
             {element_ty};"
        );
    }
    stmt
}

/// One `[name, value]` header entry's own value, rendered through the field's own encoding: a
/// `Vec` joined with `,`, everything else `String(...)`.
fn header_out_encode_expr(ty: &Type, value_expr: &str) -> String {
    let base = option_inner(ty).unwrap_or(ty);
    if vec_inner(base).is_some() {
        return format!("({value_expr}).map((piece: unknown) => String(piece)).join(\",\")");
    }
    format!("String({value_expr})")
}

/// The local identifiers a `header_out` destructure binds, one per declared entry.
fn header_out_idents(shape: &HttpShape) -> Vec<String> {
    (0..shape.header_out.len())
        .map(|index| format!("headerOut{index}"))
        .collect()
}

/// [`header_out_idents`]'s own twin on the error side, named apart so an operation declaring both
/// never binds two locals under one name.
fn error_header_out_idents(shape: &HttpShape) -> Vec<String> {
    (0..shape.error_header_out.len())
        .map(|index| format!("errorHeaderOut{index}"))
        .collect()
}

/// One checked push per declared name: an illegal value answers a fault through `onFault`
/// instead of reaching the response. `types` holds each header's own element type.
fn checked_header_pushes(
    prefix: &str,
    wire: &str,
    names: &[String],
    idents: &[String],
    types: &[&Type],
) -> Vec<String> {
    names
        .iter()
        .zip(idents)
        .zip(types)
        .map(|((name, ident), ty)| {
            let encode = header_out_encode_expr(ty, ident);
            let push = checked_header_push_stmt(prefix, wire, name, &encode);
            // An `Option<T>` entry pushes nothing for `undefined`, rather than the literal text
            // `String(undefined)` would otherwise render.
            if option_inner(ty).is_some() {
                format!("if ({ident} !== undefined) {{\n          {push}\n        }}")
            } else {
                push
            }
        })
        .collect()
}

/// One checked header push: `value_expr` is checked before it is pushed onto the `headers` local
/// already in scope wherever this is spliced in.
fn checked_header_push_stmt(prefix: &str, wire: &str, name: &str, value_expr: &str) -> String {
    format!(
        "{{\n            \
         const rendered = {value_expr};\n            \
         if (!{prefix}HttpLegalResponseHeaderValue(rendered)) {{\n              \
         return onFault({prefix}HttpFault(\"handler-panic\", \"{wire}\", \"a response header \
         value contained a character illegal in an HTTP header\"));\n            \
         }}\n            \
         headers.push([\"{name}\", rendered]);\n          \
         }}"
    )
}

/// A `body = "bytes"` reply: the bytes and their content type destructured off `envelope.value`,
/// any declared `header_out` element read back off the dispatcher's own headers, answered bare —
/// mirrors the Rust `bytes_answer_block`. The content type is checked the same way every other
/// runtime-computed header value is.
fn bytes_reply_block(
    operation: &OperationDef,
    shape: &HttpShape,
    message_expr: &str,
    error: &Type,
    success: &Type,
    ctx: &DispatcherContext,
) -> String {
    let wire = &operation.wire_name;
    let pair = tuple_elements(success)
        .into_iter()
        .flatten()
        .take(2)
        .map(|ty| get_field_def("value", ty, "").typescript_typename())
        .collect::<Vec<_>>()
        .join(", ");
    let mut out =
        dispatch_envelope_preamble(wire, message_expr, &format!("[{pair}]"), shape, error, ctx);
    out.push_str("        const [bytes, contentType] = envelope.value;\n");
    let content_type_push =
        checked_header_push_stmt(ctx.prefix, wire, "content-type", "contentType");
    out.push_str(&header_out_writes(
        wire,
        shape,
        success,
        ctx,
        &format!("        {content_type_push}\n"),
    ));
    let _ = writeln!(
        out,
        "        return {{ status: {}, headers, body: new Uint8Array(bytes) }};",
        shape.ok_status,
    );
    out
}

/// A `body = "stream"` reply: `206` with `content-range` where the answer was partial, the
/// declared `ok_status` with no extra header where it was full, either way the body handed on
/// undrained — mirrors the Rust `stream_answer_block`. `content-range` is checked the same way
/// every other runtime-computed header value is.
fn stream_reply_block(
    operation: &OperationDef,
    shape: &HttpShape,
    message_expr: &str,
    error: &Type,
    success: &Type,
    ctx: &DispatcherContext,
) -> String {
    let wire = &operation.wire_name;
    let mut out = dispatch_envelope_preamble(
        wire,
        message_expr,
        STREAMED_ANSWER_TS_TYPE,
        shape,
        error,
        ctx,
    );
    out.push_str("        const answer = envelope.value;\n");
    let range_push =
        checked_header_push_stmt(ctx.prefix, wire, "content-range", "answer.contentRange");
    out.push_str(&header_out_writes(
        wire,
        shape,
        success,
        ctx,
        &format!("        if (answer.contentRange !== undefined) {{\n          {range_push}\n        }}\n"),
    ));
    let _ = writeln!(
        out,
        "        const status = answer.contentRange === undefined ? {} : 206;",
        shape.ok_status,
    );
    out.push_str("        return { status, headers, body: answer.body };\n");
    out
}

/// A JSON reply declaring `header_out`, an `error_header_out`, or both: the value off
/// `envelope.value`, and each declared `header_out` element off the dispatcher's own headers.
fn header_out_json_reply_block(
    operation: &OperationDef,
    shape: &HttpShape,
    message_expr: &str,
    error: &Type,
    success: &Type,
    ctx: &DispatcherContext,
) -> String {
    let wire = &operation.wire_name;
    let body = message::body_type(shape.header_out.len(), success);
    let body_ty = get_field_def("value", body, "").typescript_typename();
    let mut out = dispatch_envelope_preamble(wire, message_expr, &body_ty, shape, error, ctx);
    out.push_str("        const value = envelope.value;\n");
    out.push_str(&header_out_writes(wire, shape, success, ctx, ""));
    let _ = writeln!(
        out,
        "        return {}HttpJson({}, headers, value);",
        ctx.prefix, shape.ok_status
    );
    out
}

/// The response's `headers` list: the body kind's own header pushes (`own`), then a checked push
/// per declared `header_out` element read back off the dispatcher's own headers.
fn header_out_writes(
    wire: &str,
    shape: &HttpShape,
    success: &Type,
    ctx: &DispatcherContext,
    own: &str,
) -> String {
    let idents = header_out_idents(shape);
    let types = message::header_types(shape.header_out.len(), success);
    let mut out = replied_header_reads(ctx.prefix, &shape.header_out, &idents, &types, "        ");
    out.push_str("        const headers: Array<[string, string]> = [];\n");
    out.push_str(own);
    for push in checked_header_pushes(ctx.prefix, wire, &shape.header_out, &idents, &types) {
        let _ = writeln!(out, "        {push}");
    }
    out
}

fn custom_reply_block(
    operation: &OperationDef,
    shape: &HttpShape,
    message_expr: &str,
    error: &Type,
    success: &Type,
    ctx: &DispatcherContext,
) -> String {
    if matches!(shape.body_kind, BodyKind::Bytes) {
        bytes_reply_block(operation, shape, message_expr, error, success, ctx)
    } else if matches!(shape.body_kind, BodyKind::Stream) {
        stream_reply_block(operation, shape, message_expr, error, success, ctx)
    } else {
        header_out_json_reply_block(operation, shape, message_expr, error, success, ctx)
    }
}
