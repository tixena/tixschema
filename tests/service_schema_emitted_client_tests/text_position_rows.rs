//! A service that takes each type of [`WIRE_FORMS`] a service argument can be written with, in a
//! path, a query, a header and a part, and what a client is held to of each row handed there: the
//! text it writes in the form serde reads, or a refusal before the transport is reached, and a
//! request the Rust `http_rest` dispatcher reads that value from.
//!
//! Each client's run calls, once per row and position, the operation that takes the row's
//! argument there, the row's value in that argument and, in a path, a value in the common form in
//! every other of the operation's. The rows of a `u64`, a `usize`, a literal, a date under
//! `as_number` and an `ObjectId` are left out: a build with `swift` or `kotlin` refuses the first
//! two as an argument, and no argument takes the others.

#![cfg(all(
    feature = "chrono",
    any(feature = "dart", feature = "kotlin", feature = "swift")
))]

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use std::io;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::{model_schema, service_schema};

use super::text_placing_http_rest_transport;
use super::wire_forms_models::{
    ConveyedSlot, Sends, WIRE_FORMS, WireFormRow, conveyed_slot_schema,
};

/// What each operation was handed, under the name of its argument, as the JSON serde writes.
macro_rules! echoed {
    ($($argument:ident),* $(,)?) => {{
        ready(()).await;
        Ok(json!({ $(stringify!($argument): $argument),* }).to_string())
    }};
}

/// The argument each type is held in, by the model of [`WIRE_FORMS`] that holds it, with the
/// value, as JSON, an operation hands it where another row is handed: the arguments of each
/// operation, in the order the operation declares them.
pub const ARGUMENTS: [(&str, &str, &str); 18] = [
    ("flag", "ConveyedFlag", "true"),
    ("letter", "ConveyedLetter", r#""a""#),
    ("text", "ConveyedText", r#""t""#),
    ("octet", "ConveyedU8", "8"),
    ("pair", "ConveyedU16", "16"),
    ("quad", "ConveyedU32", "32"),
    ("tiny", "ConveyedI8", "-8"),
    ("small", "ConveyedI16", "-16"),
    ("medium", "ConveyedI32", "-32"),
    ("large", "ConveyedI64", "-64"),
    ("extent", "ConveyedIsize", "-1"),
    ("single", "ConveyedF32", "0.25"),
    ("precise", "ConveyedF64", "0.125"),
    ("day", "ConveyedDay", r#""2024-01-05""#),
    ("hour", "ConveyedHour", r#""10:20:30""#),
    ("stamp", "ConveyedStamp", r#""2024-01-05T10:20:30""#),
    (
        "instant",
        "ConveyedInstant",
        r#""2024-01-05T10:20:30.000Z""#,
    ),
    ("slot", "ConveyedSlotted", r#""Daily""#),
];

/// How many of [`ARGUMENTS`] an operation takes: each position has one operation per group of this
/// many, in order, which keeps every operation within seven parameters.
pub const GROUPED: usize = 4;

/// Texts a path holds only percent-encoded, each handed in the path alone, beside the table's rows:
/// a slash, a percent sign and a character beyond ASCII. The table's own text holds a space.
pub const ENCODED: [WireFormRow; 3] = [
    handed_text(r#""a/b""#),
    handed_text(r#""100%""#),
    handed_text("\"\u{e9}\""),
];

/// The positions, in the order each run calls them for a row.
pub const POSITIONS: [Position; 4] = [
    Position::Path,
    Position::Query,
    Position::Header,
    Position::Part,
];

/// A declared error, so each operation answers a reply a client reads.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum TextPlacingError {
    Refused,
}

/// Each type of [`ARGUMENTS`], in a path, a query, a header and a part, four arguments to an
/// operation, in the groups [`GROUPED`] cuts. Each operation answers what the dispatcher read, as
/// the JSON serde writes of it.
#[service_schema(transports = ["http_rest"])]
pub trait TextPlacingClientService<Ctx> {
    #[service_schema_op(http(
        method = "POST",
        path = "/headed/0",
        header_in("x-flag" = flag),
        header_in("x-letter" = letter),
        header_in("x-text" = text),
        header_in("x-octet" = octet),
    ))]
    async fn header_0(
        &self,
        ctx: &Ctx,
        note: String,
        flag: Option<bool>,
        letter: Option<char>,
        text: Option<String>,
        octet: Option<u8>,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(
        method = "POST",
        path = "/headed/1",
        header_in("x-pair" = pair),
        header_in("x-quad" = quad),
        header_in("x-tiny" = tiny),
        header_in("x-small" = small),
    ))]
    async fn header_1(
        &self,
        ctx: &Ctx,
        note: String,
        pair: Option<u16>,
        quad: Option<u32>,
        tiny: Option<i8>,
        small: Option<i16>,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(
        method = "POST",
        path = "/headed/2",
        header_in("x-medium" = medium),
        header_in("x-large" = large),
        header_in("x-extent" = extent),
        header_in("x-single" = single),
    ))]
    async fn header_2(
        &self,
        ctx: &Ctx,
        note: String,
        medium: Option<i32>,
        large: Option<i64>,
        extent: Option<isize>,
        single: Option<f32>,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(
        method = "POST",
        path = "/headed/3",
        header_in("x-precise" = precise),
        header_in("x-day" = day),
        header_in("x-hour" = hour),
        header_in("x-stamp" = stamp),
    ))]
    async fn header_3(
        &self,
        ctx: &Ctx,
        note: String,
        precise: Option<f64>,
        day: Option<NaiveDate>,
        hour: Option<NaiveTime>,
        stamp: Option<NaiveDateTime>,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(
        method = "POST",
        path = "/headed/4",
        header_in("x-instant" = instant),
        header_in("x-slot" = slot),
    ))]
    async fn header_4(
        &self,
        ctx: &Ctx,
        note: String,
        instant: Option<DateTime<Utc>>,
        slot: Option<ConveyedSlot>,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(method = "POST", path = "/parted/0", body = "multipart"))]
    async fn part_0(
        &self,
        ctx: &Ctx,
        flag: Option<bool>,
        letter: Option<char>,
        text: Option<String>,
        octet: Option<u8>,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(method = "POST", path = "/parted/1", body = "multipart"))]
    async fn part_1(
        &self,
        ctx: &Ctx,
        pair: Option<u16>,
        quad: Option<u32>,
        tiny: Option<i8>,
        small: Option<i16>,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(method = "POST", path = "/parted/2", body = "multipart"))]
    async fn part_2(
        &self,
        ctx: &Ctx,
        medium: Option<i32>,
        large: Option<i64>,
        extent: Option<isize>,
        single: Option<f32>,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(method = "POST", path = "/parted/3", body = "multipart"))]
    async fn part_3(
        &self,
        ctx: &Ctx,
        precise: Option<f64>,
        day: Option<NaiveDate>,
        hour: Option<NaiveTime>,
        stamp: Option<NaiveDateTime>,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(method = "POST", path = "/parted/4", body = "multipart"))]
    async fn part_4(
        &self,
        ctx: &Ctx,
        instant: Option<DateTime<Utc>>,
        slot: Option<ConveyedSlot>,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(method = "GET", path = "/placed/0/{flag}/{letter}/{text}/{octet}"))]
    async fn path_0(
        &self,
        ctx: &Ctx,
        flag: bool,
        letter: char,
        text: String,
        octet: u8,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(method = "GET", path = "/placed/1/{pair}/{quad}/{tiny}/{small}"))]
    async fn path_1(
        &self,
        ctx: &Ctx,
        pair: u16,
        quad: u32,
        tiny: i8,
        small: i16,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(
        method = "GET",
        path = "/placed/2/{medium}/{large}/{extent}/{single}"
    ))]
    async fn path_2(
        &self,
        ctx: &Ctx,
        medium: i32,
        large: i64,
        extent: isize,
        single: f32,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(method = "GET", path = "/placed/3/{precise}/{day}/{hour}/{stamp}"))]
    async fn path_3(
        &self,
        ctx: &Ctx,
        precise: f64,
        day: NaiveDate,
        hour: NaiveTime,
        stamp: NaiveDateTime,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(method = "GET", path = "/placed/4/{instant}/{slot}"))]
    async fn path_4(
        &self,
        ctx: &Ctx,
        instant: DateTime<Utc>,
        slot: ConveyedSlot,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(method = "GET", path = "/queried/0"))]
    async fn query_0(
        &self,
        ctx: &Ctx,
        flag: Option<bool>,
        letter: Option<char>,
        text: Option<String>,
        octet: Option<u8>,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(method = "GET", path = "/queried/1"))]
    async fn query_1(
        &self,
        ctx: &Ctx,
        pair: Option<u16>,
        quad: Option<u32>,
        tiny: Option<i8>,
        small: Option<i16>,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(method = "GET", path = "/queried/2"))]
    async fn query_2(
        &self,
        ctx: &Ctx,
        medium: Option<i32>,
        large: Option<i64>,
        extent: Option<isize>,
        single: Option<f32>,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(method = "GET", path = "/queried/3"))]
    async fn query_3(
        &self,
        ctx: &Ctx,
        precise: Option<f64>,
        day: Option<NaiveDate>,
        hour: Option<NaiveTime>,
        stamp: Option<NaiveDateTime>,
    ) -> Result<String, TextPlacingError>;

    #[service_schema_op(http(method = "GET", path = "/queried/4"))]
    async fn query_4(
        &self,
        ctx: &Ctx,
        instant: Option<DateTime<Utc>>,
        slot: Option<ConveyedSlot>,
    ) -> Result<String, TextPlacingError>;
}

/// Answers each operation with what it was handed.
pub struct TextPlacingBackEnd;

impl TextPlacingClientService<()> for TextPlacingBackEnd {
    async fn header_0(
        &self,
        _ctx: &(),
        note: String,
        flag: Option<bool>,
        letter: Option<char>,
        text: Option<String>,
        octet: Option<u8>,
    ) -> Result<String, TextPlacingError> {
        echoed!(note, flag, letter, text, octet)
    }

    async fn header_1(
        &self,
        _ctx: &(),
        note: String,
        pair: Option<u16>,
        quad: Option<u32>,
        tiny: Option<i8>,
        small: Option<i16>,
    ) -> Result<String, TextPlacingError> {
        echoed!(note, pair, quad, tiny, small)
    }

    async fn header_2(
        &self,
        _ctx: &(),
        note: String,
        medium: Option<i32>,
        large: Option<i64>,
        extent: Option<isize>,
        single: Option<f32>,
    ) -> Result<String, TextPlacingError> {
        echoed!(note, medium, large, extent, single)
    }

    async fn header_3(
        &self,
        _ctx: &(),
        note: String,
        precise: Option<f64>,
        day: Option<NaiveDate>,
        hour: Option<NaiveTime>,
        stamp: Option<NaiveDateTime>,
    ) -> Result<String, TextPlacingError> {
        echoed!(note, precise, day, hour, stamp)
    }

    async fn header_4(
        &self,
        _ctx: &(),
        note: String,
        instant: Option<DateTime<Utc>>,
        slot: Option<ConveyedSlot>,
    ) -> Result<String, TextPlacingError> {
        echoed!(note, instant, slot)
    }

    async fn part_0(
        &self,
        _ctx: &(),
        flag: Option<bool>,
        letter: Option<char>,
        text: Option<String>,
        octet: Option<u8>,
    ) -> Result<String, TextPlacingError> {
        echoed!(flag, letter, text, octet)
    }

    async fn part_1(
        &self,
        _ctx: &(),
        pair: Option<u16>,
        quad: Option<u32>,
        tiny: Option<i8>,
        small: Option<i16>,
    ) -> Result<String, TextPlacingError> {
        echoed!(pair, quad, tiny, small)
    }

    async fn part_2(
        &self,
        _ctx: &(),
        medium: Option<i32>,
        large: Option<i64>,
        extent: Option<isize>,
        single: Option<f32>,
    ) -> Result<String, TextPlacingError> {
        echoed!(medium, large, extent, single)
    }

    async fn part_3(
        &self,
        _ctx: &(),
        precise: Option<f64>,
        day: Option<NaiveDate>,
        hour: Option<NaiveTime>,
        stamp: Option<NaiveDateTime>,
    ) -> Result<String, TextPlacingError> {
        echoed!(precise, day, hour, stamp)
    }

    async fn part_4(
        &self,
        _ctx: &(),
        instant: Option<DateTime<Utc>>,
        slot: Option<ConveyedSlot>,
    ) -> Result<String, TextPlacingError> {
        echoed!(instant, slot)
    }

    async fn path_0(
        &self,
        _ctx: &(),
        flag: bool,
        letter: char,
        text: String,
        octet: u8,
    ) -> Result<String, TextPlacingError> {
        echoed!(flag, letter, text, octet)
    }

    async fn path_1(
        &self,
        _ctx: &(),
        pair: u16,
        quad: u32,
        tiny: i8,
        small: i16,
    ) -> Result<String, TextPlacingError> {
        echoed!(pair, quad, tiny, small)
    }

    async fn path_2(
        &self,
        _ctx: &(),
        medium: i32,
        large: i64,
        extent: isize,
        single: f32,
    ) -> Result<String, TextPlacingError> {
        echoed!(medium, large, extent, single)
    }

    async fn path_3(
        &self,
        _ctx: &(),
        precise: f64,
        day: NaiveDate,
        hour: NaiveTime,
        stamp: NaiveDateTime,
    ) -> Result<String, TextPlacingError> {
        echoed!(precise, day, hour, stamp)
    }

    async fn path_4(
        &self,
        _ctx: &(),
        instant: DateTime<Utc>,
        slot: ConveyedSlot,
    ) -> Result<String, TextPlacingError> {
        echoed!(instant, slot)
    }

    async fn query_0(
        &self,
        _ctx: &(),
        flag: Option<bool>,
        letter: Option<char>,
        text: Option<String>,
        octet: Option<u8>,
    ) -> Result<String, TextPlacingError> {
        echoed!(flag, letter, text, octet)
    }

    async fn query_1(
        &self,
        _ctx: &(),
        pair: Option<u16>,
        quad: Option<u32>,
        tiny: Option<i8>,
        small: Option<i16>,
    ) -> Result<String, TextPlacingError> {
        echoed!(pair, quad, tiny, small)
    }

    async fn query_2(
        &self,
        _ctx: &(),
        medium: Option<i32>,
        large: Option<i64>,
        extent: Option<isize>,
        single: Option<f32>,
    ) -> Result<String, TextPlacingError> {
        echoed!(medium, large, extent, single)
    }

    async fn query_3(
        &self,
        _ctx: &(),
        precise: Option<f64>,
        day: Option<NaiveDate>,
        hour: Option<NaiveTime>,
        stamp: Option<NaiveDateTime>,
    ) -> Result<String, TextPlacingError> {
        echoed!(precise, day, hour, stamp)
    }

    async fn query_4(
        &self,
        _ctx: &(),
        instant: Option<DateTime<Utc>>,
        slot: Option<ConveyedSlot>,
    ) -> Result<String, TextPlacingError> {
        echoed!(instant, slot)
    }
}

/// Answers every fault `499`, under a header naming its kind.
struct RefusedAs499;

impl text_placing_http_rest_transport::FaultHandler for RefusedAs499 {
    fn on_fault(
        &self,
        fault: &text_placing_client_service_schema::ServiceFault,
    ) -> text_placing_http_rest_transport::OutgoingResponse {
        text_placing_http_rest_transport::OutgoingResponse::new(
            499,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            b"refused".to_vec(),
        )
    }
}

/// Where an operation writes its arguments as text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Position {
    Header,
    Part,
    Path,
    Query,
}

/// One call a run makes: the row it hands, the argument the row's value is handed in, and where.
pub struct Call {
    pub argument: &'static str,
    pub position: Position,
    pub row: &'static WireFormRow,
}

impl Call {
    /// The arguments of the operation the call is made through, in the order it declares them.
    pub fn arguments(&self) -> &'static [(&'static str, &'static str, &'static str)] {
        ARGUMENTS
            .chunks(GROUPED)
            .nth(self.group())
            .unwrap_or_default()
    }

    /// The number of the group of [`ARGUMENTS`] the call's argument is in.
    fn group(&self) -> usize {
        ARGUMENTS
            .iter()
            .position(|(named, _, _)| *named == self.argument)
            .map_or(0, |at| at / GROUPED)
    }

    /// The value `argument` is handed, as JSON: the row's in the row's argument, the common one in
    /// every other of a path, which takes every argument, and none in every other elsewhere.
    pub fn handed(&self, argument: &str) -> Option<&'static str> {
        if argument == self.argument {
            return Some(self.row.handed);
        }
        (self.position == Position::Path).then(|| {
            ARGUMENTS
                .iter()
                .find(|(named, _, _)| *named == argument)
                .map_or("null", |(_, _, common)| *common)
        })
    }

    /// The message the macro declares for the call's operation: `Path0Request`. A header's
    /// operation takes its one other argument, `note`, as its message, and declares none.
    pub fn message(&self) -> String {
        let operation = self.operation();
        let mut letters = operation.chars();
        let first = letters.next().map(|letter| letter.to_ascii_uppercase());
        format!("{}{}Request", first.unwrap_or_default(), letters.as_str())
    }

    /// The operation the call is made through, as its position and its group spell it: `path`,
    /// `query`, `header` or `part`, then the group's number.
    pub fn operation(&self) -> String {
        let position = match self.position {
            Position::Header => "header",
            Position::Part => "part",
            Position::Path => "path",
            Position::Query => "query",
        };
        format!("{position}{}", self.group())
    }
}

/// Every call a client whose column is `column` makes: each row it holds a value of, at each
/// position, in order, then each of [`ENCODED`] in a path.
pub fn calls(column: fn(&WireFormRow) -> Sends) -> Vec<Call> {
    let mut made: Vec<Call> = WIRE_FORMS
        .iter()
        .filter(|row| !matches!(column(row), Sends::Unheld | Sends::Unemitted))
        .filter_map(|row| {
            ARGUMENTS
                .iter()
                .find(|(_, held_by, _)| *held_by == row.held_by)
                .map(|(argument, _, _)| (row, *argument))
        })
        .flat_map(|(row, argument)| {
            POSITIONS.into_iter().map(move |position| Call {
                argument,
                position,
                row,
            })
        })
        .collect();
    made.extend(ENCODED.iter().map(|row| Call {
        argument: "text",
        position: Position::Path,
        row,
    }));
    made
}

/// A row of a text every client sends as it is handed, which serde reads.
const fn handed_text(handed: &'static str) -> WireFormRow {
    WireFormRow {
        dart: Sends::Handed,
        handed,
        held_by: "ConveyedText",
        kotlin: Sends::Handed,
        serde: "read",
        swift: Sends::Handed,
        typescript: Sends::Handed,
    }
}

/// The text a client writes of a value it sends as `sent`, JSON: a string as itself, anything else
/// as its JSON.
fn text_of(sent: &str) -> String {
    serde_json::from_str::<Value>(sent)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| sent.to_owned())
}

/// Whether every character of `text` is legal in an HTTP header: visible ASCII, a space or a tab.
fn legal_in_a_header(text: &str) -> bool {
    text.chars()
        .all(|character| matches!(character, '\t' | ' ' | '!'..='~'))
}

/// What a client in `column` sends of the call's value, as JSON, and the text it writes of it, or
/// `None` where it refuses it before the transport is reached: the value the table states it sends
/// in a body, and a refusal of a header's text HTTP does not allow.
fn expected(call: &Call, column: fn(&WireFormRow) -> Sends) -> Option<(&'static str, String)> {
    let sent = match column(call.row) {
        Sends::Handed => call.row.handed,
        Sends::As(value) | Sends::Apart(value, _) => value,
        Sends::Refused | Sends::Unheld | Sends::Unemitted => return None,
    };
    let text = text_of(sent);
    (call.position != Position::Header || legal_in_a_header(&text)).then_some((sent, text))
}

/// `raw` with every `%XX` escape decoded.
fn percent_decoded(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && let Some(hex) = raw.get(index + 1..index + 3)
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            decoded.push(byte);
            index += 3;
            continue;
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(decoded).unwrap()
}

/// The text a recorded request carries in the argument of `call`, after the path's literal
/// segments, `placed` and the group's number.
fn carried(request: &Value, call: &Call) -> Option<String> {
    let text = |value: &Value| value.as_str().map(str::to_owned);
    match call.position {
        Position::Path => {
            let at = call
                .arguments()
                .iter()
                .position(|(named, _, _)| *named == call.argument)?;
            text(&request["path"])?
                .split('/')
                .nth(at + 3)
                .map(percent_decoded)
        }
        Position::Query => text(&request["query"])?
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .find(|(key, _)| *key == call.argument)
            .map(|(_, value)| percent_decoded(value)),
        Position::Header => request["headers"]
            .as_array()?
            .iter()
            .find(|pair| pair[0] == format!("x-{}", call.argument))
            .and_then(|pair| text(&pair[1])),
        Position::Part => request["parts"]
            .as_array()?
            .iter()
            .find(|pair| pair[0] == call.argument)
            .and_then(|pair| text(&pair[1])),
    }
}

/// What serde writes back of `json` read as the type the model `held_by` holds, or `None` where no
/// argument is of that type.
fn written_back(held_by: &str, json: &str) -> Option<Value> {
    fn back<T>(json: &str) -> Value
    where
        T: DeserializeOwned + Serialize,
    {
        serde_json::to_value(serde_json::from_str::<T>(json).unwrap()).unwrap()
    }
    Some(match held_by {
        "ConveyedFlag" => back::<bool>(json),
        "ConveyedLetter" => back::<char>(json),
        "ConveyedText" => back::<String>(json),
        "ConveyedU8" => back::<u8>(json),
        "ConveyedU16" => back::<u16>(json),
        "ConveyedU32" => back::<u32>(json),
        "ConveyedI8" => back::<i8>(json),
        "ConveyedI16" => back::<i16>(json),
        "ConveyedI32" => back::<i32>(json),
        "ConveyedI64" => back::<i64>(json),
        "ConveyedIsize" => back::<isize>(json),
        "ConveyedF32" => back::<f32>(json),
        "ConveyedF64" => back::<f64>(json),
        "ConveyedDay" => back::<NaiveDate>(json),
        "ConveyedHour" => back::<NaiveTime>(json),
        "ConveyedStamp" => back::<NaiveDateTime>(json),
        "ConveyedInstant" => back::<DateTime<Utc>>(json),
        "ConveyedSlotted" => back::<ConveyedSlot>(json),
        _ => return None,
    })
}

/// The dispatcher never suspends, so one poll answers it.
fn poll_once<Answering>(answering: Answering) -> Option<Answering::Output>
where
    Answering: Future,
{
    let mut pinned = pin!(answering);
    match pinned
        .as_mut()
        .poll(&mut PollContext::from_waker(Waker::noop()))
    {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    }
}

/// The status the Rust dispatcher answers `request` with, and what the operation answered of the
/// arguments it was handed. The path is handed as the client wrote it, percent-encoded.
fn dispatched(request: &Value) -> (u16, Value) {
    let text = |key: &str| request[key].as_str().unwrap().to_owned();
    let pairs = |key: &str| -> Vec<(String, String)> {
        request[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|pair| {
                (
                    pair[0].as_str().unwrap().to_owned(),
                    pair[1].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    };
    let incoming = text_placing_http_rest_transport::IncomingRequest::new(
        text("method"),
        text("path"),
        text("query"),
        pairs("headers"),
        text("body").into_bytes(),
    );
    let parts = pairs("parts")
        .into_iter()
        .map(|(name, value)| {
            (
                name,
                text_placing_http_rest_transport::IncomingPart::Text(value),
            )
        })
        .collect();
    let response = poll_once(text_placing_http_rest_transport::dispatch(
        &TextPlacingBackEnd,
        &(),
        &incoming,
        parts,
        &text_placing_http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    (
        response.status(),
        answered_of(response.body()).unwrap_or(Value::Null),
    )
}

/// What an operation answered in `body`, the JSON string of the JSON of what it was handed.
fn answered_of(body: &[u8]) -> Option<Value> {
    let said: Value = serde_json::from_slice(body).ok()?;
    serde_json::from_str(said.as_str()?).ok()
}

/// Holds what a client in `column` printed of each of its [`calls`], one JSON value per line, to
/// the table: each refused call refused, and each sent one carrying the row's value in the form
/// the table states at its position, which the Rust dispatcher reads back as serde reads that
/// form.
pub fn held_to_the_table(column: fn(&WireFormRow) -> Sends, printed: &str) {
    let made = calls(column);
    let lines: Vec<Value> = printed
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        lines.len(),
        made.len(),
        "the run printed one line per call:\n{printed}"
    );
    for (call, line) in made.iter().zip(&lines) {
        let named = format!(
            "{} {} in the {:?}",
            call.row.held_by, call.row.handed, call.position
        );
        let Some((sent, text)) = expected(call, column) else {
            assert_eq!(line, "refused", "{named} is refused before it is sent");
            continue;
        };
        assert_eq!(
            carried(line, call).as_deref(),
            Some(text.as_str()),
            "{named} is sent in the form the table states: {line}"
        );
        let (status, answered) = dispatched(line);
        assert_eq!(status, 200, "the Rust dispatcher reads {named}: {line}");
        assert_eq!(
            answered[call.argument],
            written_back(call.row.held_by, sent).unwrap(),
            "the Rust dispatcher reads {named} as serde reads the form sent: {line}"
        );
    }
}

/// Each argument of the operation of a call, in the order it declares them, spelled by `literal`,
/// a language's own spelling of a value of the model the argument's type is held by in the table,
/// or `absent` where the call hands it none.
pub fn spelled(call: &Call, literal: &dyn Fn(&str, &str) -> String, absent: &str) -> Vec<String> {
    call.arguments()
        .iter()
        .map(|(argument, held_by, _)| {
            call.handed(argument)
                .map_or_else(|| absent.to_owned(), |json| literal(held_by, json))
        })
        .collect()
}

/// The Swift definition of every message the service declares.
#[cfg(feature = "swift")]
pub fn messages_swift() -> String {
    [
        part0request_swift::swift_definition(),
        part1request_swift::swift_definition(),
        part2request_swift::swift_definition(),
        part3request_swift::swift_definition(),
        part4request_swift::swift_definition(),
        path0request_swift::swift_definition(),
        path1request_swift::swift_definition(),
        path2request_swift::swift_definition(),
        path3request_swift::swift_definition(),
        path4request_swift::swift_definition(),
        query0request_swift::swift_definition(),
        query1request_swift::swift_definition(),
        query2request_swift::swift_definition(),
        query3request_swift::swift_definition(),
        query4request_swift::swift_definition(),
    ]
    .join("\n\n")
}

/// The Kotlin definition of every message the service declares.
#[cfg(feature = "kotlin")]
pub fn messages_kotlin() -> String {
    [
        part0request_kotlin::kotlin_definition(),
        part1request_kotlin::kotlin_definition(),
        part2request_kotlin::kotlin_definition(),
        part3request_kotlin::kotlin_definition(),
        part4request_kotlin::kotlin_definition(),
        path0request_kotlin::kotlin_definition(),
        path1request_kotlin::kotlin_definition(),
        path2request_kotlin::kotlin_definition(),
        path3request_kotlin::kotlin_definition(),
        path4request_kotlin::kotlin_definition(),
        query0request_kotlin::kotlin_definition(),
        query1request_kotlin::kotlin_definition(),
        query2request_kotlin::kotlin_definition(),
        query3request_kotlin::kotlin_definition(),
        query4request_kotlin::kotlin_definition(),
    ]
    .join("\n\n")
}

#[test]
fn every_argument_a_row_is_handed_in_names_a_model_of_the_table() {
    for (_, held_by, common) in ARGUMENTS {
        assert!(
            WIRE_FORMS.iter().any(|row| row.held_by == held_by),
            "{held_by} holds no row of the table"
        );
        assert!(
            written_back(held_by, common).is_some(),
            "{held_by} is no type an argument is written with"
        );
    }
}

#[test]
fn the_route_table_names_each_operation_as_it_is_declared() {
    let table: Vec<_> = text_placing_http_rest_transport::ROUTES
        .iter()
        .map(|route| {
            (
                route.method(),
                route.operation().to_owned(),
                route.ok_status(),
                route.error_statuses(),
            )
        })
        .collect();
    let declared: &[u16] = &[422];
    let expected: Vec<_> = [
        ("POST", "header"),
        ("POST", "part"),
        ("GET", "path"),
        ("GET", "query"),
    ]
    .into_iter()
    .flat_map(|(method, position)| {
        (0..ARGUMENTS.len().div_ceil(GROUPED))
            .map(move |group| (method, format!("{position}-{group}"), 200, declared))
    })
    .collect();
    assert_eq!(table, expected, "the route table holds each operation");
    assert!(
        text_placing_http_rest_transport::ROUTES
            .iter()
            .any(|route| route.path() == "/placed/0/{flag}/{letter}/{text}/{octet}"),
        "a path is written as it is declared"
    );
}

/// A placeholder is read percent-decoded: each escape the byte it names, the bytes read as UTF-8,
/// and a malformed escape read as it stands.
#[test]
fn the_rust_dispatcher_reads_a_placeholder_percent_decoded() {
    let (status, read) = dispatched(&json!({
        "method": "GET",
        "path": "/placed/0/true/a/a%20b%2Fc%25%C3%A9%ZZ/8",
        "query": "",
        "headers": [],
        "body": "",
        "parts": [],
    }));
    assert_eq!(
        (status, &read["text"]),
        (200, &json!("a b/c%\u{e9}%ZZ")),
        "the dispatcher hands the implementation another text"
    );
}

/// A file part, which the test reads itself, the dispatcher reading none of this service's.
fn file_part(content: &[u8]) -> text_placing_http_rest_transport::IncomingPart {
    text_placing_http_rest_transport::IncomingPart::File(Box::new(io::Cursor::new(
        content.to_vec(),
    )))
}

/// A header the dispatcher does not read as its type is refused, and a file part where a text is
/// read is read as no value: what the Rust dispatcher answers of a value no client sends.
#[test]
fn the_rust_dispatcher_refuses_a_header_it_does_not_read_and_reads_a_file_part_as_none() {
    let refused = text_placing_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/headed/0".to_owned(),
        String::new(),
        vec![("x-octet".to_owned(), "300".to_owned())],
        br#""n""#.to_vec(),
    );
    assert_eq!(refused.headers().len(), 1, "the request carries its header");
    let answered = poll_once(text_placing_http_rest_transport::dispatch(
        &TextPlacingBackEnd,
        &(),
        &refused,
        Vec::new(),
        &RefusedAs499,
    ))
    .unwrap();
    assert_eq!(
        (answered.status(), answered.headers()),
        (
            499,
            &[("x-fault-kind".to_owned(), "failed validation".to_owned())][..]
        ),
        "a `u8` header of 300 is refused"
    );
    let mut held = [0_u8; 2];
    let pulled = match file_part(b"8") {
        text_placing_http_rest_transport::IncomingPart::File(mut source) => {
            source.pull(&mut held).unwrap()
        }
        text_placing_http_rest_transport::IncomingPart::Text(_) => 0,
    };
    assert_eq!(pulled, 1, "a file part holds its bytes");
    let filed = text_placing_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/parted/0".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let parted = poll_once(text_placing_http_rest_transport::dispatch(
        &TextPlacingBackEnd,
        &(),
        &filed,
        vec![("octet".to_owned(), file_part(b"8"))],
        &RefusedAs499,
    ))
    .unwrap();
    let read = answered_of(parted.body()).unwrap();
    assert_eq!(
        (parted.status(), &read["octet"]),
        (200, &Value::Null),
        "a file part is no text"
    );
}
