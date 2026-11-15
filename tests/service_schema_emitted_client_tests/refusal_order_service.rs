//! `HoistService`, whose messages are declared above it, below it and in another crate, each with a
//! bound on its members and on the item it holds: the failed-validation fault of a read lists one
//! issue per broken rule whichever way the message is placed.

use super::tests::poll_once;
use core::future::{Future, ready};
use refusal_order_apart::{ApartLoad, ApartRenamed, apart_load_schema, apart_renamed_schema};
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

/// The operations of the service, each the place or the name its message is declared under.
pub const OPERATIONS: [&str; 6] = ["above", "apart", "below", "moved", "shifted", "tasked"];

/// The payload of a load of each place that breaks all three rules: the label, the weight and the
/// tag of its first item.
pub const BROKEN_LOAD: &str = r#"{"items":[{"tag":""}],"label":"","weight":200}"#;

/// What a load declared above the service holds.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AboveItem {
    #[model_schema_prop(minLength = 1)]
    pub tag: String,
}

/// A load declared above the service.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AboveLoad {
    pub items: Vec<AboveItem>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    #[model_schema_prop(maximum = 100)]
    pub weight: u32,
}

/// A load of a consumer whose name ends as a type the service macro writes does, and which has
/// rules of its own.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShipmentMcpTask {
    pub items: Vec<AboveItem>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    #[model_schema_prop(maximum = 100)]
    pub weight: u32,
}

/// What a hoist declines to lift.
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum HoistError {
    Declined,
}

/// Lifts loads, each declared in another place.
#[service_schema(transports = ["mcp", "ws_rpc"])]
pub trait HoistService<Ctx> {
    /// Lifts a load declared above the service.
    async fn above(&self, ctx: &Ctx, load: AboveLoad) -> Result<AboveLoad, HoistError>;

    /// Lifts a load declared in another crate.
    async fn apart(&self, ctx: &Ctx, load: ApartLoad) -> Result<ApartLoad, HoistError>;

    /// Lifts a load declared below the service.
    async fn below(&self, ctx: &Ctx, load: BelowLoad) -> Result<BelowLoad, HoistError>;

    /// Lifts a load published under another name, declared below the service.
    async fn moved(&self, ctx: &Ctx, load: BelowRenamed) -> Result<BelowRenamed, HoistError>;

    /// Lifts a load published under another name, declared in another crate.
    async fn shifted(&self, ctx: &Ctx, load: ApartRenamed) -> Result<ApartRenamed, HoistError>;

    /// Lifts a load of a consumer whose name ends in `McpTask`.
    async fn tasked(&self, ctx: &Ctx, load: ShipmentMcpTask)
    -> Result<ShipmentMcpTask, HoistError>;
}

/// Lifts each load as it was handed.
pub struct Hoist;

impl HoistService<()> for Hoist {
    fn above(
        &self,
        _ctx: &(),
        load: AboveLoad,
    ) -> impl Future<Output = Result<AboveLoad, HoistError>> {
        ready(Ok(load))
    }

    fn apart(
        &self,
        _ctx: &(),
        load: ApartLoad,
    ) -> impl Future<Output = Result<ApartLoad, HoistError>> {
        ready(Ok(load))
    }

    fn below(
        &self,
        _ctx: &(),
        load: BelowLoad,
    ) -> impl Future<Output = Result<BelowLoad, HoistError>> {
        ready(Ok(load))
    }

    fn moved(
        &self,
        _ctx: &(),
        load: BelowRenamed,
    ) -> impl Future<Output = Result<BelowRenamed, HoistError>> {
        ready(Ok(load))
    }

    fn shifted(
        &self,
        _ctx: &(),
        load: ApartRenamed,
    ) -> impl Future<Output = Result<ApartRenamed, HoistError>> {
        ready(Ok(load))
    }

    fn tasked(
        &self,
        _ctx: &(),
        load: ShipmentMcpTask,
    ) -> impl Future<Output = Result<ShipmentMcpTask, HoistError>> {
        ready(Ok(load))
    }
}

/// What a load declared below the service holds.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BelowItem {
    #[model_schema_prop(minLength = 1)]
    pub tag: String,
}

/// A load declared below the service.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BelowLoad {
    pub items: Vec<BelowItem>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    #[model_schema_prop(maximum = 100)]
    pub weight: u32,
}

/// A load published under another name, declared below the service.
#[model_schema(enable = ["jsonschema"], name = "PublishedBelow")]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BelowRenamed {
    pub items: Vec<BelowItem>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    #[model_schema_prop(maximum = 100)]
    pub weight: u32,
}

/// The field and the words of each rule a read of [`BROKEN_LOAD`] refuses, as Rust words them.
fn issues_of(refused: &str) -> Vec<(String, String)> {
    let listed = refused
        .rsplit_once(" at line ")
        .map_or(refused, |(head, _)| head);
    listed
        .split("; ")
        .map(|line| {
            let (field, detail) = line
                .strip_prefix('\'')
                .and_then(|rest| rest.split_once("': "))
                .unwrap();
            (field.to_owned(), detail.to_owned())
        })
        .collect()
}

/// What Rust's read answers of [`BROKEN_LOAD`], one `(field, detail)` for each broken rule.
pub fn broken_issues() -> Vec<(String, String)> {
    issues_of(
        &serde_json::from_str::<AboveLoad>(BROKEN_LOAD)
            .unwrap_err()
            .to_string(),
    )
}

#[test]
fn rust_lists_one_issue_per_broken_rule_whichever_place_declares_the_load() {
    let expected = broken_issues();
    assert_eq!(
        expected
            .iter()
            .map(|(field, _)| field.as_str())
            .collect::<Vec<_>>(),
        ["items.0.tag", "label", "weight"]
    );
    assert_eq!(
        issues_of(
            &serde_json::from_str::<BelowLoad>(BROKEN_LOAD)
                .unwrap_err()
                .to_string()
        ),
        expected
    );
    assert_eq!(
        issues_of(
            &serde_json::from_str::<ApartLoad>(BROKEN_LOAD)
                .unwrap_err()
                .to_string()
        ),
        expected
    );
    for refused in [
        serde_json::from_str::<BelowRenamed>(BROKEN_LOAD)
            .unwrap_err()
            .to_string(),
        serde_json::from_str::<ApartRenamed>(BROKEN_LOAD)
            .unwrap_err()
            .to_string(),
        serde_json::from_str::<ShipmentMcpTask>(BROKEN_LOAD)
            .unwrap_err()
            .to_string(),
    ] {
        assert_eq!(issues_of(&refused), expected);
    }
    let sound: AboveLoad = serde_json::from_str(r#"{"items":[],"label":"a","weight":1}"#).unwrap();
    assert!(matches!(poll_once(Hoist.above(&(), sound)), Some(Ok(_))));
    let apart: ApartLoad = serde_json::from_str(r#"{"items":[],"label":"a","weight":1}"#).unwrap();
    assert!(matches!(poll_once(Hoist.apart(&(), apart)), Some(Ok(_))));
    let below: BelowLoad = serde_json::from_str(r#"{"items":[],"label":"a","weight":1}"#).unwrap();
    assert!(matches!(poll_once(Hoist.below(&(), below)), Some(Ok(_))));
    let moved: BelowRenamed =
        serde_json::from_str(r#"{"items":[],"label":"a","weight":1}"#).unwrap();
    assert!(matches!(poll_once(Hoist.moved(&(), moved)), Some(Ok(_))));
    let shifted: ApartRenamed =
        serde_json::from_str(r#"{"items":[],"label":"a","weight":1}"#).unwrap();
    assert!(matches!(
        poll_once(Hoist.shifted(&(), shifted)),
        Some(Ok(_))
    ));
    let tasked: ShipmentMcpTask =
        serde_json::from_str(r#"{"items":[],"label":"a","weight":1}"#).unwrap();
    assert!(matches!(poll_once(Hoist.tasked(&(), tasked)), Some(Ok(_))));
}
