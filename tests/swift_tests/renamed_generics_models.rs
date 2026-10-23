//! The models of `swift_tests/renamed_generics.rs` and of
//! `service_schema_emitted_client_tests/run_swift_renamed_generics.rs`: a type that declares a
//! parameter and publishes under a name other than its Rust ident, which it republishes as a
//! `typealias`, and a holder that names each by its Rust ident, declared above them and below them.
//! The holder declared above holds no alias of a list, a map or an `Option`: Swift reads a type
//! declared below its holder as a model, which such an alias is not.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each type below, named by its Rust ident ahead of its declaration.
#[model_schema(default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lectern<Leaf> {
    pub adjacent_scroll: AdjacentScroll<Leaf>,
    pub diptych: Diptych<Leaf>,
    pub scroll: Scroll<Leaf>,
    pub shelfmark: Shelfmark<Leaf>,
    pub spread: Spread<Leaf>,
    pub tagged_scroll: TaggedScroll<Leaf>,
    pub tome: Tome<Leaf>,
    pub untagged_scroll: UntaggedScroll<Leaf>,
    pub volume: Volume<Leaf>,
}

/// A model filled with the parameter.
#[model_schema(default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Folio<Leaf> {
    pub leaf: Leaf,
    pub page: u32,
}

/// An alias of a list of the parameter.
#[model_schema(default_types(Leaf = String))]
pub type Leaves<Leaf> = Vec<Leaf>;

/// An alias of a map of the parameter.
#[model_schema(default_types(Leaf = String))]
pub type Catalog<Leaf> = HashMap<String, Leaf>;

/// An alias of a model filled with the parameter.
#[model_schema(default_types(Leaf = String))]
pub type Volume<Leaf> = Folio<Leaf>;

/// An alias of an `Option` of the parameter.
#[model_schema(default_types(Leaf = String))]
pub type Margin<Leaf> = Option<Leaf>;

/// An alias of a tuple of the parameter.
#[model_schema(default_types(Leaf = String))]
pub type Spread<Leaf> = (Leaf, u32);

/// An alias of a map of the parameter under a key Swift wraps. Compiled, and not read: Swift reads
/// such an alias through its own `Dictionary` coding, where serde writes an object.
#[model_schema(default_types(Leaf = String))]
pub type Quire<Leaf> = HashMap<u32, Leaf>;

/// A struct renamed by `name`.
#[model_schema(name = "Codex", default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tome<Leaf> {
    pub folio: Folio<Leaf>,
    pub leaf: Leaf,
}

/// A tuple struct renamed by `name`.
#[model_schema(name = "Twofold", default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Diptych<Leaf>(pub Leaf, pub u32);

/// An enum renamed by `name`, under no tag attribute.
#[model_schema(name = "Parchment", default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Scroll<Leaf> {
    Paired(Leaf, u32),
    Sealed(Leaf),
    Unrolled { folio: Folio<Leaf>, leaf: Leaf },
}

/// [`Scroll`] under a tag alone, renamed by `name`.
#[model_schema(name = "TaggedParchment", default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum TaggedScroll<Leaf> {
    Bound(Folio<Leaf>),
    Unrolled { folio: Folio<Leaf>, leaf: Leaf },
}

/// [`Scroll`] under a tag and a content key, renamed by `name`.
#[model_schema(name = "AdjacentParchment", default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentScroll<Leaf> {
    Paired(Leaf, u32),
    Sealed(Leaf),
    Unrolled { folio: Folio<Leaf>, leaf: Leaf },
}

/// [`Scroll`] untagged, renamed by `name`.
#[model_schema(name = "UntaggedParchment", default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UntaggedScroll<Leaf> {
    Sealed(Leaf),
    Unrolled { folio: Folio<Leaf>, leaf: Leaf },
}

/// A brand over the parameter, renamed by `name`.
#[model_schema(name = "CallNumber", default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Shelfmark<Leaf>(pub Leaf);

/// Each type above, named by its Rust ident after its declaration.
#[model_schema(default_types(Leaf = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Library<Leaf> {
    pub adjacent_scroll: AdjacentScroll<Leaf>,
    pub catalog: Catalog<Leaf>,
    pub diptych: Diptych<Leaf>,
    pub leaves: Leaves<Leaf>,
    #[model_schema_prop(nullable)]
    pub margin: Margin<Leaf>,
    pub scroll: Scroll<Leaf>,
    pub shelfmark: Shelfmark<Leaf>,
    pub spread: Spread<Leaf>,
    pub tagged_scroll: TaggedScroll<Leaf>,
    pub tome: Tome<Leaf>,
    pub untagged_scroll: UntaggedScroll<Leaf>,
    pub volume: Volume<Leaf>,
}
