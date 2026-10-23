//! How the client and the service name the one message an operation receives.

use crate::field_type::get_field_def;
use crate::rename_rule::RenameRule;
use crate::service_schema::parse::{
    BodyKind, HttpShape, OperationDef, OperationInputs, ScalarKind, held_as_a_bigint, option_inner,
    scalar_kind, tuple_elements, vec_inner, written,
};
use crate::utils::{INEXACT_KEY_NOTE, ts_commented};
use syn::{Ident, Type};

/// The locals and parameters an emitted dispatcher's arm writes around a path placeholder's own
/// local.
const TAKEN_BY_A_DISPATCHER: [&str; 25] = [
    "answer",
    "bytes",
    "captured",
    "contentType",
    "ctx",
    "declaredError",
    "dispatched",
    "envelope",
    "error",
    "found",
    "headers",
    "headersIn",
    "message",
    "method",
    "parsedBody",
    "path",
    "queryMap",
    "raw",
    "rejected",
    "rendered",
    "replied",
    "request",
    "status",
    "thrown",
    "value",
];

/// The locals and parameters an emitted method writes around an operation's own argument: the
/// REST client's, the client's over a transport, and the dispatcher's.
const TAKEN_BY_A_METHOD: [&str; 29] = [
    "answer",
    "answered",
    "body",
    "contentRange",
    "contentType",
    "ctx",
    "declared",
    "error",
    "headers",
    "held",
    "impl",
    "operation",
    "outcome",
    "parsed",
    "parts",
    "path",
    "payload",
    "query",
    "queryParts",
    "received",
    "rendered",
    "replied",
    "req",
    "response",
    "sending",
    "status",
    "transport",
    "uncarried",
    "validated",
];

/// The words `tsc` refuses as a parameter or a local in a module: the reserved words, the ones
/// strict mode adds, and `arguments`, `await` and `eval`.
const REFUSED_AS_A_NAME: [&str; 48] = [
    "arguments",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "eval",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

/// The function named `function` that answers a value as it is written to the wire: each
/// `bigint` in it, at any depth of a list or a plain object, through `JSON.rawJSON`, which
/// `JSON.stringify` writes as the JSON number of its digits. Without `JSON.rawJSON` a `bigint` a
/// `number` holds exactly is that `number`, and any other is refused with a `RangeError`. Every
/// other value is answered as it is.
pub fn wire_fn(function: &str) -> String {
    format!(
        "/**\n \
         * `value` as it is written to the wire: each `bigint` through `JSON.rawJSON`, the JSON \
         number of\n \
         * its digits. Without `JSON.rawJSON`, a `bigint` a `number` holds exactly is that \
         `number`, and any\n \
         * other is refused.\n \
         */\n\
         function {function}(value: unknown): unknown {{\n  \
         if (typeof value === \"bigint\") {{\n    \
         const raw: unknown = Reflect.get(JSON, \"rawJSON\");\n    \
         if (typeof raw === \"function\") return raw(value.toString());\n    \
         if (Number.isSafeInteger(Number(value))) return Number(value);\n    \
         throw new RangeError(`${{value}} is written exactly only through JSON.rawJSON, which this \
         runtime has not`);\n  \
         }}\n  \
         if (Array.isArray(value)) return value.map((held) => {function}(held));\n  \
         if (typeof value === \"object\" && value !== null && Object.getPrototypeOf(value) === \
         Object.prototype) {{\n    \
         return Object.fromEntries(Object.entries(value).map(([key, held]) => [key, \
         {function}(held)]));\n  \
         }}\n  \
         return value;\n\
         }}"
    )
}

/// The function named `function` that reads a JSON text: each integer a `number` does not hold
/// exactly rebuilt from its source text as a `bigint`, as a `u64` and an `i64` beyond `2^53 - 1`
/// are written. A runtime that hands a reviver no source text keeps the `number`.
pub fn read_fn(function: &str) -> String {
    format!(
        "/**\n \
         * `text` read as JSON, each integer a `number` does not hold exactly rebuilt from its \
         source text\n \
         * as a `bigint`.\n \
         */\n\
         function {function}(text: string): unknown {{\n  \
         return JSON.parse(text, (_key, value: unknown, context?: {{ source?: string }}) =>\n    \
         typeof value === \"number\" &&\n    \
         !Number.isSafeInteger(value) &&\n    \
         context?.source !== undefined &&\n    \
         /^-?(0|[1-9][0-9]*)$/.test(context.source)\n      \
         ? BigInt(context.source)\n      \
         : value,\n  \
         );\n\
         }}"
    )
}

/// The function named `function` that parses `value` with `schema` and writes what it answers
/// through `writer`, the function [`wire_fn`] writes: a value the schema refuses, or one the wire
/// cannot hold, answers its issues.
pub fn checked_fn(function: &str, writer: &str) -> String {
    format!(
        "/**\n \
         * `value` parsed by `schema` and written as it goes on the wire: the issues of a value \
         the schema\n \
         * refuses, or one the wire cannot hold, otherwise.\n \
         */\n\
         function {function}<Parsed>(\n  \
         schema: ZodType<Parsed>,\n  \
         value: unknown,\n\
         ):\n  \
         | {{ success: true; data: Parsed; sent: unknown }}\n  \
         | {{ success: false; error: {{ issues: ReadonlyArray<{{ path: ReadonlyArray<PropertyKey>; \
         message: string }}> }} }} {{\n  \
         const parsed = schema.safeParse(value);\n  \
         if (!parsed.success) return {{ success: false, error: {{ issues: parsed.error.issues }} \
         }};\n  \
         try {{\n    \
         return {{ success: true, data: parsed.data, sent: {writer}(parsed.data) }};\n  \
         }} catch (unsent) {{\n    \
         return {{\n      \
         success: false,\n      \
         error: {{ issues: [{{ path: [], message: unsent instanceof Error ? unsent.message : \
         String(unsent) }}] }},\n    \
         }};\n  \
         }}\n\
         }}"
    )
}

/// `text` as a comment the TypeScript service code writes can hold it: each line terminator
/// spelled as its escape, so a `//` comment keeps to its line, and the rest as [`ts_commented`]
/// writes it, so a `/** */` one runs on.
pub fn commented(text: &str) -> String {
    ts_commented(&super::on_one_line(text))
}

/// The schema the message validates against: the one `#[model_schema()]` published for it, read
/// through the same field walk every other reference to the type goes through rather than by
/// pasting a suffix onto a name.
pub fn schema(operation: &OperationDef) -> String {
    match &operation.inputs {
        OperationInputs::Named(declared) => get_field_def("req", declared, "").zod_type(),
        OperationInputs::Empty | OperationInputs::Generated(_) => operation
            .generated_message_ident()
            .map_or_else(String::new, |declared| {
                let named: syn::Type = syn::parse_quote! { #declared };
                get_field_def("req", &named, "").zod_type()
            }),
    }
}

/// The message's TypeScript name: the type the operation named, or the one the macro declared for
/// an operation that named none.
pub fn typename(operation: &OperationDef) -> String {
    match &operation.inputs {
        OperationInputs::Named(declared) => {
            get_field_def("req", declared, "").typescript_typename()
        }
        OperationInputs::Empty | OperationInputs::Generated(_) => operation
            .generated_message_ident()
            .map_or_else(|| "unknown".to_owned(), |declared| declared.to_string()),
    }
}

/// `name` with a trailing underscore where TypeScript refuses the word, having no escape for
/// one, and where `taken` holds it: the names written around it, which it would be read as.
fn moved_off(name: &str, taken: &[&str]) -> String {
    if REFUSED_AS_A_NAME.contains(&name) || taken.contains(&name) {
        format!("{name}_")
    } else {
        name.to_owned()
    }
}

/// The local a dispatcher's arm holds the captured text of the placeholder `name` in.
pub fn placeholder_local(name: &str) -> String {
    moved_off(name, &TAKEN_BY_A_DISPATCHER)
}

/// The TypeScript name of an operation's own argument: its Rust name camel-cased. An argument is
/// taken by position, so moving its name changes nothing for a caller.
pub fn parameter_name(parameter: &Ident) -> String {
    moved_off(
        &RenameRule::CamelCase.apply_to_field(&written(parameter)),
        &TAKEN_BY_A_METHOD,
    )
}

/// The arguments a bound header and a bound part add after the message, in declaration order,
/// named and typed the same way for the client's method and the implementation's.
pub fn binding_params(shape: &HttpShape) -> Vec<(String, String)> {
    let mut params = Vec::new();
    for header in &shape.header_in {
        let name = parameter_name(&header.parameter);
        params.push((
            name.clone(),
            get_field_def(&name, &header.ty, "").typescript_typename(),
        ));
    }
    for part in &shape.multipart_parts {
        let name = parameter_name(&part.parameter);
        params.push((
            name.clone(),
            get_field_def(&name, &part.ty, "").typescript_typename(),
        ));
    }
    params
}

/// Whether an argument a method of the operation takes writes a map keyed by a `u64` or an `i64`:
/// the message it names, a bound header or a bound part. A message the macro declares from the
/// arguments carries the note on each member that writes one.
pub fn sends_an_inexact_key(operation: &OperationDef) -> bool {
    let shape = HttpShape::of(operation);
    let named = match &operation.inputs {
        OperationInputs::Named(declared) => {
            get_field_def("req", declared, "").writes_an_inexact_key()
        }
        OperationInputs::Empty | OperationInputs::Generated(_) => false,
    };
    named
        || shape
            .header_in
            .iter()
            .map(|header| &header.ty)
            .chain(shape.multipart_parts.iter().map(|part| &part.ty))
            .any(|ty| get_field_def("value", ty, "").writes_an_inexact_key())
}

/// The `JSDoc` a method of a client or an implementation is written under: `summary` on one line,
/// or a block of `summary` and `below`, the block's own lines, closed by [`INEXACT_KEY_NOTE`] where
/// the method sends a map keyed by a `u64` or an `i64`.
pub fn method_jsdoc(summary: &str, below: &str, operation: &OperationDef) -> String {
    let mut lines: Vec<String> = Vec::new();
    if !below.is_empty() {
        lines.push(below.to_owned());
    }
    if sends_an_inexact_key(operation) {
        lines.push(format!("   * {INEXACT_KEY_NOTE}"));
    }
    if lines.is_empty() {
        format!("  /** {summary} */")
    } else {
        format!("  /**\n   * {summary}\n   *\n{}\n   */", lines.join("\n"))
    }
}

/// The type a header tuple carries in its body slot: the tuple's first element where `headers`
/// elements were declared after it, or `ty` itself where none were.
pub fn body_type(headers: usize, ty: &Type) -> &Type {
    if headers == 0 {
        return ty;
    }
    tuple_elements(ty)
        .and_then(|elements| elements.first())
        .unwrap_or(ty)
}

/// `text` with `by` more spaces ahead of each of its lines.
pub fn deeper(text: &str, by: usize) -> String {
    let margin = " ".repeat(by);
    text.lines().fold(String::new(), |mut moved, line| {
        moved.push_str(&margin);
        moved.push_str(line);
        moved.push('\n');
        moved
    })
}

/// The type a reply's success carries as its body: [`body_type`], but a pair of the bytes and
/// their content type for a `body = "bytes"` operation that declares a header beside them, as the
/// TypeScript dispatcher answers it.
pub fn success_body(shape: &HttpShape, success: &Type) -> Type {
    if matches!(shape.body_kind, BodyKind::Bytes)
        && !shape.header_out.is_empty()
        && let Some(elements) = tuple_elements(success)
        && let (Some(bytes), Some(content_type)) = (elements.get(0), elements.get(1))
    {
        return syn::parse_quote! { (#bytes, #content_type) };
    }
    body_type(shape.header_out.len(), success).clone()
}

/// The types of a header tuple's trailing header elements, one per declared name, in order.
pub fn header_types(headers: usize, ty: &Type) -> Vec<&Type> {
    let elements: Vec<&Type> = tuple_elements(ty).into_iter().flatten().collect();
    elements[elements.len().saturating_sub(headers)..].to_vec()
}

/// Mirrors the Rust `decode_expr`: a `Vec<...>` splits `raw` on `,` and coerces each piece the
/// same way; otherwise a boolean text, a numeric coercion, or the text itself, a `u64` and an
/// `i64` read from the text of its digits as the `bigint` it holds. Shared by the REST server's
/// query/path decoding and the dispatcher's header/part decoding.
pub fn decode_ts_expr(ty: &Type, raw: &str, prefix: &str) -> String {
    let base = option_inner(ty).unwrap_or(ty);
    if let Some(inner) = vec_inner(base) {
        let element = decode_ts_expr(inner, "piece", prefix);
        return format!("({raw}).split(\",\").map((piece: string) => {element})");
    }
    match scalar_kind(base) {
        ScalarKind::Bool => {
            format!("({raw} === \"true\" ? true : {raw} === \"false\" ? false : {raw})")
        }
        ScalarKind::Number if held_as_a_bigint(base) => format!("{prefix}HttpCoerceBigInt({raw})"),
        ScalarKind::Number => format!("{prefix}HttpCoerceNumber({raw})"),
        ScalarKind::Text => raw.to_owned(),
    }
}
