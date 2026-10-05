//! A model type declared by `macro_rules!`, with a field type handed in from the call site,
//! describes as the same declaration written by hand.

#[cfg(all(feature = "serde", feature = "typescript"))]
mod a_field_type_handed_in_from_the_call_site {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    macro_rules! choosing {
        ($name:ident, $held:ident) => {
            #[model_schema()]
            #[derive(Debug, Deserialize, Serialize)]
            #[serde(tag = "kind")]
            pub enum $name {
                Held { held: $held },
                Listed { held: Vec<$held> },
            }
        };
    }

    macro_rules! filling {
        ($name:ident, $held:ident) => {
            #[model_schema()]
            #[derive(Debug, Deserialize, Serialize)]
            pub struct $name {
                pub held: Boxed<$held>,
            }
        };
    }

    macro_rules! holding {
        ($name:ident, $held:ident) => {
            #[model_schema()]
            #[derive(Debug, Deserialize, Serialize)]
            pub struct $name {
                pub held: $held,
                pub id: String,
            }
        };
    }

    macro_rules! holding_a_type {
        ($name:ident, $held:ty) => {
            #[model_schema()]
            #[derive(Debug, Deserialize, Serialize)]
            pub struct $name {
                pub held: $held,
                pub id: String,
            }
        };
    }

    macro_rules! slotted {
        ($name:ident, $held:ident) => {
            #[model_schema()]
            #[derive(Debug, Deserialize, Serialize)]
            pub struct $name(pub $held, pub String);
        };
    }

    /// Declared ahead of every type that holds one, hand-written or not, so that neither kind
    /// reaches it as a type declared later.
    #[model_schema(default_types(T = String))]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Boxed<T> {
        pub boxed: T,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Carried {
        pub number: i32,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(tag = "kind")]
    pub enum HandChosen {
        Held { held: Carried },
        Listed { held: Vec<Carried> },
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct HandFilled {
        pub held: Boxed<Carried>,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct HandHolder {
        pub held: Carried,
        pub id: String,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct HandSlotted(pub Carried, pub String);

    choosing!(MacroChosen, Carried);
    filling!(MacroFilled, Carried);
    holding!(MacroHolder, Carried);
    holding_a_type!(MacroHolderOfAType, Carried);
    slotted!(MacroSlotted, Carried);

    /// `text` with the hand-written type's name replaced by the name the macro declared.
    fn named(text: &str, declared: &str) -> String {
        let hand_written = declared
            .replace("MacroHolderOfAType", "HandHolder")
            .replace("Macro", "Hand");
        text.replace(&hand_written, declared)
    }

    #[test]
    fn each_shape_publishes_the_typescript_of_the_hand_written_one() {
        assert_eq!(
            MacroHolder::ts_definition(),
            named(&HandHolder::ts_definition(), "MacroHolder")
        );
        assert_eq!(
            MacroHolderOfAType::ts_definition(),
            named(&HandHolder::ts_definition(), "MacroHolderOfAType")
        );
        assert_eq!(
            MacroChosen::ts_definition(),
            named(&HandChosen::ts_definition(), "MacroChosen")
        );
        assert_eq!(
            MacroSlotted::ts_definition(),
            named(&HandSlotted::ts_definition(), "MacroSlotted")
        );
        assert_eq!(
            MacroFilled::ts_definition(),
            named(&HandFilled::ts_definition(), "MacroFilled")
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn each_shape_publishes_the_zod_schema_of_the_hand_written_one() {
        assert_eq!(
            MacroHolder::zod_schema(),
            named(&HandHolder::zod_schema(), "MacroHolder")
        );
        assert_eq!(
            MacroHolderOfAType::zod_schema(),
            named(&HandHolder::zod_schema(), "MacroHolderOfAType")
        );
        assert_eq!(
            MacroChosen::zod_schema(),
            named(&HandChosen::zod_schema(), "MacroChosen")
        );
        assert_eq!(
            MacroSlotted::zod_schema(),
            named(&HandSlotted::zod_schema(), "MacroSlotted")
        );
        assert_eq!(
            MacroFilled::zod_schema(),
            named(&HandFilled::zod_schema(), "MacroFilled")
        );
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn each_shape_publishes_the_json_schema_of_the_hand_written_one() {
        assert_eq!(MacroHolder::json_schema(), HandHolder::json_schema());
        assert_eq!(MacroHolderOfAType::json_schema(), HandHolder::json_schema());
        assert_eq!(MacroChosen::json_schema(), HandChosen::json_schema());
        assert_eq!(MacroSlotted::json_schema(), HandSlotted::json_schema());
        assert_eq!(MacroFilled::json_schema(), HandFilled::json_schema());
    }
}
