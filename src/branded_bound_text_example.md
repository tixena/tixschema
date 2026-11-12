This builds, and `validate()` holds `latest` to the bound:

```rust
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct Label(String);

#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct Record {
    #[model_schema_prop(minLength = 1)]
    pub latest: Label,
}

fn main() {
    let empty = Record { latest: Label::new(String::new()).unwrap() };
    assert!(empty.validate().is_err());
}
```
