//! A read judges each model once, and a refused read names every broken model at its full path,
//! wherever the member that holds the model names one of the holder's type parameters or `Self`.

use core::fmt::Debug;

use serde::de::DeserializeOwned;

use super::models::{
    Book, BoundedPair, Corrected, DefaultedPair, DefaultedTree, DisplayedLedger, Entry, Index,
    Ledger, Node, Pair, Posting, Shelf, StringLedger, Tag, TiedPair, Tree, Wrapped,
    corrections_taken, taken,
};

/// A ledger of two sound entries and two tags.
const LEDGER: &str = r#"{"entries":[{"id":"a","label":"x"},{"id":"b","label":"y"}],"tags":[{"name":"x"},{"name":"y"}]}"#;

/// A ledger whose second and third entries have no label.
const BROKEN_LEDGER: &str =
    r#"{"entries":[{"id":"a","label":"x"},{"id":"b","label":""},{"id":"c","label":""}],"tags":[]}"#;

/// What a holder of [`BROKEN_LEDGER`]'s entries is refused with.
const BROKEN_ENTRIES: &str =
    "'entries.1.label': an entry has a label; 'entries.2.label': an entry has a label";

/// Two sound entries, as a holder writes them under `entries`.
const ENTRIES: &str = r#"{"entries":[{"id":"a","label":"x"},{"id":"b","label":"y"}],"held":[]}"#;

/// The checks `text` runs as it is read as a `T`, which it must be.
fn checks_of_reading<T>(text: &str) -> [usize; 3]
where
    T: DeserializeOwned,
{
    taken();
    serde_json::from_str::<T>(text).unwrap();
    taken()
}

/// What `text` is refused with as it is read as a `T`.
fn refusal_of_reading<T>(text: &str) -> String
where
    T: Debug + DeserializeOwned,
{
    serde_json::from_str::<T>(text).unwrap_err().to_string()
}

fn entry(id: &str, label: &str) -> Entry<String> {
    Entry {
        id: id.to_owned(),
        label: label.to_owned(),
    }
}

#[test]
fn a_model_held_under_the_holders_type_parameter_is_judged_once() {
    assert_eq!(
        checks_of_reading::<Ledger<String>>(LEDGER),
        [2, 2, 0],
        "each entry and each tag is judged once"
    );
    assert_eq!(
        checks_of_reading::<Book<String>>(&format!(r#"{{"ledgers":[{LEDGER}]}}"#)),
        [2, 2, 0],
        "a level up, still once"
    );
}

#[test]
fn a_model_held_under_self_is_judged_once() {
    assert_eq!(
        checks_of_reading::<Node>(r#"{"children":[{"children":[{"children":[]}]}]}"#),
        [0, 0, 3],
        "each node of the chain is judged once"
    );
}

#[test]
fn a_broken_model_held_under_the_holders_type_parameter_is_named_at_its_full_path() {
    assert_eq!(
        refusal_of_reading::<Ledger<String>>(BROKEN_LEDGER),
        BROKEN_ENTRIES
    );
    assert_eq!(
        refusal_of_reading::<Book<String>>(&format!(r#"{{"ledgers":[{BROKEN_LEDGER}]}}"#)),
        "'ledgers.0.entries.1.label': an entry has a label; \
         'ledgers.0.entries.2.label': an entry has a label"
    );
}

#[test]
fn a_model_held_through_every_wrapper_is_judged_once_and_named_at_its_full_path() {
    let sound = Wrapped {
        boxed: Box::new(entry("a", "x")),
        by_key: [("north".to_owned(), entry("b", "x"))].into(),
        listed: vec![Some(entry("c", "x")), None, Some(entry("d", "x"))],
        spare: Some(entry("e", "x")),
    };
    let written = serde_json::to_string(&sound).unwrap();
    assert_eq!(checks_of_reading::<Wrapped<String>>(&written), [5, 0, 0]);
    assert_eq!(
        serde_json::from_str::<Wrapped<String>>(&written).unwrap(),
        sound
    );

    let broken = Wrapped {
        boxed: Box::new(entry("a", "")),
        by_key: [("north".to_owned(), entry("b", ""))].into(),
        listed: vec![Some(entry("c", "x")), None, Some(entry("d", ""))],
        spare: Some(entry("e", "")),
    };
    let refused = refusal_of_reading::<Wrapped<String>>(&serde_json::to_string(&broken).unwrap());
    assert_eq!(
        refused,
        "'boxed.label': an entry has a label; 'byKey.north.label': an entry has a label; \
         'listed.2.label': an entry has a label; 'spare.label': an entry has a label"
    );
    assert_eq!(refused, broken.validate().unwrap_err().join("; "));
}

#[test]
fn a_parameter_named_by_a_maps_key_alone_reads_its_values_unjudged() {
    assert_eq!(
        checks_of_reading::<Index<String>>(r#"{"tags":{"a":{"name":"x"},"b":{"name":"y"}}}"#),
        [0, 2, 0]
    );
}

#[test]
fn a_member_that_names_one_of_two_parameters_reads_its_model_unjudged() {
    assert_eq!(checks_of_reading::<Pair<String, u32>>(ENTRIES), [2, 0, 0]);
    assert_eq!(
        refusal_of_reading::<Pair<String, u32>>(&BROKEN_LEDGER.replace("tags", "held")),
        BROKEN_ENTRIES
    );
}

#[test]
fn a_container_default_does_not_keep_a_member_from_the_unjudged_read() {
    assert_eq!(
        checks_of_reading::<DefaultedPair<String, String>>(ENTRIES),
        [2, 0, 0]
    );
    assert_eq!(
        refusal_of_reading::<DefaultedPair<String, String>>(&BROKEN_LEDGER.replace("tags", "held")),
        BROKEN_ENTRIES
    );
    let defaulted = serde_json::from_str::<DefaultedPair<String, String>>("{}").unwrap();
    assert_eq!(defaulted, DefaultedPair::default());
}

#[test]
fn a_bound_written_on_the_item_does_not_keep_a_member_from_the_unjudged_read() {
    assert_eq!(
        checks_of_reading::<BoundedPair<String, String>>(ENTRIES),
        [2, 0, 0]
    );
    assert_eq!(
        refusal_of_reading::<BoundedPair<String, String>>(&BROKEN_LEDGER.replace("tags", "held")),
        BROKEN_ENTRIES
    );
}

#[test]
fn a_type_that_holds_itself_under_a_container_default_is_judged_once_per_node() {
    assert_eq!(
        checks_of_reading::<DefaultedTree<u32>>(
            r#"{"id":1,"children":[{"id":2,"children":[{"id":3,"children":[]}]}]}"#
        ),
        [0, 0, 3]
    );
    let defaulted = serde_json::from_str::<DefaultedTree<u32>>("{}").unwrap();
    assert_eq!(defaulted, DefaultedTree::default());
}

#[test]
fn a_parameter_bounded_where_it_is_declared_reads_its_model_unjudged() {
    assert_eq!(
        checks_of_reading::<DisplayedLedger<String>>(ENTRIES),
        [2, 0, 0]
    );
    assert_eq!(
        refusal_of_reading::<DisplayedLedger<String>>(BROKEN_LEDGER),
        BROKEN_ENTRIES
    );
}

/// A bound that ties the parameter to another leaves the member read as it was: each entry is
/// judged as it is read, and the first broken one ends the read with its own message.
#[test]
fn a_member_whose_parameter_a_bound_ties_to_another_is_read_as_it_was() {
    assert_eq!(
        checks_of_reading::<TiedPair<String, String>>(ENTRIES),
        [4, 0, 0]
    );
    assert_eq!(
        refusal_of_reading::<TiedPair<String, String>>(&BROKEN_LEDGER.replace("tags", "held")),
        "'label': an entry has a label at line 1 column 57"
    );
}

#[test]
fn a_type_with_a_parameter_that_holds_itself_is_judged_once_per_node() {
    assert_eq!(
        checks_of_reading::<Tree<u32>>(
            r#"{"id":1,"children":[{"id":2,"children":[{"id":3,"children":[]}]}]}"#
        ),
        [0, 0, 3]
    );
}

#[test]
fn a_member_that_names_a_preprocess_is_corrected_and_judged_once() {
    corrections_taken();
    assert_eq!(
        checks_of_reading::<Corrected<String>>(r#"{"entries":{"id":"a","label":"x"}}"#),
        [1, 0, 0]
    );
    assert_eq!(corrections_taken(), 1);
    assert_eq!(
        refusal_of_reading::<Corrected<String>>(BROKEN_LEDGER),
        BROKEN_ENTRIES
    );
    assert_eq!(corrections_taken(), 1);
}

#[test]
fn a_concrete_filling_is_read_as_it_was() {
    assert_eq!(checks_of_reading::<StringLedger>(ENTRIES), [2, 0, 0]);
    assert_eq!(
        refusal_of_reading::<StringLedger>(BROKEN_LEDGER),
        BROKEN_ENTRIES
    );
}

#[test]
fn what_fills_a_parameter_is_judged_once_by_its_own_read() {
    assert_eq!(
        checks_of_reading::<Shelf<Tag>>(r#"{"held":[{"name":"x"},{"name":"y"}]}"#),
        [0, 2, 0]
    );
}

#[test]
fn a_variant_that_holds_a_model_under_the_enums_parameter_reads_it_unjudged() {
    assert_eq!(
        checks_of_reading::<Posting<String>>(
            r#"{"kind":"Listed","entries":[{"id":"a","label":"x"},{"id":"b","label":"y"}]}"#
        ),
        [2, 0, 0]
    );
    assert_eq!(
        checks_of_reading::<Posting<String>>(r#"{"kind":"Single","id":"a","label":"x"}"#),
        [1, 0, 0]
    );
    assert_eq!(
        refusal_of_reading::<Posting<String>>(
            r#"{"kind":"Listed","entries":[{"id":"a","label":""},{"id":"b","label":""}]}"#
        ),
        "'entries.0.label': an entry has a label; 'entries.1.label': an entry has a label"
    );
}

/// `validate()` still walks every member of a value built in code, which nothing judged before.
#[test]
fn a_value_built_in_code_is_judged_by_validate() {
    let ledger = Ledger {
        entries: vec![entry("a", ""), entry("b", "x")],
        tags: vec![Tag {
            name: "x".to_owned(),
        }],
    };
    taken();
    assert_eq!(
        ledger.validate(),
        Err(vec!["'entries.0.label': an entry has a label".to_owned()])
    );
    assert_eq!(taken(), [2, 1, 0]);
}
