//! A `Vec<u8>` through the emitted TypeScript service, run by node with the real `zod`: the client
//! of each transport hands a `Uint8Array` to the wire as the JSON array serde reads, the
//! dispatcher and the HTTP dispatcher answer the bytes an implementation returns as that array,
//! and each client reads the answer back into a `Uint8Array`.

use core::future::ready;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::{model_schema, service_schema};

use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
use super::tests::poll_once;

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

const REQUIRED_PACKAGES: &[&str] = &["zod"];

/// Sends bytes through each client to the dispatcher of its transport, and prints the text each
/// transport carried each way and what each client answered, the bytes written as `u8[..]`. The
/// bus is joined as an adapter joins it, each text read by `JSON.parse`.
const DRIVER: &str = r#"
const shown = (value) => JSON.parse(JSON.stringify(value, (_, held) => (held instanceof Uint8Array ? `u8[${[...held]}]` : held)));
const inverted = (bytes) => Uint8Array.from(bytes, (byte) => 255 - byte);

const impl = {
  async store(_ctx, req) {
    return { ok: true, value: { contentType: req.contentType, data: inverted(req.data) } };
  },
  async echo(_ctx, req) {
    return { ok: true, value: inverted(req) };
  },
};

const dispatch = createBytesClientServiceDispatcher(impl);
const handle = createBytesClientServiceHttpDispatcher(impl);

const overBus = async (call) => {
  const carried = [];
  const client = createBytesClientServiceClient({
    async notify() { throw new Error("no one-way operation is declared"); },
    async request(operation, payload, headers) {
      const sent = JSON.stringify(payload);
      const dispatched = await dispatch(undefined, operation, JSON.parse(sent), headers);
      const answered = JSON.stringify(dispatched.answered);
      carried.push(sent, answered);
      return { answered: JSON.parse(answered), headers: dispatched.headers };
    },
  });
  return { carried, answered: shown(await call(client)) };
};

const overHttp = async (call) => {
  const carried = [];
  const client = createBytesClientServiceHttpClient({
    async send(request) {
      const response = await handle(undefined, {
        body: request.body,
        headers: request.headers,
        method: request.method,
        path: request.path,
        query: request.query,
      });
      carried.push(new TextDecoder().decode(request.body), new TextDecoder().decode(response.body));
      return { status: response.status, headers: response.headers, body: response.body };
    },
  });
  return { carried, answered: shown(await call(client)) };
};

const calls = [
  (client) => client.store({ contentType: "image/png", data: new Uint8Array([137, 80, 78, 71]) }),
  (client) => client.store({ contentType: "image/png", data: Uint8Array.from([]) }),
  (client) => client.echo(new Uint8Array([0, 1, 255])),
];
const ran = [];
for (const call of calls) {
  ran.push({ bus: await overBus(call), http: await overHttp(call) });
}
console.log(JSON.stringify(ran));
"#;

/// A picture, whose bytes travel as a JSON array.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BytesPhoto {
    pub content_type: String,
    pub data: Vec<u8>,
}

/// The one error each operation declares.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "errorCode")]
pub enum BytesFault {
    Refused,
}

/// An operation over a message that holds bytes, and one over bytes as the whole message.
#[service_schema(transports = [])]
pub trait BytesClientService<Ctx> {
    #[service_schema_op(http(method = "POST", path = "/bytes"))]
    async fn echo(&self, ctx: &Ctx, req: Vec<u8>) -> Result<Vec<u8>, BytesFault>;

    #[service_schema_op(http(method = "POST", path = "/photos"))]
    async fn store(&self, ctx: &Ctx, req: BytesPhoto) -> Result<BytesPhoto, BytesFault>;
}

/// A backend answering the contract, so the trait is implementable rather than merely declared.
pub struct BytesBackEnd;

impl BytesClientService<()> for BytesBackEnd {
    async fn echo(&self, _ctx: &(), req: Vec<u8>) -> Result<Vec<u8>, BytesFault> {
        ready(()).await;
        Ok(req.iter().map(|byte| 255 - byte).collect())
    }

    async fn store(&self, _ctx: &(), req: BytesPhoto) -> Result<BytesPhoto, BytesFault> {
        ready(()).await;
        Ok(BytesPhoto {
            content_type: req.content_type,
            data: req.data.iter().map(|byte| 255 - byte).collect(),
        })
    }
}

/// The messages, the service, its dispatchers and its clients, beside the real `zod`.
fn emitted() -> String {
    [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        BytesPhoto::ts_definition(),
        BytesPhoto::zod_schema(),
        BytesFault::ts_definition(),
        BytesFault::zod_schema(),
        BytesClientServiceSchema::ts_definition(),
        BytesClientServiceSchema::ts_service(),
        BytesClientServiceSchema::ts_http_service(),
        BytesClientServiceSchema::ts_client(),
        BytesClientServiceSchema::ts_http_client(),
    ]
    .join("\n\n")
}

/// What each transport carries each way of each call of [`DRIVER`], and what each client answers.
fn carried() -> Value {
    json!([
        {
            "bus": {
                "carried": [
                    r#"{"contentType":"image/png","data":[137,80,78,71]}"#,
                    r#"{"ok":true,"value":{"contentType":"image/png","data":[118,175,177,184]}}"#,
                ],
                "answered": {
                    "ok": true,
                    "value": { "contentType": "image/png", "data": "u8[118,175,177,184]" },
                },
            },
            "http": {
                "carried": [
                    r#"{"contentType":"image/png","data":[137,80,78,71]}"#,
                    r#"{"contentType":"image/png","data":[118,175,177,184]}"#,
                ],
                "answered": {
                    "ok": true,
                    "value": { "contentType": "image/png", "data": "u8[118,175,177,184]" },
                },
            },
        },
        {
            "bus": {
                "carried": [
                    r#"{"contentType":"image/png","data":[]}"#,
                    r#"{"ok":true,"value":{"contentType":"image/png","data":[]}}"#,
                ],
                "answered": {
                    "ok": true,
                    "value": { "contentType": "image/png", "data": "u8[]" },
                },
            },
            "http": {
                "carried": [
                    r#"{"contentType":"image/png","data":[]}"#,
                    r#"{"contentType":"image/png","data":[]}"#,
                ],
                "answered": {
                    "ok": true,
                    "value": { "contentType": "image/png", "data": "u8[]" },
                },
            },
        },
        {
            "bus": {
                "carried": ["[0,1,255]", r#"{"ok":true,"value":[255,254,0]}"#],
                "answered": { "ok": true, "value": "u8[255,254,0]" },
            },
            "http": {
                "carried": ["[0,1,255]", "[255,254,0]"],
                "answered": { "ok": true, "value": "u8[255,254,0]" },
            },
        },
    ])
}

/// Each client sends the bytes as a JSON array, each dispatcher answers them as one, and each
/// client reads the answer back into a `Uint8Array`, over both transports.
#[test]
fn bytes_travel_as_a_json_array_through_each_typescript_client_and_dispatcher() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "the emitted service's bytes");
        return;
    };
    let module = format!("{}\n\n{DRIVER}", emitted());
    let Some(wrote) =
        ran_with_modules("bytes", RUNTIME_VAR, "node", "bytes.mts", &module, &modules)
    else {
        return;
    };
    let ran: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(ran, carried());
    let read: BytesPhoto =
        serde_json::from_str(ran[0]["bus"]["carried"][0].as_str().unwrap()).unwrap();
    assert_eq!(
        poll_once(BytesBackEnd.store(&(), read))
            .map(|answered| answered.map(|photo| photo.data).ok()),
        Some(Some(vec![118, 175, 177, 184]))
    );
    assert_eq!(
        poll_once(BytesBackEnd.echo(&(), vec![0, 1, 255])).map(Result::ok),
        Some(Some(vec![255, 254, 0]))
    );
}
