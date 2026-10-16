//! `validate()` of a type that holds a range of a model or of a bounded brand: each bound is
//! walked and reported under the key serde writes it under. The read refuses what `validate()`
//! refuses, the model in a range read unjudged so that the whole value is judged once.

use super::models::{Frontier, Signposted, Spanned, Stride, frontier, signposted, spanned};

/// What `validate()` answers of a broken [`Signposted`].
pub const SIGNPOSTED_REPORTS: [&str; 3] = [
    "'marks.start.mark': too short: minimum length is 1, got 0",
    "'span.start': too short: minimum length is 2, got 1",
    "'span.end': too short: minimum length is 2, got 1",
];

/// What `validate()` answers of a broken [`Frontier`].
pub const FRONTIER_REPORTS: [&str; 2] = [
    "'from.start': too short: minimum length is 2, got 1",
    "'to.end': too short: minimum length is 2, got 1",
];

#[test]
fn each_bound_of_a_range_is_reported_under_its_key() {
    assert_eq!(
        signposted(true).validate(),
        Err(SIGNPOSTED_REPORTS.map(str::to_owned).to_vec())
    );
    assert_eq!(
        frontier(true).validate(),
        Err(FRONTIER_REPORTS.map(str::to_owned).to_vec())
    );
    assert_eq!(signposted(false).validate(), Ok(()));
    assert_eq!(frontier(false).validate(), Ok(()));
}

#[test]
fn the_read_refuses_what_validate_refuses_and_judges_the_value_once() {
    for (json, reports) in [
        (
            serde_json::to_string(&signposted(true)).unwrap(),
            SIGNPOSTED_REPORTS.join("; "),
        ),
        (
            serde_json::to_string(&frontier(true)).unwrap(),
            FRONTIER_REPORTS.join("; "),
        ),
    ] {
        let refused = if json.contains("marks") {
            serde_json::from_str::<Signposted>(&json).map(|_| ())
        } else {
            serde_json::from_str::<Frontier>(&json).map(|_| ())
        };
        assert_eq!(refused.map_err(|error| error.to_string()), Err(reports));
    }
    let sound = serde_json::to_string(&signposted(false)).unwrap();
    assert_eq!(
        serde_json::from_str::<Signposted>(&sound).unwrap(),
        signposted(false)
    );
}

#[test]
fn a_range_of_a_number_or_a_parameter_reads_as_serde_writes_it() {
    let spans: Spanned = serde_json::from_str(&serde_json::to_string(&spanned()).unwrap()).unwrap();
    assert_eq!(spans, spanned());
    let stride = Stride { span: 8_u32..13 };
    let strides: Stride<u32> =
        serde_json::from_str(&serde_json::to_string(&stride).unwrap()).unwrap();
    assert_eq!(strides, stride);
}
