//! A member written with an alias that rearranges the parameters of the generic model it names,
//! declared with `#[model_schema()]` above the holder.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(default_types(Item = String))]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<Item> {
    pub items: Vec<Item>,
    #[model_schema_prop(minLength = 1)]
    pub page_title: String,
}

#[model_schema(check = ["labelled"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
}

fn labelled(value: &ParcelDraft) -> Result<(), Vec<String>> {
    if value.label.is_empty() {
        return Err(vec!["'label': a parcel has a label".to_owned()]);
    }
    Ok(())
}

#[model_schema()]
pub type PagesOf<Item> = Page<Page<Item>>;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Binder {
    pub pages: PagesOf<Parcel>,
}

fn main() {}
