//! The read of the emitted Kotlin types, compiled with `-Werror` and run: the serializer of a type
//! throws for a payload whose value breaks a bound or a check, with every message of the value at
//! its full place, and `{Type}Unjudged` reads the same payload and builds the value. A row is
//! held to what the Rust read answers of the same payload, or, where a model judges itself as it
//! is read, to the text written here. The service clients answer their fault for such a reply.

#![cfg(feature = "kotlin")]

use alloc::collections::BTreeMap;
use core::fmt::{Debug, Write as _};
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

use self::loading_client_service_schema::{
    loading_client_service_fault_fields_kotlin, loading_client_service_fault_kind_kotlin,
};
use super::runtime::ran_kotlin;

const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// The functions the checks name, written by hand as a consumer writes them.
const CHECKS: &str = r#"
fun withoutDuplicates(value: List<String>): List<String> {
    val seen = mutableSetOf<String>()
    for (code in value) {
        if (!seen.add(code)) return listOf("duplicate item code $code")
    }
    return emptyList()
}

fun withinParcel(value: Parcel): List<String> =
    value.fragileItemCodes.withIndex()
        .filter { it.value !in value.itemCodeList }
        .map { "'fragileItemCodes.${it.index}': ${it.value} is outside of itemCodeList" }

fun unreserved(value: Tag): List<String> =
    if (value.value == "NONE") listOf("a tag is not NONE") else emptyList()

fun rated(value: Rating): List<String> =
    if (value == Rating.Unrated) listOf("a slip is rated") else emptyList()
"#;

/// What the driver prints a row with: the class of the value a read handed back, or each message
/// of what it threw.
const TOLD: &str = r#"
fun told(name: String, read: () -> Any) {
    try {
        println("$name: read ${read()::class.simpleName}")
    } catch (refused: SerializationException) {
        println("$name: refused")
        (refused.message ?: "").split("; ").forEach { println("    $it") }
    }
}
"#;

/// What the type's own serializer answers of each payload, which is what the Rust read answers of
/// it: the lines of the design's sample among them, the members of `Manifest` in the order of
/// their names.
const JUDGED: &str = "\
Parcel, a code outside its list: refused
    'fragileItemCodes.1': y is outside of itemCodeList
Parcel, held: read Parcel
Manifest, four broken parcels: refused
    'firstParcel.label': too short: minimum length is 1, got 0
    'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList
    'parcelsByDock.north.label': too short: minimum length is 1, got 0
    'spareParcel.fragileItemCodes.1': y is outside of itemCodeList
Manifest with no spareParcel key, held: read Manifest
Code, one character: refused
    too short: minimum length is 2, got 1
Code, two characters: read Code
Crate<Int>, its label and its seal broken: refused
    'label': too short: minimum length is 1, got 0
    'seal': too short: minimum length is 2, got 1
Crate<Int>, held: read Crate
Stack<Int>, a crate that breaks its bound: refused
    'crates.1.label': too short: minimum length is 1, got 0
Shipment by air, three stops: refused
    'stops': too long: maximum length is 2, got 3
Shipment by ground: read ShipmentGround
Route, two legs broken: refused
    'legs.0.stops': too long: maximum length is 2, got 3
    'legs.2.stops': too long: maximum length is 2, got 4
Route, a broken item under each tagging: refused
    'hauled.0.held.haulLabel': too short: minimum length is 1, got 0
    'legs.0.stops': too long: maximum length is 2, got 3
    'loads.1.Boxed.label': too short: minimum length is 1, got 0
Freight, a parcel that breaks its bound: refused
    'Boxed.label': too short: minimum length is 1, got 0
Haul, a label that breaks its bound: refused
    'held.haulLabel': too short: minimum length is 1, got 0
Mark, within the bound of the first variant: read MarkBrief
Mark, past the bound of the first variant: read MarkCoded
Mark, past that bound and outside the bound of the brand: read MarkPlain
Mark, refused by the check of the brand it holds: refused
    a tag is not NONE
Cargo, a pallet that breaks its bound: refused
    'label': too short: minimum length is 1, got 0
Slip, a rating its check refuses: refused
    'rating': a slip is rated
Rating, read on its own, one its check refuses: refused
    a slip is rated
";

/// What the read answers where it is not the text of the Rust read. A model judges itself as it
/// is read behind a type parameter, under a bound a member writes of its own on a brand, and
/// where its type is declared below its holder: the first broken one ends the read with its own
/// messages, under no place.
const APART_FROM_RUST: &str = "\
Crate<Parcel>, the second code broken: refused
    'fragileItemCodes.1': y is outside of itemCodeList
Crate<Code>, a code broken: refused
    too short: minimum length is 2, got 1
Docket, a code that breaks its own bound: refused
    'code': too short: minimum length is 2, got 1
    'code': too short: minimum length is 3, got 1
Hitch, its own bound and the trailer's broken: refused
    'plate': too short: minimum length is 1, got 0
";

/// What `{Type}Unjudged` answers of a payload the type's own serializer refuses: the value.
const UNJUDGED: &str = "\
Parcel: read Parcel
Manifest: read Manifest
Code: read Code
Crate<Int>: read Crate
Shipment: read ShipmentAir
Route: read Route
Freight: read FreightBoxed
Haul: read HaulLoose
Mark: read MarkCoded
Cargo: read CargoBoxed
";

/// A transport of each kind that answers one reply, and what a client made of it.
const CLIENTS: &str = r#"
class Answering(private val body: String) : LoadingClientServiceHttpTransport {
    override suspend fun send(request: LoadingClientServiceHttpRequest): LoadingClientServiceHttpResponse =
        LoadingClientServiceHttpResponse(200, emptyList(), body.encodeToByteArray())
}

class Replying(private val reply: String) : LoadingClientServiceWsSocket {
    override var onMessage: ((String) -> Unit)? = null
    override var onClose: (() -> Unit)? = null
    override fun send(text: String) {
        val id = Json.parseToJsonElement(text).jsonObject.getValue("id").jsonPrimitive.content
        onMessage?.invoke("""{"kind":"reply","id":"$id","service":"LoadingClientService","ok":true,"value":$reply}""")
    }
    override fun close() {
        onClose?.invoke()
    }
}

fun made(answered: LoadingClientServiceLoadResult): JsonElement = when (answered) {
    is LoadingClientServiceLoadResult.Ok -> buildJsonObject { put("label", answered.value.label) }
    is LoadingClientServiceLoadResult.Declared -> JsonPrimitive("declared")
    is LoadingClientServiceLoadResult.Fault -> buildJsonObject {
        put("kind", Json.encodeToJsonElement(serializer<LoadingClientServiceFaultKind>(), answered.fault.kind))
        put("detail", answered.fault.detail)
    }
}

suspend fun overHttp(body: String): JsonElement =
    made(LoadingClientServiceHttpClient(Answering(body)).load(LoadRequest("north")))

suspend fun overTheSocket(reply: String): JsonElement {
    val transport = LoadingClientServiceWsTransport(Replying(reply), LoadingClientServiceWsOptions(heartbeat = null))
    val answered = made(LoadingClientServiceWsClient(transport).load(LoadRequest("north")))
    transport.close()
    return answered
}
"#;

/// One row of a run: the Kotlin expression that reads a payload, and the lines it is held to.
struct Sample {
    kotlin: String,
    lines: Vec<String>,
}

/// The parcel of the documents: a bound, a member's check and a check of its own.
#[model_schema(check = ["withinParcel"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub fragile_item_codes: Vec<String>,
    #[model_schema_prop(check = ["withoutDuplicates"])]
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A parcel in a field, a list, a map and a nullable.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub first_parcel: Parcel,
    pub parcel_list: Vec<Parcel>,
    pub parcels_by_dock: BTreeMap<String, Parcel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_parcel: Option<Parcel>,
}

#[model_schema(minLength = 2)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Code(pub String);

/// A brand with a bound and a check, which a variant of an untagged enum holds.
#[model_schema(pattern = "^[A-Z]+$", check = ["unreserved"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Tag(pub String);

/// A type with a parameter, a bound of its own and a brand.
#[model_schema(default_types(CodeType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Crate<CodeType> {
    pub codes: Vec<CodeType>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    pub seal: Code,
}

/// An enum under a tag alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Shipment {
    Air {
        flight: String,
        #[model_schema_prop(maxLength = 2)]
        stops: Vec<String>,
    },
    Ground {
        stops: Vec<String>,
    },
}

/// An enum under its variants' names.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Freight {
    Boxed(Parcel),
    Empty,
}

/// An enum under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held", rename_all_fields = "camelCase")]
pub enum Haul {
    Loose {
        #[model_schema_prop(minLength = 1)]
        haul_label: String,
    },
    Parked,
}

/// A list of enums under each tagging.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Route {
    pub hauled: Vec<Haul>,
    pub legs: Vec<Shipment>,
    pub loads: Vec<Freight>,
}

/// An untagged enum each variant of which reads a string: the first holds it to a bound of its
/// own, and the second to the bound and the check of a brand.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Mark {
    Brief(#[model_schema_prop(maxLength = 3)] String),
    Coded(Tag),
    Plain(String),
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pallet {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// An untagged enum whose first variant holds a model, and whose second reads the same payload
/// with no bound.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Cargo {
    Boxed(Pallet),
    Named { label: String },
}

/// A type with a parameter that holds another at the same parameter.
#[model_schema(default_types(CodeType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stack<CodeType> {
    pub crates: Vec<Crate<CodeType>>,
}

/// A plain enum with a check.
#[model_schema(check = ["rated"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Rating {
    Fair,
    Unrated,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Slip {
    pub rating: Rating,
}

/// A brand under a bound the member writes of its own, held beside the brand's.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Docket {
    #[model_schema_prop(minLength = 3)]
    pub code: Code,
}

/// A holder of a type declared below it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hitch {
    #[model_schema_prop(minLength = 1)]
    pub hook: String,
    pub trailer: HitchedTrailer,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HitchedTrailer {
    #[model_schema_prop(minLength = 1)]
    pub plate: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoadRequest {
    pub dock: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum LoadError {
    Closed,
}

/// A service whose one reply is a type with a bound and a check.
#[service_schema(transports = [])]
pub trait LoadingClientService<Ctx> {
    async fn load(&self, ctx: &Ctx, req: LoadRequest) -> Result<Parcel, LoadError>;
}

/// A backend answering the contract, so the trait is implementable rather than merely declared.
pub struct LoadingBackEnd;

impl LoadingClientService<()> for LoadingBackEnd {
    async fn load(&self, _ctx: &(), req: LoadRequest) -> Result<Parcel, LoadError> {
        ready(()).await;
        if req.dock.is_empty() {
            return Err(LoadError::Closed);
        }
        Ok(parcel(&req.dock, &["x"], &["x"]))
    }
}

fn without_duplicates(value: &[String]) -> Result<(), Vec<String>> {
    for (index, code) in value.iter().enumerate() {
        if value[..index].contains(code) {
            return Err(vec![format!("duplicate item code {code}")]);
        }
    }
    Ok(())
}

fn within_parcel(value: &Parcel) -> Result<(), Vec<String>> {
    let outside: Vec<String> = value
        .fragile_item_codes
        .iter()
        .enumerate()
        .filter(|(_, code)| !value.item_code_list.contains(code))
        .map(|(index, code)| {
            format!("'fragileItemCodes.{index}': {code} is outside of itemCodeList")
        })
        .collect();
    if outside.is_empty() {
        Ok(())
    } else {
        Err(outside)
    }
}

fn unreserved(value: &Tag) -> Result<(), Vec<String>> {
    if value.0 == "NONE" {
        return Err(vec!["a tag is not NONE".to_owned()]);
    }
    Ok(())
}

fn rated(value: &Rating) -> Result<(), Vec<String>> {
    if matches!(value, Rating::Unrated) {
        return Err(vec!["a slip is rated".to_owned()]);
    }
    Ok(())
}

fn texts(written: &[&str]) -> Vec<String> {
    written.iter().map(|&text| text.to_owned()).collect()
}

fn parcel(label: &str, item_code_list: &[&str], fragile_item_codes: &[&str]) -> Parcel {
    Parcel {
        fragile_item_codes: texts(fragile_item_codes),
        item_code_list: texts(item_code_list),
        label: label.to_owned(),
    }
}

fn held() -> Parcel {
    parcel("A", &["x"], &["x"])
}

/// A parcel that breaks its one bound.
fn unlabelled() -> Parcel {
    parcel("", &["x"], &["x"])
}

/// A parcel that breaks its own check.
fn outside() -> Parcel {
    parcel("A", &["x"], &["x", "y"])
}

fn manifest(first: Parcel, second: Parcel, north: Parcel, spare: Option<Parcel>) -> Manifest {
    Manifest {
        first_parcel: first,
        parcel_list: vec![held(), second],
        parcels_by_dock: BTreeMap::from([("north".to_owned(), north)]),
        spare_parcel: spare,
    }
}

fn four_broken() -> Manifest {
    manifest(unlabelled(), outside(), unlabelled(), Some(outside()))
}

fn sealed_crate<CodeType>(codes: Vec<CodeType>, label: &str, seal: &str) -> Crate<CodeType> {
    Crate {
        codes,
        label: label.to_owned(),
        seal: Code(seal.to_owned()),
    }
}

fn by_air(stops: &[&str]) -> Shipment {
    Shipment::Air {
        flight: "F1".to_owned(),
        stops: texts(stops),
    }
}

fn by_ground() -> Shipment {
    Shipment::Ground {
        stops: texts(&["a"]),
    }
}

fn loose(label: &str) -> Haul {
    Haul::Loose {
        haul_label: label.to_owned(),
    }
}

fn route(hauled: Vec<Haul>, legs: Vec<Shipment>, loads: Vec<Freight>) -> Route {
    Route {
        hauled,
        legs,
        loads,
    }
}

fn two_legs_broken() -> Route {
    route(
        Vec::new(),
        vec![
            by_air(&["a", "b", "c"]),
            by_ground(),
            by_air(&["a", "b", "c", "d"]),
        ],
        Vec::new(),
    )
}

fn broken_under_each_tagging() -> Route {
    route(
        vec![loose(""), Haul::Parked],
        vec![by_air(&["a", "b", "c"])],
        vec![Freight::Empty, Freight::Boxed(unlabelled())],
    )
}

fn route_held() -> Route {
    route(
        vec![loose("l"), Haul::Parked],
        vec![by_air(&["a"]), by_ground()],
        vec![Freight::Empty, Freight::Boxed(held())],
    )
}

fn stack() -> Stack<i32> {
    Stack {
        crates: vec![
            sealed_crate(vec![1_i32], "A", "ab"),
            sealed_crate(vec![2_i32], "", "ab"),
        ],
    }
}

fn docket() -> Docket {
    Docket {
        code: Code("a".to_owned()),
    }
}

fn hitch() -> Hitch {
    Hitch {
        hook: String::new(),
        trailer: HitchedTrailer {
            plate: String::new(),
        },
    }
}

const fn mark_class(read: &Mark) -> &'static str {
    match read {
        Mark::Brief(_) => "MarkBrief",
        Mark::Coded(_) => "MarkCoded",
        Mark::Plain(_) => "MarkPlain",
    }
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

/// The payload serde writes for `value`, as a Kotlin string.
fn payload<T>(value: &T) -> String
where
    T: Serialize,
{
    kotlin_string(&serde_json::to_string(value).unwrap())
}

/// The lines a read is told in: the class of the value where one was handed back, and each
/// message where the read was refused.
fn lines(name: &str, answered: Result<&str, &[String]>) -> Vec<String> {
    match answered {
        Ok(class) => vec![format!("{name}: read {class}")],
        Err(refused) => {
            let mut told = vec![format!("{name}: refused")];
            told.extend(refused.iter().map(|line| format!("    {line}")));
            told
        }
    }
}

/// The read of the payload serde writes for `value` by the serializer of `read_as`, held to what
/// the Rust read answers of that payload: `class_of` names the Kotlin class of the value it read.
fn sample<T>(name: &str, read_as: &str, value: &T, class_of: fn(&T) -> &'static str) -> Sample
where
    T: DeserializeOwned + Serialize,
{
    let written = serde_json::to_string(value).unwrap();
    let answered = match serde_json::from_str::<T>(&written) {
        Ok(read) => lines(name, Ok(class_of(&read))),
        Err(refused) => {
            let messages: Vec<String> = refused
                .to_string()
                .split("; ")
                .map(ToOwned::to_owned)
                .collect();
            lines(name, Err(&messages))
        }
    };
    Sample {
        kotlin: format!(
            "told({}) {{ Json.decodeFromString(serializer<{read_as}>(), {}) }}",
            kotlin_string(name),
            payload(value)
        ),
        lines: answered,
    }
}

/// The read of the payload serde writes for `value` by `serializer`, held to `answered`.
fn pinned<T>(name: &str, serializer: &str, value: &T, answered: Result<&str, &[&str]>) -> Sample
where
    T: Serialize,
{
    let refused = answered.map_err(texts);
    Sample {
        kotlin: format!(
            "told({}) {{ Json.decodeFromString({serializer}, {}) }}",
            kotlin_string(name),
            payload(value)
        ),
        lines: lines(name, refused.as_ref().copied().map_err(Vec::as_slice)),
    }
}

/// What the Rust read refuses the payload serde writes for `value` with.
fn refused_by_rust<T>(value: &T) -> String
where
    T: Debug + DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(&serde_json::to_string(value).unwrap())
        .unwrap_err()
        .to_string()
}

fn report(rows: &[Sample]) -> String {
    let mut text = String::new();
    for row in rows {
        for line in &row.lines {
            let _ = writeln!(text, "{line}");
        }
    }
    text
}

/// The rows the Rust read answers too, of the same payload.
fn judged_rows() -> Vec<Sample> {
    let mut rows = vec![
        sample(
            "Parcel, a code outside its list",
            "Parcel",
            &outside(),
            |_| "Parcel",
        ),
        sample("Parcel, held", "Parcel", &held(), |_| "Parcel"),
        sample(
            "Manifest, four broken parcels",
            "Manifest",
            &four_broken(),
            |_| "Manifest",
        ),
        sample(
            "Manifest with no spareParcel key, held",
            "Manifest",
            &manifest(held(), held(), held(), None),
            |_| "Manifest",
        ),
        sample(
            "Code, one character",
            "Code",
            &Code("a".to_owned()),
            |_| "Code",
        ),
        sample(
            "Code, two characters",
            "Code",
            &Code("ab".to_owned()),
            |_| "Code",
        ),
        sample(
            "Crate<Int>, its label and its seal broken",
            "Crate<Int>",
            &sealed_crate(vec![1_i32], "", "a"),
            |_| "Crate",
        ),
        sample(
            "Crate<Int>, held",
            "Crate<Int>",
            &sealed_crate(vec![1_i32], "A", "ab"),
            |_| "Crate",
        ),
        sample(
            "Stack<Int>, a crate that breaks its bound",
            "Stack<Int>",
            &stack(),
            |_| "Stack",
        ),
    ];
    rows.extend(enum_rows());
    rows
}

/// The rows of an enum under each tagging, alone and in a list, and of the two untagged enums.
fn enum_rows() -> Vec<Sample> {
    let mark = |text: &str| Mark::Plain(text.to_owned());
    vec![
        sample(
            "Shipment by air, three stops",
            "Shipment",
            &by_air(&["a", "b", "c"]),
            |_| "ShipmentAir",
        ),
        sample(
            "Shipment by ground",
            "Shipment",
            &by_ground(),
            |_| "ShipmentGround",
        ),
        sample(
            "Route, two legs broken",
            "Route",
            &two_legs_broken(),
            |_| "Route",
        ),
        sample(
            "Route, a broken item under each tagging",
            "Route",
            &broken_under_each_tagging(),
            |_| "Route",
        ),
        sample(
            "Freight, a parcel that breaks its bound",
            "Freight",
            &Freight::Boxed(unlabelled()),
            |_| "FreightBoxed",
        ),
        sample(
            "Haul, a label that breaks its bound",
            "Haul",
            &loose(""),
            |_| "HaulLoose",
        ),
        sample(
            "Mark, within the bound of the first variant",
            "Mark",
            &mark("ab"),
            mark_class,
        ),
        sample(
            "Mark, past the bound of the first variant",
            "Mark",
            &mark("ABCD"),
            mark_class,
        ),
        sample(
            "Mark, past that bound and outside the bound of the brand",
            "Mark",
            &mark("abcd"),
            mark_class,
        ),
        sample(
            "Mark, refused by the check of the brand it holds",
            "Mark",
            &mark("NONE"),
            mark_class,
        ),
        sample(
            "Cargo, a pallet that breaks its bound",
            "Cargo",
            &Cargo::Named {
                label: String::new(),
            },
            |_| "CargoBoxed",
        ),
        sample(
            "Slip, a rating its check refuses",
            "Slip",
            &Slip {
                rating: Rating::Unrated,
            },
            |_| "Slip",
        ),
        sample(
            "Rating, read on its own, one its check refuses",
            "Rating",
            &Rating::Unrated,
            |_| "Rating",
        ),
    ]
}

/// The rows whose text is not the Rust read's.
fn apart_rows() -> Vec<Sample> {
    vec![
        pinned(
            "Crate<Parcel>, the second code broken",
            "serializer<Crate<Parcel>>()",
            &sealed_crate(vec![held(), outside()], "A", "ab"),
            Err(&["'fragileItemCodes.1': y is outside of itemCodeList"]),
        ),
        pinned(
            "Crate<Code>, a code broken",
            "serializer<Crate<Code>>()",
            &sealed_crate(vec![Code("ab".to_owned()), Code("a".to_owned())], "A", "ab"),
            Err(&["too short: minimum length is 2, got 1"]),
        ),
        pinned(
            "Docket, a code that breaks its own bound",
            "serializer<Docket>()",
            &docket(),
            Err(&[
                "'code': too short: minimum length is 2, got 1",
                "'code': too short: minimum length is 3, got 1",
            ]),
        ),
        pinned(
            "Hitch, its own bound and the trailer's broken",
            "serializer<Hitch>()",
            &hitch(),
            Err(&["'plate': too short: minimum length is 1, got 0"]),
        ),
    ]
}

/// A payload each type's own serializer refuses, read by the type's unjudged one.
fn unjudged_rows() -> Vec<Sample> {
    vec![
        pinned("Parcel", "ParcelUnjudged", &outside(), Ok("Parcel")),
        pinned(
            "Manifest",
            "ManifestUnjudged",
            &four_broken(),
            Ok("Manifest"),
        ),
        pinned("Code", "CodeUnjudged", &Code("a".to_owned()), Ok("Code")),
        pinned(
            "Crate<Int>",
            "CrateUnjudged(serializer<Int>())",
            &sealed_crate(vec![1_i32], "", "a"),
            Ok("Crate"),
        ),
        pinned(
            "Shipment",
            "ShipmentUnjudged",
            &by_air(&["a", "b", "c"]),
            Ok("ShipmentAir"),
        ),
        pinned("Route", "RouteUnjudged", &two_legs_broken(), Ok("Route")),
        pinned(
            "Freight",
            "FreightUnjudged",
            &Freight::Boxed(unlabelled()),
            Ok("FreightBoxed"),
        ),
        pinned("Haul", "HaulUnjudged", &loose(""), Ok("HaulLoose")),
        pinned(
            "Mark",
            "MarkUnjudged",
            &Mark::Plain("NONE".to_owned()),
            Ok("MarkCoded"),
        ),
        pinned(
            "Cargo",
            "CargoUnjudged",
            &Cargo::Named {
                label: String::new(),
            },
            Ok("CargoBoxed"),
        ),
    ]
}

fn definitions() -> Vec<String> {
    vec![
        parcel_kotlin::kotlin_definition(),
        manifest_kotlin::kotlin_definition(),
        code_kotlin::kotlin_definition(),
        tag_kotlin::kotlin_definition(),
        crate_kotlin::kotlin_definition(),
        shipment_kotlin::kotlin_definition(),
        freight_kotlin::kotlin_definition(),
        haul_kotlin::kotlin_definition(),
        route_kotlin::kotlin_definition(),
        mark_kotlin::kotlin_definition(),
        pallet_kotlin::kotlin_definition(),
        cargo_kotlin::kotlin_definition(),
        stack_kotlin::kotlin_definition(),
        rating_kotlin::kotlin_definition(),
        slip_kotlin::kotlin_definition(),
        docket_kotlin::kotlin_definition(),
        hitch_kotlin::kotlin_definition(),
        hitched_trailer_kotlin::kotlin_definition(),
    ]
}

/// The emitted types, the checks, and `main`, whose statements are `body`.
fn module(written_by_hand: &[&str], body: &[String]) -> String {
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(definitions());
    parts.push(CHECKS.to_owned());
    parts.extend(written_by_hand.iter().map(|&part| part.to_owned()));
    let mut main = String::from("fun main() = runBlocking {\n");
    for statement in body {
        let _ = writeln!(main, "    {statement}");
    }
    main.push_str("}\n");
    parts.push(main);
    parts.join("\n\n")
}

/// The statement that prints what `read_as` writes back of the payload serde writes for `value`.
fn written_back<T>(read_as: &str, value: &T) -> String
where
    T: Serialize,
{
    format!(
        "println(Json.encodeToString(serializer<{read_as}>(), \
         Json.decodeFromString(serializer<{read_as}>(), {})))",
        payload(value)
    )
}

#[test]
fn every_fixture_is_written_as_serde_writes_it() {
    assert_eq!(
        serde_json::to_value(four_broken()).unwrap(),
        serde_json::json!({
            "firstParcel": { "fragileItemCodes": ["x"], "itemCodeList": ["x"], "label": "" },
            "parcelList": [
                { "fragileItemCodes": ["x"], "itemCodeList": ["x"], "label": "A" },
                { "fragileItemCodes": ["x", "y"], "itemCodeList": ["x"], "label": "A" }
            ],
            "parcelsByDock": {
                "north": { "fragileItemCodes": ["x"], "itemCodeList": ["x"], "label": "" }
            },
            "spareParcel": { "fragileItemCodes": ["x", "y"], "itemCodeList": ["x"], "label": "A" }
        })
    );
    assert_eq!(
        serde_json::to_value(route_held()).unwrap(),
        serde_json::json!({
            "hauled": [{ "kind": "Loose", "held": { "haulLabel": "l" } }, { "kind": "Parked" }],
            "legs": [
                { "kind": "air", "flight": "F1", "stops": ["a"] },
                { "kind": "ground", "stops": ["a"] }
            ],
            "loads": [
                "Empty",
                { "Boxed": { "fragileItemCodes": ["x"], "itemCodeList": ["x"], "label": "A" } }
            ]
        })
    );
    assert_eq!(
        serde_json::to_value((
            sealed_crate(vec![Tag("AB".to_owned())], "A", "ab"),
            Mark::Brief("ab".to_owned()),
            Mark::Coded(Tag("ABCD".to_owned())),
            Cargo::Boxed(Pallet {
                label: "p".to_owned()
            }),
            hitch(),
            LoadRequest {
                dock: "north".to_owned()
            },
            LoadError::Closed,
            Slip {
                rating: Rating::Fair
            },
            docket()
        ))
        .unwrap(),
        serde_json::json!([
            { "codes": ["AB"], "label": "A", "seal": "ab" },
            "ab",
            "ABCD",
            { "label": "p" },
            { "hook": "", "trailer": { "plate": "" } },
            { "dock": "north" },
            { "errorCode": "closed" },
            { "rating": "Fair" },
            { "code": "a" }
        ])
    );
}

/// The serializer of a type throws what the Rust read answers of the same payload: every message
/// of the value at its full place, in one exception, and hands back a value that keeps every
/// rule. An untagged enum takes the variant serde takes.
#[test]
fn a_read_refuses_what_the_rust_read_refuses_in_its_words() {
    let rows = judged_rows();
    assert_eq!(report(&rows), JUDGED);
    let body: Vec<String> = rows.iter().map(|row| row.kotlin.clone()).collect();
    let Some(printed) = ran_kotlin(&module(&[TOLD], &body)) else {
        return;
    };
    assert_eq!(printed, JUDGED);
}

/// A model behind a type parameter, a brand under a bound of its member's own, and a model whose
/// type is declared below its holder, is read by its own serializer: the payload is refused
/// there, with that model's messages. The Rust read answers each of these payloads in other
/// words, written out here.
#[test]
fn where_the_read_is_not_the_rust_reads_text_it_is_this() {
    let rows = apart_rows();
    assert_eq!(report(&rows), APART_FROM_RUST);
    // What the Rust read answers of the same payloads. A model that judges itself is told at its
    // position in the text, and a brand under its member's key beside the member's own bound.
    // A model declared below its holder
    // is read unjudged, so the holder's own message is answered too, and the trailer's at its
    // place.
    assert_eq!(
        [
            refused_by_rust(&sealed_crate(vec![held(), outside()], "A", "ab")),
            refused_by_rust(&sealed_crate(
                vec![Code("ab".to_owned()), Code("a".to_owned())],
                "A",
                "ab"
            )),
            refused_by_rust(&docket()),
            refused_by_rust(&hitch()),
        ],
        [
            "'fragileItemCodes.1': y is outside of itemCodeList at line 1 column 134",
            "too short: minimum length is 2, got 1 at line 1 column 19",
            "'code': too short: minimum length is 2, got 1; 'code': too short: minimum length is 3, got 1",
            "'hook': too short: minimum length is 1, got 0; 'trailer.plate': too short: minimum \
             length is 1, got 0",
        ]
    );
    let body: Vec<String> = rows.iter().map(|row| row.kotlin.clone()).collect();
    let Some(printed) = ran_kotlin(&module(&[TOLD], &body)) else {
        return;
    };
    assert_eq!(printed, APART_FROM_RUST);
}

/// `{Type}Unjudged` reads a payload the type's own serializer refuses and builds the value.
#[test]
fn the_unjudged_serializer_of_a_type_reads_what_its_own_refuses() {
    let rows = unjudged_rows();
    assert_eq!(report(&rows), UNJUDGED);
    let body: Vec<String> = rows.iter().map(|row| row.kotlin.clone()).collect();
    let Some(printed) = ran_kotlin(&module(&[TOLD], &body)) else {
        return;
    };
    assert_eq!(printed, UNJUDGED);
}

/// A value that keeps every rule is written back as serde wrote it: an enum under each tagging in
/// a list, a holder of parcels, a type with a parameter and a variant of an untagged enum.
#[test]
fn a_value_that_keeps_every_rule_is_written_back_as_serde_wrote_it() {
    let kept = manifest(held(), held(), held(), None);
    let sealed = sealed_crate(vec![1_i32], "A", "ab");
    let coded = Mark::Coded(Tag("ABCD".to_owned()));
    let body = vec![
        written_back("Route", &route_held()),
        written_back("Manifest", &kept),
        written_back("Crate<Int>", &sealed),
        written_back("Mark", &coded),
    ];
    let Some(printed) = ran_kotlin(&module(&[], &body)) else {
        return;
    };
    assert_eq!(
        printed.lines().collect::<Vec<_>>(),
        [
            serde_json::to_string(&route_held()).unwrap(),
            serde_json::to_string(&kept).unwrap(),
            serde_json::to_string(&sealed).unwrap(),
            serde_json::to_string(&coded).unwrap(),
        ]
    );
}

/// A reply that breaks a rule is the fault each client answers for a reply it cannot read: an
/// undeserializable payload over HTTP, a failed validation over the socket, each with the messages
/// of the read. A reply that keeps every rule is the value.
#[test]
fn each_client_answers_its_fault_for_a_reply_that_breaks_a_rule() {
    let broken = parcel("", &["x"], &["x", "y"]);
    assert_eq!(
        serde_json::from_str::<Parcel>(&serde_json::to_string(&broken).unwrap())
            .unwrap_err()
            .to_string(),
        "'label': too short: minimum length is 1, got 0; 'fragileItemCodes.1': y is outside of \
         itemCodeList"
    );
    let clients = [
        load_request_kotlin::kotlin_definition(),
        load_error_kotlin::kotlin_definition(),
        loading_client_service_fault_fields_kotlin::kotlin_definition(),
        loading_client_service_fault_kind_kotlin::kotlin_definition(),
        LoadingClientServiceSchema::kotlin_http_client(),
        LoadingClientServiceSchema::kotlin_ws_client(),
    ]
    .join("\n\n");
    let body = vec![
        format!("val kept = {}", payload(&held())),
        format!("val broken = {}", payload(&broken)),
        "val httpKept = overHttp(kept)".to_owned(),
        "val httpBroken = overHttp(broken)".to_owned(),
        "val socketKept = overTheSocket(kept)".to_owned(),
        "val socketBroken = overTheSocket(broken)".to_owned(),
        "println(buildJsonObject { put(\"httpKept\", httpKept); put(\"httpBroken\", httpBroken); \
         put(\"socketKept\", socketKept); put(\"socketBroken\", socketBroken) }.toString())"
            .to_owned(),
    ];
    let Some(printed) = ran_kotlin(&module(&[clients.as_str(), CLIENTS], &body)) else {
        return;
    };
    let told: serde_json::Value = serde_json::from_str(printed.trim()).unwrap();
    let thrown = "kotlinx.serialization.SerializationException: 'label': too short: minimum \
                  length is 1, got 0; 'fragileItemCodes.1': y is outside of itemCodeList";
    assert_eq!(
        told,
        serde_json::json!({
            "httpKept": { "label": "A" },
            "httpBroken": { "kind": "undeserializable-payload", "detail": thrown },
            "socketKept": { "label": "A" },
            "socketBroken": { "kind": "failed-validation", "detail": thrown }
        })
    );
}

/// The README's section on the Kotlin read, held to the rows these runs answer and to the
/// constructor the emitter writes for the holder.
#[test]
fn the_readme_shows_what_the_runs_answer() {
    let readme = include_str!("../../README.md");
    let four: Vec<&str> = JUDGED
        .lines()
        .skip_while(|line| *line != "Manifest, four broken parcels: refused")
        .skip(1)
        .take(4)
        .map(str::trim_start)
        .collect();
    assert!(
        readme.contains(&format!("```text\n{}\n```", four.join(";\n"))),
        "the README no longer shows the four messages of the manifest"
    );
    let manifest = manifest_kotlin::kotlin_definition();
    for property in [
        "@Serializable(with = ParcelUnjudged::class) val firstParcel: Parcel",
        "val parcelList: List<@Serializable(with = ParcelUnjudged::class) Parcel>",
        "val parcelsByDock: Map<String, @Serializable(with = ParcelUnjudged::class) Parcel>",
        "@Serializable(with = ParcelUnjudged::class) val spareParcel: Parcel? = null",
    ] {
        assert!(manifest.contains(property), "not emitted: {property}");
        assert!(
            readme.contains(&format!("\n  {property},\n")),
            "not in the README: {property}"
        );
    }
    for import in KOTLIN_IMPORTS
        .lines()
        .filter(|import| import.contains("kotlinx.serialization"))
    {
        assert!(
            readme.contains(&format!("\n{import}\n")),
            "not in the README: {import}"
        );
    }
    for shown in [
        "// SerializationException: 'label': too short: minimum length is 1, got 0; \
         'fragileItemCodes.1': y is outside of itemCodeList\n",
        "| `\"ab\"` | `MarkBrief` |",
        "| `\"ABCD\"` | `MarkCoded`: ",
        "| `\"abcd\"` | `MarkPlain`: ",
        "| `\"NONE\"` | refused with `a tag is not NONE`: ",
        "#[model_schema(pattern = \"^[A-Z]+$\", check = [\"unreserved\"])]",
        "    Brief(#[model_schema_prop(maxLength = 3)] String),\n    Coded(Tag),\n    \
         Plain(String),\n",
        "with a broken parcel answers `'fragileItemCodes.1': y is outside of itemCodeList`",
        "`undeserializable-payload` from the `http_rest` client, and `failed-validation` from \
         the `ws_rpc` one",
        "Json.decodeFromString(ParcelUnjudged, payload)",
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
        poll_once(LoadingBackEnd.load(
            &(),
            LoadRequest {
                dock: dock.to_owned(),
            },
        ))
        .unwrap()
        .map(|loaded| loaded.label)
        .map_err(|refused| matches!(refused, LoadError::Closed))
    };
    assert_eq!(asked("north"), Ok("north".to_owned()));
    assert_eq!(asked(""), Err(true));
}
