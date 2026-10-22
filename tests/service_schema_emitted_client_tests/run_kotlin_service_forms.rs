//! A service whose replies are a `NaiveDate` and a `char` and whose request carries a `char`
//! header, run by the emitted Kotlin clients and dispatcher, compiled with `-Werror`: each reads
//! and writes the bare text through the serializer of its form, as serde reads it, so a text of
//! another form is refused and answered as the fault each answers for what it cannot read or
//! write.

#![cfg(all(feature = "kotlin", feature = "chrono"))]

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

use self::dating_client_service_schema::{
    dating_client_service_fault_fields_kotlin, dating_client_service_fault_kind_kotlin,
};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// A transport of each kind that answers one reply, a socket a dispatcher answers over, and what
/// each client and the dispatcher made of the reply or the request.
const DRIVER: &str = r#"
class Answering(private val body: String) : DatingClientServiceHttpTransport {
    override suspend fun send(request: DatingClientServiceHttpRequest): DatingClientServiceHttpResponse =
        DatingClientServiceHttpResponse(200, emptyList(), body.encodeToByteArray())
}

class Replying(private val reply: String) : DatingClientServiceWsSocket {
    override var onMessage: ((String) -> Unit)? = null
    override var onClose: (() -> Unit)? = null
    override fun send(text: String) {
        val id = Json.parseToJsonElement(text).jsonObject.getValue("id").jsonPrimitive.content
        onMessage?.invoke("""{"kind":"reply","id":"$id","service":"DatingClientService","ok":true,"value":$reply}""")
    }
    override fun close() {
        onClose?.invoke()
    }
}

class Capturing : DatingClientServiceWsSocket {
    val answered = CompletableDeferred<String>()
    override var onMessage: ((String) -> Unit)? = null
    override var onClose: (() -> Unit)? = null
    override fun send(text: String) {
        answered.complete(text)
    }
    override fun close() {
        onClose?.invoke()
    }
}

class Writing(private val stamped: String, private val lettered: String) : DatingClientServiceHandlers {
    override suspend fun initial(req: String): DatingClientServiceInitialResult =
        DatingClientServiceInitialResult.Ok(lettered)
    override suspend fun stamp(req: String): DatingClientServiceStampResult =
        DatingClientServiceStampResult.Ok(stamped)
    override suspend fun letter(req: String, initial: String): DatingClientServiceLetterResult =
        DatingClientServiceLetterResult.Ok(req + initial)
}

val Handlers = Writing("2024-01-05", "a")

fun faulted(fault: DatingClientServiceFaultFields): String =
    "fault " + Json.encodeToJsonElement(serializer<DatingClientServiceFaultKind>(), fault.kind).jsonPrimitive.content +
        (fault.field?.let { " at $it" } ?: "") + ": " + fault.detail

fun made(answered: DatingClientServiceStampResult): String = when (answered) {
    is DatingClientServiceStampResult.Ok -> "read " + answered.value
    is DatingClientServiceStampResult.Declared -> "declared"
    is DatingClientServiceStampResult.Fault -> faulted(answered.fault)
}

suspend fun overHttp(body: String): String =
    made(DatingClientServiceHttpClient(Answering(body)).stamp("north"))

suspend fun overTheSocket(reply: String): String {
    val transport = DatingClientServiceWsTransport(Replying(reply), DatingClientServiceWsOptions(heartbeat = null))
    val answered = made(DatingClientServiceWsClient(transport).stamp("north"))
    transport.close()
    return answered
}

suspend fun dispatched(initial: String): String = replied(
    Handlers,
    """{"kind":"request","id":"1","service":"DatingClientService","operation":"letter","payload":"n","headers":{"x-initial":$initial}}""",
)

suspend fun replied(handlers: DatingClientServiceHandlers, request: String): String {
    val socket = Capturing()
    val transport = DatingClientServiceWsTransport(socket, DatingClientServiceWsOptions(heartbeat = null))
    val faults = mutableListOf<DatingClientServiceFaultFields>()
    attachDatingClientServiceWsDispatcher(transport.frames, handlers, onFault = { faults.add(it) })
    socket.onMessage?.invoke(request)
    val reply = Json.parseToJsonElement(socket.answered.await()).jsonObject
    transport.close()
    val ok = reply.getValue("ok").jsonPrimitive.boolean
    return if (ok) {
        "read " + reply.getValue("value").jsonPrimitive.content
    } else {
        faulted(Json.decodeFromJsonElement(serializer<DatingClientServiceFaultFields>(), reply.getValue("error").jsonObject.getValue("fault"))) +
            " (" + faults.size + " to onFault)"
    }
}

fun main() = runBlocking {
    println(overHttp("\"2024-01-05\""))
    println(overHttp("\"x\""))
    println(overTheSocket("\"2024-01-05\""))
    println(overTheSocket("\"x\""))
    println(dispatched("\"a\""))
    println(dispatched("\"ab\""))
    for ((operation, handlers) in listOf(
        "stamp" to Writing("2024-01-05", "a"),
        "stamp" to Writing("x", "a"),
        "initial" to Writing("2024-01-05", "a"),
        "initial" to Writing("2024-01-05", "ab"),
    )) {
        println(replied(handlers, """{"kind":"request","id":"1","service":"DatingClientService","operation":"$operation","payload":"n"}"""))
    }
}
"#;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DatingError {
    Missing,
}

/// A service whose replies are a bare `char` and a bare date, and whose other operation reads a
/// `char` header.
#[service_schema(transports = [])]
pub trait DatingClientService<Ctx> {
    #[service_schema_op(http(method = "POST", path = "/initial"))]
    async fn initial(&self, ctx: &Ctx, label: String) -> Result<char, DatingError>;

    #[service_schema_op(http(method = "POST", path = "/letter", header_in("x-initial" = initial)))]
    async fn letter(&self, ctx: &Ctx, label: String, initial: char) -> Result<String, DatingError>;

    #[service_schema_op(http(method = "POST", path = "/stamp", error_status(Missing = 404)))]
    async fn stamp(&self, ctx: &Ctx, label: String) -> Result<NaiveDate, DatingError>;
}

/// A backend answering the contract: the reply the Kotlin clients read is one it writes.
pub struct DatingBackEnd;

impl DatingClientService<()> for DatingBackEnd {
    async fn initial(&self, _ctx: &(), label: String) -> Result<char, DatingError> {
        ready(()).await;
        label.chars().next().ok_or(DatingError::Missing)
    }

    async fn letter(&self, _ctx: &(), label: String, initial: char) -> Result<String, DatingError> {
        ready(()).await;
        Ok(format!("{label}{initial}"))
    }

    async fn stamp(&self, _ctx: &(), _label: String) -> Result<NaiveDate, DatingError> {
        ready(()).await;
        NaiveDate::from_ymd_opt(2024, 1, 5).ok_or(DatingError::Missing)
    }
}

/// What `answering` answers when it is polled once, or `None` where it is not ready.
fn poll_once<Answering>(answering: Answering) -> Option<Answering::Output>
where
    Answering: Future,
{
    let mut pinned = pin!(answering);
    let mut polling = PollContext::from_waker(Waker::noop());
    match pinned.as_mut().poll(&mut polling) {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    }
}

/// The backend writes the date the Kotlin clients read, and answers the letter the dispatcher's
/// handler answers.
#[test]
fn the_backend_writes_what_the_kotlin_clients_read() {
    let stamped = poll_once(DatingBackEnd.stamp(&(), "north".to_owned()));
    assert_eq!(
        stamped.map(|answered| serde_json::to_string(&answered.unwrap()).unwrap()),
        Some("\"2024-01-05\"".to_owned()),
        "the backend no longer writes the date the Kotlin clients read"
    );
    let lettered = poll_once(DatingBackEnd.letter(&(), "n".to_owned(), 'a'));
    assert_eq!(
        lettered,
        Some(Ok("na".to_owned())),
        "the backend no longer answers the letter the dispatcher's handler answers"
    );
    let initialled = poll_once(DatingBackEnd.initial(&(), "north".to_owned()));
    assert_eq!(
        initialled.map(|answered| serde_json::to_string(&answered.unwrap()).unwrap()),
        Some("\"n\"".to_owned()),
        "the backend no longer writes the character the dispatcher's handler answers"
    );
}

/// The Kotlin HTTP client and WebSocket client read a reply of a date through the serializer of its
/// form, and answer their fault for a text of another, and the dispatcher reads a header of a
/// `char` so, and answers a fault for a text of more than one character, as serde refuses each.
/// The dispatcher writes a reply of a date and of a `char` through the same serializers, and
/// answers a handler's text of another form as a `handler-panic` fault, as the Dart one does.
#[test]
fn a_bare_char_or_date_message_is_read_in_its_one_form_by_the_kotlin_clients() {
    assert_eq!(
        serde_json::from_str::<NaiveDate>("\"x\"")
            .unwrap_err()
            .to_string(),
        "input contains invalid characters at line 1 column 3",
        "serde no longer refuses the reply as it did"
    );
    assert_eq!(
        serde_json::from_str::<char>("\"ab\"")
            .unwrap_err()
            .to_string(),
        "invalid value: string \"ab\", expected a character at line 1 column 4",
        "serde no longer refuses the header as it did"
    );
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        dating_error_kotlin::kotlin_definition(),
        dating_client_service_fault_fields_kotlin::kotlin_definition(),
        dating_client_service_fault_kind_kotlin::kotlin_definition(),
        DatingClientServiceSchema::kotlin_http_client(),
        DatingClientServiceSchema::kotlin_ws_client(),
        DRIVER.to_owned(),
    ];
    let Some(printed) = ran_kotlin(&source.join("\n\n")) else {
        return;
    };
    assert_eq!(
        printed,
        "read 2024-01-05\n\
         fault undeserializable-payload: kotlinx.serialization.SerializationException: expected \
         a date, found \"x\"\n\
         read 2024-01-05\n\
         fault failed-validation: kotlinx.serialization.SerializationException: expected a date, \
         found \"x\"\n\
         read na\n\
         fault failed-validation at x-initial: kotlinx.serialization.SerializationException: \
         expected a character, found \"ab\" (1 to onFault)\n\
         read 2024-01-05\n\
         fault handler-panic: kotlinx.serialization.SerializationException: expected a date, \
         found \"x\" (1 to onFault)\n\
         read a\n\
         fault handler-panic: kotlinx.serialization.SerializationException: expected a \
         character, found \"ab\" (1 to onFault)\n",
        "the Kotlin clients no longer read each message as above"
    );
}
