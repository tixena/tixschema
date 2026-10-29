//! A reply typed with a plain enum that names a check, handed to the emitted Kotlin clients of
//! each transport by a canned transport, compiled with `-Werror`: a reply the check refuses is
//! the fault each client answers for a reply it cannot read, with the check's sentence, and one
//! it keeps is the value. Each payload is held beside what the serde read says of it.

#![cfg(feature = "kotlin")]

use super::plain_enum_check_rows::appraisal_client_service_schema::{
    appraisal_client_service_fault_fields_kotlin, appraisal_client_service_fault_kind_kotlin,
};
use super::plain_enum_check_rows::{
    AppraisalClientServiceSchema, Verdict, appraisal_error_kotlin, verdict_kotlin,
};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

const CLIENTS: &str = r#"
fun affirmed(value: Verdict): List<String> =
    if (value == Verdict.Overturned) listOf("a verdict is upheld") else emptyList()

class Answering(private val body: String) : AppraisalClientServiceHttpTransport {
    override suspend fun send(request: AppraisalClientServiceHttpRequest): AppraisalClientServiceHttpResponse =
        AppraisalClientServiceHttpResponse(200, emptyList(), body.encodeToByteArray())
}

class Replying(private val reply: String) : AppraisalClientServiceWsSocket {
    override var onMessage: ((String) -> Unit)? = null
    override var onClose: (() -> Unit)? = null
    override fun send(text: String) {
        val id = Json.parseToJsonElement(text).jsonObject.getValue("id").jsonPrimitive.content
        onMessage?.invoke("""{"kind":"reply","id":"$id","service":"AppraisalClientService","ok":true,"value":$reply}""")
    }
    override fun close() {
        onClose?.invoke()
    }
}

fun made(answered: AppraisalClientServiceAppraiseResult): JsonElement = when (answered) {
    is AppraisalClientServiceAppraiseResult.Ok -> JsonPrimitive("read " + answered.value.name)
    is AppraisalClientServiceAppraiseResult.Declared -> JsonPrimitive("declared")
    is AppraisalClientServiceAppraiseResult.Fault -> buildJsonObject {
        put("kind", Json.encodeToJsonElement(serializer<AppraisalClientServiceFaultKind>(), answered.fault.kind))
        put("detail", answered.fault.detail)
    }
}

suspend fun overHttp(body: String): JsonElement =
    made(AppraisalClientServiceHttpClient(Answering(body)).appraise("c1"))

suspend fun overTheSocket(reply: String): JsonElement {
    val transport = AppraisalClientServiceWsTransport(Replying(reply), AppraisalClientServiceWsOptions(heartbeat = null))
    val answered = made(AppraisalClientServiceWsClient(transport).appraise("c1"))
    transport.close()
    return answered
}
"#;

#[test]
fn each_kotlin_client_refuses_a_reply_typed_with_a_plain_enum_that_names_a_check() {
    assert_eq!(
        serde_json::from_str::<Verdict>(r#""Overturned""#)
            .unwrap_err()
            .to_string(),
        "a verdict is upheld"
    );
    serde_json::from_str::<Verdict>(r#""Upheld""#).unwrap();
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        verdict_kotlin::kotlin_definition(),
        appraisal_error_kotlin::kotlin_definition(),
        appraisal_client_service_fault_fields_kotlin::kotlin_definition(),
        appraisal_client_service_fault_kind_kotlin::kotlin_definition(),
        AppraisalClientServiceSchema::kotlin_http_client(),
        AppraisalClientServiceSchema::kotlin_ws_client(),
        CLIENTS.to_owned(),
        r#"fun main() = runBlocking {
    println(buildJsonObject {
        put("httpKept", overHttp("\"Upheld\""))
        put("httpBroken", overHttp("\"Overturned\""))
        put("socketKept", overTheSocket("\"Upheld\""))
        put("socketBroken", overTheSocket("\"Overturned\""))
    }.toString())
}
"#
        .to_owned(),
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    let told: serde_json::Value = serde_json::from_str(printed.trim()).unwrap();
    let thrown = "kotlinx.serialization.SerializationException: a verdict is upheld";
    assert_eq!(
        told,
        serde_json::json!({
            "httpKept": "read Upheld",
            "httpBroken": { "kind": "undeserializable-payload", "detail": thrown },
            "socketKept": "read Upheld",
            "socketBroken": { "kind": "failed-validation", "detail": thrown }
        })
    );
}
