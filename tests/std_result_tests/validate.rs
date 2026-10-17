//! `validate()` of a type that holds a `Result` of a model or of a bounded brand: the arm the value
//! holds is walked, each report under the key serde writes the arm under. The read refuses what
//! `validate()` refuses, the model in an arm read unjudged so that the whole value is judged once.

use super::models::{
    Errata, Missive, Notebook, Reply, ballot, masthead, missive, notebook, unparsed,
};

/// What `validate()` answers of a broken [`Missive`].
pub const MISSIVE_REPORTS: [&str; 2] = [
    "'frontmatter.Ok.slug': too long: maximum length is 8, got 12",
    "'frontmatter.Ok.title': too short: minimum length is 1, got 0",
];

/// What `validate()` answers of a broken [`Notebook`].
pub const NOTEBOOK_REPORTS: [&str; 2] = [
    "'keyed.k.Ok': too short: minimum length is 2, got 1",
    "'listed.Ok.0.title': too short: minimum length is 1, got 0",
];

#[test]
fn the_arm_a_result_holds_is_reported_under_its_key() {
    assert_eq!(
        missive(true).validate(),
        Err(MISSIVE_REPORTS.map(str::to_owned).to_vec())
    );
    assert_eq!(
        Missive {
            frontmatter: Some(Err(unparsed(""))),
        }
        .validate(),
        Err(vec![
            "'frontmatter.Err.reason': too short: minimum length is 1, got 0".to_owned()
        ])
    );
    assert_eq!(
        notebook(true).validate(),
        Err(NOTEBOOK_REPORTS.map(str::to_owned).to_vec())
    );
    assert_eq!(
        Errata::Listed {
            marks: Some(vec![Err(unparsed("")), Ok(ballot("a"))]),
        }
        .validate(),
        Err(vec![
            "'marks.0.Err.reason': too short: minimum length is 1, got 0".to_owned(),
            "'marks.1.Ok': too short: minimum length is 2, got 1".to_owned(),
        ])
    );
    assert_eq!(missive(false).validate(), Ok(()));
    assert_eq!(notebook(false).validate(), Ok(()));
    assert_eq!(
        Errata::Filed {
            frontmatter: Ok(masthead("intro", "Intro")),
        }
        .validate(),
        Ok(())
    );
}

#[test]
fn the_read_refuses_with_each_message_once() {
    let missive_read =
        serde_json::from_str::<Missive>(&serde_json::to_string(&missive(true)).unwrap())
            .map_err(|error| error.to_string());
    assert_eq!(missive_read, Err(MISSIVE_REPORTS.join("; ")));
    let notebook_read =
        serde_json::from_str::<Notebook>(&serde_json::to_string(&notebook(true)).unwrap())
            .map_err(|error| error.to_string());
    assert_eq!(notebook_read, Err(NOTEBOOK_REPORTS.join("; ")));
    let sound = serde_json::to_string(&missive(false)).unwrap();
    assert_eq!(
        serde_json::from_str::<Missive>(&sound).unwrap(),
        missive(false)
    );
}

#[test]
fn a_result_of_no_model_reads_as_serde_writes_it() {
    let reply = super::models::reply();
    let read: Reply = serde_json::from_str(&serde_json::to_string(&reply).unwrap()).unwrap();
    assert_eq!(read, reply);
}
