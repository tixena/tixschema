//! The emitted Kotlin `http_rest` client, compiled with `-Werror`, handed each value of
//! [`WIRE_FORMS`](super::wire_forms_models::WIRE_FORMS) in a path, a query, a header and a part:
//! each call prints the request the transport was handed, or `refused` where the client refused
//! the value before the transport was reached, and the Rust dispatcher reads each request.

#![cfg(all(feature = "kotlin", feature = "chrono"))]

use core::fmt::Write as _;

use super::run_kotlin_wire_forms::kotlin_value;
use super::runtime::ran_kotlin;
use super::text_position_rows::text_placing_client_service_schema::{
    text_placing_client_service_fault_fields_kotlin, text_placing_client_service_fault_kind_kotlin,
};
use super::text_position_rows::{
    Call, Position, TextPlacingClientServiceSchema, calls, held_to_the_table, messages_kotlin,
    spelled, text_placing_error_kotlin,
};
use super::wire_forms_models::conveyed_slot_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// A transport that records the request it is handed and answers the empty text, and the one
/// line each call prints: that request, or `refused` where none reached the transport.
const RECORDER: &str = r#"
class Recorder : TextPlacingClientServiceHttpTransport {
    var recorded: TextPlacingClientServiceHttpRequest? = null
    override suspend fun send(request: TextPlacingClientServiceHttpRequest): TextPlacingClientServiceHttpResponse {
        recorded = request
        return TextPlacingClientServiceHttpResponse(200, emptyList(), "\"\"".encodeToByteArray())
    }
}

suspend fun call(recorder: Recorder, made: suspend () -> Unit) {
    recorder.recorded = null
    try {
        made()
    } catch (refusal: Exception) {
    }
    val request = recorder.recorded
    if (request == null) {
        println("\"refused\"")
        return
    }
    val sent = buildJsonObject {
        put("method", request.method)
        put("path", request.path)
        put("query", request.query)
        put("headers", buildJsonArray {
            for ((name, value) in request.headers) {
                add(buildJsonArray { add(name); add(value) })
            }
        })
        put("body", request.body.decodeToString())
        put("parts", buildJsonArray {
            for ((name, value) in request.parts) {
                add(buildJsonArray { add(name); add(value.toString()) })
            }
        })
    }
    println(sent.toString())
}
"#;

/// The statement that makes `call` through the client.
fn statement(call: &Call) -> String {
    let arguments = spelled(call, &kotlin_value, "null");
    let named = call
        .arguments()
        .iter()
        .zip(&arguments)
        .map(|((argument, _, _), value)| format!("{argument} = {value}"))
        .collect::<Vec<_>>()
        .join(", ");
    let operation = call.operation();
    let made = if call.position == Position::Header {
        format!("client.{operation}(\"n\", {named})")
    } else {
        format!("client.{operation}({}({named}))", call.message())
    };
    format!("    call(recorder) {{ {made} }}")
}

/// Each value the table's Kotlin column hands the client is written in a path, a query, a header
/// and a part in the form the table states, and refused where the table states it.
#[test]
fn each_value_is_written_as_text_by_the_kotlin_client_in_the_form_the_dispatcher_reads() {
    let mut main = String::from(
        "fun main() = runBlocking {\n    val recorder = Recorder()\n    \
         val client = TextPlacingClientServiceHttpClient(recorder)\n",
    );
    for call in calls(|row| row.kotlin) {
        let _ = writeln!(main, "{}", statement(&call));
    }
    main.push_str("}\n");
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        conveyed_slot_kotlin::kotlin_definition(),
        text_placing_error_kotlin::kotlin_definition(),
        messages_kotlin(),
        text_placing_client_service_fault_fields_kotlin::kotlin_definition(),
        text_placing_client_service_fault_kind_kotlin::kotlin_definition(),
        TextPlacingClientServiceSchema::kotlin_http_client(),
        RECORDER.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    held_to_the_table(|row| row.kotlin, &printed);
}
