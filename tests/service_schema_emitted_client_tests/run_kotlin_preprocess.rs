//! `preprocess` in the emitted Kotlin types, compiled with `-Werror` and run: a type's functions
//! are called on its payload and a member's on the member's own, ahead of the read, and the
//! bounds and the checks judge what they answered. A row is the value written back, held to what
//! the Rust read answers of the same payload. The service clients read a reply the same way.

#![cfg(feature = "kotlin")]

use alloc::collections::BTreeMap;
use core::cell::RefCell;
use core::fmt::Write as _;
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::{model_schema, service_schema};

use self::packet_client_service_schema::{
    packet_client_service_fault_fields_kotlin, packet_client_service_fault_kind_kotlin,
};
use super::runtime::{ran_kotlin, refused_by_kotlinc};

const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// The two functions of the documents' sample, written by hand as a consumer writes them.
const SAMPLE_FUNCTIONS: &str = r#"
fun trimmed(payload: JsonElement): JsonElement =
    if (payload is JsonPrimitive && payload.isString) JsonPrimitive(payload.content.trim()) else payload

fun withoutFragile(payload: JsonElement): JsonElement =
    if (payload is JsonObject && "fragileItemCodes" !in payload) {
        JsonObject(payload + ("fragileItemCodes" to JsonArray(emptyList())))
    } else {
        payload
    }
"#;

/// The functions the other types name, and the one check.
const SHAPE_FUNCTIONS: &str = r#"
val calls = mutableListOf<String>()

fun lowered(payload: JsonElement): JsonElement =
    if (payload is JsonPrimitive && payload.isString) JsonPrimitive(payload.content.lowercase()) else payload

fun noted(payload: JsonElement): JsonElement {
    calls.add(payload.toString())
    return payload
}

fun orUnnamed(payload: JsonElement): JsonElement =
    if (payload is JsonNull) JsonPrimitive("unnamed") else payload

fun textual(payload: JsonElement): JsonElement =
    if (payload is JsonPrimitive && !payload.isString && payload.longOrNull != null) {
        JsonPrimitive(payload.content)
    } else {
        payload
    }

fun trimmedPlate(payload: JsonElement): JsonElement {
    val plate = (payload as? JsonObject)?.get("plate")
    return if (payload is JsonObject && plate is JsonPrimitive && plate.isString) {
        JsonObject(payload + ("plate" to JsonPrimitive(plate.content.trim())))
    } else {
        payload
    }
}

fun unwrapped(payload: JsonElement): JsonElement =
    if (payload is JsonObject && "payload" in payload) payload.getValue("payload") else payload

fun withItsKeys(payload: JsonElement): JsonElement =
    if (payload is JsonObject) {
        JsonObject(mapOf("keys" to JsonArray(payload.keys.sorted().map { JsonPrimitive(it) })) + payload)
    } else {
        payload
    }

fun withinLimit(value: Scaled): List<String> =
    if (value.grams > 1000u) listOf("'grams': ${value.grams} is over the limit of 1000") else emptyList()
"#;

/// What the driver prints a row with: the value a read handed back, written back, or each message
/// of what the read threw. `thrown` names what was thrown, and the first line of what the
/// serialization library says of it.
const TOLD: &str = r#"
fun told(name: String, read: () -> String) {
    try {
        println("$name: read ${read()}")
    } catch (refused: SerializationException) {
        println("$name: refused")
        (refused.message ?: "").split("; ").forEach { println("    $it") }
    }
}

fun thrown(name: String, read: () -> Any?) {
    try {
        println("$name: read ${read()}")
    } catch (refused: SerializationException) {
        println("$name: ${refused::class.simpleName}: ${(refused.message ?: "").lines().first()}")
    } catch (refused: Throwable) {
        println("$name: ${refused::class.simpleName}")
    }
}
"#;

/// What every side prints for the sample of the documents: the Rust read, the Zod schema and the
/// Kotlin read alike.
const SAMPLE: &str = "\
row 1: read {\"fragileItemCodes\":[\"x\"],\"itemCodeList\":[\"x\"],\"label\":\"A\"}
row 2: refused
    'label': too short: minimum length is 1, got 0
row 3: read {\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"A\"}
row 5: read {\"packet\":{\"fragileItemCodes\":[\"x\"],\"itemCodeList\":[\"x\"],\"label\":\"A\"}}
a number where a text is read: read {\"label\":\"5\"}
";

/// What the Rust read and the Kotlin read both print where another type holds a preprocessed
/// type, and for each shape that names a function on itself.
const POSITIONS: &str = "\
a field: read {\"packet\":{\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"A\"}}
a list, a map and an Option: read {\"by_dock\":{\"north\":{\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"A\"}},\"list\":[{\"fragileItemCodes\":[],\"itemCodeList\":[],\"label\":\"B\"}],\"spare\":{\"fragileItemCodes\":[],\"itemCodeList\":[],\"label\":\"C\"}}
a flattened type: read {\"dock\":\"north\",\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"A\"}
the keys a flattened type is handed: read {\"dock\":\"north\",\"keys\":[\"tag\"],\"tag\":\"t\"}
a variant beside its tag: read {\"kind\":\"Held\",\"detail\":{\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"A\"}}
a variant under its name: read {\"Held\":{\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"A\"}}
an untagged variant: read {\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"A\"}
a flattened choice: read {\"dock\":\"north\",\"plate\":\"AB 1\"}
a type and a brand with a parameter, held, and a type behind one: read {\"bin\":{\"codes\":[\"x\"],\"label\":\"A\"},\"bins\":[{\"codes\":[7],\"label\":\"B\"}],\"of_packets\":{\"codes\":[{\"fragileItemCodes\":[],\"itemCodeList\":[],\"label\":\"C\"}],\"label\":\"D\"},\"stamped\":\"s\"}
a unit struct: read {}
a tuple struct: read [[\"x\"],2]
a brand: read \"A\"
a brand with a parameter: read \"A\"
a plain enum: read \"Express\"
a plain enum, held: read {\"by_tier\":{\"Express\":1},\"tier\":\"Standard\",\"tiers\":[\"Express\"]}
an internally tagged enum: read {\"kind\":\"Air\",\"flight\":\"TX 1\"}
an externally tagged enum: read {\"Rail\":\"TX 1\"}
an adjacently tagged enum: read {\"kind\":\"Left\",\"detail\":\"door\"}
an untagged enum: read 7
a type with a parameter, at a text: read {\"codes\":[\"x\"],\"label\":\"A\"}
a type with a parameter, at a number: read {\"codes\":[7],\"label\":\"A\"}
two functions, in the order written: read {\"fragileItemCodes\":[],\"label\":\"A\"}
a check, after the function: refused
    'grams': 2000 is over the limit of 1000
a null the payload holds: read {\"label\":\"unnamed\"}
";

/// What both reads print for a member that names a function, in each place a member is read.
const MEMBERS: &str = "\
two functions on a field, a nullable, a list of models and a tuple: read {\"code\":\"ab\",\"note\":\"n\",\"packets\":[{\"fragileItemCodes\":[],\"itemCodeList\":[],\"label\":\"A\"}],\"pair\":[\"p\",2]}
a nullable the payload leaves out: read {\"code\":\"ab\",\"packets\":[],\"pair\":[\"p\",2]}
a member that holds a model: read {\"packet\":{\"fragileItemCodes\":[],\"itemCodeList\":[],\"label\":\"A\"}}
a member that holds a model, judged by its holder: refused
    'packet.label': too short: minimum length is 1, got 0
a member beside a flattened one: read {\"dock\":\"north\",\"note\":\"n\",\"fragileItemCodes\":[],\"itemCodeList\":[],\"label\":\"A\"}
a flattened member that names a function: read {\"dock\":\"north\",\"keys\":[\"other\",\"tag\"],\"tag\":\"t\"}
a flattened map that names a function: read {\"keys\":[\"other\",\"tag\"],\"other\":[\"1\"],\"tag\":[\"t\"]}
a flattened type after one that took its keys: read {\"dock\":\"north\",\"strap\":\"s\",\"keys\":[\"tag\"],\"tag\":\"t\"}
a member that reaches a parameter, at a text: read {\"codes\":[\"x\"],\"held\":\"h\",\"label\":\"A\",\"spare\":\"s\"}
a member that reaches a parameter, at a number: read {\"codes\":[7],\"held\":8,\"label\":\"A\"}
a member of a variant under a tag: read {\"kind\":\"Air\",\"flight\":\"TX 1\"}
a member of a variant beside its tag: read {\"kind\":\"Boxed\",\"detail\":{\"label\":\"A\"}}
a slot of a variant beside its tag: read {\"kind\":\"Lone\",\"detail\":\"A\"}
a member of a variant under its name: read {\"Boxed\":{\"label\":\"A\"}}
a slot of a variant under its name: read {\"Lone\":\"A\"}
a member of an untagged variant: read {\"code\":\"ab\"}
a slot of an untagged variant, within its bound: read \"a\"
a slot of an untagged variant, past its bound: read \"  abc  \"
a slot of a tuple struct: read [\"A\",2]
a member of a variant that reaches a parameter: read {\"kind\":\"Many\",\"detail\":{\"codes\":[\"x\"]}}
a slot of a variant that reaches a parameter: read {\"kind\":\"One\",\"detail\":\"x\"}
";

/// What the Kotlin read prints where its words are its own: a key the payload leaves out, a
/// payload the read refuses below a function, the unjudged read and a value written.
const APART_FROM_RUST: &str = "\
a key the payload leaves out: MissingFieldException: Field 'label' is required for type with serial name 'Packet', but it was missing at path: $
a number left as it is: JsonDecodingException: String literal for value of key 'primitive' should be quoted at path: $.primitive
a list left as it is: IndexOutOfBoundsException
an item left as it is, below a type that names a function: JsonDecodingException: String literal for value of key '0' should be quoted at path: $.0
the unjudged read: read Packet(fragileItemCodes=[], itemCodeList=[x], label=)
a value built in code, written: read {\"fragileItemCodes\":[],\"itemCodeList\":[\"x\"],\"label\":\"  A  \"}
the calls of a nullable's function: read [null, \"a\"]
";

/// A transport of each kind that answers one reply, and what a client made of it.
const CLIENTS: &str = r#"
class Answering(private val body: String) : PacketClientServiceHttpTransport {
    override suspend fun send(request: PacketClientServiceHttpRequest): PacketClientServiceHttpResponse =
        PacketClientServiceHttpResponse(200, emptyList(), body.encodeToByteArray())
}

class Replying(private val reply: String) : PacketClientServiceWsSocket {
    override var onMessage: ((String) -> Unit)? = null
    override var onClose: (() -> Unit)? = null
    override fun send(text: String) {
        val id = Json.parseToJsonElement(text).jsonObject.getValue("id").jsonPrimitive.content
        onMessage?.invoke("""{"kind":"reply","id":"$id","service":"PacketClientService","ok":true,"value":$reply}""")
    }
    override fun close() {
        onClose?.invoke()
    }
}

fun made(answered: PacketClientServiceSentResult): JsonElement = when (answered) {
    is PacketClientServiceSentResult.Ok -> buildJsonObject {
        put("label", answered.value.label)
        put("fragile", answered.value.fragileItemCodes.size)
    }
    is PacketClientServiceSentResult.Declared -> JsonPrimitive("declared")
    is PacketClientServiceSentResult.Fault -> buildJsonObject {
        put("kind", Json.encodeToJsonElement(serializer<PacketClientServiceFaultKind>(), answered.fault.kind))
        put("detail", answered.fault.detail)
    }
}

suspend fun overHttp(body: String): JsonElement =
    made(PacketClientServiceHttpClient(Answering(body)).sent(PacketRequest("north")))

suspend fun overTheSocket(reply: String): JsonElement {
    val transport = PacketClientServiceWsTransport(Replying(reply), PacketClientServiceWsOptions(heartbeat = null))
    val answered = made(PacketClientServiceWsClient(transport).sent(PacketRequest("north")))
    transport.close()
    return answered
}
"#;

/// One row of a run: the Kotlin statement that reads a payload, and what the Rust read prints of
/// the same payload.
struct Reading {
    kotlin: String,
    told: String,
}

/// The parcel of the documents' sample, under a name of this file's own: a function of the type,
/// and a function and a bound on its label.
#[model_schema(preprocess = ["withoutFragile"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Packet {
    pub fragile_item_codes: Vec<String>,
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1, preprocess = ["trimmed"])]
    pub label: String,
}

/// The type that holds the packet under a key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lading {
    pub packet: Packet,
}

/// A packet in a list, as a map's value and in an `Option`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Locker {
    pub by_dock: BTreeMap<String, Packet>,
    pub list: Vec<Packet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<Packet>,
}

/// A packet flattened into the object of its holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Encased {
    pub dock: String,
    #[serde(flatten)]
    pub packet: Packet,
}

/// A type whose function writes, under `keys`, the keys of the object it was handed.
#[model_schema(preprocess = ["withItsKeys"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Itemized {
    pub keys: Vec<String>,
    pub tag: String,
}

/// The type above flattened into its holder: `keys` is what its function was handed.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Fastened {
    pub dock: String,
    #[serde(flatten)]
    pub keyed: Itemized,
}

/// A choice of two objects, whose function trims the plate of the one that holds one.
#[model_schema(preprocess = ["trimmedPlate"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Hauler {
    Rail { wagon: String },
    Road { plate: String },
}

/// The choice above flattened into the object of its holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Directed {
    pub dock: String,
    #[serde(flatten)]
    pub via: Hauler,
}

/// An adjacently tagged enum whose variant holds a packet.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum Entrusted {
    Held(Packet),
    Lost,
}

/// An externally tagged enum whose variant holds a packet.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Niche {
    Empty,
    Held(Packet),
}

/// An untagged enum that holds a packet.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OneOf {
    Code(String),
    Packet(Packet),
}

/// An internally tagged enum whose variant holds a packet: the emitted type reads the packet
/// under a `value` key, where serde writes its keys beside the tag, so no row runs it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Handoff {
    Held(Packet),
    Lost,
}

/// A unit struct.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShutGate;

/// A tuple struct.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pile(pub Vec<String>, pub u32);

/// A brand whose two functions run in the order written.
#[model_schema(preprocess = ["unwrapped", "trimmed"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Caption(pub String);

/// A brand with a parameter.
#[model_schema(default_types(IdType = String), preprocess = ["trimmed"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Inked<IdType>(pub IdType);

/// An enum of unit variants.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Tier {
    Express,
    Standard,
}

/// The enum above in a field, in a list and as the key of a map.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tiered {
    pub by_tier: BTreeMap<Tier, u32>,
    pub tier: Tier,
    pub tiers: Vec<Tier>,
}

/// An internally tagged enum.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Conveyance {
    Air { flight: String },
    Ground,
}

/// An externally tagged enum.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Relay {
    Held,
    Rail(String),
    Road { stops: Vec<String> },
}

/// An adjacently tagged enum.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum Receipt {
    Left(String),
    Signed { by: String },
}

/// An untagged enum.
#[model_schema(preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Pointer {
    Named { name: String },
    Numbered(u32),
}

/// A type with a parameter.
#[model_schema(default_types(CodeType = String), preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bin<CodeType> {
    pub codes: Vec<CodeType>,
    pub label: String,
}

/// A type and a brand with a parameter, each held by another type, and the packet behind the
/// parameter of one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stockroom {
    pub bin: Bin<String>,
    pub bins: Vec<Bin<u32>>,
    pub of_packets: Bin<Packet>,
    pub stamped: Inked<String>,
}

/// A packet whose two functions run in the order written: out of its envelope, then completed.
#[model_schema(preprocess = ["unwrapped", "withoutFragile"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Enveloped {
    pub fragile_item_codes: Vec<String>,
    pub label: String,
}

/// A type that names a function and a check: the check judges what the function answered.
#[model_schema(check = ["withinLimit"], preprocess = ["unwrapped"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Scaled {
    pub grams: u32,
}

/// A label whose function names it where the payload holds none.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Nameless {
    #[model_schema_prop(preprocess = ["orUnnamed"])]
    pub label: String,
}

/// A label whose function answers a number as its text.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lot {
    #[model_schema_prop(preprocess = ["textual"])]
    pub label: String,
}

/// A member with two functions, a nullable one, a list of models and a tuple, each with a
/// function of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Chit {
    #[model_schema_prop(preprocess = ["trimmed", "lowered"])]
    pub code: String,
    #[model_schema_prop(preprocess = ["trimmed"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub packets: Vec<Packet>,
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub pair: (String, u32),
}

/// A member that holds a model, which its holder reads with nothing judged.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Housed {
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub packet: Packet,
}

/// Two members with a function of their own beside a flattened packet.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bonded {
    #[model_schema_prop(preprocess = ["trimmed"])]
    pub dock: String,
    #[model_schema_prop(preprocess = ["trimmed"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(flatten)]
    pub packet: Packet,
}

/// A type that names no function, which lists under `keys` what a function of its holder writes.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Loose {
    pub keys: Vec<String>,
    pub tag: String,
}

/// A flattened member that names a function, which is handed the keys its holder does not take.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Girdled {
    pub dock: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub loose: Loose,
}

/// A flattened map of lists that names a function, which adds the list of its keys.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sashed {
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: BTreeMap<String, Vec<String>>,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Belted {
    pub strap: String,
}

/// A type that names a function, flattened after one that takes its own key as it is read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lashed {
    pub dock: String,
    #[serde(flatten)]
    pub first: Belted,
    #[serde(flatten)]
    pub keyed: Itemized,
}

/// A type with a parameter whose members name functions: a list of the parameter, the parameter
/// itself, a text, and the parameter in an `Option`.
#[model_schema(default_types(CodeType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hamper<CodeType> {
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub codes: Vec<CodeType>,
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub held: CodeType,
    #[model_schema_prop(preprocess = ["trimmed"])]
    pub label: String,
    #[model_schema_prop(preprocess = ["unwrapped"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<CodeType>,
}

/// A member of a variant under a tag alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Passage {
    Air {
        #[model_schema_prop(preprocess = ["trimmed"])]
        flight: String,
    },
    Ground,
}

/// A member and a slot of a variant beside its tag.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum Portage {
    Boxed {
        #[model_schema_prop(preprocess = ["trimmed"])]
        label: String,
    },
    Lone(#[model_schema_prop(preprocess = ["trimmed"])] String),
}

/// A member and a slot of a variant under its name.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Carriage {
    Boxed {
        #[model_schema_prop(preprocess = ["trimmed"])]
        label: String,
    },
    Lone(#[model_schema_prop(preprocess = ["trimmed"])] String),
}

/// A member of an untagged variant held to a bound, and a slot held to one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Guess {
    Coded {
        #[model_schema_prop(minLength = 2, preprocess = ["trimmed"])]
        code: String,
    },
    Short(#[model_schema_prop(maxLength = 2, preprocess = ["trimmed"])] String),
    Whole(String),
}

/// A slot of a tuple struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stretch(
    #[model_schema_prop(preprocess = ["trimmed"])] pub String,
    pub u32,
);

/// A member and a slot that reach the parameter of their enum.
#[model_schema(default_types(CodeType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum Pick<CodeType> {
    Many {
        #[model_schema_prop(preprocess = ["unwrapped"])]
        codes: Vec<CodeType>,
    },
    One(#[model_schema_prop(preprocess = ["unwrapped"])] CodeType),
}

/// A nullable whose function notes each payload it is handed.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Logged {
    #[model_schema_prop(preprocess = ["noted"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PacketRequest {
    pub dock: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum PacketError {
    Closed,
}

/// A service whose one reply is the packet.
#[service_schema(transports = [])]
pub trait PacketClientService<Ctx> {
    async fn sent(&self, ctx: &Ctx, req: PacketRequest) -> Result<Packet, PacketError>;
}

/// A backend answering the contract, so the trait is implementable rather than merely declared.
pub struct PacketBackEnd;

impl PacketClientService<()> for PacketBackEnd {
    async fn sent(&self, _ctx: &(), req: PacketRequest) -> Result<Packet, PacketError> {
        ready(()).await;
        if req.dock.is_empty() {
            return Err(PacketError::Closed);
        }
        Ok(Packet {
            fragile_item_codes: Vec::new(),
            item_code_list: Vec::new(),
            label: req.dock,
        })
    }
}

thread_local! {
    /// Each payload `noted` was handed, as it is written.
    static NOTED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn lowered(payload: Value) -> Value {
    if let Value::String(text) = payload {
        Value::String(text.to_lowercase())
    } else {
        payload
    }
}

fn noted(payload: Value) -> Value {
    NOTED.with(|calls| calls.borrow_mut().push(payload.to_string()));
    payload
}

fn or_unnamed(payload: Value) -> Value {
    if payload.is_null() {
        Value::String("unnamed".to_owned())
    } else {
        payload
    }
}

fn textual(payload: Value) -> Value {
    if let Value::Number(number) = &payload {
        Value::String(number.to_string())
    } else {
        payload
    }
}

fn trimmed(payload: Value) -> Value {
    if let Value::String(text) = payload {
        Value::String(text.trim().to_owned())
    } else {
        payload
    }
}

fn trimmed_plate(mut payload: Value) -> Value {
    if let Some(Value::String(plate)) = payload.get_mut("plate") {
        *plate = plate.trim().to_owned();
    }
    payload
}

fn unwrapped(payload: Value) -> Value {
    if let Value::Object(mut entries) = payload {
        entries
            .remove("payload")
            .unwrap_or_else(|| Value::Object(entries))
    } else {
        payload
    }
}

fn with_its_keys(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        let mut keys: Vec<String> = object.keys().cloned().collect();
        keys.sort_unstable();
        object.insert(
            "keys".to_owned(),
            Value::Array(keys.into_iter().map(Value::String).collect()),
        );
    }
    payload
}

fn within_limit(value: &Scaled) -> Result<(), Vec<String>> {
    if value.grams > 1000 {
        Err(vec![format!(
            "'grams': {} is over the limit of 1000",
            value.grams
        )])
    } else {
        Ok(())
    }
}

fn without_fragile(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object
            .entry("fragileItemCodes")
            .or_insert_with(|| Value::Array(Vec::new()));
    }
    payload
}

/// `text` as a Kotlin string writes it: a backslash, a quote and a `$` each behind a backslash.
fn kotlin_string(text: &str) -> String {
    format!(
        "\"{}\"",
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
    )
}

/// What a serde read of `payload` as a `T` prints under `name`: the value written back, or each
/// message of the refusal on a line of its own.
fn rust_read<T>(name: &str, payload: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    match serde_json::from_str::<T>(payload) {
        Ok(read) => format!("{name}: read {}\n", serde_json::to_string(&read).unwrap()),
        Err(refused) => {
            let mut printed = format!("{name}: refused\n");
            for message in refused.to_string().split("; ") {
                let _ = writeln!(printed, "    {message}");
            }
            printed
        }
    }
}

/// The read of `payload` by the serializer of the Kotlin type `read_as`, held to what the Rust
/// read of it as a `T` prints.
fn row<T>(name: &str, read_as: &str, payload: &str) -> Reading
where
    T: DeserializeOwned + Serialize,
{
    Reading {
        kotlin: format!(
            "told({}) {{ Json.encodeToString(serializer<{read_as}>(), \
             Json.decodeFromString(serializer<{read_as}>(), {})) }}",
            kotlin_string(name),
            kotlin_string(payload)
        ),
        told: rust_read::<T>(name, payload),
    }
}

fn report(rows: &[Reading]) -> String {
    rows.iter().map(|row| row.told.as_str()).collect()
}

fn statements(rows: &[Reading]) -> Vec<String> {
    rows.iter().map(|row| row.kotlin.clone()).collect()
}

/// The rows of the documents' sample, and the design's fourth line.
fn sample_rows() -> Vec<Reading> {
    vec![
        row::<Packet>(
            "row 1",
            "Packet",
            r#"{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}"#,
        ),
        row::<Packet>(
            "row 2",
            "Packet",
            r#"{"label":"   ","itemCodeList":["x"],"fragileItemCodes":["x"]}"#,
        ),
        row::<Packet>("row 3", "Packet", r#"{"label":"A","itemCodeList":["x"]}"#),
        row::<Lading>(
            "row 5",
            "Lading",
            r#"{"packet":{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}}"#,
        ),
        row::<Lot>("a number where a text is read", "Lot", r#"{"label":5}"#),
    ]
}

/// Every position another type reads the packet in.
fn held_rows() -> Vec<Reading> {
    vec![
        row::<Lading>(
            "a field",
            "Lading",
            r#"{"packet":{"label":"  A  ","itemCodeList":["x"]}}"#,
        ),
        row::<Locker>(
            "a list, a map and an Option",
            "Locker",
            r#"{"by_dock":{"north":{"label":" A ","itemCodeList":["x"]}},"list":[{"label":" B ","itemCodeList":[]}],"spare":{"label":" C ","itemCodeList":[]}}"#,
        ),
        row::<Encased>(
            "a flattened type",
            "Encased",
            r#"{"dock":"north","label":"  A  ","itemCodeList":["x"]}"#,
        ),
        row::<Fastened>(
            "the keys a flattened type is handed",
            "Fastened",
            r#"{"dock":"north","tag":"t"}"#,
        ),
        row::<Entrusted>(
            "a variant beside its tag",
            "Entrusted",
            r#"{"kind":"Held","detail":{"label":"  A  ","itemCodeList":["x"]}}"#,
        ),
        row::<Niche>(
            "a variant under its name",
            "Niche",
            r#"{"Held":{"label":"  A  ","itemCodeList":["x"]}}"#,
        ),
        row::<OneOf>(
            "an untagged variant",
            "OneOf",
            r#"{"label":"  A  ","itemCodeList":["x"]}"#,
        ),
        row::<Directed>(
            "a flattened choice",
            "Directed",
            r#"{"dock":"north","plate":"  AB 1  "}"#,
        ),
        row::<Stockroom>(
            "a type and a brand with a parameter, held, and a type behind one",
            "Stockroom",
            r#"{"bin":{"payload":{"codes":["x"],"label":"A"}},"bins":[{"payload":{"codes":[7],"label":"B"}}],"of_packets":{"payload":{"codes":[{"label":" C ","itemCodeList":[]}],"label":"D"}},"stamped":"  s  "}"#,
        ),
    ]
}

/// Every shape that names a function on itself, its payload in an envelope the function takes it
/// out of.
fn shape_rows() -> Vec<Reading> {
    vec![
        row::<ShutGate>("a unit struct", "ShutGate", r#"{"payload":{}}"#),
        row::<Pile>("a tuple struct", "Pile", r#"{"payload":[["x"],2]}"#),
        row::<Caption>("a brand", "Caption", r#"{"payload":"  A  "}"#),
        row::<Inked<String>>("a brand with a parameter", "Inked<String>", r#""  A  ""#),
        row::<Tier>("a plain enum", "Tier", r#"{"payload":"Express"}"#),
        row::<Tiered>(
            "a plain enum, held",
            "Tiered",
            r#"{"by_tier":{"Express":1},"tier":{"payload":"Standard"},"tiers":[{"payload":"Express"}]}"#,
        ),
        row::<Conveyance>(
            "an internally tagged enum",
            "Conveyance",
            r#"{"payload":{"kind":"Air","flight":"TX 1"}}"#,
        ),
        row::<Relay>(
            "an externally tagged enum",
            "Relay",
            r#"{"payload":{"Rail":"TX 1"}}"#,
        ),
        row::<Receipt>(
            "an adjacently tagged enum",
            "Receipt",
            r#"{"payload":{"kind":"Left","detail":"door"}}"#,
        ),
        row::<Pointer>("an untagged enum", "Pointer", r#"{"payload":7}"#),
        row::<Bin<String>>(
            "a type with a parameter, at a text",
            "Bin<String>",
            r#"{"payload":{"codes":["x"],"label":"A"}}"#,
        ),
        row::<Bin<u32>>(
            "a type with a parameter, at a number",
            "Bin<UInt>",
            r#"{"payload":{"codes":[7],"label":"A"}}"#,
        ),
        row::<Enveloped>(
            "two functions, in the order written",
            "Enveloped",
            r#"{"payload":{"label":"A"}}"#,
        ),
        row::<Scaled>(
            "a check, after the function",
            "Scaled",
            r#"{"payload":{"grams":2000}}"#,
        ),
        row::<Nameless>("a null the payload holds", "Nameless", r#"{"label":null}"#),
    ]
}

/// A member that names a function, in a class the plugin reads and in one the emitter reads.
fn struct_member_rows() -> Vec<Reading> {
    vec![
        row::<Chit>(
            "two functions on a field, a nullable, a list of models and a tuple",
            "Chit",
            r#"{"code":"  AB  ","note":"  n  ","packets":{"payload":[{"label":" A ","itemCodeList":[]}]},"pair":{"payload":["p",2]}}"#,
        ),
        row::<Chit>(
            "a nullable the payload leaves out",
            "Chit",
            r#"{"code":"ab","packets":[],"pair":["p",2]}"#,
        ),
        row::<Housed>(
            "a member that holds a model",
            "Housed",
            r#"{"packet":{"payload":{"label":"  A  ","itemCodeList":[]}}}"#,
        ),
        row::<Housed>(
            "a member that holds a model, judged by its holder",
            "Housed",
            r#"{"packet":{"payload":{"label":"   ","itemCodeList":[]}}}"#,
        ),
        row::<Bonded>(
            "a member beside a flattened one",
            "Bonded",
            r#"{"dock":"  north  ","note":"  n  ","label":"  A  ","itemCodeList":[]}"#,
        ),
        row::<Girdled>(
            "a flattened member that names a function",
            "Girdled",
            r#"{"dock":"north","other":1,"tag":"t"}"#,
        ),
        row::<Sashed>(
            "a flattened map that names a function",
            "Sashed",
            r#"{"other":["1"],"tag":["t"]}"#,
        ),
        row::<Lashed>(
            "a flattened type after one that took its keys",
            "Lashed",
            r#"{"dock":"north","strap":"s","tag":"t"}"#,
        ),
        row::<Hamper<String>>(
            "a member that reaches a parameter, at a text",
            "Hamper<String>",
            r#"{"codes":{"payload":["x"]},"held":{"payload":"h"},"label":"  A  ","spare":{"payload":"s"}}"#,
        ),
        row::<Hamper<u32>>(
            "a member that reaches a parameter, at a number",
            "Hamper<UInt>",
            r#"{"codes":{"payload":[7]},"held":{"payload":8},"label":"  A  "}"#,
        ),
    ]
}

/// A member and a slot that name a function, in a variant of each tagging and in a tuple struct.
fn variant_member_rows() -> Vec<Reading> {
    vec![
        row::<Passage>(
            "a member of a variant under a tag",
            "Passage",
            r#"{"kind":"Air","flight":"  TX 1  "}"#,
        ),
        row::<Portage>(
            "a member of a variant beside its tag",
            "Portage",
            r#"{"kind":"Boxed","detail":{"label":"  A  "}}"#,
        ),
        row::<Portage>(
            "a slot of a variant beside its tag",
            "Portage",
            r#"{"kind":"Lone","detail":"  A  "}"#,
        ),
        row::<Carriage>(
            "a member of a variant under its name",
            "Carriage",
            r#"{"Boxed":{"label":"  A  "}}"#,
        ),
        row::<Carriage>(
            "a slot of a variant under its name",
            "Carriage",
            r#"{"Lone":"  A  "}"#,
        ),
        row::<Guess>(
            "a member of an untagged variant",
            "Guess",
            r#"{"code":"  ab  "}"#,
        ),
        row::<Guess>(
            "a slot of an untagged variant, within its bound",
            "Guess",
            r#""  a  ""#,
        ),
        row::<Guess>(
            "a slot of an untagged variant, past its bound",
            "Guess",
            r#""  abc  ""#,
        ),
        row::<Stretch>("a slot of a tuple struct", "Stretch", r#"["  A  ",2]"#),
        row::<Pick<String>>(
            "a member of a variant that reaches a parameter",
            "Pick<String>",
            r#"{"kind":"Many","detail":{"codes":{"payload":["x"]}}}"#,
        ),
        row::<Pick<String>>(
            "a slot of a variant that reaches a parameter",
            "Pick<String>",
            r#"{"kind":"One","detail":{"payload":"x"}}"#,
        ),
    ]
}

fn definitions() -> Vec<String> {
    vec![
        packet_kotlin::kotlin_definition(),
        lading_kotlin::kotlin_definition(),
        locker_kotlin::kotlin_definition(),
        encased_kotlin::kotlin_definition(),
        itemized_kotlin::kotlin_definition(),
        fastened_kotlin::kotlin_definition(),
        hauler_kotlin::kotlin_definition(),
        directed_kotlin::kotlin_definition(),
        entrusted_kotlin::kotlin_definition(),
        niche_kotlin::kotlin_definition(),
        one_of_kotlin::kotlin_definition(),
        handoff_kotlin::kotlin_definition(),
        shut_gate_kotlin::kotlin_definition(),
        pile_kotlin::kotlin_definition(),
        caption_kotlin::kotlin_definition(),
        inked_kotlin::kotlin_definition(),
        tier_kotlin::kotlin_definition(),
        tiered_kotlin::kotlin_definition(),
        conveyance_kotlin::kotlin_definition(),
        relay_kotlin::kotlin_definition(),
        receipt_kotlin::kotlin_definition(),
        pointer_kotlin::kotlin_definition(),
        bin_kotlin::kotlin_definition(),
        stockroom_kotlin::kotlin_definition(),
        enveloped_kotlin::kotlin_definition(),
        scaled_kotlin::kotlin_definition(),
        nameless_kotlin::kotlin_definition(),
        lot_kotlin::kotlin_definition(),
        chit_kotlin::kotlin_definition(),
        housed_kotlin::kotlin_definition(),
        bonded_kotlin::kotlin_definition(),
        loose_kotlin::kotlin_definition(),
        girdled_kotlin::kotlin_definition(),
        sashed_kotlin::kotlin_definition(),
        belted_kotlin::kotlin_definition(),
        lashed_kotlin::kotlin_definition(),
        hamper_kotlin::kotlin_definition(),
        passage_kotlin::kotlin_definition(),
        portage_kotlin::kotlin_definition(),
        carriage_kotlin::kotlin_definition(),
        guess_kotlin::kotlin_definition(),
        stretch_kotlin::kotlin_definition(),
        pick_kotlin::kotlin_definition(),
        logged_kotlin::kotlin_definition(),
    ]
}

/// The emitted types, what is written by hand, and `main`, whose statements are `body`.
fn module(written_by_hand: &[&str], body: &[String]) -> String {
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(definitions());
    parts.extend(written_by_hand.iter().map(|&part| part.to_owned()));
    let mut main = String::from("fun main() = runBlocking {\n");
    for statement in body {
        let _ = writeln!(main, "    {statement}");
    }
    main.push_str("}\n");
    parts.push(main);
    parts.join("\n\n")
}

/// The four lines of the documents' sample, and the design's fourth: the Kotlin read prints what
/// the Rust read prints.
#[test]
fn the_sample_of_the_documents_reads_in_kotlin_as_it_reads_in_rust() {
    let rows = sample_rows();
    assert_eq!(report(&rows), SAMPLE);
    let Some(printed) = ran_kotlin(&module(
        &[SAMPLE_FUNCTIONS, SHAPE_FUNCTIONS, TOLD],
        &statements(&rows),
    )) else {
        return;
    };
    assert_eq!(printed, SAMPLE);
}

/// A type that holds the packet reads it through the packet's functions wherever it holds it,
/// and every shape that names a function on itself hands it its payload.
#[test]
fn a_type_is_corrected_wherever_it_is_held_and_in_every_shape() {
    let mut rows = held_rows();
    rows.extend(shape_rows());
    assert_eq!(report(&rows), POSITIONS);
    let Some(printed) = ran_kotlin(&module(
        &[SAMPLE_FUNCTIONS, SHAPE_FUNCTIONS, TOLD],
        &statements(&rows),
    )) else {
        return;
    };
    assert_eq!(printed, POSITIONS);
}

/// A member's functions are called on the member's own payload, in the order written, wherever
/// the member is read: by the plugin, by a serializer the emitter writes, behind a type parameter
/// and in a variant of each tagging.
#[test]
fn a_member_is_corrected_wherever_it_is_read() {
    let mut rows = struct_member_rows();
    rows.extend(variant_member_rows());
    assert_eq!(report(&rows), MEMBERS);
    let Some(printed) = ran_kotlin(&module(
        &[SAMPLE_FUNCTIONS, SHAPE_FUNCTIONS, TOLD],
        &statements(&rows),
    )) else {
        return;
    };
    assert_eq!(printed, MEMBERS);
}

/// Where the Kotlin read answers in words of its own: no function is called for a key the
/// payload leaves out, a `null` is handed to it, a payload the read refuses below a function is
/// named inside the value the function answered, `{Type}Unjudged` calls the functions too, and
/// writing calls none.
#[test]
fn where_the_kotlin_read_answers_in_its_own_words_it_is_this() {
    for payload in ["{}", r#"{"note":null}"#, r#"{"note":"a"}"#] {
        serde_json::from_str::<Logged>(payload).unwrap();
    }
    assert_eq!(NOTED.take(), ["null", "\"a\""]);
    let written = Packet {
        fragile_item_codes: Vec::new(),
        item_code_list: vec!["x".to_owned()],
        label: "  A  ".to_owned(),
    };
    assert_eq!(
        serde_json::to_value(&written).unwrap(),
        json!({ "fragileItemCodes": [], "itemCodeList": ["x"], "label": "  A  " })
    );
    let read = |read_as: &str, payload: &str| {
        format!(
            "Json.decodeFromString({read_as}, {})",
            kotlin_string(payload)
        )
    };
    let body =
        vec![
        format!(
            "thrown(\"a key the payload leaves out\") {{ {} }}",
            read("serializer<Packet>()", r#"{"itemCodeList":["x"]}"#)
        ),
        format!(
            "thrown(\"a number left as it is\") {{ {} }}",
            read("serializer<Packet>()", r#"{"label":5,"itemCodeList":["x"]}"#)
        ),
        format!(
            "thrown(\"a list left as it is\") {{ {} }}",
            read("serializer<Packet>()", r#"{"label":["A"],"itemCodeList":["x"]}"#)
        ),
        format!(
            "thrown(\"an item left as it is, below a type that names a function\") {{ {} }}",
            read(
                "serializer<Lading>()",
                r#"{"packet":{"label":"A","itemCodeList":[1]}}"#
            )
        ),
        format!(
            "thrown(\"the unjudged read\") {{ {} }}",
            read("PacketUnjudged", r#"{"label":"   ","itemCodeList":["x"]}"#)
        ),
        "thrown(\"a value built in code, written\") { Json.encodeToString(serializer<Packet>(), \
         Packet(emptyList(), listOf(\"x\"), \"  A  \")) }"
            .to_owned(),
        format!(
            "{}; {}; {}",
            read("serializer<Logged>()", "{}"),
            read("serializer<Logged>()", r#"{"note":null}"#),
            read("serializer<Logged>()", r#"{"note":"a"}"#)
        ),
        "thrown(\"the calls of a nullable's function\") { calls }".to_owned(),
    ];
    let Some(printed) = ran_kotlin(&module(&[SAMPLE_FUNCTIONS, SHAPE_FUNCTIONS, TOLD], &body))
    else {
        return;
    };
    assert_eq!(printed, APART_FROM_RUST);
}

/// A reply is read through the functions of its type by each client: a reply a function corrects
/// is the value, and one the bound refuses once corrected is the client's fault for a reply it
/// cannot read. So is a list where the label is read, which the decoder throws as no
/// `SerializationException`.
#[test]
fn each_client_reads_a_reply_through_the_functions_of_its_type() {
    let clients = [
        packet_request_kotlin::kotlin_definition(),
        packet_error_kotlin::kotlin_definition(),
        packet_client_service_fault_fields_kotlin::kotlin_definition(),
        packet_client_service_fault_kind_kotlin::kotlin_definition(),
        PacketClientServiceSchema::kotlin_http_client(),
        PacketClientServiceSchema::kotlin_ws_client(),
    ]
    .join("\n\n");
    let body = vec![
        format!(
            "val corrected = {}",
            kotlin_string(r#"{"label":"  A  ","itemCodeList":["x"]}"#)
        ),
        format!(
            "val refused = {}",
            kotlin_string(r#"{"label":"   ","itemCodeList":["x"]}"#)
        ),
        format!(
            "val listed = {}",
            kotlin_string(r#"{"label":["A"],"itemCodeList":["x"]}"#)
        ),
        "val httpCorrected = overHttp(corrected)".to_owned(),
        "val httpRefused = overHttp(refused)".to_owned(),
        "val httpListed = overHttp(listed)".to_owned(),
        "val socketCorrected = overTheSocket(corrected)".to_owned(),
        "val socketRefused = overTheSocket(refused)".to_owned(),
        "val socketListed = overTheSocket(listed)".to_owned(),
        "println(buildJsonObject { put(\"httpCorrected\", httpCorrected); put(\"httpRefused\", \
         httpRefused); put(\"socketCorrected\", socketCorrected); put(\"socketRefused\", \
         socketRefused) }.toString())"
            .to_owned(),
        "println(buildJsonObject { put(\"http\", httpListed); put(\"socket\", socketListed) \
         }.toString())"
            .to_owned(),
    ];
    let Some(printed) = ran_kotlin(&module(
        &[SAMPLE_FUNCTIONS, SHAPE_FUNCTIONS, clients.as_str(), CLIENTS],
        &body,
    )) else {
        return;
    };
    let (answered, listed) = printed.trim().split_once('\n').unwrap();
    let told: Value = serde_json::from_str(answered).unwrap();
    let thrown = "kotlinx.serialization.SerializationException: 'label': too short: minimum \
                  length is 1, got 0";
    assert_eq!(
        told,
        json!({
            "httpCorrected": { "label": "A", "fragile": 0_u32 },
            "httpRefused": { "kind": "undeserializable-payload", "detail": thrown },
            "socketCorrected": { "label": "A", "fragile": 0_u32 },
            "socketRefused": { "kind": "failed-validation", "detail": thrown }
        })
    );
    let of_a_list: Value = serde_json::from_str(listed).unwrap();
    for (client, kind) in [
        ("http", "undeserializable-payload"),
        ("socket", "failed-validation"),
    ] {
        assert_eq!(of_a_list[client]["kind"], kind);
        let detail = of_a_list[client]["detail"].as_str().unwrap();
        assert!(
            detail.starts_with("java.lang.IndexOutOfBoundsException"),
            "got: {detail}"
        );
    }
}

/// A name with no Kotlin function is the compiler's to refuse, at the call.
#[test]
fn a_preprocess_with_no_kotlin_function_does_not_compile() {
    let Some(refused) = refused_by_kotlinc(&module(&[SHAPE_FUNCTIONS, TOLD], &[])) else {
        return;
    };
    for named in ["trimmed", "withoutFragile"] {
        assert!(
            refused.contains(&format!("error: unresolved reference '{named}'.")),
            "no refusal of `{named}` in:\n{refused}"
        );
    }
}

/// The variant of an enum under a tag alone reads the packet it holds through the packet's
/// unjudged serializer, which calls the packet's functions. The emitted subclass reads it under a
/// `value` key, where serde writes the packet's keys beside the tag, so the row is not run.
#[test]
fn a_variant_under_a_tag_alone_reads_what_it_holds_through_its_functions() {
    assert_eq!(
        serde_json::to_value(Handoff::Held(Packet {
            fragile_item_codes: Vec::new(),
            item_code_list: Vec::new(),
            label: "A".to_owned(),
        }))
        .unwrap(),
        json!({ "kind": "Held", "fragileItemCodes": [], "itemCodeList": [], "label": "A" })
    );
    let emitted = handoff_kotlin::kotlin_definition();
    assert!(
        emitted.contains(
            "@SerialName(\"Held\") @Serializable data class HandoffHeld(@Serializable(with = \
             PacketUnjudged::class) val value: Packet) : Handoff"
        ),
        "got: {emitted}"
    );
}

/// The README's section on `preprocess` in the Kotlin client, held to the functions this file
/// compiles and to the rows its runs answer.
#[test]
fn the_readme_shows_the_functions_and_the_rows_that_run() {
    let readme = include_str!("../../README.md");
    assert!(
        readme.contains(&format!("```kotlin{SAMPLE_FUNCTIONS}```")),
        "the README no longer shows the functions the run compiles"
    );
    for shown in [
        "| A field's function | `fun trimmed(payload: JsonElement): JsonElement` |",
        "| A type's function | `fun withoutFragile(payload: JsonElement): JsonElement` |",
        "| 1 | `{\"label\":\"  A  \",\"itemCodeList\":[\"x\"],\"fragileItemCodes\":[\"x\"]}` | \
         read; `label` is `\"A\"` |",
        "| 2 | `{\"label\":\"   \",\"itemCodeList\":[\"x\"],\"fragileItemCodes\":[\"x\"]}` | \
         refused: `'label': too short: minimum length is 1, got 0` |",
        "| 3 | `{\"label\":\"A\",\"itemCodeList\":[\"x\"]}` | read; `fragileItemCodes` is `[]` |",
        "| 5 | row 1 inside `Manifest`, under `parcel` | read; `parcel.label` is `\"A\"` |",
        "String literal for value of key 'primitive' should be quoted at path: $.primitive",
        "`IndexOutOfBoundsException`",
        "error: unresolved reference 'trimmed'.",
    ] {
        assert!(readme.contains(shown), "not in the README: {shown}");
    }
}

/// The backend never suspends, so one poll answers it.
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

#[test]
fn the_backend_answers_the_contract() {
    let asked = |dock: &str| {
        poll_once(PacketBackEnd.sent(
            &(),
            PacketRequest {
                dock: dock.to_owned(),
            },
        ))
        .unwrap()
        .map(|sent| sent.label)
        .map_err(|refused| matches!(refused, PacketError::Closed))
    };
    assert_eq!(asked("north"), Ok("north".to_owned()));
    assert_eq!(asked(""), Err(true));
}
