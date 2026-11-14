//! The declarations of `run_kotlin_held_models.rs`: the models a holder holds, and the holder,
//! each written in the layout a module of that file names.

macro_rules! held_types {
    (
        check: $check:tt,
        crated: $crated:ident,
        pair: $pair:ident,
        pick: $pick:ident,
        plain: $plain:ident,
        served: $served:ident,
        shape: $shape:ident,
        stuff: $stuff:ident,
        tag: $tag:ident,
    ) => {
        #[model_schema()]
        #[derive(Clone, Debug, Deserialize, Serialize)]
        pub struct $stuff {
            #[model_schema_prop(minLength = 1)]
            pub name: String,
        }

        #[model_schema(minLength = 2)]
        #[derive(Clone, Debug, Deserialize, Serialize)]
        #[serde(transparent)]
        pub struct $tag(String);

        #[model_schema(check = [$check])]
        #[derive(Clone, Debug, Deserialize, Serialize)]
        pub enum $pick {
            A,
            B,
        }

        #[model_schema()]
        #[derive(Clone, Debug, Deserialize, Serialize)]
        pub enum $plain {
            A,
            B,
        }

        #[model_schema()]
        #[derive(Clone, Debug, Deserialize, Serialize)]
        pub struct $pair(#[model_schema_prop(minLength = 1)] pub String, pub u32);

        #[model_schema()]
        #[derive(Clone, Debug, Deserialize, Serialize)]
        pub struct $crated<T> {
            pub content: T,
            #[model_schema_prop(minLength = 1)]
            pub label: String,
        }

        #[model_schema()]
        #[derive(Clone, Debug, Deserialize, Serialize)]
        #[serde(tag = "kind")]
        pub enum $shape {
            Bare,
            Named {
                #[model_schema_prop(minLength = 1)]
                name: String,
            },
        }

        #[model_schema(__tixschema_undrafted)]
        #[derive(Clone, Debug, Deserialize, Serialize)]
        pub struct $served {
            pub note: String,
        }
    };
}

macro_rules! holder {
    (
        crated: $crated:ident,
        holder: $holder:ident,
        pair: $pair:ident,
        pick: $pick:ident,
        plain: $plain:ident,
        served: $served:ident,
        shape: $shape:ident,
        stuff: $stuff:ident,
        tag: $tag:ident,
    ) => {
        #[model_schema()]
        #[derive(Clone, Debug, Deserialize, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $holder {
            pub crated: $crated<u32>,
            #[model_schema_prop(minLength = 1)]
            pub label: String,
            pub pair: $pair,
            pub pick: $pick,
            pub plain: $plain,
            pub served: $served,
            pub shape: $shape,
            pub stuff: $stuff,
            pub tag: $tag,
        }
    };
}
