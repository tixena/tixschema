//! Tests of `#[serde(flatten)]`: the intersection, the merged schema and the choices a flattened
//! source multiplies into.

#[cfg(feature = "mongodb")]
use bson2::oid::ObjectId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tixschema::model_schema;

/// The check an externally tagged enum's union refuses an object of one key through.
#[cfg(feature = "zod")]
const NAMED_ARM: &str = ".superRefine((_read, ctx) => { const held = ctx.value; const keys = typeof held === \"object\" && held !== null && !Array.isArray(held) ? Object.keys(held) : []; const at = ctx.issues.findIndex((issue) => issue.code === \"invalid_union\" && (issue.path ?? []).length === 0); const refused = ctx.issues[at]; const arm = keys.length === 1 && refused?.code === \"invalid_union\" ? refused.errors.find((issues) => issues.some((issue) => issue.path[0] === keys[0])) : undefined; if (arm !== undefined) ctx.issues.splice(at, 1, ...arm.map((issue) => ({ ...issue, input: undefined }))); }, { when: () => true })";

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct BasePart {
    owner: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "dataType")]
enum DataElementSampleValueVariant {
    Alphanumeric {
        #[serde(rename = "sampleValues")]
        sample_values: Vec<String>,
    },
    Logical {
        #[serde(rename = "sampleValues")]
        sample_values: Vec<bool>,
    },
    Numeric {
        #[serde(rename = "sampleValues")]
        sample_values: Vec<i64>,
    },
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
struct DataElementSampleValueEntry {
    data_element_id: String,
    #[serde(flatten)]
    variant: DataElementSampleValueVariant,
}

/// Two bases that flatten each other: neither body exists when the other's merge asks for it, and
/// no finite value inhabits either type.
#[model_schema(nonstrict, enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct CycleFirst {
    first_own: String,
    #[serde(flatten)]
    second: CycleSecond,
}

#[model_schema(nonstrict, enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct CycleSecond {
    #[serde(flatten)]
    first: Box<CycleFirst>,
    second_own: String,
}

/// The same cycle with a union in the middle: the branch that closes it is the deferred name, and
/// a reference carries none of the members it stands for.
#[model_schema(nonstrict, enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct CycleUnionNode {
    #[serde(flatten)]
    either: CycleUnionEither,
    own: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum CycleUnionEither {
    Only(Box<CycleUnionNode>),
}

/// A cycle spanning both places an intersection operand is written: a struct's `#[serde(flatten)]`
/// base on one side, an internally tagged newtype variant's content on the other. Each names the
/// other's `const`, so neither declaration order puts both above the other.
#[cfg(feature = "zod")]
#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct CycleVariantContent {
    #[serde(flatten)]
    back: CycleVariantHost,
    own: String,
}

#[cfg(feature = "zod")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
enum CycleVariantHost {
    Wrapped(Box<CycleVariantContent>),
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ExtraPart {
    priority: i64,
}

/// A base that names itself, written only where something asks what it describes as: what a
/// flattened branch that is a reference rather than an object contributes to its container.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatHolder {
    #[serde(flatten)]
    base: FlatNode,
    extra: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatNode {
    children: Vec<Self>,
    val: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlattenOnly {
    #[serde(flatten)]
    variant: DataElementSampleValueVariant,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct MultiFlatten {
    #[serde(flatten)]
    base: BasePart,
    #[serde(flatten)]
    extra: ExtraPart,
    id: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct NoFlatten {
    id: String,
    name: String,
}

/// A plain enum flattened into a struct. Declared without `#[model_schema()]`: a plain enum writes
/// its own variant name rather than an object, so the crate refuses the declaration and the wire
/// form is only readable from a plain serde type.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum FlatHue {
    Red,
}

/// Flattens `FlatHue`, as a plain serde type.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatOverEnum {
    own: String,
    #[serde(flatten)]
    tone: FlatHue,
}

/// An untagged enum every member of which serde writes as an object, and the struct that flattens
/// it. serde writes whichever member matched into the object the struct is writing, so what the
/// struct writes is one key set per member.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatFirst {
    a: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatSecond {
    b: bool,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum FlatEither {
    First(FlatFirst),
    Second(FlatSecond),
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatOverUntagged {
    #[serde(flatten)]
    either: FlatEither,
    own: String,
}

/// [`FlatFirst`] and [`FlatSecond`] `nonstrict`, which a flattened `Option` of a union over them
/// asks: serde reads that `Option` as `None` beside a key a strict member refuses.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct LooseFirst {
    a: String,
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct LooseSecond {
    b: bool,
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum LooseEither {
    First(LooseFirst),
    Second(LooseSecond),
}

/// The same union reached through an `Option`, so the object writes one of the members' key sets
/// beside its own or writes its own alone: two choices, multiplied.
#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatOverOptionalUntagged {
    #[serde(flatten, skip_serializing_if = "Option::is_none", default)]
    either: Option<LooseEither>,
    own: String,
}

/// The same union under an alias. An alias *is* the type it names, so the object that flattens it
/// writes exactly what the object flattening the enum writes.
#[cfg(feature = "zod")]
#[model_schema()]
type FlatEitherAlias = FlatEither;

#[cfg(feature = "zod")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatOverAliasedUntagged {
    #[serde(flatten)]
    either: FlatEitherAlias,
    own: String,
}

/// An object that flattens a union declared *below* it. Nothing at the merge tells that source
/// apart from a struct declared below — both are names the expansion has not seen — so the merge
/// takes the same fallback a forward reference already takes and names the source as one operand.
#[cfg(feature = "zod")]
#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatOverLaterUntagged {
    #[serde(flatten)]
    either: LaterEither,
    own: String,
}

#[cfg(feature = "zod")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum LaterEither {
    First(FlatFirst),
    Second(FlatSecond),
}

/// The same shape with one member serde writes as a string rather than an object. Standing on its
/// own it is an ordinary union — an untagged enum may hold a scalar — and every surface describes
/// it.
#[cfg(feature = "zod")]
#[cfg(feature = "zod")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum FlatScalarEither {
    Obj(FlatFirst),
    Text(String),
}

/// The same shape with one member reached through an `Option`. Standing on its own it is an
/// ordinary union — a member may be written as its value or as a bare `null`, and every surface
/// describes the choice.
#[cfg(feature = "zod")]
#[cfg(feature = "zod")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum FlatNullableEither {
    Base(FlatFirst),
    Maybe(Option<FlatSecond>),
}

/// Three members that name another item rather than spelling a type out: a brand over a string, a
/// brand over a `bool`, and a plain unit enum. What serde writes each as lives on the named item,
/// so the merge asks the registry rather than the spelling.
#[cfg(feature = "zod")]
#[cfg(feature = "zod")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(transparent)]
struct MemberSlug(String);

#[cfg(feature = "zod")]
#[cfg(feature = "zod")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(transparent)]
struct MemberSwitch(bool);

#[cfg(feature = "zod")]
#[cfg(feature = "zod")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum MemberHue {
    Blue,
    Red,
}

#[cfg(feature = "zod")]
#[cfg(feature = "zod")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum MemberSlugEither {
    Base(FlatFirst),
    Slug(MemberSlug),
}

#[cfg(feature = "zod")]
#[cfg(feature = "zod")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum MemberSwitchEither {
    Base(FlatFirst),
    Switch(MemberSwitch),
}

#[cfg(feature = "zod")]
#[cfg(feature = "zod")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum MemberHueEither {
    Base(FlatFirst),
    Hue(MemberHue),
}

/// A generic untagged enum filled at `()`, flattened by a struct declared above it: its member is
/// its own type parameter, and the enum publishes no record at a filling, so the merge reads the
/// `null` of that member as no key.
#[model_schema(nonstrict, enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatOverLaterHeldUnit {
    #[serde(flatten)]
    held: LaterHeldMember<()>,
    id: String,
}

#[model_schema(default_types(T = String), enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum LaterHeldMember<T> {
    Fields(FlatFirst),
    Held(T),
}

/// A flattened `Option` of a struct declared below its holder: its `null` is the absence.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatOverLaterMaybeFields {
    #[serde(flatten, skip_serializing_if = "Option::is_none", default)]
    held: Option<LaterFieldsBelow>,
    id: String,
}

/// The same `Option` through a struct of one slot declared below its holder.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatOverLaterMaybeSlot {
    #[serde(flatten)]
    held: LaterMaybeSlot,
    id: String,
}

#[model_schema(no_display, enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct LaterMaybeSlot(Option<LaterFieldsBelow>);

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct LaterFieldsBelow {
    below: String,
}

/// Three members naming another item rather than spelling a type out, each a different wire
/// shape: one is a JSON array, one is the object a map writes, one has a nullable published
/// surface. serde flattens only the map and refuses the other two.
#[cfg(feature = "zod")]
#[cfg(feature = "zod")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct MemberBag(Vec<String>);

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct MemberBucket(HashMap<String, String>);

#[cfg(feature = "zod")]
#[cfg(feature = "zod")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct MemberMaybeSecond(Option<FlatSecond>);

#[cfg(feature = "zod")]
#[cfg(feature = "zod")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum MemberBagEither {
    Base(FlatFirst),
    Items(MemberBag),
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum MemberBucketEither {
    Base(FlatFirst),
    Bucket(MemberBucket),
}

#[cfg(feature = "zod")]
#[cfg(feature = "zod")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum MemberMaybeEither {
    Base(FlatFirst),
    Maybe(MemberMaybeSecond),
}

/// The map-shaped member is the one of the three that stays where it was: serde writes a map's keys
/// straight into the object being written, which is what flattening is, so the multiplication keeps
/// its branch and the merge keeps its document. Flattened alone: beside a member it is refused.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatOverMemberBucketUntagged {
    #[serde(flatten)]
    either: MemberBucketEither,
}

/// A member naming a tagged enum: serde writes a data-carrying variant as a single-key object but
/// a unit variant as a bare string — a leaf no object can be merged with, one level in from the
/// member.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum MemberExtBare {
    Bare,
    Wrapped(FlatSecond),
}

/// The same tagged enum flattened directly, with no union between: flattened, serde writes a unit
/// variant as its name holding `null` rather than the bare string it is standing alone, and reads
/// that back as the variant — so the merges owe it one branch per variant.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ExtBareDirectHolder {
    #[serde(flatten)]
    ext: MemberExtBare,
    own: String,
}

/// A tagged enum that declares a parameter, of a unit variant and a variant that holds it.
#[model_schema(default_types(Load = String), enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum MemberExtCarried<Load> {
    Bare,
    Carried(Load),
}

/// That enum written with an argument and flattened: each variant's entry is its own with the
/// parameter filled by the argument.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ExtCarriedHolder {
    #[serde(flatten)]
    ext: MemberExtCarried<u32>,
    own: String,
}

/// That enum flattened at its holder's own parameter, which the holder's factory binds.
#[cfg(any(feature = "zod", feature = "typescript"))]
#[model_schema(default_types(Held = bool))]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ExtCarriedOwnHolder<Held> {
    #[serde(flatten)]
    ext: MemberExtCarried<Held>,
    own: String,
}

/// An alias of that enum that declares a parameter of its own.
#[model_schema(default_types(Stowed = String), enable = ["jsonschema"])]
type MemberExtCarriedOf<Stowed> = MemberExtCarried<Stowed>;

/// The alias written with an argument and flattened.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ExtCarriedAliasHolder {
    #[serde(flatten)]
    ext: MemberExtCarriedOf<bool>,
    own: String,
}

/// An untagged enum that declares a parameter: a member that is the parameter, and one of named
/// fields.
#[model_schema(default_types(Load = BasePart), enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum MemberUntaggedCarried<Load> {
    Counted { count: u32 },
    Held(Load),
}

/// That enum written with an argument and flattened: each member is its own with the parameter
/// filled by the argument.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct UntaggedCarriedHolder {
    #[serde(flatten)]
    either: MemberUntaggedCarried<BasePart>,
    own: String,
}

/// An untagged enum that declares a parameter, of two members of named fields, each closed to the
/// other's key where it is flattened.
#[model_schema(default_types(Load = String), enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum MemberUntaggedPicked<Load> {
    Carried { load: Load },
    Counted { count: u32 },
}

/// That enum written with an argument and flattened.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct UntaggedPickedHolder {
    own: String,
    #[serde(flatten)]
    pick: MemberUntaggedPicked<u32>,
}

/// An alias of that enum that declares a parameter of its own.
#[model_schema(default_types(Stowed = String), enable = ["jsonschema"])]
type MemberUntaggedPickedOf<Stowed> = MemberUntaggedPicked<Stowed>;

/// The alias written with an argument and flattened.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct UntaggedPickedAliasHolder {
    own: String,
    #[serde(flatten)]
    pick: MemberUntaggedPickedOf<bool>,
}

/// A flattened `ObjectId`, whose `$oid` key serde writes beside the object's own.
#[cfg(feature = "mongodb")]
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlattenedOidHolder {
    id: u32,
    #[serde(flatten)]
    oid: ObjectId,
}

/// An `ObjectId` in the lone slot of a variant under a tag alone, whose `$oid` key serde writes
/// beside the tag, as it writes a flattened one beside the object's own.
#[cfg(feature = "mongodb")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind")]
enum TaggedOidHolder {
    Held(ObjectId),
}

/// A type parameter flattened beside a member, as serde alone writes it.
#[derive(Serialize, Debug)]
struct SerdeOnlyBag<T> {
    id: u32,
    #[serde(flatten)]
    inner: T,
}

/// A type parameter held by the lone slot of a variant under a tag alone, as serde alone writes it.
#[derive(Serialize, Debug)]
#[serde(tag = "kind")]
enum SerdeOnlyWaybill<Held> {
    Held(Held),
}

/// A brand over text, as serde alone writes it.
#[derive(Serialize, Debug)]
#[serde(transparent)]
struct SerdeOnlyCode(String);

/// And a tagged enum whose every variant carries data, which serde writes as the object its name
/// tags in every case — admitted where it always was, on every surface.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum MemberExtObjects {
    One(FlatFirst),
    Two(FlatSecond),
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum MemberExtObjEither {
    Base(FlatFirst),
    Ext(MemberExtObjects),
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatOverMemberExtObjUntagged {
    #[serde(flatten)]
    either: MemberExtObjEither,
    own: String,
}

/// That enum flattened directly, where Zod and TypeScript multiply for different reasons: a Zod
/// intersection recognizes only the keys its operands name and a `z.union` names none, so joining
/// admits nothing; TypeScript distributes over every branch instead.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ExtObjDirectHolder {
    #[serde(flatten)]
    ext: MemberExtObjects,
    own: String,
}

/// The same enum through an `Option`: `keyof` a union is the keys its branches share, which for
/// two differently-tagged variants is none — an empty mapped type every object passes through —
/// unless each variant is closed against the other.
#[cfg(feature = "typescript")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct OptExtObjHolder {
    #[serde(flatten, skip_serializing_if = "Option::is_none", default)]
    ext: Option<MemberExtObjects>,
    own: String,
}

/// An untagged union written over the objects its members are, rather than over names — the only
/// case where the declaration itself spells each member's keys, since serde matches a member by
/// shape.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum InlineUntagEither {
    Left { left: String },
    Right { right: bool },
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct InlineUntagDirectHolder {
    #[serde(flatten)]
    either: InlineUntagEither,
    own: String,
}

/// The same union under an alias, which the object that flattens it joins by the union's members.
#[cfg(any(feature = "zod", feature = "typescript"))]
#[model_schema()]
type InlineUntagAlias = InlineUntagEither;

#[cfg(any(feature = "zod", feature = "typescript"))]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct InlineUntagAliasHolder {
    #[serde(flatten)]
    either: InlineUntagAlias,
    own: String,
}

/// A union member that names itself, and the struct that flattens the union. The member describes
/// as a reference into the definitions, and the body it points at is written by the time the merge
/// asks for it.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatSelfNode {
    kids: Vec<Self>,
    leaf: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum FlatSelfEither {
    Node(FlatSelfNode),
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatOverSelfUntagged {
    #[serde(flatten)]
    either: FlatSelfEither,
    own: String,
}

/// An untagged enum whose members overlap: one member's key set is a subset of the other's, and
/// the difference is a key that member omits when it is absent. serde writes the first member that
/// matches, so the payload it writes for the narrower member is one the wider member admits too.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct OverlapNarrow {
    a: String,
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct OverlapWide {
    a: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    b: Option<String>,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum OverlapEither {
    Narrow(OverlapNarrow),
    Wide(OverlapWide),
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatOverOverlap {
    #[serde(flatten)]
    either: OverlapEither,
    own: String,
}

/// One object that merges both spellings of a union: a discriminated enum, whose members are
/// exclusive, and the overlapping untagged one, whose members are not.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind")]
enum MixedTagged {
    Left { left: String },
    Right { right: bool },
}

/// The members of [`MixedEither`], each reading past a key it does not declare.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct MixedFull {
    a: String,
    b: String,
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct MixedPartial {
    a: String,
}

/// An untagged enum whose members overlap, neither reading every value of the other: `Full`
/// requires a key `Partial` never writes.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum MixedEither {
    Full(MixedFull),
    Partial(MixedPartial),
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct FlatOverMixed {
    #[serde(flatten)]
    either: MixedEither,
    own: String,
    #[serde(flatten)]
    tagged: MixedTagged,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct StampDepth {
    depth: u32,
}

/// A struct that flattens a member, read from every key its holder leaves, which refuses a key it
/// does not declare.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct StrictStamp {
    #[serde(flatten)]
    depth: StampDepth,
    ink: String,
}

/// A nonstrict holder whose strict member refuses a stray key.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct GuestLetter {
    #[serde(flatten)]
    stamp: StrictStamp,
    title: String,
}

/// A nonstrict enum under a tag alone whose newtype variant holds a strict struct, which refuses a
/// stray key serde hands it beside the tag, beside a variant that reads one past.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind")]
enum Docket {
    Noted { note: String },
    Stamped(StrictStamp),
}

/// A union one member of which is itself a union, and the struct that flattens the outer one. serde
/// writes whichever leaf member matched into the object the struct is writing, so the nesting
/// contributes no key of its own and what the struct writes is one key set per leaf.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct NestPlain {
    plain: String,
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind")]
enum NestTagged {
    Left { left: String },
    Right { right: bool },
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum NestEither {
    Plain(NestPlain),
    Tagged(NestTagged),
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct NestHolder {
    #[serde(flatten)]
    either: NestEither,
    own: String,
}

/// The same nesting with one member at every level: a choice of one is no choice, so no wrapper is
/// written and the holder describes as the one key set it writes.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct NestOnly {
    a: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum NestOnlyInner {
    A(NestOnly),
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum NestOnlyOuter {
    Inner(NestOnlyInner),
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct NestOnlyHolder {
    #[serde(flatten)]
    either: NestOnlyOuter,
    own: String,
}

/// Two unions that name each other. The outer one is deferred by the frame that reads it and its
/// body is written by the time the merge asks for it, so the cycle carries no missing body — it is
/// visible only as a name the expansion reaches twice on one path.
#[model_schema(nonstrict, enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct NestCycleHolder {
    #[serde(flatten)]
    either: NestCycleOuter,
    own: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum NestCycleOuter {
    Inner(Box<NestCycleInner>),
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum NestCycleInner {
    Back(Box<NestCycleOuter>),
}

/// A base reached through an `Option`: serde writes the base's members beside the object's own
/// when `Some`, and the object alone when `None` — the guard forces `skip_serializing_if`, so
/// absence is the only `None` the crate admits. Two required members, so a payload carrying only
/// one is a base serde never writes.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct OptBase {
    left: String,
    right: bool,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct OptHolder {
    #[serde(flatten, skip_serializing_if = "Option::is_none", default)]
    maybe: Option<OptBase>,
    own: String,
}

/// The other spelling of the same absence: a nullable-surfaced registration flattened directly, no
/// union between. serde reads and writes both forms exactly as the `Option`-typed field does, so
/// the merge owes both spellings one answer.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct MaybeOptBase(Option<OptBase>);

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct NamedMaybeHolder {
    #[serde(flatten)]
    maybe: MaybeOptBase,
    own: String,
}

/// And a name whose published surface is not nullable, which offers no absence and stays the one
/// operand it always was.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct PlainOptBase(OptBase);

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct NamedPlainHolder {
    own: String,
    #[serde(flatten)]
    plain: PlainOptBase,
}

/// The same absence over a source that is itself a union: serde writes the matched member's keys or
/// no keys at all, so the choice the `Option` adds sits outside the choice the enum already offered.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct OptUnionHolder {
    #[serde(flatten, skip_serializing_if = "Option::is_none", default)]
    maybe: Option<NestTagged>,
    own: String,
}

/// The source an enum's own struct variants flatten, and one form of the enum per tagging: serde
/// writes the source's members into the variant's own content object, wherever that object sits.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct VariantExtra {
    note: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct VariantRank {
    rank: i64,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum ExternalFlatVariant {
    Named {
        #[serde(flatten)]
        extra: VariantExtra,
        x: String,
    },
    Plain {
        y: String,
    },
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind")]
enum InternalFlatVariant {
    Named {
        #[serde(flatten)]
        extra: VariantExtra,
        x: String,
    },
    Plain {
        y: String,
    },
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind", content = "data")]
enum AdjacentFlatVariant {
    Named {
        #[serde(flatten)]
        extra: VariantExtra,
        x: String,
    },
    Plain {
        y: String,
    },
}

/// Two sources flattened into one variant: both sets of members join the same content object.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum TwiceFlatVariant {
    Named {
        #[serde(flatten)]
        extra: VariantExtra,
        #[serde(flatten)]
        rank: VariantRank,
        x: String,
    },
}

/// The same source at an untagged enum's own member position: no discriminator stands over it, so
/// serde writes the source's members beside the variant's own and nothing else.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum UntaggedFlatVariant {
    Named {
        #[serde(flatten)]
        extra: VariantExtra,
        x: String,
    },
    Plain {
        y: String,
    },
}

/// Two sources flattened into one untagged member.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum TwiceUntaggedFlatVariant {
    Named {
        #[serde(flatten)]
        extra: VariantExtra,
        #[serde(flatten)]
        rank: VariantRank,
        x: String,
    },
}

/// An object that flattens such an enum. A member that flattens does not prove its own key list, so
/// no member of this union is closed against keys the expansion cannot enumerate.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct UntaggedFlatVariantHolder {
    #[serde(flatten)]
    either: UntaggedFlatVariant,
    own: String,
}

/// The members a variant's flattened choice is built from, one key apiece so a payload names which
/// branch wrote it.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ChoiceLeft {
    left: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ChoiceMiddle {
    middle: bool,
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ChoiceRight {
    right: i64,
}

#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ChoiceUp {
    up: String,
}

#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct ChoiceDown {
    down: String,
}

/// A registered choice: serde writes one key set per member, so a variant flattening it writes one
/// key set per member too.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum VariantChoice {
    Left(ChoiceLeft),
    Right(ChoiceRight),
}

/// The same with a third member, so the multiplication is counted rather than assumed to be a pair.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum ThreeBranchChoice {
    Left(ChoiceLeft),
    Middle(ChoiceMiddle),
    Right(ChoiceRight),
}

/// A second choice over keys the first one does not write, so two of them in one variant multiply
/// into a cross product a payload can be matched against.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum PairChoice {
    Down(ChoiceDown),
    Up(ChoiceUp),
}

/// One form of enum per tagging, each flattening the same two-member choice into a struct variant.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum ExternalChoiceVariant {
    Named {
        #[serde(flatten)]
        choice: VariantChoice,
        x: String,
    },
    Plain {
        y: String,
    },
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind")]
enum InternalChoiceVariant {
    Named {
        #[serde(flatten)]
        choice: VariantChoice,
        x: String,
    },
    Plain {
        y: String,
    },
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind", content = "data")]
enum AdjacentChoiceVariant {
    Named {
        #[serde(flatten)]
        choice: VariantChoice,
        x: String,
    },
    Plain {
        y: String,
    },
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum UntaggedChoiceVariant {
    Named {
        #[serde(flatten)]
        choice: VariantChoice,
        x: String,
    },
    Plain {
        y: String,
    },
}

/// Three branches, so the variant's object is written three times over.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum ThreeBranchChoiceVariant {
    Named {
        #[serde(flatten)]
        choice: ThreeBranchChoice,
        x: String,
    },
}

/// An `Option` over a plain source: serde writes the source's members or leaves them out, which is
/// two key sets and no branching of the source's own.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum OptionalSourceVariant {
    Named {
        #[serde(flatten, skip_serializing_if = "Option::is_none", default)]
        extra: Option<VariantExtra>,
        x: String,
    },
}

/// Two choices in one variant: the combinations are the cross product of their members.
#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum TwoChoiceVariant {
    Named {
        #[serde(flatten)]
        choice: VariantChoice,
        #[serde(flatten)]
        pair: PairChoice,
        x: String,
    },
}

/// An `Option` over a choice: the source's own members, and the absence beside them.
#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum OptionalChoiceVariant {
    Named {
        #[serde(flatten, skip_serializing_if = "Option::is_none", default)]
        choice: Option<VariantChoice>,
        x: String,
    },
}

/// A `nonstrict` enum one variant of which flattens a choice that reads past a key it does not
/// declare, beside a variant whose choice refuses one: serde judges each variant by itself, so it
/// reads such a key past in the one and refuses it in the other.
#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum RefusingBesideOpenVariant {
    Open {
        #[serde(flatten)]
        choice: VariantChoice,
        x: String,
    },
    Refusing {
        #[serde(flatten)]
        choice: ThreeBranchChoice,
        x: String,
    },
}

/// A multiplying variant whose own member reaches the enum being defined, so every combination has
/// to carry the deferral a single combination already carries.
#[model_schema(nonstrict)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum RecursiveChoiceVariant {
    Named {
        #[serde(flatten)]
        choice: VariantChoice,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        next: Option<Box<Self>>,
    },
    Plain {
        y: String,
    },
}

/// The branch of an untagged document that names `key`.
fn untagged_branch(document: &serde_json::Value, key: &str) -> serde_json::Value {
    document["anyOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|branch| branch["properties"].as_object().unwrap().contains_key(key))
        .unwrap()
        .clone()
}

/// The content object of `variant` in `document`, whichever of the three taggings put it there.
fn variant_content(document: &serde_json::Value, variant: &str) -> serde_json::Value {
    let branches = document["oneOf"].as_array().unwrap();
    let member = branches
        .iter()
        .find(|branch| {
            let properties = branch["properties"].as_object().unwrap();
            properties.contains_key(variant)
                || properties
                    .get("kind")
                    .is_some_and(|tag| tag["const"] == variant)
        })
        .unwrap();
    let properties = member["properties"].as_object().unwrap();
    for key in [variant, "data"] {
        if let Some(content) = properties.get(key) {
            return content.clone();
        }
    }
    member.clone()
}

/// The keys a content object names, and the keys it requires.
fn described_keys(content: &serde_json::Value) -> (Vec<String>, Vec<String>) {
    let named = content["properties"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    let required = content["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|key| key.as_str().unwrap().to_owned())
        .collect();
    (named, required)
}

/// Whether `payload` is accepted by a document every leaf of which is an object closed by
/// `additionalProperties: false`: a leaf accepts when it names every key the payload carries and
/// requires no key it does not.
fn closed_document_accepts(schema: &serde_json::Value, payload: &serde_json::Value) -> bool {
    if let Some(branches) = schema.get("oneOf") {
        return accepting_branches(branches, payload) == 1;
    }
    if let Some(branches) = schema.get("anyOf") {
        return accepting_branches(branches, payload) >= 1;
    }
    let written = payload.as_object().unwrap();
    let named = schema["properties"].as_object().unwrap();
    let required = schema["required"].as_array().unwrap();
    written.keys().all(|key| named.contains_key(key))
        && required
            .iter()
            .all(|key| written.contains_key(key.as_str().unwrap()))
}

/// How many of a union's branches accept `payload` — what the two spellings disagree about.
fn accepting_branches(branches: &serde_json::Value, payload: &serde_json::Value) -> usize {
    branches
        .as_array()
        .unwrap()
        .iter()
        .filter(|branch| closed_document_accepts(branch, payload))
        .count()
}

#[test]
fn test_flatten_structs_constructible() {
    let base = BasePartDraft {
        owner: String::new(),
    };
    assert_eq!(base.owner, String::new());
    let extra = ExtraPartDraft { priority: 0 };
    assert_eq!(extra.priority, 0_i64);
    let flatten_only = FlattenOnlyDraft {
        variant: DataElementSampleValueVariantDraft::Alphanumeric {
            sample_values: Vec::new(),
        }
        .validate()
        .unwrap(),
    };
    assert!(matches!(
        &*flatten_only.variant,
        DataElementSampleValueVariantDraft::Alphanumeric {
            sample_values: _sample_values
        }
    ));
    let multi = MultiFlattenDraft {
        base: BasePartDraft {
            owner: String::new(),
        }
        .validate()
        .unwrap(),
        extra: ExtraPartDraft { priority: 0 }.validate().unwrap(),
        id: String::new(),
    };
    assert_eq!(multi.id, String::new());
    let no_flatten = NoFlattenDraft {
        id: String::new(),
        name: String::new(),
    };
    assert_eq!(no_flatten.id, String::new());
}

#[test]
#[cfg(feature = "typescript")]
fn test_flatten_typescript_intersection() {
    let ts = DataElementSampleValueEntry::ts_definition();
    assert!(ts.contains("export type DataElementSampleValueEntryDraft = {"));
    assert!(ts.contains("dataElementId: string;"));
    assert!(ts.contains("} & DataElementSampleValueVariantDraft;"));
    assert!(!ts.contains("variant:"));
}

#[test]
#[cfg(feature = "typescript")]
fn test_flatten_typescript_multiple() {
    let ts = MultiFlatten::ts_definition();
    assert!(ts.contains("id: string;"));
    assert!(ts.contains("} & BasePartDraft & ExtraPartDraft;"));
    assert!(!ts.contains("base:"));
    assert!(!ts.contains("extra:"));
}

#[test]
#[cfg(feature = "typescript")]
fn test_flatten_only_typescript_is_alias() {
    let ts = FlattenOnly::ts_definition();
    assert!(ts.contains("export type FlattenOnlyDraft = DataElementSampleValueVariantDraft;"));
    assert!(!ts.contains("Record<string, never>"));
    assert!(!ts.contains("variant:"));
}

#[test]
#[cfg(feature = "typescript")]
fn test_no_flatten_typescript_unchanged() {
    let ts = NoFlatten::ts_definition();
    assert!(ts.contains("export type NoFlattenDraft = {"));
    assert!(ts.contains("id: string;"));
    assert!(ts.contains("name: string;"));
    let draft = ts.split("\n\nexport type NoFlatten =").next().unwrap();
    assert!(!draft.contains(" & "));
}

#[test]
#[cfg(feature = "typescript")]
fn test_flatten_variant_keeps_pascal_discriminator_and_camel_fields() {
    let ts = DataElementSampleValueVariant::ts_definition();
    assert!(ts.contains("dataType: \"Numeric\""));
    assert!(ts.contains("sampleValues:"));
    assert!(!ts.contains("sample_values"));
}

/// Whether `zod` closes the statement `expression` is bound in right after it: the raw `const`'s
/// own `;` beside TypeScript, and the exported one's, after the examples it holds, without it.
#[cfg(feature = "zod")]
fn closes_on(zod: &str, expression: &str) -> bool {
    #[cfg(feature = "typescript")]
    let closed = format!("{expression};");
    #[cfg(not(feature = "typescript"))]
    let closed = format!("{expression}.meta({{\n  examples: []\n}});");
    zod.contains(&closed)
}

/// [`NAMED_ARM`] of a union that holds the variants `carrying`, each with what serde expects of it
/// where a string names it.
#[cfg(feature = "zod")]
fn named_arm(carrying: &[(&str, &str)]) -> String {
    if carrying.is_empty() {
        return NAMED_ARM.to_owned();
    }
    let listed: Vec<String> = carrying
        .iter()
        .map(|(name, expected)| format!("[\"{name}\", \"{expected}\"]"))
        .collect();
    format!(
        ".superRefine((_read, ctx) => {{ const held = ctx.value; const carrying = [{}]; const keys = typeof held === \"object\" && held !== null && !Array.isArray(held) ? Object.keys(held) : []; const at = ctx.issues.findIndex((issue) => issue.code === \"invalid_union\" && (issue.path ?? []).length === 0); const refused = ctx.issues[at]; const named = typeof held === \"string\" ? carrying.find(([name]) => name === held) : undefined; const arm = keys.length === 1 && refused?.code === \"invalid_union\" ? refused.errors.find((issues) => issues.some((issue) => issue.path[0] === keys[0])) : undefined; if (named !== undefined && refused?.code === \"invalid_union\") ctx.issues.splice(at, 1, {{ code: \"custom\", message: \"invalid type: unit variant, expected \" + named[1], input: undefined, path: [named[0]], params: {{ variant: true }} }}); else if (arm !== undefined) ctx.issues.splice(at, 1, ...arm.map((issue) => ({{ ...issue, input: undefined }}))); }}, {{ when: () => true }})",
        listed.join(", ")
    )
}

/// `union`, the union of an externally tagged enum, behind the check that refuses a value that is
/// neither a string nor an object of one key and before [`named_arm`] of `carrying`, as the enum's
/// own schema writes it.
#[cfg(feature = "zod")]
fn one_key(union: &str, carrying: &[(&str, &str)]) -> String {
    let arm = named_arm(carrying);
    format!(
        "z.unknown().check((ctx) => {{ if (ctx.value !== undefined && typeof ctx.value !== \
         \"string\" && (typeof ctx.value !== \"object\" || ctx.value === null || \
         Array.isArray(ctx.value) || Object.keys(ctx.value).length !== 1)) ctx.issues.push({{ \
         code: \"custom\", message: \"expected value\", input: ctx.value, params: {{ \
         undeserializable: true }} }}); }}).pipe({union}{arm})"
    )
}

#[test]
#[cfg(feature = "zod")]
fn test_flatten_zod_intersection() {
    let zod = DataElementSampleValueEntry::zod_schema();
    assert!(zod.contains("z.strictObject({"));
    assert!(zod.contains("dataElementId:"));
    assert!(closes_on(
        &zod,
        "}).and(z.lazy(() => DataElementSampleValueVariant$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))"
    ));
    assert!(!zod.contains("variant:"));
}

#[test]
#[cfg(feature = "zod")]
fn test_flatten_zod_multiple_chained() {
    let zod = MultiFlatten::zod_schema();
    assert!(closes_on(
        &zod,
        "}).and(z.lazy(() => BasePart$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })).and(z.lazy(() => ExtraPart$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))"
    ));
}

#[test]
#[cfg(feature = "zod")]
fn test_no_flatten_zod_unchanged() {
    let zod = NoFlatten::zod_schema();
    assert!(zod.contains("z.strictObject({"));
    assert!(!zod.contains(".and("));
}

#[test]
#[cfg(feature = "zod")]
fn test_a_flattened_base_is_never_read_while_the_const_initializes() {
    for zod in [
        DataElementSampleValueEntry::zod_schema(),
        MultiFlatten::zod_schema(),
        FlattenOnly::zod_schema(),
    ] {
        for name in ["DataElementSampleValueVariant", "BasePart", "ExtraPart"] {
            assert!(
                !zod.contains(&format!(".and({name}$Schema)")),
                "`{name}$Schema` is read eagerly in: {zod}"
            );
        }
    }
}

#[test]
#[cfg(feature = "zod")]
fn test_a_flatten_cycle_defers_both_sides_of_the_pair() {
    let first = CycleFirst::zod_schema();
    let second = CycleSecond::zod_schema();

    assert!(
        closes_on(
            &first,
            "}).and(z.lazy(() => CycleSecond$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))"
        ),
        "expected a deferred base, got: {first}"
    );
    assert!(
        closes_on(
            &second,
            "}).and(z.lazy(() => CycleFirst$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))"
        ),
        "expected a deferred base, got: {second}"
    );
    assert!(
        !first.contains(".and(CycleSecond$Schema)"),
        "`CycleSecond$Schema` is read eagerly in: {first}"
    );
    assert!(
        !second.contains(".and(CycleFirst$Schema)"),
        "`CycleFirst$Schema` is read eagerly in: {second}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_a_flatten_cycle_through_a_variants_content_defers_both_sides() {
    let content = CycleVariantContent::zod_schema();
    let host = CycleVariantHost::zod_schema();

    assert!(
        closes_on(
            &content,
            "}).and(z.lazy(() => CycleVariantHost$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))"
        ),
        "expected a deferred base, got: {content}"
    );
    assert!(
        host.contains("}).and(z.lazy(() => CycleVariantContent$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))"),
        "expected a deferred content, got: {host}"
    );
    assert!(
        !content.contains(".and(CycleVariantHost$Schema)"),
        "`CycleVariantHost$Schema` is read eagerly in: {content}"
    );
    assert!(
        !host.contains(".and(CycleVariantContent$Schema)"),
        "`CycleVariantContent$Schema` is read eagerly in: {host}"
    );
}

#[test]
fn test_flatten_json_schema_distributes_base_into_variants() {
    let schema = DataElementSampleValueEntry::json_schema();
    let one_of = schema["oneOf"].as_array().unwrap();
    assert_eq!(one_of.len(), 3);
    for branch in one_of {
        assert_eq!(
            branch["additionalProperties"],
            serde_json::Value::Bool(false)
        );
        let props = branch["properties"].as_object().unwrap();
        assert!(props.contains_key("dataElementId"));
        assert!(props.contains_key("dataType"));
        assert!(props.contains_key("sampleValues"));
        let req = branch["required"].as_array().unwrap();
        assert!(req.iter().any(|v| v.as_str() == Some("dataElementId")));
        assert!(req.iter().any(|v| v.as_str() == Some("dataType")));
        assert!(req.iter().any(|v| v.as_str() == Some("sampleValues")));
    }
}

#[test]
fn test_multi_flatten_json_schema_merges_plain_structs() {
    let schema = MultiFlatten::json_schema();
    assert!(schema.get("oneOf").is_none());
    assert_eq!(
        schema["additionalProperties"],
        serde_json::Value::Bool(false)
    );
    let props = schema["properties"].as_object().unwrap();
    assert!(props.contains_key("id"));
    assert!(props.contains_key("owner"));
    assert!(props.contains_key("priority"));
}

#[test]
fn test_flatten_only_json_schema_is_union() {
    let schema = FlattenOnly::json_schema();
    let one_of = schema["oneOf"].as_array().unwrap();
    assert_eq!(one_of.len(), 3);
    for branch in one_of {
        let props = branch["properties"].as_object().unwrap();
        assert!(props.contains_key("dataType"));
        assert!(props.contains_key("sampleValues"));
        assert!(!props.contains_key("dataElementId"));
    }
}

#[test]
fn test_no_flatten_json_schema_closes_additional_properties() {
    let schema = NoFlatten::json_schema();
    assert_eq!(
        schema["additionalProperties"],
        serde_json::Value::Bool(false)
    );
    assert!(schema.get("oneOf").is_none());
}

#[test]
fn test_flatten_recursive_base_contributes_its_fields() {
    let schema = FlatHolder::json_schema();
    assert!(schema.get("oneOf").is_none());
    assert_eq!(
        schema["additionalProperties"],
        serde_json::Value::Bool(false)
    );
    let props = schema["properties"].as_object().unwrap();
    assert!(props.contains_key("children"));
    assert!(props.contains_key("val"));
    assert!(props.contains_key("extra"));
    let req = schema["required"].as_array().unwrap();
    for name in ["children", "val", "extra"] {
        assert!(
            req.iter().any(|v| v.as_str() == Some(name)),
            "{name} missing from {req:?}"
        );
    }
}

/// The base's own self-reference is written from the container's root, so it has to resolve there.
#[test]
fn test_flatten_recursive_base_self_reference_resolves_from_the_container() {
    let schema = FlatHolder::json_schema();
    let reference = schema["properties"]["children"]["items"]["$ref"]
        .as_str()
        .unwrap();
    let pointer = reference.strip_prefix('#').unwrap();
    let resolved = schema.pointer(pointer).unwrap();
    let resolved_props = resolved["properties"].as_object().unwrap();
    assert!(resolved_props.contains_key("children"));
    assert!(resolved_props.contains_key("val"));
}

#[test]
#[should_panic(
    expected = "`CycleSecond`: `#[serde(flatten)]` of `CycleFirst` closes a flatten cycle"
)]
fn test_flatten_cycle_is_rejected_rather_than_described() {
    assert!(CycleFirst::json_schema().is_object());
}

#[test]
#[should_panic(
    expected = "`CycleFirst`: `#[serde(flatten)]` of `CycleSecond` closes a flatten cycle"
)]
fn test_flatten_cycle_is_rejected_from_either_end() {
    assert!(CycleSecond::json_schema().is_object());
}

#[test]
#[should_panic(
    expected = "`CycleUnionNode`: `#[serde(flatten)]` of `CycleUnionEither` closes a flatten cycle through a union member — its branch 1 is `CycleUnionNode`"
)]
fn test_a_flatten_cycle_through_a_union_member_is_rejected() {
    assert!(CycleUnionNode::json_schema().is_object());
}

/// The remedy is the one a cycle closed through the value itself names.
#[test]
#[should_panic(
    expected = "write the field as a named member so the cycle defers through a reference"
)]
fn test_the_union_member_cycle_refusal_names_the_remedy() {
    assert!(CycleUnionNode::json_schema().is_object());
}

/// Flattening a base that does not name itself writes the document it wrote before, byte for byte.
#[test]
fn test_non_recursive_flatten_documents_are_byte_identical() {
    assert_eq!(
        serde_json::to_string(&DataElementSampleValueEntry::json_schema()).unwrap(),
        r#"{"type":"object","oneOf":[{"type":"object","properties":{"dataElementId":{"type":"string"},"dataType":{"type":"string","const":"Alphanumeric"},"sampleValues":{"type":"array","items":{"type":"string"}}},"required":["dataElementId","dataType","sampleValues"],"additionalProperties":false},{"type":"object","properties":{"dataElementId":{"type":"string"},"dataType":{"type":"string","const":"Logical"},"sampleValues":{"type":"array","items":{"type":"boolean"}}},"required":["dataElementId","dataType","sampleValues"],"additionalProperties":false},{"type":"object","properties":{"dataElementId":{"type":"string"},"dataType":{"type":"string","const":"Numeric"},"sampleValues":{"type":"array","items":{"type":"integer"}}},"required":["dataElementId","dataType","sampleValues"],"additionalProperties":false}]}"#
    );
    assert_eq!(
        serde_json::to_string(&MultiFlatten::json_schema()).unwrap(),
        r#"{"type":"object","properties":{"id":{"type":"string"},"owner":{"type":"string"},"priority":{"type":"integer"}},"required":["id","owner","priority"],"additionalProperties":false}"#
    );
    assert_eq!(
        serde_json::to_string(&FlattenOnly::json_schema()).unwrap(),
        r#"{"type":"object","oneOf":[{"type":"object","properties":{"dataType":{"type":"string","const":"Alphanumeric"},"sampleValues":{"type":"array","items":{"type":"string"}}},"required":["dataType","sampleValues"],"additionalProperties":false},{"type":"object","properties":{"dataType":{"type":"string","const":"Logical"},"sampleValues":{"type":"array","items":{"type":"boolean"}}},"required":["dataType","sampleValues"],"additionalProperties":false},{"type":"object","properties":{"dataType":{"type":"string","const":"Numeric"},"sampleValues":{"type":"array","items":{"type":"integer"}}},"required":["dataType","sampleValues"],"additionalProperties":false}]}"#
    );
    assert_eq!(
        serde_json::to_string(&NoFlatten::json_schema()).unwrap(),
        r#"{"type":"object","additionalProperties":false,"properties":{"id":{"type":"string"},"name":{"type":"string"}},"required":["id","name"]}"#
    );
}

#[test]
fn test_the_deferred_flatten_document_is_byte_identical() {
    assert_eq!(
        serde_json::to_string(&FlatHolder::json_schema()).unwrap(),
        r##"{"$defs":{"FlatNode":{"type":"object","additionalProperties":false,"properties":{"children":{"type":"array","items":{"$ref":"#/$defs/FlatNode"}},"val":{"type":"string"}},"required":["children","val"]}},"type":"object","properties":{"extra":{"type":"string"},"children":{"type":"array","items":{"$ref":"#/$defs/FlatNode"}},"val":{"type":"string"}},"required":["extra","children","val"],"additionalProperties":false}"##
    );
}

#[test]
fn test_flatten_serialization_is_flat() {
    let entry = DataElementSampleValueEntryDraft {
        data_element_id: "abc".to_owned(),
        variant: DataElementSampleValueVariantDraft::Numeric {
            sample_values: vec![1_i64, 2_i64, 3_i64],
        }
        .validate()
        .unwrap(),
    };
    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json["dataElementId"], "abc");
    assert_eq!(json["dataType"], "Numeric");
    assert_eq!(
        json["sampleValues"],
        serde_json::json!([1_i64, 2_i64, 3_i64])
    );
    assert!(json.get("variant").is_none());

    let back: DataElementSampleValueEntry = serde_json::from_value(json).unwrap();
    assert_eq!(back, entry.validate().unwrap());
}

#[test]
fn test_flatten_recursive_base_serializes_flat() {
    let holder = FlatHolderDraft {
        base: FlatNodeDraft {
            children: vec![
                FlatNodeDraft {
                    children: Vec::new(),
                    val: "leaf".to_owned(),
                }
                .validate()
                .unwrap(),
            ],
            val: "root".to_owned(),
        }
        .validate()
        .unwrap(),
        extra: "x".to_owned(),
    };
    let json = serde_json::to_value(&holder).unwrap();
    assert_eq!(json["val"], "root");
    assert_eq!(json["extra"], "x");
    assert_eq!(json["children"][0]["val"], "leaf");
    assert!(json.get("base").is_none());

    let back: FlatHolder = serde_json::from_value(json).unwrap();
    assert_eq!(back, holder.validate().unwrap());
}

#[test]
fn test_flattening_a_plain_enum_writes_a_key_the_struct_does_not_name() {
    assert_eq!(
        serde_json::to_value(FlatOverEnum {
            own: "o".to_owned(),
            tone: FlatHue::Red,
        })
        .unwrap(),
        serde_json::json!({ "own": "o", "Red": null })
    );
}

#[test]
fn test_flattening_an_untagged_enum_writes_the_matched_members_keys() {
    assert_eq!(
        serde_json::to_value(FlatOverUntaggedDraft {
            own: "o".to_owned(),
            either: FlatEitherDraft::First(
                FlatFirstDraft { a: "x".to_owned() }.validate().unwrap()
            )
            .validate()
            .unwrap(),
        })
        .unwrap(),
        serde_json::json!({ "own": "o", "a": "x" })
    );
    assert_eq!(
        serde_json::to_value(FlatOverUntaggedDraft {
            own: "o".to_owned(),
            either: FlatEitherDraft::Second(FlatSecondDraft { b: true }.validate().unwrap())
                .validate()
                .unwrap(),
        })
        .unwrap(),
        serde_json::json!({ "own": "o", "b": true })
    );
}

#[test]
fn test_flattening_an_untagged_enum_multiplies_the_base_over_its_members() {
    assert_eq!(
        serde_json::to_string(&FlatOverUntagged::json_schema()).unwrap(),
        r#"{"type":"object","anyOf":[{"type":"object","properties":{"own":{"type":"string"},"a":{"type":"string"}},"required":["own","a"],"additionalProperties":false},{"type":"object","properties":{"own":{"type":"string"},"b":{"type":"boolean"}},"required":["own","b"],"additionalProperties":false}]}"#
    );
}

#[test]
fn test_the_untagged_flatten_schema_accepts_every_payload_serde_writes() {
    let schema = FlatOverUntagged::json_schema();
    for payload in [
        serde_json::json!({ "own": "o", "a": "x" }),
        serde_json::json!({ "own": "o", "b": true }),
    ] {
        assert!(
            closed_document_accepts(&schema, &payload),
            "{payload} is rejected by {schema}"
        );
    }
    assert!(
        !closed_document_accepts(
            &serde_json::json!({
                "type": "object",
                "properties": { "own": { "type": "string" } },
                "required": ["own"],
                "additionalProperties": false
            }),
            &serde_json::json!({ "own": "o", "a": "x" })
        ),
        "the document closed around the base alone accepts a key it does not name"
    );
}

/// The `null` of a member over a type parameter filled at `()` is read as no key, which serde
/// does not read back: the limit the scope sentence and the README state. The holder is
/// `nonstrict`, as a holder of an untagged enum declared below it must be, so each branch reads past
/// a key it does not declare.
#[test]
fn test_a_unit_filling_of_an_untagged_enum_flattened_below_its_holder_is_read_as_no_key() {
    let holder = FlatOverLaterHeldUnitDraft {
        held: LaterHeldMemberDraft::Held(()).validate().unwrap(),
        id: "i".to_owned(),
    };
    let written = serde_json::to_value(&holder).unwrap();
    assert_eq!(written, serde_json::json!({ "id": "i" }));
    assert!(
        serde_json::from_value::<FlatOverLaterHeldUnit>(written).is_err(),
        "serde reads the object alone back"
    );
    let own = serde_json::json!({
        "type": "object",
        "properties": { "id": { "type": "string" } },
        "required": ["id"],
        "additionalProperties": true,
    });
    let fields = serde_json::json!({
        "type": "object",
        "properties": { "a": { "type": "string" }, "id": { "type": "string" } },
        "required": ["id", "a"],
        "additionalProperties": true,
    });
    assert_eq!(
        FlatOverLaterHeldUnit::json_schema(),
        serde_json::json!({ "type": "object", "anyOf": [fields, own] })
    );
}

/// An `Option` of a type declared below its holder offers its absence as before, flattened
/// directly and through a struct of one slot.
#[test]
fn test_an_option_flattened_below_its_holder_still_offers_its_absence() {
    let offered = |open: bool| {
        let own = serde_json::json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"],
            "additionalProperties": open,
        });
        let below = serde_json::json!({
            "type": "object",
            "properties": { "below": { "type": "string" }, "id": { "type": "string" } },
            "required": ["id", "below"],
            "additionalProperties": open,
        });
        serde_json::json!({ "type": "object", "anyOf": [below, own] })
    };
    for (holder, described, open) in [
        (
            "FlatOverLaterMaybeFields",
            FlatOverLaterMaybeFields::json_schema as fn() -> serde_json::Value,
            false,
        ),
        (
            "FlatOverLaterMaybeSlot",
            FlatOverLaterMaybeSlot::json_schema,
            false,
        ),
    ] {
        assert_eq!(described(), offered(open), "for {holder}");
    }
}

#[test]
fn test_flattening_a_self_naming_union_member_writes_its_keys() {
    assert_eq!(
        serde_json::to_value(FlatOverSelfUntaggedDraft {
            own: "o".to_owned(),
            either: FlatSelfEitherDraft::Node(
                FlatSelfNodeDraft {
                    kids: Vec::new(),
                    leaf: "l".to_owned(),
                }
                .validate()
                .unwrap()
            )
            .validate()
            .unwrap(),
        })
        .unwrap(),
        serde_json::json!({ "own": "o", "kids": [], "leaf": "l" })
    );
}

#[test]
fn test_a_deferred_union_member_merges_as_the_body_it_names() {
    assert_eq!(
        serde_json::to_string(&FlatOverSelfUntagged::json_schema()).unwrap(),
        r##"{"$defs":{"FlatSelfNode":{"type":"object","additionalProperties":false,"properties":{"kids":{"type":"array","items":{"$ref":"#/$defs/FlatSelfNode"}},"leaf":{"type":"string"}},"required":["kids","leaf"]}},"type":"object","properties":{"own":{"type":"string"},"kids":{"type":"array","items":{"$ref":"#/$defs/FlatSelfNode"}},"leaf":{"type":"string"}},"required":["own","kids","leaf"],"additionalProperties":false}"##
    );
}

#[test]
fn test_the_deferred_union_member_schema_accepts_the_payload_serde_writes() {
    let schema = FlatOverSelfUntagged::json_schema();
    let payload = serde_json::json!({ "own": "o", "kids": [], "leaf": "l" });
    assert!(
        closed_document_accepts(&schema, &payload),
        "{payload} is rejected by {schema}"
    );
    let reference = schema["properties"]["kids"]["items"]["$ref"]
        .as_str()
        .unwrap();
    let resolved = schema
        .pointer(reference.strip_prefix('#').unwrap())
        .unwrap();
    assert!(
        resolved["properties"]
            .as_object()
            .unwrap()
            .contains_key("leaf")
    );
}

#[test]
fn test_flattening_an_overlapping_untagged_enum_writes_the_narrower_members_keys() {
    let narrow = FlatOverOverlapDraft {
        own: "o".to_owned(),
        either: OverlapEitherDraft::Narrow(
            OverlapNarrowDraft { a: "x".to_owned() }.validate().unwrap(),
        )
        .validate()
        .unwrap(),
    };
    let written = serde_json::to_value(&narrow).unwrap();
    assert_eq!(written, serde_json::json!({ "own": "o", "a": "x" }));
    let back: FlatOverOverlap = serde_json::from_value(written).unwrap();
    assert_eq!(back, narrow.validate().unwrap());
}

#[test]
fn test_the_overlapping_payload_is_admitted_by_two_branches() {
    let schema = FlatOverOverlap::json_schema();
    assert_eq!(
        accepting_branches(
            &schema["anyOf"],
            &serde_json::json!({ "own": "o", "a": "x" })
        ),
        2,
        "Got: {schema}"
    );
}

#[test]
fn test_an_overlapping_untagged_flatten_keeps_the_any_of_spelling() {
    assert_eq!(
        serde_json::to_string(&FlatOverOverlap::json_schema()).unwrap(),
        r#"{"type":"object","anyOf":[{"type":"object","properties":{"own":{"type":"string"},"a":{"type":"string"}},"required":["own","a"],"additionalProperties":false},{"type":"object","properties":{"own":{"type":"string"},"a":{"type":"string"},"b":{"anyOf":[{"type":"string"},{"type":"null"}]}},"required":["own","a"],"additionalProperties":false}]}"#
    );
}

#[test]
fn test_the_overlapping_untagged_flatten_schema_accepts_every_payload_serde_writes() {
    let schema = FlatOverOverlap::json_schema();
    for payload in [
        serde_json::json!({ "own": "o", "a": "x" }),
        serde_json::json!({ "own": "o", "a": "x", "b": "y" }),
    ] {
        assert!(
            closed_document_accepts(&schema, &payload),
            "{payload} is rejected by {schema}"
        );
    }
}

#[test]
fn test_flattening_both_spellings_of_a_union_writes_every_members_keys() {
    let mixed = FlatOverMixedDraft {
        own: "o".to_owned(),
        tagged: MixedTaggedDraft::Left {
            left: "l".to_owned(),
        }
        .validate()
        .unwrap(),
        either: MixedEitherDraft::Partial(
            MixedPartialDraft { a: "x".to_owned() }.validate().unwrap(),
        )
        .validate()
        .unwrap(),
    };
    let written = serde_json::to_value(&mixed).unwrap();
    assert_eq!(
        written,
        serde_json::json!({ "kind": "Left", "left": "l", "a": "x", "own": "o" })
    );
    let back: FlatOverMixed = serde_json::from_value(written).unwrap();
    assert_eq!(back, mixed.validate().unwrap());
}

#[test]
fn test_a_mixed_merge_keeps_each_sources_wrapper() {
    let schema = FlatOverMixed::json_schema();
    let untagged = schema["anyOf"].as_array().unwrap();
    assert_eq!(untagged.len(), 2, "Got: {schema}");
    for branch in untagged {
        assert_eq!(
            branch["oneOf"].as_array().unwrap().len(),
            2,
            "Got: {schema}"
        );
    }
}

#[test]
fn test_the_mixed_merge_schema_accepts_every_payload_serde_writes() {
    let schema = FlatOverMixed::json_schema();
    for payload in [
        serde_json::json!({ "kind": "Left", "left": "l", "a": "x", "own": "o" }),
        serde_json::json!({ "kind": "Left", "left": "l", "a": "x", "b": "y", "own": "o" }),
        serde_json::json!({ "kind": "Right", "right": true, "a": "x", "own": "o" }),
        serde_json::json!({ "kind": "Right", "right": true, "a": "x", "b": "y", "own": "o" }),
    ] {
        assert!(
            closed_document_accepts(&schema, &payload),
            "{payload} is rejected by {schema}"
        );
    }
}

#[test]
fn test_flattening_a_nested_union_writes_the_leaf_members_keys() {
    let nested: [(NestEither, serde_json::Value); 3] = [
        (
            NestEitherDraft::Plain(
                NestPlainDraft {
                    plain: "p".to_owned(),
                }
                .validate()
                .unwrap(),
            )
            .validate()
            .unwrap(),
            serde_json::json!({ "own": "o", "plain": "p" }),
        ),
        (
            NestEitherDraft::Tagged(
                NestTaggedDraft::Left {
                    left: "l".to_owned(),
                }
                .validate()
                .unwrap(),
            )
            .validate()
            .unwrap(),
            serde_json::json!({ "own": "o", "kind": "Left", "left": "l" }),
        ),
        (
            NestEitherDraft::Tagged(NestTaggedDraft::Right { right: true }.validate().unwrap())
                .validate()
                .unwrap(),
            serde_json::json!({ "own": "o", "kind": "Right", "right": true }),
        ),
    ];
    for (either, expected) in nested {
        let holder = NestHolderDraft {
            own: "o".to_owned(),
            either,
        };
        let written = serde_json::to_value(&holder).unwrap();
        assert_eq!(written, expected);
        let back: NestHolder = serde_json::from_value(written).unwrap();
        assert_eq!(back, holder.validate().unwrap());
    }
}

#[test]
fn test_the_nested_union_schema_accepts_every_payload_serde_writes() {
    let schema = NestHolder::json_schema();
    for payload in [
        serde_json::json!({ "own": "o", "plain": "p" }),
        serde_json::json!({ "own": "o", "kind": "Left", "left": "l" }),
        serde_json::json!({ "own": "o", "kind": "Right", "right": true }),
    ] {
        assert!(
            closed_document_accepts(&schema, &payload),
            "{payload} is rejected by {schema}"
        );
    }
}

#[test]
fn test_a_nested_union_branch_expands_under_its_own_spelling() {
    assert_eq!(
        serde_json::to_string(&NestHolder::json_schema()).unwrap(),
        r#"{"type":"object","anyOf":[{"type":"object","properties":{"own":{"type":"string"},"plain":{"type":"string"}},"required":["own","plain"],"additionalProperties":false},{"type":"object","oneOf":[{"type":"object","properties":{"own":{"type":"string"},"kind":{"type":"string","const":"Left"},"left":{"type":"string"}},"required":["own","kind","left"],"additionalProperties":false},{"type":"object","properties":{"own":{"type":"string"},"kind":{"type":"string","const":"Right"},"right":{"type":"boolean"}},"required":["own","kind","right"],"additionalProperties":false}]}]}"#
    );
}

#[test]
fn test_flattening_a_single_member_nested_union_writes_the_leafs_keys() {
    let holder = NestOnlyHolderDraft {
        own: "o".to_owned(),
        either: NestOnlyOuterDraft::Inner(
            NestOnlyInnerDraft::A(NestOnlyDraft { a: "x".to_owned() }.validate().unwrap())
                .validate()
                .unwrap(),
        )
        .validate()
        .unwrap(),
    };
    let written = serde_json::to_value(&holder).unwrap();
    assert_eq!(written, serde_json::json!({ "own": "o", "a": "x" }));
    let back: NestOnlyHolder = serde_json::from_value(written).unwrap();
    assert_eq!(back, holder.validate().unwrap());
}

#[test]
fn test_a_single_member_nested_union_collapses_to_the_leafs_object() {
    assert_eq!(
        serde_json::to_string(&NestOnlyHolder::json_schema()).unwrap(),
        r#"{"type":"object","properties":{"own":{"type":"string"},"a":{"type":"string"}},"required":["own","a"],"additionalProperties":false}"#
    );
    assert!(closed_document_accepts(
        &NestOnlyHolder::json_schema(),
        &serde_json::json!({ "own": "o", "a": "x" })
    ));
}

#[test]
#[should_panic(
    expected = "`NestCycleHolder`: `#[serde(flatten)]` of `NestCycleOuter` closes a flatten cycle through nested unions — its branch 1.1 names `NestCycleOuter`, already expanding on the path `NestCycleOuter`"
)]
fn test_a_cycle_through_nested_unions_names_the_path_it_closes() {
    assert!(NestCycleHolder::json_schema().is_object());
}

/// The remedy is the one every flatten cycle names.
#[test]
#[should_panic(
    expected = "write the field as a named member so the cycle defers through a reference"
)]
fn test_the_nested_union_cycle_refusal_names_the_remedy() {
    assert!(NestCycleHolder::json_schema().is_object());
}

#[test]
fn test_an_optional_flattened_base_writes_its_members_or_nothing() {
    let forms: [(Option<OptBase>, serde_json::Value); 2] = [
        (
            Some(
                OptBaseDraft {
                    left: "l".to_owned(),
                    right: true,
                }
                .validate()
                .unwrap(),
            ),
            serde_json::json!({ "left": "l", "right": true, "own": "o" }),
        ),
        (None, serde_json::json!({ "own": "o" })),
    ];
    for (maybe, expected) in forms {
        let holder = OptHolderDraft {
            maybe,
            own: "o".to_owned(),
        };
        let written = serde_json::to_value(&holder).unwrap();
        assert_eq!(written, expected);
        let back: OptHolder = serde_json::from_value(written).unwrap();
        assert_eq!(back, holder.validate().unwrap());
    }
}

#[test]
fn test_an_optional_flattened_union_writes_the_matched_members_keys_or_nothing() {
    let forms: [(Option<NestTagged>, serde_json::Value); 3] = [
        (
            Some(
                NestTaggedDraft::Left {
                    left: "l".to_owned(),
                }
                .validate()
                .unwrap(),
            ),
            serde_json::json!({ "kind": "Left", "left": "l", "own": "o" }),
        ),
        (
            Some(NestTaggedDraft::Right { right: true }.validate().unwrap()),
            serde_json::json!({ "kind": "Right", "right": true, "own": "o" }),
        ),
        (None, serde_json::json!({ "own": "o" })),
    ];
    for (maybe, expected) in forms {
        let holder = OptUnionHolderDraft {
            maybe,
            own: "o".to_owned(),
        };
        let written = serde_json::to_value(&holder).unwrap();
        assert_eq!(written, expected);
        let back: OptUnionHolder = serde_json::from_value(written).unwrap();
        assert_eq!(back, holder.validate().unwrap());
    }
}

#[test]
fn test_the_optional_flatten_schema_accepts_every_payload_serde_writes() {
    let schema = OptHolder::json_schema();
    for payload in [
        serde_json::json!({ "left": "l", "right": true, "own": "o" }),
        serde_json::json!({ "own": "o" }),
    ] {
        assert!(
            closed_document_accepts(&schema, &payload),
            "{payload} is rejected by {schema}"
        );
    }
}

#[test]
fn test_the_optional_flatten_schema_rejects_a_partial_base() {
    let schema = OptHolder::json_schema();
    for payload in [
        serde_json::json!({ "left": "l", "own": "o" }),
        serde_json::json!({ "right": true, "own": "o" }),
    ] {
        assert!(
            !closed_document_accepts(&schema, &payload),
            "{payload} is accepted by {schema}"
        );
    }
}

#[test]
fn test_the_optional_flatten_document_offers_the_base_and_its_absence() {
    assert_eq!(
        serde_json::to_string(&OptHolder::json_schema()).unwrap(),
        r#"{"type":"object","anyOf":[{"type":"object","properties":{"own":{"type":"string"},"left":{"type":"string"},"right":{"type":"boolean"}},"required":["own","left","right"],"additionalProperties":false},{"type":"object","properties":{"own":{"type":"string"}},"required":["own"],"additionalProperties":false}]}"#
    );
}

#[test]
fn test_an_optional_flattened_union_offers_every_member_and_their_absence() {
    let schema = OptUnionHolder::json_schema();
    for payload in [
        serde_json::json!({ "kind": "Left", "left": "l", "own": "o" }),
        serde_json::json!({ "kind": "Right", "right": true, "own": "o" }),
        serde_json::json!({ "own": "o" }),
    ] {
        assert!(
            closed_document_accepts(&schema, &payload),
            "{payload} is rejected by {schema}"
        );
    }
    assert_eq!(
        serde_json::to_string(&schema).unwrap(),
        r#"{"type":"object","anyOf":[{"type":"object","oneOf":[{"type":"object","properties":{"own":{"type":"string"},"kind":{"type":"string","const":"Left"},"left":{"type":"string"}},"required":["own","kind","left"],"additionalProperties":true},{"type":"object","properties":{"own":{"type":"string"},"kind":{"type":"string","const":"Right"},"right":{"type":"boolean"}},"required":["own","kind","right"],"additionalProperties":true}]},{"type":"object","properties":{"own":{"type":"string"}},"required":["own"],"additionalProperties":true}]}"#
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_the_optional_flatten_type_offers_the_base_and_its_absence() {
    let ts = OptHolder::ts_definition();
    assert!(
        ts.contains(
            "} & (OptBaseDraft | { [K in Exclude<keyof OptBaseDraft, \"own\">]?: never });"
        ),
        "expected the base beside its absence, got: {ts}"
    );
    assert!(
        !ts.contains("| undefined"),
        "the whole value is offered as absent in: {ts}"
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_an_optional_flattened_union_type_offers_its_members_and_their_absence() {
    let ts = OptUnionHolder::ts_definition();
    assert!(
        ts.contains(
            "} & (NestTaggedDraft | { [K in Exclude<keyof NestTaggedDraft, \"own\">]?: never });"
        ),
        "expected the union beside its absence, got: {ts}"
    );
    assert!(
        !ts.contains("| undefined"),
        "the whole value is offered as absent in: {ts}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_the_optional_flatten_schema_offers_the_base_and_its_absence() {
    let zod = OptHolder::zod_schema();
    assert!(
        zod.contains(
            "z.union([\n  OptHolder$OwnSchema.and(z.lazy(() => OptBase$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  OptHolder$OwnSchema,\n])"
        ),
        "expected the base beside its absence, got: {zod}"
    );
    assert!(
        !zod.contains(".and(z.lazy(() => z.union("),
        "the choice is written inside the intersection in: {zod}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_the_optional_flatten_schema_admits_no_partial_base() {
    let zod = OptHolder::zod_schema();
    assert!(
        !zod.contains("z.undefined()") && !zod.contains(".prefault("),
        "the whole value is offered as absent in: {zod}"
    );
    assert!(
        !zod.contains(".optional()"),
        "the base's members are offered one at a time in: {zod}"
    );
    assert_eq!(
        zod.matches("OptBase$Schema").count(),
        1,
        "the base is named more than once in: {zod}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_the_optional_flatten_schema_binds_the_objects_own_keys_once() {
    let zod = OptHolder::zod_schema();
    assert!(
        zod.contains("const OptHolder$OwnSchema = z.strictObject({\n  own: z.string(),\n});"),
        "expected the object's own keys bound once, got: {zod}"
    );
    assert_eq!(
        zod.matches("z.strictObject(").count(),
        1,
        "the object's own keys are written more than once in: {zod}"
    );
}

#[test]
fn test_a_named_nullable_flattened_base_writes_its_members_or_nothing() {
    let forms: [(Option<OptBase>, serde_json::Value); 2] = [
        (
            Some(
                OptBaseDraft {
                    left: "l".to_owned(),
                    right: true,
                }
                .validate()
                .unwrap(),
            ),
            serde_json::json!({ "left": "l", "right": true, "own": "o" }),
        ),
        (None, serde_json::json!({ "own": "o" })),
    ];
    for (maybe, expected) in forms {
        let holder = NamedMaybeHolderDraft {
            maybe: MaybeOptBaseDraft(maybe).validate().unwrap(),
            own: "o".to_owned(),
        };
        let written = serde_json::to_value(&holder).unwrap();
        assert_eq!(written, expected);
        let back: NamedMaybeHolder = serde_json::from_value(written).unwrap();
        assert_eq!(back, holder.validate().unwrap());
    }
}

#[test]
fn test_the_named_nullable_flatten_document_is_the_optional_flattens_own() {
    assert_eq!(
        serde_json::to_string(&NamedMaybeHolder::json_schema()).unwrap(),
        serde_json::to_string(&OptHolder::json_schema()).unwrap(),
    );
    assert_eq!(
        serde_json::to_string(&NamedMaybeHolder::json_schema()).unwrap(),
        r#"{"type":"object","anyOf":[{"type":"object","properties":{"own":{"type":"string"},"left":{"type":"string"},"right":{"type":"boolean"}},"required":["own","left","right"],"additionalProperties":false},{"type":"object","properties":{"own":{"type":"string"}},"required":["own"],"additionalProperties":false}]}"#
    );
}

#[test]
fn test_the_named_nullable_flatten_document_admits_the_captured_payloads() {
    let schema = NamedMaybeHolder::json_schema();
    for payload in [
        serde_json::json!({ "left": "l", "right": true, "own": "o" }),
        serde_json::json!({ "own": "o" }),
    ] {
        assert!(
            closed_document_accepts(&schema, &payload),
            "{payload} is rejected by {schema}"
        );
    }
    for payload in [
        serde_json::json!({ "left": "l", "own": "o" }),
        serde_json::json!({ "right": true, "own": "o" }),
    ] {
        assert!(
            !closed_document_accepts(&schema, &payload),
            "{payload} is accepted by {schema}"
        );
    }
}

#[test]
#[cfg(feature = "zod")]
fn test_the_named_nullable_flatten_schema_offers_the_base_and_its_absence() {
    let zod = NamedMaybeHolder::zod_schema();
    assert!(
        zod.contains(
            "z.union([\n  NamedMaybeHolder$OwnSchema.and(z.lazy(() => MaybeOptBase$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  NamedMaybeHolder$OwnSchema,\n])"
        ),
        "expected the base beside its absence, got: {zod}"
    );
    assert_eq!(
        zod.matches("MaybeOptBase$Schema").count(),
        1,
        "the base is named more than once in: {zod}"
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_the_named_nullable_flatten_type_offers_the_base_and_its_absence() {
    let ts = NamedMaybeHolder::ts_definition();
    assert!(
        ts.contains(
            "} & (MaybeOptBaseDraft | { [K in Exclude<keyof NonNullable<MaybeOptBaseDraft>, \"own\">]?: \
             never });"
        ),
        "expected the base beside its absence, got: {ts}"
    );
    assert!(
        !ts.contains("| undefined"),
        "the whole value is offered as absent in: {ts}"
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_a_named_non_nullable_flatten_type_is_byte_identical() {
    let ts = NamedPlainHolder::ts_definition();
    let declared = &ts[ts.find("export type").unwrap()..];
    assert_eq!(
        declared,
        "export type NamedPlainHolderDraft = {\n  /**\n   * own\n   * \n   */\n  own: string;\n} & PlainOptBaseDraft;\n\nexport type NamedPlainHolder = Readonly<NamedPlainHolderDraft> & $brand<\"NamedPlainHolder\">;"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_a_named_non_nullable_flatten_schema_is_byte_identical() {
    #[cfg(feature = "typescript")]
    const EXPECTED: &str = "const NamedPlainHolderDraft$RawSchema = z.strictObject({\n  own: z.string(),\n}).and(z.lazy(() => PlainOptBase$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }));\n\nexport const NamedPlainHolderDraft$Schema: ZodType<NamedPlainHolderDraft> = NamedPlainHolderDraft$RawSchema;\n\nexport const NamedPlainHolder$Schema = NamedPlainHolderDraft$Schema.readonly().brand<\"NamedPlainHolder\">().meta(NamedPlainHolderDraft$Schema.meta() ?? {}).meta({\n  examples: []\n});";
    #[cfg(not(feature = "typescript"))]
    const EXPECTED: &str = "export const NamedPlainHolder$Schema = z.strictObject({\n  own: z.string(),\n}).and(z.lazy(() => PlainOptBase$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })).meta({\n  examples: []\n});";

    assert_eq!(NamedPlainHolder::zod_schema(), EXPECTED);
}

#[test]
fn test_a_named_non_nullable_flatten_document_is_byte_identical() {
    assert_eq!(
        serde_json::to_string(&NamedPlainHolder::json_schema()).unwrap(),
        r#"{"type":"object","properties":{"own":{"type":"string"},"left":{"type":"string"},"right":{"type":"boolean"}},"required":["own","left","right"],"additionalProperties":false}"#
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_a_non_optional_flatten_type_is_byte_identical() {
    let ts = MultiFlatten::ts_definition();
    let declared = &ts[ts.find("export type").unwrap()..];
    assert_eq!(
        declared,
        "export type MultiFlattenDraft = {\n  /**\n   * id\n   * \n   */\n  id: string;\n} & BasePartDraft & ExtraPartDraft;\n\nexport type MultiFlatten = Readonly<MultiFlattenDraft> & $brand<\"MultiFlatten\">;"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_a_non_optional_flatten_schema_is_byte_identical() {
    #[cfg(feature = "typescript")]
    const EXPECTED: &str = "const MultiFlattenDraft$RawSchema = z.strictObject({\n  id: z.string(),\n}).and(z.lazy(() => BasePart$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })).and(z.lazy(() => ExtraPart$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }));\n\nexport const MultiFlattenDraft$Schema: ZodType<MultiFlattenDraft> = MultiFlattenDraft$RawSchema;\n\nexport const MultiFlatten$Schema = MultiFlattenDraft$Schema.readonly().brand<\"MultiFlatten\">().meta(MultiFlattenDraft$Schema.meta() ?? {}).meta({\n  examples: []\n});";
    #[cfg(not(feature = "typescript"))]
    const EXPECTED: &str = "export const MultiFlatten$Schema = z.strictObject({\n  id: z.string(),\n}).and(z.lazy(() => BasePart$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })).and(z.lazy(() => ExtraPart$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })).meta({\n  examples: []\n});";

    assert_eq!(MultiFlatten::zod_schema(), EXPECTED);
}

#[test]
#[cfg(feature = "zod")]
fn test_the_untagged_flatten_schema_multiplies_the_object_over_the_unions_members() {
    #[cfg(feature = "typescript")]
    const EXPECTED: &str = "const FlatOverUntagged$OwnSchema = z.strictObject({\n  own: z.string(),\n});\n\nconst FlatOverUntaggedDraft$RawSchema = z.union([\n  FlatOverUntagged$OwnSchema.and(z.lazy(() => FlatFirst$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  FlatOverUntagged$OwnSchema.and(z.lazy(() => FlatSecond$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n]);\n\nexport const FlatOverUntaggedDraft$Schema: ZodType<FlatOverUntaggedDraft> = FlatOverUntaggedDraft$RawSchema;\n\nexport const FlatOverUntagged$Schema = FlatOverUntaggedDraft$Schema.readonly().brand<\"FlatOverUntagged\">().meta(FlatOverUntaggedDraft$Schema.meta() ?? {}).meta({\n  examples: []\n});";
    #[cfg(not(feature = "typescript"))]
    const EXPECTED: &str = "const FlatOverUntagged$OwnSchema = z.strictObject({\n  own: z.string(),\n});\n\nexport const FlatOverUntagged$Schema = z.union([\n  FlatOverUntagged$OwnSchema.and(z.lazy(() => FlatFirst$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  FlatOverUntagged$OwnSchema.and(z.lazy(() => FlatSecond$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n]).meta({\n  examples: []\n});";

    assert_eq!(FlatOverUntagged::zod_schema(), EXPECTED);
}

#[test]
#[cfg(feature = "zod")]
fn test_the_untagged_flatten_schema_names_no_union_as_an_operand() {
    let zod = FlatOverUntagged::zod_schema();
    assert!(
        !zod.contains("FlatEither"),
        "the union is named as an operand in: {zod}"
    );
    assert!(
        !zod.contains(".and(FlatFirst$Schema)") && !zod.contains(".and(FlatSecond$Schema)"),
        "a member is read eagerly in: {zod}"
    );
    assert_eq!(
        zod.matches("z.strictObject(").count(),
        1,
        "the object's own keys are written more than once in: {zod}"
    );
}

/// The two choices multiply.
#[test]
#[cfg(feature = "zod")]
fn test_an_optional_untagged_flatten_multiplies_the_members_and_the_absence() {
    let zod = FlatOverOptionalUntagged::zod_schema();
    assert!(
        zod.contains(
            "z.union([\n  FlatOverOptionalUntagged$OwnSchema.and(z.lazy(() => LooseFirst$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  FlatOverOptionalUntagged$OwnSchema.and(z.lazy(() => LooseSecond$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  FlatOverOptionalUntagged$OwnSchema,\n])"
        ),
        "expected every member beside the absence, got: {zod}"
    );
    assert!(
        !zod.contains("z.undefined()") && !zod.contains(".optional()"),
        "the absence is offered as a value rather than as a branch in: {zod}"
    );
}

#[test]
fn test_an_optional_flattened_untagged_enum_writes_a_members_keys_or_nothing() {
    let forms: [(Option<LooseEither>, serde_json::Value); 3] = [
        (
            Some(
                LooseEitherDraft::First(LooseFirstDraft { a: "x".to_owned() }.validate().unwrap())
                    .validate()
                    .unwrap(),
            ),
            serde_json::json!({ "a": "x", "own": "o" }),
        ),
        (
            Some(
                LooseEitherDraft::Second(LooseSecondDraft { b: true }.validate().unwrap())
                    .validate()
                    .unwrap(),
            ),
            serde_json::json!({ "b": true, "own": "o" }),
        ),
        (None, serde_json::json!({ "own": "o" })),
    ];
    for (either, expected) in forms {
        let holder = FlatOverOptionalUntaggedDraft {
            either,
            own: "o".to_owned(),
        };
        let written = serde_json::to_value(&holder).unwrap();
        assert_eq!(written, expected);
        let back: FlatOverOptionalUntagged = serde_json::from_value(written).unwrap();
        assert_eq!(back, holder.validate().unwrap());
    }
}

#[test]
#[cfg(feature = "zod")]
fn test_an_aliased_untagged_flatten_multiplies_over_the_same_members() {
    let zod = FlatOverAliasedUntagged::zod_schema();
    assert!(
        zod.contains(
            "z.union([\n  FlatOverAliasedUntagged$OwnSchema.and(z.lazy(() => FlatFirst$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  FlatOverAliasedUntagged$OwnSchema.and(z.lazy(() => FlatSecond$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n])"
        ),
        "expected the aliased union's members, got: {zod}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_a_nested_untagged_flatten_multiplies_over_the_leaf_members() {
    let nested = NestHolder::zod_schema();
    assert!(
        nested.contains(
            "z.union([\n  NestHolder$OwnSchema.and(z.lazy(() => NestPlain$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  NestHolder$OwnSchema.and(z.lazy(() => NestTagged$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n])"
        ),
        "expected the leaves of the nesting, got: {nested}"
    );
    let only = NestOnlyHolder::zod_schema();
    assert!(
        closes_on(
            &only,
            "}).and(z.lazy(() => NestOnly$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))"
        ),
        "expected the one leaf joined directly, got: {only}"
    );
    assert!(
        !only.contains("$OwnSchema") && !only.contains("z.union(["),
        "a choice of one is written as a choice in: {only}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_a_union_declared_below_the_object_is_named_as_one_operand() {
    let zod = FlatOverLaterUntagged::zod_schema();
    assert!(
        closes_on(
            &zod,
            "}).and(z.lazy(() => LaterEither$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))"
        ),
        "expected the union named as one operand, got: {zod}"
    );
    assert!(
        !zod.contains("$OwnSchema"),
        "a source with no members to multiply wrote a choice in: {zod}"
    );
}

/// A base Zod does read a key set off is untouched.
#[test]
#[cfg(feature = "zod")]
fn test_an_internally_tagged_flatten_schema_is_byte_identical() {
    #[cfg(feature = "typescript")]
    const EXPECTED: &str = "const DataElementSampleValueEntryDraft$RawSchema = z.strictObject({\n  dataElementId: z.string(),\n}).and(z.lazy(() => DataElementSampleValueVariant$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }));\n\nexport const DataElementSampleValueEntryDraft$Schema: ZodType<DataElementSampleValueEntryDraft> = DataElementSampleValueEntryDraft$RawSchema;\n\nexport const DataElementSampleValueEntry$Schema = DataElementSampleValueEntryDraft$Schema.readonly().brand<\"DataElementSampleValueEntry\">().meta(DataElementSampleValueEntryDraft$Schema.meta() ?? {}).meta({\n  examples: []\n});";
    #[cfg(not(feature = "typescript"))]
    const EXPECTED: &str = "export const DataElementSampleValueEntry$Schema = z.strictObject({\n  dataElementId: z.string(),\n}).and(z.lazy(() => DataElementSampleValueVariant$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })).meta({\n  examples: []\n});";

    assert_eq!(DataElementSampleValueEntry::zod_schema(), EXPECTED);
}

#[test]
#[cfg(feature = "zod")]
fn test_a_standalone_union_with_a_scalar_member_is_byte_identical() {
    #[cfg(feature = "typescript")]
    const EXPECTED: &str = "const FlatScalarEitherDraft$RawSchema = z.union([FlatFirst$Schema, z.string()]);\n\nexport const FlatScalarEitherDraft$Schema: ZodType<FlatScalarEitherDraft> = FlatScalarEitherDraft$RawSchema;\n\nexport const FlatScalarEither$Schema = FlatScalarEitherDraft$Schema.readonly().brand<\"FlatScalarEither\">().meta(FlatScalarEitherDraft$Schema.meta() ?? {}).meta({\n  examples: []\n});";
    #[cfg(not(feature = "typescript"))]
    const EXPECTED: &str = "export const FlatScalarEither$Schema = z.union([FlatFirst$Schema, z.string()]).meta({\n  examples: []\n});";

    assert_eq!(FlatScalarEither::zod_schema(), EXPECTED);
}

#[test]
#[cfg(feature = "zod")]
fn test_standalone_unions_the_merge_now_refuses_are_byte_identical() {
    #[cfg(feature = "typescript")]
    const EXPECTED: [&str; 4] = [
        "const FlatNullableEitherDraft$RawSchema = z.union([FlatFirst$Schema, z.nullable(FlatSecond$Schema)]);\n\nexport const FlatNullableEitherDraft$Schema: ZodType<FlatNullableEitherDraft> = FlatNullableEitherDraft$RawSchema;\n\nexport const FlatNullableEither$Schema = FlatNullableEitherDraft$Schema.readonly().brand<\"FlatNullableEither\">().meta(FlatNullableEitherDraft$Schema.meta() ?? {}).meta({\n  examples: []\n});",
        "const MemberSlugEitherDraft$RawSchema = z.union([FlatFirst$Schema, MemberSlug$Schema]);\n\nexport const MemberSlugEitherDraft$Schema: ZodType<MemberSlugEitherDraft> = MemberSlugEitherDraft$RawSchema;\n\nexport const MemberSlugEither$Schema = MemberSlugEitherDraft$Schema.readonly().brand<\"MemberSlugEither\">().meta(MemberSlugEitherDraft$Schema.meta() ?? {}).meta({\n  examples: []\n});",
        "const MemberSwitchEitherDraft$RawSchema = z.union([FlatFirst$Schema, MemberSwitch$Schema]);\n\nexport const MemberSwitchEitherDraft$Schema: ZodType<MemberSwitchEitherDraft> = MemberSwitchEitherDraft$RawSchema;\n\nexport const MemberSwitchEither$Schema = MemberSwitchEitherDraft$Schema.readonly().brand<\"MemberSwitchEither\">().meta(MemberSwitchEitherDraft$Schema.meta() ?? {}).meta({\n  examples: []\n});",
        "const MemberHueEitherDraft$RawSchema = z.union([FlatFirst$Schema, MemberHue$Schema]);\n\nexport const MemberHueEitherDraft$Schema: ZodType<MemberHueEitherDraft> = MemberHueEitherDraft$RawSchema;\n\nexport const MemberHueEither$Schema = MemberHueEitherDraft$Schema.readonly().brand<\"MemberHueEither\">().meta(MemberHueEitherDraft$Schema.meta() ?? {}).meta({\n  examples: []\n});",
    ];
    #[cfg(not(feature = "typescript"))]
    const EXPECTED: [&str; 4] = [
        "export const FlatNullableEither$Schema = z.union([FlatFirst$Schema, z.nullable(FlatSecond$Schema)]).meta({\n  examples: []\n});",
        "export const MemberSlugEither$Schema = z.union([FlatFirst$Schema, MemberSlug$Schema]).meta({\n  examples: []\n});",
        "export const MemberSwitchEither$Schema = z.union([FlatFirst$Schema, MemberSwitch$Schema]).meta({\n  examples: []\n});",
        "export const MemberHueEither$Schema = z.union([FlatFirst$Schema, MemberHue$Schema]).meta({\n  examples: []\n});",
    ];

    assert_eq!(
        [
            FlatNullableEither::zod_schema(),
            MemberSlugEither::zod_schema(),
            MemberSwitchEither::zod_schema(),
            MemberHueEither::zod_schema(),
        ],
        EXPECTED.map(str::to_owned)
    );
}

#[test]
fn test_a_named_map_member_of_a_flattened_untagged_enum_round_trips() {
    let written = serde_json::to_value(FlatOverMemberBucketUntaggedDraft {
        either: MemberBucketEitherDraft::Bucket(
            MemberBucketDraft(HashMap::from([("k".to_owned(), "v".to_owned())]))
                .validate()
                .unwrap(),
        )
        .validate()
        .unwrap(),
    })
    .unwrap();
    assert_eq!(written, serde_json::json!({ "k": "v" }));
    serde_json::from_value::<FlatOverMemberBucketUntagged>(written).unwrap();
}

#[test]
#[cfg(feature = "zod")]
fn test_a_named_map_member_keeps_its_multiplication() {
    let zod = FlatOverMemberBucketUntagged::zod_schema();
    assert!(
        zod.contains(
            "z.union([\n  FlatOverMemberBucketUntagged$OwnSchema.and(z.lazy(() => FlatFirst$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  FlatOverMemberBucketUntagged$OwnSchema.and(z.lazy(() => MemberBucket$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n])"
        ),
        "expected the map member's branch, got: {zod}"
    );
}

#[test]
fn test_a_named_map_member_keeps_its_merged_document() {
    assert!(FlatOverMemberBucketUntagged::json_schema().is_object());
}

#[test]
#[cfg(feature = "zod")]
fn test_the_named_wire_unions_are_byte_identical_standing_alone() {
    #[cfg(feature = "typescript")]
    const EXPECTED: [&str; 3] = [
        "const MemberBagEitherDraft$RawSchema = z.union([FlatFirst$Schema, MemberBag$Schema]);\n\nexport const MemberBagEitherDraft$Schema: ZodType<MemberBagEitherDraft> = MemberBagEitherDraft$RawSchema;\n\nexport const MemberBagEither$Schema = MemberBagEitherDraft$Schema.readonly().brand<\"MemberBagEither\">().meta(MemberBagEitherDraft$Schema.meta() ?? {}).meta({\n  examples: []\n});",
        "const MemberBucketEitherDraft$RawSchema = z.union([FlatFirst$Schema, MemberBucket$Schema]);\n\nexport const MemberBucketEitherDraft$Schema: ZodType<MemberBucketEitherDraft> = MemberBucketEitherDraft$RawSchema;\n\nexport const MemberBucketEither$Schema = MemberBucketEitherDraft$Schema.readonly().brand<\"MemberBucketEither\">().meta(MemberBucketEitherDraft$Schema.meta() ?? {}).meta({\n  examples: []\n});",
        "const MemberMaybeEitherDraft$RawSchema = z.union([FlatFirst$Schema, MemberMaybeSecond$Schema]);\n\nexport const MemberMaybeEitherDraft$Schema: ZodType<MemberMaybeEitherDraft> = MemberMaybeEitherDraft$RawSchema;\n\nexport const MemberMaybeEither$Schema = MemberMaybeEitherDraft$Schema.readonly().brand<\"MemberMaybeEither\">().meta(MemberMaybeEitherDraft$Schema.meta() ?? {}).meta({\n  examples: []\n});",
    ];
    #[cfg(not(feature = "typescript"))]
    const EXPECTED: [&str; 3] = [
        "export const MemberBagEither$Schema = z.union([FlatFirst$Schema, MemberBag$Schema]).meta({\n  examples: []\n});",
        "export const MemberBucketEither$Schema = z.union([FlatFirst$Schema, MemberBucket$Schema]).meta({\n  examples: []\n});",
        "export const MemberMaybeEither$Schema = z.union([FlatFirst$Schema, MemberMaybeSecond$Schema]).meta({\n  examples: []\n});",
    ];

    assert_eq!(
        [
            MemberBagEither::zod_schema(),
            MemberBucketEither::zod_schema(),
            MemberMaybeEither::zod_schema(),
        ],
        EXPECTED.map(str::to_owned)
    );
}

#[test]
fn test_an_all_object_tagged_enum_member_round_trips() {
    let written = serde_json::to_value(FlatOverMemberExtObjUntaggedDraft {
        own: "o".to_owned(),
        either: MemberExtObjEitherDraft::Ext(
            MemberExtObjectsDraft::One(FlatFirstDraft { a: "x".to_owned() }.validate().unwrap())
                .validate()
                .unwrap(),
        )
        .validate()
        .unwrap(),
    })
    .unwrap();
    assert_eq!(
        written,
        serde_json::json!({ "own": "o", "One": { "a": "x" } })
    );
    serde_json::from_value::<FlatOverMemberExtObjUntagged>(written).unwrap();
}

#[test]
#[cfg(feature = "zod")]
fn test_an_all_object_tagged_enum_member_keeps_its_multiplication() {
    let zod = FlatOverMemberExtObjUntagged::zod_schema();
    assert!(
        zod.contains(
            "z.union([\n  FlatOverMemberExtObjUntagged$OwnSchema.and(z.lazy(() => FlatFirst$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  FlatOverMemberExtObjUntagged$OwnSchema.and(z.lazy(() => MemberExtObjects$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n])"
        ),
        "expected the tagged enum's branch, got: {zod}"
    );
}

#[test]
fn test_an_all_object_tagged_enum_member_keeps_its_merged_document() {
    assert!(FlatOverMemberExtObjUntagged::json_schema().is_object());
}

#[test]
#[cfg(feature = "zod")]
fn test_the_tagged_enums_are_byte_identical_standing_alone() {
    let bare = one_key(
        "z.union([z.literal(\"Bare\"), z.strictObject({\n  \"Wrapped\": FlatSecond$Schema,\n})])",
        &[("Wrapped", "newtype variant")],
    );
    let objects = one_key(
        "z.union([z.strictObject({\n  \"One\": FlatFirst$Schema,\n}), z.strictObject({\n  \"Two\": FlatSecond$Schema,\n})])",
        &[("One", "newtype variant"), ("Two", "newtype variant")],
    );
    #[cfg(feature = "typescript")]
    let expected = [
        format!(
            "const MemberExtBareDraft$RawSchema = {bare};\n\nexport const MemberExtBareDraft$Schema: ZodType<MemberExtBareDraft> = MemberExtBareDraft$RawSchema;\n\nexport const MemberExtBare$Schema = MemberExtBareDraft$Schema.readonly().brand<\"MemberExtBare\">().meta(MemberExtBareDraft$Schema.meta() ?? {{}}).meta({{\n  examples: []\n}});"
        ),
        format!(
            "const MemberExtObjectsDraft$RawSchema = {objects};\n\nexport const MemberExtObjectsDraft$Schema: ZodType<MemberExtObjectsDraft> = MemberExtObjectsDraft$RawSchema;\n\nexport const MemberExtObjects$Schema = MemberExtObjectsDraft$Schema.readonly().brand<\"MemberExtObjects\">().meta(MemberExtObjectsDraft$Schema.meta() ?? {{}}).meta({{\n  examples: []\n}});"
        ),
    ];
    #[cfg(not(feature = "typescript"))]
    let expected = [
        format!(
            "export const MemberExtBareDraft$Schema = {bare};\n\nexport const MemberExtBare$Schema = MemberExtBareDraft$Schema.readonly().brand().meta(MemberExtBareDraft$Schema.meta() ?? {{}}).meta({{\n  examples: []\n}});"
        ),
        format!(
            "export const MemberExtObjectsDraft$Schema = {objects};\n\nexport const MemberExtObjects$Schema = MemberExtObjectsDraft$Schema.readonly().brand().meta(MemberExtObjectsDraft$Schema.meta() ?? {{}}).meta({{\n  examples: []\n}});"
        ),
    ];

    assert_eq!(
        [MemberExtBare::zod_schema(), MemberExtObjects::zod_schema(),],
        expected
    );
}

#[test]
fn test_a_directly_flattened_tagged_enum_round_trips_every_variant() {
    let forms = [
        (
            MemberExtBareDraft::Bare,
            serde_json::json!({ "Bare": null, "own": "o" }),
        ),
        (
            MemberExtBareDraft::Wrapped(FlatSecondDraft { b: true }.validate().unwrap()),
            serde_json::json!({ "Wrapped": { "b": true }, "own": "o" }),
        ),
    ];
    for (ext, expected) in forms {
        let holder = ExtBareDirectHolderDraft {
            ext: ext.validate().unwrap(),
            own: "o".to_owned(),
        };
        let written = serde_json::to_value(&holder).unwrap();
        assert_eq!(written, expected);
        let back: ExtBareDirectHolder = serde_json::from_value(written).unwrap();
        assert_eq!(back, holder.validate().unwrap());
    }
}

#[test]
fn test_the_direct_tagged_flatten_document_writes_one_branch_per_variant() {
    assert_eq!(
        serde_json::to_string(&ExtBareDirectHolder::json_schema()).unwrap(),
        r#"{"type":"object","oneOf":[{"type":"object","properties":{"own":{"type":"string"},"Bare":{"type":"null"}},"required":["own","Bare"],"additionalProperties":false},{"type":"object","properties":{"own":{"type":"string"},"Wrapped":{"type":"object","additionalProperties":false,"properties":{"b":{"type":"boolean"}},"required":["b"]}},"required":["own","Wrapped"],"additionalProperties":false}]}"#
    );
}

#[test]
fn test_the_direct_tagged_flatten_document_admits_the_captured_payloads() {
    let schema = ExtBareDirectHolder::json_schema();
    for payload in [
        serde_json::json!({ "Bare": null, "own": "o" }),
        serde_json::json!({ "Wrapped": { "b": true }, "own": "o" }),
    ] {
        assert!(
            closed_document_accepts(&schema, &payload),
            "{payload} is rejected by {schema}"
        );
    }
    for payload in [
        serde_json::json!({ "own": "o" }),
        serde_json::json!({ "Bare": null, "Wrapped": { "b": true }, "own": "o" }),
        serde_json::json!({ "Bare": null }),
    ] {
        assert!(
            !closed_document_accepts(&schema, &payload),
            "{payload} is accepted by {schema}"
        );
    }
}

#[test]
#[cfg(feature = "zod")]
fn test_the_direct_tagged_flatten_schema_multiplies_the_object_over_the_variants() {
    let zod = ExtBareDirectHolder::zod_schema();
    assert!(
        zod.contains(
            "z.union([\n  ExtBareDirectHolder$OwnSchema.and(z.lazy(() => z.strictObject({\n  \"Bare\": z.null(),\n})).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  ExtBareDirectHolder$OwnSchema.and(z.lazy(() => z.strictObject({\n  \"Wrapped\": FlatSecond$Schema,\n})).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n])"
        ),
        "expected one operand per variant, got: {zod}"
    );
    assert!(
        !zod.contains("MemberExtBare$Schema"),
        "the union is named as an operand in: {zod}"
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_the_direct_tagged_flatten_type_spells_the_key_set_serde_writes() {
    let ts = ExtBareDirectHolder::ts_definition();
    assert!(
        ts.contains(
            "} & ({\n  /**\n   * Bare\n   * \n   */\n  \"Bare\": null;\n  \"Wrapped\"?: never;\n} | {\n  /**\n   * Wrapped\n   * \n   */\n  \"Wrapped\": FlatSecond;\n  \"Bare\"?: never;\n});"
        ),
        "expected one key set per variant, got: {ts}"
    );
    assert!(
        !ts.contains("& MemberExtBare"),
        "the union is named as an operand in: {ts}"
    );
}

#[test]
fn test_a_directly_flattened_all_object_tagged_enum_round_trips_every_variant() {
    let forms = [
        (
            MemberExtObjectsDraft::One(FlatFirstDraft { a: "x".to_owned() }.validate().unwrap()),
            serde_json::json!({ "One": { "a": "x" }, "own": "o" }),
        ),
        (
            MemberExtObjectsDraft::Two(FlatSecondDraft { b: true }.validate().unwrap()),
            serde_json::json!({ "Two": { "b": true }, "own": "o" }),
        ),
    ];
    for (ext, expected) in forms {
        let holder = ExtObjDirectHolderDraft {
            ext: ext.validate().unwrap(),
            own: "o".to_owned(),
        };
        let written = serde_json::to_value(&holder).unwrap();
        assert_eq!(written, expected);
        let back: ExtObjDirectHolder = serde_json::from_value(written).unwrap();
        assert_eq!(back, holder.validate().unwrap());
    }
}

#[test]
#[cfg(feature = "zod")]
fn test_the_all_object_tagged_direct_flatten_schema_multiplies_the_object_over_the_variants() {
    let zod = ExtObjDirectHolder::zod_schema();
    assert!(
        zod.contains(
            "z.union([\n  ExtObjDirectHolder$OwnSchema.and(z.lazy(() => z.strictObject({\n  \"One\": FlatFirst$Schema,\n})).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  ExtObjDirectHolder$OwnSchema.and(z.lazy(() => z.strictObject({\n  \"Two\": FlatSecond$Schema,\n})).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n])"
        ),
        "expected one operand per variant, got: {zod}"
    );
    assert!(
        !zod.contains("MemberExtObjects$Schema"),
        "the union is named as an operand in: {zod}"
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_an_all_object_tagged_direct_flatten_type_closes_each_variant_against_the_other() {
    let ts = ExtObjDirectHolder::ts_definition();
    let declared = &ts[ts.find("export type").unwrap()..];
    assert_eq!(
        declared,
        "export type ExtObjDirectHolderDraft = {\n  /**\n   * own\n   * \n   */\n  own: string;\n} & ({\n  /**\n   * One\n   * \n   */\n  \"One\": FlatFirst;\n  \"Two\"?: never;\n} | {\n  /**\n   * Two\n   * \n   */\n  \"Two\": FlatSecond;\n  \"One\"?: never;\n});\n\nexport type ExtObjDirectHolder = Readonly<ExtObjDirectHolderDraft> & $brand<\"ExtObjDirectHolder\">;"
    );
}

#[test]
fn test_a_flattened_enum_written_with_an_argument_round_trips_every_variant() {
    for (ext, expected) in [
        (
            MemberExtCarriedDraft::Bare,
            serde_json::json!({ "Bare": null, "own": "o" }),
        ),
        (
            MemberExtCarriedDraft::Carried(7),
            serde_json::json!({ "Carried": 7_u32, "own": "o" }),
        ),
    ] {
        let holder = ExtCarriedHolderDraft {
            ext: ext.validate().unwrap(),
            own: "o".to_owned(),
        };
        let written = serde_json::to_value(&holder).unwrap();
        assert_eq!(written, expected);
        let back: ExtCarriedHolder = serde_json::from_value(written).unwrap();
        assert_eq!(back, holder.validate().unwrap());
    }
}

/// Each holder of [`MemberExtCarried`] filled with an argument, beside the TypeScript and the Zod
/// text the argument is written as in the variant that holds the parameter.
#[cfg(any(feature = "zod", feature = "typescript"))]
fn carried_holders() -> [(&'static str, String, String, &'static str, &'static str); 3] {
    #[cfg(feature = "typescript")]
    let ts = [
        ExtCarriedHolder::ts_definition(),
        ExtCarriedOwnHolder::<bool>::ts_definition(),
        ExtCarriedAliasHolder::ts_definition(),
    ];
    #[cfg(not(feature = "typescript"))]
    let ts = [String::new(), String::new(), String::new()];
    #[cfg(feature = "zod")]
    let zod = [
        ExtCarriedHolder::zod_schema(),
        ExtCarriedOwnHolder::<bool>::zod_schema(),
        ExtCarriedAliasHolder::zod_schema(),
    ];
    #[cfg(not(feature = "zod"))]
    let zod = [String::new(), String::new(), String::new()];
    let [own_ts, held_ts, alias_ts] = ts;
    let [own_zod, held_zod, alias_zod] = zod;
    [
        (
            "ExtCarriedHolder",
            own_ts,
            own_zod,
            "number",
            "z.number().int().min(0).max(4294967295)",
        ),
        ("ExtCarriedOwnHolder", held_ts, held_zod, "Held", "held"),
        (
            "ExtCarriedAliasHolder",
            alias_ts,
            alias_zod,
            "boolean",
            "z.boolean()",
        ),
    ]
}

/// TypeScript joins each variant's entry of an enum written with an argument, the parameter
/// written as the argument, and names no parameter the holder does not declare.
#[test]
#[cfg(feature = "typescript")]
fn test_a_flattened_enum_written_with_an_argument_types_each_entry_at_the_argument() {
    for (holder, ts, _, argument, _) in carried_holders() {
        assert!(
            ts.contains(&format!(
                "}} & ({{\n  /**\n   * Bare\n   * \n   */\n  \"Bare\": null;\n  \"Carried\"?: \
                 never;\n}} | {{\n  /**\n   * Carried\n   * \n   */\n  \"Carried\": \
                 {argument};\n  \"Bare\"?: never;\n}});"
            )),
            "{holder}: expected one key set per variant, got: {ts}"
        );
        for unbound in ["Load", "Stowed", "__tixschema_parameter_"] {
            assert!(!ts.contains(unbound), "{holder} names `{unbound}`: {ts}");
        }
    }
}

/// Zod joins each variant's entry of an enum written with an argument, the parameter written as
/// the argument, and names no parameter the holder's factory does not bind.
#[test]
#[cfg(feature = "zod")]
fn test_a_flattened_enum_written_with_an_argument_parses_each_entry_at_the_argument() {
    for (holder, _, zod, _, argument) in carried_holders() {
        for entry in [
            "z.strictObject({\n  \"Bare\": z.null(),\n})".to_owned(),
            format!("z.strictObject({{\n  \"Carried\": {argument},\n}})"),
        ] {
            assert!(
                zod.contains(&format!(".and(z.lazy(() => {entry}).superRefine((_read, ctx) => {{ ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => {{ const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? {{ ...issue, path: [] }} : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? {{ ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] }} : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return {{ ...issue, path: [last] }}; return width || (!absent && refused) ? {{ ...issue, path: [] }} : issue; }})); }}, {{ when: () => true }}))")),
                "{holder}: no operand `{entry}` in: {zod}"
            );
        }
        for unbound in [
            "load",
            "stowed",
            "__tixschema_parameter_",
            "MemberExtCarried$Schema",
        ] {
            assert!(!zod.contains(unbound), "{holder} names `{unbound}`: {zod}");
        }
    }
}

/// The enum's own text names its own parameter, as it did before any holder filled it.
#[test]
#[cfg(all(feature = "zod", feature = "typescript"))]
fn test_a_generic_enum_flattened_with_an_argument_keeps_its_own_text() {
    let ts = MemberExtCarried::<String>::ts_definition();
    assert!(
        ts.contains("\"Carried\": Load;") && !ts.contains("__tixschema_parameter_"),
        "Got: {ts}"
    );
    let zod = MemberExtCarried::<String>::zod_schema();
    assert!(
        zod.contains("z.union([z.literal(\"Bare\"), z.strictObject({\n  \"Carried\": load,\n})])")
            && !zod.contains("__tixschema_parameter_"),
        "Got: {zod}"
    );
}

/// The JSON Schema of each holder admits each variant's entry at the argument, and refuses the
/// holder's own member alone, two entries and an entry of another type.
#[test]
fn test_a_flattened_enum_written_with_an_argument_documents_each_entry_at_the_argument() {
    for (schema, carried, other) in [
        (
            ExtCarriedHolder::json_schema(),
            serde_json::json!(7_u32),
            serde_json::json!(true),
        ),
        (
            ExtCarriedAliasHolder::json_schema(),
            serde_json::json!(true),
            serde_json::json!(7_u32),
        ),
    ] {
        for payload in [
            serde_json::json!({ "Bare": null, "own": "o" }),
            serde_json::json!({ "Carried": carried, "own": "o" }),
        ] {
            assert!(
                super::documents::admits(&schema, &payload),
                "{payload} is rejected by {schema}"
            );
        }
        for payload in [
            serde_json::json!({ "own": "o" }),
            serde_json::json!({ "Bare": null, "Carried": carried, "own": "o" }),
            serde_json::json!({ "Carried": other, "own": "o" }),
        ] {
            assert!(
                !super::documents::admits(&schema, &payload),
                "{payload} is accepted by {schema}"
            );
        }
    }
}

#[test]
fn test_a_flattened_untagged_enum_written_with_an_argument_round_trips_every_member() {
    let owner = BasePartDraft {
        owner: "w".to_owned(),
    };
    for (either, expected) in [
        (
            MemberUntaggedCarriedDraft::Held(owner.validate().unwrap()),
            serde_json::json!({ "owner": "w", "own": "o" }),
        ),
        (
            MemberUntaggedCarriedDraft::Counted { count: 3 },
            serde_json::json!({ "count": 3_u32, "own": "o" }),
        ),
    ] {
        let holder = UntaggedCarriedHolderDraft {
            either: either.validate().unwrap(),
            own: "o".to_owned(),
        };
        let written = serde_json::to_value(&holder).unwrap();
        assert_eq!(written, expected);
        let back: UntaggedCarriedHolder = serde_json::from_value(written).unwrap();
        assert_eq!(back, holder.validate().unwrap());
    }
    for (pick, expected) in [
        (
            MemberUntaggedPickedDraft::Carried { load: 7 },
            serde_json::json!({ "load": 7_u32, "own": "o" }),
        ),
        (
            MemberUntaggedPickedDraft::Counted { count: 3 },
            serde_json::json!({ "count": 3_u32, "own": "o" }),
        ),
    ] {
        let holder = UntaggedPickedHolderDraft {
            pick: pick.validate().unwrap(),
            own: "o".to_owned(),
        };
        let written = serde_json::to_value(&holder).unwrap();
        assert_eq!(written, expected);
        let back: UntaggedPickedHolder = serde_json::from_value(written).unwrap();
        assert_eq!(back, holder.validate().unwrap());
    }
}

/// Zod joins each member of an untagged enum written with an argument, the parameter written as
/// the argument, and names no parameter the holder's factory does not bind.
#[test]
#[cfg(feature = "zod")]
fn test_a_flattened_untagged_enum_written_with_an_argument_parses_each_member_at_the_argument() {
    let counted = "z.strictObject({ count: z.number().int().min(0).max(4294967295), })";
    for (holder, zod, members) in [
        (
            "UntaggedCarriedHolder",
            UntaggedCarriedHolder::zod_schema(),
            ["BasePart$Schema", counted],
        ),
        (
            "UntaggedPickedHolder",
            UntaggedPickedHolder::zod_schema(),
            [
                "z.strictObject({ load: z.number().int().min(0).max(4294967295), })",
                counted,
            ],
        ),
        (
            "UntaggedPickedAliasHolder",
            UntaggedPickedAliasHolder::zod_schema(),
            ["z.strictObject({ load: z.boolean(), })", counted],
        ),
    ] {
        for member in members {
            assert!(
                zod.contains(&format!("$OwnSchema.and(z.lazy(() => {member}).superRefine((_read, ctx) => {{ ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => {{ const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? {{ ...issue, path: [] }} : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? {{ ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] }} : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return {{ ...issue, path: [last] }}; return width || (!absent && refused) ? {{ ...issue, path: [] }} : issue; }})); }}, {{ when: () => true }}))")),
                "{holder}: no operand `{member}` in: {zod}"
            );
        }
        for unbound in [
            "=> load)",
            "=> stowed)",
            "__tixschema_parameter_",
            "MemberUntaggedCarried$Schema",
        ] {
            assert!(!zod.contains(unbound), "{holder} names `{unbound}`: {zod}");
        }
    }
}

/// TypeScript joins each member of an untagged enum written with an argument where its members
/// close each other's keys, the parameter written as the argument, directly and through an alias
/// that declares a parameter of its own, and names no parameter the holder does not declare.
#[test]
#[cfg(feature = "typescript")]
fn test_a_flattened_untagged_enum_written_with_an_argument_types_each_member_at_the_argument() {
    let picked = UntaggedPickedHolder::ts_definition();
    assert!(
        picked.contains("} & ({ load: number; count?: never } | { count: number; load?: never });"),
        "expected one key set per member, got: {picked}"
    );
    let aliased = UntaggedPickedAliasHolder::ts_definition();
    assert!(
        aliased
            .contains("} & ({ load: boolean; count?: never } | { count: number; load?: never });"),
        "expected one key set per member of the alias, got: {aliased}"
    );
    for (holder, ts) in [
        ("UntaggedPickedHolder", picked),
        (
            "UntaggedCarriedHolder",
            UntaggedCarriedHolder::ts_definition(),
        ),
        ("UntaggedPickedAliasHolder", aliased),
    ] {
        for unbound in ["Load", "Stowed", "__tixschema_parameter_"] {
            assert!(!ts.contains(unbound), "{holder} names `{unbound}`: {ts}");
        }
    }
}

/// The untagged enum's own text names its own parameter, as it did before any holder filled it.
#[test]
#[cfg(all(feature = "zod", feature = "typescript"))]
fn test_a_generic_untagged_enum_flattened_with_an_argument_keeps_its_own_text() {
    let ts = MemberUntaggedPicked::<String>::ts_definition();
    assert!(
        ts.contains("{ load: Load } | { count: number }") && !ts.contains("__tixschema_parameter_"),
        "Got: {ts}"
    );
    let zod = MemberUntaggedPicked::<String>::zod_schema();
    assert!(
        zod.contains(
            "z.union([z.strictObject({ load: load, }), z.strictObject({ count: \
             z.number().int().min(0).max(4294967295), \
             })])"
        ) && !zod.contains("__tixschema_parameter_"),
        "Got: {zod}"
    );
}

/// The JSON Schema of each holder admits each member at the argument, and refuses the holder's own
/// member alone and a member of another type.
#[test]
fn test_a_flattened_untagged_enum_written_with_an_argument_documents_each_member_at_the_argument() {
    for (schema, admitted, refused) in [
        (
            UntaggedCarriedHolder::json_schema(),
            [
                serde_json::json!({ "owner": "w", "own": "o" }),
                serde_json::json!({ "count": 3_u32, "own": "o" }),
            ],
            serde_json::json!({ "owner": 3_u32, "own": "o" }),
        ),
        (
            UntaggedPickedAliasHolder::json_schema(),
            [
                serde_json::json!({ "load": true, "own": "o" }),
                serde_json::json!({ "count": 3_u32, "own": "o" }),
            ],
            serde_json::json!({ "load": 7_u32, "own": "o" }),
        ),
    ] {
        for payload in admitted {
            assert!(
                super::documents::admits(&schema, &payload),
                "{payload} is rejected by {schema}"
            );
        }
        for payload in [serde_json::json!({ "own": "o" }), refused] {
            assert!(
                !super::documents::admits(&schema, &payload),
                "{payload} is accepted by {schema}"
            );
        }
    }
}

/// The JSON Schema of a flattened `ObjectId` admits the `$oid` key serde writes beside the
/// object's own, and refuses the object without it and a key that is no `ObjectId`.
#[test]
#[cfg(feature = "mongodb")]
fn test_a_flattened_object_id_document_admits_what_serde_writes() {
    let holder = FlattenedOidHolderDraft {
        id: 1,
        oid: ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap(),
    };
    let written = serde_json::to_value(&holder).unwrap();
    assert_eq!(
        written,
        serde_json::json!({ "id": 1_u32, "$oid": "507f1f77bcf86cd799439011" }),
        "serde no longer writes the `$oid` key beside the object's own"
    );
    let schema = FlattenedOidHolder::json_schema();
    assert!(
        super::documents::admits(&schema, &written),
        "{written} is rejected by {schema}"
    );
    for refused in [
        serde_json::json!({ "id": 1_u32 }),
        serde_json::json!({ "id": 1_u32, "$oid": "x" }),
    ] {
        assert!(
            !super::documents::admits(&schema, &refused),
            "{refused} is accepted by {schema}"
        );
    }
}

/// The JSON Schema of an `ObjectId` in the lone slot of a variant under a tag alone admits the
/// `$oid` key serde writes beside the tag, and refuses what serde refuses: the tag alone, a key
/// that is no `ObjectId`, and a key the `ObjectId` does not read.
#[test]
#[cfg(feature = "mongodb")]
fn test_an_object_id_beside_the_tag_document_admits_what_serde_writes() {
    let held = TaggedOidHolderDraft::Held(ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap());
    let written = serde_json::to_value(&held).unwrap();
    assert_eq!(
        written,
        serde_json::json!({ "kind": "Held", "$oid": "507f1f77bcf86cd799439011" }),
        "serde no longer writes the `$oid` key beside the tag"
    );
    let schema = TaggedOidHolder::json_schema();
    assert!(
        super::documents::admits(&schema, &written),
        "{written} is rejected by {schema}"
    );
    for refused in [
        serde_json::json!({ "kind": "Held" }),
        serde_json::json!({ "kind": "Held", "$oid": "x" }),
        serde_json::json!({ "kind": "Held", "$oid": "507f1f77bcf86cd799439011", "id": 1_u32 }),
    ] {
        assert!(
            serde_json::from_value::<TaggedOidHolder>(refused.clone()).is_err(),
            "serde no longer refuses {refused}"
        );
        assert!(
            !super::documents::admits(&schema, &refused),
            "{refused} is accepted by {schema}"
        );
    }
}

/// serde refuses every write of a type parameter flattened, or held by the lone slot of a variant
/// under a tag alone, filled at a value it writes as no object, and of that slot filled at an
/// `Option`: the fillings the build refuses where they are written.
#[test]
fn test_serde_refuses_every_write_of_a_parameter_filled_at_a_value_written_as_no_object() {
    let refused = |written: Result<String, serde_json::Error>| written.unwrap_err().to_string();
    for (filling, said) in [
        (
            refused(serde_json::to_string(&SerdeOnlyBag {
                id: 1,
                inner: "x".to_owned(),
            })),
            "can only flatten structs and maps (got a string)",
        ),
        (
            refused(serde_json::to_string(&SerdeOnlyBag {
                id: 1,
                inner: 7_u32,
            })),
            "can only flatten structs and maps (got an integer)",
        ),
        (
            refused(serde_json::to_string(&SerdeOnlyBag {
                id: 1,
                inner: vec![1_u8],
            })),
            "can only flatten structs and maps (got a sequence)",
        ),
        (
            refused(serde_json::to_string(&SerdeOnlyBag {
                id: 1,
                inner: Some("x".to_owned()),
            })),
            "can only flatten structs and maps (got a string)",
        ),
        (
            refused(serde_json::to_string(&SerdeOnlyBag {
                id: 1,
                inner: SerdeOnlyCode("c".to_owned()),
            })),
            "can only flatten structs and maps (got a string)",
        ),
        (
            refused(serde_json::to_string(&SerdeOnlyWaybill::Held(
                "x".to_owned(),
            ))),
            "cannot serialize tagged newtype variant SerdeOnlyWaybill::Held containing a string",
        ),
        (
            refused(serde_json::to_string(&SerdeOnlyWaybill::Held(Some(
                BasePartDraft {
                    owner: "o".to_owned(),
                },
            )))),
            "cannot serialize tagged newtype variant SerdeOnlyWaybill::Held containing an optional",
        ),
        (
            refused(serde_json::to_string(
                &SerdeOnlyWaybill::<Option<BasePart>>::Held(None),
            )),
            "cannot serialize tagged newtype variant SerdeOnlyWaybill::Held containing an optional",
        ),
    ] {
        assert_eq!(filling, said);
    }
    assert_eq!(
        serde_json::to_string(&SerdeOnlyBag { id: 1, inner: () }).unwrap(),
        r#"{"id":1}"#,
        "serde writes `()` flattened as no key"
    );
}

#[test]
fn test_an_all_object_tagged_direct_flatten_document_is_byte_identical() {
    assert_eq!(
        serde_json::to_string(&ExtObjDirectHolder::json_schema()).unwrap(),
        r#"{"type":"object","oneOf":[{"type":"object","properties":{"own":{"type":"string"},"One":{"type":"object","additionalProperties":false,"properties":{"a":{"type":"string"}},"required":["a"]}},"required":["own","One"],"additionalProperties":false},{"type":"object","properties":{"own":{"type":"string"},"Two":{"type":"object","additionalProperties":false,"properties":{"b":{"type":"boolean"}},"required":["b"]}},"required":["own","Two"],"additionalProperties":false}]}"#
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_the_optional_all_object_tagged_flatten_type_names_the_keys_its_absence_leaves_out() {
    let ts = OptExtObjHolder::ts_definition();
    let declared = &ts[ts.find("export type").unwrap()..];
    assert_eq!(
        declared,
        "export type OptExtObjHolderDraft = {\n  /**\n   * own\n   * \n   */\n  own: string;\n} & (({\n  /**\n   * One\n   * \n   */\n  \"One\": FlatFirst;\n  \"Two\"?: never;\n} | {\n  /**\n   * Two\n   * \n   */\n  \"Two\": FlatSecond;\n  \"One\"?: never;\n}) | { [K in Exclude<keyof ({\n  /**\n   * One\n   * \n   */\n  \"One\": FlatFirst;\n  \"Two\"?: never;\n} | {\n  /**\n   * Two\n   * \n   */\n  \"Two\": FlatSecond;\n  \"One\"?: never;\n}), \"own\">]?: never });\n\nexport type OptExtObjHolder = Readonly<OptExtObjHolderDraft> & $brand<\"OptExtObjHolder\">;"
    );
}

/// Standing on their own, the two tagged enums close each object member against the other object
/// members' keys on TypeScript, as serde and Zod refuse an object of two variants' keys.
#[test]
#[cfg(feature = "typescript")]
fn test_the_tagged_enums_standing_alone_close_each_object_member_on_typescript() {
    let bare = MemberExtBare::ts_definition();
    let objects = MemberExtObjects::ts_definition();
    // Scoped to the type declaration alone: the `$Variant` reader appended after it is not part
    // of what stands here byte-identical.
    assert_eq!(
        [
            bare[bare.find("export type").unwrap()..]
                .split("\n\n")
                .next()
                .unwrap(),
            objects[objects.find("export type").unwrap()..]
                .split("\n\n")
                .next()
                .unwrap(),
        ],
        [
            "export type MemberExtBareDraft =   /**\n   * Bare\n   * \n   */\n  \"Bare\" | {\n  /**\n   * Wrapped\n   * \n   */\n  \"Wrapped\": FlatSecond;\n};",
            "export type MemberExtObjectsDraft = {\n  /**\n   * One\n   * \n   */\n  \"One\": FlatFirst;\n  \"Two\"?: never;\n} | {\n  /**\n   * Two\n   * \n   */\n  \"Two\": FlatSecond;\n  \"One\"?: never;\n};",
        ]
    );
}

/// The untagged footing answers the same way, on the members whose keys the declaration spells.
#[test]
fn test_a_directly_flattened_inline_untagged_union_round_trips_every_member() {
    let forms = [
        (
            InlineUntagEitherDraft::Left {
                left: "l".to_owned(),
            },
            serde_json::json!({ "left": "l", "own": "o" }),
        ),
        (
            InlineUntagEitherDraft::Right { right: true },
            serde_json::json!({ "right": true, "own": "o" }),
        ),
    ];
    for (either, expected) in forms {
        let holder = InlineUntagDirectHolderDraft {
            either: either.validate().unwrap(),
            own: "o".to_owned(),
        };
        let written = serde_json::to_value(&holder).unwrap();
        assert_eq!(written, expected);
        let back: InlineUntagDirectHolder = serde_json::from_value(written).unwrap();
        assert_eq!(back, holder.validate().unwrap());
    }
}

#[test]
#[cfg(feature = "typescript")]
fn test_an_inline_untagged_direct_flatten_type_closes_each_member_against_the_other() {
    for (holder, ts) in [
        (
            "InlineUntagDirectHolder",
            InlineUntagDirectHolder::ts_definition(),
        ),
        (
            "InlineUntagAliasHolder",
            InlineUntagAliasHolder::ts_definition(),
        ),
    ] {
        let declared = &ts[ts.find("export type").unwrap()..];
        assert_eq!(
            declared,
            format!(
                "export type {holder}Draft = {{\n  /**\n   * own\n   * \n   */\n  own: string;\n}} & \
                 ({{ left: string; right?: never }} | {{ right: boolean; left?: never }});\n\n\
                 export type {holder} = Readonly<{holder}Draft> & $brand<\"{holder}\">;"
            ),
            "{holder} does not close each member against the other"
        );
    }
}

#[test]
#[cfg(feature = "typescript")]
fn test_the_inline_untagged_union_is_byte_identical_standing_alone() {
    let ts = InlineUntagEither::ts_definition();
    // Scoped to the type declaration alone: the `$Variant` reader appended after it is not part
    // of what stands here byte-identical.
    let declared = ts[ts.find("export type").unwrap()..]
        .split("\n\n")
        .next()
        .unwrap();
    assert_eq!(
        declared,
        "export type InlineUntagEitherDraft = { left: string } | { right: boolean };"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_the_inline_untagged_direct_flatten_schema_is_unchanged() {
    let zod = InlineUntagDirectHolder::zod_schema();
    assert!(
        zod.contains(
            "z.union([\n  InlineUntagDirectHolder$OwnSchema.and(z.lazy(() => z.strictObject({ left: z.string(), })).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  InlineUntagDirectHolder$OwnSchema.and(z.lazy(() => z.strictObject({ right: z.boolean(), })).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n])"
        ),
        "expected one operand per member, got: {zod}"
    );
}

#[test]
fn test_the_inline_untagged_direct_flatten_document_is_unchanged() {
    assert_eq!(
        serde_json::to_string(&InlineUntagDirectHolder::json_schema()).unwrap(),
        r#"{"type":"object","anyOf":[{"type":"object","properties":{"own":{"type":"string"},"left":{"type":"string"}},"required":["own","left"],"additionalProperties":false},{"type":"object","properties":{"own":{"type":"string"},"right":{"type":"boolean"}},"required":["own","right"],"additionalProperties":false}]}"#
    );
}

#[test]
fn test_a_flattened_variant_field_writes_its_members_into_the_variants_content() {
    assert_eq!(
        serde_json::to_value(ExternalFlatVariantDraft::Named {
            extra: VariantExtraDraft {
                note: "n".to_owned()
            }
            .validate()
            .unwrap(),
            x: "y".to_owned(),
        })
        .unwrap(),
        serde_json::json!({ "Named": { "note": "n", "x": "y" } })
    );
    assert_eq!(
        serde_json::to_value(InternalFlatVariantDraft::Named {
            extra: VariantExtraDraft {
                note: "n".to_owned()
            }
            .validate()
            .unwrap(),
            x: "y".to_owned(),
        })
        .unwrap(),
        serde_json::json!({ "kind": "Named", "note": "n", "x": "y" })
    );
    assert_eq!(
        serde_json::to_value(AdjacentFlatVariantDraft::Named {
            extra: VariantExtraDraft {
                note: "n".to_owned()
            }
            .validate()
            .unwrap(),
            x: "y".to_owned(),
        })
        .unwrap(),
        serde_json::json!({ "kind": "Named", "data": { "note": "n", "x": "y" } })
    );
    assert_eq!(
        serde_json::to_value(TwiceFlatVariantDraft::Named {
            extra: VariantExtraDraft {
                note: "n".to_owned()
            }
            .validate()
            .unwrap(),
            rank: VariantRankDraft { rank: 3 }.validate().unwrap(),
            x: "y".to_owned(),
        })
        .unwrap(),
        serde_json::json!({ "Named": { "note": "n", "rank": 3_i64, "x": "y" } })
    );
}

#[test]
fn test_a_flattened_variant_field_reads_back_from_the_merged_shape() {
    let external: ExternalFlatVariant =
        serde_json::from_value(serde_json::json!({ "Named": { "note": "n", "x": "y" } })).unwrap();
    assert_eq!(
        external,
        ExternalFlatVariantDraft::Named {
            extra: VariantExtraDraft {
                note: "n".to_owned()
            }
            .validate()
            .unwrap(),
            x: "y".to_owned(),
        }
        .validate()
        .unwrap()
    );
    let internal: InternalFlatVariant =
        serde_json::from_value(serde_json::json!({ "kind": "Named", "note": "n", "x": "y" }))
            .unwrap();
    assert_eq!(
        internal,
        InternalFlatVariantDraft::Named {
            extra: VariantExtraDraft {
                note: "n".to_owned()
            }
            .validate()
            .unwrap(),
            x: "y".to_owned(),
        }
        .validate()
        .unwrap()
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_a_flattened_variant_field_is_a_typescript_intersection_inside_the_variant() {
    let external = ExternalFlatVariant::ts_definition();
    assert!(
        external.contains("} & VariantExtraDraft;"),
        "Got: {external}"
    );
    assert!(!external.contains("extra:"), "Got: {external}");
    assert!(external.contains("y: string;"), "Got: {external}");

    let internal = InternalFlatVariant::ts_definition();
    assert!(
        internal.contains("} & VariantExtraDraft"),
        "Got: {internal}"
    );
    assert!(!internal.contains("extra:"), "Got: {internal}");

    let adjacent = AdjacentFlatVariant::ts_definition();
    assert!(
        adjacent.contains("} & VariantExtraDraft;"),
        "Got: {adjacent}"
    );
    assert!(!adjacent.contains("extra:"), "Got: {adjacent}");
}

#[test]
#[cfg(feature = "typescript")]
fn test_two_flattened_variant_fields_join_the_same_content_object() {
    let ts = TwiceFlatVariant::ts_definition();
    assert!(
        ts.contains("} & VariantExtraDraft & VariantRankDraft;"),
        "Got: {ts}"
    );
    assert!(!ts.contains("rank:"), "Got: {ts}");
}

#[test]
#[cfg(feature = "zod")]
fn test_a_flattened_variant_field_is_a_zod_intersection_inside_the_variant() {
    for zod in [
        ExternalFlatVariant::zod_schema(),
        InternalFlatVariant::zod_schema(),
        AdjacentFlatVariant::zod_schema(),
    ] {
        assert!(
            zod.contains("}).and(z.lazy(() => VariantExtra$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))"),
            "Got: {zod}"
        );
        assert!(!zod.contains("extra:"), "Got: {zod}");
        assert!(
            !zod.contains(".and(VariantExtra$Schema)"),
            "the source is read while the const initializes: {zod}"
        );
    }
}

#[test]
#[cfg(feature = "zod")]
fn test_two_flattened_variant_fields_chain_their_zod_operands() {
    let zod = TwiceFlatVariant::zod_schema();
    assert!(
        zod.contains(
            "}).and(z.lazy(() => VariantExtra$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })).and(z.lazy(() => VariantRank$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))"
        ),
        "Got: {zod}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_an_internally_tagged_flattened_variant_forces_a_plain_zod_union() {
    let zod = InternalFlatVariant::zod_schema();
    assert!(zod.contains("z.union(["), "Got: {zod}");
    assert!(!zod.contains("z.discriminatedUnion("), "Got: {zod}");
}

#[test]
fn test_a_flattened_variant_field_merges_into_the_variants_json_content() {
    for document in [
        ExternalFlatVariant::json_schema(),
        InternalFlatVariant::json_schema(),
        AdjacentFlatVariant::json_schema(),
    ] {
        let content = variant_content(&document, "Named");
        let (named, required) = described_keys(&content);
        assert!(named.contains(&"note".to_owned()), "Got: {content}");
        assert!(named.contains(&"x".to_owned()), "Got: {content}");
        assert!(!named.contains(&"extra".to_owned()), "Got: {content}");
        assert!(required.contains(&"note".to_owned()), "Got: {content}");
        assert!(required.contains(&"x".to_owned()), "Got: {content}");
        assert_eq!(content["additionalProperties"], serde_json::json!(false));
    }
}

#[test]
fn test_two_flattened_variant_fields_merge_into_one_json_content() {
    let content = variant_content(&TwiceFlatVariant::json_schema(), "Named");
    let (named, required) = described_keys(&content);
    for key in ["note", "rank", "x"] {
        assert!(named.contains(&key.to_owned()), "Got: {content}");
        assert!(required.contains(&key.to_owned()), "Got: {content}");
    }
}

#[test]
fn test_the_flattened_variant_document_accepts_what_serde_writes() {
    let cases = [
        (
            ExternalFlatVariant::json_schema(),
            serde_json::to_value(ExternalFlatVariantDraft::Named {
                extra: VariantExtraDraft {
                    note: "n".to_owned(),
                }
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            })
            .unwrap(),
        ),
        (
            InternalFlatVariant::json_schema(),
            serde_json::to_value(InternalFlatVariantDraft::Named {
                extra: VariantExtraDraft {
                    note: "n".to_owned(),
                }
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            })
            .unwrap(),
        ),
    ];
    for (document, payload) in cases {
        assert!(
            closed_document_accepts(&document, &payload),
            "{document} rejected {payload}"
        );
    }
}

#[test]
#[cfg(feature = "typescript")]
fn test_a_variant_without_a_flattened_field_keeps_its_typescript_object() {
    let ts = ExternalFlatVariant::ts_definition();
    let declared = &ts[ts.find("export type").unwrap()..];
    let draft = declared
        .split("\n\nexport type ExternalFlatVariant =")
        .next()
        .unwrap();
    let plain = &draft[draft.find("\"Plain\"").unwrap()..];
    assert!(!plain.contains(" & "), "Got: {plain}");
}

#[test]
#[cfg(feature = "zod")]
fn test_a_variant_without_a_flattened_field_keeps_its_zod_object() {
    let zod = AdjacentFlatVariant::zod_schema();
    let plain = &zod[zod.find("z.literal(\"Plain\")").unwrap()..];
    assert!(!plain.contains(".and("), "Got: {plain}");
}

#[test]
fn test_a_flattened_untagged_variant_field_writes_its_members_beside_the_variants_own() {
    assert_eq!(
        serde_json::to_value(UntaggedFlatVariantDraft::Named {
            extra: VariantExtraDraft {
                note: "n".to_owned()
            }
            .validate()
            .unwrap(),
            x: "y".to_owned(),
        })
        .unwrap(),
        serde_json::json!({ "note": "n", "x": "y" })
    );
    assert_eq!(
        serde_json::to_value(TwiceUntaggedFlatVariantDraft::Named {
            extra: VariantExtraDraft {
                note: "n".to_owned()
            }
            .validate()
            .unwrap(),
            rank: VariantRankDraft { rank: 3 }.validate().unwrap(),
            x: "y".to_owned(),
        })
        .unwrap(),
        serde_json::json!({ "note": "n", "rank": 3_i64, "x": "y" })
    );
}

#[test]
fn test_a_flattened_untagged_variant_field_round_trips_both_members() {
    for value in [
        UntaggedFlatVariantDraft::Named {
            extra: VariantExtraDraft {
                note: "n".to_owned(),
            }
            .validate()
            .unwrap(),
            x: "y".to_owned(),
        },
        UntaggedFlatVariantDraft::Plain { y: "p".to_owned() },
    ] {
        let written = serde_json::to_value(&value).unwrap();
        let back: UntaggedFlatVariant = serde_json::from_value(written).unwrap();
        assert_eq!(back, value.validate().unwrap());
    }
}

#[test]
#[cfg(feature = "typescript")]
fn test_a_flattened_untagged_variant_field_is_a_typescript_intersection_inside_the_member() {
    let ts = UntaggedFlatVariant::ts_definition();
    // Scoped to the type declaration alone: the `$Variant` reader appended after it is not part
    // of what stands here byte-identical.
    let declared = ts[ts.find("export type").unwrap()..]
        .split("\n\n")
        .next()
        .unwrap();
    assert_eq!(
        declared,
        "export type UntaggedFlatVariantDraft = { x: string } & VariantExtraDraft | { y: string };"
    );
}

#[test]
#[cfg(feature = "typescript")]
fn test_two_flattened_untagged_variant_fields_join_the_same_member_object() {
    let ts = TwiceUntaggedFlatVariant::ts_definition();
    assert!(
        ts.contains("{ x: string } & VariantExtraDraft & VariantRankDraft;"),
        "Got: {ts}"
    );
    assert!(!ts.contains("rank:"), "Got: {ts}");
}

#[test]
#[cfg(feature = "zod")]
fn test_a_flattened_untagged_variant_field_is_a_zod_intersection_inside_the_member() {
    let zod = UntaggedFlatVariant::zod_schema();
    assert!(
        zod.contains("}).and(z.lazy(() => VariantExtra$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))"),
        "Got: {zod}"
    );
    assert!(!zod.contains("extra:"), "Got: {zod}");
    assert!(
        !zod.contains(".and(VariantExtra$Schema)"),
        "the source is read while the const initializes: {zod}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_two_flattened_untagged_variant_fields_chain_their_zod_operands() {
    let zod = TwiceUntaggedFlatVariant::zod_schema();
    assert!(
        zod.contains(
            "}).and(z.lazy(() => VariantExtra$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })).and(z.lazy(() => VariantRank$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))"
        ),
        "Got: {zod}"
    );
}

/// The keys `payload` carries, sorted, beside the keys `branch` names and the keys it requires.
fn written_against_described(
    branch: &serde_json::Value,
    payload: &serde_json::Value,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut written: Vec<String> = payload.as_object().unwrap().keys().cloned().collect();
    written.sort();
    let (mut named, mut required) = described_keys(branch);
    named.sort();
    required.sort();
    (written, named, required)
}

#[test]
fn test_a_flattened_untagged_variant_field_merges_into_the_members_json_object() {
    let payload = serde_json::to_value(UntaggedFlatVariantDraft::Named {
        extra: VariantExtraDraft {
            note: "n".to_owned(),
        }
        .validate()
        .unwrap(),
        x: "y".to_owned(),
    })
    .unwrap();
    let branch = untagged_branch(&UntaggedFlatVariant::json_schema(), "x");
    let (written, named, required) = written_against_described(&branch, &payload);
    assert_eq!(named, written, "Got: {branch}");
    assert_eq!(required, written, "Got: {branch}");
    assert_eq!(branch["additionalProperties"], serde_json::json!(false));
}

#[test]
fn test_two_flattened_untagged_variant_fields_merge_into_one_json_object() {
    let payload = serde_json::to_value(TwiceUntaggedFlatVariantDraft::Named {
        extra: VariantExtraDraft {
            note: "n".to_owned(),
        }
        .validate()
        .unwrap(),
        rank: VariantRankDraft { rank: 3 }.validate().unwrap(),
        x: "y".to_owned(),
    })
    .unwrap();
    let branch = untagged_branch(&TwiceUntaggedFlatVariant::json_schema(), "x");
    let (written, named, required) = written_against_described(&branch, &payload);
    assert_eq!(named, written, "Got: {branch}");
    assert_eq!(required, written, "Got: {branch}");
}

#[test]
fn test_the_flattened_untagged_variant_document_accepts_what_serde_writes() {
    let document = UntaggedFlatVariant::json_schema();
    for value in [
        UntaggedFlatVariantDraft::Named {
            extra: VariantExtraDraft {
                note: "n".to_owned(),
            }
            .validate()
            .unwrap(),
            x: "y".to_owned(),
        },
        UntaggedFlatVariantDraft::Plain { y: "p".to_owned() },
    ] {
        let payload = serde_json::to_value(&value).unwrap();
        assert_eq!(
            accepting_branches(&document["anyOf"], &payload),
            1,
            "{document} on {payload}"
        );
    }
}

#[test]
fn test_the_flattened_untagged_variant_document_rejects_the_stale_nested_shape() {
    let document = UntaggedFlatVariant::json_schema();
    assert!(
        !closed_document_accepts(
            &document,
            &serde_json::json!({ "extra": { "note": "n" }, "x": "y" })
        ),
        "Got: {document}"
    );
}

#[test]
fn test_flattening_an_enum_whose_member_flattens_round_trips_every_member() {
    let forms = [
        (
            UntaggedFlatVariantDraft::Named {
                extra: VariantExtraDraft {
                    note: "n".to_owned(),
                }
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            },
            serde_json::json!({ "note": "n", "x": "y", "own": "o" }),
        ),
        (
            UntaggedFlatVariantDraft::Plain { y: "p".to_owned() },
            serde_json::json!({ "y": "p", "own": "o" }),
        ),
    ];
    for (either, expected) in forms {
        let holder = UntaggedFlatVariantHolderDraft {
            either: either.validate().unwrap(),
            own: "o".to_owned(),
        };
        let written = serde_json::to_value(&holder).unwrap();
        assert_eq!(written, expected);
        let back: UntaggedFlatVariantHolder = serde_json::from_value(written).unwrap();
        assert_eq!(back, holder.validate().unwrap());
    }
}

#[test]
#[cfg(feature = "typescript")]
fn test_a_flattening_untagged_member_closes_no_sibling_against_unprovable_keys() {
    let ts = UntaggedFlatVariantHolder::ts_definition();
    assert!(!ts.contains("?: never"), "Got: {ts}");
    assert!(ts.contains("} & UntaggedFlatVariantDraft;"), "Got: {ts}");
    assert!(
        UntaggedFlatVariant::ts_definition().contains("| { y: string };"),
        "Got: {}",
        UntaggedFlatVariant::ts_definition()
    );
}

#[test]
fn test_a_multi_branch_variant_source_writes_one_key_set_per_branch() {
    assert_eq!(
        serde_json::to_value(ExternalChoiceVariantDraft::Named {
            choice: VariantChoiceDraft::Left(
                ChoiceLeftDraft {
                    left: "l".to_owned()
                }
                .validate()
                .unwrap()
            )
            .validate()
            .unwrap(),
            x: "y".to_owned(),
        })
        .unwrap(),
        serde_json::json!({ "Named": { "left": "l", "x": "y" } })
    );
    assert_eq!(
        serde_json::to_value(ExternalChoiceVariantDraft::Named {
            choice: VariantChoiceDraft::Right(ChoiceRightDraft { right: 3 }.validate().unwrap())
                .validate()
                .unwrap(),
            x: "y".to_owned(),
        })
        .unwrap(),
        serde_json::json!({ "Named": { "right": 3_i64, "x": "y" } })
    );
    assert_eq!(
        serde_json::to_value(InternalChoiceVariantDraft::Named {
            choice: VariantChoiceDraft::Left(
                ChoiceLeftDraft {
                    left: "l".to_owned()
                }
                .validate()
                .unwrap()
            )
            .validate()
            .unwrap(),
            x: "y".to_owned(),
        })
        .unwrap(),
        serde_json::json!({ "kind": "Named", "left": "l", "x": "y" })
    );
    assert_eq!(
        serde_json::to_value(AdjacentChoiceVariantDraft::Named {
            choice: VariantChoiceDraft::Right(ChoiceRightDraft { right: 3 }.validate().unwrap())
                .validate()
                .unwrap(),
            x: "y".to_owned(),
        })
        .unwrap(),
        serde_json::json!({ "kind": "Named", "data": { "right": 3_i64, "x": "y" } })
    );
    assert_eq!(
        serde_json::to_value(UntaggedChoiceVariantDraft::Named {
            choice: VariantChoiceDraft::Left(
                ChoiceLeftDraft {
                    left: "l".to_owned()
                }
                .validate()
                .unwrap()
            )
            .validate()
            .unwrap(),
            x: "y".to_owned(),
        })
        .unwrap(),
        serde_json::json!({ "left": "l", "x": "y" })
    );
}

#[test]
fn test_a_multi_branch_variant_source_reads_back_every_key_set() {
    for (payload, expected) in [
        (
            serde_json::json!({ "Named": { "left": "l", "x": "y" } }),
            ExternalChoiceVariantDraft::Named {
                choice: VariantChoiceDraft::Left(
                    ChoiceLeftDraft {
                        left: "l".to_owned(),
                    }
                    .validate()
                    .unwrap(),
                )
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            },
        ),
        (
            serde_json::json!({ "Named": { "right": 3_i64, "x": "y" } }),
            ExternalChoiceVariantDraft::Named {
                choice: VariantChoiceDraft::Right(
                    ChoiceRightDraft { right: 3 }.validate().unwrap(),
                )
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            },
        ),
        (
            serde_json::json!({ "Plain": { "y": "p" } }),
            ExternalChoiceVariantDraft::Plain { y: "p".to_owned() },
        ),
    ] {
        let read: ExternalChoiceVariant = serde_json::from_value(payload.clone()).unwrap();
        assert_eq!(read, expected.validate().unwrap());
        assert_eq!(serde_json::to_value(&read).unwrap(), payload);
    }

    for value in [
        UntaggedChoiceVariantDraft::Named {
            choice: VariantChoiceDraft::Left(
                ChoiceLeftDraft {
                    left: "l".to_owned(),
                }
                .validate()
                .unwrap(),
            )
            .validate()
            .unwrap(),
            x: "y".to_owned(),
        },
        UntaggedChoiceVariantDraft::Named {
            choice: VariantChoiceDraft::Right(ChoiceRightDraft { right: 3 }.validate().unwrap())
                .validate()
                .unwrap(),
            x: "y".to_owned(),
        },
        UntaggedChoiceVariantDraft::Plain { y: "p".to_owned() },
    ] {
        let written = serde_json::to_value(&value).unwrap();
        assert_eq!(
            serde_json::from_value::<UntaggedChoiceVariant>(written).unwrap(),
            value.validate().unwrap()
        );
    }
}

#[test]
#[cfg(feature = "zod")]
fn test_a_multi_branch_variant_source_writes_one_zod_object_per_branch() {
    let external = ExternalChoiceVariant::zod_schema();
    assert!(
        external.contains(
            "\"Named\": z.union([\n  \
             z.object({\n  x: z.string(),\n}).and(z.lazy(() => ChoiceLeft$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  \
             z.object({\n  x: z.string(),\n}).and(z.lazy(() => ChoiceRight$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n])"
        ),
        "Got: {external}"
    );

    let internal = InternalChoiceVariant::zod_schema();
    assert!(
        internal.contains(
            "z.union([\n  \
             z.object({\n  kind: z.literal(\"Named\"),\n  x: z.string(),\n}).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })\
             .and(z.lazy(() => ChoiceLeft$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  \
             z.object({\n  kind: z.literal(\"Named\"),\n  x: z.string(),\n}).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })\
             .and(z.lazy(() => ChoiceRight$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n])"
        ),
        "Got: {internal}"
    );

    let adjacent = AdjacentChoiceVariant::zod_schema();
    assert!(
        adjacent.contains(
            "data: z.union([\n  \
             z.object({\n  x: z.string(),\n}).and(z.lazy(() => ChoiceLeft$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  \
             z.object({\n  x: z.string(),\n}).and(z.lazy(() => ChoiceRight$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n])"
        ),
        "Got: {adjacent}"
    );

    let untagged = UntaggedChoiceVariant::zod_schema();
    assert!(
        untagged.contains(
            "z.union([\n  \
             z.object({ x: z.string(), }).and(z.lazy(() => ChoiceLeft$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  \
             z.object({ x: z.string(), }).and(z.lazy(() => ChoiceRight$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n])"
        ),
        "Got: {untagged}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_the_multiplied_variant_zod_names_the_branch_every_serde_payload_wrote() {
    let cases = [
        (
            ExternalChoiceVariant::zod_schema(),
            serde_json::to_value(ExternalChoiceVariantDraft::Named {
                choice: VariantChoiceDraft::Left(
                    ChoiceLeftDraft {
                        left: "l".to_owned(),
                    }
                    .validate()
                    .unwrap(),
                )
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            })
            .unwrap(),
            "ChoiceLeft$Schema",
        ),
        (
            ExternalChoiceVariant::zod_schema(),
            serde_json::to_value(ExternalChoiceVariantDraft::Named {
                choice: VariantChoiceDraft::Right(
                    ChoiceRightDraft { right: 3 }.validate().unwrap(),
                )
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            })
            .unwrap(),
            "ChoiceRight$Schema",
        ),
        (
            InternalChoiceVariant::zod_schema(),
            serde_json::to_value(InternalChoiceVariantDraft::Named {
                choice: VariantChoiceDraft::Right(
                    ChoiceRightDraft { right: 3 }.validate().unwrap(),
                )
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            })
            .unwrap(),
            "ChoiceRight$Schema",
        ),
        (
            AdjacentChoiceVariant::zod_schema(),
            serde_json::to_value(AdjacentChoiceVariantDraft::Named {
                choice: VariantChoiceDraft::Left(
                    ChoiceLeftDraft {
                        left: "l".to_owned(),
                    }
                    .validate()
                    .unwrap(),
                )
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            })
            .unwrap(),
            "ChoiceLeft$Schema",
        ),
        (
            UntaggedChoiceVariant::zod_schema(),
            serde_json::to_value(UntaggedChoiceVariantDraft::Named {
                choice: VariantChoiceDraft::Right(
                    ChoiceRightDraft { right: 3 }.validate().unwrap(),
                )
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            })
            .unwrap(),
            "ChoiceRight$Schema",
        ),
    ];
    for (zod, payload, branch) in cases {
        assert!(
            zod.contains(&format!(".and(z.lazy(() => {branch}).superRefine((_read, ctx) => {{ ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => {{ const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? {{ ...issue, path: [] }} : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? {{ ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] }} : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return {{ ...issue, path: [last] }}; return width || (!absent && refused) ? {{ ...issue, path: [] }} : issue; }})); }}, {{ when: () => true }}))")),
            "nothing joins {branch} for {payload}: {zod}"
        );
        assert!(!zod.contains("choice:"), "Got: {zod}");
    }
}

#[test]
fn test_a_three_branch_variant_source_writes_three_key_sets() {
    for (value, written) in [
        (
            ThreeBranchChoiceVariantDraft::Named {
                choice: ThreeBranchChoiceDraft::Left(
                    ChoiceLeftDraft {
                        left: "l".to_owned(),
                    }
                    .validate()
                    .unwrap(),
                )
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            },
            serde_json::json!({ "Named": { "left": "l", "x": "y" } }),
        ),
        (
            ThreeBranchChoiceVariantDraft::Named {
                choice: ThreeBranchChoiceDraft::Middle(
                    ChoiceMiddleDraft { middle: true }.validate().unwrap(),
                )
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            },
            serde_json::json!({ "Named": { "middle": true, "x": "y" } }),
        ),
        (
            ThreeBranchChoiceVariantDraft::Named {
                choice: ThreeBranchChoiceDraft::Right(
                    ChoiceRightDraft { right: 3 }.validate().unwrap(),
                )
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            },
            serde_json::json!({ "Named": { "right": 3_i64, "x": "y" } }),
        ),
    ] {
        assert_eq!(serde_json::to_value(&value).unwrap(), written);
        assert_eq!(
            serde_json::from_value::<ThreeBranchChoiceVariant>(written).unwrap(),
            value.validate().unwrap()
        );
    }
}

#[test]
#[cfg(feature = "zod")]
fn test_a_three_branch_variant_source_writes_three_zod_objects() {
    let zod = ThreeBranchChoiceVariant::zod_schema();
    assert!(
        zod.contains(
            "\"Named\": z.union([\n  \
             z.strictObject({\n  x: z.string(),\n}).and(z.lazy(() => ChoiceLeft$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  \
             z.strictObject({\n  x: z.string(),\n}).and(z.lazy(() => ChoiceMiddle$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  \
             z.strictObject({\n  x: z.string(),\n}).and(z.lazy(() => ChoiceRight$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n])"
        ),
        "Got: {zod}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_an_internally_tagged_multiplying_variant_forces_a_plain_zod_union() {
    let zod = InternalChoiceVariant::zod_schema();
    assert!(zod.contains("z.union(["), "Got: {zod}");
    assert!(!zod.contains("z.discriminatedUnion("), "Got: {zod}");
}

#[test]
#[cfg(feature = "zod")]
fn test_an_adjacently_tagged_multiplying_variant_keeps_its_discriminated_union() {
    let zod = AdjacentChoiceVariant::zod_schema();
    assert!(
        zod.contains("z.discriminatedUnion(\"kind\", ["),
        "Got: {zod}"
    );
    assert!(zod.contains("kind: z.literal(\"Named\"),"), "Got: {zod}");
}

#[test]
fn test_an_optional_variant_source_writes_the_object_with_the_source_and_without_it() {
    assert_eq!(
        serde_json::to_value(OptionalSourceVariantDraft::Named {
            extra: Some(
                VariantExtraDraft {
                    note: "n".to_owned()
                }
                .validate()
                .unwrap()
            ),
            x: "y".to_owned(),
        })
        .unwrap(),
        serde_json::json!({ "Named": { "note": "n", "x": "y" } })
    );
    assert_eq!(
        serde_json::to_value(OptionalSourceVariantDraft::Named {
            extra: None,
            x: "y".to_owned(),
        })
        .unwrap(),
        serde_json::json!({ "Named": { "x": "y" } })
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_an_optional_variant_source_offers_the_source_and_its_absence_in_zod() {
    let zod = OptionalSourceVariant::zod_schema();
    assert!(
        zod.contains(
            "\"Named\": z.union([\n  \
             z.strictObject({\n  x: z.string(),\n}).and(z.lazy(() => VariantExtra$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  \
             z.strictObject({\n  x: z.string(),\n}),\n])"
        ),
        "Got: {zod}"
    );
}

#[test]
fn test_two_variant_sources_write_their_cross_product() {
    for (value, written) in [
        (
            TwoChoiceVariantDraft::Named {
                choice: VariantChoiceDraft::Left(
                    ChoiceLeftDraft {
                        left: "l".to_owned(),
                    }
                    .validate()
                    .unwrap(),
                )
                .validate()
                .unwrap(),
                pair: PairChoiceDraft::Up(ChoiceUpDraft { up: "u".to_owned() }.validate().unwrap())
                    .validate()
                    .unwrap(),
                x: "y".to_owned(),
            },
            serde_json::json!({ "Named": { "left": "l", "up": "u", "x": "y" } }),
        ),
        (
            TwoChoiceVariantDraft::Named {
                choice: VariantChoiceDraft::Right(
                    ChoiceRightDraft { right: 3 }.validate().unwrap(),
                )
                .validate()
                .unwrap(),
                pair: PairChoiceDraft::Down(
                    ChoiceDownDraft {
                        down: "d".to_owned(),
                    }
                    .validate()
                    .unwrap(),
                )
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            },
            serde_json::json!({ "Named": { "right": 3_i64, "down": "d", "x": "y" } }),
        ),
    ] {
        assert_eq!(serde_json::to_value(&value).unwrap(), written);
        assert_eq!(
            serde_json::from_value::<TwoChoiceVariant>(written).unwrap(),
            value.validate().unwrap()
        );
    }
}

#[test]
#[cfg(feature = "zod")]
fn test_two_variant_sources_write_their_cross_product_in_zod() {
    let zod = TwoChoiceVariant::zod_schema();
    let own = "z.object({\n  x: z.string(),\n})";
    assert!(
        zod.contains(&format!(
            "\"Named\": z.union([\n  \
             {own}.and(z.lazy(() => ChoiceLeft$Schema).superRefine((_read, ctx) => {{ ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => {{ const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? {{ ...issue, path: [] }} : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? {{ ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] }} : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return {{ ...issue, path: [last] }}; return width || (!absent && refused) ? {{ ...issue, path: [] }} : issue; }})); }}, {{ when: () => true }})).and(z.lazy(() => ChoiceDown$Schema).superRefine((_read, ctx) => {{ ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => {{ const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? {{ ...issue, path: [] }} : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? {{ ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] }} : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return {{ ...issue, path: [last] }}; return width || (!absent && refused) ? {{ ...issue, path: [] }} : issue; }})); }}, {{ when: () => true }})),\n  \
             {own}.and(z.lazy(() => ChoiceLeft$Schema).superRefine((_read, ctx) => {{ ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => {{ const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? {{ ...issue, path: [] }} : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? {{ ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] }} : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return {{ ...issue, path: [last] }}; return width || (!absent && refused) ? {{ ...issue, path: [] }} : issue; }})); }}, {{ when: () => true }})).and(z.lazy(() => ChoiceUp$Schema).superRefine((_read, ctx) => {{ ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => {{ const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? {{ ...issue, path: [] }} : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? {{ ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] }} : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return {{ ...issue, path: [last] }}; return width || (!absent && refused) ? {{ ...issue, path: [] }} : issue; }})); }}, {{ when: () => true }})),\n  \
             {own}.and(z.lazy(() => ChoiceRight$Schema).superRefine((_read, ctx) => {{ ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => {{ const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? {{ ...issue, path: [] }} : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? {{ ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] }} : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return {{ ...issue, path: [last] }}; return width || (!absent && refused) ? {{ ...issue, path: [] }} : issue; }})); }}, {{ when: () => true }})).and(z.lazy(() => ChoiceDown$Schema).superRefine((_read, ctx) => {{ ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => {{ const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? {{ ...issue, path: [] }} : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? {{ ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] }} : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return {{ ...issue, path: [last] }}; return width || (!absent && refused) ? {{ ...issue, path: [] }} : issue; }})); }}, {{ when: () => true }})),\n  \
             {own}.and(z.lazy(() => ChoiceRight$Schema).superRefine((_read, ctx) => {{ ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => {{ const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? {{ ...issue, path: [] }} : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? {{ ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] }} : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return {{ ...issue, path: [last] }}; return width || (!absent && refused) ? {{ ...issue, path: [] }} : issue; }})); }}, {{ when: () => true }})).and(z.lazy(() => ChoiceUp$Schema).superRefine((_read, ctx) => {{ ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => {{ const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? {{ ...issue, path: [] }} : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? {{ ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] }} : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return {{ ...issue, path: [last] }}; return width || (!absent && refused) ? {{ ...issue, path: [] }} : issue; }})); }}, {{ when: () => true }})),\n])"
        )),
        "Got: {zod}"
    );
}

#[test]
fn test_an_optional_multi_branch_variant_source_writes_a_members_keys_or_none() {
    for (value, written) in [
        (
            OptionalChoiceVariantDraft::Named {
                choice: Some(
                    VariantChoiceDraft::Left(
                        ChoiceLeftDraft {
                            left: "l".to_owned(),
                        }
                        .validate()
                        .unwrap(),
                    )
                    .validate()
                    .unwrap(),
                ),
                x: "y".to_owned(),
            },
            serde_json::json!({ "Named": { "left": "l", "x": "y" } }),
        ),
        (
            OptionalChoiceVariantDraft::Named {
                choice: None,
                x: "y".to_owned(),
            },
            serde_json::json!({ "Named": { "x": "y" } }),
        ),
    ] {
        assert_eq!(serde_json::to_value(&value).unwrap(), written);
        assert_eq!(
            serde_json::from_value::<OptionalChoiceVariant>(written).unwrap(),
            value.validate().unwrap()
        );
    }
}

#[test]
#[cfg(feature = "zod")]
fn test_an_optional_multi_branch_variant_source_offers_its_branches_and_the_absence() {
    let zod = OptionalChoiceVariant::zod_schema();
    assert!(
        zod.contains(
            "\"Named\": z.union([\n  \
             z.object({\n  x: z.string(),\n}).and(z.lazy(() => ChoiceLeft$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  \
             z.object({\n  x: z.string(),\n}).and(z.lazy(() => ChoiceRight$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n  \
             z.object({\n  x: z.string(),\n}),\n])"
        ),
        "Got: {zod}"
    );
}

#[test]
fn test_a_nonstrict_enum_reads_a_stray_key_past_or_refuses_it_in_each_variant_by_itself() {
    for (value, written) in [
        (
            RefusingBesideOpenVariantDraft::Open {
                choice: VariantChoiceDraft::Left(
                    ChoiceLeftDraft {
                        left: "l".to_owned(),
                    }
                    .validate()
                    .unwrap(),
                )
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            },
            serde_json::json!({ "Open": { "left": "l", "x": "y" } }),
        ),
        (
            RefusingBesideOpenVariantDraft::Refusing {
                choice: ThreeBranchChoiceDraft::Middle(
                    ChoiceMiddleDraft { middle: true }.validate().unwrap(),
                )
                .validate()
                .unwrap(),
                x: "y".to_owned(),
            },
            serde_json::json!({ "Refusing": { "middle": true, "x": "y" } }),
        ),
    ] {
        assert_eq!(serde_json::to_value(&value).unwrap(), written);
        assert_eq!(
            serde_json::from_value::<RefusingBesideOpenVariant>(written).unwrap(),
            value.validate().unwrap()
        );
    }
    assert_eq!(
        serde_json::from_value::<RefusingBesideOpenVariant>(
            serde_json::json!({ "Open": { "left": "l", "stray": 1_u8, "x": "y" } }),
        )
        .unwrap(),
        RefusingBesideOpenVariantDraft::Open {
            choice: VariantChoiceDraft::Left(
                ChoiceLeftDraft {
                    left: "l".to_owned(),
                }
                .validate()
                .unwrap()
            )
            .validate()
            .unwrap(),
            x: "y".to_owned(),
        }
        .validate()
        .unwrap()
    );
    serde_json::from_value::<RefusingBesideOpenVariant>(
        serde_json::json!({ "Refusing": { "middle": true, "stray": 1_u8, "x": "y" } }),
    )
    .unwrap_err();
}

#[test]
#[cfg(feature = "zod")]
fn test_a_multiplying_variant_defers_the_member_that_reaches_the_enum() {
    let zod = RecursiveChoiceVariant::zod_schema();
    let getter = "get next() { return z.union([z.null().transform(() => undefined), \
                  RecursiveChoiceVariant$Schema, z.undefined()]).superRefine((_read, ctx) => { \
                  const held = ctx.value; const at = ctx.issues.findIndex((issue) => issue.code === \
                  \"invalid_union\" && (issue.path ?? []).length === 0); const refused = \
                  ctx.issues[at]; if (held !== null && held !== undefined && refused?.code === \
                  \"invalid_union\") ctx.issues.splice(at, 1, ...(refused.errors[1] ?? \
                  []).map((issue) => ({ ...issue, input: undefined }))); }, { when: () => true \
                  }).prefault(undefined); },";
    assert_eq!(zod.matches(getter).count(), 2, "Got: {zod}");
    assert!(
        zod.contains(&format!(
            "z.union([\n  z.object({{\n  {getter}\n}})\
             .and(z.lazy(() => ChoiceLeft$Schema).superRefine((_read, ctx) => {{ ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => {{ const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? {{ ...issue, path: [] }} : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? {{ ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] }} : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return {{ ...issue, path: [last] }}; return width || (!absent && refused) ? {{ ...issue, path: [] }} : issue; }})); }}, {{ when: () => true }})),\n  \
             z.object({{\n  {getter}\n}}).and(z.lazy(() => ChoiceRight$Schema).superRefine((_read, ctx) => {{ ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => {{ const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? {{ ...issue, path: [] }} : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? {{ ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] }} : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return {{ ...issue, path: [last] }}; return width || (!absent && refused) ? {{ ...issue, path: [] }} : issue; }})); }}, {{ when: () => true }})),\n])"
        )),
        "Got: {zod}"
    );
    assert!(
        !zod.contains("next: z."),
        "the enum is read while the const initializes: {zod}"
    );
}

#[test]
fn test_a_multiplying_recursive_variant_round_trips_every_branch() {
    for value in [
        RecursiveChoiceVariantDraft::Named {
            choice: VariantChoiceDraft::Left(
                ChoiceLeftDraft {
                    left: "l".to_owned(),
                }
                .validate()
                .unwrap(),
            )
            .validate()
            .unwrap(),
            next: None,
        },
        RecursiveChoiceVariantDraft::Named {
            choice: VariantChoiceDraft::Right(ChoiceRightDraft { right: 3 }.validate().unwrap())
                .validate()
                .unwrap(),
            next: Some(Box::new(
                RecursiveChoiceVariantDraft::Plain { y: "p".to_owned() }
                    .validate()
                    .unwrap(),
            )),
        },
    ] {
        let written = serde_json::to_value(&value).unwrap();
        assert_eq!(
            serde_json::from_value::<RecursiveChoiceVariant>(written).unwrap(),
            value.validate().unwrap()
        );
    }
}

#[test]
fn test_the_multiplied_variant_document_accepts_what_serde_writes() {
    for (content, payload) in [
        (
            variant_content(&ExternalChoiceVariant::json_schema(), "Named"),
            serde_json::json!({ "left": "l", "x": "y" }),
        ),
        (
            variant_content(&ExternalChoiceVariant::json_schema(), "Named"),
            serde_json::json!({ "right": 3_i64, "x": "y" }),
        ),
        (
            variant_content(&AdjacentChoiceVariant::json_schema(), "Named"),
            serde_json::json!({ "left": "l", "x": "y" }),
        ),
        (
            variant_content(&AdjacentChoiceVariant::json_schema(), "Named"),
            serde_json::json!({ "right": 3_i64, "x": "y" }),
        ),
    ] {
        assert!(
            closed_document_accepts(&content, &payload),
            "{content} rejected {payload}"
        );
    }

    for (document, payload) in [
        (
            InternalChoiceVariant::json_schema(),
            serde_json::json!({ "kind": "Named", "left": "l", "x": "y" }),
        ),
        (
            InternalChoiceVariant::json_schema(),
            serde_json::json!({ "kind": "Named", "right": 3_i64, "x": "y" }),
        ),
        (
            InternalChoiceVariant::json_schema(),
            serde_json::json!({ "kind": "Plain", "y": "p" }),
        ),
        (
            UntaggedChoiceVariant::json_schema(),
            serde_json::json!({ "left": "l", "x": "y" }),
        ),
        (
            UntaggedChoiceVariant::json_schema(),
            serde_json::json!({ "right": 3_i64, "x": "y" }),
        ),
        (
            UntaggedChoiceVariant::json_schema(),
            serde_json::json!({ "y": "p" }),
        ),
    ] {
        assert!(
            closed_document_accepts(&document, &payload),
            "{document} rejected {payload}"
        );
    }
}

#[test]
fn test_the_multiplied_variant_document_rejects_what_no_branch_writes() {
    let content = variant_content(&ExternalChoiceVariant::json_schema(), "Named");
    for payload in [
        serde_json::json!({ "left": "l", "right": 3_i64, "x": "y" }),
        serde_json::json!({ "x": "y" }),
        serde_json::json!({ "choice": { "left": "l" }, "x": "y" }),
        serde_json::json!({ "left": "l" }),
    ] {
        assert!(
            !closed_document_accepts(&content, &payload),
            "{content} admitted {payload}"
        );
    }

    for payload in [
        serde_json::json!({ "kind": "Named", "left": "l", "right": 3_i64, "x": "y" }),
        serde_json::json!({ "kind": "Named", "x": "y" }),
    ] {
        let document = InternalChoiceVariant::json_schema();
        assert!(
            !closed_document_accepts(&document, &payload),
            "{document} admitted {payload}"
        );
    }
}

#[test]
fn test_the_multiplied_variant_document_writes_one_branch_per_key_set() {
    let content = variant_content(&ExternalChoiceVariant::json_schema(), "Named");
    let branches = content["anyOf"].as_array().unwrap();
    assert_eq!(branches.len(), 2, "Got: {content}");
    for (branch, key) in branches.iter().zip(["left", "right"]) {
        let (named, required) = described_keys(branch);
        assert_eq!(named, vec!["x".to_owned(), key.to_owned()], "Got: {branch}");
        assert_eq!(
            required,
            vec!["x".to_owned(), key.to_owned()],
            "Got: {branch}"
        );
        assert_eq!(branch["additionalProperties"], serde_json::json!(true));
    }
}

#[test]
#[cfg(feature = "typescript")]
fn test_the_multiplied_variant_type_is_the_intersection_typescript_distributes() {
    assert!(
        ExternalChoiceVariant::ts_definition()
            .contains("} & (ChoiceLeftDraft | ChoiceRightDraft);"),
        "Got: {}",
        ExternalChoiceVariant::ts_definition()
    );
    assert!(
        InternalChoiceVariant::ts_definition()
            .contains("} & (ChoiceLeftDraft | ChoiceRightDraft) |"),
        "Got: {}",
        InternalChoiceVariant::ts_definition()
    );
    assert!(
        AdjacentChoiceVariant::ts_definition()
            .contains("} & (ChoiceLeftDraft | ChoiceRightDraft);"),
        "Got: {}",
        AdjacentChoiceVariant::ts_definition()
    );
    assert!(
        UntaggedChoiceVariant::ts_definition()
            .contains("= { x: string } & (ChoiceLeftDraft | ChoiceRightDraft) | { y: string };"),
        "Got: {}",
        UntaggedChoiceVariant::ts_definition()
    );
    assert!(
        TwoChoiceVariant::ts_definition().contains(
            "} & (ChoiceLeftDraft | ChoiceRightDraft) & (ChoiceDownDraft | ChoiceUpDraft);"
        ),
        "Got: {}",
        TwoChoiceVariant::ts_definition()
    );
    for ts in [
        OptionalSourceVariant::ts_definition(),
        OptionalChoiceVariant::ts_definition(),
    ] {
        assert!(ts.contains("]?: never });"), "Got: {ts}");
    }
}

#[test]
#[cfg(feature = "zod")]
fn test_a_single_key_set_variant_flatten_schema_is_byte_identical() {
    let external = ExternalFlatVariant::zod_schema();
    assert!(
        closes_on(
            &external,
            &one_key(
                "z.union([z.strictObject({\n  \"Named\": z.strictObject({\n  x: z.string(),\n})\
                 .and(z.lazy(() => VariantExtra$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n}), z.strictObject({\n  \
                 \"Plain\": z.strictObject({\n  y: z.string(),\n}),\n})])",
                &[("Named", "struct variant"), ("Plain", "struct variant")],
            )
        ),
        "Got: {external}"
    );
    for (schema, expression) in [
        (
            InternalFlatVariant::zod_schema(),
            "z.union([z.strictObject({\n  kind: z.literal(\"Named\"),\n  x: z.string(),\n}).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })\
             .and(z.lazy(() => VariantExtra$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })), z.strictObject({\n  \
             kind: z.literal(\"Plain\"),\n  y: z.string(),\n}).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })])",
        ),
        (
            AdjacentFlatVariant::zod_schema(),
            "z.discriminatedUnion(\"kind\", [z.strictObject({\n  kind: z.literal(\"Named\"),\n  \
             data: z.strictObject({\n  x: z.string(),\n})\
             .and(z.lazy(() => VariantExtra$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n}), z.strictObject({\n  \
             kind: z.literal(\"Plain\"),\n  data: z.strictObject({\n  y: z.string(),\n}),\n})])",
        ),
        (
            UntaggedFlatVariant::zod_schema(),
            "z.union([z.strictObject({ x: z.string(), })\
             .and(z.lazy(() => VariantExtra$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })), z.strictObject({ y: z.string(), })])",
        ),
    ] {
        assert!(closes_on(&schema, expression), "Got: {schema}");
    }
}

#[test]
#[cfg(feature = "zod")]
fn test_two_single_key_set_variant_sources_are_byte_identical() {
    let external = TwiceFlatVariant::zod_schema();
    assert!(
        closes_on(
            &external,
            &one_key(
                "z.union([z.strictObject({\n  \"Named\": z.strictObject({\n  x: z.string(),\n})\
                 .and(z.lazy(() => VariantExtra$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))\
                 .and(z.lazy(() => VariantRank$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true })),\n})])",
                &[("Named", "struct variant")],
            )
        ),
        "Got: {external}"
    );
    let untagged = TwiceUntaggedFlatVariant::zod_schema();
    assert!(
        closes_on(
            &untagged,
            "z.union([z.strictObject({ x: z.string(), })\
             .and(z.lazy(() => VariantExtra$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))\
             .and(z.lazy(() => VariantRank$Schema).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === \"custom\" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === \"custom\" && issue.params?.undeserializable === true && issue.message === \"expected value\" ? { ...issue, message: ((held) => held === undefined ? \"invalid type: expected string or map\" : typeof held === \"object\" && held !== null && !Array.isArray(held) ? \"invalid value: map, expected map with a single key\" : \"invalid type: \" + ((held) => typeof held === \"number\" ? (Number.isInteger(held) ? \"integer `\" : \"floating point `\") + held + \"`\" : Object.prototype.toString.call(held) === \"[object Number]\" ? \"floating point `\" + Number(held) + \"`\" : typeof held === \"string\" ? \"string \" + JSON.stringify(held) : typeof held === \"boolean\" ? \"boolean `\" + held + \"`\" : Array.isArray(held) ? \"sequence\" : held === null ? \"unit value\" : \"map\")(held) + \", expected string or map\")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) && !(issue.code === \"invalid_value\" && issue.inst?._zod.def.error !== undefined); const width = [\"too_big\", \"too_small\"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))])"
        ),
        "Got: {untagged}"
    );
}

#[test]
#[cfg(feature = "zod")]
fn test_a_variant_flattening_nothing_is_byte_identical() {
    let zod = ExternalChoiceVariant::zod_schema();
    let plain = &zod[zod.find("\"Plain\"").unwrap()..];
    assert!(
        plain.starts_with("\"Plain\": z.object({\n  y: z.string(),\n}),\n})])")
            && closes_on(
                plain,
                &format!(
                    "\"Plain\": z.object({{\n  y: z.string(),\n}}),\n}})]){})",
                    named_arm(&[("Named", "struct variant"), ("Plain", "struct variant")])
                )
            ),
        "Got: {plain}"
    );
}

#[test]
fn the_schema_of_a_nonstrict_holder_refuses_a_stray_key_its_strict_member_refuses() {
    let stray = r#"{"depth":2,"ink":"red","stray":1,"title":"t"}"#;
    serde_json::from_str::<GuestLetter>(stray).unwrap_err();
    assert_eq!(
        GuestLetter::json_schema()["additionalProperties"],
        serde_json::json!(false)
    );
}

#[cfg(feature = "zod")]
#[test]
fn the_zod_of_a_nonstrict_holder_closes_the_object_of_its_strict_member() {
    assert!(GuestLetter::zod_schema().contains("z.strictObject({\n  title: z.string(),"));
}

#[test]
fn the_schema_of_a_nonstrict_enum_refuses_a_stray_key_in_the_variant_that_refuses_one() {
    serde_json::from_str::<Docket>(r#"{"depth":2,"ink":"red","kind":"Stamped","stray":1}"#)
        .unwrap_err();
    serde_json::from_str::<Docket>(r#"{"kind":"Noted","note":"n","stray":1}"#).unwrap();
    let written = serde_json::to_string(&Docket::json_schema()).unwrap();
    assert_eq!(
        written.matches(r#""additionalProperties":true"#).count(),
        1,
        "{written}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn the_zod_of_a_nonstrict_enum_closes_the_variant_that_refuses_a_stray_key() {
    let zod = Docket::zod_schema();
    assert!(
        zod.contains("z.object({\n  kind: z.literal(\"Noted\"),")
            && zod.contains("z.strictObject({\n  kind: z.literal(\"Stamped\"),\n})"),
        "{zod}"
    );
}
