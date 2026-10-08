//! A field typed with a type tixschema has not seen as a flagged model where the field's type is
//! expanded: one declared below, a plain alias, the type itself, and one of two types that hold
//! each other. Each is one whole value, and the type's own function gives the paths below it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

// The JSON schema of a type holding a `Label` names its module from here, beside the type.
#[cfg(feature = "jsonschema")]
use self::tag_schema as label_schema;
use super::shown;

/// An alias no `#[model_schema]` is written on.
type Label = Tag;

/// Holds itself, in a list and in an `Option`.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Category {
    children: Vec<Self>,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent: Option<Box<Self>>,
}

/// Holds [`Tag`], declared below it, and the same type behind a plain alias.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Post {
    label: Label,
    tag: Tag,
    title: String,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Tag {
    name: String,
}

/// Holds [`Reply`], which holds it back: the first of the two is expanded before the second is
/// seen.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Thread {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    latest: Option<Box<Reply>>,
    subject: String,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Reply {
    body: String,
    thread: Thread,
}

fn tag(name: &str) -> Tag {
    Tag {
        name: name.to_owned(),
    }
}

#[test]
fn a_model_declared_below_and_one_behind_a_plain_alias_are_one_whole_value() {
    let post = Post::MONGO_FIELDS;
    assert_eq!(
        shown(post.tag.eq(tag("news")).unwrap()),
        r#"{ "tag": { "$eq": { "name": "news" } } }"#
    );
    assert_eq!(
        shown(post.label.set(tag("pinned")).unwrap()),
        r#"{ "$set": { "label": { "name": "pinned" } } }"#
    );
}

/// The paths below a whole-value member are the nested type's own, built under the keys the
/// member's path holds.
#[test]
fn the_types_own_function_gives_the_paths_below_a_whole_value() {
    let post = Post::MONGO_FIELDS;
    let below = Tag::mongo_fields_under::<Post>(post.tag.segments());
    assert_eq!(
        shown(
            post.title
                .eq("Hello".to_owned())
                .unwrap()
                .and(below.name.eq("news".to_owned()).unwrap())
        ),
        r#"{ "$and": [{ "title": { "$eq": "Hello" } }, { "tag.name": { "$eq": "news" } }] }"#
    );
}

#[test]
fn a_type_that_holds_itself_is_one_whole_value_there() {
    let category = Category::MONGO_FIELDS;
    let leaf = Category {
        children: Vec::new(),
        name: "leaf".to_owned(),
        parent: None,
    };
    assert_eq!(
        shown(category.children.push(leaf).unwrap()),
        r#"{ "$push": { "children": { "children": [], "name": "leaf" } } }"#
    );
    assert_eq!(
        shown(category.parent.exists(false)),
        r#"{ "parent": { "$exists": false } }"#
    );
    let below = Category::mongo_fields_under::<Category>(category.parent.segments());
    assert_eq!(
        shown(below.name.eq("root".to_owned()).unwrap()),
        r#"{ "parent.name": { "$eq": "root" } }"#
    );
}

/// Of two types that hold each other, the one declared first holds the other as one whole value,
/// and the one declared second reaches the first one's paths.
#[test]
fn two_types_that_hold_each_other_both_build() {
    let thread = Thread::MONGO_FIELDS;
    assert_eq!(
        shown(thread.latest.exists(true)),
        r#"{ "latest": { "$exists": true } }"#
    );
    let reply = Reply::MONGO_FIELDS;
    assert_eq!(
        shown(reply.thread.subject.eq("Hi".to_owned()).unwrap()),
        r#"{ "thread.subject": { "$eq": "Hi" } }"#
    );
    assert_eq!(
        shown(reply.thread.latest.exists(false)),
        r#"{ "thread.latest": { "$exists": false } }"#
    );
    let below = Reply::mongo_fields_under::<Thread>(thread.latest.segments());
    assert_eq!(
        shown(below.thread.subject.eq("Hi".to_owned()).unwrap()),
        r#"{ "latest.thread.subject": { "$eq": "Hi" } }"#
    );
}
