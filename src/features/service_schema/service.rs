//! The TypeScript a service implements: an interface it satisfies in full or does not compile, the
//! outcomes its operations answer with, and the factory that turns an implementation into a
//! dispatcher.
//!
//! # Why an interface and not a table of handlers
//!
//! This is the piece the whole construct exists for. An operation declared and implemented by
//! nobody is only prevented if the compiler refuses the incomplete implementation, so the emitted
//! interface has one required member per operation — no optional members, no index signature,
//! nothing a partial implementation slips through. Adding an operation breaks every implementation
//! of the service, which is the point.
//!
//! # An implementation cannot fabricate a fault
//!
//! An operation publishes two types, not one. `<Service><Operation>Result` is what a *caller*
//! reads: the value, the declared error, or a fault. `<Service><Operation>Outcome` is what an
//! *implementation* returns, and its failure arm is the declared error alone. A fault reports a
//! failure the operation never declared, and the two places entitled to build one are both
//! generated — this dispatcher and the client.
//!
//! # The dispatcher exists only beside a schema
//!
//! An arm parses the payload before it calls, which is what entitles an implementation to assume
//! its message is valid. The parse is against the `<Message>$Schema` const `#[model_schema()]`
//! publishes, so this module is gated with the Zod surface that writes one: a build without it
//! publishes no dispatcher rather than one that narrows an unread payload with `as` and hands it to
//! an implementation written against a guarantee nothing checked.
//!
//! # The context is explicit and generic
//!
//! The interface carries a context type parameter and every method takes it, mirroring the Rust
//! trait. The code owning the transport constructs one per message and hands it to the dispatcher.
//! It appears in no message and no schema.

use super::fault;
use super::message;
use super::result::result_name;
use crate::field_type::get_field_def;
use crate::rename_rule::RenameRule;
use crate::service_schema::parse::{
    BodyKind, HttpShape, OperationDef, OperationOutcome, ServiceDef, is_unit_type, option_inner,
};
use core::fmt::Write as _;
use syn::Type;

/// One side of [`header_tuple_split`].
struct SplitSide<'shape> {
    /// The answered envelope around the tuple's body: what goes before it, and after it.
    envelope: (&'static str, &'static str),
    ident_prefix: &'static str,
    indent: &'static str,
    names: &'shape [String],
    tuple: &'static str,
    types: Vec<&'shape Type>,
}

impl SplitSide<'_> {
    /// Destructures the tuple, pushes each named header, and answers the envelope with the body
    /// in place of the tuple. With no header named, the outcome is answered as it is.
    fn stmt(&self, body_width: usize) -> String {
        let margin = self.indent;
        if self.names.is_empty() {
            return format!("{margin}return {{ answered: outcome, headers: [] }};\n");
        }
        let body: Vec<String> = (0..body_width)
            .map(|index| format!("body{index}"))
            .collect();
        let idents: Vec<String> = (0..self.names.len())
            .map(|index| format!("{}{index}", self.ident_prefix))
            .collect();
        let mut stmt = format!(
            "{margin}const [{}, {}] = {};\n\
             {margin}const replied: Array<[string, string]> = [];\n",
            body.join(", "),
            idents.join(", "),
            self.tuple,
        );
        for ((name, ty), ident) in self.names.iter().zip(&self.types).zip(&idents) {
            let push = format!("replied.push([\"{name}\", JSON.stringify({ident})]);");
            if option_inner(ty).is_some() {
                let _ = writeln!(stmt, "{margin}if ({ident} != null) {push}");
            } else {
                let _ = writeln!(stmt, "{margin}{push}");
            }
        }
        let answered = if body_width == 1 {
            body.join("")
        } else {
            format!("[{}]", body.join(", "))
        };
        let (before, after) = self.envelope;
        let _ = writeln!(
            stmt,
            "{margin}return {{ answered: {before}{answered}{after}, headers: replied }};"
        );
        stmt
    }
}

pub fn emit(service: &ServiceDef) -> Vec<String> {
    let mut published = outcome_types(service);
    published.push(interface(service));
    published.push(dispatched_type(&service.ident.to_string()));
    published.extend(fault_helpers(service));
    published.push(dispatcher(service));
    published
}

/// One arm of the dispatcher's `switch`: parse the payload, read and decode each bound header and
/// part, then call. An implementation may assume its message is valid and every bound value
/// present or `undefined` as declared, because neither ever reaches it otherwise.
fn arm(service: &ServiceDef, operation: &OperationDef) -> String {
    let wire = &operation.wire_name;
    let call = &operation.ts_name;
    let received = payload_check(service, operation);
    let shape = HttpShape::of(operation);
    let (bindings, arg_names) = binding_reads(service, operation, &shape);
    let mut arguments = vec!["ctx".to_owned(), "received.data".to_owned()];
    arguments.extend(arg_names);
    let call_args = arguments.join(", ");
    let answering = match &operation.outcome {
        OperationOutcome::OneWay => {
            format!("        await impl.{call}({call_args});\n        return undefined;")
        }
        OperationOutcome::Reply { error, success }
            if !shape.header_out.is_empty() || !shape.error_header_out.is_empty() =>
        {
            let mut split = format!("        const outcome = await impl.{call}({call_args});\n");
            split.push_str(header_tuple_split(&shape, error, success).trim_end());
            split
        }
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => {
            format!("        return {{ answered: await impl.{call}({call_args}), headers: [] }};")
        }
    };
    format!("      case \"{wire}\": {{\n{received}{bindings}{answering}\n      }}")
}

/// Splits an outcome whose success or declared error is a header tuple: the body answers as the
/// envelope's `value` or `error`, and each header element is written JSON-encoded under its
/// declared name, an optional one holding `null` written nowhere. A body two elements wide —
/// `body = "bytes"`'s bytes and content type — stays a pair, the value the operation would answer
/// with no header declared.
fn header_tuple_split(shape: &HttpShape, error: &Type, success: &Type) -> String {
    let body_width = if matches!(shape.body_kind, BodyKind::Bytes) {
        2
    } else {
        1
    };
    let success_side = SplitSide {
        envelope: ("{ ok: true, value: ", " }"),
        ident_prefix: "headerOut",
        indent: "          ",
        names: &shape.header_out,
        tuple: "outcome.value",
        types: message::header_types(shape.header_out.len(), success),
    };
    let error_side = SplitSide {
        envelope: ("{ ok: false, error: ", " }"),
        ident_prefix: "errorHeaderOut",
        indent: "        ",
        names: &shape.error_header_out,
        tuple: "outcome.error",
        types: message::header_types(shape.error_header_out.len(), error),
    };
    format!(
        "        if (outcome.ok) {{\n{}        }}\n{}",
        success_side.stmt(body_width),
        error_side.stmt(1),
    )
}

/// The statements that look up and decode each `header_in` and `part` binding, refusing through
/// the same framed fault a bad payload gets where a required one is missing — mirrors the Rust
/// dispatcher's own `header_in_reads`/`multipart_part_let`. A header's text is the JSON encoding
/// both the `ws_rpc` frame and the AMQP headers table carry, checked against the header's own
/// schema.
fn binding_reads(
    service: &ServiceDef,
    operation: &OperationDef,
    shape: &HttpShape,
) -> (String, Vec<String>) {
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let wire = &operation.wire_name;
    let mut stmt = String::new();
    let mut bound = Vec::new();
    for header in &shape.header_in {
        let name = RenameRule::CamelCase.apply_to_field(&header.parameter.to_string());
        let read = format!("{name}Header");
        let schema = get_field_def(&name, &header.ty, "").zod_type();
        let _ = write!(
            stmt,
            "        const {read} = {prefix}RequestHeader(\"{wire}\", headers, \"{header_name}\", \
             {schema});\n        \
             if (!{read}.ok) {{\n          \
             return {prefix}Framed({read}.fault);\n        \
             }}\n",
            header_name = header.name,
        );
        bound.push(format!("{read}.value"));
    }
    for part in &shape.multipart_parts {
        let name = RenameRule::CamelCase.apply_to_field(&part.parameter.to_string());
        let _ = writeln!(
            stmt,
            "        const {name} = parts.find(([name]) => name === \"{part_name}\")?.[1];",
            part_name = part.name,
        );
        let _ = write!(
            stmt,
            "        if ({name} === undefined) {{\n          \
             return {prefix}Framed({prefix}InboundFault(\"{wire}\", [{{ path: \
             [\"{part_name}\"], message: \"a required multipart part was not carried\" \
             }}]));\n        \
             }}\n",
            part_name = part.name,
        );
        bound.push(name);
    }
    (stmt, bound)
}

/// What the dispatcher answers a request-and-reply operation with: the envelope that goes on the
/// wire, and the headers written beside it.
fn dispatched_type(named: &str) -> String {
    format!(
        "/**\n \
         * What a `{named}` dispatcher answers a request with: the envelope that goes on the wire \
         —\n \
         * the operation's own result envelope, or a fault framed inside a failure arm — and the \
         headers\n \
         * written beside it, each value JSON-encoded. A transport writes both: a `ws_rpc` reply \
         frame\n \
         * under `headers`, an AMQP reply as the message's own headers.\n \
         */\n\
         export type {named}Dispatched = {{\n  \
         answered: unknown;\n  \
         headers: ReadonlyArray<readonly [string, string]>;\n\
         }};"
    )
}

/// The factory: an implementation in, a dispatch function out. It answers with what the transport
/// puts on the wire — the operation's envelope, or a fault framed inside a failure arm, beside the
/// headers to write — and with nothing at all for a one-way operation that ran.
fn dispatcher(service: &ServiceDef) -> String {
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let arms = service
        .operations
        .iter()
        .map(|operation| arm(service, operation))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "/**\n \
         * Turns a `{named}` implementation into the function a transport drives it with.\n \
         *\n \
         * The operation is read from the argument the transport passed beside the payload, never \
         out\n \
         * of the payload itself, and each `header_in` value from the headers beside it, \
         JSON-encoded as\n \
         * a `ws_rpc` frame and an AMQP message both carry them. What comes back is what goes on \
         the\n \
         * wire: the envelope and its headers, or nothing at all where the operation expects no \
         reply.\n \
         */\n\
         export function create{named}Dispatcher<Ctx>(\n  \
         impl: {named}Impl<Ctx>,\n\
         ): (\n  \
         ctx: Ctx,\n  \
         operation: string,\n  \
         payload: unknown,\n  \
         headers?: ReadonlyArray<readonly [string, string]>,\n  \
         parts?: ReadonlyArray<readonly [string, unknown]>,\n\
         ) => Promise<{named}Dispatched | undefined> {{\n  \
         return async (ctx, operation, payload, headers = [], parts = []) => {{\n    \
         switch (operation) {{\n\
         {arms}\n      \
         default:\n        \
         return {prefix}Framed({prefix}UnknownOperation(operation));\n    \
         }}\n  \
         }};\n\
         }}"
    )
}

/// The three readers the dispatcher answers through: the framing a fault crosses in, the fault an
/// unrecognised operation gets, and the one a payload that failed its schema gets.
fn fault_helpers(service: &ServiceDef) -> Vec<String> {
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let mut helpers = vec![
        format!(
            "/**\n \
             * How a fault reaches a caller: inside the failure arm, behind the literal a caller \
             in\n \
             * either language narrows on, with no header beside it. It is the shape every \
             `{named}`\n \
             * result type declares, and the shape the Rust dispatcher's transport frames.\n \
             */\n\
             function {prefix}Framed(fault: {named}Fault): {named}Dispatched {{\n  \
             return {{ answered: {{ ok: false, error: {{ isServiceFault: true, fault }} }}, \
             headers: [] }};\n\
             }}"
        ),
        format!(
            "/**\n \
             * The fault an operation name nothing on `{named}` answers to produces. The name is \
             the\n \
             * one that arrived, not one the service declares.\n \
             */\n\
             function {prefix}UnknownOperation(operation: string): {named}Fault {{\n\
             {minted}\n\
             }}",
            minted = fault::minted(
                &named,
                "    detail: \"the service answers to no operation by that name\",\n    \
                 field: undefined,\n    \
                 kind: \"unknown-operation\",\n    \
                 operation,"
            )
        ),
    ];
    helpers.extend(inbound_fault(service));
    if service
        .operations
        .iter()
        .any(|operation| !HttpShape::of(operation).header_in.is_empty())
    {
        helpers.push(request_header_fn(&named, &prefix));
    }
    helpers
}

/// Reads one `header_in` value off the JSON text a transport handed over, checked against the
/// header's own schema: an absent header reads as `undefined`, which only an optional binding
/// accepts. Refuses through [`inbound_fault`], naming the header — the Rust dispatcher's own
/// `decoded_header` read.
fn request_header_fn(named: &str, prefix: &str) -> String {
    format!(
        "/**\n \
         * Reads one `header_in` value off the JSON text the transport handed over, checked \
         against\n \
         * the header's own schema. An absent header reads as `undefined`, which only an optional\n \
         * binding accepts.\n \
         */\n\
         function {prefix}RequestHeader<Parsed>(\n  \
         operation: string,\n  \
         headers: ReadonlyArray<readonly [string, string]>,\n  \
         name: string,\n  \
         schema: ZodType<Parsed>,\n\
         ): {{ ok: true; value: Parsed }} | {{ ok: false; fault: {named}Fault }} {{\n  \
         const carried = headers.find(([candidate]) => candidate.toLowerCase() === \
         name.toLowerCase())?.[1];\n  \
         let issues: ReadonlyArray<{{ path: ReadonlyArray<PropertyKey>; message: string }}>;\n  \
         try {{\n    \
         const parsed = schema.safeParse(carried === undefined ? undefined : \
         JSON.parse(carried));\n    \
         if (parsed.success) return {{ ok: true, value: parsed.data }};\n    \
         issues = carried === undefined\n      \
         ? [{{ path: [name], message: \"a required header was not carried\" }}]\n      \
         : parsed.error.issues.map((issue) => ({{ path: [name, ...issue.path], message: \
         issue.message }}));\n  \
         }} catch (rejected) {{\n    \
         issues = [{{ path: [name], message: String(rejected) }}];\n  \
         }}\n  \
         return {{ ok: false, fault: {prefix}InboundFault(operation, issues) }};\n\
         }}"
    )
}

/// The fault a payload that will not become the operation's message produces, under the one kind
/// this dispatcher can raise.
///
/// It runs on a payload somebody already parsed — the dispatcher takes the decoded value, not the
/// bytes — so by the time it is reached the bytes *were* a document and the only thing left to be
/// wrong is what the document said. That is what the Rust dispatcher calls a failed validation, and
/// it is the same reading: there, `serde_json`'s own classification separates bytes that are not a
/// document (`Syntax`, `Eof`) from a document that is not this message (`Data`), and only the second
/// can happen here at all. `undeserializable-payload` is the answer to the first, which belongs to
/// whatever turns bytes into the value handed in.
///
/// A failure at no key still names no field: a value that is not an object at all is not a message,
/// and there is no key to send a caller to.
fn inbound_fault(service: &ServiceDef) -> Vec<String> {
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    vec![format!(
        "/**\n \
         * The fault a payload produces when it will not become the operation's message.\n \
         *\n \
         * The payload reaching here was already read out of the bytes, so what is wrong with it \
         is\n \
         * what it *said* — the same thing the Rust dispatcher answers `failed-validation` for. \
         Bytes\n \
         * that are no document at all never reach this, and are the other kind.\n \
         *\n \
         * The fault names the key the failure was at. A failure at no key names none: a value \
         that\n \
         * is not an object is not this message, and there is no key to send a caller to.\n \
         */\n\
         function {prefix}InboundFault(\n  \
         operation: string,\n  \
         issues: ReadonlyArray<{{ path: ReadonlyArray<PropertyKey>; message: string }}>,\n\
         ): {named}Fault {{\n  \
         const [first] = issues;\n  \
         const failedAt = first === undefined ? \"\" : first.path.join(\".\");\n\
         {minted}\n\
         }}",
        minted = fault::minted(
            &named,
            "    detail: issues\n      \
             .map((issue) =>\n        \
             issue.path.length === 0 ? issue.message : `'${issue.path.join(\".\")}': \
             ${issue.message}`,\n      \
             )\n      \
             .join(\"; \"),\n    \
             field: failedAt === \"\" ? undefined : failedAt,\n    \
             kind: \"failed-validation\",\n    \
             operation,"
        )
    )]
}

/// The interface an implementation satisfies. Every member is required and none is optional, so an
/// implementation missing one is refused where it reaches the factory.
fn interface(service: &ServiceDef) -> String {
    let named = service.ident.to_string();
    let methods = service
        .operations
        .iter()
        .map(|operation| {
            format!(
                "  /** {} */\n  {}({}): Promise<{}>;",
                method_summary(&named, operation),
                operation.ts_name,
                interface_method_params(operation),
                implementation_answers(&named, operation)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "/**\n \
         * What a `{named}` implementation satisfies, in full or not at all.\n \
         *\n \
         * Every operation the service declares is a required member, so an implementation missing \
         one\n \
         * is refused where it reaches `create{named}Dispatcher`. Adding an operation breaks every\n \
         * implementation until each one handles it, which is what the interface is for.\n \
         *\n \
         * The context is the implementation's own type, constructed per message by whatever owns \
         the\n \
         * transport. It reaches no message and no schema.\n \
         *\n \
         * A bound `header_in` or `part(...)` binding adds one more argument after the message, in\n \
         * declaration order, named and typed exactly as `create{named}HttpClient`'s own method \
         takes\n \
         * it: the dispatcher decodes and refuses it the same way it does the message, so an \
         argument\n \
         * an implementation reads here is as trustworthy as `req` is.\n \
         */\n\
         export interface {named}Impl<Ctx> {{\n\
         {methods}\n\
         }}"
    )
}

/// One method's own parameter list: the context, the message, then one argument per `header_in`
/// binding and one per `part` binding, in declaration order — the same list
/// [`super::http_client::method_params`] renders for the client's own method.
fn interface_method_params(operation: &OperationDef) -> String {
    let shape = HttpShape::of(operation);
    let mut params = vec![
        "ctx: Ctx".to_owned(),
        format!("req: {}", message::typename(operation)),
    ];
    params.extend(
        message::binding_params(&shape)
            .into_iter()
            .map(|(name, ty)| format!("{name}: {ty}")),
    );
    params.join(", ")
}

/// What an implementation's method answers: the two arms the operation declared, and never a
/// fault.
fn implementation_answers(service: &str, operation: &OperationDef) -> String {
    outcome_name(service, operation).unwrap_or_else(|| "void".to_owned())
}

/// A one-line summary for the member's own `JSDoc`.
fn method_summary(service: &str, operation: &OperationDef) -> String {
    let wire = &operation.wire_name;
    match &operation.outcome {
        OperationOutcome::OneWay => {
            format!("Handles `{wire}` on `{service}`, which expects no reply.")
        }
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => {
            format!("Handles `{wire}` on `{service}` and answers it.")
        }
    }
}

/// What one operation's implementation-side type is called. It sits beside the caller-side result
/// type and differs from it in exactly one way: no fault.
fn outcome_name(service: &str, operation: &OperationDef) -> Option<String> {
    result_name(service, operation)
        .map(|published| format!("{}Outcome", published.trim_end_matches("Result")))
}

/// The outcome type per operation that answers, written out in full rather than derived from the
/// caller-side result: the two are read side by side, and a reader should be able to see that one
/// admits a fault and the other does not.
fn outcome_types(service: &ServiceDef) -> Vec<String> {
    let named = service.ident.to_string();
    service
        .operations
        .iter()
        .filter_map(|operation| outcome_type(&named, operation))
        .collect()
}

fn outcome_type(service: &str, operation: &OperationDef) -> Option<String> {
    let OperationOutcome::Reply { error, success } = &operation.outcome else {
        return None;
    };
    let published = outcome_name(service, operation)?;
    let success_arm = if is_unit_type(success) {
        "{ ok: true }".to_owned()
    } else {
        let value = get_field_def("value", success, "").typescript_typename();
        format!("{{ ok: true; value: {value} }}")
    };
    let failure = get_field_def("error", error, "").typescript_typename();
    let called = &operation.ts_name;
    Some(format!(
        "/**\n \
         * What an implementation of `{called}` on `{service}` answers with: the value it \
         declared, or\n \
         * the error it declared.\n \
         *\n \
         * A fault is not among them. It reports a failure the operation never declared, and the \
         two\n \
         * places entitled to build one are both generated — the dispatcher and the client.\n \
         */\n\
         export type {published} =\n  \
         | {success_arm}\n  \
         | {{ ok: false; error: {failure} }};"
    ))
}

/// The parse that runs before the implementation is called. It runs in every arm, this module
/// being emitted only where there is a schema to parse against.
fn payload_check(service: &ServiceDef, operation: &OperationDef) -> String {
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let wire = &operation.wire_name;
    let schema = message::schema(operation);
    format!(
        "        const received = {schema}.safeParse(payload);\n        \
         if (!received.success) {{\n          \
         return {prefix}Framed({prefix}InboundFault(\"{wire}\", received.error.issues));\n        \
         }}\n"
    )
}
