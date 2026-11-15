//! The `ws_rpc` attachment and client and the `mcp` server of `HoistService` run under a Kotlin
//! toolchain: a message declared above the service, below it and in another crate is refused with
//! one issue per broken rule, each at its own field, as Rust's own read lists them.

#![cfg(feature = "kotlin")]

use super::mcp_library_rows::request;
use super::refusal_order_service::{
    BROKEN_LIST, BROKEN_LOAD, BROKEN_MAP, HoistServiceSchema, OPERATIONS, above_item_kotlin,
    above_load_kotlin, below_item_kotlin, below_load_kotlin, below_renamed_kotlin, broken_issues,
    collection_issues, hoist_error_kotlin, hoist_service_schema, shipment_mcp_task_kotlin,
};
use super::runtime::ran_kotlin;
use refusal_order_apart::{apart_item_kotlin, apart_load_kotlin, apart_renamed_kotlin};
use serde_json::{Value, json};

const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// Hands each operation a broken message through the attachment, the `mcp` server and the client.
const DRIVER: &str = r#"
class ManualSocket(private val onSend: ((String) -> Unit)? = null) : HoistServiceWsSocket {
    val sent = java.util.Collections.synchronizedList(mutableListOf<String>())
    override var onMessage: ((String) -> Unit)? = null
    override var onClose: (() -> Unit)? = null
    override fun send(text: String) {
        sent.add(text)
        onSend?.invoke(text)
    }
    override fun close() {
        onClose?.invoke()
    }
    fun deliver(text: String) {
        onMessage?.invoke(text)
    }
}

object Handlers : HoistServiceHandlers {
    override suspend fun above(req: AboveLoad): HoistServiceAboveResult = HoistServiceAboveResult.Ok(req)
    override suspend fun apart(req: ApartLoad): HoistServiceApartResult = HoistServiceApartResult.Ok(req)
    override suspend fun below(req: BelowLoad): HoistServiceBelowResult = HoistServiceBelowResult.Ok(req)
    override suspend fun keyed(req: Map<String, AboveItem>): HoistServiceKeyedResult = HoistServiceKeyedResult.Ok(req.size.toUInt())
    override suspend fun listed(req: List<AboveItem>): HoistServiceListedResult = HoistServiceListedResult.Ok(req.size.toUInt())
    override suspend fun moved(req: PublishedBelow): HoistServiceMovedResult = HoistServiceMovedResult.Ok(req)
    override suspend fun shifted(req: PublishedApart): HoistServiceShiftedResult = HoistServiceShiftedResult.Ok(req)
    override suspend fun tasked(req: ShipmentMcpTask): HoistServiceTaskedResult = HoistServiceTaskedResult.Ok(req)
}

fun listed(fault: HoistServiceFaultFields): JsonElement = buildJsonObject {
    put("kind", fault.kind.name)
    put("issues", JsonArray((fault.issues ?: emptyList()).map { issue ->
        buildJsonObject {
            put("detail", issue.detail)
            put("field", issue.field)
        }
    }))
}

fun main() = runBlocking {
    val operations = listOf("above", "apart", "below", "moved", "shifted", "tasked")

    val socket = ManualSocket()
    val transport = HoistServiceWsTransport(socket, HoistServiceWsOptions(heartbeat = null))
    attachHoistServiceWsDispatcher(transport.frames, Handlers, onFault = { })
    for (operation in operations) {
        socket.deliver("""{"kind":"request","id":"$operation","service":"HoistService","operation":"$operation","payload":$BROKEN}""")
    }
    val deadline = System.currentTimeMillis() + 5000
    while (socket.sent.size < 6 && System.currentTimeMillis() < deadline) delay(5)
    val attached = socket.sent.toList()

    val collectionsSocket = ManualSocket()
    val collectionsTransport = HoistServiceWsTransport(collectionsSocket, HoistServiceWsOptions(heartbeat = null))
    attachHoistServiceWsDispatcher(collectionsTransport.frames, Handlers, onFault = { })
    collectionsSocket.deliver("""{"kind":"request","id":"listed","service":"HoistService","operation":"listed","payload":$BROKEN_LIST}""")
    collectionsSocket.deliver("""{"kind":"request","id":"keyed","service":"HoistService","operation":"keyed","payload":$BROKEN_MAP}""")
    val collectionsDeadline = System.currentTimeMillis() + 5000
    while (collectionsSocket.sent.size < 2 && System.currentTimeMillis() < collectionsDeadline) delay(5)
    val collections = collectionsSocket.sent.toList()

    val dispatch = createHoistServiceMcpDispatcher { _ -> Handlers }
    val inFlight = HoistServiceMcpInFlight()
    val nowhere: HoistServiceMcpWriter = { }
    val served = mutableMapOf<String, JsonElement>()
    for ((name, text) in Json.parseToJsonElement(MCP_REQUESTS).jsonObject) {
        val answer = dispatch(text.jsonPrimitive.content, nowhere, inFlight)
        served[name] = if (answer == null) JsonNull else JsonPrimitive(answer)
    }

    val read = mutableMapOf<String, JsonElement>()
    for (operation in operations) {
        lateinit var clientSocket: ManualSocket
        clientSocket = ManualSocket(onSend = { text ->
            val id = Json.parseToJsonElement(text).jsonObject["id"]!!.jsonPrimitive.content
            clientSocket.deliver("""{"kind":"reply","id":"$id","service":"HoistService","ok":true,"value":$BROKEN}""")
        })
        val client = HoistServiceWsClient(HoistServiceWsTransport(clientSocket, HoistServiceWsOptions(heartbeat = null)))
        val fault = when (operation) {
            "above" -> (client.above(AboveLoad.of(emptyList(), "a", 1u)) as? HoistServiceAboveResult.Fault)?.fault
            "apart" -> (client.apart(ApartLoad.of(emptyList(), "a", 1u)) as? HoistServiceApartResult.Fault)?.fault
            "below" -> (client.below(BelowLoad.of(emptyList(), "a", 1u)) as? HoistServiceBelowResult.Fault)?.fault
            "moved" -> (client.moved(PublishedBelow.of(emptyList(), "a", 1u)) as? HoistServiceMovedResult.Fault)?.fault
            "shifted" -> (client.shifted(PublishedApart.of(emptyList(), "a", 1u)) as? HoistServiceShiftedResult.Fault)?.fault
            else -> (client.tasked(ShipmentMcpTask.of(emptyList(), "a", 1u)) as? HoistServiceTaskedResult.Fault)?.fault
        }
        read[operation] = if (fault == null) JsonNull else listed(fault)
    }

    println(buildJsonObject {
        put("attached", JsonArray(attached.map { JsonPrimitive(it) }))
        put("collections", JsonArray(collections.map { JsonPrimitive(it) }))
        put("served", JsonObject(served))
        put("read", JsonObject(read))
    }.toString())
}
"#;

/// Every definition, the data the driver reads, and the driver.
fn module() -> String {
    let requests: serde_json::Map<String, Value> = OPERATIONS
        .iter()
        .zip(1_u64..)
        .map(|(operation, id)| {
            let broken: Value = serde_json::from_str(BROKEN_LOAD).unwrap();
            let asked = request(
                &json!(id),
                "tools/call",
                &json!({ "name": operation, "arguments": broken }),
            );
            ((*operation).to_owned(), Value::String(asked.to_string()))
        })
        .collect();
    let literal = Value::Object(requests)
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$");
    [
        KOTLIN_IMPORTS.to_owned(),
        above_item_kotlin::kotlin_definition(),
        above_load_kotlin::kotlin_definition(),
        apart_item_kotlin::kotlin_definition(),
        apart_load_kotlin::kotlin_definition(),
        below_item_kotlin::kotlin_definition(),
        below_load_kotlin::kotlin_definition(),
        below_renamed_kotlin::kotlin_definition(),
        apart_renamed_kotlin::kotlin_definition(),
        shipment_mcp_task_kotlin::kotlin_definition(),
        hoist_error_kotlin::kotlin_definition(),
        hoist_service_schema::hoist_service_fault_fields_kotlin::kotlin_definition(),
        hoist_service_schema::hoist_service_fault_kind_kotlin::kotlin_definition(),
        HoistServiceSchema::kotlin_http_client(),
        HoistServiceSchema::kotlin_ws_client(),
        HoistServiceSchema::kotlin_mcp_service(),
        format!("const val BROKEN = \"\"\"{BROKEN_LOAD}\"\"\""),
        format!("const val BROKEN_LIST = \"\"\"{BROKEN_LIST}\"\"\""),
        format!("const val BROKEN_MAP = \"\"\"{BROKEN_MAP}\"\"\""),
        format!("const val MCP_REQUESTS = \"{literal}\""),
        DRIVER.to_owned(),
    ]
    .join("\n\n")
}

/// The issues Rust lists, as a fault writes them.
fn expected() -> Value {
    Value::Array(
        broken_issues()
            .into_iter()
            .map(|(field, detail)| json!({ "detail": detail, "field": field }))
            .collect(),
    )
}

#[test]
fn a_message_declared_above_below_or_in_another_crate_is_listed_issue_by_issue() {
    let Some(wrote) = ran_kotlin(&module()) else {
        return;
    };
    let told: Value = serde_json::from_str(wrote.trim()).unwrap();
    for operation in OPERATIONS {
        let reply: Value = told["attached"]
            .as_array()
            .unwrap()
            .iter()
            .map(|frame| serde_json::from_str::<Value>(frame.as_str().unwrap()).unwrap())
            .find(|reply| reply["id"] == operation)
            .unwrap();
        assert_eq!(
            reply["error"]["fault"]["kind"], "failed-validation",
            "{told}"
        );
        assert_eq!(
            reply["error"]["fault"]["issues"],
            expected(),
            "{operation}: {told}"
        );

        let answer: Value =
            serde_json::from_str(told["served"][operation].as_str().unwrap()).unwrap();
        let text = answer["result"]["content"][0]["text"].as_str().unwrap();
        let carried: Value = serde_json::from_str(text).unwrap();
        assert_eq!(carried["fault"]["kind"], "failed-validation", "{told}");
        assert_eq!(
            carried["fault"]["issues"],
            expected(),
            "{operation}: {told}"
        );

        assert_eq!(
            told["read"][operation]["kind"], "FailedValidation",
            "{told}"
        );
        assert_eq!(
            told["read"][operation]["issues"],
            expected(),
            "{operation}: {told}"
        );
    }
}

#[test]
fn a_list_and_a_map_message_name_the_step_to_the_broken_item_as_rust_does() {
    let Some(wrote) = ran_kotlin(&module()) else {
        return;
    };
    let told: Value = serde_json::from_str(wrote.trim()).unwrap();
    let replies: Vec<Value> = told["collections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|frame| serde_json::from_str(frame.as_str().unwrap()).unwrap())
        .collect();
    for (operation, (field, detail)) in ["listed", "keyed"].into_iter().zip(collection_issues()) {
        let reply = replies
            .iter()
            .find(|reply| reply["id"] == operation)
            .unwrap();
        assert_eq!(
            reply["error"]["fault"]["kind"], "failed-validation",
            "{told}"
        );
        assert_eq!(
            reply["error"]["fault"]["issues"],
            json!([{ "detail": detail, "field": field }]),
            "{told}"
        );
    }
}
