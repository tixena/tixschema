//! A flagged type serde writes as one value: a brand, a tuple struct of one slot, a plain enum, a
//! unit struct. Its struct of paths is the path of that value. A member typed with one keeps the
//! operators of a plain value, bare and under an `Option`, and a list of one is a list of plain
//! values. Its module names no type of its author's, so one declared inside a function builds.

use bson::{Bson, Document};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::shown;
use super::stored::looked_up;

/// A brand over a number.
#[model_schema(decode_with)]
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(transparent)]
struct Score(u32);

/// A tuple struct of one slot, which serde writes as the value it holds with no attribute.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Nickname(String);

#[model_schema(decode_with)]
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Rank {
    Rookie,
    TopSeed,
}

/// A brand over whatever fills it.
#[model_schema(decode_with, default_types(T = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
struct Sealed<T>(T);

/// Holds each of the four above bare or under an `Option`, and in a list.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Game {
    best: Score,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last: Option<Score>,
    nicknames: Vec<Nickname>,
    past_ranks: Vec<Rank>,
    rank: Rank,
    scores: Vec<Score>,
    seal: Sealed<String>,
    seals: Vec<Sealed<u32>>,
}

/// The struct of paths of [`Score`] in a row of [`Game`], as [`Game`] writes it: the path is the
/// struct's last parameter.
type ScorePaths = score_schema::MongoFields<Game, score_schema::Field<Game, Score>>;

/// The struct of paths of a [`Sealed`] text in a row of [`Game`]: what the type is written with
/// comes before the path.
type SealPaths =
    sealed_schema::MongoFields<Game, String, sealed_schema::Field<Game, Sealed<String>>>;

fn game() -> Game {
    Game {
        best: Score(84_u32),
        last: Some(Score(72_u32)),
        nicknames: vec![Nickname("ace".to_owned())],
        past_ranks: vec![Rank::Rookie],
        rank: Rank::TopSeed,
        scores: vec![Score(72_u32), Score(84_u32)],
        seal: Sealed("wax".to_owned()),
        seals: vec![Sealed(3_u32)],
    }
}

/// Every key of `value`, at every depth.
fn keys(value: &Bson) -> Vec<String> {
    if let Bson::Document(document) = value {
        document
            .iter()
            .flat_map(|(key, held)| {
                let mut below = keys(held);
                below.push(key.clone());
                below
            })
            .collect()
    } else if let Bson::Array(items) = value {
        items.iter().flat_map(keys).collect()
    } else {
        Vec::new()
    }
}

#[test]
fn a_list_of_a_type_written_as_one_value_is_matched_through_its_elements() {
    let game = Game::MONGO_FIELDS;
    let scores: &game_schema::ListField<Game, Score> = &game.scores;
    assert_eq!(
        shown(scores.elem_match(scores.element().gt(Score(5_u32)).unwrap())),
        r#"{ "scores": { "$elemMatch": { "$gt": Int64(5) } } }"#
    );
    assert_eq!(
        shown(scores.contains(Score(84_u32)).unwrap()),
        r#"{ "scores": { "$eq": Int64(84) } }"#
    );
    let nicknames: &game_schema::ListField<Game, Nickname> = &game.nicknames;
    assert_eq!(
        shown(nicknames.elem_match(nicknames.element().eq(Nickname("ace".to_owned())).unwrap())),
        r#"{ "nicknames": { "$elemMatch": { "$eq": "ace" } } }"#
    );
    let past_ranks: &game_schema::ListField<Game, Rank> = &game.past_ranks;
    assert_eq!(
        shown(past_ranks.contains_none([Rank::TopSeed]).unwrap()),
        r#"{ "pastRanks": { "$nin": ["top-seed"] } }"#
    );
}

#[test]
fn bare_and_under_an_option_it_keeps_the_path_of_its_own_type() {
    let game = Game::MONGO_FIELDS;
    let best: &game_schema::Model<Game, Score, ScorePaths> = &game.best;
    assert_eq!(
        shown(best.gt(Score(80_u32)).unwrap()),
        r#"{ "best": { "$gt": Int64(80) } }"#
    );
    let last: &game_schema::OptionalModel<Game, Score, ScorePaths> = &game.last;
    assert_eq!(
        shown(last.lte(Score(72_u32)).unwrap()),
        r#"{ "last": { "$lte": Int64(72) } }"#
    );
    assert_eq!(
        shown(last.exists(false)),
        r#"{ "last": { "$exists": false } }"#
    );
    assert_eq!(
        shown(game.rank.eq(Rank::TopSeed).unwrap()),
        r#"{ "rank": { "$eq": "top-seed" } }"#
    );
}

/// A generic one's struct of paths carries what the type is written with, then the path.
#[test]
fn a_generic_one_is_held_with_its_arguments_and_its_path() {
    let game = Game::MONGO_FIELDS;
    let seal: &game_schema::Model<Game, Sealed<String>, SealPaths> = &game.seal;
    assert_eq!(
        shown(seal.ne(Sealed("lead".to_owned())).unwrap()),
        r#"{ "seal": { "$ne": "lead" } }"#
    );
    let seals: &game_schema::ListField<Game, Sealed<u32>> = &game.seals;
    assert_eq!(
        shown(seals.elem_match(seals.element().lt(Sealed(9_u32)).unwrap())),
        r#"{ "seals": { "$elemMatch": { "$lt": Int64(9) } } }"#
    );
    let whole: sealed_schema::MongoFields<
        Sealed<u32>,
        u32,
        sealed_schema::Field<Sealed<u32>, Sealed<u32>>,
    > = Sealed::<u32>::MONGO_FIELDS;
    assert_eq!(
        shown(whole.gte(Sealed(1_u32)).unwrap()),
        r#"{ "": { "$gte": Int64(1) } }"#
    );
}

#[test]
fn no_operator_of_a_list_of_one_writes_an_empty_key() {
    let game = Game::MONGO_FIELDS;
    let written: Vec<Document> = vec![
        game.scores
            .elem_match(game.scores.element().gt(Score(5_u32)).unwrap())
            .into(),
        game.scores.contains(Score(84_u32)).unwrap().into(),
        game.scores.contains_any([Score(1_u32)]).unwrap().into(),
        game.scores.contains_none([Score(1_u32)]).unwrap().into(),
        game.scores.size(2).into(),
        game.scores.push(Score(90_u32)).unwrap().into(),
        game.scores.pull(Score(72_u32)).unwrap().into(),
        game.scores.set([Score(1_u32)]).unwrap().into(),
        game.scores.set_on_insert([Score(1_u32)]).unwrap().into(),
        game.past_ranks
            .elem_match(game.past_ranks.element().eq(Rank::Rookie).unwrap())
            .into(),
        game.nicknames
            .contains(Nickname("ace".to_owned()))
            .unwrap()
            .into(),
    ];
    for document in written {
        let held = keys(&Bson::Document(document.clone()));
        assert!(
            held.iter().all(|key| !key.is_empty()),
            "an empty key in {}",
            shown(document)
        );
    }
}

#[test]
fn each_path_is_found_in_what_serde_stores_for_the_row() {
    let row = game().serialize(bson::Serializer::new()).unwrap();
    let stored = row.as_document().unwrap();
    let game = Game::MONGO_FIELDS;
    let held = [
        looked_up(stored, game.best.eq(Score(0_u32)).unwrap()),
        looked_up(stored, game.last.exists(true)),
        looked_up(stored, game.nicknames.size(1)),
        looked_up(stored, game.past_ranks.size(1)),
        looked_up(stored, game.rank.eq(Rank::Rookie).unwrap()),
        looked_up(stored, game.scores.size(2)),
        looked_up(stored, game.seal.eq(Sealed(String::new())).unwrap()),
        looked_up(stored, game.seals.size(1)),
    ];
    let expected = [
        ("best", "Int64(84)"),
        ("last", "Int64(72)"),
        ("nicknames", r#"["ace"]"#),
        ("pastRanks", r#"["rookie"]"#),
        ("rank", r#""top-seed""#),
        ("scores", "[Int64(72), Int64(84)]"),
        ("seal", r#""wax""#),
        ("seals", "[Int64(3)]"),
    ]
    .map(|(key, value)| (key.to_owned(), value.to_owned()));
    assert_eq!(held, expected, "stored: {}", shown(stored.clone()));
}

#[test]
fn a_plain_enum_declared_inside_a_function_builds() {
    #[model_schema(decode_with)]
    #[derive(Clone, Copy, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "kebab-case")]
    enum Standing {
        OnHold,
        Settled,
    }

    assert_eq!(
        shown(
            Standing::MONGO_FIELDS
                .is_in([Standing::OnHold, Standing::Settled])
                .unwrap()
        ),
        r#"{ "": { "$in": ["on-hold", "settled"] } }"#
    );
}

#[test]
fn a_brand_declared_inside_a_function_builds() {
    #[model_schema(decode_with)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(transparent)]
    struct Callsign(String);

    assert_eq!(
        shown(
            Callsign::MONGO_FIELDS
                .eq(Callsign("maverick".to_owned()))
                .unwrap()
        ),
        r#"{ "": { "$eq": "maverick" } }"#
    );
    assert_eq!(
        shown(Callsign::MONGO_FIELDS.regex("^mav", "i")),
        r#"{ "": { "$regex": "^mav", "$options": "i" } }"#
    );
}

#[test]
fn a_tuple_struct_of_one_slot_declared_inside_a_function_builds() {
    #[model_schema(decode_with)]
    #[derive(Clone, Copy, Debug, Deserialize, Serialize)]
    struct Altitude(u32);

    assert_eq!(
        shown(Altitude::MONGO_FIELDS.gt(Altitude(9000_u32)).unwrap()),
        r#"{ "": { "$gt": Int64(9000) } }"#
    );
}

#[test]
fn a_unit_struct_declared_inside_a_function_builds() {
    #[model_schema(decode_with)]
    #[derive(Clone, Copy, Debug, Deserialize, Serialize)]
    struct Grounded;

    assert_eq!(
        shown(Grounded::MONGO_FIELDS.eq(Grounded).unwrap()),
        r#"{ "": { "$eq": {} } }"#
    );
}
