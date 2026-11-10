## arky Frontmatter Conversions (`arky`)

With tixschema's `arky` feature on, a type declared with `#[model_schema(enable = ["arky"])]` converts to arky's frontmatter `Value`, and is read back from one. Both go through serde, so a `Value` holds what serde writes for the type:

```rust
#[model_schema(enable = ["arky"])]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_on: Option<String>,
    pub tags: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    pub title: String,
}

let value: Value = (&note).try_into()?;  // { "tags": ["parser"], "title": "Notes" }
let note = Note::try_from(&value)?;
```

- **The write** is `TryFrom<&Note> for arky::parser::ast::Value`, through `arky::value::to_value`. It fails only where a member's own `Serialize` returns an error, or where a map key is not text, a number or a boolean.
- **The read** is `TryFrom<&Value> for Note`, through `arky::value::from_value`. It is the type's own read, so it refuses what [`validate()`](validation.md#a-read-refuses-what-validate-refuses) refuses: a `title` written `""` is refused with `'title': too short: minimum length is 1, got 0`. A date, a time and a date-time the frontmatter holds are read as the text chrono reads.
- **Both errors** are `arky::value::ValueError`.
- **A type with parameters** converts at any filling. Each impl is bounded on the type itself: the write on `Tagged<Id>: Serialize`, the read on `Tagged<Id>: DeserializeOwned`.
- **The recovering decode needs no entry of its own.** arky's `Value` turns into a `serde_json::Value`, and `from_value_with` reads it: `Note::from_value_with((&value).into(), decide)`, for a type that also lists `"decode_with"`.
- **A type alias is refused**, in every build: `` `enable = ["arky"]` cannot be written on a type alias: `Short` is another name for `String`, and the conversions would have to be written for that type. List it on the model types the alias reaches.``
- **What your crate lists:** `arky` and `serde`. Without the `arky` feature, the entry adds nothing.
