//! A type that lists `"jsonschema"` hands its const parameter to a written type, in a build with no
//! feature that describes a type: its document is the surface that cannot spell the argument.

use tixschema::model_schema;

#[model_schema(enable = ["jsonschema"])]
pub struct Leaf<const WIDTH: usize> {
    pub items: [u8; WIDTH],
}

#[model_schema(enable = ["jsonschema"])]
pub struct Probe<const WIDTH: usize> {
    pub held: Leaf<WIDTH>,
}

fn main() {}
