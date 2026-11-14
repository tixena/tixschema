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
    Package::model_schema_wrap_unjudged(PackageDraft {
        item_label: String::new(),
    })
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
        strapped_load: Strapped::model_schema_wrap_unjudged(StrappedDraft(unlabelled())),
    };
    let loaded = CraneDraft {
        strapped_load: StrappedDraft(sound()).into(),
    };
    let hoist = HoistDraft {
        code: Lone::model_schema_wrap_unjudged(LoneDraft(String::new())),
    };
    let coded = HoistDraft {
        code: Lone::model_schema_wrap_unjudged(LoneDraft("a".to_owned())),
    };
    vec![
        case(
            "Package, unlabelled",
            &unlabelled(),
            crate::judged_issues(unlabelled().validation_issues()),
        ),
        case(
            "Package",
            &sound(),
            crate::judged_issues(sound().validation_issues()),
        ),
        case(
            "Docket, empty and numbered zero",
            &docket,
            crate::judged_issues(docket.validation_issues()),
        ),
        case(
            "Docket",
            &filled,
            crate::judged_issues(filled.validation_issues()),
        ),
        case(
            "Bundle, unnamed and unlabelled",
            &bundle,
            crate::judged_issues(bundle.validation_issues()),
        ),
        case(
            "Bundle",
            &bundled,
            crate::judged_issues(bundled.validation_issues()),
        ),
        case(
            "Measured, unnamed and unlabelled after a skipped slot",
            &measured,
            crate::judged_issues(measured.validation_issues()),
        ),
        case(
            "Measured",
            &weighed,
            crate::judged_issues(weighed.validation_issues()),
        ),
        case(
            "Lone, empty",
            &LoneDraft(String::new()),
            crate::judged_issues(LoneDraft(String::new()).validation_issues()),
        ),
        case(
            "Lone",
            &LoneDraft("a".to_owned()),
            crate::judged_issues(LoneDraft("a".to_owned()).validation_issues()),
        ),
        case(
            "Strapped, an unlabelled parcel",
            &StrappedDraft(unlabelled()),
            crate::judged_issues(StrappedDraft(unlabelled()).validation_issues()),
        ),
        case(
            "Strapped",
            &StrappedDraft(sound()),
            crate::judged_issues(StrappedDraft(sound()).validation_issues()),
        ),
        case(
            "Crane, an unlabelled strapped load",
            &crane,
            crate::judged_issues(crane.validation_issues()),
        ),
        case(
            "Crane",
            &loaded,
            crate::judged_issues(loaded.validation_issues()),
        ),
        case(
            "Hoist, an empty code",
            &hoist,
            crate::judged_issues(hoist.validation_issues()),
        ),
        case(
            "Hoist",
            &coded,
            crate::judged_issues(coded.validation_issues()),
        ),
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
            crate::judged_issues(boxed.validation_issues()),
        ),
        case(
            "Freight, crated unlabelled",
            &crated,
            crate::judged_issues(crated.validation_issues()),
        ),
        case(
            "Freight, loose and unlabelled",
            &loose,
            crate::judged_issues(loose.validation_issues()),
        ),
        case(
            "Freight, split unnamed and unlabelled",
            &split,
            crate::judged_issues(split.validation_issues()),
        ),
        case(
            "Freight, stacked with one unlabelled",
            &stacked,
            crate::judged_issues(stacked.validation_issues()),
        ),
        case(
            "Freight, stacked three high",
            &high,
            crate::judged_issues(high.validation_issues()),
        ),
        case(
            "Freight, boxed",
            &kept,
            crate::judged_issues(kept.validation_issues()),
        ),
        case(
            "Seal, stamped with nothing",
            &stamped,
            crate::judged_issues(stamped.validation_issues()),
        ),
        case(
            "Seal, signed by no one",
            &signed,
            crate::judged_issues(signed.validation_issues()),
        ),
        case(
            "Seal, stamped",
            &inked,
            crate::judged_issues(inked.validation_issues()),
        ),
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
            crate::judged_issues(boxed.validation_issues()),
        ),
        case(
            "Haul, crated unlabelled",
            &crated,
            crate::judged_issues(crated.validation_issues()),
        ),
        case(
            "Haul, loose and unlabelled",
            &loose,
            crate::judged_issues(loose.validation_issues()),
        ),
        case(
            "Haul, split unnamed and unlabelled",
            &split,
            crate::judged_issues(split.validation_issues()),
        ),
        case(
            "Haul, stamped with nothing",
            &stamped,
            crate::judged_issues(stamped.validation_issues()),
        ),
        case(
            "Haul, boxed",
            &kept,
            crate::judged_issues(kept.validation_issues()),
        ),
        case(
            "Lift, boxed unlabelled",
            &lifted,
            crate::judged_issues(lifted.validation_issues()),
        ),
        case(
            "Lift, crated unlabelled",
            &lifted_crate,
            crate::judged_issues(lifted_crate.validation_issues()),
        ),
        case(
            "Lift, loose and unlabelled",
            &lifted_loose,
            crate::judged_issues(lifted_loose.validation_issues()),
        ),
        case(
            "Lift, boxed",
            &lifted_whole,
            crate::judged_issues(lifted_whole.validation_issues()),
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
        starting_origin: Origin::model_schema_wrap_unjudged(OriginDraft {
            origin_name: String::new(),
        }),
    };
    let named = DepartureDraft::ByRoad {
        starting_origin: Origin::model_schema_wrap_unjudged(OriginDraft {
            origin_name: "north".to_owned(),
        }),
    };
    vec![
        case(
            "Load, labelled with nothing",
            &labelled,
            crate::judged_issues(labelled.validation_issues()),
        ),
        case(
            "Load, an empty text",
            &text,
            crate::judged_issues(text.validation_issues()),
        ),
        case(
            "Load, an unlabelled whole",
            &whole,
            crate::judged_issues(whole.validation_issues()),
        ),
        case(
            "Load, a whole",
            &kept,
            crate::judged_issues(kept.validation_issues()),
        ),
        case(
            "Mooring, anchored with no code",
            &anchored,
            crate::judged_issues(anchored.validation_issues()),
        ),
        case(
            "Mooring, an unlabelled parcel in dry dock",
            &docked,
            crate::judged_issues(docked.validation_issues()),
        ),
        case(
            "Mooring, in dry dock",
            &dry,
            crate::judged_issues(dry.validation_issues()),
        ),
        case(
            "Departure, by road from an unnamed origin",
            &unnamed,
            crate::judged_issues(unnamed.validation_issues()),
        ),
        case(
            "Departure, by road",
            &named,
            crate::judged_issues(named.validation_issues()),
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
