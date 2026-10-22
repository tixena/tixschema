//! The emitted TypeScript clients, run by node with the real `zod`, sending a message over each
//! transport: the HTTP client, the transport-agnostic client and that client over the WebSocket
//! transport each send what the message's Zod check gives back, a date and time with no zone in
//! the form serde reads, and none of them sends a number outside its type's range.

#![cfg(feature = "chrono")]

use core::future::ready;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::{model_schema, service_schema};

use super::runtime::{node_modules, ran_with_modules, stand_down_modules};
use super::tests::poll_once;

const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

const REQUIRED_PACKAGES: &[&str] = &["zod"];

/// Sends each parcel through each client, and prints what each transport was handed of it, or
/// `refused` where the client refused it before reaching the transport.
const DRIVER: &str = r#"
async function through(send) {
  let carried = "refused";
  try {
    await send((payload) => { carried = payload; });
  } catch (_) {}
  return carried;
}

const overHttp = (parcel) => through(async (carry) => {
  const client = createConveyingClientServiceHttpClient({
    async send(request) {
      carry(JSON.parse(request.body));
      return { status: 204, headers: [], body: "" };
    },
  });
  await client.post(parcel);
});

const overBus = (parcel) => through(async (carry) => {
  const client = createConveyingClientServiceClient({
    async notify(_operation, payload, _headers) { carry(payload); },
    async request() { throw new Error("no reply is asked"); },
  });
  await client.post(parcel);
});

const overSocket = (parcel) => through(async (carry) => {
  const socket = {
    send(text) { carry(JSON.parse(text).payload); },
    close() {},
    addEventListener() {},
    removeEventListener() {},
  };
  const transport = createConveyingClientServiceWsTransport(socket, { heartbeat: false });
  await createConveyingClientServiceClient(transport).post(parcel);
  transport.close();
});

const parcels = [
  { at: "2024-01-05T10:20:30.000Z", count: 214, letter: "a" },
  { at: "2024-01-05T10:20", count: 214, letter: "a" },
  { at: "2024-01-05T10:20:30", count: 300, letter: "a" },
  { at: "2024-01-05T10:20:30", count: 214, letter: "ab" },
];
const sent = [];
for (const parcel of parcels) {
  sent.push({
    http: await overHttp(parcel),
    bus: await overBus(parcel),
    socket: await overSocket(parcel),
  });
}
console.log(JSON.stringify(sent));
"#;

/// A message whose members each client holds in a wider form than the Rust type.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct ConveyedParcel {
    pub at: NaiveDateTime,
    pub count: u8,
    pub letter: char,
}

/// One one-way operation, so what each client writes is all there is to see.
#[service_schema(transports = [])]
pub trait ConveyingClientService<Ctx> {
    #[service_schema_op(one_way, http(method = "POST", path = "/parcels"))]
    async fn post(&self, ctx: &Ctx, req: ConveyedParcel);
}

/// A backend answering the contract, so the trait is implementable rather than merely declared.
pub struct ConveyingBackEnd;

impl ConveyingClientService<()> for ConveyingBackEnd {
    async fn post(&self, _ctx: &(), _req: ConveyedParcel) {
        ready(()).await;
    }
}

/// The message, the service and the three clients, beside the real `zod`.
fn emitted() -> String {
    [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        ConveyedParcel::ts_definition(),
        ConveyedParcel::zod_schema(),
        ConveyingClientServiceSchema::ts_definition(),
        ConveyingClientServiceSchema::ts_client(),
        ConveyingClientServiceSchema::ts_http_client(),
        ConveyingClientServiceSchema::ts_ws_client(),
    ]
    .join("\n\n")
}

/// Each transport is handed the message in the form serde reads, or nothing.
#[test]
fn each_typescript_client_sends_the_form_serde_reads_over_every_transport() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "the emitted clients' wire forms");
        return;
    };
    let module = format!("{}\n\n{DRIVER}", emitted());
    let Some(wrote) = ran_with_modules(
        "wire-forms",
        RUNTIME_VAR,
        "node",
        "forms.mts",
        &module,
        &modules,
    ) else {
        return;
    };
    let sent: Value = serde_json::from_str(wrote.trim()).unwrap();
    let everywhere =
        |payload: &Value| json!({ "http": payload, "bus": payload, "socket": payload });
    assert_eq!(
        sent,
        json!([
            everywhere(&json!({ "at": "2024-01-05T10:20:30.000", "count": 214_u8, "letter": "a" })),
            everywhere(&json!({ "at": "2024-01-05T10:20:00", "count": 214_u8, "letter": "a" })),
            everywhere(&json!("refused")),
            everywhere(&json!("refused")),
        ]),
        "a TypeScript client sends a value in another form than serde reads"
    );
    for parcel in sent.as_array().unwrap().iter().take(2) {
        let read = serde_json::from_value::<ConveyedParcel>(parcel["http"].clone());
        assert!(read.is_ok(), "serde refuses {parcel}: {read:?}");
        assert_eq!(
            poll_once(ConveyingBackEnd.post(&(), read.unwrap())),
            Some(()),
            "the backend takes what serde read of {parcel}"
        );
    }
}
