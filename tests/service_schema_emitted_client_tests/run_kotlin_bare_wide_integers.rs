//! The emitted Kotlin `http_rest` client, compiled with `-Werror`, handed a bare `i64` as a
//! message, a header and a list, and answered a reply, a header beside it and a declared error of
//! one: each call prints the request the transport was handed and what the client answered. The
//! `ws_rpc` client and its dispatcher compile beside it.

#![cfg(all(feature = "kotlin", feature = "serde"))]

use core::fmt::Write as _;

use super::bare_wide_integer_rows::{BARE_EXCHANGES, bare_printed};
use super::run_node_wide_integers::tallying_bare_service_schema::{
    tallying_bare_service_fault_fields_kotlin, tallying_bare_service_fault_kind_kotlin,
};
use super::run_node_wide_integers::{TallyingBareServiceSchema, tally_error_kotlin};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// A transport that prints each request it is handed and answers the next response of
/// `responses`, and what prints each answer.
const RECORDER: &str = r#"
class Recorder(private val responses: List<TallyingBareServiceHttpResponse>) : TallyingBareServiceHttpTransport {
    private var at = 0
    override suspend fun send(request: TallyingBareServiceHttpRequest): TallyingBareServiceHttpResponse {
        val headed = request.headers.joinToString(",") { (name, value) -> "$name=$value" }
        println("${request.method} ${request.path} $headed ${request.body.decodeToString()}")
        val answered = responses[at]
        at += 1
        return answered
    }
}

fun shownShift(result: TallyingBareServiceShiftResult): String = when (result) {
    is TallyingBareServiceShiftResult.Ok -> "ok ${result.value.value} ${result.value.headerOut0}"
    is TallyingBareServiceShiftResult.Declared -> "declared ${result.error}"
    is TallyingBareServiceShiftResult.Fault -> "fault ${Json.encodeToJsonElement(result.fault.kind).jsonPrimitive.content}"
}

fun shownTotal(result: TallyingBareServiceTotalResult): String = when (result) {
    is TallyingBareServiceTotalResult.Ok -> "ok ${result.value}"
    is TallyingBareServiceTotalResult.Declared -> "declared ${result.error}"
    is TallyingBareServiceTotalResult.Fault -> "fault ${Json.encodeToJsonElement(result.fault.kind).jsonPrimitive.content}"
}
"#;

/// Each call hands its transport the request the Rust client sends of it, and reads each response
/// the Rust dispatcher answers back into the integers.
#[test]
fn a_bare_i64_travels_as_a_number_through_the_kotlin_client() {
    let mut responses = String::new();
    for exchange in &BARE_EXCHANGES {
        let headers: Vec<String> = exchange
            .headers
            .iter()
            .map(|(name, value)| format!("\"{name}\" to \"{value}\""))
            .collect();
        let _ = write!(
            responses,
            "TallyingBareServiceHttpResponse({}, listOf<Pair<String, String>>({}), \"\"\"{}\"\"\".encodeToByteArray()), ",
            exchange.status,
            headers.join(", "),
            exchange.body
        );
    }
    let main = format!(
        "fun main() = runBlocking {{\n    \
         val client = TallyingBareServiceHttpClient(Recorder(listOf({responses})))\n    \
         println(shownShift(client.shift(9223372036854775806L, 1L)))\n    \
         println(shownShift(client.shift(4611686018427387903L, null)))\n    \
         println(shownTotal(client.total(listOf(9223372036854775807L, 1L))))\n    \
         client.record(Long.MIN_VALUE)\n    \
         println(\"sent\")\n    \
         println(shownTotal(client.total(listOf(1L))))\n\
         }}\n"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        tally_error_kotlin::kotlin_definition(),
        tallying_bare_service_fault_fields_kotlin::kotlin_definition(),
        tallying_bare_service_fault_kind_kotlin::kotlin_definition(),
        TallyingBareServiceSchema::kotlin_http_client(),
        TallyingBareServiceSchema::kotlin_ws_client(),
        RECORDER.to_owned(),
        main,
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(
        printed,
        bare_printed(),
        "the Kotlin client is not the Rust one"
    );
}
