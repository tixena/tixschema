//! The TypeScript client: a factory over an injected transport, with one camelCase method per
//! operation, shaped so a call site reads exactly as the hand-written one it replaces.
//!
//! # What a call site sees
//!
//! ```typescript
//! const usage = createUsageServiceClient(transport);
//! const result = await usage.getBalance({ organizationId });
//! if (result.ok) {
//!   render(result.value.credits);
//! } else if ("isServiceFault" in result.error) {
//!   reportUnexpected(result.error.fault);
//! } else {
//!   switch (result.error.errorCode) { … }
//! }
//! ```
//!
//! # Outbound validation comes before the transport, not after it
//!
//! Every method parses its message against the message's own generated schema first. A failure
//! answers with a fault **without reaching the transport**, naming the key that failed: the
//! operation never ran, so what came back is not one of the errors it declared, and a caller's
//! code is identical whether the fault came from here or from the far end.
//!
//! # Where the schemas come from, and why this module exists only beside them
//!
//! The schema a message validates against is the one `#[model_schema()]` publishes for it, which
//! only a build with the Zod surface on writes at all. This module is gated with it: a client whose
//! check was dropped would forward whatever it was handed while reading exactly like the checked
//! one, so a build that publishes no schema publishes no client either.
//!
//! # A one-way method has nowhere to return a fault
//!
//! A one-way operation's method answers `Promise<void>`, so a refused message cannot come back as
//! a value. It is thrown instead — the transport is still never reached, and a defect that would
//! otherwise vanish stays visible.
//!
//! What is thrown is part of the published surface rather than something to be discovered from the
//! emitted body, so it is named: `<Service>Refusal`, an `Error` carrying the fault on a `fault`
//! property. The method's own `JSDoc` names it too, `Promise<void>` having no room to say it.

use super::fault;
use super::message;
use super::result::result_name;
use crate::field_type::get_field_def;
use crate::rename_rule::RenameRule;
use crate::service_schema::parse::{
    HttpShape, OperationDef, OperationOutcome, ServiceDef, is_unit_type, option_inner,
};
use core::fmt::Write as _;
use core::iter::once;
use syn::Type;

/// One side of a header-tuple reply: the body as it arrived, and the headers `names` binds,
/// rejoined into the tuple the operation declared.
struct Rejoined<'shape> {
    /// The returned envelope around the rejoined tuple: what goes before it, and after it.
    answered: (&'static str, &'static str),
    body: &'static str,
    ident_prefix: &'static str,
    indent: &'static str,
    names: &'shape [String],
    types: Vec<&'shape Type>,
}

impl Rejoined<'_> {
    /// Reads each named header back off `replied`, returning the fault a missing or malformed one
    /// produces, then returns the envelope with the body rejoined to them. With no header named,
    /// the body is returned as it arrived.
    fn stmt(&self, prefix: &str, wire: &str) -> String {
        let margin = self.indent;
        let mut stmt = String::new();
        let mut joined = vec![self.body.to_owned()];
        for (index, (name, ty)) in self.names.iter().zip(&self.types).enumerate() {
            let ident = format!("{}{index}", self.ident_prefix);
            let schema = get_field_def("value", ty, "").zod_slot_type();
            let _ = write!(
                stmt,
                "{margin}const {ident} = {prefix}ReplyHeader(\"{wire}\", replied, \"{name}\", \
                 {schema});\n\
                 {margin}if (!{ident}.ok) {{\n\
                 {margin}  return {{ ok: false, error: {{ isServiceFault: true, fault: \
                 {ident}.fault }} }};\n\
                 {margin}}}\n"
            );
            joined.push(format!("{ident}.value"));
        }
        let rejoined = if self.names.is_empty() {
            self.body.to_owned()
        } else {
            format!("[{}]", joined.join(", "))
        };
        let (before, after) = self.answered;
        let _ = writeln!(stmt, "{margin}return {before}{rejoined}{after};");
        stmt
    }
}

pub fn emit(service: &ServiceDef) -> Vec<String> {
    let mut published = vec![
        transport_type(&service.ident.to_string()),
        client_type(service),
    ];
    // The readers land ahead of the factory that calls them, a bundle being read top to bottom.
    published.extend(fault_helpers(service));
    published.push(factory(service));
    published
}

/// The client type: one method per operation, named the way a TypeScript caller expects to type it
/// and answering the operation's own result type.
fn client_type(service: &ServiceDef) -> String {
    let named = service.ident.to_string();
    let methods = service
        .operations
        .iter()
        .map(|operation| {
            format!(
                "{}\n  {}({}): Promise<{}>;",
                method_doc(&named, operation),
                operation.ts_name,
                method_params(operation),
                answers(&named, operation)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "/**\n \
         * A `{named}` caller, over any transport that can send an operation name beside a \
         payload.\n \
         *\n \
         * Every operation the service declares has a method here, taking the message and then \
         one\n \
         * argument per `header_in` binding. A request-and-reply operation answers its own result \
         type;\n \
         * a one-way operation answers nothing beyond the send.\n \
         */\n\
         export type {named}Client = {{\n\
         {methods}\n\
         }};"
    )
}

/// The factory: it binds a transport and answers with the client, every method built the same way
/// — validate, then send.
fn factory(service: &ServiceDef) -> String {
    let named = service.ident.to_string();
    let methods = service
        .operations
        .iter()
        .map(|operation| method(service, operation))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "/**\n \
         * Binds a `{named}` client to a transport.\n \
         *\n \
         * The operation name is handed to the transport beside the payload, never inside it, so \
         no\n \
         * message type has to reserve a key for routing.\n \
         */\n\
         export function create{named}Client(transport: {named}Transport): {named}Client {{\n  \
         return {{\n\
         {methods}\n  \
         }};\n\
         }}"
    )
}

/// The two readers a validation failure goes through, emitted only where there is a schema to fail
/// against — and the thrower only where a one-way operation needs somewhere to put a fault.
///
/// Their names carry the service for the same reason every published type's does: a bundle is one
/// flat file, and ten services would otherwise declare one of each ten times over.
fn fault_helpers(service: &ServiceDef) -> Vec<String> {
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let mut helpers = vec![format!(
        "/**\n \
         * The fault a `{named}` client answers with when the message it was about to send failed \
         its\n \
         * own schema. The operation never ran, so this is not one of the errors it declared, and \
         the\n \
         * transport was never reached.\n \
         */\n\
         function {prefix}OutboundFault(\n  \
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
    )];
    if service.operations.iter().any(|operation| {
        let shape = HttpShape::of(operation);
        !shape.header_out.is_empty() || !shape.error_header_out.is_empty()
    }) {
        helpers.extend(reply_header_helpers(&named, &prefix));
    }
    if service
        .operations
        .iter()
        .any(|operation| matches!(operation.outcome, OperationOutcome::OneWay))
    {
        helpers.push(format!(
            "/**\n \
             * What a one-way `{named}` method throws when it refuses the message it was \
             handed.\n \
             *\n \
             * `Promise<void>` has no failure arm and no value position, so the fault rides on \
             the\n \
             * thrown error rather than on something returned. Narrow a caught error with \
             `\"fault\" in\n \
             * caught` to read it.\n \
             */\n\
             export type {named}Refusal = Error & {{ fault: {named}Fault }};"
        ));
        helpers.push(format!(
            "/**\n \
             * How a one-way `{named}` method reports a message it refused. It answers \
             `Promise<void>`,\n \
             * so there is no failure arm to put a fault in and it is thrown instead — the \
             transport\n \
             * still never reached, the defect still visible.\n \
             */\n\
             function {prefix}Refused(fault: {named}Fault): {named}Refusal {{\n  \
             return Object.assign(\n    \
             new Error(`${{fault.kind}} in operation \\`${{fault.operation}}\\`: \
             ${{fault.detail}}`),\n    \
             {{ fault }},\n  \
             );\n\
             }}"
        ));
    }
    helpers
}

/// What one operation's method answers: its own result type, or nothing for a one-way operation.
fn answers(service: &str, operation: &OperationDef) -> String {
    result_name(service, operation).unwrap_or_else(|| "void".to_owned())
}

/// One method on the factory's returned object: parse the message, then reach the transport. The
/// transport is named only on the far side of the check, which is what makes "the transport was
/// never touched" something a test can observe.
///
/// A unit success normalizes `value` to `undefined` here, whatever the transport handed back.
fn method(service: &ServiceDef, operation: &OperationDef) -> String {
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let wire = &operation.wire_name;
    let call = &operation.ts_name;
    let shape = HttpShape::of(operation);
    let arguments = once("req".to_owned())
        .chain(
            shape
                .header_in
                .iter()
                .map(|header| RenameRule::CamelCase.apply_to_field(&header.parameter.to_string())),
        )
        .collect::<Vec<_>>()
        .join(", ");
    let checked = validation(service, operation);
    let (headers_build, headers) = header_in_build_stmt(&shape);
    let sending = match &operation.outcome {
        OperationOutcome::OneWay => {
            format!("      await transport.notify(\"{wire}\", validated.data, {headers});")
        }
        OperationOutcome::Reply { error, success }
            if !shape.header_out.is_empty() || !shape.error_header_out.is_empty() =>
        {
            header_tuple_answer(&named, &prefix, wire, &shape, error, success, &headers)
        }
        OperationOutcome::Reply {
            error: _error,
            success,
        } if is_unit_type(success) => format!(
            "      const {{ answered }} = await transport.request<{result}>(\"{wire}\", \
             validated.data, {headers});\n      \
             return answered.ok === true ? {{ ok: true, value: undefined }} : answered;",
            result = answers(&named, operation)
        ),
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => format!(
            "      const {{ answered }} = await transport.request<{}>(\"{wire}\", \
             validated.data, {headers});\n      \
             return answered;",
            answers(&named, operation)
        ),
    };
    format!("    async {call}({arguments}) {{\n{checked}{headers_build}{sending}\n    }},")
}

/// The parameter list the client type's member declares: the message, then one argument per
/// `header_in` binding, named and typed as the REST client's own method takes it.
fn method_params(operation: &OperationDef) -> String {
    let shape = HttpShape::of(operation);
    once(format!("req: {}", message::typename(operation)))
        .chain(shape.header_in.iter().map(|header| {
            let name = RenameRule::CamelCase.apply_to_field(&header.parameter.to_string());
            let ty = get_field_def(&name, &header.ty, "").typescript_typename();
            format!("{name}: {ty}")
        }))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The outgoing header list, one JSON-encoded entry per `header_in` binding, and the expression
/// the transport call passes: `headers`, or an empty list for an operation binding none. An
/// optional binding holding `undefined` goes out as `null`, as the Rust client writes a `None`.
fn header_in_build_stmt(shape: &HttpShape) -> (String, String) {
    if shape.header_in.is_empty() {
        return (String::new(), "[]".to_owned());
    }
    let mut stmt = String::from("      const headers: Array<[string, string]> = [];\n");
    for header in &shape.header_in {
        let name = &header.name;
        let parameter = RenameRule::CamelCase.apply_to_field(&header.parameter.to_string());
        let value = if option_inner(&header.ty).is_some() {
            format!("{parameter} ?? null")
        } else {
            parameter
        };
        let _ = writeln!(
            stmt,
            "      headers.push([\"{name}\", JSON.stringify({value})]);"
        );
    }
    (stmt, "headers".to_owned())
}

/// A reply whose success or declared error is a header tuple: the body arrives as the envelope's
/// `value` or `error`, and each header element is read back off the reply's own headers and
/// rejoined into the tuple the operation declared.
fn header_tuple_answer(
    named: &str,
    prefix: &str,
    wire: &str,
    shape: &HttpShape,
    error: &Type,
    success: &Type,
    headers: &str,
) -> String {
    let success_body = message::body_type(shape.header_out.len(), success);
    let error_body = message::body_type(shape.error_header_out.len(), error);
    let value_ty = if is_unit_type(success_body) {
        "undefined".to_owned()
    } else {
        get_field_def("value", success_body, "").typescript_typename()
    };
    let error_ty = get_field_def("error", error_body, "").typescript_typename();
    let mut stmt = format!(
        "      const {{ answered, headers: replied }} = await transport.request<\n        \
         | {{ ok: true; value: {value_ty} }}\n        \
         | {{ ok: false; error: {error_ty} | {{ isServiceFault: true; fault: {named}Fault }} }}\n      \
         >(\"{wire}\", validated.data, {headers});\n      \
         if (answered.ok) {{\n"
    );
    let value = if is_unit_type(success_body) {
        "undefined"
    } else {
        "answered.value"
    };
    let success_side = Rejoined {
        answered: ("{ ok: true, value: ", " }"),
        body: value,
        ident_prefix: "headerOut",
        indent: "        ",
        names: &shape.header_out,
        types: message::header_types(shape.header_out.len(), success),
    };
    stmt.push_str(&success_side.stmt(prefix, wire));
    stmt.push_str(
        "      }\n      \
         const error = answered.error;\n      \
         if (typeof error === \"object\" && error !== null && \"isServiceFault\" in error) {\n        \
         return { ok: false, error };\n      \
         }\n",
    );
    let error_side = Rejoined {
        answered: ("{ ok: false, error: ", " }"),
        body: "error",
        ident_prefix: "errorHeaderOut",
        indent: "      ",
        names: &shape.error_header_out,
        types: message::header_types(shape.error_header_out.len(), error),
    };
    stmt.push_str(&error_side.stmt(prefix, wire));
    stmt.truncate(stmt.trim_end().len());
    stmt
}

/// The method's own `JSDoc`: one line where the signature already says everything, a block where
/// it does not. A one-way method's `Promise<void>` cannot say what the method throws, so the
/// `JSDoc` is the only place it can be said.
fn method_doc(service: &str, operation: &OperationDef) -> String {
    let summary = method_summary(service, operation);
    let thrown = throws_clause(service, operation);
    if thrown.is_empty() {
        format!("  /** {summary} */")
    } else {
        format!("  /**\n   * {summary}\n   *\n{thrown}\n   */")
    }
}

/// A one-line summary for the method's own `JSDoc`, so a bundle reader learns what a method sends
/// without opening the trait.
fn method_summary(service: &str, operation: &OperationDef) -> String {
    let wire = &operation.wire_name;
    match &operation.outcome {
        OperationOutcome::OneWay => {
            format!("Sends `{wire}` on `{service}`, which expects no reply.")
        }
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => {
            format!("Calls `{wire}` on `{service}` and waits for the answer.")
        }
    }
}

/// The outbound check and the refusal it leads to. It runs on every method, this module being
/// emitted only where there is a schema for a message to be checked against.
fn validation(service: &ServiceDef, operation: &OperationDef) -> String {
    let named = service.ident.to_string();
    let prefix = RenameRule::CamelCase.apply_to_variant(&named);
    let wire = &operation.wire_name;
    let schema = message::schema(operation);
    let refusal = match &operation.outcome {
        OperationOutcome::OneWay => format!(
            "        throw {prefix}Refused({prefix}OutboundFault(\"{wire}\", \
             validated.error.issues));"
        ),
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => format!(
            "        return {{\n          \
             ok: false,\n          \
             error: {{\n            \
             isServiceFault: true,\n            \
             fault: {prefix}OutboundFault(\"{wire}\", validated.error.issues),\n          \
             }},\n        \
             }};"
        ),
    };
    format!(
        "      const validated = {schema}.safeParse(req);\n      \
         if (!validated.success) {{\n{refusal}\n      \
         }}\n"
    )
}

/// What a method's `JSDoc` says it throws. Only a one-way method throws at all — a replying one
/// answers its refusal into the failure arm it already has.
fn throws_clause(service: &str, operation: &OperationDef) -> String {
    match &operation.outcome {
        OperationOutcome::OneWay => format!(
            "   * @throws {{{service}Refusal}} when the message fails its own schema. The \
             operation\n   \
             * answers `Promise<void>`, so there is no failure arm to put the fault in; the \
             transport\n   \
             * is still never reached."
        ),
        OperationOutcome::Reply {
            error: _error,
            success: _success,
        } => String::new(),
    }
}

/// The reader a header-tuple reply goes through for each declared header, and the fault it answers
/// with where one is missing or will not parse as its declared type — the same failure the Rust
/// client reports for a reply header it cannot decode.
fn reply_header_helpers(named: &str, prefix: &str) -> Vec<String> {
    vec![
        format!(
            "/**\n \
             * The fault a `{named}` reply produces when a header its operation declared is \
             missing, or\n \
             * will not become the header's declared type.\n \
             */\n\
             function {prefix}ReplyHeaderFault(operation: string, name: string, detail: string): \
             {named}Fault {{\n\
             {minted}\n\
             }}",
            minted = fault::minted(
                named,
                "    detail,\n    \
                 field: name,\n    \
                 kind: \"failed-validation\",\n    \
                 operation,"
            )
        ),
        format!(
            "/**\n \
             * Reads one declared reply header off the JSON text the transport handed back, \
             checked\n \
             * against the header's own schema. An absent header reads as `null`, which only an\n \
             * optional header accepts — the value its slot in the declared tuple holds.\n \
             */\n\
             function {prefix}ReplyHeader<Parsed>(\n  \
             operation: string,\n  \
             headers: ReadonlyArray<readonly [string, string]>,\n  \
             name: string,\n  \
             schema: ZodType<Parsed>,\n\
             ): {{ ok: true; value: Parsed }} | {{ ok: false; fault: {named}Fault }} {{\n  \
             const carried = headers.find(([candidate]) => candidate.toLowerCase() === \
             name.toLowerCase())?.[1];\n  \
             let detail: string;\n  \
             try {{\n    \
             const parsed = schema.safeParse(carried === undefined ? null : \
             JSON.parse(carried));\n    \
             if (parsed.success) return {{ ok: true, value: parsed.data }};\n    \
             detail = carried === undefined\n      \
             ? \"a declared reply header was missing\"\n      \
             : parsed.error.issues.map((issue) => issue.message).join(\"; \");\n  \
             }} catch (rejected) {{\n    \
             detail = String(rejected);\n  \
             }}\n  \
             return {{ ok: false, fault: {prefix}ReplyHeaderFault(operation, name, detail) }};\n\
             }}"
        ),
    ]
}

/// The transport seam: an operation name, a payload, the headers beside it, and an answer.
/// Emitted per service for the same reason the Rust side declares one `Transport` trait per
/// service module — TypeScript has no per-service scope to keep two of them apart.
fn transport_type(service: &str) -> String {
    format!(
        "/**\n \
         * What binds a `{service}` client to a bus.\n \
         *\n \
         * The operation name travels beside the payload rather than inside it, so no message type \
         has\n \
         * to reserve a key for routing. The payload is handed over as a value rather than as \
         bytes:\n \
         * a transport merges its own fields — a correlation id, an error flag — into the object \
         before\n \
         * serializing it, and neither is reachable behind an encoded buffer.\n \
         *\n \
         * Headers travel beside the payload both ways, each value JSON-encoded: a request's \
         `header_in`\n \
         * values out, and a reply's `header_out` or `error_header_out` values back. An AMQP \
         transport\n \
         * carries them as the message's own headers, as they are.\n \
         */\n\
         export type {service}Transport = {{\n  \
         /** Sends a message no reply is expected for. */\n  \
         notify(\n    \
         operation: string,\n    \
         payload: unknown,\n    \
         headers: ReadonlyArray<readonly [string, string]>,\n  \
         ): Promise<void>;\n  \
         /** Sends a message and answers with the reply the far side wrote, and its headers. */\n  \
         request<Answered>(\n    \
         operation: string,\n    \
         payload: unknown,\n    \
         headers: ReadonlyArray<readonly [string, string]>,\n  \
         ): Promise<{{ answered: Answered; headers: ReadonlyArray<readonly [string, string]> }}>;\n\
         }};"
    )
}
