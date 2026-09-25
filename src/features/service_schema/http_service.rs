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
//! and refuses the message. This module keeps no presence check of its own: it hands the request's
//! own `headers` and, where the service declares multipart, its own `parts` straight through to
//! `dispatch`, exactly as it hands the assembled message through.

use super::message;
use super::result::stream_success_ts_type;
use crate::field_type::get_field_def;
use crate::rename_rule::RenameRule;
use crate::service_schema::parse::{
    BodyKind, DEFAULT_BINDING_ERROR_STATUS, HttpShape, OperationDef, OperationInputs,
    OperationOutcome, PathSegment, ServiceDef, is_scalar_named_type, option_inner,
    service_declares_a_stream, service_declares_multipart, tuple_elements, type_leaf_name,
    vec_inner, wire_key,
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
    vec![
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
        dispatcher_fn(service, &named, &prefix),
    ]
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
        OperationInputs::Named(named_type) => named_message_build(
            operation,
            named_type,
            bodied,
            multipart,
            &shape.placeholder_names(),
            prefix,
        ),
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

fn named_message_build(
    operation: &OperationDef,
    named_type: &Type,
    bodied: bool,
    multipart: bool,
    placeholder_names: &[String],
    prefix: &str,
) -> (String, String) {
    if placeholder_names.is_empty() {
        return empty_message_build(operation, bodied, multipart, prefix);
    }
    if placeholder_names.len() == 1 && is_scalar_named_type(named_type) {
        return (
            String::new(),
            message::decode_ts_expr(named_type, &placeholder_names[0], prefix),
        );
    }
    let mut setup = if bodied && !multipart {
        parsed_body_base_stmt()
    } else {
        "        const message: Record<string, unknown> = {};\n".to_owned()
    };
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
/// Takes the whole request so it can hand `headers` and `parts` to the dispatcher unchecked.
fn answer_fn(ctx: &DispatcherContext) -> String {
    let DispatcherContext {
        named,
        prefix,
        has_multipart,
    } = *ctx;
    let parts_arg = if has_multipart { ", request.parts" } else { "" };
    format!(
        "  const answer = async (ctx: Ctx, request: {named}HttpRequest, operation: string, \
         payload: unknown, okStatus: number, errorStatus: (error: unknown) => number) => {{\n    \
         let answered: unknown;\n    \
         try {{\n      \
         answered = await dispatch(ctx, operation, payload, request.headers{parts_arg});\n    \
         }} catch (thrown) {{\n      \
         return onFault({prefix}HttpFault(\"handler-panic\", operation, thrown instanceof \
         Error ? thrown.message : String(thrown)));\n    \
         }}\n    \
         if (answered === undefined) return {{ status: okStatus, headers: [], body: new \
         Uint8Array() }};\n    \
         const envelope = answered as {{ ok: true; value: unknown }} | {{ ok: false; error: \
         unknown }};\n    \
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
    out.push_str(&arm_body(operation, &shape, &message_expr, ctx));
    out.push_str("      }\n");
    out.push_str("    }\n");
    out
}

fn arm_body(
    operation: &OperationDef,
    shape: &HttpShape,
    message_expr: &str,
    ctx: &DispatcherContext,
) -> String {
    let OperationOutcome::Reply { error, success } = &operation.outcome else {
        return format!(
            "        return answer(ctx, request, \"{}\", {message_expr}, {}, () => \
             {DEFAULT_BINDING_ERROR_STATUS});\n",
            operation.wire_name, shape.ok_status,
        );
    };
    let plain_json = shape.header_out.is_empty()
        && !matches!(shape.body_kind, BodyKind::Bytes | BodyKind::Stream);
    if plain_json {
        let closure = error_status_closure(shape, error);
        return format!(
            "        return answer(ctx, request, \"{}\", {message_expr}, {}, {closure});\n",
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
            named_rule_lines(named_type, bodied, multipart, &shape.placeholder_names())
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

fn named_rule_lines(
    named_type: &Type,
    bodied: bool,
    multipart: bool,
    placeholders: &[String],
) -> Vec<String> {
    if placeholders.is_empty() {
        return empty_rule_lines(bodied, multipart);
    }
    if placeholders.len() == 1 && is_scalar_named_type(named_type) {
        return vec!["the message IS the one placeholder (a wire scalar).".to_owned()];
    }
    let (word, is_are) = if placeholders.len() == 1 {
        ("placeholder", "is")
    } else {
        ("placeholders", "are")
    };
    if bodied && !multipart {
        return vec![format!(
            "an author-declared message: the {word} {is_are} inserted under its written \
             spelling as a string, merged onto the parsed body."
        )];
    }
    vec![
        format!("an author-declared message: the {word}"),
        format!(
            "{is_are} inserted under its written spelling as a string; a bodyless method reads \
             no body (Rust reads no query here either)."
        ),
    ]
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

/// The statements shared by every custom reply: dispatch, catch a panic, and — on a declared
/// error — either hand a framed fault to `onFault` or answer the mapped status as JSON. Ends with
/// `envelope.value` ready to read on the success path, which the caller writes on.
fn dispatch_envelope_preamble(
    wire: &str,
    message_expr: &str,
    success_ty: &str,
    closure: &str,
    ctx: &DispatcherContext,
) -> String {
    let DispatcherContext {
        named,
        prefix,
        has_multipart,
    } = *ctx;
    let parts_arg = if has_multipart { ", request.parts" } else { "" };
    format!(
        "        let answered: unknown;\n        \
         try {{\n          \
         answered = await dispatch(ctx, \"{wire}\", {message_expr}, request.headers{parts_arg});\n        \
         }} catch (thrown) {{\n          \
         return onFault({prefix}HttpFault(\"handler-panic\", \"{wire}\", thrown instanceof \
         Error ? thrown.message : String(thrown)));\n        \
         }}\n        \
         const envelope = answered as {{ ok: true; value: {success_ty} }} | {{ ok: false; \
         error: unknown }};\n        \
         if (!envelope.ok) {{\n          \
         const error = envelope.error as {{ isServiceFault?: true; fault?: {named}Fault \
         }};\n          \
         if (typeof error === \"object\" && error !== null && error.isServiceFault === true \
         && error.fault !== undefined) return onFault(error.fault);\n          \
         return {prefix}HttpJson(({closure})(envelope.error), [], envelope.error);\n        \
         }}\n"
    )
}

/// One `[name, value]` header entry, `value` rendered through the field's own encoding — a `Vec`
/// joined with `,`, everything else `String(...)`. Mirrors the Rust `encode_expr`'s own reading,
/// backwards: writing a header out rather than reading one in.
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

/// The `[["name", encoded], ...]` entries a custom reply writes its `header_out` values as,
/// reading each element's type `skip` slots into `success`'s tuple (past the body elements).
fn header_out_entries(shape: &HttpShape, success: &Type, idents: &[String], skip: usize) -> String {
    let elements: Vec<&Type> = tuple_elements(success).into_iter().flatten().collect();
    shape
        .header_out
        .iter()
        .zip(idents)
        .enumerate()
        .map(|(index, (name, ident))| {
            let encode = elements.get(skip + index).map_or_else(
                || format!("String({ident})"),
                |ty| header_out_encode_expr(ty, ident),
            );
            format!("[\"{name}\", {encode}]")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// A `body = "bytes"` reply: the bytes and their content type (and any declared `header_out`
/// elements after them) destructured off `envelope.value`, answered bare — mirrors the Rust
/// `bytes_answer_block`.
fn bytes_reply_block(
    operation: &OperationDef,
    shape: &HttpShape,
    message_expr: &str,
    error: &Type,
    success: &Type,
    ctx: &DispatcherContext,
) -> String {
    let wire = &operation.wire_name;
    let closure = error_status_closure(shape, error);
    let success_ty = get_field_def("value", success, "").typescript_typename();
    let mut out = dispatch_envelope_preamble(wire, message_expr, &success_ty, &closure, ctx);
    let idents = header_out_idents(shape);
    let extra = if idents.is_empty() {
        String::new()
    } else {
        format!(", {}", idents.join(", "))
    };
    let _ = writeln!(
        out,
        "        const [bytes, contentType{extra}] = envelope.value;"
    );
    let header_entries = header_out_entries(shape, success, &idents, 2);
    let headers_expr = if header_entries.is_empty() {
        "[[\"content-type\", contentType]]".to_owned()
    } else {
        format!("[[\"content-type\", contentType], {header_entries}]")
    };
    let _ = writeln!(
        out,
        "        const headers: Array<[string, string]> = {headers_expr};"
    );
    let _ = writeln!(
        out,
        "        return {{ status: {}, headers, body: new Uint8Array(bytes) }};",
        shape.ok_status,
    );
    out
}

/// A `body = "stream"` reply: `206` with `content-range` where the answer was partial, the
/// declared `ok_status` with no extra header where it was full, either way the body handed on
/// undrained — mirrors the Rust `stream_answer_block`.
fn stream_reply_block(
    operation: &OperationDef,
    shape: &HttpShape,
    message_expr: &str,
    error: &Type,
    success: &Type,
    ctx: &DispatcherContext,
) -> String {
    let wire = &operation.wire_name;
    let closure = error_status_closure(shape, error);
    let success_ty = stream_success_ts_type(shape, success);
    let mut out = dispatch_envelope_preamble(wire, message_expr, &success_ty, &closure, ctx);
    let idents = header_out_idents(shape);
    if idents.is_empty() {
        out.push_str("        const answer = envelope.value;\n");
    } else {
        let _ = writeln!(
            out,
            "        const [answer, {}] = envelope.value;",
            idents.join(", ")
        );
    }
    let header_entries = header_out_entries(shape, success, &idents, 1);
    let range_entry = "[\"content-range\", answer.contentRange]";
    let (partial_headers, full_headers) = if header_entries.is_empty() {
        (format!("[{range_entry}]"), "[]".to_owned())
    } else {
        (
            format!("[{range_entry}, {header_entries}]"),
            format!("[{header_entries}]"),
        )
    };
    let _ = writeln!(
        out,
        "        const status = answer.contentRange === undefined ? {} : 206;",
        shape.ok_status,
    );
    let _ = writeln!(
        out,
        "        const headers: Array<[string, string]> = answer.contentRange === undefined ? \
         {full_headers} : {partial_headers};"
    );
    out.push_str("        return { status, headers, body: answer.body };\n");
    out
}

/// A JSON reply declaring `header_out`: the value and each declared header element destructured
/// off `envelope.value`, the value alone answered as the JSON body — mirrors the Rust
/// `answer_block`'s own `header_out` composition.
fn header_out_json_reply_block(
    operation: &OperationDef,
    shape: &HttpShape,
    message_expr: &str,
    error: &Type,
    success: &Type,
    ctx: &DispatcherContext,
) -> String {
    let wire = &operation.wire_name;
    let closure = error_status_closure(shape, error);
    let success_ty = get_field_def("value", success, "").typescript_typename();
    let mut out = dispatch_envelope_preamble(wire, message_expr, &success_ty, &closure, ctx);
    let idents = header_out_idents(shape);
    let _ = writeln!(
        out,
        "        const [value, {}] = envelope.value;",
        idents.join(", ")
    );
    let header_entries = header_out_entries(shape, success, &idents, 1);
    let _ = writeln!(
        out,
        "        const headers: Array<[string, string]> = [{header_entries}];"
    );
    let _ = writeln!(
        out,
        "        return {}HttpJson({}, headers, value);",
        ctx.prefix, shape.ok_status
    );
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
