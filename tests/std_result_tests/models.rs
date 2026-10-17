//! The models a std `Result` is written on, and the payloads serde writes and refuses for them.
//! Every surface and every client is held to the same payloads.

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A model with two bounds of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Masthead {
    #[model_schema_prop(maxLength = 8)]
    pub slug: String,
    #[model_schema_prop(minLength = 1)]
    pub title: String,
}

/// What a frontmatter that does not read holds, with a bound of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Unparsed {
    #[model_schema_prop(minLength = 1)]
    pub reason: String,
}

/// A brand with a bound of its own.
#[model_schema(minLength = 2)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Ballot(pub String);

/// A type that declares a parameter, which a `Result` fills.
#[model_schema(default_types(Held = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Mailbag<Held> {
    pub held: Held,
}

/// A `Result` of a number, and one whose `Ok` is `()`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reply {
    pub outcome: Result<u32, String>,
    pub stored: Result<(), String>,
}

/// A frontmatter that reads as a model, or is held as what does not read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Missive {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontmatter: Option<Result<Masthead, Unparsed>>,
}

/// A `Result` in each position that holds a value: a type argument, a map's value, a list, an
/// `Option`, the arm of another `Result` and a tuple's position.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Postbox {
    pub filled: Mailbag<Result<u32, String>>,
    pub keyed: BTreeMap<String, Result<u32, String>>,
    pub listed: Vec<Result<u32, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<Result<u32, String>>,
    pub nested: Result<Result<u32, String>, String>,
    pub paired: (Result<u32, String>, String),
}

/// A `Result` as a brand's inner value, which writes no text a `Display` could.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Ruling(pub Result<u32, String>);

/// A `Result` as the target of an alias.
#[model_schema()]
pub type Settlement = Result<u32, String>;

/// A brand over a `Result` and an alias of one, held.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Errand {
    pub ruled: Ruling,
    pub settled: Settlement,
}

/// A `Result` of the type's own parameter.
#[model_schema(default_types(Held = u32))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Attempted<Held> {
    pub outcome: Result<Held, String>,
}

/// A `Result` in a variant's member and in a variant's slot, of the type's own parameter.
#[model_schema(default_types(Held = u32))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Turn<Held> {
    Named { outcome: Result<Held, String> },
    Slot(Result<Held, String>),
}

/// Each shape of a `Result` an arm of which holds `()`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Summary {
    pub both: Result<(), ()>,
    pub err: Result<u32, ()>,
    pub ok: Result<(), String>,
}

/// An untagged enum whose variants a `Result` tells apart: one of a bounded brand, which the bound
/// of the brand chooses, and one of a number.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Bureau {
    Coded { codes: Result<Ballot, u32> },
    Plain { counted: Result<u32, String> },
}

/// A map of `Result`s of a bounded brand, flattened into [`Notebook`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Jacket {
    pub keyed: BTreeMap<String, Result<Ballot, u32>>,
}

/// A `Result` of a list of a model beside a flattened member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Notebook {
    #[serde(flatten)]
    pub jacket: Jacket,
    pub listed: Result<Vec<Masthead>, Unparsed>,
}

/// `Result`s of a model and of a bounded brand in the variants of an internally tagged enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Errata {
    Filed {
        frontmatter: Result<Masthead, Unparsed>,
    },
    Listed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        marks: Option<Vec<Result<Ballot, Unparsed>>>,
    },
}

/// One payload: its name, the type it is read as, and the JSON.
pub struct Payload {
    pub json: String,
    pub name: &'static str,
    pub read_as: &'static str,
}

/// A masthead of `slug` titled `title`.
pub fn masthead(slug: &str, title: &str) -> Masthead {
    Masthead {
        slug: slug.to_owned(),
        title: title.to_owned(),
    }
}

/// What does not read, for `reason`.
pub fn unparsed(reason: &str) -> Unparsed {
    Unparsed {
        reason: reason.to_owned(),
    }
}

/// A ballot of `code`.
pub fn ballot(code: &str) -> Ballot {
    Ballot(code.to_owned())
}

/// The README's reply, which reads.
pub const fn reply() -> Reply {
    Reply {
        outcome: Ok(7),
        stored: Ok(()),
    }
}

/// A missive whose frontmatter reads, and whose two bounds break where `broken` says so.
pub fn missive(broken: bool) -> Missive {
    let (slug, title) = if broken {
        ("far-too-long", "")
    } else {
        ("intro", "Intro")
    };
    Missive {
        frontmatter: Some(Ok(masthead(slug, title))),
    }
}

/// A `Result` in each position, with `maybe` holding one where `held` says so.
pub fn postbox(held: bool) -> Postbox {
    Postbox {
        filled: Mailbag { held: Ok(1) },
        keyed: BTreeMap::from([("a".to_owned(), Err("b".to_owned()))]),
        listed: vec![Ok(2), Err("c".to_owned())],
        maybe: held.then_some(Ok(3)),
        nested: Ok(Err("d".to_owned())),
        paired: (Ok(4), "e".to_owned()),
    }
}

/// A notebook whose every bound breaks where `broken` says so.
pub fn notebook(broken: bool) -> Notebook {
    let (code, title) = if broken { ("a", "") } else { ("ab", "Intro") };
    Notebook {
        jacket: Jacket {
            keyed: BTreeMap::from([("k".to_owned(), Ok(ballot(code)))]),
        },
        listed: Ok(vec![masthead("intro", title)]),
    }
}

/// What serde writes of `value`, read as `read_as`.
fn written<T>(name: &'static str, read_as: &'static str, value: &T) -> Payload
where
    T: Serialize,
{
    Payload {
        json: serde_json::to_string(value).unwrap(),
        name,
        read_as,
    }
}

/// A payload of `read_as`, written as it stands.
fn payload(name: &'static str, read_as: &'static str, json: &str) -> Payload {
    Payload {
        json: json.to_owned(),
        name,
        read_as,
    }
}

/// What serde writes of a `Result` in every shape and position, each of which reads back.
pub fn read_payloads() -> Vec<Payload> {
    vec![
        written("the reply", "Reply", &reply()),
        written(
            "an Err reply",
            "Reply",
            &Reply {
                outcome: Err("late".to_owned()),
                stored: Err("full".to_owned()),
            },
        ),
        written(
            "a frontmatter that does not read",
            "Missive",
            &Missive {
                frontmatter: Some(Err(unparsed("no title"))),
            },
        ),
        written("no frontmatter", "Missive", &Missive { frontmatter: None }),
        written("each position", "Postbox", &postbox(true)),
        written("no maybe", "Postbox", &postbox(false)),
        written(
            "a brand and an alias",
            "Errand",
            &Errand {
                ruled: Ruling(Err("x".to_owned())),
                settled: Ok(5),
            },
        ),
        written(
            "a Result of a parameter",
            "Attempted",
            &Attempted::<u32> { outcome: Ok(8) },
        ),
        written(
            "a Result in a member of a variant",
            "Turn",
            &Turn::<u32>::Named { outcome: Ok(1) },
        ),
        written(
            "a Result in the slot of a variant",
            "Turn",
            &Turn::<u32>::Slot(Err("y".to_owned())),
        ),
        written(
            "arms of unit, Ok",
            "Summary",
            &Summary {
                both: Ok(()),
                err: Err(()),
                ok: Ok(()),
            },
        ),
        written(
            "arms of unit, Err",
            "Summary",
            &Summary {
                both: Err(()),
                err: Ok(3),
                ok: Err("z".to_owned()),
            },
        ),
        written(
            "an untagged variant of a Result of a brand",
            "Bureau",
            &Bureau::Coded {
                codes: Ok(ballot("ab")),
            },
        ),
        written(
            "an untagged variant of a Result of a number",
            "Bureau",
            &Bureau::Plain {
                counted: Err("w".to_owned()),
            },
        ),
        written(
            "an internally tagged variant of no marks",
            "Errata",
            &Errata::Listed { marks: None },
        ),
    ]
}

/// Payloads serde refuses: an object of both keys, of neither, of another key, a value under an
/// arm of `()`, an arm of another type, and no object.
pub fn refused_payloads() -> Vec<Payload> {
    vec![
        payload(
            "both keys",
            "Reply",
            r#"{"outcome":{"Ok":7,"Err":"x"},"stored":{"Ok":null}}"#,
        ),
        payload(
            "neither key",
            "Reply",
            r#"{"outcome":{},"stored":{"Ok":null}}"#,
        ),
        payload(
            "another key",
            "Reply",
            r#"{"outcome":{"Fine":7},"stored":{"Ok":null}}"#,
        ),
        payload(
            "a value under an arm of unit",
            "Reply",
            r#"{"outcome":{"Ok":7},"stored":{"Ok":1}}"#,
        ),
        payload(
            "an arm of another type",
            "Reply",
            r#"{"outcome":{"Ok":"seven"},"stored":{"Ok":null}}"#,
        ),
        payload(
            "no object",
            "Reply",
            r#"{"outcome":7,"stored":{"Ok":null}}"#,
        ),
        payload(
            "a value under an Err of unit",
            "Summary",
            r#"{"both":{"Ok":null},"err":{"Err":1},"ok":{"Ok":null}}"#,
        ),
        payload(
            "an untagged variant no bound of a brand admits",
            "Bureau",
            r#"{"codes":{"Ok":"a"}}"#,
        ),
    ]
}

/// What serde writes of a `Result` of a model and of a brand, sound and with broken bounds, which
/// `validate()` refuses.
pub fn judged_payloads() -> Vec<Payload> {
    vec![
        written("a sound frontmatter", "Missive", &missive(false)),
        written("a broken frontmatter", "Missive", &missive(true)),
        written(
            "a broken Err",
            "Missive",
            &Missive {
                frontmatter: Some(Err(unparsed(""))),
            },
        ),
        written("a sound notebook", "Notebook", &notebook(false)),
        written("a broken notebook", "Notebook", &notebook(true)),
        written(
            "a broken tagged variant",
            "Errata",
            &Errata::Filed {
                frontmatter: Ok(masthead("intro", "")),
            },
        ),
        written(
            "broken marks of a tagged variant",
            "Errata",
            &Errata::Listed {
                marks: Some(vec![Err(unparsed("")), Ok(ballot("a"))]),
            },
        ),
    ]
}
