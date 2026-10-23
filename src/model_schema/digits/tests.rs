use syn::Item;

use super::{DigitMember, members};

/// The members of `source` whose integers are written as digit strings, each as its place and
/// the directions serde writes and reads it in.
fn found(source: &str) -> Vec<(Option<String>, usize, bool, bool)> {
    let item: Item = syn::parse_str(source).unwrap();
    members(&item)
        .into_iter()
        .map(|member: DigitMember| (member.variant, member.index, member.writes, member.reads))
        .collect()
}

/// Every position a member holds a `u64` or an `i64` in through a std wrapper, a list, a set, an
/// array, a map's value or a tuple is written as digits, and so is a member's own type through a
/// lock.
#[test]
fn each_position_the_macro_takes_apart_is_found() {
    for written in [
        "u64",
        "i64",
        "Option<i64>",
        "Box<u64>",
        "Arc<i64>",
        "Box<[i64]>",
        "Vec<Option<i64>>",
        "VecDeque<u64>",
        "HashSet<i64>",
        "BTreeSet<u64>",
        "BinaryHeap<i64>",
        "[i64; 3]",
        "HashMap<String, Vec<i64>>",
        "BTreeMap<u32, i64>",
        "(i64, String)",
        "Option<(String, Vec<u64>)>",
        "Cell<i64>",
        "Vec<Cell<Option<u64>>>",
        "Cow<'static, i64>",
        "Cow<'static, [u64]>",
        "RefCell<i64>",
        "Mutex<Option<i64>>",
        "Arc<RwLock<Vec<u64>>>",
    ] {
        assert_eq!(
            found(&format!(
                "#[derive(Serialize, Deserialize)] pub struct Wide {{ pub held: {written} }}"
            )),
            [(None, 0, true, true)],
            "for {written}"
        );
    }
}

/// A key, a range's bound, a `Result`'s arm, another type's argument and a name that stands for
/// one are written as their own types write them, and a level no written function takes apart,
/// a reference and a lock below another level, keeps the whole member a number.
#[test]
fn a_position_serde_writes_through_another_impl_is_not_found() {
    for written in [
        "u32",
        "BTreeMap<u64, String>",
        "Range<i64>",
        "Result<i64, String>",
        "Wrapper<i64>",
        "Count",
        "Vec<Mutex<i64>>",
        "Option<RefCell<i64>>",
        "(Mutex<i64>, String)",
        "&'static [i64]",
        "Vec<(u64, T)>",
        "Vec<(u64, Box<Self>)>",
    ] {
        assert_eq!(
            found(&format!(
                "#[derive(Serialize, Deserialize)] pub struct Wide<T> {{ pub held: {written}, \
                 pub marker: PhantomData<T> }}"
            )),
            [],
            "for {written}"
        );
    }
}

/// serde writes and reads the member only in the directions its derives and its attributes leave
/// it, and a member its author writes or reads through a function of their own is theirs.
#[test]
fn the_directions_follow_the_derives_and_the_attributes() {
    assert_eq!(
        found("#[derive(Serialize)] pub struct Wide { pub held: i64 }"),
        [(None, 0, true, false)]
    );
    assert_eq!(
        found("#[derive(Deserialize)] pub struct Wide { pub held: i64 }"),
        [(None, 0, false, true)]
    );
    assert_eq!(found("pub struct Wide { pub held: i64 }"), []);
    assert_eq!(
        found(
            "#[derive(Serialize, Deserialize)] pub struct Wide { #[serde(skip_serializing)] pub \
             held: i64, #[serde(skip)] pub gone: i64, #[serde(with = \"mine\")] pub own: i64 }"
        ),
        [(None, 0, false, true)]
    );
    assert_eq!(
        found(
            "#[derive(Serialize, Deserialize)] #[serde(into = \"Other\")] pub struct Wide { pub \
             held: i64 }"
        ),
        [(None, 0, false, true)]
    );
    assert_eq!(
        found(
            "#[derive(Serialize, Deserialize)] pub struct Wide { #[model_schema_prop(literal = \
             7)] pub stated: i64, #[model_schema_prop(as = String)] pub named: u64 }"
        ),
        []
    );
}

/// Every member of an enum is found by its variant and its place, a variant's own attributes
/// read as a member's.
#[test]
fn each_member_of_an_enum_is_found_by_its_variant() {
    assert_eq!(
        found(
            "#[derive(Serialize, Deserialize)] pub enum Wide { Held(i64), Named { kept: String, \
             count: u64 }, #[serde(skip_deserializing)] Written(u64), #[serde(with = \"mine\")] \
             Own(i64), Empty }"
        ),
        [
            (Some("Held".to_owned()), 0, true, true),
            (Some("Named".to_owned()), 1, true, true),
            (Some("Written".to_owned()), 0, true, false),
        ]
    );
}
