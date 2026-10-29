//! `preprocess` in the Kotlin client: the functions a type and its members name, each called on a
//! payload inside the unjudged read, ahead of what reads the payload it answers.
//!
//! A type's functions are called by `{Type}Unjudged`, on the whole payload. A member's are called
//! by a serializer of the member's own where the plugin reads the member, by whatever hands the
//! plugin its object where the member's type reaches a type parameter, and in place where the
//! emitter writes the read.

use core::cell::RefCell;
use core::fmt::Write as _;
use std::collections::HashMap;

use super::{KotlinField, field_reaches_type_parameter, kotlin_name, kotlin_quoted, read};
use crate::features::model_schema_prop::FunctionName;
use crate::field_type::{FieldDef, FieldDefType};
use crate::rename_rule::RenameRule;

/// A class whose serializer the plugin writes, which reads each member by what its property
/// names.
pub struct Plugin<'item> {
    /// The class's name, which leads the name of each serializer a member is read through.
    pub owner: &'item str,
    pub type_parameters: &'item [String],
}

thread_local! {
    /// Whether each type the emitter wrote hands the object it is read from to a function, by
    /// Rust ident: the type names one, or flattens a member that is handed one.
    static HANDED: RefCell<HashMap<String, bool>> = RefCell::new(HashMap::new());
    /// The serializers the members of the item being written are read through, each a file-level
    /// declaration. Drained by [`serializers`].
    static SERIALIZERS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

impl Plugin<'_> {
    /// How the member `rust_name`, typed `def`, is declared: the annotation that names its
    /// serializer, and its type. A member that names a function and reaches no type parameter is
    /// read through a serializer of its own, which this queues.
    pub fn declared(&self, rust_name: &str, wire_name: &str, def: &FieldDef) -> (String, String) {
        let functions = of(def);
        if functions.is_empty() || self.reaches(def) {
            return read::plugin_read(def);
        }
        let (ty, read_by) = read::read_whole(def, self.type_parameters);
        let named = format!(
            "{}{}Preprocess",
            self.owner,
            RenameRule::PascalCase.apply_to_field(rust_name)
        );
        let written = read::corrected_member(
            &named,
            &ty,
            &read_by,
            &corrected(functions, "element"),
            wire_name,
        );
        SERIALIZERS.with(|queued| queued.borrow_mut().push(written));
        (format!("@Serializable(with = {named}::class) "), ty)
    }

    /// The members of `fields` whose functions are called at their key by whatever hands the
    /// plugin the object: those whose type reaches a type parameter, which the plugin takes no
    /// serializer of the member's own for.
    pub fn keyed(&self, fields: &[KotlinField]) -> Vec<(String, Vec<FunctionName>)> {
        fields
            .iter()
            .filter(|field| self.reaches(&field.field_def))
            .filter_map(|field| {
                let functions = of(&field.field_def);
                (!functions.is_empty()).then(|| (field.wire_name.clone(), functions.to_vec()))
            })
            .collect()
    }

    fn reaches(&self, def: &FieldDef) -> bool {
        field_reaches_type_parameter(def, self.type_parameters)
    }
}

/// `payload`, the Kotlin expression of a `JsonElement`, as `functions` correct it: each called on
/// the answer of the one before, in the order written, under the name the attribute writes.
pub fn corrected(functions: &[FunctionName], payload: &str) -> String {
    functions.iter().fold(payload.to_owned(), |held, function| {
        format!("{}({held})", kotlin_name(&function.name))
    })
}

/// `payload` with what an object holds under each key of `members` corrected by that member's
/// functions, and a payload that is no object as it is.
pub fn keyed(payload: &str, members: &[(String, Vec<FunctionName>)]) -> String {
    if members.is_empty() {
        return payload.to_owned();
    }
    let arms = members
        .iter()
        .fold(String::new(), |mut written, (key, functions)| {
            let _ = write!(
                written,
                "{} -> {}; ",
                kotlin_quoted(key),
                corrected(functions, "held")
            );
            written
        });
    format!(
        "{payload}.let {{ whole: JsonElement -> if (whole is JsonObject) \
         JsonObject(whole.mapValues {{ (key, held) -> when (key) {{ {arms}else -> held }} }}) \
         else whole }}"
    )
}

/// Notes whether the type written under `rust_ident` hands the object it is read from to a
/// function.
pub fn note(rust_ident: &str, hands: bool) {
    HANDED.with(|noted| {
        noted.borrow_mut().insert(rust_ident.to_owned(), hands);
    });
}

/// The functions the member typed `def` names, in the order written.
pub fn of(def: &FieldDef) -> &[FunctionName] {
    def.model_schema_prop_meta
        .as_ref()
        .map_or(&[], |meta| meta.preprocess.as_slice())
}

/// The serializers queued for the item being written.
pub fn serializers() -> Vec<String> {
    SERIALIZERS.with(RefCell::take)
}

/// Whether a member typed `def`, flattened into its holder, takes none of the keys it reads, as
/// serde's read of it takes none: a member that names a function, and one whose type is no map
/// and is not known to hand its object to none, a type the emitter has not written among them.
pub fn takes_no_key(def: &FieldDef) -> bool {
    if !of(def).is_empty() {
        return true;
    }
    if let FieldDefType::SiblingType(name, _) = &def.field_type {
        HANDED.with(|noted| noted.borrow().get(name).copied() != Some(false))
    } else {
        !matches!(def.field_type, FieldDefType::Map(_, _))
    }
}
