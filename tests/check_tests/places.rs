//! One type per place serde writes a member at, each with a bound on the member and a model held
//! in it. Each case is a value built here, with the payload serde writes for it and what
//! `validate()` answers of it, which names every value by the steps serde writes on the way to it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::{Case, case};

/// What every side prints for the places: the Rust `validate()` and the Zod schema alike.
pub const PLACES: &str = "\
Package, unlabelled: refused
    'itemLabel': too short: minimum length is 1, got 0
Package: passes
Docket, empty and numbered zero: refused
    '0': too short: minimum length is 1, got 0
    '1': too small: minimum is 1, got 0
Docket: passes
Bundle, unnamed and unlabelled: refused
    '0': too short: minimum length is 1, got 0
    '1.itemLabel': too short: minimum length is 1, got 0
Bundle: passes
Measured, unnamed and unlabelled after a skipped slot: refused
    '0': too short: minimum length is 1, got 0
    '1.itemLabel': too short: minimum length is 1, got 0
Measured: passes
Lone, empty: refused
    too short: minimum length is 1, got 0
Lone: passes
Strapped, an unlabelled parcel: refused
    'itemLabel': too short: minimum length is 1, got 0
Strapped: passes
Crane, an unlabelled strapped load: refused
    'strappedLoad.itemLabel': too short: minimum length is 1, got 0
Crane: passes
Hoist, an empty code: refused
    'code': too short: minimum length is 1, got 0
Hoist: passes
Freight, boxed unlabelled: refused
    'Boxed.itemLabel': too short: minimum length is 1, got 0
Freight, crated unlabelled: refused
    'Crated.cratedParcel.itemLabel': too short: minimum length is 1, got 0
Freight, loose and unlabelled: refused
    'Loose.freightLabel': too short: minimum length is 1, got 0
Freight, split unnamed and unlabelled: refused
    'Split.0': too short: minimum length is 1, got 0
    'Split.1.itemLabel': too short: minimum length is 1, got 0
Freight, stacked with one unlabelled: refused
    'Stacked.1.itemLabel': too short: minimum length is 1, got 0
Freight, stacked three high: refused
    'Stacked': a stack is at most two parcels high
Freight, boxed: passes
Seal, stamped with nothing: refused
    'Stamped': too short: minimum length is 1, got 0
Seal, signed by no one: refused
    'Signed.signedBy': too short: minimum length is 1, got 0
Seal, stamped: passes
Haul, boxed unlabelled: refused
    'held.itemLabel': too short: minimum length is 1, got 0
Haul, crated unlabelled: refused
    'held.cratedParcel.itemLabel': too short: minimum length is 1, got 0
Haul, loose and unlabelled: refused
    'held.haulLabel': too short: minimum length is 1, got 0
Haul, split unnamed and unlabelled: refused
    'held.0': too short: minimum length is 1, got 0
    'held.1.itemLabel': too short: minimum length is 1, got 0
Haul, stamped with nothing: refused
    'held': too short: minimum length is 1, got 0
Haul, boxed: passes
Lift, boxed unlabelled: refused
    'itemLabel': too short: minimum length is 1, got 0
Lift, crated unlabelled: refused
    'cratedParcel.itemLabel': too short: minimum length is 1, got 0
Lift, loose and unlabelled: refused
    'liftLabel': too short: minimum length is 1, got 0
Lift, boxed: passes
Load, labelled with nothing: refused
    'loadLabel': too short: minimum length is 1, got 0
    'loadParcels.1.itemLabel': too short: minimum length is 1, got 0
Load, an empty text: refused
    too short: minimum length is 1, got 0
Load, an unlabelled whole: refused
    'itemLabel': too short: minimum length is 1, got 0
Load, a whole: passes
Mooring, anchored with no code: refused
    'atAnchor.anchorCode': too short: minimum length is 1, got 0
Mooring, an unlabelled parcel in dry dock: refused
    'dry_dock.itemLabel': too short: minimum length is 1, got 0
Mooring, in dry dock: passes
Departure, by road from an unnamed origin: refused
    'ByRoad.originName': too short: minimum length is 1, got 0
Departure, by road: passes
";

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    #[model_schema_prop(minLength = 1)]
    pub item_label: String,
}

/// A struct of several slots, which serde writes as an array.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Docket(
    #[model_schema_prop(minLength = 1)] pub String,
    #[model_schema_prop(minimum = 1)] pub u32,
);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bundle(#[model_schema_prop(minLength = 1)] pub String, pub Package);

/// A struct whose first slot serde skips: the slots after it move up in the array.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Measured(
    #[serde(skip)] pub u8,
    #[model_schema_prop(minLength = 1)] pub String,
    pub Package,
);

/// A struct of one slot, which serde writes as the slot's value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lone(#[model_schema_prop(minLength = 1)] pub String);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Strapped(pub Package);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Crane {
    pub strapped_load: Strapped,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hoist {
    pub code: Lone,
}

/// An enum serde writes under its variant's name.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all_fields = "camelCase")]
pub enum Freight {
    Boxed(Package),
    Crated {
        crated_parcel: Package,
    },
    Loose {
        #[model_schema_prop(minLength = 1)]
        freight_label: String,
    },
    Split(#[model_schema_prop(minLength = 1)] String, Package),
    Stacked(#[model_schema_prop(check = ["stackedLow"])] Vec<Package>),
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all_fields = "camelCase")]
pub enum Seal {
    Signed {
        #[model_schema_prop(minLength = 1)]
        signed_by: String,
    },
    Stamped(#[model_schema_prop(minLength = 1)] String),
}

/// An enum under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held", rename_all_fields = "camelCase")]
pub enum Haul {
    Boxed(Package),
    Crated {
        crated_parcel: Package,
    },
    Loose {
        #[model_schema_prop(minLength = 1)]
        haul_label: String,
    },
    Split(#[model_schema_prop(minLength = 1)] String, Package),
    Stamped(#[model_schema_prop(minLength = 1)] String),
}

/// An enum under a tag alone, which serde writes beside what the variant holds.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
pub enum Lift {
    Boxed(Package),
    Crated {
        crated_parcel: Package,
    },
    Loose {
        #[model_schema_prop(minLength = 1)]
        lift_label: String,
    },
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, rename_all_fields = "camelCase")]
pub enum Load {
    Labelled {
        #[model_schema_prop(minLength = 1)]
        load_label: String,
        load_parcels: Vec<Package>,
    },
    Text(#[model_schema_prop(minLength = 1)] String),
    Whole(Package),
}

/// A variant under a name of its own, and one under the casing of its enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum Mooring {
    #[serde(rename = "atAnchor")]
    Anchored {
        #[model_schema_prop(minLength = 1)]
        anchor_code: String,
    },
    DryDock(Package),
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Origin {
    #[model_schema_prop(minLength = 1)]
    pub origin_name: String,
}

/// A variant that writes the members of what it flattens as its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Departure {
    ByRoad {
        #[serde(flatten)]
        starting_origin: Origin,
    },
    Held,
}

fn stacked_low(value: &[Package]) -> Result<(), Vec<String>> {
    if value.len() > 2 {
        return Err(vec!["a stack is at most two parcels high".to_owned()]);
    }
    Ok(())
}

fn sound() -> Package {
    PackageDraft {
        item_label: "A".to_owned(),
    }
    .validate()
    .unwrap()
}

fn unlabelled() -> Package {
    PackageDraft {
        item_label: String::new(),
    }
    .validate()
    .unwrap()
}

/// The cases of the structs: a named field, a slot among several, and the one slot of a struct,
/// held by itself and by a field.
fn struct_cases() -> Vec<Case> {
    let docket = DocketDraft(String::new(), 0);
    let filled = DocketDraft("a".to_owned(), 1);
    let bundle = BundleDraft(String::new(), unlabelled());
    let bundled = BundleDraft("a".to_owned(), sound());
    let measured = MeasuredDraft(7, String::new(), unlabelled());
    let weighed = MeasuredDraft(measured.0, "a".to_owned(), sound());
    let crane = CraneDraft {
        strapped_load: StrappedDraft(unlabelled()).validate().unwrap(),
    };
    let loaded = CraneDraft {
        strapped_load: StrappedDraft(sound()).validate().unwrap(),
    };
    let hoist = HoistDraft {
        code: LoneDraft(String::new()).validate().unwrap(),
    };
    let coded = HoistDraft {
        code: LoneDraft("a".to_owned()).validate().unwrap(),
    };
    vec![
        case(
            "Package, unlabelled",
            &unlabelled(),
            unlabelled().validate().map(|_valid| ()),
        ),
        case("Package", &sound(), sound().validate().map(|_valid| ())),
        case(
            "Docket, empty and numbered zero",
            &docket,
            docket.validate().map(|_valid| ()),
        ),
        case("Docket", &filled, filled.validate().map(|_valid| ())),
        case(
            "Bundle, unnamed and unlabelled",
            &bundle,
            bundle.validate().map(|_valid| ()),
        ),
        case("Bundle", &bundled, bundled.validate().map(|_valid| ())),
        case(
            "Measured, unnamed and unlabelled after a skipped slot",
            &measured,
            measured.validate().map(|_valid| ()),
        ),
        case("Measured", &weighed, weighed.validate().map(|_valid| ())),
        case(
            "Lone, empty",
            &LoneDraft(String::new()),
            LoneDraft(String::new()).validate().map(|_valid| ()),
        ),
        case(
            "Lone",
            &LoneDraft("a".to_owned()),
            LoneDraft("a".to_owned()).validate().map(|_valid| ()),
        ),
        case(
            "Strapped, an unlabelled parcel",
            &StrappedDraft(unlabelled()),
            StrappedDraft(unlabelled()).validate().map(|_valid| ()),
        ),
        case(
            "Strapped",
            &StrappedDraft(sound()),
            StrappedDraft(sound()).validate().map(|_valid| ()),
        ),
        case(
            "Crane, an unlabelled strapped load",
            &crane,
            crane.validate().map(|_valid| ()),
        ),
        case("Crane", &loaded, loaded.validate().map(|_valid| ())),
        case(
            "Hoist, an empty code",
            &hoist,
            hoist.validate().map(|_valid| ()),
        ),
        case("Hoist", &coded, coded.validate().map(|_valid| ())),
    ]
}

/// The cases of the enums serde writes under the variant's name.
fn named_variant_cases() -> Vec<Case> {
    let boxed = FreightDraft::Boxed(unlabelled());
    let crated = FreightDraft::Crated {
        crated_parcel: unlabelled(),
    };
    let loose = FreightDraft::Loose {
        freight_label: String::new(),
    };
    let split = FreightDraft::Split(String::new(), unlabelled());
    let stacked = FreightDraft::Stacked(vec![sound(), unlabelled()]);
    let high = FreightDraft::Stacked(vec![sound(), sound(), sound()]);
    let kept = FreightDraft::Boxed(sound());
    let stamped = SealDraft::Stamped(String::new());
    let signed = SealDraft::Signed {
        signed_by: String::new(),
    };
    let inked = SealDraft::Stamped("a".to_owned());
    vec![
        case(
            "Freight, boxed unlabelled",
            &boxed,
            boxed.validate().map(|_valid| ()),
        ),
        case(
            "Freight, crated unlabelled",
            &crated,
            crated.validate().map(|_valid| ()),
        ),
        case(
            "Freight, loose and unlabelled",
            &loose,
            loose.validate().map(|_valid| ()),
        ),
        case(
            "Freight, split unnamed and unlabelled",
            &split,
            split.validate().map(|_valid| ()),
        ),
        case(
            "Freight, stacked with one unlabelled",
            &stacked,
            stacked.validate().map(|_valid| ()),
        ),
        case(
            "Freight, stacked three high",
            &high,
            high.validate().map(|_valid| ()),
        ),
        case("Freight, boxed", &kept, kept.validate().map(|_valid| ())),
        case(
            "Seal, stamped with nothing",
            &stamped,
            stamped.validate().map(|_valid| ()),
        ),
        case(
            "Seal, signed by no one",
            &signed,
            signed.validate().map(|_valid| ()),
        ),
        case("Seal, stamped", &inked, inked.validate().map(|_valid| ())),
    ]
}

/// The cases of the enums under a tag: with a content key, and with none.
fn tagged_cases() -> Vec<Case> {
    let boxed = HaulDraft::Boxed(unlabelled());
    let crated = HaulDraft::Crated {
        crated_parcel: unlabelled(),
    };
    let loose = HaulDraft::Loose {
        haul_label: String::new(),
    };
    let split = HaulDraft::Split(String::new(), unlabelled());
    let stamped = HaulDraft::Stamped(String::new());
    let kept = HaulDraft::Boxed(sound());
    let lifted = LiftDraft::Boxed(unlabelled());
    let lifted_crate = LiftDraft::Crated {
        crated_parcel: unlabelled(),
    };
    let lifted_loose = LiftDraft::Loose {
        lift_label: String::new(),
    };
    let lifted_whole = LiftDraft::Boxed(sound());
    vec![
        case(
            "Haul, boxed unlabelled",
            &boxed,
            boxed.validate().map(|_valid| ()),
        ),
        case(
            "Haul, crated unlabelled",
            &crated,
            crated.validate().map(|_valid| ()),
        ),
        case(
            "Haul, loose and unlabelled",
            &loose,
            loose.validate().map(|_valid| ()),
        ),
        case(
            "Haul, split unnamed and unlabelled",
            &split,
            split.validate().map(|_valid| ()),
        ),
        case(
            "Haul, stamped with nothing",
            &stamped,
            stamped.validate().map(|_valid| ()),
        ),
        case("Haul, boxed", &kept, kept.validate().map(|_valid| ())),
        case(
            "Lift, boxed unlabelled",
            &lifted,
            lifted.validate().map(|_valid| ()),
        ),
        case(
            "Lift, crated unlabelled",
            &lifted_crate,
            lifted_crate.validate().map(|_valid| ()),
        ),
        case(
            "Lift, loose and unlabelled",
            &lifted_loose,
            lifted_loose.validate().map(|_valid| ()),
        ),
        case(
            "Lift, boxed",
            &lifted_whole,
            lifted_whole.validate().map(|_valid| ()),
        ),
    ]
}

/// The cases of the untagged enum, of the variants serde writes under another name, and of a
/// variant that flattens a member.
fn untagged_and_renamed_cases() -> Vec<Case> {
    let labelled = LoadDraft::Labelled {
        load_label: String::new(),
        load_parcels: vec![sound(), unlabelled()],
    };
    let text = LoadDraft::Text(String::new());
    let whole = LoadDraft::Whole(unlabelled());
    let kept = LoadDraft::Whole(sound());
    let anchored = MooringDraft::Anchored {
        anchor_code: String::new(),
    };
    let docked = MooringDraft::DryDock(unlabelled());
    let dry = MooringDraft::DryDock(sound());
    let unnamed = DepartureDraft::ByRoad {
        starting_origin: OriginDraft {
            origin_name: String::new(),
        }
        .validate()
        .unwrap(),
    };
    let named = DepartureDraft::ByRoad {
        starting_origin: OriginDraft {
            origin_name: "north".to_owned(),
        }
        .validate()
        .unwrap(),
    };
    vec![
        case(
            "Load, labelled with nothing",
            &labelled,
            labelled.validate().map(|_valid| ()),
        ),
        case(
            "Load, an empty text",
            &text,
            text.validate().map(|_valid| ()),
        ),
        case(
            "Load, an unlabelled whole",
            &whole,
            whole.validate().map(|_valid| ()),
        ),
        case("Load, a whole", &kept, kept.validate().map(|_valid| ())),
        case(
            "Mooring, anchored with no code",
            &anchored,
            anchored.validate().map(|_valid| ()),
        ),
        case(
            "Mooring, an unlabelled parcel in dry dock",
            &docked,
            docked.validate().map(|_valid| ()),
        ),
        case(
            "Mooring, in dry dock",
            &dry,
            dry.validate().map(|_valid| ()),
        ),
        case(
            "Departure, by road from an unnamed origin",
            &unnamed,
            unnamed.validate().map(|_valid| ()),
        ),
        case(
            "Departure, by road",
            &named,
            named.validate().map(|_valid| ()),
        ),
    ]
}

/// One breaking value for a bound on the member and for a model held in it, in each place serde
/// writes a member at, and one passing value of every type.
pub fn place_cases() -> Vec<Case> {
    let mut cases = struct_cases();
    cases.extend(named_variant_cases());
    cases.extend(tagged_cases());
    cases.extend(untagged_and_renamed_cases());
    cases
}
