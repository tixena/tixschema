//! The emitted TypeScript `http_rest` client, run by node beside the real `zod`, handed each value
//! of [`WIRE_FORMS`](super::wire_forms_models::WIRE_FORMS) in a path, a query, a header and a part:
//! each call prints the request the transport was handed, or `refused` where the client refused
//! the value before the transport was reached, and the Rust dispatcher reads each request.

#![cfg(all(
    feature = "chrono",
    any(feature = "dart", feature = "kotlin", feature = "swift")
))]

use core::fmt::Write as _;

use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
use super::text_position_rows::{
    Call, Position, TextPlacingClientServiceSchema, TextPlacingError, calls, held_to_the_table,
    spelled,
};
use super::wire_forms_models::{ConveyedSlot, Sends, WireFormRow};

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

const REQUIRED_PACKAGES: &[&str] = &["zod"];

/// A transport that records the request it is handed and answers the empty text, and the one
/// line each call prints: that request, or `refused` where none reached the transport.
const RECORDER: &str = r#"
process.env.TZ = "UTC";
let recorded;
const client = createTextPlacingClientServiceHttpClient({
  async send(request) {
    recorded = {
      method: request.method,
      path: request.path,
      query: request.query,
      headers: request.headers,
      body: new TextDecoder().decode(request.body),
      parts: request.parts,
    };
    return { status: 200, headers: [], body: new TextEncoder().encode('""') };
  },
});

async function call(made) {
  recorded = undefined;
  try {
    await made();
  } catch (_) {}
  console.log(JSON.stringify(recorded ?? "refused"));
}
"#;

/// What the TypeScript client sends of `row` at a text position: its column, which states what it
/// sends of a model that holds the value, but for a `DateTime<Tz>` the column refuses where the
/// model's key refuses it. At a text position the value alone is read, as the JavaScript `Date`
/// the Zod schema coerces it to, and written as `toISOString` writes it: a text of no zone read in
/// the run's own zone, `UTC`.
fn typescript_at_a_text_position(row: &WireFormRow) -> Sends {
    match (row.held_by, row.handed) {
        ("ConveyedInstant", r#""2026-10-22T13:07:11.032917""#) => {
            Sends::As(r#""2026-10-22T13:07:11.032Z""#)
        }
        ("ConveyedInstant", r#""2024-01-05T10:20Z""#) => Sends::As(r#""2024-01-05T10:20:00.000Z""#),
        _ => row.typescript,
    }
}

/// The TypeScript value of `json` handed at `position`: itself, but the float JSON writes none of,
/// and an `i64` a member of the message holds as a `bigint`, which a header is not.
fn ts_value(held_by: &str, json: &str, position: Position) -> String {
    if json == "NaN" {
        "NaN".to_owned()
    } else if held_by == "ConveyedI64" && position != Position::Header {
        format!("{json}n")
    } else {
        json.to_owned()
    }
}

/// The statement that makes `call` through the client.
fn statement(call: &Call) -> String {
    let arguments = spelled(
        call,
        &|held_by, json| ts_value(held_by, json, call.position),
        "undefined",
    );
    let operation = call.operation();
    let made = if call.position == Position::Header {
        format!("client.{operation}(\"n\", {})", arguments.join(", "))
    } else {
        let members = call
            .arguments()
            .iter()
            .zip(&arguments)
            .filter(|(_, value)| *value != "undefined")
            .map(|((named, _, _), value)| format!("{named}: {value}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("client.{operation}({{ {members} }})")
    };
    format!("await call(() => {made});")
}

/// The types the client names, the service and the client, beside the real `zod`.
fn emitted() -> String {
    [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        ConveyedSlot::ts_definition(),
        ConveyedSlot::zod_schema(),
        TextPlacingError::ts_definition(),
        TextPlacingError::zod_schema(),
        TextPlacingClientServiceSchema::ts_definition(),
        TextPlacingClientServiceSchema::ts_http_client(),
    ]
    .join("\n\n")
}

/// Each value the table's TypeScript column hands the client is written in a path, a query, a
/// header and a part in the form the table states, and refused where the table states it.
#[test]
fn each_value_is_written_as_text_by_the_typescript_client_in_the_form_the_dispatcher_reads() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "the TypeScript client's text positions");
        return;
    };
    let mut driver = String::from(RECORDER);
    for call in calls(typescript_at_a_text_position) {
        let _ = writeln!(driver, "{}", statement(&call));
    }
    let module = format!("{}\n\n{driver}", emitted());
    let Some(printed) = ran_with_modules(
        "text-positions",
        RUNTIME_VAR,
        "node",
        "positions.mts",
        &module,
        &modules,
    ) else {
        return;
    };
    held_to_the_table(typescript_at_a_text_position, &printed);
}
