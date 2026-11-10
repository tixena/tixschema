//! The sixth of eight parts of the shapes of flattened members serde was measured to read
//! back that `#[model_schema]` declares, generated once from that measure: each holder's
//! rows are written, read back equal, and read with a stray key first, which the holder refuses
//! (`R`), reads past (`E`) or reads into a map (`U`), as the measure found, but for the unit
//! variant of a strict enum under a tag alone, which `#[model_schema]` reads refusing the key
//! that serde's own derive reads past.

#![cfg(feature = "serde")]

extern crate alloc;

#[cfg(test)]
#[path = "flatten_matrix/harness.rs"]
mod harness;

#[cfg(test)]
mod holders {
    use alloc::collections::BTreeMap;

    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    use super::harness::{Sample, check, rows1, rows2, rows3};

    /// Each holder of this part, written, read back and read with a stray key first.
    const HOLDERS: [fn() -> Vec<String>; 1106] = [
        holder5, holder13, holder21, holder29, holder37, holder45, holder53, holder61, holder69,
        holder77, holder85, holder93, holder101, holder109, holder117, holder125, holder133,
        holder141, holder149, holder157, holder165, holder173, holder181, holder189, holder197,
        holder205, holder213, holder221, holder229, holder237, holder245, holder253, holder261,
        holder269, holder277, holder285, holder293, holder301, holder309, holder317, holder325,
        holder333, holder341, holder349, holder357, holder365, holder373, holder381, holder389,
        holder397, holder405, holder413, holder421, holder429, holder437, holder445, holder453,
        holder461, holder469, holder477, holder485, holder493, holder501, holder509, holder517,
        holder525, holder533, holder541, holder549, holder557, holder565, holder573, holder581,
        holder589, holder597, holder605, holder613, holder621, holder629, holder637, holder645,
        holder653, holder661, holder669, holder677, holder685, holder693, holder701, holder709,
        holder717, holder725, holder733, holder741, holder749, holder757, holder765, holder773,
        holder781, holder789, holder797, holder805, holder813, holder821, holder829, holder837,
        holder845, holder853, holder861, holder869, holder877, holder885, holder893, holder901,
        holder909, holder917, holder925, holder933, holder941, holder949, holder957, holder965,
        holder973, holder981, holder989, holder997, holder1005, holder1013, holder1021, holder1029,
        holder1037, holder1045, holder1053, holder1061, holder1069, holder1077, holder1085,
        holder1093, holder1101, holder1109, holder1117, holder1125, holder1133, holder1141,
        holder1149, holder1157, holder1165, holder1173, holder1181, holder1189, holder1197,
        holder1205, holder1213, holder1221, holder1229, holder1237, holder1245, holder1253,
        holder1261, holder1269, holder1277, holder1285, holder1293, holder1301, holder1309,
        holder1317, holder1325, holder1333, holder1341, holder1349, holder1357, holder1365,
        holder1373, holder1381, holder1389, holder1397, holder1405, holder1413, holder1421,
        holder1429, holder1437, holder1445, holder1453, holder1461, holder1469, holder1477,
        holder1485, holder1493, holder1501, holder1509, holder1517, holder1525, holder1533,
        holder1541, holder1549, holder1557, holder1565, holder1573, holder1581, holder1589,
        holder1597, holder1605, holder1613, holder1621, holder1629, holder1637, holder1645,
        holder1653, holder1661, holder1669, holder1677, holder1685, holder1693, holder1701,
        holder1709, holder1717, holder1725, holder1733, holder1741, holder1749, holder1757,
        holder1765, holder1773, holder1781, holder1789, holder1797, holder1805, holder1813,
        holder1821, holder1829, holder1837, holder1845, holder1853, holder1861, holder1869,
        holder1877, holder1885, holder1893, holder1901, holder1909, holder1917, holder1925,
        holder1933, holder1941, holder1949, holder1957, holder1965, holder1973, holder1981,
        holder1989, holder1997, holder2005, holder2013, holder2021, holder2029, holder2037,
        holder2045, holder2053, holder2061, holder2069, holder2077, holder2085, holder2093,
        holder2101, holder2109, holder2117, holder2125, holder2133, holder2141, holder2149,
        holder2157, holder2165, holder2173, holder2181, holder2189, holder2197, holder2205,
        holder2213, holder2221, holder2229, holder2237, holder2245, holder2253, holder2261,
        holder2269, holder2277, holder2285, holder2293, holder2301, holder2309, holder2317,
        holder2325, holder2333, holder2341, holder2349, holder2357, holder2365, holder2373,
        holder2381, holder2389, holder2397, holder2405, holder2413, holder2421, holder2429,
        holder2437, holder2445, holder2453, holder2461, holder2469, holder2477, holder2485,
        holder2493, holder2501, holder2509, holder2517, holder2525, holder2533, holder2541,
        holder2549, holder2557, holder2565, holder2573, holder2581, holder2589, holder2597,
        holder2605, holder2613, holder2621, holder2629, holder2637, holder2645, holder2653,
        holder2661, holder2669, holder2677, holder2685, holder2693, holder2701, holder2709,
        holder2717, holder2725, holder2733, holder2741, holder2749, holder2757, holder2765,
        holder2773, holder2781, holder2789, holder2797, holder2805, holder2813, holder2821,
        holder2829, holder2837, holder2845, holder2853, holder2861, holder2869, holder2877,
        holder2885, holder2893, holder2901, holder2909, holder2917, holder2925, holder2933,
        holder2941, holder2949, holder2957, holder2965, holder2973, holder2981, holder2989,
        holder2997, holder3005, holder3013, holder3021, holder3029, holder3037, holder3045,
        holder3053, holder3061, holder3069, holder3077, holder3085, holder3093, holder3101,
        holder3109, holder3117, holder3125, holder3133, holder3141, holder3149, holder3157,
        holder3165, holder3173, holder3181, holder3189, holder3197, holder3205, holder3213,
        holder3221, holder3229, holder3237, holder3245, holder3253, holder3261, holder3269,
        holder3277, holder3285, holder3293, holder3301, holder3309, holder3317, holder3325,
        holder3333, holder3341, holder3349, holder3357, holder3365, holder3373, holder3381,
        holder3389, holder3397, holder3405, holder3413, holder3421, holder3429, holder3437,
        holder3445, holder3453, holder3461, holder3469, holder3477, holder3485, holder3493,
        holder3501, holder3509, holder3517, holder3525, holder3533, holder3541, holder3549,
        holder3557, holder3565, holder3573, holder3581, holder3589, holder3597, holder3605,
        holder3613, holder3621, holder3629, holder3637, holder3645, holder3653, holder3661,
        holder3669, holder3677, holder3685, holder3693, holder3701, holder3709, holder3717,
        holder3725, holder3733, holder3741, holder3749, holder3757, holder3765, holder3773,
        holder3781, holder3789, holder3797, holder3805, holder3813, holder3821, holder3829,
        holder3837, holder3845, holder3853, holder3861, holder3869, holder3877, holder3885,
        holder3893, holder3901, holder3909, holder3917, holder3925, holder3933, holder3941,
        holder3949, holder3957, holder3965, holder3973, holder3981, holder3989, holder3997,
        holder4005, holder4013, holder4021, holder4029, holder4037, holder4045, holder4053,
        holder4061, holder4069, holder4077, holder4085, holder4093, holder4101, holder4109,
        holder4117, holder4125, holder4133, holder4141, holder4149, holder4157, holder4165,
        holder4173, holder4181, holder4189, holder4197, holder4205, holder4213, holder4221,
        holder4229, holder4237, holder4245, holder4253, holder4261, holder4269, holder4277,
        holder4285, holder4293, holder4301, holder4309, holder4317, holder4325, holder4333,
        holder4341, holder4349, holder4357, holder4365, holder4373, holder4381, holder4389,
        holder4397, holder4405, holder4413, holder4421, holder4429, holder4437, holder4445,
        holder4453, holder4461, holder4469, holder4477, holder4485, holder4493, holder4501,
        holder4509, holder4517, holder4525, holder4533, holder4541, holder4549, holder4557,
        holder4565, holder4573, holder4581, holder4589, holder4597, holder4605, holder4613,
        holder4621, holder4629, holder4637, holder4645, holder4653, holder4661, holder4669,
        holder4677, holder4685, holder4693, holder4701, holder4709, holder4717, holder4725,
        holder4733, holder4741, holder4749, holder4757, holder4765, holder4773, holder4781,
        holder4789, holder4797, holder4805, holder4813, holder4821, holder4829, holder4837,
        holder4845, holder4853, holder4861, holder4869, holder4877, holder4885, holder4893,
        holder4901, holder4909, holder4917, holder4925, holder4933, holder4941, holder4949,
        holder4957, holder4965, holder4973, holder4981, holder4989, holder4997, holder5005,
        holder5013, holder5021, holder5029, holder5037, holder5045, holder5053, holder5061,
        holder5069, holder5077, holder5085, holder5093, holder5101, holder5109, holder5117,
        holder5125, holder5133, holder5141, holder5149, holder5157, holder5165, holder5173,
        holder5181, holder5189, holder5197, holder5205, holder5213, holder5221, holder5229,
        holder5237, holder5245, holder5253, holder5261, holder5269, holder5277, holder5285,
        holder5293, holder5301, holder5309, holder5317, holder5325, holder5333, holder5341,
        holder5349, holder5357, holder5365, holder5373, holder5381, holder5389, holder5397,
        holder5405, holder5413, holder5421, holder5429, holder5437, holder5445, holder5453,
        holder5461, holder5469, holder5477, holder5485, holder5493, holder5501, holder5509,
        holder5517, holder5525, holder5533, holder5541, holder5549, holder5557, holder5565,
        holder5573, holder5581, holder5589, holder5597, holder5605, holder5613, holder5621,
        holder5629, holder5637, holder5645, holder5653, holder5661, holder5669, holder5677,
        holder5685, holder5693, holder5701, holder5709, holder5717, holder5725, holder5733,
        holder5741, holder5749, holder5757, holder5765, holder5773, holder5781, holder5789,
        holder5797, holder5805, holder5813, holder5821, holder5829, holder5837, holder5845,
        holder5853, holder5861, holder5869, holder5877, holder5885, holder5893, holder5901,
        holder5909, holder5917, holder5925, holder5933, holder5941, holder5949, holder5957,
        holder5965, holder5973, holder5981, holder5989, holder5997, holder6005, holder6013,
        holder6021, holder6029, holder6037, holder6045, holder6053, holder6061, holder6069,
        holder6077, holder6085, holder6093, holder6101, holder6109, holder6117, holder6125,
        holder6133, holder6141, holder6149, holder6157, holder6165, holder6173, holder6181,
        holder6189, holder6197, holder6205, holder6213, holder6221, holder6229, holder6237,
        holder6245, holder6253, holder6261, holder6269, holder6277, holder6285, holder6293,
        holder6301, holder6309, holder6317, holder6325, holder6333, holder6341, holder6349,
        holder6357, holder6365, holder6373, holder6381, holder6389, holder6397, holder6405,
        holder6413, holder6421, holder6429, holder6437, holder6445, holder6453, holder6461,
        holder6469, holder6477, holder6485, holder6493, holder6501, holder6509, holder6517,
        holder6525, holder6533, holder6541, holder6549, holder6557, holder6565, holder6573,
        holder6581, holder6589, holder6597, holder6605, holder6613, holder6621, holder6629,
        holder6637, holder6645, holder6653, holder6661, holder6669, holder6677, holder6685,
        holder6693, holder6701, holder6709, holder6717, holder6725, holder6733, holder6741,
        holder6749, holder6757, holder6765, holder6773, holder6781, holder6789, holder6797,
        holder6805, holder6813, holder6821, holder6829, holder6837, holder6845, holder6853,
        holder6861, holder6869, holder6877, holder6885, holder6893, holder6901, holder6909,
        holder6917, holder6925, holder6933, holder6941, holder6949, holder6957, holder6965,
        holder6973, holder6981, holder6989, holder6997, holder7005, holder7013, holder7021,
        holder7029, holder7037, holder7045, holder7053, holder7061, holder7069, holder7077,
        holder7085, holder7093, holder7101, holder7109, holder7117, holder7125, holder7133,
        holder7141, holder7149, holder7157, holder7165, holder7173, holder7181, holder7189,
        holder7197, holder7205, holder7213, holder7221, holder7229, holder7237, holder7245,
        holder7253, holder7261, holder7269, holder7277, holder7285, holder7293, holder7301,
        holder7309, holder7317, holder7325, holder7333, holder7341, holder7349, holder7357,
        holder7365, holder7373, holder7381, holder7389, holder7397, holder7405, holder7413,
        holder7421, holder7429, holder7437, holder7445, holder7453, holder7461, holder7469,
        holder7477, holder7485, holder7493, holder7501, holder7509, holder7517, holder7525,
        holder7533, holder7541, holder7549, holder7557, holder7565, holder7573, holder7581,
        holder7589, holder7597, holder7605, holder7613, holder7621, holder7629, holder7637,
        holder7645, holder7653, holder7661, holder7669, holder7677, holder7685, holder7693,
        holder7701, holder7709, holder7717, holder7725, holder7733, holder7741, holder7749,
        holder7757, holder7765, holder7773, holder7781, holder7789, holder7797, holder7805,
        holder7813, holder7821, holder7829, holder7837, holder7845, holder7853, holder7861,
        holder7869, holder7877, holder7885, holder7893, holder7901, holder7909, holder7917,
        holder7925, holder7933, holder7941, holder7949, holder7957, holder7965, holder7973,
        holder7981, holder7989, holder7997, holder8005, holder8013, holder8021, holder8029,
        holder8037, holder8045, holder8053, holder8061, holder8069, holder8077, holder8085,
        holder8093, holder8101, holder8109, holder8117, holder8125, holder8133, holder8141,
        holder8149, holder8157, holder8165, holder8173, holder8181, holder8189, holder8197,
        holder8205, holder8213, holder8221, holder8229, holder8237, holder8245, holder8253,
        holder8261, holder8269, holder8277, holder8285, holder8293, holder8301, holder8309,
        holder8317, holder8325, holder8333, holder8341, holder8349, holder8357, holder8365,
        holder8373, holder8381, holder8389, holder8397, holder8405, holder8413, holder8421,
        holder8429, holder8437, holder8445, holder8453, holder8461, holder8469, holder8477,
        holder8485, holder8493, holder8501, holder8509, holder8517, holder8525, holder8533,
        holder8541, holder8549, holder8557, holder8565, holder8573, holder8581, holder8589,
        holder8597, holder8605, holder8613, holder8621, holder8629, holder8637, holder8645,
        holder8653, holder8661, holder8669, holder8677, holder8685, holder8693, holder8701,
        holder8709, holder8717, holder8725, holder8733, holder8741, holder8749, holder8757,
        holder8765, holder8773, holder8781, holder8789, holder8797, holder8805, holder8813,
        holder8821, holder8829, holder8837, holder8845,
    ];

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CAW10NN {
        #[serde(rename = "a10_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind", content = "a10_c")]
    pub enum CA10NN {
        Dot,
        Rect {
            #[serde(rename = "a10_a")]
            a: u32,
        },
        Wrap(CAW10NN),
    }
    impl Sample for CA10NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 40 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(CAW10NN { b: 45 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CAW10SS {
        #[serde(rename = "a10_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind", content = "a10_c")]
    pub enum CA10SS {
        Dot,
        Rect {
            #[serde(rename = "a10_a")]
            a: u32,
        },
        Wrap(CAW10SS),
    }
    impl Sample for CA10SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 40 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(CAW10SS { b: 45 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CFTW10NNN {
        #[serde(rename = "ft10_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind")]
    pub enum CFTI10NNN {
        Dot,
        Rect {
            #[serde(rename = "ft10_a")]
            a: u32,
        },
        Wrap(CFTW10NNN),
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CFT10NNN {
        #[serde(flatten)]
        pub inner: CFTI10NNN,
        #[serde(rename = "ft10_own")]
        pub own: u32,
    }
    impl Sample for CFT10NNN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                (
                    "Rect".to_owned(),
                    Self {
                        own: 80,
                        inner: CFTI10NNN::Rect { a: 82 },
                    },
                ),
                (
                    "Dot".to_owned(),
                    Self {
                        own: 80,
                        inner: CFTI10NNN::Dot,
                    },
                ),
                (
                    "Wrap".to_owned(),
                    Self {
                        own: 80,
                        inner: CFTI10NNN::Wrap(CFTW10NNN { b: 85 }),
                    },
                ),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CTW10NN {
        #[serde(rename = "t10_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind")]
    pub enum CT10NN {
        Dot,
        Rect {
            #[serde(rename = "t10_a")]
            a: u32,
        },
        Wrap(CTW10NN),
    }
    impl Sample for CT10NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 50 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(CTW10NN { b: 55 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CEW11NN {
        #[serde(rename = "e11_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum CE11NN {
        #[serde(rename = "e11_dot")]
        Dot,
        #[serde(rename = "shared")]
        Rect {
            #[serde(rename = "e11_a")]
            a: u32,
        },
        #[serde(rename = "e11_wrap")]
        Wrap(CEW11NN),
    }
    impl Sample for CE11NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 31 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(CEW11NN { b: 36 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CFOY11NN {
        #[serde(rename = "fo11_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CFOX11NN {
        #[serde(rename = "shared")]
        pub a: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CFO11NN {
        #[serde(flatten)]
        pub x: CFOX11NN,
        #[serde(flatten)]
        pub y: CFOY11NN,
    }
    impl Sample for CFO11NN {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    x: CFOX11NN { a: 111 },
                    y: CFOY11NN { b: 136 },
                },
            )]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CFPI11NN {
        #[serde(rename = "fp11_a")]
        pub a: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CFP11NN {
        #[serde(flatten)]
        pub inner: CFPI11NN,
        #[serde(rename = "shared")]
        pub own: u32,
    }
    impl Sample for CFP11NN {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    own: 111,
                    inner: CFPI11NN { a: 76 },
                },
            )]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct COI11N {
        #[serde(rename = "shared")]
        pub a: u32,
    }
    impl Sample for COI11N {
        fn samples() -> Vec<(String, Self)> {
            vec![("Some".to_owned(), Self { a: 111 })]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CP11S {
        #[serde(rename = "shared")]
        pub a: u32,
    }
    impl Sample for CP11S {
        fn samples() -> Vec<(String, Self)> {
            vec![("-".to_owned(), Self { a: 111 })]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CVW11NN {
        #[serde(rename = "v11_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum CV11NN {
        Rect {
            #[serde(rename = "shared")]
            a: u32,
        },
        Wrap(CVW11NN),
    }
    impl Sample for CV11NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 111 }),
                ("Wrap".to_owned(), Self::Wrap(CVW11NN { b: 146 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CEW10NN {
        #[serde(rename = "e10_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum CE10NN {
        #[serde(rename = "e10_dot")]
        Dot,
        #[serde(rename = "shared")]
        Rect {
            #[serde(rename = "e10_a")]
            a: u32,
        },
        #[serde(rename = "e10_wrap")]
        Wrap(CEW10NN),
    }
    impl Sample for CE10NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 30 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(CEW10NN { b: 35 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CAW11NN {
        #[serde(rename = "a11_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind", content = "a11_c")]
    pub enum CA11NN {
        Dot,
        Rect {
            #[serde(rename = "a11_a")]
            a: u32,
        },
        Wrap(CAW11NN),
    }
    impl Sample for CA11NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 41 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(CAW11NN { b: 46 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CAW11SS {
        #[serde(rename = "a11_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind", content = "a11_c")]
    pub enum CA11SS {
        Dot,
        Rect {
            #[serde(rename = "a11_a")]
            a: u32,
        },
        Wrap(CAW11SS),
    }
    impl Sample for CA11SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 41 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(CAW11SS { b: 46 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CFTW11NNN {
        #[serde(rename = "ft11_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind")]
    pub enum CFTI11NNN {
        Dot,
        Rect {
            #[serde(rename = "ft11_a")]
            a: u32,
        },
        Wrap(CFTW11NNN),
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CFT11NNN {
        #[serde(flatten)]
        pub inner: CFTI11NNN,
        #[serde(rename = "ft11_own")]
        pub own: u32,
    }
    impl Sample for CFT11NNN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                (
                    "Rect".to_owned(),
                    Self {
                        own: 81,
                        inner: CFTI11NNN::Rect { a: 83 },
                    },
                ),
                (
                    "Dot".to_owned(),
                    Self {
                        own: 81,
                        inner: CFTI11NNN::Dot,
                    },
                ),
                (
                    "Wrap".to_owned(),
                    Self {
                        own: 81,
                        inner: CFTI11NNN::Wrap(CFTW11NNN { b: 86 }),
                    },
                ),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CTW11NN {
        #[serde(rename = "t11_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind")]
    pub enum CT11NN {
        Dot,
        Rect {
            #[serde(rename = "t11_a")]
            a: u32,
        },
        Wrap(CTW11NN),
    }
    impl Sample for CT11NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 51 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(CTW11NN { b: 56 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CTW11SS {
        #[serde(rename = "t11_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind")]
    pub enum CT11SS {
        Dot,
        Rect {
            #[serde(rename = "t11_a")]
            a: u32,
        },
        Wrap(CTW11SS),
    }
    impl Sample for CT11SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 51 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(CTW11SS { b: 56 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CEW10SS {
        #[serde(rename = "e10_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum CE10SS {
        #[serde(rename = "e10_dot")]
        Dot,
        #[serde(rename = "shared")]
        Rect {
            #[serde(rename = "e10_a")]
            a: u32,
        },
        #[serde(rename = "e10_wrap")]
        Wrap(CEW10SS),
    }
    impl Sample for CE10SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 30 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(CEW10SS { b: 35 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CFOY10NN {
        #[serde(rename = "fo10_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CFOX10NN {
        #[serde(rename = "shared")]
        pub a: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CFO10NN {
        #[serde(flatten)]
        pub x: CFOX10NN,
        #[serde(flatten)]
        pub y: CFOY10NN,
    }
    impl Sample for CFO10NN {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    x: CFOX10NN { a: 110 },
                    y: CFOY10NN { b: 135 },
                },
            )]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CFPI10NN {
        #[serde(rename = "fp10_a")]
        pub a: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CFP10NN {
        #[serde(flatten)]
        pub inner: CFPI10NN,
        #[serde(rename = "shared")]
        pub own: u32,
    }
    impl Sample for CFP10NN {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    own: 110,
                    inner: CFPI10NN { a: 75 },
                },
            )]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct COI10N {
        #[serde(rename = "shared")]
        pub a: u32,
    }
    impl Sample for COI10N {
        fn samples() -> Vec<(String, Self)> {
            vec![("Some".to_owned(), Self { a: 110 })]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct COI10S {
        #[serde(rename = "shared")]
        pub a: u32,
    }
    impl Sample for COI10S {
        fn samples() -> Vec<(String, Self)> {
            vec![("Some".to_owned(), Self { a: 110 })]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CP10N {
        #[serde(rename = "shared")]
        pub a: u32,
    }
    impl Sample for CP10N {
        fn samples() -> Vec<(String, Self)> {
            vec![("-".to_owned(), Self { a: 110 })]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CP10S {
        #[serde(rename = "shared")]
        pub a: u32,
    }
    impl Sample for CP10S {
        fn samples() -> Vec<(String, Self)> {
            vec![("-".to_owned(), Self { a: 110 })]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct CVW10NN {
        #[serde(rename = "v10_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum CV10NN {
        Rect {
            #[serde(rename = "shared")]
            a: u32,
        },
        Wrap(CVW10NN),
    }
    impl Sample for CV10NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 110 }),
                ("Wrap".to_owned(), Self::Wrap(CVW10NN { b: 145 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct AW0NN {
        #[serde(rename = "a0_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "a0_t", content = "a0_c")]
    pub enum A0NN {
        Dot,
        Rect {
            #[serde(rename = "a0_a")]
            a: u32,
        },
        Wrap(AW0NN),
    }
    impl Sample for A0NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 30 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(AW0NN { b: 35 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct AW0SS {
        #[serde(rename = "a0_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "a0_t", content = "a0_c")]
    pub enum A0SS {
        Dot,
        Rect {
            #[serde(rename = "a0_a")]
            a: u32,
        },
        Wrap(AW0SS),
    }
    impl Sample for A0SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 30 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(AW0SS { b: 35 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct EW0NN {
        #[serde(rename = "e0_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum E0NN {
        #[serde(rename = "e0_dot")]
        Dot,
        #[serde(rename = "e0_rect")]
        Rect {
            #[serde(rename = "e0_a")]
            a: u32,
        },
        #[serde(rename = "e0_wrap")]
        Wrap(EW0NN),
    }
    impl Sample for E0NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 20 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(EW0NN { b: 25 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct EW0SS {
        #[serde(rename = "e0_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum E0SS {
        #[serde(rename = "e0_dot")]
        Dot,
        #[serde(rename = "e0_rect")]
        Rect {
            #[serde(rename = "e0_a")]
            a: u32,
        },
        #[serde(rename = "e0_wrap")]
        Wrap(EW0SS),
    }
    impl Sample for E0SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 20 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(EW0SS { b: 25 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOY0NN {
        #[serde(rename = "fo0_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOX0NN {
        #[serde(rename = "fo0_a")]
        pub a: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FO0NN {
        #[serde(flatten)]
        pub x: FOX0NN,
        #[serde(flatten)]
        pub y: FOY0NN,
    }
    impl Sample for FO0NN {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    x: FOX0NN { a: 120 },
                    y: FOY0NN { b: 125 },
                },
            )]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOY0SS {
        #[serde(rename = "fo0_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOX0SS {
        #[serde(rename = "fo0_a")]
        pub a: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FO0SS {
        #[serde(flatten)]
        pub x: FOX0SS,
        #[serde(flatten)]
        pub y: FOY0SS,
    }
    impl Sample for FO0SS {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    x: FOX0SS { a: 120 },
                    y: FOY0SS { b: 125 },
                },
            )]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FPI0NN {
        #[serde(rename = "fp0_a")]
        pub a: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FP0NN {
        #[serde(flatten)]
        pub inner: FPI0NN,
        #[serde(rename = "fp0_own")]
        pub own: u32,
    }
    impl Sample for FP0NN {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    own: 60,
                    inner: FPI0NN { a: 65 },
                },
            )]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FPI0SS {
        #[serde(rename = "fp0_a")]
        pub a: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FP0SS {
        #[serde(flatten)]
        pub inner: FPI0SS,
        #[serde(rename = "fp0_own")]
        pub own: u32,
    }
    impl Sample for FP0SS {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    own: 60,
                    inner: FPI0SS { a: 65 },
                },
            )]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FTW0NNN {
        #[serde(rename = "ft0_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "ft0_t")]
    pub enum FTI0NNN {
        Dot,
        Rect {
            #[serde(rename = "ft0_a")]
            a: u32,
        },
        Wrap(FTW0NNN),
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FT0NNN {
        #[serde(flatten)]
        pub inner: FTI0NNN,
        #[serde(rename = "ft0_own")]
        pub own: u32,
    }
    impl Sample for FT0NNN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                (
                    "Rect".to_owned(),
                    Self {
                        own: 70,
                        inner: FTI0NNN::Rect { a: 72 },
                    },
                ),
                (
                    "Dot".to_owned(),
                    Self {
                        own: 70,
                        inner: FTI0NNN::Dot,
                    },
                ),
                (
                    "Wrap".to_owned(),
                    Self {
                        own: 70,
                        inner: FTI0NNN::Wrap(FTW0NNN { b: 75 }),
                    },
                ),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct OI0N {
        #[serde(rename = "o0_a")]
        pub a: u32,
    }
    impl Sample for OI0N {
        fn samples() -> Vec<(String, Self)> {
            vec![("Some".to_owned(), Self { a: 110 })]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct OI0S {
        #[serde(rename = "o0_a")]
        pub a: u32,
    }
    impl Sample for OI0S {
        fn samples() -> Vec<(String, Self)> {
            vec![("Some".to_owned(), Self { a: 110 })]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct P0N {
        #[serde(rename = "p0_a")]
        pub a: u32,
    }
    impl Sample for P0N {
        fn samples() -> Vec<(String, Self)> {
            vec![("-".to_owned(), Self { a: 10 })]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct P0S {
        #[serde(rename = "p0_a")]
        pub a: u32,
    }
    impl Sample for P0S {
        fn samples() -> Vec<(String, Self)> {
            vec![("-".to_owned(), Self { a: 10 })]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct TW0NN {
        #[serde(rename = "t0_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "t0_t")]
    pub enum T0NN {
        Dot,
        Rect {
            #[serde(rename = "t0_a")]
            a: u32,
        },
        Wrap(TW0NN),
    }
    impl Sample for T0NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 40 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(TW0NN { b: 45 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct TW0SS {
        #[serde(rename = "t0_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "t0_t")]
    pub enum T0SS {
        Dot,
        Rect {
            #[serde(rename = "t0_a")]
            a: u32,
        },
        Wrap(TW0SS),
    }
    impl Sample for T0SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 40 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(TW0SS { b: 45 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct VW0NN {
        #[serde(rename = "v0_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum V0NN {
        Rect {
            #[serde(rename = "v0_a")]
            a: u32,
        },
        Wrap(VW0NN),
    }
    impl Sample for V0NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 130 }),
                ("Wrap".to_owned(), Self::Wrap(VW0NN { b: 135 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct VW0SS {
        #[serde(rename = "v0_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum V0SS {
        Rect {
            #[serde(rename = "v0_a")]
            a: u32,
        },
        Wrap(VW0SS),
    }
    impl Sample for V0SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 130 }),
                ("Wrap".to_owned(), Self::Wrap(VW0SS { b: 135 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct AW1NN {
        #[serde(rename = "a1_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "a1_t", content = "a1_c")]
    pub enum A1NN {
        Dot,
        Rect {
            #[serde(rename = "a1_a")]
            a: u32,
        },
        Wrap(AW1NN),
    }
    impl Sample for A1NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 31 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(AW1NN { b: 36 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct AW1SS {
        #[serde(rename = "a1_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "a1_t", content = "a1_c")]
    pub enum A1SS {
        Dot,
        Rect {
            #[serde(rename = "a1_a")]
            a: u32,
        },
        Wrap(AW1SS),
    }
    impl Sample for A1SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 31 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(AW1SS { b: 36 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct EW1NN {
        #[serde(rename = "e1_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum E1NN {
        #[serde(rename = "e1_dot")]
        Dot,
        #[serde(rename = "e1_rect")]
        Rect {
            #[serde(rename = "e1_a")]
            a: u32,
        },
        #[serde(rename = "e1_wrap")]
        Wrap(EW1NN),
    }
    impl Sample for E1NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 21 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(EW1NN { b: 26 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct EW1SS {
        #[serde(rename = "e1_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum E1SS {
        #[serde(rename = "e1_dot")]
        Dot,
        #[serde(rename = "e1_rect")]
        Rect {
            #[serde(rename = "e1_a")]
            a: u32,
        },
        #[serde(rename = "e1_wrap")]
        Wrap(EW1SS),
    }
    impl Sample for E1SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 21 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(EW1SS { b: 26 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOY1NN {
        #[serde(rename = "fo1_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOX1NN {
        #[serde(rename = "fo1_a")]
        pub a: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FO1NN {
        #[serde(flatten)]
        pub x: FOX1NN,
        #[serde(flatten)]
        pub y: FOY1NN,
    }
    impl Sample for FO1NN {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    x: FOX1NN { a: 121 },
                    y: FOY1NN { b: 126 },
                },
            )]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOY1SS {
        #[serde(rename = "fo1_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOX1SS {
        #[serde(rename = "fo1_a")]
        pub a: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FO1SS {
        #[serde(flatten)]
        pub x: FOX1SS,
        #[serde(flatten)]
        pub y: FOY1SS,
    }
    impl Sample for FO1SS {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    x: FOX1SS { a: 121 },
                    y: FOY1SS { b: 126 },
                },
            )]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FPI1NN {
        #[serde(rename = "fp1_a")]
        pub a: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FP1NN {
        #[serde(flatten)]
        pub inner: FPI1NN,
        #[serde(rename = "fp1_own")]
        pub own: u32,
    }
    impl Sample for FP1NN {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    own: 61,
                    inner: FPI1NN { a: 66 },
                },
            )]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FPI1SS {
        #[serde(rename = "fp1_a")]
        pub a: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FP1SS {
        #[serde(flatten)]
        pub inner: FPI1SS,
        #[serde(rename = "fp1_own")]
        pub own: u32,
    }
    impl Sample for FP1SS {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    own: 61,
                    inner: FPI1SS { a: 66 },
                },
            )]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FTW1NNN {
        #[serde(rename = "ft1_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "ft1_t")]
    pub enum FTI1NNN {
        Dot,
        Rect {
            #[serde(rename = "ft1_a")]
            a: u32,
        },
        Wrap(FTW1NNN),
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FT1NNN {
        #[serde(flatten)]
        pub inner: FTI1NNN,
        #[serde(rename = "ft1_own")]
        pub own: u32,
    }
    impl Sample for FT1NNN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                (
                    "Rect".to_owned(),
                    Self {
                        own: 71,
                        inner: FTI1NNN::Rect { a: 73 },
                    },
                ),
                (
                    "Dot".to_owned(),
                    Self {
                        own: 71,
                        inner: FTI1NNN::Dot,
                    },
                ),
                (
                    "Wrap".to_owned(),
                    Self {
                        own: 71,
                        inner: FTI1NNN::Wrap(FTW1NNN { b: 76 }),
                    },
                ),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct OI1N {
        #[serde(rename = "o1_a")]
        pub a: u32,
    }
    impl Sample for OI1N {
        fn samples() -> Vec<(String, Self)> {
            vec![("Some".to_owned(), Self { a: 111 })]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct OI1S {
        #[serde(rename = "o1_a")]
        pub a: u32,
    }
    impl Sample for OI1S {
        fn samples() -> Vec<(String, Self)> {
            vec![("Some".to_owned(), Self { a: 111 })]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct P1N {
        #[serde(rename = "p1_a")]
        pub a: u32,
    }
    impl Sample for P1N {
        fn samples() -> Vec<(String, Self)> {
            vec![("-".to_owned(), Self { a: 11 })]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct P1S {
        #[serde(rename = "p1_a")]
        pub a: u32,
    }
    impl Sample for P1S {
        fn samples() -> Vec<(String, Self)> {
            vec![("-".to_owned(), Self { a: 11 })]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct TW1NN {
        #[serde(rename = "t1_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "t1_t")]
    pub enum T1NN {
        Dot,
        Rect {
            #[serde(rename = "t1_a")]
            a: u32,
        },
        Wrap(TW1NN),
    }
    impl Sample for T1NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 41 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(TW1NN { b: 46 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct TW1SS {
        #[serde(rename = "t1_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "t1_t")]
    pub enum T1SS {
        Dot,
        Rect {
            #[serde(rename = "t1_a")]
            a: u32,
        },
        Wrap(TW1SS),
    }
    impl Sample for T1SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 41 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(TW1SS { b: 46 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct VW1NN {
        #[serde(rename = "v1_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum V1NN {
        Rect {
            #[serde(rename = "v1_a")]
            a: u32,
        },
        Wrap(VW1NN),
    }
    impl Sample for V1NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 131 }),
                ("Wrap".to_owned(), Self::Wrap(VW1NN { b: 136 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct VW1SS {
        #[serde(rename = "v1_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum V1SS {
        Rect {
            #[serde(rename = "v1_a")]
            a: u32,
        },
        Wrap(VW1SS),
    }
    impl Sample for V1SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 131 }),
                ("Wrap".to_owned(), Self::Wrap(VW1SS { b: 136 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct AW2NN {
        #[serde(rename = "a2_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "a2_t", content = "a2_c")]
    pub enum A2NN {
        Dot,
        Rect {
            #[serde(rename = "a2_a")]
            a: u32,
        },
        Wrap(AW2NN),
    }
    impl Sample for A2NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 32 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(AW2NN { b: 37 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct AW2SS {
        #[serde(rename = "a2_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "a2_t", content = "a2_c")]
    pub enum A2SS {
        Dot,
        Rect {
            #[serde(rename = "a2_a")]
            a: u32,
        },
        Wrap(AW2SS),
    }
    impl Sample for A2SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 32 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(AW2SS { b: 37 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct EW2NN {
        #[serde(rename = "e2_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum E2NN {
        #[serde(rename = "e2_dot")]
        Dot,
        #[serde(rename = "e2_rect")]
        Rect {
            #[serde(rename = "e2_a")]
            a: u32,
        },
        #[serde(rename = "e2_wrap")]
        Wrap(EW2NN),
    }
    impl Sample for E2NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 22 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(EW2NN { b: 27 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct EW2SS {
        #[serde(rename = "e2_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum E2SS {
        #[serde(rename = "e2_dot")]
        Dot,
        #[serde(rename = "e2_rect")]
        Rect {
            #[serde(rename = "e2_a")]
            a: u32,
        },
        #[serde(rename = "e2_wrap")]
        Wrap(EW2SS),
    }
    impl Sample for E2SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 22 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(EW2SS { b: 27 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOY2NN {
        #[serde(rename = "fo2_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOX2NN {
        #[serde(rename = "fo2_a")]
        pub a: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FO2NN {
        #[serde(flatten)]
        pub x: FOX2NN,
        #[serde(flatten)]
        pub y: FOY2NN,
    }
    impl Sample for FO2NN {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    x: FOX2NN { a: 122 },
                    y: FOY2NN { b: 127 },
                },
            )]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOY2SS {
        #[serde(rename = "fo2_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOX2SS {
        #[serde(rename = "fo2_a")]
        pub a: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FO2SS {
        #[serde(flatten)]
        pub x: FOX2SS,
        #[serde(flatten)]
        pub y: FOY2SS,
    }
    impl Sample for FO2SS {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    x: FOX2SS { a: 122 },
                    y: FOY2SS { b: 127 },
                },
            )]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FPI2NN {
        #[serde(rename = "fp2_a")]
        pub a: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FP2NN {
        #[serde(flatten)]
        pub inner: FPI2NN,
        #[serde(rename = "fp2_own")]
        pub own: u32,
    }
    impl Sample for FP2NN {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    own: 62,
                    inner: FPI2NN { a: 67 },
                },
            )]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FPI2SS {
        #[serde(rename = "fp2_a")]
        pub a: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FP2SS {
        #[serde(flatten)]
        pub inner: FPI2SS,
        #[serde(rename = "fp2_own")]
        pub own: u32,
    }
    impl Sample for FP2SS {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    own: 62,
                    inner: FPI2SS { a: 67 },
                },
            )]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FTW2NNN {
        #[serde(rename = "ft2_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "ft2_t")]
    pub enum FTI2NNN {
        Dot,
        Rect {
            #[serde(rename = "ft2_a")]
            a: u32,
        },
        Wrap(FTW2NNN),
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FT2NNN {
        #[serde(flatten)]
        pub inner: FTI2NNN,
        #[serde(rename = "ft2_own")]
        pub own: u32,
    }
    impl Sample for FT2NNN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                (
                    "Rect".to_owned(),
                    Self {
                        own: 72,
                        inner: FTI2NNN::Rect { a: 74 },
                    },
                ),
                (
                    "Dot".to_owned(),
                    Self {
                        own: 72,
                        inner: FTI2NNN::Dot,
                    },
                ),
                (
                    "Wrap".to_owned(),
                    Self {
                        own: 72,
                        inner: FTI2NNN::Wrap(FTW2NNN { b: 77 }),
                    },
                ),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct OI2N {
        #[serde(rename = "o2_a")]
        pub a: u32,
    }
    impl Sample for OI2N {
        fn samples() -> Vec<(String, Self)> {
            vec![("Some".to_owned(), Self { a: 112 })]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct OI2S {
        #[serde(rename = "o2_a")]
        pub a: u32,
    }
    impl Sample for OI2S {
        fn samples() -> Vec<(String, Self)> {
            vec![("Some".to_owned(), Self { a: 112 })]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct P2N {
        #[serde(rename = "p2_a")]
        pub a: u32,
    }
    impl Sample for P2N {
        fn samples() -> Vec<(String, Self)> {
            vec![("-".to_owned(), Self { a: 12 })]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct P2S {
        #[serde(rename = "p2_a")]
        pub a: u32,
    }
    impl Sample for P2S {
        fn samples() -> Vec<(String, Self)> {
            vec![("-".to_owned(), Self { a: 12 })]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct TW2NN {
        #[serde(rename = "t2_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "t2_t")]
    pub enum T2NN {
        Dot,
        Rect {
            #[serde(rename = "t2_a")]
            a: u32,
        },
        Wrap(TW2NN),
    }
    impl Sample for T2NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 42 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(TW2NN { b: 47 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct TW2SS {
        #[serde(rename = "t2_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "t2_t")]
    pub enum T2SS {
        Dot,
        Rect {
            #[serde(rename = "t2_a")]
            a: u32,
        },
        Wrap(TW2SS),
    }
    impl Sample for T2SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 42 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(TW2SS { b: 47 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct VW2NN {
        #[serde(rename = "v2_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum V2NN {
        Rect {
            #[serde(rename = "v2_a")]
            a: u32,
        },
        Wrap(VW2NN),
    }
    impl Sample for V2NN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 132 }),
                ("Wrap".to_owned(), Self::Wrap(VW2NN { b: 137 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct VW2SS {
        #[serde(rename = "v2_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum V2SS {
        Rect {
            #[serde(rename = "v2_a")]
            a: u32,
        },
        Wrap(VW2SS),
    }
    impl Sample for V2SS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 132 }),
                ("Wrap".to_owned(), Self::Wrap(VW2SS { b: 137 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOY0NS {
        #[serde(rename = "fo0_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOX0NS {
        #[serde(rename = "fo0_a")]
        pub a: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FO0NS {
        #[serde(flatten)]
        pub x: FOX0NS,
        #[serde(flatten)]
        pub y: FOY0NS,
    }
    impl Sample for FO0NS {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    x: FOX0NS { a: 120 },
                    y: FOY0NS { b: 125 },
                },
            )]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FPI0NS {
        #[serde(rename = "fp0_a")]
        pub a: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FP0NS {
        #[serde(flatten)]
        pub inner: FPI0NS,
        #[serde(rename = "fp0_own")]
        pub own: u32,
    }
    impl Sample for FP0NS {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    own: 60,
                    inner: FPI0NS { a: 65 },
                },
            )]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FTW0NNS {
        #[serde(rename = "ft0_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "ft0_t")]
    pub enum FTI0NNS {
        Dot,
        Rect {
            #[serde(rename = "ft0_a")]
            a: u32,
        },
        Wrap(FTW0NNS),
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FT0NNS {
        #[serde(flatten)]
        pub inner: FTI0NNS,
        #[serde(rename = "ft0_own")]
        pub own: u32,
    }
    impl Sample for FT0NNS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                (
                    "Rect".to_owned(),
                    Self {
                        own: 70,
                        inner: FTI0NNS::Rect { a: 72 },
                    },
                ),
                (
                    "Dot".to_owned(),
                    Self {
                        own: 70,
                        inner: FTI0NNS::Dot,
                    },
                ),
                (
                    "Wrap".to_owned(),
                    Self {
                        own: 70,
                        inner: FTI0NNS::Wrap(FTW0NNS { b: 75 }),
                    },
                ),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct TW0NS {
        #[serde(rename = "t0_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "t0_t")]
    pub enum T0NS {
        Dot,
        Rect {
            #[serde(rename = "t0_a")]
            a: u32,
        },
        Wrap(TW0NS),
    }
    impl Sample for T0NS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 40 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(TW0NS { b: 45 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOY1NS {
        #[serde(rename = "fo1_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOX1NS {
        #[serde(rename = "fo1_a")]
        pub a: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FO1NS {
        #[serde(flatten)]
        pub x: FOX1NS,
        #[serde(flatten)]
        pub y: FOY1NS,
    }
    impl Sample for FO1NS {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    x: FOX1NS { a: 121 },
                    y: FOY1NS { b: 126 },
                },
            )]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOY1SN {
        #[serde(rename = "fo1_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FOX1SN {
        #[serde(rename = "fo1_a")]
        pub a: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FO1SN {
        #[serde(flatten)]
        pub x: FOX1SN,
        #[serde(flatten)]
        pub y: FOY1SN,
    }
    impl Sample for FO1SN {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    x: FOX1SN { a: 121 },
                    y: FOY1SN { b: 126 },
                },
            )]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FPI1SN {
        #[serde(rename = "fp1_a")]
        pub a: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FP1SN {
        #[serde(flatten)]
        pub inner: FPI1SN,
        #[serde(rename = "fp1_own")]
        pub own: u32,
    }
    impl Sample for FP1SN {
        fn samples() -> Vec<(String, Self)> {
            vec![(
                "-".to_owned(),
                Self {
                    own: 61,
                    inner: FPI1SN { a: 66 },
                },
            )]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FTW1NSS {
        #[serde(rename = "ft1_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "ft1_t")]
    pub enum FTI1NSS {
        Dot,
        Rect {
            #[serde(rename = "ft1_a")]
            a: u32,
        },
        Wrap(FTW1NSS),
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct FT1NSS {
        #[serde(flatten)]
        pub inner: FTI1NSS,
        #[serde(rename = "ft1_own")]
        pub own: u32,
    }
    impl Sample for FT1NSS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                (
                    "Rect".to_owned(),
                    Self {
                        own: 71,
                        inner: FTI1NSS::Rect { a: 73 },
                    },
                ),
                (
                    "Dot".to_owned(),
                    Self {
                        own: 71,
                        inner: FTI1NSS::Dot,
                    },
                ),
                (
                    "Wrap".to_owned(),
                    Self {
                        own: 71,
                        inner: FTI1NSS::Wrap(FTW1NSS { b: 76 }),
                    },
                ),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct TW1SN {
        #[serde(rename = "t1_b")]
        pub b: u32,
    }
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "t1_t")]
    pub enum T1SN {
        Dot,
        Rect {
            #[serde(rename = "t1_a")]
            a: u32,
        },
        Wrap(TW1SN),
    }
    impl Sample for T1SN {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 41 }),
                ("Dot".to_owned(), Self::Dot),
                ("Wrap".to_owned(), Self::Wrap(TW1SN { b: 46 })),
            ]
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct VW1NS {
        #[serde(rename = "v1_b")]
        pub b: u32,
    }
    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum V1NS {
        Rect {
            #[serde(rename = "v1_a")]
            a: u32,
        },
        Wrap(VW1NS),
    }
    impl Sample for V1NS {
        fn samples() -> Vec<(String, Self)> {
            vec![
                ("Rect".to_owned(), Self::Rect { a: 131 }),
                ("Wrap".to_owned(), Self::Wrap(VW1NS { b: 136 })),
            ]
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0A0NN {
        #[serde(flatten)]
        pub held: A0NN,
    }
    impl Sample for IN0A0NN {
        fn samples() -> Vec<(String, Self)> {
            <A0NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0A0SS {
        #[serde(flatten)]
        pub held: A0SS,
    }
    impl Sample for IN0A0SS {
        fn samples() -> Vec<(String, Self)> {
            <A0SS>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0E0NN {
        #[serde(flatten)]
        pub held: E0NN,
    }
    impl Sample for IN0E0NN {
        fn samples() -> Vec<(String, Self)> {
            <E0NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0E0SS {
        #[serde(flatten)]
        pub held: E0SS,
    }
    impl Sample for IN0E0SS {
        fn samples() -> Vec<(String, Self)> {
            <E0SS>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0FO0NN {
        #[serde(flatten)]
        pub held: FO0NN,
    }
    impl Sample for IN0FO0NN {
        fn samples() -> Vec<(String, Self)> {
            <FO0NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0FP0NN {
        #[serde(flatten)]
        pub held: FP0NN,
    }
    impl Sample for IN0FP0NN {
        fn samples() -> Vec<(String, Self)> {
            <FP0NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0FT0NNN {
        #[serde(flatten)]
        pub held: FT0NNN,
    }
    impl Sample for IN0FT0NNN {
        fn samples() -> Vec<(String, Self)> {
            <FT0NNN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0Map {
        #[serde(flatten)]
        pub held: BTreeMap<String, u32>,
    }
    impl Sample for IN0Map {
        fn samples() -> Vec<(String, Self)> {
            <BTreeMap<String, u32>>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0OptOI0N {
        #[serde(flatten)]
        pub held: Option<OI0N>,
    }
    impl Sample for IN0OptOI0N {
        fn samples() -> Vec<(String, Self)> {
            <Option<OI0N>>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0OptOI0S {
        #[serde(flatten)]
        pub held: Option<OI0S>,
    }
    impl Sample for IN0OptOI0S {
        fn samples() -> Vec<(String, Self)> {
            <Option<OI0S>>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0P0N {
        #[serde(flatten)]
        pub held: P0N,
    }
    impl Sample for IN0P0N {
        fn samples() -> Vec<(String, Self)> {
            <P0N>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0P0S {
        #[serde(flatten)]
        pub held: P0S,
    }
    impl Sample for IN0P0S {
        fn samples() -> Vec<(String, Self)> {
            <P0S>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0T0NN {
        #[serde(flatten)]
        pub held: T0NN,
    }
    impl Sample for IN0T0NN {
        fn samples() -> Vec<(String, Self)> {
            <T0NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0T0SS {
        #[serde(flatten)]
        pub held: T0SS,
    }
    impl Sample for IN0T0SS {
        fn samples() -> Vec<(String, Self)> {
            <T0SS>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0V0NN {
        #[serde(flatten)]
        pub held: V0NN,
    }
    impl Sample for IN0V0NN {
        fn samples() -> Vec<(String, Self)> {
            <V0NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0V0SS {
        #[serde(flatten)]
        pub held: V0SS,
    }
    impl Sample for IN0V0SS {
        fn samples() -> Vec<(String, Self)> {
            <V0SS>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1A0NN {
        #[serde(flatten)]
        pub held: A0NN,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1A0NN {
        fn samples() -> Vec<(String, Self)> {
            <A0NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1A0SS {
        #[serde(flatten)]
        pub held: A0SS,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1A0SS {
        fn samples() -> Vec<(String, Self)> {
            <A0SS>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1E0NN {
        #[serde(flatten)]
        pub held: E0NN,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1E0NN {
        fn samples() -> Vec<(String, Self)> {
            <E0NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1E0SS {
        #[serde(flatten)]
        pub held: E0SS,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1E0SS {
        fn samples() -> Vec<(String, Self)> {
            <E0SS>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1FO0NN {
        #[serde(flatten)]
        pub held: FO0NN,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1FO0NN {
        fn samples() -> Vec<(String, Self)> {
            <FO0NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1FO0SS {
        #[serde(flatten)]
        pub held: FO0SS,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1FO0SS {
        fn samples() -> Vec<(String, Self)> {
            <FO0SS>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1FP0NN {
        #[serde(flatten)]
        pub held: FP0NN,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1FP0NN {
        fn samples() -> Vec<(String, Self)> {
            <FP0NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1FP0SS {
        #[serde(flatten)]
        pub held: FP0SS,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1FP0SS {
        fn samples() -> Vec<(String, Self)> {
            <FP0SS>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1FT0NNN {
        #[serde(flatten)]
        pub held: FT0NNN,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1FT0NNN {
        fn samples() -> Vec<(String, Self)> {
            <FT0NNN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1OptOI0N {
        #[serde(flatten)]
        pub held: Option<OI0N>,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1OptOI0N {
        fn samples() -> Vec<(String, Self)> {
            <Option<OI0N>>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1OptOI0S {
        #[serde(flatten)]
        pub held: Option<OI0S>,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1OptOI0S {
        fn samples() -> Vec<(String, Self)> {
            <Option<OI0S>>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1P0N {
        #[serde(flatten)]
        pub held: P0N,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1P0N {
        fn samples() -> Vec<(String, Self)> {
            <P0N>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1P0S {
        #[serde(flatten)]
        pub held: P0S,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1P0S {
        fn samples() -> Vec<(String, Self)> {
            <P0S>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1T0NN {
        #[serde(flatten)]
        pub held: T0NN,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1T0NN {
        fn samples() -> Vec<(String, Self)> {
            <T0NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1V0NN {
        #[serde(flatten)]
        pub held: V0NN,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1V0NN {
        fn samples() -> Vec<(String, Self)> {
            <V0NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IS0OptOI0N {
        #[serde(flatten)]
        pub held: Option<OI0N>,
    }
    impl Sample for IS0OptOI0N {
        fn samples() -> Vec<(String, Self)> {
            <Option<OI0N>>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IS0P0N {
        #[serde(flatten)]
        pub held: P0N,
    }
    impl Sample for IS0P0N {
        fn samples() -> Vec<(String, Self)> {
            <P0N>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IS1OptOI0N {
        #[serde(flatten)]
        pub held: Option<OI0N>,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IS1OptOI0N {
        fn samples() -> Vec<(String, Self)> {
            <Option<OI0N>>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IS1P0N {
        #[serde(flatten)]
        pub held: P0N,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IS1P0N {
        fn samples() -> Vec<(String, Self)> {
            <P0N>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0A1NN {
        #[serde(flatten)]
        pub held: A1NN,
    }
    impl Sample for IN0A1NN {
        fn samples() -> Vec<(String, Self)> {
            <A1NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0E1NN {
        #[serde(flatten)]
        pub held: E1NN,
    }
    impl Sample for IN0E1NN {
        fn samples() -> Vec<(String, Self)> {
            <E1NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0FO1NN {
        #[serde(flatten)]
        pub held: FO1NN,
    }
    impl Sample for IN0FO1NN {
        fn samples() -> Vec<(String, Self)> {
            <FO1NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0FP1NN {
        #[serde(flatten)]
        pub held: FP1NN,
    }
    impl Sample for IN0FP1NN {
        fn samples() -> Vec<(String, Self)> {
            <FP1NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0FT1NNN {
        #[serde(flatten)]
        pub held: FT1NNN,
    }
    impl Sample for IN0FT1NNN {
        fn samples() -> Vec<(String, Self)> {
            <FT1NNN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0OptOI1S {
        #[serde(flatten)]
        pub held: Option<OI1S>,
    }
    impl Sample for IN0OptOI1S {
        fn samples() -> Vec<(String, Self)> {
            <Option<OI1S>>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0P1S {
        #[serde(flatten)]
        pub held: P1S,
    }
    impl Sample for IN0P1S {
        fn samples() -> Vec<(String, Self)> {
            <P1S>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0T1SS {
        #[serde(flatten)]
        pub held: T1SS,
    }
    impl Sample for IN0T1SS {
        fn samples() -> Vec<(String, Self)> {
            <T1SS>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0V1NN {
        #[serde(flatten)]
        pub held: V1NN,
    }
    impl Sample for IN0V1NN {
        fn samples() -> Vec<(String, Self)> {
            <V1NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN0V1SS {
        #[serde(flatten)]
        pub held: V1SS,
    }
    impl Sample for IN0V1SS {
        fn samples() -> Vec<(String, Self)> {
            <V1SS>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1A1SS {
        #[serde(flatten)]
        pub held: A1SS,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1A1SS {
        fn samples() -> Vec<(String, Self)> {
            <A1SS>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1E1SS {
        #[serde(flatten)]
        pub held: E1SS,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1E1SS {
        fn samples() -> Vec<(String, Self)> {
            <E1SS>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1FO1SS {
        #[serde(flatten)]
        pub held: FO1SS,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1FO1SS {
        fn samples() -> Vec<(String, Self)> {
            <FO1SS>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1FP1NN {
        #[serde(flatten)]
        pub held: FP1NN,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1FP1NN {
        fn samples() -> Vec<(String, Self)> {
            <FP1NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1FP1SS {
        #[serde(flatten)]
        pub held: FP1SS,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1FP1SS {
        fn samples() -> Vec<(String, Self)> {
            <FP1SS>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1OptOI1N {
        #[serde(flatten)]
        pub held: Option<OI1N>,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1OptOI1N {
        fn samples() -> Vec<(String, Self)> {
            <Option<OI1N>>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1P1N {
        #[serde(flatten)]
        pub held: P1N,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1P1N {
        fn samples() -> Vec<(String, Self)> {
            <P1N>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1T1NN {
        #[serde(flatten)]
        pub held: T1NN,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1T1NN {
        fn samples() -> Vec<(String, Self)> {
            <T1NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IN1V1NN {
        #[serde(flatten)]
        pub held: V1NN,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IN1V1NN {
        fn samples() -> Vec<(String, Self)> {
            <V1NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IS0A1NN {
        #[serde(flatten)]
        pub held: A1NN,
    }
    impl Sample for IS0A1NN {
        fn samples() -> Vec<(String, Self)> {
            <A1NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IS0E1NN {
        #[serde(flatten)]
        pub held: E1NN,
    }
    impl Sample for IS0E1NN {
        fn samples() -> Vec<(String, Self)> {
            <E1NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IS0OptOI1N {
        #[serde(flatten)]
        pub held: Option<OI1N>,
    }
    impl Sample for IS0OptOI1N {
        fn samples() -> Vec<(String, Self)> {
            <Option<OI1N>>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IS0P1N {
        #[serde(flatten)]
        pub held: P1N,
    }
    impl Sample for IS0P1N {
        fn samples() -> Vec<(String, Self)> {
            <P1N>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { held }))
                .collect()
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IS1A1NN {
        #[serde(flatten)]
        pub held: A1NN,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IS1A1NN {
        fn samples() -> Vec<(String, Self)> {
            <A1NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IS1E1NN {
        #[serde(flatten)]
        pub held: E1NN,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IS1E1NN {
        fn samples() -> Vec<(String, Self)> {
            <E1NN>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IS1OptOI1N {
        #[serde(flatten)]
        pub held: Option<OI1N>,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IS1OptOI1N {
        fn samples() -> Vec<(String, Self)> {
            <Option<OI1N>>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct IS1P1N {
        #[serde(flatten)]
        pub held: P1N,
        #[serde(rename = "i_own")]
        pub own: u32,
    }
    impl Sample for IS1P1N {
        fn samples() -> Vec<(String, Self)> {
            <P1N>::samples()
                .into_iter()
                .map(|(label, held)| (label, Self { own: 6, held }))
                .collect()
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5 {
        #[serde(flatten)]
        pub m0: CA10NN,
        #[serde(rename = "shared")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder13 {
        #[serde(flatten)]
        pub m0: CA10NN,
        #[serde(flatten)]
        pub m1: Option<COI11N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder21 {
        #[serde(flatten)]
        pub m0: CA10SS,
        #[serde(flatten)]
        pub m1: CFO11NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder29 {
        #[serde(flatten)]
        pub m0: CA10SS,
        #[serde(flatten)]
        pub m1: CV11NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder37 {
        #[serde(flatten)]
        pub m0: CE10SS,
        #[serde(flatten)]
        pub m1: CA11SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder45 {
        #[serde(flatten)]
        pub m0: CFP10NN,
        #[serde(flatten)]
        pub m1: CA11NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder53 {
        #[serde(flatten)]
        pub m0: CFT10NNN,
        #[serde(flatten)]
        pub m1: Option<COI11N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder61 {
        #[serde(flatten)]
        pub m0: Option<COI10N>,
        #[serde(flatten)]
        pub m1: CT11NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder69 {
        #[serde(flatten)]
        pub m0: CP10N,
        #[serde(flatten)]
        pub m1: CA11SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder77 {
        #[serde(flatten)]
        pub m0: CP10S,
        #[serde(flatten)]
        pub m1: CT11SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder85 {
        #[serde(flatten)]
        pub m0: CT10NN,
        #[serde(flatten)]
        pub m1: CP11S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder93 {
        #[serde(flatten)]
        pub m0: CA10NN,
        #[serde(flatten)]
        pub m1: CFO11NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder101 {
        #[serde(flatten)]
        pub m0: CA10NN,
        #[serde(flatten)]
        pub m1: CV11NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder109 {
        #[serde(flatten)]
        pub m0: CA10SS,
        #[serde(flatten)]
        pub m1: Option<COI11N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder117 {
        #[serde(flatten)]
        pub m0: CE10NN,
        #[serde(flatten)]
        pub m1: CFT11NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder125 {
        #[serde(flatten)]
        pub m0: CFO10NN,
        #[serde(flatten)]
        pub m1: CA11NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder133 {
        #[serde(flatten)]
        pub m0: CFT10NNN,
        #[serde(flatten)]
        pub m1: CE11NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder141 {
        #[serde(flatten)]
        pub m0: CFT10NNN,
        #[serde(flatten)]
        pub m1: CV11NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder149 {
        #[serde(flatten)]
        pub m0: Option<COI10S>,
        #[serde(flatten)]
        pub m1: CFT11NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder157 {
        #[serde(flatten)]
        pub m0: CP10S,
        #[serde(flatten)]
        pub m1: CA11NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder165 {
        #[serde(flatten)]
        pub m0: CT10NN,
        #[serde(flatten)]
        pub m1: CFP11NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder173 {
        #[serde(flatten)]
        pub m0: CV10NN,
        #[serde(flatten)]
        pub m1: CFT11NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder181 {
        #[serde(flatten)]
        pub m0: CA10SS,
        #[serde(flatten)]
        pub m1: CE11NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder189 {
        #[serde(flatten)]
        pub m0: CE10SS,
        #[serde(flatten)]
        pub m1: CA11NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder197 {
        #[serde(flatten)]
        pub m0: CP10S,
        #[serde(flatten)]
        pub m1: CA11NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder205 {
        #[serde(flatten)]
        pub m0: CA10SS,
        #[serde(flatten)]
        pub m1: CE11NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder213 {
        #[serde(flatten)]
        pub m0: CE10SS,
        #[serde(flatten)]
        pub m1: CA11NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder221 {
        #[serde(flatten)]
        pub m0: CP10S,
        #[serde(flatten)]
        pub m1: CA11NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder229 {
        #[serde(flatten)]
        pub m0: FP0NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder237 {
        #[serde(flatten)]
        pub m0: T0NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder245 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder253 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder261 {
        #[serde(flatten)]
        pub m0: E0SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder269 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder277 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: E1SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder285 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: P1N,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder293 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: E1NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder301 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder309 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder317 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder325 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder333 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: FT1NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder341 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: V1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder349 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder357 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: E1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder365 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: P1S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder373 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: FP1NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder381 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: A1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder389 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: FT1NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder397 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: V1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder405 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: FP1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder413 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: V1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder421 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: FP1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder429 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: T1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder437 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: FO1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder445 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: T1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder453 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder461 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: V1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder469 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder477 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder485 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder493 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder501 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder509 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder517 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder525 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: V1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder533 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: FP1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder541 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder549 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder557 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder565 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder573 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder581 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder589 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: FP1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder597 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder605 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder613 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: T1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder621 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: FO1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder629 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder637 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder645 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder653 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder661 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder669 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder677 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: A1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder685 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder693 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder701 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder709 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: A1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder717 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: A1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder725 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: A1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder733 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: A1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder741 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder749 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder757 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder765 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder773 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder781 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder789 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder797 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder805 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder813 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder821 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder829 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder837 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: V2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder845 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FP2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder853 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder861 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder869 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: T2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder877 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder885 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder893 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder901 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder909 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder917 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder925 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: T2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder933 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FO2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder941 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder949 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder957 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder965 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder973 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder981 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder989 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder997 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1005 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1013 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1021 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: V2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1029 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: FP2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1037 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1045 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1053 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: T2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1061 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FO2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1069 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1077 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1085 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1093 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1101 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1109 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1117 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FO2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1125 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1133 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1141 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1149 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1157 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1165 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1173 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1181 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1189 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1197 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1205 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1213 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: FP2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1221 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1229 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1237 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: T2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1245 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FO2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1253 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1261 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1269 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1277 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1285 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1293 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1301 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1309 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1317 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1325 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1333 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1341 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1349 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1357 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1365 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1373 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1381 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1389 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1397 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1405 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1413 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1421 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: T2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1429 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: FO2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1437 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1445 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1453 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1461 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1469 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1477 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1485 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1493 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1501 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1509 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1517 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1525 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1533 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1541 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1549 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1557 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1565 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1573 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1581 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: V2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1589 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1597 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1605 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1613 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1621 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1629 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1637 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1645 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1653 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1661 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1669 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1677 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1685 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1693 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1701 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1709 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1717 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1725 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1733 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1741 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1749 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1757 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1765 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1773 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1781 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1789 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1797 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1805 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1813 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1821 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1829 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1837 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1845 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1853 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1861 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1869 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1877 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1885 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1893 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1901 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1909 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1917 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1925 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1933 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1941 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1949 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1957 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1965 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1973 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1981 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1989 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder1997 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2005 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2013 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2021 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2029 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2037 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2045 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2053 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2061 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2069 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2077 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2085 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2093 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2101 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2109 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2117 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2125 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2133 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2141 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2149 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2157 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2165 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: V2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2173 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FP2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2181 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2189 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2197 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2205 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2213 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2221 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2229 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FP2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2237 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2245 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2253 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: T2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2261 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: FO2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2269 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2277 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2285 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2293 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2301 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2309 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2317 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2325 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2333 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2341 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2349 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: V2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2357 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FP2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2365 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2373 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2381 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: T2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2389 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2397 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2405 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2413 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2421 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2429 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2437 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: T2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2445 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FO2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2453 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2461 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2469 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2477 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2485 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2493 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2501 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2509 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2517 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2525 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2533 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: V2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2541 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: FP2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2549 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2557 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2565 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: T2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2573 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FO2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2581 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2589 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2597 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2605 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2613 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2621 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2629 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FO2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2637 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2645 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2653 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2661 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2669 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2677 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2685 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2693 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2701 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2709 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2717 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2725 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: FP2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2733 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2741 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2749 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: T2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2757 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FO2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2765 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2773 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2781 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2789 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2797 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2805 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2813 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2821 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2829 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2837 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2845 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2853 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2861 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2869 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2877 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2885 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2893 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2901 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2909 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2917 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2925 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2933 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2941 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2949 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2957 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2965 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2973 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2981 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2989 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder2997 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3005 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3013 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3021 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3029 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3037 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3045 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3053 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3061 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3069 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3077 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3085 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3093 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3101 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3109 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3117 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3125 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3133 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3141 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3149 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3157 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3165 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: V2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3173 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3181 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: E2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3189 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3197 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FP2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3205 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: A2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3213 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3221 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: E2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3229 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: T2NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3237 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3245 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: A2SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3253 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: P2N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3261 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3269 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3277 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3285 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3293 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3301 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3309 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3317 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3325 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3333 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3341 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3349 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3357 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3365 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3373 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3381 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3389 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3397 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: V2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3405 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: FP2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3413 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3421 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3429 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: T2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3437 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3445 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3453 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3461 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3469 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3477 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3485 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3493 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3501 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3509 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3517 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3525 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: V2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3533 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3541 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3549 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3557 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3565 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3573 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3581 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: V2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3589 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FP2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3597 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3605 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3613 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: T2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3621 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: FO2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3629 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3637 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3645 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3653 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3661 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3669 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3677 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3685 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3693 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3701 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3709 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: V2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3717 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FP2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3725 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3733 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3741 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3749 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3757 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3765 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3773 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FP2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3781 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3789 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3797 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: T2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3805 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: FO2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3813 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3821 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3829 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3837 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3845 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3853 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3861 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3869 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3877 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3885 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3893 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: V2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3901 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FP2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3909 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3917 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3925 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: T2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3933 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3941 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3949 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3957 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3965 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3973 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3981 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: T2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3989 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FO2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder3997 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4005 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4013 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4021 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4029 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4037 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4045 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4053 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4061 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4069 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4077 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4085 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4093 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4101 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4109 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4117 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4125 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4133 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4141 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4149 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4157 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4165 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4173 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4181 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4189 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4197 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4205 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4213 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4221 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4229 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4237 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4245 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4253 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4261 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4269 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4277 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4285 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4293 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4301 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4309 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4317 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4325 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4333 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4341 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4349 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4357 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4365 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4373 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4381 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4389 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4397 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4405 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4413 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4421 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4429 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4437 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4445 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4453 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4461 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4469 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4477 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4485 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4493 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4501 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4509 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4517 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4525 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4533 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4541 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4549 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4557 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4565 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4573 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: FO2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4581 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4589 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4597 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4605 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4613 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4621 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4629 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4637 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4645 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4653 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4661 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4669 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4677 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4685 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4693 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4701 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4709 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4717 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4725 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: V2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4733 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: FP2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4741 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4749 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4757 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4765 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4773 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4781 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4789 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4797 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4805 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4813 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4821 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4829 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4837 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4845 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4853 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4861 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4869 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4877 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4885 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4893 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4901 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4909 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: V2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4917 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: FP2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4925 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4933 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4941 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: T2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4949 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4957 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4965 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4973 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4981 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4989 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder4997 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5005 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5013 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5021 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5029 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5037 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: V2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5045 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5053 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5061 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5069 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5077 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5085 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5093 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: V2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5101 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FP2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5109 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5117 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5125 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: T2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5133 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: FO2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5141 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5149 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5157 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5165 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5173 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5181 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5189 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5197 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5205 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5213 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5221 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: V2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5229 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: FP2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5237 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5245 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5253 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5261 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5269 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5277 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5285 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FP2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5293 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5301 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5309 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: T2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5317 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: FO2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5325 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5333 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema(nonstrict)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5341 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5349 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5357 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5365 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5373 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5381 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5389 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5397 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5405 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5413 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5421 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5429 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5437 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5445 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5453 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5461 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5469 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5477 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5485 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5493 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5501 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5509 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5517 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5525 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5533 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5541 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5549 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5557 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5565 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5573 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5581 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5589 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5597 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5605 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5613 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(flatten)]
        pub m2: Option<OI2N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5621 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: E2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5629 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5637 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: FP2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5645 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: A2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5653 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: Option<OI2S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5661 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: E2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5669 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: T2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5677 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: FT2NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5685 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: A2SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5693 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(flatten)]
        pub m2: P2N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5701 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: FO2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5709 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(flatten)]
        pub m2: V2NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5717 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5725 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5733 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5741 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5749 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5757 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5765 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5773 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5781 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5789 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5797 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5805 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5813 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5821 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5829 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5837 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5845 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5853 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5861 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5869 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5877 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5885 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5893 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5901 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5909 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5917 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5925 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5933 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5941 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5949 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5957 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5965 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5973 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5981 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5989 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder5997 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6005 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6013 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6021 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6029 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6037 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6045 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6053 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6061 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6069 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6077 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6085 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6093 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6101 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6109 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6117 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6125 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6133 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6141 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6149 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6157 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6165 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6173 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6181 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6189 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6197 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6205 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6213 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6221 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6229 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6237 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6245 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6253 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6261 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6269 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6277 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6285 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6293 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6301 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6309 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6317 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6325 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6333 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6341 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6349 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6357 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6365 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6373 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6381 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6389 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6397 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6405 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6413 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6421 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6429 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6437 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6445 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6453 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6461 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6469 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6477 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6485 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6493 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6501 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6509 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6517 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6525 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6533 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6541 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6549 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6557 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6565 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6573 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6581 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6589 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6597 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6605 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6613 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6621 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6629 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6637 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6645 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6653 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6661 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6669 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6677 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6685 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6693 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6701 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6709 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6717 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6725 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6733 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(flatten)]
        pub m2: P2S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6741 {
        #[serde(flatten)]
        pub m0: T0NS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6749 {
        #[serde(flatten)]
        pub m0: FT0NNS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6757 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: FO1SN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6765 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: V1NS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6773 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: FT1NSS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6781 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: FP1SN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6789 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: FO1NS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6797 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: T1SN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6805 {
        #[serde(flatten)]
        pub m0: FO0NS,
        #[serde(flatten)]
        pub m1: E1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6813 {
        #[serde(flatten)]
        pub m0: FO0NS,
        #[serde(flatten)]
        pub m1: P1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6821 {
        #[serde(flatten)]
        pub m0: FP0NS,
        #[serde(flatten)]
        pub m1: E1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6829 {
        #[serde(flatten)]
        pub m0: FP0NS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6837 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: FO1SN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6845 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: V1NS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6853 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: FT1NSS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6861 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: FP1SN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6869 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: FO1NS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6877 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: T1SN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6885 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: FO1SN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6893 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: V1NS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6901 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: FT1NSS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6909 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: FP1SN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6917 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: FO1NS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6925 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: T1SN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6933 {
        #[serde(flatten)]
        pub m0: FO0NS,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6941 {
        #[serde(flatten)]
        pub m0: FO0NS,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6949 {
        #[serde(flatten)]
        pub m0: FP0NS,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6957 {
        #[serde(flatten)]
        pub m0: FP0NS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6965 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: FO1SN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6973 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: V1NS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6981 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: FT1NSS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6989 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: FP1SN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder6997 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: FO1NS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7005 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: T1SN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7013 {
        #[serde(flatten)]
        pub m0: IN0A0SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7021 {
        #[serde(flatten)]
        pub m0: IN0Map,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7029 {
        #[serde(flatten)]
        pub m0: IN0V0SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7037 {
        #[serde(flatten)]
        pub m0: IN1FP0SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7045 {
        #[serde(flatten)]
        pub m0: IN1V0NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7053 {
        #[serde(flatten)]
        pub m0: IS0P0N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7061 {
        #[serde(flatten)]
        pub m0: IS1P0N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7069 {
        #[serde(flatten)]
        pub m0: IN0FP0NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7077 {
        #[serde(flatten)]
        pub m0: IN0T0SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7085 {
        #[serde(flatten)]
        pub m0: IN1FO0SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7093 {
        #[serde(flatten)]
        pub m0: IN1T0NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7101 {
        #[serde(flatten)]
        pub m0: IS0OptOI0N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7109 {
        #[serde(flatten)]
        pub m0: IS1OptOI0N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7117 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: IN0FO1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7125 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: IN0P1S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7133 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: IN1E1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7141 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: IN1P1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7149 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: IS0E1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7157 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: IS1E1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7165 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: IN0E1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7173 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: IN0OptOI1S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7181 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: IN1A1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7189 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: IN1OptOI1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7197 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: IS0A1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7205 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: IS1A1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7213 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: IN0A1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7221 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: IN0FT1NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7229 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: IN0V1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7237 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: IN1FP1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7245 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: IN1V1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7253 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: IS0P1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7261 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: IS1P1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7269 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: IN0FP1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7277 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: IN0T1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7285 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: IN1FO1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7293 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: IN1T1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7301 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: IS0OptOI1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7309 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: IS1OptOI1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7317 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: IN0FO1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7325 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: IN0V1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7333 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: IN1OptOI1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7341 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: IN0E1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7349 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: IN0P1S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7357 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: IN1FP1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7365 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: IN0A1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7373 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: IN0OptOI1S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7381 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: IN1E1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7389 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: IN1T1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7397 {
        #[serde(flatten)]
        pub m0: IN0A0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7405 {
        #[serde(flatten)]
        pub m0: IN0A0SS,
        #[serde(flatten)]
        pub m1: A1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7413 {
        #[serde(flatten)]
        pub m0: IN0A0SS,
        #[serde(flatten)]
        pub m1: P1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7421 {
        #[serde(flatten)]
        pub m0: IN0E0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7429 {
        #[serde(flatten)]
        pub m0: IN0E0NN,
        #[serde(flatten)]
        pub m1: V1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7437 {
        #[serde(flatten)]
        pub m0: IN0E0SS,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7445 {
        #[serde(flatten)]
        pub m0: IN0FO0NN,
        #[serde(flatten)]
        pub m1: E1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7453 {
        #[serde(flatten)]
        pub m0: IN0FO0NN,
        #[serde(flatten)]
        pub m1: P1S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7461 {
        #[serde(flatten)]
        pub m0: IN0FP0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7469 {
        #[serde(flatten)]
        pub m0: IN0FT0NNN,
        #[serde(flatten)]
        pub m1: A1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7477 {
        #[serde(flatten)]
        pub m0: IN0FT0NNN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7485 {
        #[serde(flatten)]
        pub m0: IN0OptOI0N,
        #[serde(flatten)]
        pub m1: E1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7493 {
        #[serde(flatten)]
        pub m0: IN0OptOI0N,
        #[serde(flatten)]
        pub m1: T1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7501 {
        #[serde(flatten)]
        pub m0: IN0OptOI0S,
        #[serde(flatten)]
        pub m1: FT1NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7509 {
        #[serde(flatten)]
        pub m0: IN0P0N,
        #[serde(flatten)]
        pub m1: A1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7517 {
        #[serde(flatten)]
        pub m0: IN0P0N,
        #[serde(flatten)]
        pub m1: P1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7525 {
        #[serde(flatten)]
        pub m0: IN0P0S,
        #[serde(flatten)]
        pub m1: FO1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7533 {
        #[serde(flatten)]
        pub m0: IN0P0S,
        #[serde(flatten)]
        pub m1: V1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7541 {
        #[serde(flatten)]
        pub m0: IN0T0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7549 {
        #[serde(flatten)]
        pub m0: IN0V0NN,
        #[serde(flatten)]
        pub m1: E1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7557 {
        #[serde(flatten)]
        pub m0: IN0V0NN,
        #[serde(flatten)]
        pub m1: P1S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7565 {
        #[serde(flatten)]
        pub m0: IN1A0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7573 {
        #[serde(flatten)]
        pub m0: IN1A0SS,
        #[serde(flatten)]
        pub m1: A1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7581 {
        #[serde(flatten)]
        pub m0: IN1A0SS,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7589 {
        #[serde(flatten)]
        pub m0: IN1E0NN,
        #[serde(flatten)]
        pub m1: E1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7597 {
        #[serde(flatten)]
        pub m0: IN1E0NN,
        #[serde(flatten)]
        pub m1: T1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7605 {
        #[serde(flatten)]
        pub m0: IN1E0SS,
        #[serde(flatten)]
        pub m1: FT1NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7613 {
        #[serde(flatten)]
        pub m0: IN1FO0NN,
        #[serde(flatten)]
        pub m1: A1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7621 {
        #[serde(flatten)]
        pub m0: IN1FO0NN,
        #[serde(flatten)]
        pub m1: P1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7629 {
        #[serde(flatten)]
        pub m0: IN1FP0NN,
        #[serde(flatten)]
        pub m1: FO1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7637 {
        #[serde(flatten)]
        pub m0: IN1FP0NN,
        #[serde(flatten)]
        pub m1: V1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7645 {
        #[serde(flatten)]
        pub m0: IN1FT0NNN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7653 {
        #[serde(flatten)]
        pub m0: IN1OptOI0N,
        #[serde(flatten)]
        pub m1: E1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7661 {
        #[serde(flatten)]
        pub m0: IN1OptOI0N,
        #[serde(flatten)]
        pub m1: P1S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7669 {
        #[serde(flatten)]
        pub m0: IN1OptOI0S,
        #[serde(flatten)]
        pub m1: FP1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7677 {
        #[serde(flatten)]
        pub m0: IN1P0N,
        #[serde(flatten)]
        pub m1: A1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7685 {
        #[serde(flatten)]
        pub m0: IN1P0N,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7693 {
        #[serde(flatten)]
        pub m0: IN1P0S,
        #[serde(flatten)]
        pub m1: E1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7701 {
        #[serde(flatten)]
        pub m0: IN1P0S,
        #[serde(flatten)]
        pub m1: T1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7709 {
        #[serde(flatten)]
        pub m0: IN1T0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7717 {
        #[serde(flatten)]
        pub m0: IN1V0NN,
        #[serde(flatten)]
        pub m1: A1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7725 {
        #[serde(flatten)]
        pub m0: IN1V0NN,
        #[serde(flatten)]
        pub m1: P1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7733 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: IN0FO1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7741 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: IN0P1S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7749 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: IN1E1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7757 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: IN1P1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7765 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: IS0E1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7773 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: IS1E1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7781 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: IN0E1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7789 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: IN0OptOI1S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7797 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: IN1A1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7805 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: IN1OptOI1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7813 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: IS0A1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7821 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: IS1A1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7829 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: IN0A1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7837 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: IN0FT1NNN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7845 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: IN0V1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7853 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: IN1FP1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7861 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: IN1V1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7869 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: IS0P1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7877 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: IS1P1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7885 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: IN0FP1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7893 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: IN0T1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7901 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: IN1FO1SS,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7909 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: IN1T1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7917 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: IS0OptOI1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7925 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: IS1OptOI1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7933 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: IN0FO1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7941 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: IN0V1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7949 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: IN1OptOI1N,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7957 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: IN0E1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7965 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: IN0P1S,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7973 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: IN1FP1NN,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7981 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: IN0A1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7989 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: IN0FT1NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder7997 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: IN0V1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8005 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: IN1FP1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8013 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: IN1V1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8021 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: IS0P1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8029 {
        #[serde(flatten)]
        pub m0: A0NN,
        #[serde(flatten)]
        pub m1: IS1P1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8037 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: IN0FP1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8045 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: IN0T1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8053 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: IN1FO1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8061 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: IN1T1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8069 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: IS0OptOI1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8077 {
        #[serde(flatten)]
        pub m0: A0SS,
        #[serde(flatten)]
        pub m1: IS1OptOI1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8085 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: IN0FO1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8093 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: IN0P1S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8101 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: IN1E1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8109 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: IN1P1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8117 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: IS0E1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8125 {
        #[serde(flatten)]
        pub m0: E0NN,
        #[serde(flatten)]
        pub m1: IS1E1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8133 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: IN0E1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8141 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: IN0OptOI1S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8149 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: IN1A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8157 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: IN1OptOI1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8165 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: IS0A1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8173 {
        #[serde(flatten)]
        pub m0: E0SS,
        #[serde(flatten)]
        pub m1: IS1A1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8181 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: IN0A1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8189 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: IN0OptOI1S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8197 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: IN1E1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8205 {
        #[serde(flatten)]
        pub m0: FO0NN,
        #[serde(flatten)]
        pub m1: IN1T1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8213 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: IN0FT1NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8221 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: IN1A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8229 {
        #[serde(flatten)]
        pub m0: FP0NN,
        #[serde(flatten)]
        pub m1: IN1P1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8237 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: IN0FO1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8245 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: IN0V1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8253 {
        #[serde(flatten)]
        pub m0: FT0NNN,
        #[serde(flatten)]
        pub m1: IN1OptOI1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8261 {
        #[serde(flatten)]
        pub m0: IN0A0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8269 {
        #[serde(flatten)]
        pub m0: IN0A0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8277 {
        #[serde(flatten)]
        pub m0: IN0A0SS,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8285 {
        #[serde(flatten)]
        pub m0: IN0E0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8293 {
        #[serde(flatten)]
        pub m0: IN0E0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8301 {
        #[serde(flatten)]
        pub m0: IN0E0SS,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8309 {
        #[serde(flatten)]
        pub m0: IN0E0SS,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8317 {
        #[serde(flatten)]
        pub m0: IN0FO0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8325 {
        #[serde(flatten)]
        pub m0: IN0FP0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8333 {
        #[serde(flatten)]
        pub m0: IN0FP0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8341 {
        #[serde(flatten)]
        pub m0: IN0FT0NNN,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8349 {
        #[serde(flatten)]
        pub m0: IN0FT0NNN,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8357 {
        #[serde(flatten)]
        pub m0: IN0OptOI0N,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8365 {
        #[serde(flatten)]
        pub m0: IN0OptOI0S,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8373 {
        #[serde(flatten)]
        pub m0: IN0OptOI0S,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8381 {
        #[serde(flatten)]
        pub m0: IN0P0N,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8389 {
        #[serde(flatten)]
        pub m0: IN0P0S,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8397 {
        #[serde(flatten)]
        pub m0: IN0P0S,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8405 {
        #[serde(flatten)]
        pub m0: IN0T0NN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8413 {
        #[serde(flatten)]
        pub m0: IN0T0NN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8421 {
        #[serde(flatten)]
        pub m0: IN0V0NN,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8429 {
        #[serde(flatten)]
        pub m0: IN1A0NN,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8437 {
        #[serde(flatten)]
        pub m0: IN1A0NN,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8445 {
        #[serde(flatten)]
        pub m0: IN1A0SS,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8453 {
        #[serde(flatten)]
        pub m0: IN1A0SS,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8461 {
        #[serde(flatten)]
        pub m0: IN1E0NN,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8469 {
        #[serde(flatten)]
        pub m0: IN1E0SS,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8477 {
        #[serde(flatten)]
        pub m0: IN1E0SS,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8485 {
        #[serde(flatten)]
        pub m0: IN1FO0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8493 {
        #[serde(flatten)]
        pub m0: IN1FP0NN,
        #[serde(flatten)]
        pub m1: A1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8501 {
        #[serde(flatten)]
        pub m0: IN1FP0NN,
        #[serde(flatten)]
        pub m1: Option<OI1S>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8509 {
        #[serde(flatten)]
        pub m0: IN1FT0NNN,
        #[serde(flatten)]
        pub m1: E1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8517 {
        #[serde(flatten)]
        pub m0: IN1FT0NNN,
        #[serde(flatten)]
        pub m1: T1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8525 {
        #[serde(flatten)]
        pub m0: IN1OptOI0N,
        #[serde(flatten)]
        pub m1: FT1NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8533 {
        #[serde(flatten)]
        pub m0: IN1OptOI0S,
        #[serde(flatten)]
        pub m1: A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8541 {
        #[serde(flatten)]
        pub m0: IN1OptOI0S,
        #[serde(flatten)]
        pub m1: P1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8549 {
        #[serde(flatten)]
        pub m0: IN1P0N,
        #[serde(flatten)]
        pub m1: FO1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8557 {
        #[serde(flatten)]
        pub m0: IN1P0N,
        #[serde(flatten)]
        pub m1: V1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8565 {
        #[serde(flatten)]
        pub m0: IN1P0S,
        #[serde(flatten)]
        pub m1: Option<OI1N>,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8573 {
        #[serde(flatten)]
        pub m0: IN1T0NN,
        #[serde(flatten)]
        pub m1: E1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8581 {
        #[serde(flatten)]
        pub m0: IN1T0NN,
        #[serde(flatten)]
        pub m1: P1S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8589 {
        #[serde(flatten)]
        pub m0: IN1V0NN,
        #[serde(flatten)]
        pub m1: FP1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8597 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: IN0A1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8605 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: IN0FT1NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8613 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: IN0V1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8621 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: IN1FP1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8629 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: IN1V1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8637 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: IS0P1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8645 {
        #[serde(flatten)]
        pub m0: Option<OI0N>,
        #[serde(flatten)]
        pub m1: IS1P1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8653 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: IN0FP1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8661 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: IN0T1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8669 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: IN1FO1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8677 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: IN1T1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8685 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: IS0OptOI1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8693 {
        #[serde(flatten)]
        pub m0: Option<OI0S>,
        #[serde(flatten)]
        pub m1: IS1OptOI1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8701 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: IN0FO1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8709 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: IN0P1S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8717 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: IN1E1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8725 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: IN1P1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8733 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: IS0E1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8741 {
        #[serde(flatten)]
        pub m0: P0N,
        #[serde(flatten)]
        pub m1: IS1E1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8749 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: IN0E1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8757 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: IN0OptOI1S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8765 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: IN1A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8773 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: IN1OptOI1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8781 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: IS0A1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8789 {
        #[serde(flatten)]
        pub m0: P0S,
        #[serde(flatten)]
        pub m1: IS1A1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8797 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: IN0A1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8805 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: IN0OptOI1S,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8813 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: IN1E1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8821 {
        #[serde(flatten)]
        pub m0: T0NN,
        #[serde(flatten)]
        pub m1: IN1T1NN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8829 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: IN0FT1NNN,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8837 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: IN1A1SS,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Holder8845 {
        #[serde(flatten)]
        pub m0: V0NN,
        #[serde(flatten)]
        pub m1: IN1P1N,
        #[serde(rename = "h_own")]
        pub own: u32,
    }

    #[test]
    fn each_holder_reads_back_and_meets_the_stray_key_as_measured() {
        let failed: Vec<String> = HOLDERS.iter().flat_map(|holder| holder()).collect();
        assert_eq!(failed, Vec::<String>::new());
    }

    fn holder5() -> Vec<String> {
        check(
            "collide1:C1SX:CA10_NN",
            &rows1(|m0: CA10NN| Holder5 { own: 7, m0 }),
            &[("Dot", 'R'), ("Rect", 'R'), ("Wrap", 'R')],
        )
    }

    fn holder13() -> Vec<String> {
        check(
            "collide2:C2N0:CA10_NN|CO11_N nonstrict",
            &rows2(|m0: CA10NN, m1: Option<COI11N>| Holder13 { m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder21() -> Vec<String> {
        check(
            "collide2:C2N0:CA10_SS|CFO11_NN",
            &rows2(|m0: CA10SS, m1: CFO11NN| Holder21 { m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder29() -> Vec<String> {
        check(
            "collide2:C2N0:CA10_SS|CV11_NN",
            &rows2(|m0: CA10SS, m1: CV11NN| Holder29 { m0, m1 }),
            &[
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder37() -> Vec<String> {
        check(
            "collide2:C2N0:CE10_SS|CA11_SS nonstrict",
            &rows2(|m0: CE10SS, m1: CA11SS| Holder37 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder45() -> Vec<String> {
        check(
            "collide2:C2N0:CFP10_NN|CA11_NN",
            &rows2(|m0: CFP10NN, m1: CA11NN| Holder45 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder53() -> Vec<String> {
        check(
            "collide2:C2N0:CFT10_NNN|CO11_N",
            &rows2(|m0: CFT10NNN, m1: Option<COI11N>| Holder53 { m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder61() -> Vec<String> {
        check(
            "collide2:C2N0:CO10_N|CT11_NN",
            &rows2(|m0: Option<COI10N>, m1: CT11NN| Holder61 { m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder69() -> Vec<String> {
        check(
            "collide2:C2N0:CP10_N|CA11_SS nonstrict",
            &rows2(|m0: CP10N, m1: CA11SS| Holder69 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder77() -> Vec<String> {
        check(
            "collide2:C2N0:CP10_S|CT11_SS",
            &rows2(|m0: CP10S, m1: CT11SS| Holder77 { m0, m1 }),
            &[("-,Dot", 'R'), ("-,Rect", 'R'), ("-,Wrap", 'R')],
        )
    }

    fn holder85() -> Vec<String> {
        check(
            "collide2:C2N0:CT10_NN|CP11_S",
            &rows2(|m0: CT10NN, m1: CP11S| Holder85 { m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder93() -> Vec<String> {
        check(
            "collide2:C2N1:CA10_NN|CFO11_NN",
            &rows2(|m0: CA10NN, m1: CFO11NN| Holder93 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder101() -> Vec<String> {
        check(
            "collide2:C2N1:CA10_NN|CV11_NN",
            &rows2(|m0: CA10NN, m1: CV11NN| Holder101 { own: 7, m0, m1 }),
            &[
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder109() -> Vec<String> {
        check(
            "collide2:C2N1:CA10_SS|CO11_N nonstrict",
            &rows2(|m0: CA10SS, m1: Option<COI11N>| Holder109 { own: 7, m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder117() -> Vec<String> {
        check(
            "collide2:C2N1:CE10_NN|CFT11_NNN",
            &rows2(|m0: CE10NN, m1: CFT11NNN| Holder117 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder125() -> Vec<String> {
        check(
            "collide2:C2N1:CFO10_NN|CA11_NN",
            &rows2(|m0: CFO10NN, m1: CA11NN| Holder125 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder133() -> Vec<String> {
        check(
            "collide2:C2N1:CFT10_NNN|CE11_NN",
            &rows2(|m0: CFT10NNN, m1: CE11NN| Holder133 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder141() -> Vec<String> {
        check(
            "collide2:C2N1:CFT10_NNN|CV11_NN",
            &rows2(|m0: CFT10NNN, m1: CV11NN| Holder141 { own: 7, m0, m1 }),
            &[
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder149() -> Vec<String> {
        check(
            "collide2:C2N1:CO10_S|CFT11_NNN",
            &rows2(|m0: Option<COI10S>, m1: CFT11NNN| Holder149 { own: 7, m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder157() -> Vec<String> {
        check(
            "collide2:C2N1:CP10_S|CA11_NN nonstrict",
            &rows2(|m0: CP10S, m1: CA11NN| Holder157 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder165() -> Vec<String> {
        check(
            "collide2:C2N1:CT10_NN|CFP11_NN",
            &rows2(|m0: CT10NN, m1: CFP11NN| Holder165 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder173() -> Vec<String> {
        check(
            "collide2:C2N1:CV10_NN|CFT11_NNN",
            &rows2(|m0: CV10NN, m1: CFT11NNN| Holder173 { own: 7, m0, m1 }),
            &[
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder181() -> Vec<String> {
        check(
            "collide2:C2S0:CA10_SS|CE11_NN",
            &rows2(|m0: CA10SS, m1: CE11NN| Holder181 { m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder189() -> Vec<String> {
        check(
            "collide2:C2S0:CE10_SS|CA11_NN",
            &rows2(|m0: CE10SS, m1: CA11NN| Holder189 { m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder197() -> Vec<String> {
        check(
            "collide2:C2S0:CP10_S|CA11_NN",
            &rows2(|m0: CP10S, m1: CA11NN| Holder197 { m0, m1 }),
            &[("-,Dot", 'R'), ("-,Rect", 'R'), ("-,Wrap", 'R')],
        )
    }

    fn holder205() -> Vec<String> {
        check(
            "collide2:C2S1:CA10_SS|CE11_NN",
            &rows2(|m0: CA10SS, m1: CE11NN| Holder205 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder213() -> Vec<String> {
        check(
            "collide2:C2S1:CE10_SS|CA11_NN",
            &rows2(|m0: CE10SS, m1: CA11NN| Holder213 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder221() -> Vec<String> {
        check(
            "collide2:C2S1:CP10_S|CA11_NN",
            &rows2(|m0: CP10S, m1: CA11NN| Holder221 { own: 7, m0, m1 }),
            &[("-,Dot", 'R'), ("-,Rect", 'R'), ("-,Wrap", 'R')],
        )
    }

    fn holder229() -> Vec<String> {
        check(
            "main1:H1N0:FP0_NN",
            &rows1(|m0: FP0NN| Holder229 { m0 }),
            &[("-", 'E')],
        )
    }

    fn holder237() -> Vec<String> {
        check(
            "main1:H1N0:T0_NN",
            &rows1(|m0: T0NN| Holder237 { m0 }),
            &[("Dot", 'E'), ("Rect", 'E'), ("Wrap", 'E')],
        )
    }

    fn holder245() -> Vec<String> {
        check(
            "main1:H1N1:FO0_NN",
            &rows1(|m0: FO0NN| Holder245 { own: 7, m0 }),
            &[("-", 'E')],
        )
    }

    fn holder253() -> Vec<String> {
        check(
            "main1:H1N1:P0_S nonstrict",
            &rows1(|m0: P0S| Holder253 { own: 7, m0 }),
            &[("-", 'E')],
        )
    }

    fn holder261() -> Vec<String> {
        check(
            "main1:H1S0:E0_SS",
            &rows1(|m0: E0SS| Holder261 { m0 }),
            &[("Dot", 'R'), ("Rect", 'R'), ("Wrap", 'R')],
        )
    }

    fn holder269() -> Vec<String> {
        check(
            "main1:H1S1:E0_SS",
            &rows1(|m0: E0SS| Holder269 { own: 7, m0 }),
            &[("Dot", 'R'), ("Rect", 'R'), ("Wrap", 'R')],
        )
    }

    fn holder277() -> Vec<String> {
        check(
            "main2:H2N0:A0_NN|E1_SS nonstrict",
            &rows2(|m0: A0NN, m1: E1SS| Holder277 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder285() -> Vec<String> {
        check(
            "main2:H2N0:A0_NN|P1_N nonstrict",
            &rows2(|m0: A0NN, m1: P1N| Holder285 { m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder293() -> Vec<String> {
        check(
            "main2:H2N0:A0_SS|E1_NN nonstrict",
            &rows2(|m0: A0SS, m1: E1NN| Holder293 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder301() -> Vec<String> {
        check(
            "main2:H2N0:A0_SS|O1_S nonstrict",
            &rows2(|m0: A0SS, m1: Option<OI1S>| Holder301 { m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder309() -> Vec<String> {
        check(
            "main2:H2N0:E0_NN|A1_SS nonstrict",
            &rows2(|m0: E0NN, m1: A1SS| Holder309 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder317() -> Vec<String> {
        check(
            "main2:H2N0:E0_NN|O1_N nonstrict",
            &rows2(|m0: E0NN, m1: Option<OI1N>| Holder317 { m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder325() -> Vec<String> {
        check(
            "main2:H2N0:E0_SS|A1_NN nonstrict",
            &rows2(|m0: E0SS, m1: A1NN| Holder325 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder333() -> Vec<String> {
        check(
            "main2:H2N0:E0_SS|FT1_NNN",
            &rows2(|m0: E0SS, m1: FT1NNN| Holder333 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder341() -> Vec<String> {
        check(
            "main2:H2N0:E0_SS|V1_SS",
            &rows2(|m0: E0SS, m1: V1SS| Holder341 { m0, m1 }),
            &[
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder349() -> Vec<String> {
        check(
            "main2:H2N0:FO0_NN|O1_N",
            &rows2(|m0: FO0NN, m1: Option<OI1N>| Holder349 { m0, m1 }),
            &[("-,None", 'E'), ("-,Some", 'E')],
        )
    }

    fn holder357() -> Vec<String> {
        check(
            "main2:H2N0:FP0_NN|E1_NN",
            &rows2(|m0: FP0NN, m1: E1NN| Holder357 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder365() -> Vec<String> {
        check(
            "main2:H2N0:FP0_NN|P1_S",
            &rows2(|m0: FP0NN, m1: P1S| Holder365 { m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder373() -> Vec<String> {
        check(
            "main2:H2N0:FT0_NNN|FP1_NN",
            &rows2(|m0: FT0NNN, m1: FP1NN| Holder373 { m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder381() -> Vec<String> {
        check(
            "main2:H2N0:O0_N|A1_NN nonstrict",
            &rows2(|m0: Option<OI0N>, m1: A1NN| Holder381 { m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder389() -> Vec<String> {
        check(
            "main2:H2N0:O0_N|FT1_NNN",
            &rows2(|m0: Option<OI0N>, m1: FT1NNN| Holder389 { m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder397() -> Vec<String> {
        check(
            "main2:H2N0:O0_N|V1_SS",
            &rows2(|m0: Option<OI0N>, m1: V1SS| Holder397 { m0, m1 }),
            &[
                ("None,Rect", 'R'),
                ("None,Wrap", 'R'),
                ("Some,Rect", 'R'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder405() -> Vec<String> {
        check(
            "main2:H2N0:O0_S|FP1_SS",
            &rows2(|m0: Option<OI0S>, m1: FP1SS| Holder405 { m0, m1 }),
            &[("None,-", 'R'), ("Some,-", 'R')],
        )
    }

    fn holder413() -> Vec<String> {
        check(
            "main2:H2N0:O0_S|V1_NN",
            &rows2(|m0: Option<OI0S>, m1: V1NN| Holder413 { m0, m1 }),
            &[
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder421() -> Vec<String> {
        check(
            "main2:H2N0:P0_N|FP1_NN",
            &rows2(|m0: P0N, m1: FP1NN| Holder421 { m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder429() -> Vec<String> {
        check(
            "main2:H2N0:P0_N|T1_SS",
            &rows2(|m0: P0N, m1: T1SS| Holder429 { m0, m1 }),
            &[("-,Dot", 'R'), ("-,Rect", 'R'), ("-,Wrap", 'R')],
        )
    }

    fn holder437() -> Vec<String> {
        check(
            "main2:H2N0:P0_S|FO1_SS",
            &rows2(|m0: P0S, m1: FO1SS| Holder437 { m0, m1 }),
            &[("-,-", 'R')],
        )
    }

    fn holder445() -> Vec<String> {
        check(
            "main2:H2N0:P0_S|T1_NN",
            &rows2(|m0: P0S, m1: T1NN| Holder445 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder453() -> Vec<String> {
        check(
            "main2:H2N0:T0_NN|FO1_NN",
            &rows2(|m0: T0NN, m1: FO1NN| Holder453 { m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder461() -> Vec<String> {
        check(
            "main2:H2N0:T0_NN|V1_NN",
            &rows2(|m0: T0NN, m1: V1NN| Holder461 { m0, m1 }),
            &[
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder469() -> Vec<String> {
        check(
            "main2:H2N0:V0_NN|O1_N",
            &rows2(|m0: V0NN, m1: Option<OI1N>| Holder469 { m0, m1 }),
            &[
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder477() -> Vec<String> {
        check(
            "main2:H2N1:A0_NN|E1_NN nonstrict",
            &rows2(|m0: A0NN, m1: E1NN| Holder477 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder485() -> Vec<String> {
        check(
            "main2:H2N1:A0_NN|O1_S nonstrict",
            &rows2(|m0: A0NN, m1: Option<OI1S>| Holder485 { own: 7, m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder493() -> Vec<String> {
        check(
            "main2:H2N1:A0_SS|A1_SS nonstrict",
            &rows2(|m0: A0SS, m1: A1SS| Holder493 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder501() -> Vec<String> {
        check(
            "main2:H2N1:A0_SS|O1_N nonstrict",
            &rows2(|m0: A0SS, m1: Option<OI1N>| Holder501 { own: 7, m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder509() -> Vec<String> {
        check(
            "main2:H2N1:E0_NN|A1_NN nonstrict",
            &rows2(|m0: E0NN, m1: A1NN| Holder509 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder517() -> Vec<String> {
        check(
            "main2:H2N1:E0_NN|FT1_NNN",
            &rows2(|m0: E0NN, m1: FT1NNN| Holder517 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder525() -> Vec<String> {
        check(
            "main2:H2N1:E0_NN|V1_SS",
            &rows2(|m0: E0NN, m1: V1SS| Holder525 { own: 7, m0, m1 }),
            &[
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder533() -> Vec<String> {
        check(
            "main2:H2N1:E0_SS|FP1_SS",
            &rows2(|m0: E0SS, m1: FP1SS| Holder533 { own: 7, m0, m1 }),
            &[("Dot,-", 'R'), ("Rect,-", 'R'), ("Wrap,-", 'R')],
        )
    }

    fn holder541() -> Vec<String> {
        check(
            "main2:H2N1:E0_SS|V1_NN",
            &rows2(|m0: E0SS, m1: V1NN| Holder541 { own: 7, m0, m1 }),
            &[
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder549() -> Vec<String> {
        check(
            "main2:H2N1:FO0_NN|FT1_NNN",
            &rows2(|m0: FO0NN, m1: FT1NNN| Holder549 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder557() -> Vec<String> {
        check(
            "main2:H2N1:FP0_NN|A1_SS",
            &rows2(|m0: FP0NN, m1: A1SS| Holder557 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder565() -> Vec<String> {
        check(
            "main2:H2N1:FP0_NN|P1_N",
            &rows2(|m0: FP0NN, m1: P1N| Holder565 { own: 7, m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder573() -> Vec<String> {
        check(
            "main2:H2N1:FT0_NNN|FO1_NN",
            &rows2(|m0: FT0NNN, m1: FO1NN| Holder573 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder581() -> Vec<String> {
        check(
            "main2:H2N1:FT0_NNN|V1_NN",
            &rows2(|m0: FT0NNN, m1: V1NN| Holder581 { own: 7, m0, m1 }),
            &[
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder589() -> Vec<String> {
        check(
            "main2:H2N1:O0_N|FP1_SS",
            &rows2(|m0: Option<OI0N>, m1: FP1SS| Holder589 { own: 7, m0, m1 }),
            &[("None,-", 'R'), ("Some,-", 'R')],
        )
    }

    fn holder597() -> Vec<String> {
        check(
            "main2:H2N1:O0_N|V1_NN",
            &rows2(|m0: Option<OI0N>, m1: V1NN| Holder597 { own: 7, m0, m1 }),
            &[
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder605() -> Vec<String> {
        check(
            "main2:H2N1:O0_S|FP1_NN",
            &rows2(|m0: Option<OI0S>, m1: FP1NN| Holder605 { own: 7, m0, m1 }),
            &[("None,-", 'E'), ("Some,-", 'E')],
        )
    }

    fn holder613() -> Vec<String> {
        check(
            "main2:H2N1:O0_S|T1_SS",
            &rows2(|m0: Option<OI0S>, m1: T1SS| Holder613 { own: 7, m0, m1 }),
            &[
                ("None,Dot", 'R'),
                ("None,Rect", 'R'),
                ("None,Wrap", 'R'),
                ("Some,Dot", 'R'),
                ("Some,Rect", 'R'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder621() -> Vec<String> {
        check(
            "main2:H2N1:P0_N|FO1_SS",
            &rows2(|m0: P0N, m1: FO1SS| Holder621 { own: 7, m0, m1 }),
            &[("-,-", 'R')],
        )
    }

    fn holder629() -> Vec<String> {
        check(
            "main2:H2N1:P0_N|T1_NN",
            &rows2(|m0: P0N, m1: T1NN| Holder629 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder637() -> Vec<String> {
        check(
            "main2:H2N1:P0_S|FO1_NN",
            &rows2(|m0: P0S, m1: FO1NN| Holder637 { own: 7, m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder645() -> Vec<String> {
        check(
            "main2:H2N1:P0_S|P1_S nonstrict",
            &rows2(|m0: P0S, m1: P1S| Holder645 { own: 7, m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder653() -> Vec<String> {
        check(
            "main2:H2N1:T0_NN|E1_SS",
            &rows2(|m0: T0NN, m1: E1SS| Holder653 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder661() -> Vec<String> {
        check(
            "main2:H2N1:T0_NN|T1_NN",
            &rows2(|m0: T0NN, m1: T1NN| Holder661 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder669() -> Vec<String> {
        check(
            "main2:H2N1:V0_NN|FT1_NNN",
            &rows2(|m0: V0NN, m1: FT1NNN| Holder669 { own: 7, m0, m1 }),
            &[
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder677() -> Vec<String> {
        check(
            "main2:H2S0:A0_NN|A1_SS",
            &rows2(|m0: A0NN, m1: A1SS| Holder677 { m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder685() -> Vec<String> {
        check(
            "main2:H2S0:A0_SS|A1_SS",
            &rows2(|m0: A0SS, m1: A1SS| Holder685 { m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder693() -> Vec<String> {
        check(
            "main2:H2S0:E0_NN|A1_SS",
            &rows2(|m0: E0NN, m1: A1SS| Holder693 { m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder701() -> Vec<String> {
        check(
            "main2:H2S0:E0_SS|A1_SS",
            &rows2(|m0: E0SS, m1: A1SS| Holder701 { m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder709() -> Vec<String> {
        check(
            "main2:H2S0:O0_N|A1_SS",
            &rows2(|m0: Option<OI0N>, m1: A1SS| Holder709 { m0, m1 }),
            &[
                ("None,Dot", 'R'),
                ("None,Rect", 'R'),
                ("None,Wrap", 'R'),
                ("Some,Dot", 'R'),
                ("Some,Rect", 'R'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder717() -> Vec<String> {
        check(
            "main2:H2S0:O0_S|A1_SS",
            &rows2(|m0: Option<OI0S>, m1: A1SS| Holder717 { m0, m1 }),
            &[
                ("None,Dot", 'R'),
                ("None,Rect", 'R'),
                ("None,Wrap", 'R'),
                ("Some,Dot", 'R'),
                ("Some,Rect", 'R'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder725() -> Vec<String> {
        check(
            "main2:H2S0:P0_N|A1_SS",
            &rows2(|m0: P0N, m1: A1SS| Holder725 { m0, m1 }),
            &[("-,Dot", 'R'), ("-,Rect", 'R'), ("-,Wrap", 'R')],
        )
    }

    fn holder733() -> Vec<String> {
        check(
            "main2:H2S0:P0_S|A1_SS",
            &rows2(|m0: P0S, m1: A1SS| Holder733 { m0, m1 }),
            &[("-,Dot", 'R'), ("-,Rect", 'R'), ("-,Wrap", 'R')],
        )
    }

    fn holder741() -> Vec<String> {
        check(
            "main2:H2S1:A0_NN|A1_SS",
            &rows2(|m0: A0NN, m1: A1SS| Holder741 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder749() -> Vec<String> {
        check(
            "main2:H2S1:A0_SS|A1_SS",
            &rows2(|m0: A0SS, m1: A1SS| Holder749 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder757() -> Vec<String> {
        check(
            "main2:H2S1:E0_NN|A1_SS",
            &rows2(|m0: E0NN, m1: A1SS| Holder757 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder765() -> Vec<String> {
        check(
            "main2:H2S1:E0_SS|A1_SS",
            &rows2(|m0: E0SS, m1: A1SS| Holder765 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder773() -> Vec<String> {
        check(
            "main2:H2S1:O0_N|A1_SS",
            &rows2(|m0: Option<OI0N>, m1: A1SS| Holder773 { own: 7, m0, m1 }),
            &[
                ("None,Dot", 'R'),
                ("None,Rect", 'R'),
                ("None,Wrap", 'R'),
                ("Some,Dot", 'R'),
                ("Some,Rect", 'R'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder781() -> Vec<String> {
        check(
            "main2:H2S1:O0_S|A1_SS",
            &rows2(|m0: Option<OI0S>, m1: A1SS| Holder781 { own: 7, m0, m1 }),
            &[
                ("None,Dot", 'R'),
                ("None,Rect", 'R'),
                ("None,Wrap", 'R'),
                ("Some,Dot", 'R'),
                ("Some,Rect", 'R'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder789() -> Vec<String> {
        check(
            "main2:H2S1:P0_N|A1_SS",
            &rows2(|m0: P0N, m1: A1SS| Holder789 { own: 7, m0, m1 }),
            &[("-,Dot", 'R'), ("-,Rect", 'R'), ("-,Wrap", 'R')],
        )
    }

    fn holder797() -> Vec<String> {
        check(
            "main2:H2S1:P0_S|A1_SS",
            &rows2(|m0: P0S, m1: A1SS| Holder797 { own: 7, m0, m1 }),
            &[("-,Dot", 'R'), ("-,Rect", 'R'), ("-,Wrap", 'R')],
        )
    }

    fn holder805() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|A1_NN|A2_SS nonstrict",
            &rows3(|m0: A0NN, m1: A1NN, m2: A2SS| Holder805 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder813() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|A1_NN|O2_N nonstrict",
            &rows3(|m0: A0NN, m1: A1NN, m2: Option<OI2N>| Holder813 { m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder821() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|A1_SS|A2_NN nonstrict",
            &rows3(|m0: A0NN, m1: A1SS, m2: A2NN| Holder821 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder829() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|A1_SS|FT2_NNN",
            &rows3(|m0: A0NN, m1: A1SS, m2: FT2NNN| Holder829 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder837() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|A1_SS|V2_SS",
            &rows3(|m0: A0NN, m1: A1SS, m2: V2SS| Holder837 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'R'),
                ("Dot,Dot,Wrap", 'R'),
                ("Dot,Rect,Rect", 'R'),
                ("Dot,Rect,Wrap", 'R'),
                ("Dot,Wrap,Rect", 'R'),
                ("Dot,Wrap,Wrap", 'R'),
                ("Rect,Dot,Rect", 'R'),
                ("Rect,Dot,Wrap", 'R'),
                ("Rect,Rect,Rect", 'R'),
                ("Rect,Rect,Wrap", 'R'),
                ("Rect,Wrap,Rect", 'R'),
                ("Rect,Wrap,Wrap", 'R'),
                ("Wrap,Dot,Rect", 'R'),
                ("Wrap,Dot,Wrap", 'R'),
                ("Wrap,Rect,Rect", 'R'),
                ("Wrap,Rect,Wrap", 'R'),
                ("Wrap,Wrap,Rect", 'R'),
                ("Wrap,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder845() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|E1_NN|FP2_SS",
            &rows3(|m0: A0NN, m1: E1NN, m2: FP2SS| Holder845 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder853() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|E1_NN|V2_NN",
            &rows3(|m0: A0NN, m1: E1NN, m2: V2NN| Holder853 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder861() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|E1_SS|FP2_NN",
            &rows3(|m0: A0NN, m1: E1SS, m2: FP2NN| Holder861 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder869() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|E1_SS|T2_SS",
            &rows3(|m0: A0NN, m1: E1SS, m2: T2SS| Holder869 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'R'),
                ("Dot,Dot,Rect", 'R'),
                ("Dot,Dot,Wrap", 'R'),
                ("Dot,Rect,Dot", 'R'),
                ("Dot,Rect,Rect", 'R'),
                ("Dot,Rect,Wrap", 'R'),
                ("Dot,Wrap,Dot", 'R'),
                ("Dot,Wrap,Rect", 'R'),
                ("Dot,Wrap,Wrap", 'R'),
                ("Rect,Dot,Dot", 'R'),
                ("Rect,Dot,Rect", 'R'),
                ("Rect,Dot,Wrap", 'R'),
                ("Rect,Rect,Dot", 'R'),
                ("Rect,Rect,Rect", 'R'),
                ("Rect,Rect,Wrap", 'R'),
                ("Rect,Wrap,Dot", 'R'),
                ("Rect,Wrap,Rect", 'R'),
                ("Rect,Wrap,Wrap", 'R'),
                ("Wrap,Dot,Dot", 'R'),
                ("Wrap,Dot,Rect", 'R'),
                ("Wrap,Dot,Wrap", 'R'),
                ("Wrap,Rect,Dot", 'R'),
                ("Wrap,Rect,Rect", 'R'),
                ("Wrap,Rect,Wrap", 'R'),
                ("Wrap,Wrap,Dot", 'R'),
                ("Wrap,Wrap,Rect", 'R'),
                ("Wrap,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder877() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|FO1_NN|FP2_NN",
            &rows3(|m0: A0NN, m1: FO1NN, m2: FP2NN| Holder877 { m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder885() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|FP1_NN|A2_NN",
            &rows3(|m0: A0NN, m1: FP1NN, m2: A2NN| Holder885 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder893() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|FP1_NN|O2_S",
            &rows3(|m0: A0NN, m1: FP1NN, m2: Option<OI2S>| Holder893 { m0, m1, m2 }),
            &[
                ("Dot,-,None", 'E'),
                ("Dot,-,Some", 'E'),
                ("Rect,-,None", 'E'),
                ("Rect,-,Some", 'E'),
                ("Wrap,-,None", 'E'),
                ("Wrap,-,Some", 'E'),
            ],
        )
    }

    fn holder901() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|FT1_NNN|E2_SS",
            &rows3(|m0: A0NN, m1: FT1NNN, m2: E2SS| Holder901 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder909() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|FT1_NNN|T2_NN",
            &rows3(|m0: A0NN, m1: FT1NNN, m2: T2NN| Holder909 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder917() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|O1_N|FP2_NN",
            &rows3(|m0: A0NN, m1: Option<OI1N>, m2: FP2NN| Holder917 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'E'),
                ("Dot,Some,-", 'E'),
                ("Rect,None,-", 'E'),
                ("Rect,Some,-", 'E'),
                ("Wrap,None,-", 'E'),
                ("Wrap,Some,-", 'E'),
            ],
        )
    }

    fn holder925() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|O1_N|T2_SS",
            &rows3(|m0: A0NN, m1: Option<OI1N>, m2: T2SS| Holder925 { m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'R'),
                ("Dot,None,Rect", 'R'),
                ("Dot,None,Wrap", 'R'),
                ("Dot,Some,Dot", 'R'),
                ("Dot,Some,Rect", 'R'),
                ("Dot,Some,Wrap", 'R'),
                ("Rect,None,Dot", 'R'),
                ("Rect,None,Rect", 'R'),
                ("Rect,None,Wrap", 'R'),
                ("Rect,Some,Dot", 'R'),
                ("Rect,Some,Rect", 'R'),
                ("Rect,Some,Wrap", 'R'),
                ("Wrap,None,Dot", 'R'),
                ("Wrap,None,Rect", 'R'),
                ("Wrap,None,Wrap", 'R'),
                ("Wrap,Some,Dot", 'R'),
                ("Wrap,Some,Rect", 'R'),
                ("Wrap,Some,Wrap", 'R'),
            ],
        )
    }

    fn holder933() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|O1_S|FO2_SS",
            &rows3(|m0: A0NN, m1: Option<OI1S>, m2: FO2SS| Holder933 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder941() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|O1_S|T2_NN",
            &rows3(|m0: A0NN, m1: Option<OI1S>, m2: T2NN| Holder941 { m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder949() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|P1_N|FO2_NN",
            &rows3(|m0: A0NN, m1: P1N, m2: FO2NN| Holder949 { m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder957() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|P1_N|P2_S nonstrict",
            &rows3(|m0: A0NN, m1: P1N, m2: P2S| Holder957 { m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder965() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|P1_S|E2_SS nonstrict",
            &rows3(|m0: A0NN, m1: P1S, m2: E2SS| Holder965 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder973() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|P1_S|P2_N nonstrict",
            &rows3(|m0: A0NN, m1: P1S, m2: P2N| Holder973 { m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder981() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|T1_NN|E2_NN",
            &rows3(|m0: A0NN, m1: T1NN, m2: E2NN| Holder981 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder989() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|T1_NN|P2_S",
            &rows3(|m0: A0NN, m1: T1NN, m2: P2S| Holder989 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder997() -> Vec<String> {
        check(
            "main3:H3N0:A0_NN|V1_NN|FP2_NN",
            &rows3(|m0: A0NN, m1: V1NN, m2: FP2NN| Holder997 { m0, m1, m2 }),
            &[
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1005() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|A1_NN|A2_NN nonstrict",
            &rows3(|m0: A0SS, m1: A1NN, m2: A2NN| Holder1005 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1013() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|A1_NN|FT2_NNN",
            &rows3(|m0: A0SS, m1: A1NN, m2: FT2NNN| Holder1013 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1021() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|A1_NN|V2_SS",
            &rows3(|m0: A0SS, m1: A1NN, m2: V2SS| Holder1021 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'R'),
                ("Dot,Dot,Wrap", 'R'),
                ("Dot,Rect,Rect", 'R'),
                ("Dot,Rect,Wrap", 'R'),
                ("Dot,Wrap,Rect", 'R'),
                ("Dot,Wrap,Wrap", 'R'),
                ("Rect,Dot,Rect", 'R'),
                ("Rect,Dot,Wrap", 'R'),
                ("Rect,Rect,Rect", 'R'),
                ("Rect,Rect,Wrap", 'R'),
                ("Rect,Wrap,Rect", 'R'),
                ("Rect,Wrap,Wrap", 'R'),
                ("Wrap,Dot,Rect", 'R'),
                ("Wrap,Dot,Wrap", 'R'),
                ("Wrap,Rect,Rect", 'R'),
                ("Wrap,Rect,Wrap", 'R'),
                ("Wrap,Wrap,Rect", 'R'),
                ("Wrap,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder1029() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|A1_SS|FP2_SS",
            &rows3(|m0: A0SS, m1: A1SS, m2: FP2SS| Holder1029 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder1037() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|A1_SS|V2_NN",
            &rows3(|m0: A0SS, m1: A1SS, m2: V2NN| Holder1037 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1045() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|E1_NN|FP2_NN",
            &rows3(|m0: A0SS, m1: E1NN, m2: FP2NN| Holder1045 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1053() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|E1_NN|T2_SS",
            &rows3(|m0: A0SS, m1: E1NN, m2: T2SS| Holder1053 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'R'),
                ("Dot,Dot,Rect", 'R'),
                ("Dot,Dot,Wrap", 'R'),
                ("Dot,Rect,Dot", 'R'),
                ("Dot,Rect,Rect", 'R'),
                ("Dot,Rect,Wrap", 'R'),
                ("Dot,Wrap,Dot", 'R'),
                ("Dot,Wrap,Rect", 'R'),
                ("Dot,Wrap,Wrap", 'R'),
                ("Rect,Dot,Dot", 'R'),
                ("Rect,Dot,Rect", 'R'),
                ("Rect,Dot,Wrap", 'R'),
                ("Rect,Rect,Dot", 'R'),
                ("Rect,Rect,Rect", 'R'),
                ("Rect,Rect,Wrap", 'R'),
                ("Rect,Wrap,Dot", 'R'),
                ("Rect,Wrap,Rect", 'R'),
                ("Rect,Wrap,Wrap", 'R'),
                ("Wrap,Dot,Dot", 'R'),
                ("Wrap,Dot,Rect", 'R'),
                ("Wrap,Dot,Wrap", 'R'),
                ("Wrap,Rect,Dot", 'R'),
                ("Wrap,Rect,Rect", 'R'),
                ("Wrap,Rect,Wrap", 'R'),
                ("Wrap,Wrap,Dot", 'R'),
                ("Wrap,Wrap,Rect", 'R'),
                ("Wrap,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder1061() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|E1_SS|FO2_SS",
            &rows3(|m0: A0SS, m1: E1SS, m2: FO2SS| Holder1061 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder1069() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|E1_SS|T2_NN",
            &rows3(|m0: A0SS, m1: E1SS, m2: T2NN| Holder1069 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1077() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|FO1_NN|FO2_NN",
            &rows3(|m0: A0SS, m1: FO1NN, m2: FO2NN| Holder1077 { m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder1085() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|FO1_NN|V2_NN",
            &rows3(|m0: A0SS, m1: FO1NN, m2: V2NN| Holder1085 { m0, m1, m2 }),
            &[
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder1093() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|FP1_NN|O2_N",
            &rows3(|m0: A0SS, m1: FP1NN, m2: Option<OI2N>| Holder1093 { m0, m1, m2 }),
            &[
                ("Dot,-,None", 'E'),
                ("Dot,-,Some", 'E'),
                ("Rect,-,None", 'E'),
                ("Rect,-,Some", 'E'),
                ("Wrap,-,None", 'E'),
                ("Wrap,-,Some", 'E'),
            ],
        )
    }

    fn holder1101() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|FT1_NNN|E2_NN",
            &rows3(|m0: A0SS, m1: FT1NNN, m2: E2NN| Holder1101 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1109() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|FT1_NNN|P2_S",
            &rows3(|m0: A0SS, m1: FT1NNN, m2: P2S| Holder1109 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1117() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|O1_N|FO2_SS",
            &rows3(|m0: A0SS, m1: Option<OI1N>, m2: FO2SS| Holder1117 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder1125() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|O1_N|T2_NN",
            &rows3(|m0: A0SS, m1: Option<OI1N>, m2: T2NN| Holder1125 { m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder1133() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|O1_S|FO2_NN",
            &rows3(|m0: A0SS, m1: Option<OI1S>, m2: FO2NN| Holder1133 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'E'),
                ("Dot,Some,-", 'E'),
                ("Rect,None,-", 'E'),
                ("Rect,Some,-", 'E'),
                ("Wrap,None,-", 'E'),
                ("Wrap,Some,-", 'E'),
            ],
        )
    }

    fn holder1141() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|O1_S|P2_S nonstrict",
            &rows3(|m0: A0SS, m1: Option<OI1S>, m2: P2S| Holder1141 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'E'),
                ("Dot,Some,-", 'E'),
                ("Rect,None,-", 'E'),
                ("Rect,Some,-", 'E'),
                ("Wrap,None,-", 'E'),
                ("Wrap,Some,-", 'E'),
            ],
        )
    }

    fn holder1149() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|P1_N|E2_SS nonstrict",
            &rows3(|m0: A0SS, m1: P1N, m2: E2SS| Holder1149 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder1157() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|P1_N|P2_N nonstrict",
            &rows3(|m0: A0SS, m1: P1N, m2: P2N| Holder1157 { m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder1165() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|P1_S|E2_NN nonstrict",
            &rows3(|m0: A0SS, m1: P1S, m2: E2NN| Holder1165 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder1173() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|P1_S|O2_S nonstrict",
            &rows3(|m0: A0SS, m1: P1S, m2: Option<OI2S>| Holder1173 { m0, m1, m2 }),
            &[
                ("Dot,-,None", 'E'),
                ("Dot,-,Some", 'E'),
                ("Rect,-,None", 'E'),
                ("Rect,-,Some", 'E'),
                ("Wrap,-,None", 'E'),
                ("Wrap,-,Some", 'E'),
            ],
        )
    }

    fn holder1181() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|T1_NN|A2_SS",
            &rows3(|m0: A0SS, m1: T1NN, m2: A2SS| Holder1181 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1189() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|T1_NN|P2_N",
            &rows3(|m0: A0SS, m1: T1NN, m2: P2N| Holder1189 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1197() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|V1_NN|FO2_NN",
            &rows3(|m0: A0SS, m1: V1NN, m2: FO2NN| Holder1197 { m0, m1, m2 }),
            &[
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1205() -> Vec<String> {
        check(
            "main3:H3N0:A0_SS|V1_NN|V2_NN",
            &rows3(|m0: A0SS, m1: V1NN, m2: V2NN| Holder1205 { m0, m1, m2 }),
            &[
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1213() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|A1_NN|FP2_SS",
            &rows3(|m0: E0NN, m1: A1NN, m2: FP2SS| Holder1213 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder1221() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|A1_NN|V2_NN",
            &rows3(|m0: E0NN, m1: A1NN, m2: V2NN| Holder1221 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1229() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|A1_SS|FP2_NN",
            &rows3(|m0: E0NN, m1: A1SS, m2: FP2NN| Holder1229 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1237() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|A1_SS|T2_SS",
            &rows3(|m0: E0NN, m1: A1SS, m2: T2SS| Holder1237 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'R'),
                ("Dot,Dot,Rect", 'R'),
                ("Dot,Dot,Wrap", 'R'),
                ("Dot,Rect,Dot", 'R'),
                ("Dot,Rect,Rect", 'R'),
                ("Dot,Rect,Wrap", 'R'),
                ("Dot,Wrap,Dot", 'R'),
                ("Dot,Wrap,Rect", 'R'),
                ("Dot,Wrap,Wrap", 'R'),
                ("Rect,Dot,Dot", 'R'),
                ("Rect,Dot,Rect", 'R'),
                ("Rect,Dot,Wrap", 'R'),
                ("Rect,Rect,Dot", 'R'),
                ("Rect,Rect,Rect", 'R'),
                ("Rect,Rect,Wrap", 'R'),
                ("Rect,Wrap,Dot", 'R'),
                ("Rect,Wrap,Rect", 'R'),
                ("Rect,Wrap,Wrap", 'R'),
                ("Wrap,Dot,Dot", 'R'),
                ("Wrap,Dot,Rect", 'R'),
                ("Wrap,Dot,Wrap", 'R'),
                ("Wrap,Rect,Dot", 'R'),
                ("Wrap,Rect,Rect", 'R'),
                ("Wrap,Rect,Wrap", 'R'),
                ("Wrap,Wrap,Dot", 'R'),
                ("Wrap,Wrap,Rect", 'R'),
                ("Wrap,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder1245() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|E1_NN|FO2_SS",
            &rows3(|m0: E0NN, m1: E1NN, m2: FO2SS| Holder1245 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder1253() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|E1_NN|T2_NN",
            &rows3(|m0: E0NN, m1: E1NN, m2: T2NN| Holder1253 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1261() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|E1_SS|FO2_NN",
            &rows3(|m0: E0NN, m1: E1SS, m2: FO2NN| Holder1261 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1269() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|E1_SS|P2_S nonstrict",
            &rows3(|m0: E0NN, m1: E1SS, m2: P2S| Holder1269 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1277() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|FO1_NN|E2_SS",
            &rows3(|m0: E0NN, m1: FO1NN, m2: E2SS| Holder1277 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder1285() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|FO1_NN|T2_NN",
            &rows3(|m0: E0NN, m1: FO1NN, m2: T2NN| Holder1285 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder1293() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|FP1_NN|FT2_NNN",
            &rows3(|m0: E0NN, m1: FP1NN, m2: FT2NNN| Holder1293 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder1301() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|FT1_NNN|A2_SS",
            &rows3(|m0: E0NN, m1: FT1NNN, m2: A2SS| Holder1301 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1309() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|FT1_NNN|P2_N",
            &rows3(|m0: E0NN, m1: FT1NNN, m2: P2N| Holder1309 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1317() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|O1_N|FO2_NN",
            &rows3(|m0: E0NN, m1: Option<OI1N>, m2: FO2NN| Holder1317 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'E'),
                ("Dot,Some,-", 'E'),
                ("Rect,None,-", 'E'),
                ("Rect,Some,-", 'E'),
                ("Wrap,None,-", 'E'),
                ("Wrap,Some,-", 'E'),
            ],
        )
    }

    fn holder1325() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|O1_N|P2_S nonstrict",
            &rows3(|m0: E0NN, m1: Option<OI1N>, m2: P2S| Holder1325 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'E'),
                ("Dot,Some,-", 'E'),
                ("Rect,None,-", 'E'),
                ("Rect,Some,-", 'E'),
                ("Wrap,None,-", 'E'),
                ("Wrap,Some,-", 'E'),
            ],
        )
    }

    fn holder1333() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|O1_S|E2_SS nonstrict",
            &rows3(|m0: E0NN, m1: Option<OI1S>, m2: E2SS| Holder1333 { m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder1341() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|O1_S|P2_N nonstrict",
            &rows3(|m0: E0NN, m1: Option<OI1S>, m2: P2N| Holder1341 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'E'),
                ("Dot,Some,-", 'E'),
                ("Rect,None,-", 'E'),
                ("Rect,Some,-", 'E'),
                ("Wrap,None,-", 'E'),
                ("Wrap,Some,-", 'E'),
            ],
        )
    }

    fn holder1349() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|P1_N|E2_NN nonstrict",
            &rows3(|m0: E0NN, m1: P1N, m2: E2NN| Holder1349 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder1357() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|P1_N|O2_S nonstrict",
            &rows3(|m0: E0NN, m1: P1N, m2: Option<OI2S>| Holder1357 { m0, m1, m2 }),
            &[
                ("Dot,-,None", 'E'),
                ("Dot,-,Some", 'E'),
                ("Rect,-,None", 'E'),
                ("Rect,-,Some", 'E'),
                ("Wrap,-,None", 'E'),
                ("Wrap,-,Some", 'E'),
            ],
        )
    }

    fn holder1365() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|P1_S|A2_SS nonstrict",
            &rows3(|m0: E0NN, m1: P1S, m2: A2SS| Holder1365 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder1373() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|P1_S|O2_N nonstrict",
            &rows3(|m0: E0NN, m1: P1S, m2: Option<OI2N>| Holder1373 { m0, m1, m2 }),
            &[
                ("Dot,-,None", 'E'),
                ("Dot,-,Some", 'E'),
                ("Rect,-,None", 'E'),
                ("Rect,-,Some", 'E'),
                ("Wrap,-,None", 'E'),
                ("Wrap,-,Some", 'E'),
            ],
        )
    }

    fn holder1381() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|T1_NN|A2_NN",
            &rows3(|m0: E0NN, m1: T1NN, m2: A2NN| Holder1381 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1389() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|T1_NN|O2_S",
            &rows3(|m0: E0NN, m1: T1NN, m2: Option<OI2S>| Holder1389 { m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder1397() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|V1_NN|E2_SS",
            &rows3(|m0: E0NN, m1: V1NN, m2: E2SS| Holder1397 { m0, m1, m2 }),
            &[
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1405() -> Vec<String> {
        check(
            "main3:H3N0:E0_NN|V1_NN|T2_NN",
            &rows3(|m0: E0NN, m1: V1NN, m2: T2NN| Holder1405 { m0, m1, m2 }),
            &[
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1413() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|A1_NN|FP2_NN",
            &rows3(|m0: E0SS, m1: A1NN, m2: FP2NN| Holder1413 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1421() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|A1_NN|T2_SS",
            &rows3(|m0: E0SS, m1: A1NN, m2: T2SS| Holder1421 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'R'),
                ("Dot,Dot,Rect", 'R'),
                ("Dot,Dot,Wrap", 'R'),
                ("Dot,Rect,Dot", 'R'),
                ("Dot,Rect,Rect", 'R'),
                ("Dot,Rect,Wrap", 'R'),
                ("Dot,Wrap,Dot", 'R'),
                ("Dot,Wrap,Rect", 'R'),
                ("Dot,Wrap,Wrap", 'R'),
                ("Rect,Dot,Dot", 'R'),
                ("Rect,Dot,Rect", 'R'),
                ("Rect,Dot,Wrap", 'R'),
                ("Rect,Rect,Dot", 'R'),
                ("Rect,Rect,Rect", 'R'),
                ("Rect,Rect,Wrap", 'R'),
                ("Rect,Wrap,Dot", 'R'),
                ("Rect,Wrap,Rect", 'R'),
                ("Rect,Wrap,Wrap", 'R'),
                ("Wrap,Dot,Dot", 'R'),
                ("Wrap,Dot,Rect", 'R'),
                ("Wrap,Dot,Wrap", 'R'),
                ("Wrap,Rect,Dot", 'R'),
                ("Wrap,Rect,Rect", 'R'),
                ("Wrap,Rect,Wrap", 'R'),
                ("Wrap,Wrap,Dot", 'R'),
                ("Wrap,Wrap,Rect", 'R'),
                ("Wrap,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder1429() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|A1_SS|FO2_SS",
            &rows3(|m0: E0SS, m1: A1SS, m2: FO2SS| Holder1429 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder1437() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|A1_SS|T2_NN",
            &rows3(|m0: E0SS, m1: A1SS, m2: T2NN| Holder1437 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1445() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|E1_NN|FO2_NN",
            &rows3(|m0: E0SS, m1: E1NN, m2: FO2NN| Holder1445 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1453() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|E1_NN|P2_S nonstrict",
            &rows3(|m0: E0SS, m1: E1NN, m2: P2S| Holder1453 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1461() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|E1_SS|E2_SS nonstrict",
            &rows3(|m0: E0SS, m1: E1SS, m2: E2SS| Holder1461 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1469() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|E1_SS|P2_N nonstrict",
            &rows3(|m0: E0SS, m1: E1SS, m2: P2N| Holder1469 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1477() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|FO1_NN|E2_NN",
            &rows3(|m0: E0SS, m1: FO1NN, m2: E2NN| Holder1477 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder1485() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|FO1_NN|P2_S",
            &rows3(|m0: E0SS, m1: FO1NN, m2: P2S| Holder1485 { m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder1493() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|FP1_NN|FP2_NN",
            &rows3(|m0: E0SS, m1: FP1NN, m2: FP2NN| Holder1493 { m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder1501() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|FT1_NNN|A2_NN",
            &rows3(|m0: E0SS, m1: FT1NNN, m2: A2NN| Holder1501 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1509() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|FT1_NNN|O2_S",
            &rows3(|m0: E0SS, m1: FT1NNN, m2: Option<OI2S>| Holder1509 { m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder1517() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|O1_N|E2_SS nonstrict",
            &rows3(|m0: E0SS, m1: Option<OI1N>, m2: E2SS| Holder1517 { m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder1525() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|O1_N|P2_N nonstrict",
            &rows3(|m0: E0SS, m1: Option<OI1N>, m2: P2N| Holder1525 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'E'),
                ("Dot,Some,-", 'E'),
                ("Rect,None,-", 'E'),
                ("Rect,Some,-", 'E'),
                ("Wrap,None,-", 'E'),
                ("Wrap,Some,-", 'E'),
            ],
        )
    }

    fn holder1533() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|O1_S|E2_NN nonstrict",
            &rows3(|m0: E0SS, m1: Option<OI1S>, m2: E2NN| Holder1533 { m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder1541() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|O1_S|O2_S nonstrict",
            &rows3(|m0: E0SS, m1: Option<OI1S>, m2: Option<OI2S>| Holder1541 { m0, m1, m2 }),
            &[
                ("Dot,None,None", 'E'),
                ("Dot,None,Some", 'E'),
                ("Dot,Some,None", 'E'),
                ("Dot,Some,Some", 'E'),
                ("Rect,None,None", 'E'),
                ("Rect,None,Some", 'E'),
                ("Rect,Some,None", 'E'),
                ("Rect,Some,Some", 'E'),
                ("Wrap,None,None", 'E'),
                ("Wrap,None,Some", 'E'),
                ("Wrap,Some,None", 'E'),
                ("Wrap,Some,Some", 'E'),
            ],
        )
    }

    fn holder1549() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|P1_N|A2_SS nonstrict",
            &rows3(|m0: E0SS, m1: P1N, m2: A2SS| Holder1549 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder1557() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|P1_N|O2_N nonstrict",
            &rows3(|m0: E0SS, m1: P1N, m2: Option<OI2N>| Holder1557 { m0, m1, m2 }),
            &[
                ("Dot,-,None", 'E'),
                ("Dot,-,Some", 'E'),
                ("Rect,-,None", 'E'),
                ("Rect,-,Some", 'E'),
                ("Wrap,-,None", 'E'),
                ("Wrap,-,Some", 'E'),
            ],
        )
    }

    fn holder1565() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|P1_S|A2_NN nonstrict",
            &rows3(|m0: E0SS, m1: P1S, m2: A2NN| Holder1565 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder1573() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|P1_S|FT2_NNN",
            &rows3(|m0: E0SS, m1: P1S, m2: FT2NNN| Holder1573 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder1581() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|P1_S|V2_SS",
            &rows3(|m0: E0SS, m1: P1S, m2: V2SS| Holder1581 { m0, m1, m2 }),
            &[
                ("Dot,-,Rect", 'R'),
                ("Dot,-,Wrap", 'R'),
                ("Rect,-,Rect", 'R'),
                ("Rect,-,Wrap", 'R'),
                ("Wrap,-,Rect", 'R'),
                ("Wrap,-,Wrap", 'R'),
            ],
        )
    }

    fn holder1589() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|T1_NN|O2_N",
            &rows3(|m0: E0SS, m1: T1NN, m2: Option<OI2N>| Holder1589 { m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder1597() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|V1_NN|E2_NN",
            &rows3(|m0: E0SS, m1: V1NN, m2: E2NN| Holder1597 { m0, m1, m2 }),
            &[
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1605() -> Vec<String> {
        check(
            "main3:H3N0:E0_SS|V1_NN|P2_S",
            &rows3(|m0: E0SS, m1: V1NN, m2: P2S| Holder1605 { m0, m1, m2 }),
            &[
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1613() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|A1_NN|FP2_NN",
            &rows3(|m0: FO0NN, m1: A1NN, m2: FP2NN| Holder1613 { m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder1621() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|A1_SS|A2_NN",
            &rows3(|m0: FO0NN, m1: A1SS, m2: A2NN| Holder1621 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1629() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|A1_SS|O2_S",
            &rows3(|m0: FO0NN, m1: A1SS, m2: Option<OI2S>| Holder1629 { m0, m1, m2 }),
            &[
                ("-,Dot,None", 'E'),
                ("-,Dot,Some", 'E'),
                ("-,Rect,None", 'E'),
                ("-,Rect,Some", 'E'),
                ("-,Wrap,None", 'E'),
                ("-,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder1637() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|E1_NN|E2_SS",
            &rows3(|m0: FO0NN, m1: E1NN, m2: E2SS| Holder1637 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1645() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|E1_NN|T2_NN",
            &rows3(|m0: FO0NN, m1: E1NN, m2: T2NN| Holder1645 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1653() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|E1_SS|FT2_NNN",
            &rows3(|m0: FO0NN, m1: E1SS, m2: FT2NNN| Holder1653 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1661() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|FO1_NN|A2_SS",
            &rows3(|m0: FO0NN, m1: FO1NN, m2: A2SS| Holder1661 { m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder1669() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|FO1_NN|P2_N",
            &rows3(|m0: FO0NN, m1: FO1NN, m2: P2N| Holder1669 { m0, m1, m2 }),
            &[("-,-,-", 'E')],
        )
    }

    fn holder1677() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|FP1_NN|FO2_NN",
            &rows3(|m0: FO0NN, m1: FP1NN, m2: FO2NN| Holder1677 { m0, m1, m2 }),
            &[("-,-,-", 'E')],
        )
    }

    fn holder1685() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|FP1_NN|V2_NN",
            &rows3(|m0: FO0NN, m1: FP1NN, m2: V2NN| Holder1685 { m0, m1, m2 }),
            &[("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder1693() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|FT1_NNN|O2_N",
            &rows3(|m0: FO0NN, m1: FT1NNN, m2: Option<OI2N>| Holder1693 { m0, m1, m2 }),
            &[
                ("-,Dot,None", 'E'),
                ("-,Dot,Some", 'E'),
                ("-,Rect,None", 'E'),
                ("-,Rect,Some", 'E'),
                ("-,Wrap,None", 'E'),
                ("-,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder1701() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|O1_N|E2_NN",
            &rows3(|m0: FO0NN, m1: Option<OI1N>, m2: E2NN| Holder1701 { m0, m1, m2 }),
            &[
                ("-,None,Dot", 'E'),
                ("-,None,Rect", 'E'),
                ("-,None,Wrap", 'E'),
                ("-,Some,Dot", 'E'),
                ("-,Some,Rect", 'E'),
                ("-,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder1709() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|O1_N|P2_S",
            &rows3(|m0: FO0NN, m1: Option<OI1N>, m2: P2S| Holder1709 { m0, m1, m2 }),
            &[("-,None,-", 'E'), ("-,Some,-", 'E')],
        )
    }

    fn holder1717() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|O1_S|FP2_NN",
            &rows3(|m0: FO0NN, m1: Option<OI1S>, m2: FP2NN| Holder1717 { m0, m1, m2 }),
            &[("-,None,-", 'E'), ("-,Some,-", 'E')],
        )
    }

    fn holder1725() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|P1_N|A2_NN",
            &rows3(|m0: FO0NN, m1: P1N, m2: A2NN| Holder1725 { m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder1733() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|P1_N|O2_S",
            &rows3(|m0: FO0NN, m1: P1N, m2: Option<OI2S>| Holder1733 { m0, m1, m2 }),
            &[("-,-,None", 'E'), ("-,-,Some", 'E')],
        )
    }

    fn holder1741() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|P1_S|E2_SS",
            &rows3(|m0: FO0NN, m1: P1S, m2: E2SS| Holder1741 { m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder1749() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|P1_S|T2_NN",
            &rows3(|m0: FO0NN, m1: P1S, m2: T2NN| Holder1749 { m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder1757() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|T1_NN|FT2_NNN",
            &rows3(|m0: FO0NN, m1: T1NN, m2: FT2NNN| Holder1757 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1765() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|V1_NN|A2_SS",
            &rows3(|m0: FO0NN, m1: V1NN, m2: A2SS| Holder1765 { m0, m1, m2 }),
            &[
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1773() -> Vec<String> {
        check(
            "main3:H3N0:FO0_NN|V1_NN|P2_N",
            &rows3(|m0: FO0NN, m1: V1NN, m2: P2N| Holder1773 { m0, m1, m2 }),
            &[("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder1781() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|A1_NN|FO2_NN",
            &rows3(|m0: FP0NN, m1: A1NN, m2: FO2NN| Holder1781 { m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder1789() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|A1_NN|V2_NN",
            &rows3(|m0: FP0NN, m1: A1NN, m2: V2NN| Holder1789 { m0, m1, m2 }),
            &[
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1797() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|A1_SS|O2_N",
            &rows3(|m0: FP0NN, m1: A1SS, m2: Option<OI2N>| Holder1797 { m0, m1, m2 }),
            &[
                ("-,Dot,None", 'E'),
                ("-,Dot,Some", 'E'),
                ("-,Rect,None", 'E'),
                ("-,Rect,Some", 'E'),
                ("-,Wrap,None", 'E'),
                ("-,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder1805() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|E1_NN|E2_NN",
            &rows3(|m0: FP0NN, m1: E1NN, m2: E2NN| Holder1805 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1813() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|E1_NN|P2_S",
            &rows3(|m0: FP0NN, m1: E1NN, m2: P2S| Holder1813 { m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder1821() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|E1_SS|FP2_NN",
            &rows3(|m0: FP0NN, m1: E1SS, m2: FP2NN| Holder1821 { m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder1829() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|FO1_NN|A2_NN",
            &rows3(|m0: FP0NN, m1: FO1NN, m2: A2NN| Holder1829 { m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder1837() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|FO1_NN|O2_S",
            &rows3(|m0: FP0NN, m1: FO1NN, m2: Option<OI2S>| Holder1837 { m0, m1, m2 }),
            &[("-,-,None", 'E'), ("-,-,Some", 'E')],
        )
    }

    fn holder1845() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|FP1_NN|E2_SS",
            &rows3(|m0: FP0NN, m1: FP1NN, m2: E2SS| Holder1845 { m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder1853() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|FP1_NN|T2_NN",
            &rows3(|m0: FP0NN, m1: FP1NN, m2: T2NN| Holder1853 { m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder1861() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|FT1_NNN|FT2_NNN",
            &rows3(|m0: FP0NN, m1: FT1NNN, m2: FT2NNN| Holder1861 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1869() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|O1_N|A2_SS",
            &rows3(|m0: FP0NN, m1: Option<OI1N>, m2: A2SS| Holder1869 { m0, m1, m2 }),
            &[
                ("-,None,Dot", 'E'),
                ("-,None,Rect", 'E'),
                ("-,None,Wrap", 'E'),
                ("-,Some,Dot", 'E'),
                ("-,Some,Rect", 'E'),
                ("-,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder1877() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|O1_N|P2_N",
            &rows3(|m0: FP0NN, m1: Option<OI1N>, m2: P2N| Holder1877 { m0, m1, m2 }),
            &[("-,None,-", 'E'), ("-,Some,-", 'E')],
        )
    }

    fn holder1885() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|O1_S|FO2_NN",
            &rows3(|m0: FP0NN, m1: Option<OI1S>, m2: FO2NN| Holder1885 { m0, m1, m2 }),
            &[("-,None,-", 'E'), ("-,Some,-", 'E')],
        )
    }

    fn holder1893() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|O1_S|V2_NN",
            &rows3(|m0: FP0NN, m1: Option<OI1S>, m2: V2NN| Holder1893 { m0, m1, m2 }),
            &[
                ("-,None,Rect", 'E'),
                ("-,None,Wrap", 'E'),
                ("-,Some,Rect", 'E'),
                ("-,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder1901() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|P1_N|O2_N",
            &rows3(|m0: FP0NN, m1: P1N, m2: Option<OI2N>| Holder1901 { m0, m1, m2 }),
            &[("-,-,None", 'E'), ("-,-,Some", 'E')],
        )
    }

    fn holder1909() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|P1_S|E2_NN",
            &rows3(|m0: FP0NN, m1: P1S, m2: E2NN| Holder1909 { m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder1917() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|P1_S|P2_S",
            &rows3(|m0: FP0NN, m1: P1S, m2: P2S| Holder1917 { m0, m1, m2 }),
            &[("-,-,-", 'E')],
        )
    }

    fn holder1925() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|T1_NN|FP2_NN",
            &rows3(|m0: FP0NN, m1: T1NN, m2: FP2NN| Holder1925 { m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder1933() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|V1_NN|A2_NN",
            &rows3(|m0: FP0NN, m1: V1NN, m2: A2NN| Holder1933 { m0, m1, m2 }),
            &[
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1941() -> Vec<String> {
        check(
            "main3:H3N0:FP0_NN|V1_NN|O2_S",
            &rows3(|m0: FP0NN, m1: V1NN, m2: Option<OI2S>| Holder1941 { m0, m1, m2 }),
            &[
                ("-,Rect,None", 'E'),
                ("-,Rect,Some", 'E'),
                ("-,Wrap,None", 'E'),
                ("-,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder1949() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|A1_NN|E2_SS",
            &rows3(|m0: FT0NNN, m1: A1NN, m2: E2SS| Holder1949 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1957() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|A1_NN|T2_NN",
            &rows3(|m0: FT0NNN, m1: A1NN, m2: T2NN| Holder1957 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1965() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|A1_SS|FT2_NNN",
            &rows3(|m0: FT0NNN, m1: A1SS, m2: FT2NNN| Holder1965 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1973() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|E1_NN|A2_SS",
            &rows3(|m0: FT0NNN, m1: E1NN, m2: A2SS| Holder1973 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder1981() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|E1_NN|P2_N",
            &rows3(|m0: FT0NNN, m1: E1NN, m2: P2N| Holder1981 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1989() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|E1_SS|FO2_NN",
            &rows3(|m0: FT0NNN, m1: E1SS, m2: FO2NN| Holder1989 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder1997() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|E1_SS|V2_NN",
            &rows3(|m0: FT0NNN, m1: E1SS, m2: V2NN| Holder1997 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2005() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|FO1_NN|O2_N",
            &rows3(|m0: FT0NNN, m1: FO1NN, m2: Option<OI2N>| Holder2005 { m0, m1, m2 }),
            &[
                ("Dot,-,None", 'E'),
                ("Dot,-,Some", 'E'),
                ("Rect,-,None", 'E'),
                ("Rect,-,Some", 'E'),
                ("Wrap,-,None", 'E'),
                ("Wrap,-,Some", 'E'),
            ],
        )
    }

    fn holder2013() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|FP1_NN|E2_NN",
            &rows3(|m0: FT0NNN, m1: FP1NN, m2: E2NN| Holder2013 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder2021() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|FP1_NN|P2_S",
            &rows3(|m0: FT0NNN, m1: FP1NN, m2: P2S| Holder2021 { m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder2029() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|FT1_NNN|FP2_NN",
            &rows3(|m0: FT0NNN, m1: FT1NNN, m2: FP2NN| Holder2029 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder2037() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|O1_N|A2_NN",
            &rows3(|m0: FT0NNN, m1: Option<OI1N>, m2: A2NN| Holder2037 { m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder2045() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|O1_N|O2_S",
            &rows3(|m0: FT0NNN, m1: Option<OI1N>, m2: Option<OI2S>| Holder2045 { m0, m1, m2 }),
            &[
                ("Dot,None,None", 'E'),
                ("Dot,None,Some", 'E'),
                ("Dot,Some,None", 'E'),
                ("Dot,Some,Some", 'E'),
                ("Rect,None,None", 'E'),
                ("Rect,None,Some", 'E'),
                ("Rect,Some,None", 'E'),
                ("Rect,Some,Some", 'E'),
                ("Wrap,None,None", 'E'),
                ("Wrap,None,Some", 'E'),
                ("Wrap,Some,None", 'E'),
                ("Wrap,Some,Some", 'E'),
            ],
        )
    }

    fn holder2053() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|O1_S|E2_SS",
            &rows3(|m0: FT0NNN, m1: Option<OI1S>, m2: E2SS| Holder2053 { m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder2061() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|O1_S|T2_NN",
            &rows3(|m0: FT0NNN, m1: Option<OI1S>, m2: T2NN| Holder2061 { m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder2069() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|P1_N|FT2_NNN",
            &rows3(|m0: FT0NNN, m1: P1N, m2: FT2NNN| Holder2069 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder2077() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|P1_S|A2_SS",
            &rows3(|m0: FT0NNN, m1: P1S, m2: A2SS| Holder2077 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder2085() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|P1_S|P2_N",
            &rows3(|m0: FT0NNN, m1: P1S, m2: P2N| Holder2085 { m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder2093() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|T1_NN|FO2_NN",
            &rows3(|m0: FT0NNN, m1: T1NN, m2: FO2NN| Holder2093 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder2101() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|T1_NN|V2_NN",
            &rows3(|m0: FT0NNN, m1: T1NN, m2: V2NN| Holder2101 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2109() -> Vec<String> {
        check(
            "main3:H3N0:FT0_NNN|V1_NN|O2_N",
            &rows3(|m0: FT0NNN, m1: V1NN, m2: Option<OI2N>| Holder2109 { m0, m1, m2 }),
            &[
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder2117() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|A1_NN|E2_NN nonstrict",
            &rows3(|m0: Option<OI0N>, m1: A1NN, m2: E2NN| Holder2117 { m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2125() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|A1_NN|O2_S nonstrict",
            &rows3(|m0: Option<OI0N>, m1: A1NN, m2: Option<OI2S>| Holder2125 { m0, m1, m2 }),
            &[
                ("None,Dot,None", 'E'),
                ("None,Dot,Some", 'E'),
                ("None,Rect,None", 'E'),
                ("None,Rect,Some", 'E'),
                ("None,Wrap,None", 'E'),
                ("None,Wrap,Some", 'E'),
                ("Some,Dot,None", 'E'),
                ("Some,Dot,Some", 'E'),
                ("Some,Rect,None", 'E'),
                ("Some,Rect,Some", 'E'),
                ("Some,Wrap,None", 'E'),
                ("Some,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder2133() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|A1_SS|A2_SS nonstrict",
            &rows3(|m0: Option<OI0N>, m1: A1SS, m2: A2SS| Holder2133 { m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2141() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|A1_SS|O2_N nonstrict",
            &rows3(|m0: Option<OI0N>, m1: A1SS, m2: Option<OI2N>| Holder2141 { m0, m1, m2 }),
            &[
                ("None,Dot,None", 'E'),
                ("None,Dot,Some", 'E'),
                ("None,Rect,None", 'E'),
                ("None,Rect,Some", 'E'),
                ("None,Wrap,None", 'E'),
                ("None,Wrap,Some", 'E'),
                ("Some,Dot,None", 'E'),
                ("Some,Dot,Some", 'E'),
                ("Some,Rect,None", 'E'),
                ("Some,Rect,Some", 'E'),
                ("Some,Wrap,None", 'E'),
                ("Some,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder2149() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|E1_NN|A2_NN nonstrict",
            &rows3(|m0: Option<OI0N>, m1: E1NN, m2: A2NN| Holder2149 { m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2157() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|E1_NN|FT2_NNN",
            &rows3(|m0: Option<OI0N>, m1: E1NN, m2: FT2NNN| Holder2157 { m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2165() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|E1_NN|V2_SS",
            &rows3(|m0: Option<OI0N>, m1: E1NN, m2: V2SS| Holder2165 { m0, m1, m2 }),
            &[
                ("None,Dot,Rect", 'R'),
                ("None,Dot,Wrap", 'R'),
                ("None,Rect,Rect", 'R'),
                ("None,Rect,Wrap", 'R'),
                ("None,Wrap,Rect", 'R'),
                ("None,Wrap,Wrap", 'R'),
                ("Some,Dot,Rect", 'R'),
                ("Some,Dot,Wrap", 'R'),
                ("Some,Rect,Rect", 'R'),
                ("Some,Rect,Wrap", 'R'),
                ("Some,Wrap,Rect", 'R'),
                ("Some,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder2173() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|E1_SS|FP2_SS",
            &rows3(|m0: Option<OI0N>, m1: E1SS, m2: FP2SS| Holder2173 { m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder2181() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|E1_SS|V2_NN",
            &rows3(|m0: Option<OI0N>, m1: E1SS, m2: V2NN| Holder2181 { m0, m1, m2 }),
            &[
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2189() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|FO1_NN|FT2_NNN",
            &rows3(|m0: Option<OI0N>, m1: FO1NN, m2: FT2NNN| Holder2189 { m0, m1, m2 }),
            &[
                ("None,-,Dot", 'E'),
                ("None,-,Rect", 'E'),
                ("None,-,Wrap", 'E'),
                ("Some,-,Dot", 'E'),
                ("Some,-,Rect", 'E'),
                ("Some,-,Wrap", 'E'),
            ],
        )
    }

    fn holder2197() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|FP1_NN|A2_SS",
            &rows3(|m0: Option<OI0N>, m1: FP1NN, m2: A2SS| Holder2197 { m0, m1, m2 }),
            &[
                ("None,-,Dot", 'E'),
                ("None,-,Rect", 'E'),
                ("None,-,Wrap", 'E'),
                ("Some,-,Dot", 'E'),
                ("Some,-,Rect", 'E'),
                ("Some,-,Wrap", 'E'),
            ],
        )
    }

    fn holder2205() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|FP1_NN|P2_N",
            &rows3(|m0: Option<OI0N>, m1: FP1NN, m2: P2N| Holder2205 { m0, m1, m2 }),
            &[("None,-,-", 'E'), ("Some,-,-", 'E')],
        )
    }

    fn holder2213() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|FT1_NNN|FO2_NN",
            &rows3(|m0: Option<OI0N>, m1: FT1NNN, m2: FO2NN| Holder2213 { m0, m1, m2 }),
            &[
                ("None,Dot,-", 'E'),
                ("None,Rect,-", 'E'),
                ("None,Wrap,-", 'E'),
                ("Some,Dot,-", 'E'),
                ("Some,Rect,-", 'E'),
                ("Some,Wrap,-", 'E'),
            ],
        )
    }

    fn holder2221() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|FT1_NNN|V2_NN",
            &rows3(|m0: Option<OI0N>, m1: FT1NNN, m2: V2NN| Holder2221 { m0, m1, m2 }),
            &[
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2229() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|O1_N|FP2_SS",
            &rows3(|m0: Option<OI0N>, m1: Option<OI1N>, m2: FP2SS| Holder2229 { m0, m1, m2 }),
            &[
                ("None,None,-", 'R'),
                ("None,Some,-", 'R'),
                ("Some,None,-", 'R'),
                ("Some,Some,-", 'R'),
            ],
        )
    }

    fn holder2237() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|O1_N|V2_NN",
            &rows3(|m0: Option<OI0N>, m1: Option<OI1N>, m2: V2NN| Holder2237 { m0, m1, m2 }),
            &[
                ("None,None,Rect", 'E'),
                ("None,None,Wrap", 'E'),
                ("None,Some,Rect", 'E'),
                ("None,Some,Wrap", 'E'),
                ("Some,None,Rect", 'E'),
                ("Some,None,Wrap", 'E'),
                ("Some,Some,Rect", 'E'),
                ("Some,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder2245() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|O1_S|FP2_NN",
            &rows3(|m0: Option<OI0N>, m1: Option<OI1S>, m2: FP2NN| Holder2245 { m0, m1, m2 }),
            &[
                ("None,None,-", 'E'),
                ("None,Some,-", 'E'),
                ("Some,None,-", 'E'),
                ("Some,Some,-", 'E'),
            ],
        )
    }

    fn holder2253() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|O1_S|T2_SS",
            &rows3(|m0: Option<OI0N>, m1: Option<OI1S>, m2: T2SS| Holder2253 { m0, m1, m2 }),
            &[
                ("None,None,Dot", 'R'),
                ("None,None,Rect", 'R'),
                ("None,None,Wrap", 'R'),
                ("None,Some,Dot", 'R'),
                ("None,Some,Rect", 'R'),
                ("None,Some,Wrap", 'R'),
                ("Some,None,Dot", 'R'),
                ("Some,None,Rect", 'R'),
                ("Some,None,Wrap", 'R'),
                ("Some,Some,Dot", 'R'),
                ("Some,Some,Rect", 'R'),
                ("Some,Some,Wrap", 'R'),
            ],
        )
    }

    fn holder2261() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|P1_N|FO2_SS",
            &rows3(|m0: Option<OI0N>, m1: P1N, m2: FO2SS| Holder2261 { m0, m1, m2 }),
            &[("None,-,-", 'R'), ("Some,-,-", 'R')],
        )
    }

    fn holder2269() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|P1_N|T2_NN",
            &rows3(|m0: Option<OI0N>, m1: P1N, m2: T2NN| Holder2269 { m0, m1, m2 }),
            &[
                ("None,-,Dot", 'E'),
                ("None,-,Rect", 'E'),
                ("None,-,Wrap", 'E'),
                ("Some,-,Dot", 'E'),
                ("Some,-,Rect", 'E'),
                ("Some,-,Wrap", 'E'),
            ],
        )
    }

    fn holder2277() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|P1_S|FO2_NN",
            &rows3(|m0: Option<OI0N>, m1: P1S, m2: FO2NN| Holder2277 { m0, m1, m2 }),
            &[("None,-,-", 'E'), ("Some,-,-", 'E')],
        )
    }

    fn holder2285() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|P1_S|P2_S nonstrict",
            &rows3(|m0: Option<OI0N>, m1: P1S, m2: P2S| Holder2285 { m0, m1, m2 }),
            &[("None,-,-", 'E'), ("Some,-,-", 'E')],
        )
    }

    fn holder2293() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|T1_NN|E2_SS",
            &rows3(|m0: Option<OI0N>, m1: T1NN, m2: E2SS| Holder2293 { m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2301() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|T1_NN|T2_NN",
            &rows3(|m0: Option<OI0N>, m1: T1NN, m2: T2NN| Holder2301 { m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2309() -> Vec<String> {
        check(
            "main3:H3N0:O0_N|V1_NN|FT2_NNN",
            &rows3(|m0: Option<OI0N>, m1: V1NN, m2: FT2NNN| Holder2309 { m0, m1, m2 }),
            &[
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2317() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|A1_NN|A2_SS nonstrict",
            &rows3(|m0: Option<OI0S>, m1: A1NN, m2: A2SS| Holder2317 { m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2325() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|A1_NN|O2_N nonstrict",
            &rows3(|m0: Option<OI0S>, m1: A1NN, m2: Option<OI2N>| Holder2325 { m0, m1, m2 }),
            &[
                ("None,Dot,None", 'E'),
                ("None,Dot,Some", 'E'),
                ("None,Rect,None", 'E'),
                ("None,Rect,Some", 'E'),
                ("None,Wrap,None", 'E'),
                ("None,Wrap,Some", 'E'),
                ("Some,Dot,None", 'E'),
                ("Some,Dot,Some", 'E'),
                ("Some,Rect,None", 'E'),
                ("Some,Rect,Some", 'E'),
                ("Some,Wrap,None", 'E'),
                ("Some,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder2333() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|A1_SS|A2_NN nonstrict",
            &rows3(|m0: Option<OI0S>, m1: A1SS, m2: A2NN| Holder2333 { m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2341() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|A1_SS|FT2_NNN",
            &rows3(|m0: Option<OI0S>, m1: A1SS, m2: FT2NNN| Holder2341 { m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2349() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|A1_SS|V2_SS",
            &rows3(|m0: Option<OI0S>, m1: A1SS, m2: V2SS| Holder2349 { m0, m1, m2 }),
            &[
                ("None,Dot,Rect", 'R'),
                ("None,Dot,Wrap", 'R'),
                ("None,Rect,Rect", 'R'),
                ("None,Rect,Wrap", 'R'),
                ("None,Wrap,Rect", 'R'),
                ("None,Wrap,Wrap", 'R'),
                ("Some,Dot,Rect", 'R'),
                ("Some,Dot,Wrap", 'R'),
                ("Some,Rect,Rect", 'R'),
                ("Some,Rect,Wrap", 'R'),
                ("Some,Wrap,Rect", 'R'),
                ("Some,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder2357() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|E1_NN|FP2_SS",
            &rows3(|m0: Option<OI0S>, m1: E1NN, m2: FP2SS| Holder2357 { m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder2365() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|E1_NN|V2_NN",
            &rows3(|m0: Option<OI0S>, m1: E1NN, m2: V2NN| Holder2365 { m0, m1, m2 }),
            &[
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2373() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|E1_SS|FP2_NN",
            &rows3(|m0: Option<OI0S>, m1: E1SS, m2: FP2NN| Holder2373 { m0, m1, m2 }),
            &[
                ("None,Dot,-", 'E'),
                ("None,Rect,-", 'E'),
                ("None,Wrap,-", 'E'),
                ("Some,Dot,-", 'E'),
                ("Some,Rect,-", 'E'),
                ("Some,Wrap,-", 'E'),
            ],
        )
    }

    fn holder2381() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|E1_SS|T2_SS",
            &rows3(|m0: Option<OI0S>, m1: E1SS, m2: T2SS| Holder2381 { m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'R'),
                ("None,Dot,Rect", 'R'),
                ("None,Dot,Wrap", 'R'),
                ("None,Rect,Dot", 'R'),
                ("None,Rect,Rect", 'R'),
                ("None,Rect,Wrap", 'R'),
                ("None,Wrap,Dot", 'R'),
                ("None,Wrap,Rect", 'R'),
                ("None,Wrap,Wrap", 'R'),
                ("Some,Dot,Dot", 'R'),
                ("Some,Dot,Rect", 'R'),
                ("Some,Dot,Wrap", 'R'),
                ("Some,Rect,Dot", 'R'),
                ("Some,Rect,Rect", 'R'),
                ("Some,Rect,Wrap", 'R'),
                ("Some,Wrap,Dot", 'R'),
                ("Some,Wrap,Rect", 'R'),
                ("Some,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder2389() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|FO1_NN|FP2_NN",
            &rows3(|m0: Option<OI0S>, m1: FO1NN, m2: FP2NN| Holder2389 { m0, m1, m2 }),
            &[("None,-,-", 'E'), ("Some,-,-", 'E')],
        )
    }

    fn holder2397() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|FP1_NN|A2_NN",
            &rows3(|m0: Option<OI0S>, m1: FP1NN, m2: A2NN| Holder2397 { m0, m1, m2 }),
            &[
                ("None,-,Dot", 'E'),
                ("None,-,Rect", 'E'),
                ("None,-,Wrap", 'E'),
                ("Some,-,Dot", 'E'),
                ("Some,-,Rect", 'E'),
                ("Some,-,Wrap", 'E'),
            ],
        )
    }

    fn holder2405() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|FP1_NN|O2_S",
            &rows3(|m0: Option<OI0S>, m1: FP1NN, m2: Option<OI2S>| Holder2405 { m0, m1, m2 }),
            &[
                ("None,-,None", 'E'),
                ("None,-,Some", 'E'),
                ("Some,-,None", 'E'),
                ("Some,-,Some", 'E'),
            ],
        )
    }

    fn holder2413() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|FT1_NNN|E2_SS",
            &rows3(|m0: Option<OI0S>, m1: FT1NNN, m2: E2SS| Holder2413 { m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2421() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|FT1_NNN|T2_NN",
            &rows3(|m0: Option<OI0S>, m1: FT1NNN, m2: T2NN| Holder2421 { m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2429() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|O1_N|FP2_NN",
            &rows3(|m0: Option<OI0S>, m1: Option<OI1N>, m2: FP2NN| Holder2429 { m0, m1, m2 }),
            &[
                ("None,None,-", 'E'),
                ("None,Some,-", 'E'),
                ("Some,None,-", 'E'),
                ("Some,Some,-", 'E'),
            ],
        )
    }

    fn holder2437() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|O1_N|T2_SS",
            &rows3(|m0: Option<OI0S>, m1: Option<OI1N>, m2: T2SS| Holder2437 { m0, m1, m2 }),
            &[
                ("None,None,Dot", 'R'),
                ("None,None,Rect", 'R'),
                ("None,None,Wrap", 'R'),
                ("None,Some,Dot", 'R'),
                ("None,Some,Rect", 'R'),
                ("None,Some,Wrap", 'R'),
                ("Some,None,Dot", 'R'),
                ("Some,None,Rect", 'R'),
                ("Some,None,Wrap", 'R'),
                ("Some,Some,Dot", 'R'),
                ("Some,Some,Rect", 'R'),
                ("Some,Some,Wrap", 'R'),
            ],
        )
    }

    fn holder2445() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|O1_S|FO2_SS",
            &rows3(|m0: Option<OI0S>, m1: Option<OI1S>, m2: FO2SS| Holder2445 { m0, m1, m2 }),
            &[
                ("None,None,-", 'R'),
                ("None,Some,-", 'R'),
                ("Some,None,-", 'R'),
                ("Some,Some,-", 'R'),
            ],
        )
    }

    fn holder2453() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|O1_S|T2_NN",
            &rows3(|m0: Option<OI0S>, m1: Option<OI1S>, m2: T2NN| Holder2453 { m0, m1, m2 }),
            &[
                ("None,None,Dot", 'E'),
                ("None,None,Rect", 'E'),
                ("None,None,Wrap", 'E'),
                ("None,Some,Dot", 'E'),
                ("None,Some,Rect", 'E'),
                ("None,Some,Wrap", 'E'),
                ("Some,None,Dot", 'E'),
                ("Some,None,Rect", 'E'),
                ("Some,None,Wrap", 'E'),
                ("Some,Some,Dot", 'E'),
                ("Some,Some,Rect", 'E'),
                ("Some,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder2461() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|P1_N|FO2_NN",
            &rows3(|m0: Option<OI0S>, m1: P1N, m2: FO2NN| Holder2461 { m0, m1, m2 }),
            &[("None,-,-", 'E'), ("Some,-,-", 'E')],
        )
    }

    fn holder2469() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|P1_N|P2_S nonstrict",
            &rows3(|m0: Option<OI0S>, m1: P1N, m2: P2S| Holder2469 { m0, m1, m2 }),
            &[("None,-,-", 'E'), ("Some,-,-", 'E')],
        )
    }

    fn holder2477() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|P1_S|E2_SS nonstrict",
            &rows3(|m0: Option<OI0S>, m1: P1S, m2: E2SS| Holder2477 { m0, m1, m2 }),
            &[
                ("None,-,Dot", 'E'),
                ("None,-,Rect", 'E'),
                ("None,-,Wrap", 'E'),
                ("Some,-,Dot", 'E'),
                ("Some,-,Rect", 'E'),
                ("Some,-,Wrap", 'E'),
            ],
        )
    }

    fn holder2485() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|P1_S|P2_N nonstrict",
            &rows3(|m0: Option<OI0S>, m1: P1S, m2: P2N| Holder2485 { m0, m1, m2 }),
            &[("None,-,-", 'E'), ("Some,-,-", 'E')],
        )
    }

    fn holder2493() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|T1_NN|E2_NN",
            &rows3(|m0: Option<OI0S>, m1: T1NN, m2: E2NN| Holder2493 { m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2501() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|T1_NN|P2_S",
            &rows3(|m0: Option<OI0S>, m1: T1NN, m2: P2S| Holder2501 { m0, m1, m2 }),
            &[
                ("None,Dot,-", 'E'),
                ("None,Rect,-", 'E'),
                ("None,Wrap,-", 'E'),
                ("Some,Dot,-", 'E'),
                ("Some,Rect,-", 'E'),
                ("Some,Wrap,-", 'E'),
            ],
        )
    }

    fn holder2509() -> Vec<String> {
        check(
            "main3:H3N0:O0_S|V1_NN|FP2_NN",
            &rows3(|m0: Option<OI0S>, m1: V1NN, m2: FP2NN| Holder2509 { m0, m1, m2 }),
            &[
                ("None,Rect,-", 'E'),
                ("None,Wrap,-", 'E'),
                ("Some,Rect,-", 'E'),
                ("Some,Wrap,-", 'E'),
            ],
        )
    }

    fn holder2517() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|A1_NN|A2_NN nonstrict",
            &rows3(|m0: P0N, m1: A1NN, m2: A2NN| Holder2517 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2525() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|A1_NN|FT2_NNN",
            &rows3(|m0: P0N, m1: A1NN, m2: FT2NNN| Holder2525 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2533() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|A1_NN|V2_SS",
            &rows3(|m0: P0N, m1: A1NN, m2: V2SS| Holder2533 { m0, m1, m2 }),
            &[
                ("-,Dot,Rect", 'R'),
                ("-,Dot,Wrap", 'R'),
                ("-,Rect,Rect", 'R'),
                ("-,Rect,Wrap", 'R'),
                ("-,Wrap,Rect", 'R'),
                ("-,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder2541() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|A1_SS|FP2_SS",
            &rows3(|m0: P0N, m1: A1SS, m2: FP2SS| Holder2541 { m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder2549() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|A1_SS|V2_NN",
            &rows3(|m0: P0N, m1: A1SS, m2: V2NN| Holder2549 { m0, m1, m2 }),
            &[
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2557() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|E1_NN|FP2_NN",
            &rows3(|m0: P0N, m1: E1NN, m2: FP2NN| Holder2557 { m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder2565() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|E1_NN|T2_SS",
            &rows3(|m0: P0N, m1: E1NN, m2: T2SS| Holder2565 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'R'),
                ("-,Dot,Rect", 'R'),
                ("-,Dot,Wrap", 'R'),
                ("-,Rect,Dot", 'R'),
                ("-,Rect,Rect", 'R'),
                ("-,Rect,Wrap", 'R'),
                ("-,Wrap,Dot", 'R'),
                ("-,Wrap,Rect", 'R'),
                ("-,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder2573() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|E1_SS|FO2_SS",
            &rows3(|m0: P0N, m1: E1SS, m2: FO2SS| Holder2573 { m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder2581() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|E1_SS|T2_NN",
            &rows3(|m0: P0N, m1: E1SS, m2: T2NN| Holder2581 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2589() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|FO1_NN|FO2_NN",
            &rows3(|m0: P0N, m1: FO1NN, m2: FO2NN| Holder2589 { m0, m1, m2 }),
            &[("-,-,-", 'E')],
        )
    }

    fn holder2597() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|FO1_NN|V2_NN",
            &rows3(|m0: P0N, m1: FO1NN, m2: V2NN| Holder2597 { m0, m1, m2 }),
            &[("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder2605() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|FP1_NN|O2_N",
            &rows3(|m0: P0N, m1: FP1NN, m2: Option<OI2N>| Holder2605 { m0, m1, m2 }),
            &[("-,-,None", 'E'), ("-,-,Some", 'E')],
        )
    }

    fn holder2613() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|FT1_NNN|E2_NN",
            &rows3(|m0: P0N, m1: FT1NNN, m2: E2NN| Holder2613 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2621() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|FT1_NNN|P2_S",
            &rows3(|m0: P0N, m1: FT1NNN, m2: P2S| Holder2621 { m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder2629() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|O1_N|FO2_SS",
            &rows3(|m0: P0N, m1: Option<OI1N>, m2: FO2SS| Holder2629 { m0, m1, m2 }),
            &[("-,None,-", 'R'), ("-,Some,-", 'R')],
        )
    }

    fn holder2637() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|O1_N|T2_NN",
            &rows3(|m0: P0N, m1: Option<OI1N>, m2: T2NN| Holder2637 { m0, m1, m2 }),
            &[
                ("-,None,Dot", 'E'),
                ("-,None,Rect", 'E'),
                ("-,None,Wrap", 'E'),
                ("-,Some,Dot", 'E'),
                ("-,Some,Rect", 'E'),
                ("-,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder2645() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|O1_S|FO2_NN",
            &rows3(|m0: P0N, m1: Option<OI1S>, m2: FO2NN| Holder2645 { m0, m1, m2 }),
            &[("-,None,-", 'E'), ("-,Some,-", 'E')],
        )
    }

    fn holder2653() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|O1_S|P2_S nonstrict",
            &rows3(|m0: P0N, m1: Option<OI1S>, m2: P2S| Holder2653 { m0, m1, m2 }),
            &[("-,None,-", 'E'), ("-,Some,-", 'E')],
        )
    }

    fn holder2661() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|P1_N|E2_SS nonstrict",
            &rows3(|m0: P0N, m1: P1N, m2: E2SS| Holder2661 { m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder2669() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|P1_N|P2_N nonstrict",
            &rows3(|m0: P0N, m1: P1N, m2: P2N| Holder2669 { m0, m1, m2 }),
            &[("-,-,-", 'E')],
        )
    }

    fn holder2677() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|P1_S|E2_NN nonstrict",
            &rows3(|m0: P0N, m1: P1S, m2: E2NN| Holder2677 { m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder2685() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|P1_S|O2_S nonstrict",
            &rows3(|m0: P0N, m1: P1S, m2: Option<OI2S>| Holder2685 { m0, m1, m2 }),
            &[("-,-,None", 'E'), ("-,-,Some", 'E')],
        )
    }

    fn holder2693() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|T1_NN|A2_SS",
            &rows3(|m0: P0N, m1: T1NN, m2: A2SS| Holder2693 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2701() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|T1_NN|P2_N",
            &rows3(|m0: P0N, m1: T1NN, m2: P2N| Holder2701 { m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder2709() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|V1_NN|FO2_NN",
            &rows3(|m0: P0N, m1: V1NN, m2: FO2NN| Holder2709 { m0, m1, m2 }),
            &[("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder2717() -> Vec<String> {
        check(
            "main3:H3N0:P0_N|V1_NN|V2_NN",
            &rows3(|m0: P0N, m1: V1NN, m2: V2NN| Holder2717 { m0, m1, m2 }),
            &[
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2725() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|A1_NN|FP2_SS",
            &rows3(|m0: P0S, m1: A1NN, m2: FP2SS| Holder2725 { m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder2733() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|A1_NN|V2_NN",
            &rows3(|m0: P0S, m1: A1NN, m2: V2NN| Holder2733 { m0, m1, m2 }),
            &[
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2741() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|A1_SS|FP2_NN",
            &rows3(|m0: P0S, m1: A1SS, m2: FP2NN| Holder2741 { m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder2749() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|A1_SS|T2_SS",
            &rows3(|m0: P0S, m1: A1SS, m2: T2SS| Holder2749 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'R'),
                ("-,Dot,Rect", 'R'),
                ("-,Dot,Wrap", 'R'),
                ("-,Rect,Dot", 'R'),
                ("-,Rect,Rect", 'R'),
                ("-,Rect,Wrap", 'R'),
                ("-,Wrap,Dot", 'R'),
                ("-,Wrap,Rect", 'R'),
                ("-,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder2757() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|E1_NN|FO2_SS",
            &rows3(|m0: P0S, m1: E1NN, m2: FO2SS| Holder2757 { m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder2765() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|E1_NN|T2_NN",
            &rows3(|m0: P0S, m1: E1NN, m2: T2NN| Holder2765 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2773() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|E1_SS|FO2_NN",
            &rows3(|m0: P0S, m1: E1SS, m2: FO2NN| Holder2773 { m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder2781() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|E1_SS|P2_S nonstrict",
            &rows3(|m0: P0S, m1: E1SS, m2: P2S| Holder2781 { m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder2789() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|FO1_NN|E2_SS",
            &rows3(|m0: P0S, m1: FO1NN, m2: E2SS| Holder2789 { m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder2797() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|FO1_NN|T2_NN",
            &rows3(|m0: P0S, m1: FO1NN, m2: T2NN| Holder2797 { m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder2805() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|FP1_NN|FT2_NNN",
            &rows3(|m0: P0S, m1: FP1NN, m2: FT2NNN| Holder2805 { m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder2813() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|FT1_NNN|A2_SS",
            &rows3(|m0: P0S, m1: FT1NNN, m2: A2SS| Holder2813 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2821() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|FT1_NNN|P2_N",
            &rows3(|m0: P0S, m1: FT1NNN, m2: P2N| Holder2821 { m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder2829() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|O1_N|FO2_NN",
            &rows3(|m0: P0S, m1: Option<OI1N>, m2: FO2NN| Holder2829 { m0, m1, m2 }),
            &[("-,None,-", 'E'), ("-,Some,-", 'E')],
        )
    }

    fn holder2837() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|O1_N|P2_S nonstrict",
            &rows3(|m0: P0S, m1: Option<OI1N>, m2: P2S| Holder2837 { m0, m1, m2 }),
            &[("-,None,-", 'E'), ("-,Some,-", 'E')],
        )
    }

    fn holder2845() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|O1_S|E2_SS nonstrict",
            &rows3(|m0: P0S, m1: Option<OI1S>, m2: E2SS| Holder2845 { m0, m1, m2 }),
            &[
                ("-,None,Dot", 'E'),
                ("-,None,Rect", 'E'),
                ("-,None,Wrap", 'E'),
                ("-,Some,Dot", 'E'),
                ("-,Some,Rect", 'E'),
                ("-,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder2853() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|O1_S|P2_N nonstrict",
            &rows3(|m0: P0S, m1: Option<OI1S>, m2: P2N| Holder2853 { m0, m1, m2 }),
            &[("-,None,-", 'E'), ("-,Some,-", 'E')],
        )
    }

    fn holder2861() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|P1_N|E2_NN nonstrict",
            &rows3(|m0: P0S, m1: P1N, m2: E2NN| Holder2861 { m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder2869() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|P1_N|O2_S nonstrict",
            &rows3(|m0: P0S, m1: P1N, m2: Option<OI2S>| Holder2869 { m0, m1, m2 }),
            &[("-,-,None", 'E'), ("-,-,Some", 'E')],
        )
    }

    fn holder2877() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|P1_S|A2_SS nonstrict",
            &rows3(|m0: P0S, m1: P1S, m2: A2SS| Holder2877 { m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder2885() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|P1_S|O2_N nonstrict",
            &rows3(|m0: P0S, m1: P1S, m2: Option<OI2N>| Holder2885 { m0, m1, m2 }),
            &[("-,-,None", 'E'), ("-,-,Some", 'E')],
        )
    }

    fn holder2893() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|T1_NN|A2_NN",
            &rows3(|m0: P0S, m1: T1NN, m2: A2NN| Holder2893 { m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2901() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|T1_NN|O2_S",
            &rows3(|m0: P0S, m1: T1NN, m2: Option<OI2S>| Holder2901 { m0, m1, m2 }),
            &[
                ("-,Dot,None", 'E'),
                ("-,Dot,Some", 'E'),
                ("-,Rect,None", 'E'),
                ("-,Rect,Some", 'E'),
                ("-,Wrap,None", 'E'),
                ("-,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder2909() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|V1_NN|E2_SS",
            &rows3(|m0: P0S, m1: V1NN, m2: E2SS| Holder2909 { m0, m1, m2 }),
            &[
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2917() -> Vec<String> {
        check(
            "main3:H3N0:P0_S|V1_NN|T2_NN",
            &rows3(|m0: P0S, m1: V1NN, m2: T2NN| Holder2917 { m0, m1, m2 }),
            &[
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2925() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|A1_NN|FT2_NNN",
            &rows3(|m0: T0NN, m1: A1NN, m2: FT2NNN| Holder2925 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2933() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|A1_SS|A2_SS",
            &rows3(|m0: T0NN, m1: A1SS, m2: A2SS| Holder2933 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2941() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|A1_SS|P2_N",
            &rows3(|m0: T0NN, m1: A1SS, m2: P2N| Holder2941 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder2949() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|E1_NN|FO2_NN",
            &rows3(|m0: T0NN, m1: E1NN, m2: FO2NN| Holder2949 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder2957() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|E1_NN|V2_NN",
            &rows3(|m0: T0NN, m1: E1NN, m2: V2NN| Holder2957 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder2965() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|E1_SS|O2_N",
            &rows3(|m0: T0NN, m1: E1SS, m2: Option<OI2N>| Holder2965 { m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder2973() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|FO1_NN|E2_NN",
            &rows3(|m0: T0NN, m1: FO1NN, m2: E2NN| Holder2973 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder2981() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|FO1_NN|P2_S",
            &rows3(|m0: T0NN, m1: FO1NN, m2: P2S| Holder2981 { m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder2989() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|FP1_NN|FP2_NN",
            &rows3(|m0: T0NN, m1: FP1NN, m2: FP2NN| Holder2989 { m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder2997() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|FT1_NNN|A2_NN",
            &rows3(|m0: T0NN, m1: FT1NNN, m2: A2NN| Holder2997 { m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3005() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|FT1_NNN|O2_S",
            &rows3(|m0: T0NN, m1: FT1NNN, m2: Option<OI2S>| Holder3005 { m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder3013() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|O1_N|E2_SS",
            &rows3(|m0: T0NN, m1: Option<OI1N>, m2: E2SS| Holder3013 { m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder3021() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|O1_N|T2_NN",
            &rows3(|m0: T0NN, m1: Option<OI1N>, m2: T2NN| Holder3021 { m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder3029() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|O1_S|FT2_NNN",
            &rows3(|m0: T0NN, m1: Option<OI1S>, m2: FT2NNN| Holder3029 { m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder3037() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|P1_N|A2_SS",
            &rows3(|m0: T0NN, m1: P1N, m2: A2SS| Holder3037 { m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3045() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|P1_N|P2_N",
            &rows3(|m0: T0NN, m1: P1N, m2: P2N| Holder3045 { m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder3053() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|P1_S|FO2_NN",
            &rows3(|m0: T0NN, m1: P1S, m2: FO2NN| Holder3053 { m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder3061() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|P1_S|V2_NN",
            &rows3(|m0: T0NN, m1: P1S, m2: V2NN| Holder3061 { m0, m1, m2 }),
            &[
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3069() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|T1_NN|O2_N",
            &rows3(|m0: T0NN, m1: T1NN, m2: Option<OI2N>| Holder3069 { m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder3077() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|V1_NN|E2_NN",
            &rows3(|m0: T0NN, m1: V1NN, m2: E2NN| Holder3077 { m0, m1, m2 }),
            &[
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3085() -> Vec<String> {
        check(
            "main3:H3N0:T0_NN|V1_NN|P2_S",
            &rows3(|m0: T0NN, m1: V1NN, m2: P2S| Holder3085 { m0, m1, m2 }),
            &[
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder3093() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|A1_NN|FP2_NN",
            &rows3(|m0: V0NN, m1: A1NN, m2: FP2NN| Holder3093 { m0, m1, m2 }),
            &[
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder3101() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|A1_SS|A2_NN",
            &rows3(|m0: V0NN, m1: A1SS, m2: A2NN| Holder3101 { m0, m1, m2 }),
            &[
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3109() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|A1_SS|O2_S",
            &rows3(|m0: V0NN, m1: A1SS, m2: Option<OI2S>| Holder3109 { m0, m1, m2 }),
            &[
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder3117() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|E1_NN|E2_SS",
            &rows3(|m0: V0NN, m1: E1NN, m2: E2SS| Holder3117 { m0, m1, m2 }),
            &[
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3125() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|E1_NN|T2_NN",
            &rows3(|m0: V0NN, m1: E1NN, m2: T2NN| Holder3125 { m0, m1, m2 }),
            &[
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3133() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|E1_SS|FT2_NNN",
            &rows3(|m0: V0NN, m1: E1SS, m2: FT2NNN| Holder3133 { m0, m1, m2 }),
            &[
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3141() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|FO1_NN|A2_SS",
            &rows3(|m0: V0NN, m1: FO1NN, m2: A2SS| Holder3141 { m0, m1, m2 }),
            &[
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3149() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|FO1_NN|P2_N",
            &rows3(|m0: V0NN, m1: FO1NN, m2: P2N| Holder3149 { m0, m1, m2 }),
            &[("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder3157() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|FP1_NN|FO2_NN",
            &rows3(|m0: V0NN, m1: FP1NN, m2: FO2NN| Holder3157 { m0, m1, m2 }),
            &[("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder3165() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|FP1_NN|V2_NN",
            &rows3(|m0: V0NN, m1: FP1NN, m2: V2NN| Holder3165 { m0, m1, m2 }),
            &[
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3173() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|FT1_NNN|O2_N",
            &rows3(|m0: V0NN, m1: FT1NNN, m2: Option<OI2N>| Holder3173 { m0, m1, m2 }),
            &[
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder3181() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|O1_N|E2_NN",
            &rows3(|m0: V0NN, m1: Option<OI1N>, m2: E2NN| Holder3181 { m0, m1, m2 }),
            &[
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder3189() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|O1_N|P2_S",
            &rows3(|m0: V0NN, m1: Option<OI1N>, m2: P2S| Holder3189 { m0, m1, m2 }),
            &[
                ("Rect,None,-", 'E'),
                ("Rect,Some,-", 'E'),
                ("Wrap,None,-", 'E'),
                ("Wrap,Some,-", 'E'),
            ],
        )
    }

    fn holder3197() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|O1_S|FP2_NN",
            &rows3(|m0: V0NN, m1: Option<OI1S>, m2: FP2NN| Holder3197 { m0, m1, m2 }),
            &[
                ("Rect,None,-", 'E'),
                ("Rect,Some,-", 'E'),
                ("Wrap,None,-", 'E'),
                ("Wrap,Some,-", 'E'),
            ],
        )
    }

    fn holder3205() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|P1_N|A2_NN",
            &rows3(|m0: V0NN, m1: P1N, m2: A2NN| Holder3205 { m0, m1, m2 }),
            &[
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3213() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|P1_N|O2_S",
            &rows3(|m0: V0NN, m1: P1N, m2: Option<OI2S>| Holder3213 { m0, m1, m2 }),
            &[
                ("Rect,-,None", 'E'),
                ("Rect,-,Some", 'E'),
                ("Wrap,-,None", 'E'),
                ("Wrap,-,Some", 'E'),
            ],
        )
    }

    fn holder3221() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|P1_S|E2_SS",
            &rows3(|m0: V0NN, m1: P1S, m2: E2SS| Holder3221 { m0, m1, m2 }),
            &[
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3229() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|P1_S|T2_NN",
            &rows3(|m0: V0NN, m1: P1S, m2: T2NN| Holder3229 { m0, m1, m2 }),
            &[
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3237() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|T1_NN|FT2_NNN",
            &rows3(|m0: V0NN, m1: T1NN, m2: FT2NNN| Holder3237 { m0, m1, m2 }),
            &[
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3245() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|V1_NN|A2_SS",
            &rows3(|m0: V0NN, m1: V1NN, m2: A2SS| Holder3245 { m0, m1, m2 }),
            &[
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3253() -> Vec<String> {
        check(
            "main3:H3N0:V0_NN|V1_NN|P2_N",
            &rows3(|m0: V0NN, m1: V1NN, m2: P2N| Holder3253 { m0, m1, m2 }),
            &[
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder3261() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|A1_NN|FO2_NN",
            &rows3(|m0: A0NN, m1: A1NN, m2: FO2NN| Holder3261 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder3269() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|A1_NN|P2_S nonstrict",
            &rows3(|m0: A0NN, m1: A1NN, m2: P2S| Holder3269 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder3277() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|A1_SS|E2_SS nonstrict",
            &rows3(|m0: A0NN, m1: A1SS, m2: E2SS| Holder3277 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3285() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|A1_SS|P2_N nonstrict",
            &rows3(|m0: A0NN, m1: A1SS, m2: P2N| Holder3285 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder3293() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|E1_NN|E2_NN nonstrict",
            &rows3(|m0: A0NN, m1: E1NN, m2: E2NN| Holder3293 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3301() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|E1_NN|O2_S nonstrict",
            &rows3(|m0: A0NN, m1: E1NN, m2: Option<OI2S>| Holder3301 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder3309() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|E1_SS|A2_SS nonstrict",
            &rows3(|m0: A0NN, m1: E1SS, m2: A2SS| Holder3309 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3317() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|E1_SS|O2_N nonstrict",
            &rows3(|m0: A0NN, m1: E1SS, m2: Option<OI2N>| Holder3317 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder3325() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|FO1_NN|A2_NN",
            &rows3(|m0: A0NN, m1: FO1NN, m2: A2NN| Holder3325 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3333() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|FO1_NN|O2_S",
            &rows3(|m0: A0NN, m1: FO1NN, m2: Option<OI2S>| Holder3333 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,None", 'E'),
                ("Dot,-,Some", 'E'),
                ("Rect,-,None", 'E'),
                ("Rect,-,Some", 'E'),
                ("Wrap,-,None", 'E'),
                ("Wrap,-,Some", 'E'),
            ],
        )
    }

    fn holder3341() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|FP1_NN|E2_SS",
            &rows3(|m0: A0NN, m1: FP1NN, m2: E2SS| Holder3341 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3349() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|FP1_NN|T2_NN",
            &rows3(|m0: A0NN, m1: FP1NN, m2: T2NN| Holder3349 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3357() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|FT1_NNN|FT2_NNN",
            &rows3(|m0: A0NN, m1: FT1NNN, m2: FT2NNN| Holder3357 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3365() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|O1_N|A2_SS nonstrict",
            &rows3(|m0: A0NN, m1: Option<OI1N>, m2: A2SS| Holder3365 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder3373() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|O1_N|O2_N nonstrict",
            &rows3(|m0: A0NN, m1: Option<OI1N>, m2: Option<OI2N>| Holder3373 {
                own: 7,
                m0,
                m1,
                m2,
            }),
            &[
                ("Dot,None,None", 'E'),
                ("Dot,None,Some", 'E'),
                ("Dot,Some,None", 'E'),
                ("Dot,Some,Some", 'E'),
                ("Rect,None,None", 'E'),
                ("Rect,None,Some", 'E'),
                ("Rect,Some,None", 'E'),
                ("Rect,Some,Some", 'E'),
                ("Wrap,None,None", 'E'),
                ("Wrap,None,Some", 'E'),
                ("Wrap,Some,None", 'E'),
                ("Wrap,Some,Some", 'E'),
            ],
        )
    }

    fn holder3381() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|O1_S|A2_NN nonstrict",
            &rows3(|m0: A0NN, m1: Option<OI1S>, m2: A2NN| Holder3381 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder3389() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|O1_S|FT2_NNN",
            &rows3(|m0: A0NN, m1: Option<OI1S>, m2: FT2NNN| Holder3389 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder3397() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|O1_S|V2_SS",
            &rows3(|m0: A0NN, m1: Option<OI1S>, m2: V2SS| Holder3397 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Rect", 'R'),
                ("Dot,None,Wrap", 'R'),
                ("Dot,Some,Rect", 'R'),
                ("Dot,Some,Wrap", 'R'),
                ("Rect,None,Rect", 'R'),
                ("Rect,None,Wrap", 'R'),
                ("Rect,Some,Rect", 'R'),
                ("Rect,Some,Wrap", 'R'),
                ("Wrap,None,Rect", 'R'),
                ("Wrap,None,Wrap", 'R'),
                ("Wrap,Some,Rect", 'R'),
                ("Wrap,Some,Wrap", 'R'),
            ],
        )
    }

    fn holder3405() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|P1_N|FP2_SS",
            &rows3(|m0: A0NN, m1: P1N, m2: FP2SS| Holder3405 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder3413() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|P1_N|V2_NN",
            &rows3(|m0: A0NN, m1: P1N, m2: V2NN| Holder3413 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3421() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|P1_S|FP2_NN",
            &rows3(|m0: A0NN, m1: P1S, m2: FP2NN| Holder3421 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder3429() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|P1_S|T2_SS",
            &rows3(|m0: A0NN, m1: P1S, m2: T2SS| Holder3429 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'R'),
                ("Dot,-,Rect", 'R'),
                ("Dot,-,Wrap", 'R'),
                ("Rect,-,Dot", 'R'),
                ("Rect,-,Rect", 'R'),
                ("Rect,-,Wrap", 'R'),
                ("Wrap,-,Dot", 'R'),
                ("Wrap,-,Rect", 'R'),
                ("Wrap,-,Wrap", 'R'),
            ],
        )
    }

    fn holder3437() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|T1_NN|FP2_NN",
            &rows3(|m0: A0NN, m1: T1NN, m2: FP2NN| Holder3437 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder3445() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|V1_NN|A2_NN",
            &rows3(|m0: A0NN, m1: V1NN, m2: A2NN| Holder3445 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3453() -> Vec<String> {
        check(
            "main3:H3N1:A0_NN|V1_NN|O2_S",
            &rows3(|m0: A0NN, m1: V1NN, m2: Option<OI2S>| Holder3453 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder3461() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|A1_NN|E2_SS nonstrict",
            &rows3(|m0: A0SS, m1: A1NN, m2: E2SS| Holder3461 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3469() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|A1_NN|P2_N nonstrict",
            &rows3(|m0: A0SS, m1: A1NN, m2: P2N| Holder3469 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder3477() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|A1_SS|E2_NN nonstrict",
            &rows3(|m0: A0SS, m1: A1SS, m2: E2NN| Holder3477 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3485() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|A1_SS|O2_S nonstrict",
            &rows3(|m0: A0SS, m1: A1SS, m2: Option<OI2S>| Holder3485 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder3493() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|E1_NN|A2_SS nonstrict",
            &rows3(|m0: A0SS, m1: E1NN, m2: A2SS| Holder3493 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3501() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|E1_NN|O2_N nonstrict",
            &rows3(|m0: A0SS, m1: E1NN, m2: Option<OI2N>| Holder3501 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder3509() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|E1_SS|A2_NN nonstrict",
            &rows3(|m0: A0SS, m1: E1SS, m2: A2NN| Holder3509 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3517() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|E1_SS|FT2_NNN",
            &rows3(|m0: A0SS, m1: E1SS, m2: FT2NNN| Holder3517 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3525() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|E1_SS|V2_SS",
            &rows3(|m0: A0SS, m1: E1SS, m2: V2SS| Holder3525 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'R'),
                ("Dot,Dot,Wrap", 'R'),
                ("Dot,Rect,Rect", 'R'),
                ("Dot,Rect,Wrap", 'R'),
                ("Dot,Wrap,Rect", 'R'),
                ("Dot,Wrap,Wrap", 'R'),
                ("Rect,Dot,Rect", 'R'),
                ("Rect,Dot,Wrap", 'R'),
                ("Rect,Rect,Rect", 'R'),
                ("Rect,Rect,Wrap", 'R'),
                ("Rect,Wrap,Rect", 'R'),
                ("Rect,Wrap,Wrap", 'R'),
                ("Wrap,Dot,Rect", 'R'),
                ("Wrap,Dot,Wrap", 'R'),
                ("Wrap,Rect,Rect", 'R'),
                ("Wrap,Rect,Wrap", 'R'),
                ("Wrap,Wrap,Rect", 'R'),
                ("Wrap,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder3533() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|FO1_NN|O2_N",
            &rows3(|m0: A0SS, m1: FO1NN, m2: Option<OI2N>| Holder3533 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,None", 'E'),
                ("Dot,-,Some", 'E'),
                ("Rect,-,None", 'E'),
                ("Rect,-,Some", 'E'),
                ("Wrap,-,None", 'E'),
                ("Wrap,-,Some", 'E'),
            ],
        )
    }

    fn holder3541() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|FP1_NN|E2_NN",
            &rows3(|m0: A0SS, m1: FP1NN, m2: E2NN| Holder3541 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3549() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|FP1_NN|P2_S",
            &rows3(|m0: A0SS, m1: FP1NN, m2: P2S| Holder3549 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder3557() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|FT1_NNN|FP2_NN",
            &rows3(|m0: A0SS, m1: FT1NNN, m2: FP2NN| Holder3557 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder3565() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|O1_N|A2_NN nonstrict",
            &rows3(|m0: A0SS, m1: Option<OI1N>, m2: A2NN| Holder3565 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder3573() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|O1_N|FT2_NNN",
            &rows3(|m0: A0SS, m1: Option<OI1N>, m2: FT2NNN| Holder3573 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder3581() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|O1_N|V2_SS",
            &rows3(|m0: A0SS, m1: Option<OI1N>, m2: V2SS| Holder3581 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Rect", 'R'),
                ("Dot,None,Wrap", 'R'),
                ("Dot,Some,Rect", 'R'),
                ("Dot,Some,Wrap", 'R'),
                ("Rect,None,Rect", 'R'),
                ("Rect,None,Wrap", 'R'),
                ("Rect,Some,Rect", 'R'),
                ("Rect,Some,Wrap", 'R'),
                ("Wrap,None,Rect", 'R'),
                ("Wrap,None,Wrap", 'R'),
                ("Wrap,Some,Rect", 'R'),
                ("Wrap,Some,Wrap", 'R'),
            ],
        )
    }

    fn holder3589() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|O1_S|FP2_SS",
            &rows3(|m0: A0SS, m1: Option<OI1S>, m2: FP2SS| Holder3589 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder3597() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|O1_S|V2_NN",
            &rows3(|m0: A0SS, m1: Option<OI1S>, m2: V2NN| Holder3597 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder3605() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|P1_N|FP2_NN",
            &rows3(|m0: A0SS, m1: P1N, m2: FP2NN| Holder3605 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder3613() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|P1_N|T2_SS",
            &rows3(|m0: A0SS, m1: P1N, m2: T2SS| Holder3613 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'R'),
                ("Dot,-,Rect", 'R'),
                ("Dot,-,Wrap", 'R'),
                ("Rect,-,Dot", 'R'),
                ("Rect,-,Rect", 'R'),
                ("Rect,-,Wrap", 'R'),
                ("Wrap,-,Dot", 'R'),
                ("Wrap,-,Rect", 'R'),
                ("Wrap,-,Wrap", 'R'),
            ],
        )
    }

    fn holder3621() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|P1_S|FO2_SS",
            &rows3(|m0: A0SS, m1: P1S, m2: FO2SS| Holder3621 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder3629() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|P1_S|T2_NN",
            &rows3(|m0: A0SS, m1: P1S, m2: T2NN| Holder3629 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3637() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|T1_NN|FO2_NN",
            &rows3(|m0: A0SS, m1: T1NN, m2: FO2NN| Holder3637 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder3645() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|T1_NN|V2_NN",
            &rows3(|m0: A0SS, m1: T1NN, m2: V2NN| Holder3645 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3653() -> Vec<String> {
        check(
            "main3:H3N1:A0_SS|V1_NN|O2_N",
            &rows3(|m0: A0SS, m1: V1NN, m2: Option<OI2N>| Holder3653 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder3661() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|A1_NN|E2_NN nonstrict",
            &rows3(|m0: E0NN, m1: A1NN, m2: E2NN| Holder3661 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3669() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|A1_NN|O2_S nonstrict",
            &rows3(|m0: E0NN, m1: A1NN, m2: Option<OI2S>| Holder3669 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder3677() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|A1_SS|A2_SS nonstrict",
            &rows3(|m0: E0NN, m1: A1SS, m2: A2SS| Holder3677 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3685() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|A1_SS|O2_N nonstrict",
            &rows3(|m0: E0NN, m1: A1SS, m2: Option<OI2N>| Holder3685 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder3693() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|E1_NN|A2_NN nonstrict",
            &rows3(|m0: E0NN, m1: E1NN, m2: A2NN| Holder3693 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3701() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|E1_NN|FT2_NNN",
            &rows3(|m0: E0NN, m1: E1NN, m2: FT2NNN| Holder3701 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3709() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|E1_NN|V2_SS",
            &rows3(|m0: E0NN, m1: E1NN, m2: V2SS| Holder3709 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'R'),
                ("Dot,Dot,Wrap", 'R'),
                ("Dot,Rect,Rect", 'R'),
                ("Dot,Rect,Wrap", 'R'),
                ("Dot,Wrap,Rect", 'R'),
                ("Dot,Wrap,Wrap", 'R'),
                ("Rect,Dot,Rect", 'R'),
                ("Rect,Dot,Wrap", 'R'),
                ("Rect,Rect,Rect", 'R'),
                ("Rect,Rect,Wrap", 'R'),
                ("Rect,Wrap,Rect", 'R'),
                ("Rect,Wrap,Wrap", 'R'),
                ("Wrap,Dot,Rect", 'R'),
                ("Wrap,Dot,Wrap", 'R'),
                ("Wrap,Rect,Rect", 'R'),
                ("Wrap,Rect,Wrap", 'R'),
                ("Wrap,Wrap,Rect", 'R'),
                ("Wrap,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder3717() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|E1_SS|FP2_SS",
            &rows3(|m0: E0NN, m1: E1SS, m2: FP2SS| Holder3717 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder3725() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|E1_SS|V2_NN",
            &rows3(|m0: E0NN, m1: E1SS, m2: V2NN| Holder3725 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3733() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|FO1_NN|FT2_NNN",
            &rows3(|m0: E0NN, m1: FO1NN, m2: FT2NNN| Holder3733 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3741() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|FP1_NN|A2_SS",
            &rows3(|m0: E0NN, m1: FP1NN, m2: A2SS| Holder3741 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3749() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|FP1_NN|P2_N",
            &rows3(|m0: E0NN, m1: FP1NN, m2: P2N| Holder3749 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder3757() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|FT1_NNN|FO2_NN",
            &rows3(|m0: E0NN, m1: FT1NNN, m2: FO2NN| Holder3757 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder3765() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|FT1_NNN|V2_NN",
            &rows3(|m0: E0NN, m1: FT1NNN, m2: V2NN| Holder3765 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3773() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|O1_N|FP2_SS",
            &rows3(|m0: E0NN, m1: Option<OI1N>, m2: FP2SS| Holder3773 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder3781() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|O1_N|V2_NN",
            &rows3(|m0: E0NN, m1: Option<OI1N>, m2: V2NN| Holder3781 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder3789() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|O1_S|FP2_NN",
            &rows3(|m0: E0NN, m1: Option<OI1S>, m2: FP2NN| Holder3789 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,-", 'E'),
                ("Dot,Some,-", 'E'),
                ("Rect,None,-", 'E'),
                ("Rect,Some,-", 'E'),
                ("Wrap,None,-", 'E'),
                ("Wrap,Some,-", 'E'),
            ],
        )
    }

    fn holder3797() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|O1_S|T2_SS",
            &rows3(|m0: E0NN, m1: Option<OI1S>, m2: T2SS| Holder3797 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'R'),
                ("Dot,None,Rect", 'R'),
                ("Dot,None,Wrap", 'R'),
                ("Dot,Some,Dot", 'R'),
                ("Dot,Some,Rect", 'R'),
                ("Dot,Some,Wrap", 'R'),
                ("Rect,None,Dot", 'R'),
                ("Rect,None,Rect", 'R'),
                ("Rect,None,Wrap", 'R'),
                ("Rect,Some,Dot", 'R'),
                ("Rect,Some,Rect", 'R'),
                ("Rect,Some,Wrap", 'R'),
                ("Wrap,None,Dot", 'R'),
                ("Wrap,None,Rect", 'R'),
                ("Wrap,None,Wrap", 'R'),
                ("Wrap,Some,Dot", 'R'),
                ("Wrap,Some,Rect", 'R'),
                ("Wrap,Some,Wrap", 'R'),
            ],
        )
    }

    fn holder3805() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|P1_N|FO2_SS",
            &rows3(|m0: E0NN, m1: P1N, m2: FO2SS| Holder3805 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder3813() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|P1_N|T2_NN",
            &rows3(|m0: E0NN, m1: P1N, m2: T2NN| Holder3813 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3821() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|P1_S|FO2_NN",
            &rows3(|m0: E0NN, m1: P1S, m2: FO2NN| Holder3821 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder3829() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|P1_S|P2_S nonstrict",
            &rows3(|m0: E0NN, m1: P1S, m2: P2S| Holder3829 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder3837() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|T1_NN|E2_SS",
            &rows3(|m0: E0NN, m1: T1NN, m2: E2SS| Holder3837 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3845() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|T1_NN|T2_NN",
            &rows3(|m0: E0NN, m1: T1NN, m2: T2NN| Holder3845 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3853() -> Vec<String> {
        check(
            "main3:H3N1:E0_NN|V1_NN|FT2_NNN",
            &rows3(|m0: E0NN, m1: V1NN, m2: FT2NNN| Holder3853 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3861() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|A1_NN|A2_SS nonstrict",
            &rows3(|m0: E0SS, m1: A1NN, m2: A2SS| Holder3861 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3869() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|A1_NN|O2_N nonstrict",
            &rows3(|m0: E0SS, m1: A1NN, m2: Option<OI2N>| Holder3869 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder3877() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|A1_SS|A2_NN nonstrict",
            &rows3(|m0: E0SS, m1: A1SS, m2: A2NN| Holder3877 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3885() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|A1_SS|FT2_NNN",
            &rows3(|m0: E0SS, m1: A1SS, m2: FT2NNN| Holder3885 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3893() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|A1_SS|V2_SS",
            &rows3(|m0: E0SS, m1: A1SS, m2: V2SS| Holder3893 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'R'),
                ("Dot,Dot,Wrap", 'R'),
                ("Dot,Rect,Rect", 'R'),
                ("Dot,Rect,Wrap", 'R'),
                ("Dot,Wrap,Rect", 'R'),
                ("Dot,Wrap,Wrap", 'R'),
                ("Rect,Dot,Rect", 'R'),
                ("Rect,Dot,Wrap", 'R'),
                ("Rect,Rect,Rect", 'R'),
                ("Rect,Rect,Wrap", 'R'),
                ("Rect,Wrap,Rect", 'R'),
                ("Rect,Wrap,Wrap", 'R'),
                ("Wrap,Dot,Rect", 'R'),
                ("Wrap,Dot,Wrap", 'R'),
                ("Wrap,Rect,Rect", 'R'),
                ("Wrap,Rect,Wrap", 'R'),
                ("Wrap,Wrap,Rect", 'R'),
                ("Wrap,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder3901() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|E1_NN|FP2_SS",
            &rows3(|m0: E0SS, m1: E1NN, m2: FP2SS| Holder3901 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder3909() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|E1_NN|V2_NN",
            &rows3(|m0: E0SS, m1: E1NN, m2: V2NN| Holder3909 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3917() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|E1_SS|FP2_NN",
            &rows3(|m0: E0SS, m1: E1SS, m2: FP2NN| Holder3917 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder3925() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|E1_SS|T2_SS",
            &rows3(|m0: E0SS, m1: E1SS, m2: T2SS| Holder3925 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'R'),
                ("Dot,Dot,Rect", 'R'),
                ("Dot,Dot,Wrap", 'R'),
                ("Dot,Rect,Dot", 'R'),
                ("Dot,Rect,Rect", 'R'),
                ("Dot,Rect,Wrap", 'R'),
                ("Dot,Wrap,Dot", 'R'),
                ("Dot,Wrap,Rect", 'R'),
                ("Dot,Wrap,Wrap", 'R'),
                ("Rect,Dot,Dot", 'R'),
                ("Rect,Dot,Rect", 'R'),
                ("Rect,Dot,Wrap", 'R'),
                ("Rect,Rect,Dot", 'R'),
                ("Rect,Rect,Rect", 'R'),
                ("Rect,Rect,Wrap", 'R'),
                ("Rect,Wrap,Dot", 'R'),
                ("Rect,Wrap,Rect", 'R'),
                ("Rect,Wrap,Wrap", 'R'),
                ("Wrap,Dot,Dot", 'R'),
                ("Wrap,Dot,Rect", 'R'),
                ("Wrap,Dot,Wrap", 'R'),
                ("Wrap,Rect,Dot", 'R'),
                ("Wrap,Rect,Rect", 'R'),
                ("Wrap,Rect,Wrap", 'R'),
                ("Wrap,Wrap,Dot", 'R'),
                ("Wrap,Wrap,Rect", 'R'),
                ("Wrap,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder3933() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|FO1_NN|FP2_NN",
            &rows3(|m0: E0SS, m1: FO1NN, m2: FP2NN| Holder3933 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder3941() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|FP1_NN|A2_NN",
            &rows3(|m0: E0SS, m1: FP1NN, m2: A2NN| Holder3941 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder3949() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|FP1_NN|O2_S",
            &rows3(|m0: E0SS, m1: FP1NN, m2: Option<OI2S>| Holder3949 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,None", 'E'),
                ("Dot,-,Some", 'E'),
                ("Rect,-,None", 'E'),
                ("Rect,-,Some", 'E'),
                ("Wrap,-,None", 'E'),
                ("Wrap,-,Some", 'E'),
            ],
        )
    }

    fn holder3957() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|FT1_NNN|E2_SS",
            &rows3(|m0: E0SS, m1: FT1NNN, m2: E2SS| Holder3957 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3965() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|FT1_NNN|T2_NN",
            &rows3(|m0: E0SS, m1: FT1NNN, m2: T2NN| Holder3965 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder3973() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|O1_N|FP2_NN",
            &rows3(|m0: E0SS, m1: Option<OI1N>, m2: FP2NN| Holder3973 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,-", 'E'),
                ("Dot,Some,-", 'E'),
                ("Rect,None,-", 'E'),
                ("Rect,Some,-", 'E'),
                ("Wrap,None,-", 'E'),
                ("Wrap,Some,-", 'E'),
            ],
        )
    }

    fn holder3981() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|O1_N|T2_SS",
            &rows3(|m0: E0SS, m1: Option<OI1N>, m2: T2SS| Holder3981 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'R'),
                ("Dot,None,Rect", 'R'),
                ("Dot,None,Wrap", 'R'),
                ("Dot,Some,Dot", 'R'),
                ("Dot,Some,Rect", 'R'),
                ("Dot,Some,Wrap", 'R'),
                ("Rect,None,Dot", 'R'),
                ("Rect,None,Rect", 'R'),
                ("Rect,None,Wrap", 'R'),
                ("Rect,Some,Dot", 'R'),
                ("Rect,Some,Rect", 'R'),
                ("Rect,Some,Wrap", 'R'),
                ("Wrap,None,Dot", 'R'),
                ("Wrap,None,Rect", 'R'),
                ("Wrap,None,Wrap", 'R'),
                ("Wrap,Some,Dot", 'R'),
                ("Wrap,Some,Rect", 'R'),
                ("Wrap,Some,Wrap", 'R'),
            ],
        )
    }

    fn holder3989() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|O1_S|FO2_SS",
            &rows3(|m0: E0SS, m1: Option<OI1S>, m2: FO2SS| Holder3989 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder3997() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|O1_S|T2_NN",
            &rows3(|m0: E0SS, m1: Option<OI1S>, m2: T2NN| Holder3997 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder4005() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|P1_N|FO2_NN",
            &rows3(|m0: E0SS, m1: P1N, m2: FO2NN| Holder4005 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder4013() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|P1_N|P2_S nonstrict",
            &rows3(|m0: E0SS, m1: P1N, m2: P2S| Holder4013 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder4021() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|P1_S|E2_SS nonstrict",
            &rows3(|m0: E0SS, m1: P1S, m2: E2SS| Holder4021 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder4029() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|P1_S|P2_N nonstrict",
            &rows3(|m0: E0SS, m1: P1S, m2: P2N| Holder4029 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder4037() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|T1_NN|E2_NN",
            &rows3(|m0: E0SS, m1: T1NN, m2: E2NN| Holder4037 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4045() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|T1_NN|P2_S",
            &rows3(|m0: E0SS, m1: T1NN, m2: P2S| Holder4045 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder4053() -> Vec<String> {
        check(
            "main3:H3N1:E0_SS|V1_NN|FP2_NN",
            &rows3(|m0: E0SS, m1: V1NN, m2: FP2NN| Holder4053 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder4061() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|A1_NN|A2_NN",
            &rows3(|m0: FO0NN, m1: A1NN, m2: A2NN| Holder4061 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4069() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|A1_NN|O2_S",
            &rows3(|m0: FO0NN, m1: A1NN, m2: Option<OI2S>| Holder4069 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,None", 'E'),
                ("-,Dot,Some", 'E'),
                ("-,Rect,None", 'E'),
                ("-,Rect,Some", 'E'),
                ("-,Wrap,None", 'E'),
                ("-,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder4077() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|A1_SS|E2_SS",
            &rows3(|m0: FO0NN, m1: A1SS, m2: E2SS| Holder4077 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4085() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|A1_SS|T2_NN",
            &rows3(|m0: FO0NN, m1: A1SS, m2: T2NN| Holder4085 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4093() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|E1_NN|FT2_NNN",
            &rows3(|m0: FO0NN, m1: E1NN, m2: FT2NNN| Holder4093 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4101() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|E1_SS|A2_SS",
            &rows3(|m0: FO0NN, m1: E1SS, m2: A2SS| Holder4101 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4109() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|E1_SS|P2_N",
            &rows3(|m0: FO0NN, m1: E1SS, m2: P2N| Holder4109 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder4117() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|FO1_NN|FO2_NN",
            &rows3(|m0: FO0NN, m1: FO1NN, m2: FO2NN| Holder4117 { own: 7, m0, m1, m2 }),
            &[("-,-,-", 'E')],
        )
    }

    fn holder4125() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|FO1_NN|V2_NN",
            &rows3(|m0: FO0NN, m1: FO1NN, m2: V2NN| Holder4125 { own: 7, m0, m1, m2 }),
            &[("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder4133() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|FP1_NN|O2_N",
            &rows3(|m0: FO0NN, m1: FP1NN, m2: Option<OI2N>| Holder4133 { own: 7, m0, m1, m2 }),
            &[("-,-,None", 'E'), ("-,-,Some", 'E')],
        )
    }

    fn holder4141() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|FT1_NNN|E2_NN",
            &rows3(|m0: FO0NN, m1: FT1NNN, m2: E2NN| Holder4141 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4149() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|FT1_NNN|P2_S",
            &rows3(|m0: FO0NN, m1: FT1NNN, m2: P2S| Holder4149 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder4157() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|O1_N|FP2_NN",
            &rows3(|m0: FO0NN, m1: Option<OI1N>, m2: FP2NN| Holder4157 { own: 7, m0, m1, m2 }),
            &[("-,None,-", 'E'), ("-,Some,-", 'E')],
        )
    }

    fn holder4165() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|O1_S|A2_NN",
            &rows3(|m0: FO0NN, m1: Option<OI1S>, m2: A2NN| Holder4165 { own: 7, m0, m1, m2 }),
            &[
                ("-,None,Dot", 'E'),
                ("-,None,Rect", 'E'),
                ("-,None,Wrap", 'E'),
                ("-,Some,Dot", 'E'),
                ("-,Some,Rect", 'E'),
                ("-,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder4173() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|O1_S|O2_S",
            &rows3(|m0: FO0NN, m1: Option<OI1S>, m2: Option<OI2S>| Holder4173 {
                own: 7,
                m0,
                m1,
                m2,
            }),
            &[
                ("-,None,None", 'E'),
                ("-,None,Some", 'E'),
                ("-,Some,None", 'E'),
                ("-,Some,Some", 'E'),
            ],
        )
    }

    fn holder4181() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|P1_N|E2_SS",
            &rows3(|m0: FO0NN, m1: P1N, m2: E2SS| Holder4181 { own: 7, m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder4189() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|P1_N|T2_NN",
            &rows3(|m0: FO0NN, m1: P1N, m2: T2NN| Holder4189 { own: 7, m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder4197() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|P1_S|FT2_NNN",
            &rows3(|m0: FO0NN, m1: P1S, m2: FT2NNN| Holder4197 { own: 7, m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder4205() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|T1_NN|A2_SS",
            &rows3(|m0: FO0NN, m1: T1NN, m2: A2SS| Holder4205 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4213() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|T1_NN|P2_N",
            &rows3(|m0: FO0NN, m1: T1NN, m2: P2N| Holder4213 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder4221() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|V1_NN|FO2_NN",
            &rows3(|m0: FO0NN, m1: V1NN, m2: FO2NN| Holder4221 { own: 7, m0, m1, m2 }),
            &[("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder4229() -> Vec<String> {
        check(
            "main3:H3N1:FO0_NN|V1_NN|V2_NN",
            &rows3(|m0: FO0NN, m1: V1NN, m2: V2NN| Holder4229 { own: 7, m0, m1, m2 }),
            &[
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4237() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|A1_NN|O2_N",
            &rows3(|m0: FP0NN, m1: A1NN, m2: Option<OI2N>| Holder4237 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,None", 'E'),
                ("-,Dot,Some", 'E'),
                ("-,Rect,None", 'E'),
                ("-,Rect,Some", 'E'),
                ("-,Wrap,None", 'E'),
                ("-,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder4245() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|A1_SS|E2_NN",
            &rows3(|m0: FP0NN, m1: A1SS, m2: E2NN| Holder4245 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4253() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|A1_SS|P2_S",
            &rows3(|m0: FP0NN, m1: A1SS, m2: P2S| Holder4253 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder4261() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|E1_NN|FP2_NN",
            &rows3(|m0: FP0NN, m1: E1NN, m2: FP2NN| Holder4261 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder4269() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|E1_SS|A2_NN",
            &rows3(|m0: FP0NN, m1: E1SS, m2: A2NN| Holder4269 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4277() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|E1_SS|O2_S",
            &rows3(|m0: FP0NN, m1: E1SS, m2: Option<OI2S>| Holder4277 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,None", 'E'),
                ("-,Dot,Some", 'E'),
                ("-,Rect,None", 'E'),
                ("-,Rect,Some", 'E'),
                ("-,Wrap,None", 'E'),
                ("-,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder4285() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|FO1_NN|E2_SS",
            &rows3(|m0: FP0NN, m1: FO1NN, m2: E2SS| Holder4285 { own: 7, m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder4293() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|FO1_NN|T2_NN",
            &rows3(|m0: FP0NN, m1: FO1NN, m2: T2NN| Holder4293 { own: 7, m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder4301() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|FP1_NN|FT2_NNN",
            &rows3(|m0: FP0NN, m1: FP1NN, m2: FT2NNN| Holder4301 { own: 7, m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder4309() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|FT1_NNN|A2_SS",
            &rows3(|m0: FP0NN, m1: FT1NNN, m2: A2SS| Holder4309 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4317() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|FT1_NNN|P2_N",
            &rows3(|m0: FP0NN, m1: FT1NNN, m2: P2N| Holder4317 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder4325() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|O1_N|FO2_NN",
            &rows3(|m0: FP0NN, m1: Option<OI1N>, m2: FO2NN| Holder4325 { own: 7, m0, m1, m2 }),
            &[("-,None,-", 'E'), ("-,Some,-", 'E')],
        )
    }

    fn holder4333() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|O1_N|V2_NN",
            &rows3(|m0: FP0NN, m1: Option<OI1N>, m2: V2NN| Holder4333 { own: 7, m0, m1, m2 }),
            &[
                ("-,None,Rect", 'E'),
                ("-,None,Wrap", 'E'),
                ("-,Some,Rect", 'E'),
                ("-,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder4341() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|O1_S|O2_N",
            &rows3(|m0: FP0NN, m1: Option<OI1S>, m2: Option<OI2N>| Holder4341 {
                own: 7,
                m0,
                m1,
                m2,
            }),
            &[
                ("-,None,None", 'E'),
                ("-,None,Some", 'E'),
                ("-,Some,None", 'E'),
                ("-,Some,Some", 'E'),
            ],
        )
    }

    fn holder4349() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|P1_N|E2_NN",
            &rows3(|m0: FP0NN, m1: P1N, m2: E2NN| Holder4349 { own: 7, m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder4357() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|P1_N|P2_S",
            &rows3(|m0: FP0NN, m1: P1N, m2: P2S| Holder4357 { own: 7, m0, m1, m2 }),
            &[("-,-,-", 'E')],
        )
    }

    fn holder4365() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|P1_S|FP2_NN",
            &rows3(|m0: FP0NN, m1: P1S, m2: FP2NN| Holder4365 { own: 7, m0, m1, m2 }),
            &[("-,-,-", 'E')],
        )
    }

    fn holder4373() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|T1_NN|A2_NN",
            &rows3(|m0: FP0NN, m1: T1NN, m2: A2NN| Holder4373 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4381() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|T1_NN|O2_S",
            &rows3(|m0: FP0NN, m1: T1NN, m2: Option<OI2S>| Holder4381 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,None", 'E'),
                ("-,Dot,Some", 'E'),
                ("-,Rect,None", 'E'),
                ("-,Rect,Some", 'E'),
                ("-,Wrap,None", 'E'),
                ("-,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder4389() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|V1_NN|E2_SS",
            &rows3(|m0: FP0NN, m1: V1NN, m2: E2SS| Holder4389 { own: 7, m0, m1, m2 }),
            &[
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4397() -> Vec<String> {
        check(
            "main3:H3N1:FP0_NN|V1_NN|T2_NN",
            &rows3(|m0: FP0NN, m1: V1NN, m2: T2NN| Holder4397 { own: 7, m0, m1, m2 }),
            &[
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4405() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|A1_NN|FT2_NNN",
            &rows3(|m0: FT0NNN, m1: A1NN, m2: FT2NNN| Holder4405 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4413() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|A1_SS|A2_SS",
            &rows3(|m0: FT0NNN, m1: A1SS, m2: A2SS| Holder4413 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4421() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|A1_SS|P2_N",
            &rows3(|m0: FT0NNN, m1: A1SS, m2: P2N| Holder4421 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder4429() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|E1_NN|FO2_NN",
            &rows3(|m0: FT0NNN, m1: E1NN, m2: FO2NN| Holder4429 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder4437() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|E1_NN|V2_NN",
            &rows3(|m0: FT0NNN, m1: E1NN, m2: V2NN| Holder4437 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4445() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|E1_SS|O2_N",
            &rows3(|m0: FT0NNN, m1: E1SS, m2: Option<OI2N>| Holder4445 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder4453() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|FO1_NN|E2_NN",
            &rows3(|m0: FT0NNN, m1: FO1NN, m2: E2NN| Holder4453 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder4461() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|FO1_NN|P2_S",
            &rows3(|m0: FT0NNN, m1: FO1NN, m2: P2S| Holder4461 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder4469() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|FP1_NN|FP2_NN",
            &rows3(|m0: FT0NNN, m1: FP1NN, m2: FP2NN| Holder4469 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder4477() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|FT1_NNN|A2_NN",
            &rows3(|m0: FT0NNN, m1: FT1NNN, m2: A2NN| Holder4477 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4485() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|FT1_NNN|O2_S",
            &rows3(|m0: FT0NNN, m1: FT1NNN, m2: Option<OI2S>| Holder4485 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder4493() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|O1_N|E2_SS",
            &rows3(|m0: FT0NNN, m1: Option<OI1N>, m2: E2SS| Holder4493 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder4501() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|O1_N|T2_NN",
            &rows3(|m0: FT0NNN, m1: Option<OI1N>, m2: T2NN| Holder4501 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder4509() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|O1_S|FT2_NNN",
            &rows3(|m0: FT0NNN, m1: Option<OI1S>, m2: FT2NNN| Holder4509 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder4517() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|P1_N|A2_SS",
            &rows3(|m0: FT0NNN, m1: P1N, m2: A2SS| Holder4517 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder4525() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|P1_N|P2_N",
            &rows3(|m0: FT0NNN, m1: P1N, m2: P2N| Holder4525 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder4533() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|P1_S|FO2_NN",
            &rows3(|m0: FT0NNN, m1: P1S, m2: FO2NN| Holder4533 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder4541() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|P1_S|V2_NN",
            &rows3(|m0: FT0NNN, m1: P1S, m2: V2NN| Holder4541 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder4549() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|T1_NN|O2_N",
            &rows3(|m0: FT0NNN, m1: T1NN, m2: Option<OI2N>| Holder4549 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder4557() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|V1_NN|E2_NN",
            &rows3(|m0: FT0NNN, m1: V1NN, m2: E2NN| Holder4557 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4565() -> Vec<String> {
        check(
            "main3:H3N1:FT0_NNN|V1_NN|P2_S",
            &rows3(|m0: FT0NNN, m1: V1NN, m2: P2S| Holder4565 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder4573() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|A1_NN|FO2_SS",
            &rows3(|m0: Option<OI0N>, m1: A1NN, m2: FO2SS| Holder4573 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder4581() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|A1_NN|T2_NN",
            &rows3(|m0: Option<OI0N>, m1: A1NN, m2: T2NN| Holder4581 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4589() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|A1_SS|FO2_NN",
            &rows3(|m0: Option<OI0N>, m1: A1SS, m2: FO2NN| Holder4589 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'E'),
                ("None,Rect,-", 'E'),
                ("None,Wrap,-", 'E'),
                ("Some,Dot,-", 'E'),
                ("Some,Rect,-", 'E'),
                ("Some,Wrap,-", 'E'),
            ],
        )
    }

    fn holder4597() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|A1_SS|P2_S nonstrict",
            &rows3(|m0: Option<OI0N>, m1: A1SS, m2: P2S| Holder4597 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'E'),
                ("None,Rect,-", 'E'),
                ("None,Wrap,-", 'E'),
                ("Some,Dot,-", 'E'),
                ("Some,Rect,-", 'E'),
                ("Some,Wrap,-", 'E'),
            ],
        )
    }

    fn holder4605() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|E1_NN|E2_SS nonstrict",
            &rows3(|m0: Option<OI0N>, m1: E1NN, m2: E2SS| Holder4605 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4613() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|E1_NN|P2_N nonstrict",
            &rows3(|m0: Option<OI0N>, m1: E1NN, m2: P2N| Holder4613 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'E'),
                ("None,Rect,-", 'E'),
                ("None,Wrap,-", 'E'),
                ("Some,Dot,-", 'E'),
                ("Some,Rect,-", 'E'),
                ("Some,Wrap,-", 'E'),
            ],
        )
    }

    fn holder4621() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|E1_SS|E2_NN nonstrict",
            &rows3(|m0: Option<OI0N>, m1: E1SS, m2: E2NN| Holder4621 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4629() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|E1_SS|O2_S nonstrict",
            &rows3(|m0: Option<OI0N>, m1: E1SS, m2: Option<OI2S>| Holder4629 {
                own: 7,
                m0,
                m1,
                m2,
            }),
            &[
                ("None,Dot,None", 'E'),
                ("None,Dot,Some", 'E'),
                ("None,Rect,None", 'E'),
                ("None,Rect,Some", 'E'),
                ("None,Wrap,None", 'E'),
                ("None,Wrap,Some", 'E'),
                ("Some,Dot,None", 'E'),
                ("Some,Dot,Some", 'E'),
                ("Some,Rect,None", 'E'),
                ("Some,Rect,Some", 'E'),
                ("Some,Wrap,None", 'E'),
                ("Some,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder4637() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|FO1_NN|A2_SS",
            &rows3(|m0: Option<OI0N>, m1: FO1NN, m2: A2SS| Holder4637 { own: 7, m0, m1, m2 }),
            &[
                ("None,-,Dot", 'E'),
                ("None,-,Rect", 'E'),
                ("None,-,Wrap", 'E'),
                ("Some,-,Dot", 'E'),
                ("Some,-,Rect", 'E'),
                ("Some,-,Wrap", 'E'),
            ],
        )
    }

    fn holder4645() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|FO1_NN|P2_N",
            &rows3(|m0: Option<OI0N>, m1: FO1NN, m2: P2N| Holder4645 { own: 7, m0, m1, m2 }),
            &[("None,-,-", 'E'), ("Some,-,-", 'E')],
        )
    }

    fn holder4653() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|FP1_NN|FO2_NN",
            &rows3(|m0: Option<OI0N>, m1: FP1NN, m2: FO2NN| Holder4653 { own: 7, m0, m1, m2 }),
            &[("None,-,-", 'E'), ("Some,-,-", 'E')],
        )
    }

    fn holder4661() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|FP1_NN|V2_NN",
            &rows3(|m0: Option<OI0N>, m1: FP1NN, m2: V2NN| Holder4661 { own: 7, m0, m1, m2 }),
            &[
                ("None,-,Rect", 'E'),
                ("None,-,Wrap", 'E'),
                ("Some,-,Rect", 'E'),
                ("Some,-,Wrap", 'E'),
            ],
        )
    }

    fn holder4669() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|FT1_NNN|O2_N",
            &rows3(
                |m0: Option<OI0N>, m1: FT1NNN, m2: Option<OI2N>| Holder4669 { own: 7, m0, m1, m2 },
            ),
            &[
                ("None,Dot,None", 'E'),
                ("None,Dot,Some", 'E'),
                ("None,Rect,None", 'E'),
                ("None,Rect,Some", 'E'),
                ("None,Wrap,None", 'E'),
                ("None,Wrap,Some", 'E'),
                ("Some,Dot,None", 'E'),
                ("Some,Dot,Some", 'E'),
                ("Some,Rect,None", 'E'),
                ("Some,Rect,Some", 'E'),
                ("Some,Wrap,None", 'E'),
                ("Some,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder4677() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|O1_N|E2_NN nonstrict",
            &rows3(|m0: Option<OI0N>, m1: Option<OI1N>, m2: E2NN| Holder4677 {
                own: 7,
                m0,
                m1,
                m2,
            }),
            &[
                ("None,None,Dot", 'E'),
                ("None,None,Rect", 'E'),
                ("None,None,Wrap", 'E'),
                ("None,Some,Dot", 'E'),
                ("None,Some,Rect", 'E'),
                ("None,Some,Wrap", 'E'),
                ("Some,None,Dot", 'E'),
                ("Some,None,Rect", 'E'),
                ("Some,None,Wrap", 'E'),
                ("Some,Some,Dot", 'E'),
                ("Some,Some,Rect", 'E'),
                ("Some,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder4685() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|O1_N|O2_S nonstrict",
            &rows3(
                |m0: Option<OI0N>, m1: Option<OI1N>, m2: Option<OI2S>| Holder4685 {
                    own: 7,
                    m0,
                    m1,
                    m2,
                },
            ),
            &[
                ("None,None,None", 'E'),
                ("None,None,Some", 'E'),
                ("None,Some,None", 'E'),
                ("None,Some,Some", 'E'),
                ("Some,None,None", 'E'),
                ("Some,None,Some", 'E'),
                ("Some,Some,None", 'E'),
                ("Some,Some,Some", 'E'),
            ],
        )
    }

    fn holder4693() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|O1_S|A2_SS nonstrict",
            &rows3(|m0: Option<OI0N>, m1: Option<OI1S>, m2: A2SS| Holder4693 {
                own: 7,
                m0,
                m1,
                m2,
            }),
            &[
                ("None,None,Dot", 'E'),
                ("None,None,Rect", 'E'),
                ("None,None,Wrap", 'E'),
                ("None,Some,Dot", 'E'),
                ("None,Some,Rect", 'E'),
                ("None,Some,Wrap", 'E'),
                ("Some,None,Dot", 'E'),
                ("Some,None,Rect", 'E'),
                ("Some,None,Wrap", 'E'),
                ("Some,Some,Dot", 'E'),
                ("Some,Some,Rect", 'E'),
                ("Some,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder4701() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|O1_S|O2_N nonstrict",
            &rows3(
                |m0: Option<OI0N>, m1: Option<OI1S>, m2: Option<OI2N>| Holder4701 {
                    own: 7,
                    m0,
                    m1,
                    m2,
                },
            ),
            &[
                ("None,None,None", 'E'),
                ("None,None,Some", 'E'),
                ("None,Some,None", 'E'),
                ("None,Some,Some", 'E'),
                ("Some,None,None", 'E'),
                ("Some,None,Some", 'E'),
                ("Some,Some,None", 'E'),
                ("Some,Some,Some", 'E'),
            ],
        )
    }

    fn holder4709() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|P1_N|A2_NN nonstrict",
            &rows3(|m0: Option<OI0N>, m1: P1N, m2: A2NN| Holder4709 { own: 7, m0, m1, m2 }),
            &[
                ("None,-,Dot", 'E'),
                ("None,-,Rect", 'E'),
                ("None,-,Wrap", 'E'),
                ("Some,-,Dot", 'E'),
                ("Some,-,Rect", 'E'),
                ("Some,-,Wrap", 'E'),
            ],
        )
    }

    fn holder4717() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|P1_N|FT2_NNN",
            &rows3(|m0: Option<OI0N>, m1: P1N, m2: FT2NNN| Holder4717 { own: 7, m0, m1, m2 }),
            &[
                ("None,-,Dot", 'E'),
                ("None,-,Rect", 'E'),
                ("None,-,Wrap", 'E'),
                ("Some,-,Dot", 'E'),
                ("Some,-,Rect", 'E'),
                ("Some,-,Wrap", 'E'),
            ],
        )
    }

    fn holder4725() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|P1_N|V2_SS",
            &rows3(|m0: Option<OI0N>, m1: P1N, m2: V2SS| Holder4725 { own: 7, m0, m1, m2 }),
            &[
                ("None,-,Rect", 'R'),
                ("None,-,Wrap", 'R'),
                ("Some,-,Rect", 'R'),
                ("Some,-,Wrap", 'R'),
            ],
        )
    }

    fn holder4733() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|P1_S|FP2_SS",
            &rows3(|m0: Option<OI0N>, m1: P1S, m2: FP2SS| Holder4733 { own: 7, m0, m1, m2 }),
            &[("None,-,-", 'R'), ("Some,-,-", 'R')],
        )
    }

    fn holder4741() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|P1_S|V2_NN",
            &rows3(|m0: Option<OI0N>, m1: P1S, m2: V2NN| Holder4741 { own: 7, m0, m1, m2 }),
            &[
                ("None,-,Rect", 'E'),
                ("None,-,Wrap", 'E'),
                ("Some,-,Rect", 'E'),
                ("Some,-,Wrap", 'E'),
            ],
        )
    }

    fn holder4749() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|T1_NN|FT2_NNN",
            &rows3(|m0: Option<OI0N>, m1: T1NN, m2: FT2NNN| Holder4749 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4757() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|V1_NN|A2_SS",
            &rows3(|m0: Option<OI0N>, m1: V1NN, m2: A2SS| Holder4757 { own: 7, m0, m1, m2 }),
            &[
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4765() -> Vec<String> {
        check(
            "main3:H3N1:O0_N|V1_NN|P2_N",
            &rows3(|m0: Option<OI0N>, m1: V1NN, m2: P2N| Holder4765 { own: 7, m0, m1, m2 }),
            &[
                ("None,Rect,-", 'E'),
                ("None,Wrap,-", 'E'),
                ("Some,Rect,-", 'E'),
                ("Some,Wrap,-", 'E'),
            ],
        )
    }

    fn holder4773() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|A1_NN|FO2_NN",
            &rows3(|m0: Option<OI0S>, m1: A1NN, m2: FO2NN| Holder4773 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'E'),
                ("None,Rect,-", 'E'),
                ("None,Wrap,-", 'E'),
                ("Some,Dot,-", 'E'),
                ("Some,Rect,-", 'E'),
                ("Some,Wrap,-", 'E'),
            ],
        )
    }

    fn holder4781() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|A1_NN|P2_S nonstrict",
            &rows3(|m0: Option<OI0S>, m1: A1NN, m2: P2S| Holder4781 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'E'),
                ("None,Rect,-", 'E'),
                ("None,Wrap,-", 'E'),
                ("Some,Dot,-", 'E'),
                ("Some,Rect,-", 'E'),
                ("Some,Wrap,-", 'E'),
            ],
        )
    }

    fn holder4789() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|A1_SS|E2_SS nonstrict",
            &rows3(|m0: Option<OI0S>, m1: A1SS, m2: E2SS| Holder4789 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4797() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|A1_SS|P2_N nonstrict",
            &rows3(|m0: Option<OI0S>, m1: A1SS, m2: P2N| Holder4797 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'E'),
                ("None,Rect,-", 'E'),
                ("None,Wrap,-", 'E'),
                ("Some,Dot,-", 'E'),
                ("Some,Rect,-", 'E'),
                ("Some,Wrap,-", 'E'),
            ],
        )
    }

    fn holder4805() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|E1_NN|E2_NN nonstrict",
            &rows3(|m0: Option<OI0S>, m1: E1NN, m2: E2NN| Holder4805 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4813() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|E1_NN|O2_S nonstrict",
            &rows3(|m0: Option<OI0S>, m1: E1NN, m2: Option<OI2S>| Holder4813 {
                own: 7,
                m0,
                m1,
                m2,
            }),
            &[
                ("None,Dot,None", 'E'),
                ("None,Dot,Some", 'E'),
                ("None,Rect,None", 'E'),
                ("None,Rect,Some", 'E'),
                ("None,Wrap,None", 'E'),
                ("None,Wrap,Some", 'E'),
                ("Some,Dot,None", 'E'),
                ("Some,Dot,Some", 'E'),
                ("Some,Rect,None", 'E'),
                ("Some,Rect,Some", 'E'),
                ("Some,Wrap,None", 'E'),
                ("Some,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder4821() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|E1_SS|A2_SS nonstrict",
            &rows3(|m0: Option<OI0S>, m1: E1SS, m2: A2SS| Holder4821 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4829() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|E1_SS|O2_N nonstrict",
            &rows3(|m0: Option<OI0S>, m1: E1SS, m2: Option<OI2N>| Holder4829 {
                own: 7,
                m0,
                m1,
                m2,
            }),
            &[
                ("None,Dot,None", 'E'),
                ("None,Dot,Some", 'E'),
                ("None,Rect,None", 'E'),
                ("None,Rect,Some", 'E'),
                ("None,Wrap,None", 'E'),
                ("None,Wrap,Some", 'E'),
                ("Some,Dot,None", 'E'),
                ("Some,Dot,Some", 'E'),
                ("Some,Rect,None", 'E'),
                ("Some,Rect,Some", 'E'),
                ("Some,Wrap,None", 'E'),
                ("Some,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder4837() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|FO1_NN|A2_NN",
            &rows3(|m0: Option<OI0S>, m1: FO1NN, m2: A2NN| Holder4837 { own: 7, m0, m1, m2 }),
            &[
                ("None,-,Dot", 'E'),
                ("None,-,Rect", 'E'),
                ("None,-,Wrap", 'E'),
                ("Some,-,Dot", 'E'),
                ("Some,-,Rect", 'E'),
                ("Some,-,Wrap", 'E'),
            ],
        )
    }

    fn holder4845() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|FO1_NN|O2_S",
            &rows3(|m0: Option<OI0S>, m1: FO1NN, m2: Option<OI2S>| Holder4845 {
                own: 7,
                m0,
                m1,
                m2,
            }),
            &[
                ("None,-,None", 'E'),
                ("None,-,Some", 'E'),
                ("Some,-,None", 'E'),
                ("Some,-,Some", 'E'),
            ],
        )
    }

    fn holder4853() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|FP1_NN|E2_SS",
            &rows3(|m0: Option<OI0S>, m1: FP1NN, m2: E2SS| Holder4853 { own: 7, m0, m1, m2 }),
            &[
                ("None,-,Dot", 'E'),
                ("None,-,Rect", 'E'),
                ("None,-,Wrap", 'E'),
                ("Some,-,Dot", 'E'),
                ("Some,-,Rect", 'E'),
                ("Some,-,Wrap", 'E'),
            ],
        )
    }

    fn holder4861() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|FP1_NN|T2_NN",
            &rows3(|m0: Option<OI0S>, m1: FP1NN, m2: T2NN| Holder4861 { own: 7, m0, m1, m2 }),
            &[
                ("None,-,Dot", 'E'),
                ("None,-,Rect", 'E'),
                ("None,-,Wrap", 'E'),
                ("Some,-,Dot", 'E'),
                ("Some,-,Rect", 'E'),
                ("Some,-,Wrap", 'E'),
            ],
        )
    }

    fn holder4869() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|FT1_NNN|FT2_NNN",
            &rows3(|m0: Option<OI0S>, m1: FT1NNN, m2: FT2NNN| Holder4869 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,Dot", 'E'),
                ("None,Dot,Rect", 'E'),
                ("None,Dot,Wrap", 'E'),
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Dot,Dot", 'E'),
                ("Some,Dot,Rect", 'E'),
                ("Some,Dot,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4877() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|O1_N|A2_SS nonstrict",
            &rows3(|m0: Option<OI0S>, m1: Option<OI1N>, m2: A2SS| Holder4877 {
                own: 7,
                m0,
                m1,
                m2,
            }),
            &[
                ("None,None,Dot", 'E'),
                ("None,None,Rect", 'E'),
                ("None,None,Wrap", 'E'),
                ("None,Some,Dot", 'E'),
                ("None,Some,Rect", 'E'),
                ("None,Some,Wrap", 'E'),
                ("Some,None,Dot", 'E'),
                ("Some,None,Rect", 'E'),
                ("Some,None,Wrap", 'E'),
                ("Some,Some,Dot", 'E'),
                ("Some,Some,Rect", 'E'),
                ("Some,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder4885() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|O1_N|O2_N nonstrict",
            &rows3(
                |m0: Option<OI0S>, m1: Option<OI1N>, m2: Option<OI2N>| Holder4885 {
                    own: 7,
                    m0,
                    m1,
                    m2,
                },
            ),
            &[
                ("None,None,None", 'E'),
                ("None,None,Some", 'E'),
                ("None,Some,None", 'E'),
                ("None,Some,Some", 'E'),
                ("Some,None,None", 'E'),
                ("Some,None,Some", 'E'),
                ("Some,Some,None", 'E'),
                ("Some,Some,Some", 'E'),
            ],
        )
    }

    fn holder4893() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|O1_S|A2_NN nonstrict",
            &rows3(|m0: Option<OI0S>, m1: Option<OI1S>, m2: A2NN| Holder4893 {
                own: 7,
                m0,
                m1,
                m2,
            }),
            &[
                ("None,None,Dot", 'E'),
                ("None,None,Rect", 'E'),
                ("None,None,Wrap", 'E'),
                ("None,Some,Dot", 'E'),
                ("None,Some,Rect", 'E'),
                ("None,Some,Wrap", 'E'),
                ("Some,None,Dot", 'E'),
                ("Some,None,Rect", 'E'),
                ("Some,None,Wrap", 'E'),
                ("Some,Some,Dot", 'E'),
                ("Some,Some,Rect", 'E'),
                ("Some,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder4901() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|O1_S|FT2_NNN",
            &rows3(
                |m0: Option<OI0S>, m1: Option<OI1S>, m2: FT2NNN| Holder4901 { own: 7, m0, m1, m2 },
            ),
            &[
                ("None,None,Dot", 'E'),
                ("None,None,Rect", 'E'),
                ("None,None,Wrap", 'E'),
                ("None,Some,Dot", 'E'),
                ("None,Some,Rect", 'E'),
                ("None,Some,Wrap", 'E'),
                ("Some,None,Dot", 'E'),
                ("Some,None,Rect", 'E'),
                ("Some,None,Wrap", 'E'),
                ("Some,Some,Dot", 'E'),
                ("Some,Some,Rect", 'E'),
                ("Some,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder4909() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|O1_S|V2_SS",
            &rows3(|m0: Option<OI0S>, m1: Option<OI1S>, m2: V2SS| Holder4909 {
                own: 7,
                m0,
                m1,
                m2,
            }),
            &[
                ("None,None,Rect", 'R'),
                ("None,None,Wrap", 'R'),
                ("None,Some,Rect", 'R'),
                ("None,Some,Wrap", 'R'),
                ("Some,None,Rect", 'R'),
                ("Some,None,Wrap", 'R'),
                ("Some,Some,Rect", 'R'),
                ("Some,Some,Wrap", 'R'),
            ],
        )
    }

    fn holder4917() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|P1_N|FP2_SS",
            &rows3(|m0: Option<OI0S>, m1: P1N, m2: FP2SS| Holder4917 { own: 7, m0, m1, m2 }),
            &[("None,-,-", 'R'), ("Some,-,-", 'R')],
        )
    }

    fn holder4925() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|P1_N|V2_NN",
            &rows3(|m0: Option<OI0S>, m1: P1N, m2: V2NN| Holder4925 { own: 7, m0, m1, m2 }),
            &[
                ("None,-,Rect", 'E'),
                ("None,-,Wrap", 'E'),
                ("Some,-,Rect", 'E'),
                ("Some,-,Wrap", 'E'),
            ],
        )
    }

    fn holder4933() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|P1_S|FP2_NN",
            &rows3(|m0: Option<OI0S>, m1: P1S, m2: FP2NN| Holder4933 { own: 7, m0, m1, m2 }),
            &[("None,-,-", 'E'), ("Some,-,-", 'E')],
        )
    }

    fn holder4941() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|P1_S|T2_SS",
            &rows3(|m0: Option<OI0S>, m1: P1S, m2: T2SS| Holder4941 { own: 7, m0, m1, m2 }),
            &[
                ("None,-,Dot", 'R'),
                ("None,-,Rect", 'R'),
                ("None,-,Wrap", 'R'),
                ("Some,-,Dot", 'R'),
                ("Some,-,Rect", 'R'),
                ("Some,-,Wrap", 'R'),
            ],
        )
    }

    fn holder4949() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|T1_NN|FP2_NN",
            &rows3(|m0: Option<OI0S>, m1: T1NN, m2: FP2NN| Holder4949 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'E'),
                ("None,Rect,-", 'E'),
                ("None,Wrap,-", 'E'),
                ("Some,Dot,-", 'E'),
                ("Some,Rect,-", 'E'),
                ("Some,Wrap,-", 'E'),
            ],
        )
    }

    fn holder4957() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|V1_NN|A2_NN",
            &rows3(|m0: Option<OI0S>, m1: V1NN, m2: A2NN| Holder4957 { own: 7, m0, m1, m2 }),
            &[
                ("None,Rect,Dot", 'E'),
                ("None,Rect,Rect", 'E'),
                ("None,Rect,Wrap", 'E'),
                ("None,Wrap,Dot", 'E'),
                ("None,Wrap,Rect", 'E'),
                ("None,Wrap,Wrap", 'E'),
                ("Some,Rect,Dot", 'E'),
                ("Some,Rect,Rect", 'E'),
                ("Some,Rect,Wrap", 'E'),
                ("Some,Wrap,Dot", 'E'),
                ("Some,Wrap,Rect", 'E'),
                ("Some,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4965() -> Vec<String> {
        check(
            "main3:H3N1:O0_S|V1_NN|O2_S",
            &rows3(|m0: Option<OI0S>, m1: V1NN, m2: Option<OI2S>| Holder4965 {
                own: 7,
                m0,
                m1,
                m2,
            }),
            &[
                ("None,Rect,None", 'E'),
                ("None,Rect,Some", 'E'),
                ("None,Wrap,None", 'E'),
                ("None,Wrap,Some", 'E'),
                ("Some,Rect,None", 'E'),
                ("Some,Rect,Some", 'E'),
                ("Some,Wrap,None", 'E'),
                ("Some,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder4973() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|A1_NN|E2_SS nonstrict",
            &rows3(|m0: P0N, m1: A1NN, m2: E2SS| Holder4973 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4981() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|A1_NN|P2_N nonstrict",
            &rows3(|m0: P0N, m1: A1NN, m2: P2N| Holder4981 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder4989() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|A1_SS|E2_NN nonstrict",
            &rows3(|m0: P0N, m1: A1SS, m2: E2NN| Holder4989 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder4997() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|A1_SS|O2_S nonstrict",
            &rows3(|m0: P0N, m1: A1SS, m2: Option<OI2S>| Holder4997 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,None", 'E'),
                ("-,Dot,Some", 'E'),
                ("-,Rect,None", 'E'),
                ("-,Rect,Some", 'E'),
                ("-,Wrap,None", 'E'),
                ("-,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder5005() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|E1_NN|A2_SS nonstrict",
            &rows3(|m0: P0N, m1: E1NN, m2: A2SS| Holder5005 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5013() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|E1_NN|O2_N nonstrict",
            &rows3(|m0: P0N, m1: E1NN, m2: Option<OI2N>| Holder5013 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,None", 'E'),
                ("-,Dot,Some", 'E'),
                ("-,Rect,None", 'E'),
                ("-,Rect,Some", 'E'),
                ("-,Wrap,None", 'E'),
                ("-,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder5021() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|E1_SS|A2_NN nonstrict",
            &rows3(|m0: P0N, m1: E1SS, m2: A2NN| Holder5021 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5029() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|E1_SS|FT2_NNN",
            &rows3(|m0: P0N, m1: E1SS, m2: FT2NNN| Holder5029 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5037() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|E1_SS|V2_SS",
            &rows3(|m0: P0N, m1: E1SS, m2: V2SS| Holder5037 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Rect", 'R'),
                ("-,Dot,Wrap", 'R'),
                ("-,Rect,Rect", 'R'),
                ("-,Rect,Wrap", 'R'),
                ("-,Wrap,Rect", 'R'),
                ("-,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder5045() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|FO1_NN|O2_N",
            &rows3(|m0: P0N, m1: FO1NN, m2: Option<OI2N>| Holder5045 { own: 7, m0, m1, m2 }),
            &[("-,-,None", 'E'), ("-,-,Some", 'E')],
        )
    }

    fn holder5053() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|FP1_NN|E2_NN",
            &rows3(|m0: P0N, m1: FP1NN, m2: E2NN| Holder5053 { own: 7, m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder5061() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|FP1_NN|P2_S",
            &rows3(|m0: P0N, m1: FP1NN, m2: P2S| Holder5061 { own: 7, m0, m1, m2 }),
            &[("-,-,-", 'E')],
        )
    }

    fn holder5069() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|FT1_NNN|FP2_NN",
            &rows3(|m0: P0N, m1: FT1NNN, m2: FP2NN| Holder5069 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder5077() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|O1_N|A2_NN nonstrict",
            &rows3(|m0: P0N, m1: Option<OI1N>, m2: A2NN| Holder5077 { own: 7, m0, m1, m2 }),
            &[
                ("-,None,Dot", 'E'),
                ("-,None,Rect", 'E'),
                ("-,None,Wrap", 'E'),
                ("-,Some,Dot", 'E'),
                ("-,Some,Rect", 'E'),
                ("-,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder5085() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|O1_N|FT2_NNN",
            &rows3(|m0: P0N, m1: Option<OI1N>, m2: FT2NNN| Holder5085 { own: 7, m0, m1, m2 }),
            &[
                ("-,None,Dot", 'E'),
                ("-,None,Rect", 'E'),
                ("-,None,Wrap", 'E'),
                ("-,Some,Dot", 'E'),
                ("-,Some,Rect", 'E'),
                ("-,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder5093() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|O1_N|V2_SS",
            &rows3(|m0: P0N, m1: Option<OI1N>, m2: V2SS| Holder5093 { own: 7, m0, m1, m2 }),
            &[
                ("-,None,Rect", 'R'),
                ("-,None,Wrap", 'R'),
                ("-,Some,Rect", 'R'),
                ("-,Some,Wrap", 'R'),
            ],
        )
    }

    fn holder5101() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|O1_S|FP2_SS",
            &rows3(|m0: P0N, m1: Option<OI1S>, m2: FP2SS| Holder5101 { own: 7, m0, m1, m2 }),
            &[("-,None,-", 'R'), ("-,Some,-", 'R')],
        )
    }

    fn holder5109() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|O1_S|V2_NN",
            &rows3(|m0: P0N, m1: Option<OI1S>, m2: V2NN| Holder5109 { own: 7, m0, m1, m2 }),
            &[
                ("-,None,Rect", 'E'),
                ("-,None,Wrap", 'E'),
                ("-,Some,Rect", 'E'),
                ("-,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder5117() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|P1_N|FP2_NN",
            &rows3(|m0: P0N, m1: P1N, m2: FP2NN| Holder5117 { own: 7, m0, m1, m2 }),
            &[("-,-,-", 'E')],
        )
    }

    fn holder5125() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|P1_N|T2_SS",
            &rows3(|m0: P0N, m1: P1N, m2: T2SS| Holder5125 { own: 7, m0, m1, m2 }),
            &[("-,-,Dot", 'R'), ("-,-,Rect", 'R'), ("-,-,Wrap", 'R')],
        )
    }

    fn holder5133() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|P1_S|FO2_SS",
            &rows3(|m0: P0N, m1: P1S, m2: FO2SS| Holder5133 { own: 7, m0, m1, m2 }),
            &[("-,-,-", 'R')],
        )
    }

    fn holder5141() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|P1_S|T2_NN",
            &rows3(|m0: P0N, m1: P1S, m2: T2NN| Holder5141 { own: 7, m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder5149() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|T1_NN|FO2_NN",
            &rows3(|m0: P0N, m1: T1NN, m2: FO2NN| Holder5149 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder5157() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|T1_NN|V2_NN",
            &rows3(|m0: P0N, m1: T1NN, m2: V2NN| Holder5157 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5165() -> Vec<String> {
        check(
            "main3:H3N1:P0_N|V1_NN|O2_N",
            &rows3(|m0: P0N, m1: V1NN, m2: Option<OI2N>| Holder5165 { own: 7, m0, m1, m2 }),
            &[
                ("-,Rect,None", 'E'),
                ("-,Rect,Some", 'E'),
                ("-,Wrap,None", 'E'),
                ("-,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder5173() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|A1_NN|E2_NN nonstrict",
            &rows3(|m0: P0S, m1: A1NN, m2: E2NN| Holder5173 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5181() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|A1_NN|O2_S nonstrict",
            &rows3(|m0: P0S, m1: A1NN, m2: Option<OI2S>| Holder5181 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,None", 'E'),
                ("-,Dot,Some", 'E'),
                ("-,Rect,None", 'E'),
                ("-,Rect,Some", 'E'),
                ("-,Wrap,None", 'E'),
                ("-,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder5189() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|A1_SS|A2_SS nonstrict",
            &rows3(|m0: P0S, m1: A1SS, m2: A2SS| Holder5189 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5197() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|A1_SS|O2_N nonstrict",
            &rows3(|m0: P0S, m1: A1SS, m2: Option<OI2N>| Holder5197 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,None", 'E'),
                ("-,Dot,Some", 'E'),
                ("-,Rect,None", 'E'),
                ("-,Rect,Some", 'E'),
                ("-,Wrap,None", 'E'),
                ("-,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder5205() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|E1_NN|A2_NN nonstrict",
            &rows3(|m0: P0S, m1: E1NN, m2: A2NN| Holder5205 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5213() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|E1_NN|FT2_NNN",
            &rows3(|m0: P0S, m1: E1NN, m2: FT2NNN| Holder5213 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5221() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|E1_NN|V2_SS",
            &rows3(|m0: P0S, m1: E1NN, m2: V2SS| Holder5221 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Rect", 'R'),
                ("-,Dot,Wrap", 'R'),
                ("-,Rect,Rect", 'R'),
                ("-,Rect,Wrap", 'R'),
                ("-,Wrap,Rect", 'R'),
                ("-,Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder5229() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|E1_SS|FP2_SS",
            &rows3(|m0: P0S, m1: E1SS, m2: FP2SS| Holder5229 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder5237() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|E1_SS|V2_NN",
            &rows3(|m0: P0S, m1: E1SS, m2: V2NN| Holder5237 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5245() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|FO1_NN|FT2_NNN",
            &rows3(|m0: P0S, m1: FO1NN, m2: FT2NNN| Holder5245 { own: 7, m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder5253() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|FP1_NN|A2_SS",
            &rows3(|m0: P0S, m1: FP1NN, m2: A2SS| Holder5253 { own: 7, m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder5261() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|FP1_NN|P2_N",
            &rows3(|m0: P0S, m1: FP1NN, m2: P2N| Holder5261 { own: 7, m0, m1, m2 }),
            &[("-,-,-", 'E')],
        )
    }

    fn holder5269() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|FT1_NNN|FO2_NN",
            &rows3(|m0: P0S, m1: FT1NNN, m2: FO2NN| Holder5269 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'E'), ("-,Rect,-", 'E'), ("-,Wrap,-", 'E')],
        )
    }

    fn holder5277() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|FT1_NNN|V2_NN",
            &rows3(|m0: P0S, m1: FT1NNN, m2: V2NN| Holder5277 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5285() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|O1_N|FP2_SS",
            &rows3(|m0: P0S, m1: Option<OI1N>, m2: FP2SS| Holder5285 { own: 7, m0, m1, m2 }),
            &[("-,None,-", 'R'), ("-,Some,-", 'R')],
        )
    }

    fn holder5293() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|O1_N|V2_NN",
            &rows3(|m0: P0S, m1: Option<OI1N>, m2: V2NN| Holder5293 { own: 7, m0, m1, m2 }),
            &[
                ("-,None,Rect", 'E'),
                ("-,None,Wrap", 'E'),
                ("-,Some,Rect", 'E'),
                ("-,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder5301() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|O1_S|FP2_NN",
            &rows3(|m0: P0S, m1: Option<OI1S>, m2: FP2NN| Holder5301 { own: 7, m0, m1, m2 }),
            &[("-,None,-", 'E'), ("-,Some,-", 'E')],
        )
    }

    fn holder5309() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|O1_S|T2_SS",
            &rows3(|m0: P0S, m1: Option<OI1S>, m2: T2SS| Holder5309 { own: 7, m0, m1, m2 }),
            &[
                ("-,None,Dot", 'R'),
                ("-,None,Rect", 'R'),
                ("-,None,Wrap", 'R'),
                ("-,Some,Dot", 'R'),
                ("-,Some,Rect", 'R'),
                ("-,Some,Wrap", 'R'),
            ],
        )
    }

    fn holder5317() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|P1_N|FO2_SS",
            &rows3(|m0: P0S, m1: P1N, m2: FO2SS| Holder5317 { own: 7, m0, m1, m2 }),
            &[("-,-,-", 'R')],
        )
    }

    fn holder5325() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|P1_N|T2_NN",
            &rows3(|m0: P0S, m1: P1N, m2: T2NN| Holder5325 { own: 7, m0, m1, m2 }),
            &[("-,-,Dot", 'E'), ("-,-,Rect", 'E'), ("-,-,Wrap", 'E')],
        )
    }

    fn holder5333() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|P1_S|FO2_NN",
            &rows3(|m0: P0S, m1: P1S, m2: FO2NN| Holder5333 { own: 7, m0, m1, m2 }),
            &[("-,-,-", 'E')],
        )
    }

    fn holder5341() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|P1_S|P2_S nonstrict",
            &rows3(|m0: P0S, m1: P1S, m2: P2S| Holder5341 { own: 7, m0, m1, m2 }),
            &[("-,-,-", 'E')],
        )
    }

    fn holder5349() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|T1_NN|E2_SS",
            &rows3(|m0: P0S, m1: T1NN, m2: E2SS| Holder5349 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5357() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|T1_NN|T2_NN",
            &rows3(|m0: P0S, m1: T1NN, m2: T2NN| Holder5357 { own: 7, m0, m1, m2 }),
            &[
                ("-,Dot,Dot", 'E'),
                ("-,Dot,Rect", 'E'),
                ("-,Dot,Wrap", 'E'),
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5365() -> Vec<String> {
        check(
            "main3:H3N1:P0_S|V1_NN|FT2_NNN",
            &rows3(|m0: P0S, m1: V1NN, m2: FT2NNN| Holder5365 { own: 7, m0, m1, m2 }),
            &[
                ("-,Rect,Dot", 'E'),
                ("-,Rect,Rect", 'E'),
                ("-,Rect,Wrap", 'E'),
                ("-,Wrap,Dot", 'E'),
                ("-,Wrap,Rect", 'E'),
                ("-,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5373() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|A1_NN|A2_SS",
            &rows3(|m0: T0NN, m1: A1NN, m2: A2SS| Holder5373 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5381() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|A1_NN|P2_N",
            &rows3(|m0: T0NN, m1: A1NN, m2: P2N| Holder5381 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder5389() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|A1_SS|FO2_NN",
            &rows3(|m0: T0NN, m1: A1SS, m2: FO2NN| Holder5389 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder5397() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|A1_SS|V2_NN",
            &rows3(|m0: T0NN, m1: A1SS, m2: V2NN| Holder5397 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5405() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|E1_NN|O2_N",
            &rows3(|m0: T0NN, m1: E1NN, m2: Option<OI2N>| Holder5405 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,None", 'E'),
                ("Dot,Dot,Some", 'E'),
                ("Dot,Rect,None", 'E'),
                ("Dot,Rect,Some", 'E'),
                ("Dot,Wrap,None", 'E'),
                ("Dot,Wrap,Some", 'E'),
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder5413() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|E1_SS|E2_NN",
            &rows3(|m0: T0NN, m1: E1SS, m2: E2NN| Holder5413 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5421() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|E1_SS|P2_S",
            &rows3(|m0: T0NN, m1: E1SS, m2: P2S| Holder5421 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder5429() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|FO1_NN|FP2_NN",
            &rows3(|m0: T0NN, m1: FO1NN, m2: FP2NN| Holder5429 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder5437() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|FP1_NN|A2_NN",
            &rows3(|m0: T0NN, m1: FP1NN, m2: A2NN| Holder5437 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Dot", 'E'),
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder5445() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|FP1_NN|O2_S",
            &rows3(|m0: T0NN, m1: FP1NN, m2: Option<OI2S>| Holder5445 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,None", 'E'),
                ("Dot,-,Some", 'E'),
                ("Rect,-,None", 'E'),
                ("Rect,-,Some", 'E'),
                ("Wrap,-,None", 'E'),
                ("Wrap,-,Some", 'E'),
            ],
        )
    }

    fn holder5453() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|FT1_NNN|E2_SS",
            &rows3(|m0: T0NN, m1: FT1NNN, m2: E2SS| Holder5453 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5461() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|FT1_NNN|T2_NN",
            &rows3(|m0: T0NN, m1: FT1NNN, m2: T2NN| Holder5461 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5469() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|O1_N|FT2_NNN",
            &rows3(|m0: T0NN, m1: Option<OI1N>, m2: FT2NNN| Holder5469 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder5477() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|O1_S|A2_SS",
            &rows3(|m0: T0NN, m1: Option<OI1S>, m2: A2SS| Holder5477 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,Dot", 'E'),
                ("Dot,None,Rect", 'E'),
                ("Dot,None,Wrap", 'E'),
                ("Dot,Some,Dot", 'E'),
                ("Dot,Some,Rect", 'E'),
                ("Dot,Some,Wrap", 'E'),
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder5485() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|O1_S|P2_N",
            &rows3(|m0: T0NN, m1: Option<OI1S>, m2: P2N| Holder5485 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,-", 'E'),
                ("Dot,Some,-", 'E'),
                ("Rect,None,-", 'E'),
                ("Rect,Some,-", 'E'),
                ("Wrap,None,-", 'E'),
                ("Wrap,Some,-", 'E'),
            ],
        )
    }

    fn holder5493() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|P1_N|FO2_NN",
            &rows3(|m0: T0NN, m1: P1N, m2: FO2NN| Holder5493 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'E'), ("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder5501() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|P1_N|V2_NN",
            &rows3(|m0: T0NN, m1: P1N, m2: V2NN| Holder5501 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,Rect", 'E'),
                ("Dot,-,Wrap", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder5509() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|P1_S|O2_N",
            &rows3(|m0: T0NN, m1: P1S, m2: Option<OI2N>| Holder5509 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,-,None", 'E'),
                ("Dot,-,Some", 'E'),
                ("Rect,-,None", 'E'),
                ("Rect,-,Some", 'E'),
                ("Wrap,-,None", 'E'),
                ("Wrap,-,Some", 'E'),
            ],
        )
    }

    fn holder5517() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|T1_NN|E2_NN",
            &rows3(|m0: T0NN, m1: T1NN, m2: E2NN| Holder5517 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,Dot", 'E'),
                ("Dot,Dot,Rect", 'E'),
                ("Dot,Dot,Wrap", 'E'),
                ("Dot,Rect,Dot", 'E'),
                ("Dot,Rect,Rect", 'E'),
                ("Dot,Rect,Wrap", 'E'),
                ("Dot,Wrap,Dot", 'E'),
                ("Dot,Wrap,Rect", 'E'),
                ("Dot,Wrap,Wrap", 'E'),
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5525() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|T1_NN|P2_S",
            &rows3(|m0: T0NN, m1: T1NN, m2: P2S| Holder5525 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'E'),
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder5533() -> Vec<String> {
        check(
            "main3:H3N1:T0_NN|V1_NN|FP2_NN",
            &rows3(|m0: T0NN, m1: V1NN, m2: FP2NN| Holder5533 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Rect,-", 'E'),
                ("Dot,Wrap,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder5541() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|A1_NN|A2_NN",
            &rows3(|m0: V0NN, m1: A1NN, m2: A2NN| Holder5541 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5549() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|A1_NN|O2_S",
            &rows3(|m0: V0NN, m1: A1NN, m2: Option<OI2S>| Holder5549 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,Dot,None", 'E'),
                ("Rect,Dot,Some", 'E'),
                ("Rect,Rect,None", 'E'),
                ("Rect,Rect,Some", 'E'),
                ("Rect,Wrap,None", 'E'),
                ("Rect,Wrap,Some", 'E'),
                ("Wrap,Dot,None", 'E'),
                ("Wrap,Dot,Some", 'E'),
                ("Wrap,Rect,None", 'E'),
                ("Wrap,Rect,Some", 'E'),
                ("Wrap,Wrap,None", 'E'),
                ("Wrap,Wrap,Some", 'E'),
            ],
        )
    }

    fn holder5557() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|A1_SS|E2_SS",
            &rows3(|m0: V0NN, m1: A1SS, m2: E2SS| Holder5557 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5565() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|A1_SS|T2_NN",
            &rows3(|m0: V0NN, m1: A1SS, m2: T2NN| Holder5565 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5573() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|E1_NN|FT2_NNN",
            &rows3(|m0: V0NN, m1: E1NN, m2: FT2NNN| Holder5573 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5581() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|E1_SS|A2_SS",
            &rows3(|m0: V0NN, m1: E1SS, m2: A2SS| Holder5581 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5589() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|E1_SS|P2_N",
            &rows3(|m0: V0NN, m1: E1SS, m2: P2N| Holder5589 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder5597() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|FO1_NN|FO2_NN",
            &rows3(|m0: V0NN, m1: FO1NN, m2: FO2NN| Holder5597 { own: 7, m0, m1, m2 }),
            &[("Rect,-,-", 'E'), ("Wrap,-,-", 'E')],
        )
    }

    fn holder5605() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|FO1_NN|V2_NN",
            &rows3(|m0: V0NN, m1: FO1NN, m2: V2NN| Holder5605 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder5613() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|FP1_NN|O2_N",
            &rows3(|m0: V0NN, m1: FP1NN, m2: Option<OI2N>| Holder5613 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,-,None", 'E'),
                ("Rect,-,Some", 'E'),
                ("Wrap,-,None", 'E'),
                ("Wrap,-,Some", 'E'),
            ],
        )
    }

    fn holder5621() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|FT1_NNN|E2_NN",
            &rows3(|m0: V0NN, m1: FT1NNN, m2: E2NN| Holder5621 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5629() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|FT1_NNN|P2_S",
            &rows3(|m0: V0NN, m1: FT1NNN, m2: P2S| Holder5629 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder5637() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|O1_N|FP2_NN",
            &rows3(|m0: V0NN, m1: Option<OI1N>, m2: FP2NN| Holder5637 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,None,-", 'E'),
                ("Rect,Some,-", 'E'),
                ("Wrap,None,-", 'E'),
                ("Wrap,Some,-", 'E'),
            ],
        )
    }

    fn holder5645() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|O1_S|A2_NN",
            &rows3(|m0: V0NN, m1: Option<OI1S>, m2: A2NN| Holder5645 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,None,Dot", 'E'),
                ("Rect,None,Rect", 'E'),
                ("Rect,None,Wrap", 'E'),
                ("Rect,Some,Dot", 'E'),
                ("Rect,Some,Rect", 'E'),
                ("Rect,Some,Wrap", 'E'),
                ("Wrap,None,Dot", 'E'),
                ("Wrap,None,Rect", 'E'),
                ("Wrap,None,Wrap", 'E'),
                ("Wrap,Some,Dot", 'E'),
                ("Wrap,Some,Rect", 'E'),
                ("Wrap,Some,Wrap", 'E'),
            ],
        )
    }

    fn holder5653() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|O1_S|O2_S",
            &rows3(|m0: V0NN, m1: Option<OI1S>, m2: Option<OI2S>| Holder5653 {
                own: 7,
                m0,
                m1,
                m2,
            }),
            &[
                ("Rect,None,None", 'E'),
                ("Rect,None,Some", 'E'),
                ("Rect,Some,None", 'E'),
                ("Rect,Some,Some", 'E'),
                ("Wrap,None,None", 'E'),
                ("Wrap,None,Some", 'E'),
                ("Wrap,Some,None", 'E'),
                ("Wrap,Some,Some", 'E'),
            ],
        )
    }

    fn holder5661() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|P1_N|E2_SS",
            &rows3(|m0: V0NN, m1: P1N, m2: E2SS| Holder5661 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder5669() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|P1_N|T2_NN",
            &rows3(|m0: V0NN, m1: P1N, m2: T2NN| Holder5669 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder5677() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|P1_S|FT2_NNN",
            &rows3(|m0: V0NN, m1: P1S, m2: FT2NNN| Holder5677 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,-,Dot", 'E'),
                ("Rect,-,Rect", 'E'),
                ("Rect,-,Wrap", 'E'),
                ("Wrap,-,Dot", 'E'),
                ("Wrap,-,Rect", 'E'),
                ("Wrap,-,Wrap", 'E'),
            ],
        )
    }

    fn holder5685() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|T1_NN|A2_SS",
            &rows3(|m0: V0NN, m1: T1NN, m2: A2SS| Holder5685 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,Dot,Dot", 'E'),
                ("Rect,Dot,Rect", 'E'),
                ("Rect,Dot,Wrap", 'E'),
                ("Rect,Rect,Dot", 'E'),
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Dot", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Dot,Dot", 'E'),
                ("Wrap,Dot,Rect", 'E'),
                ("Wrap,Dot,Wrap", 'E'),
                ("Wrap,Rect,Dot", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Dot", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5693() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|T1_NN|P2_N",
            &rows3(|m0: V0NN, m1: T1NN, m2: P2N| Holder5693 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,Dot,-", 'E'),
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Dot,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder5701() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|V1_NN|FO2_NN",
            &rows3(|m0: V0NN, m1: V1NN, m2: FO2NN| Holder5701 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,Rect,-", 'E'),
                ("Rect,Wrap,-", 'E'),
                ("Wrap,Rect,-", 'E'),
                ("Wrap,Wrap,-", 'E'),
            ],
        )
    }

    fn holder5709() -> Vec<String> {
        check(
            "main3:H3N1:V0_NN|V1_NN|V2_NN",
            &rows3(|m0: V0NN, m1: V1NN, m2: V2NN| Holder5709 { own: 7, m0, m1, m2 }),
            &[
                ("Rect,Rect,Rect", 'E'),
                ("Rect,Rect,Wrap", 'E'),
                ("Rect,Wrap,Rect", 'E'),
                ("Rect,Wrap,Wrap", 'E'),
                ("Wrap,Rect,Rect", 'E'),
                ("Wrap,Rect,Wrap", 'E'),
                ("Wrap,Wrap,Rect", 'E'),
                ("Wrap,Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder5717() -> Vec<String> {
        check(
            "main3:H3S0:A0_NN|A1_NN|P2_S",
            &rows3(|m0: A0NN, m1: A1NN, m2: P2S| Holder5717 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5725() -> Vec<String> {
        check(
            "main3:H3S0:A0_NN|A1_SS|P2_S",
            &rows3(|m0: A0NN, m1: A1SS, m2: P2S| Holder5725 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5733() -> Vec<String> {
        check(
            "main3:H3S0:A0_NN|E1_NN|P2_S",
            &rows3(|m0: A0NN, m1: E1NN, m2: P2S| Holder5733 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5741() -> Vec<String> {
        check(
            "main3:H3S0:A0_NN|E1_SS|P2_S",
            &rows3(|m0: A0NN, m1: E1SS, m2: P2S| Holder5741 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5749() -> Vec<String> {
        check(
            "main3:H3S0:A0_NN|O1_N|P2_S",
            &rows3(|m0: A0NN, m1: Option<OI1N>, m2: P2S| Holder5749 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder5757() -> Vec<String> {
        check(
            "main3:H3S0:A0_NN|O1_S|P2_S",
            &rows3(|m0: A0NN, m1: Option<OI1S>, m2: P2S| Holder5757 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder5765() -> Vec<String> {
        check(
            "main3:H3S0:A0_NN|P1_N|P2_S",
            &rows3(|m0: A0NN, m1: P1N, m2: P2S| Holder5765 { m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder5773() -> Vec<String> {
        check(
            "main3:H3S0:A0_NN|P1_S|P2_S",
            &rows3(|m0: A0NN, m1: P1S, m2: P2S| Holder5773 { m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder5781() -> Vec<String> {
        check(
            "main3:H3S0:A0_SS|A1_NN|P2_S",
            &rows3(|m0: A0SS, m1: A1NN, m2: P2S| Holder5781 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5789() -> Vec<String> {
        check(
            "main3:H3S0:A0_SS|A1_SS|P2_S",
            &rows3(|m0: A0SS, m1: A1SS, m2: P2S| Holder5789 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5797() -> Vec<String> {
        check(
            "main3:H3S0:A0_SS|E1_NN|P2_S",
            &rows3(|m0: A0SS, m1: E1NN, m2: P2S| Holder5797 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5805() -> Vec<String> {
        check(
            "main3:H3S0:A0_SS|E1_SS|P2_S",
            &rows3(|m0: A0SS, m1: E1SS, m2: P2S| Holder5805 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5813() -> Vec<String> {
        check(
            "main3:H3S0:A0_SS|O1_N|P2_S",
            &rows3(|m0: A0SS, m1: Option<OI1N>, m2: P2S| Holder5813 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder5821() -> Vec<String> {
        check(
            "main3:H3S0:A0_SS|O1_S|P2_S",
            &rows3(|m0: A0SS, m1: Option<OI1S>, m2: P2S| Holder5821 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder5829() -> Vec<String> {
        check(
            "main3:H3S0:A0_SS|P1_N|P2_S",
            &rows3(|m0: A0SS, m1: P1N, m2: P2S| Holder5829 { m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder5837() -> Vec<String> {
        check(
            "main3:H3S0:A0_SS|P1_S|P2_S",
            &rows3(|m0: A0SS, m1: P1S, m2: P2S| Holder5837 { m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder5845() -> Vec<String> {
        check(
            "main3:H3S0:E0_NN|A1_NN|P2_S",
            &rows3(|m0: E0NN, m1: A1NN, m2: P2S| Holder5845 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5853() -> Vec<String> {
        check(
            "main3:H3S0:E0_NN|A1_SS|P2_S",
            &rows3(|m0: E0NN, m1: A1SS, m2: P2S| Holder5853 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5861() -> Vec<String> {
        check(
            "main3:H3S0:E0_NN|E1_NN|P2_S",
            &rows3(|m0: E0NN, m1: E1NN, m2: P2S| Holder5861 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5869() -> Vec<String> {
        check(
            "main3:H3S0:E0_NN|E1_SS|P2_S",
            &rows3(|m0: E0NN, m1: E1SS, m2: P2S| Holder5869 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5877() -> Vec<String> {
        check(
            "main3:H3S0:E0_NN|O1_N|P2_S",
            &rows3(|m0: E0NN, m1: Option<OI1N>, m2: P2S| Holder5877 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder5885() -> Vec<String> {
        check(
            "main3:H3S0:E0_NN|O1_S|P2_S",
            &rows3(|m0: E0NN, m1: Option<OI1S>, m2: P2S| Holder5885 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder5893() -> Vec<String> {
        check(
            "main3:H3S0:E0_NN|P1_N|P2_S",
            &rows3(|m0: E0NN, m1: P1N, m2: P2S| Holder5893 { m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder5901() -> Vec<String> {
        check(
            "main3:H3S0:E0_NN|P1_S|P2_S",
            &rows3(|m0: E0NN, m1: P1S, m2: P2S| Holder5901 { m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder5909() -> Vec<String> {
        check(
            "main3:H3S0:E0_SS|A1_NN|P2_S",
            &rows3(|m0: E0SS, m1: A1NN, m2: P2S| Holder5909 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5917() -> Vec<String> {
        check(
            "main3:H3S0:E0_SS|A1_SS|P2_S",
            &rows3(|m0: E0SS, m1: A1SS, m2: P2S| Holder5917 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5925() -> Vec<String> {
        check(
            "main3:H3S0:E0_SS|E1_NN|P2_S",
            &rows3(|m0: E0SS, m1: E1NN, m2: P2S| Holder5925 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5933() -> Vec<String> {
        check(
            "main3:H3S0:E0_SS|E1_SS|P2_S",
            &rows3(|m0: E0SS, m1: E1SS, m2: P2S| Holder5933 { m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5941() -> Vec<String> {
        check(
            "main3:H3S0:E0_SS|O1_N|P2_S",
            &rows3(|m0: E0SS, m1: Option<OI1N>, m2: P2S| Holder5941 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder5949() -> Vec<String> {
        check(
            "main3:H3S0:E0_SS|O1_S|P2_S",
            &rows3(|m0: E0SS, m1: Option<OI1S>, m2: P2S| Holder5949 { m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder5957() -> Vec<String> {
        check(
            "main3:H3S0:E0_SS|P1_N|P2_S",
            &rows3(|m0: E0SS, m1: P1N, m2: P2S| Holder5957 { m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder5965() -> Vec<String> {
        check(
            "main3:H3S0:E0_SS|P1_S|P2_S",
            &rows3(|m0: E0SS, m1: P1S, m2: P2S| Holder5965 { m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder5973() -> Vec<String> {
        check(
            "main3:H3S0:O0_N|A1_NN|P2_S",
            &rows3(|m0: Option<OI0N>, m1: A1NN, m2: P2S| Holder5973 { m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5981() -> Vec<String> {
        check(
            "main3:H3S0:O0_N|A1_SS|P2_S",
            &rows3(|m0: Option<OI0N>, m1: A1SS, m2: P2S| Holder5981 { m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5989() -> Vec<String> {
        check(
            "main3:H3S0:O0_N|E1_NN|P2_S",
            &rows3(|m0: Option<OI0N>, m1: E1NN, m2: P2S| Holder5989 { m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder5997() -> Vec<String> {
        check(
            "main3:H3S0:O0_N|E1_SS|P2_S",
            &rows3(|m0: Option<OI0N>, m1: E1SS, m2: P2S| Holder5997 { m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6005() -> Vec<String> {
        check(
            "main3:H3S0:O0_N|O1_N|P2_S",
            &rows3(|m0: Option<OI0N>, m1: Option<OI1N>, m2: P2S| Holder6005 { m0, m1, m2 }),
            &[
                ("None,None,-", 'R'),
                ("None,Some,-", 'R'),
                ("Some,None,-", 'R'),
                ("Some,Some,-", 'R'),
            ],
        )
    }

    fn holder6013() -> Vec<String> {
        check(
            "main3:H3S0:O0_N|O1_S|P2_S",
            &rows3(|m0: Option<OI0N>, m1: Option<OI1S>, m2: P2S| Holder6013 { m0, m1, m2 }),
            &[
                ("None,None,-", 'R'),
                ("None,Some,-", 'R'),
                ("Some,None,-", 'R'),
                ("Some,Some,-", 'R'),
            ],
        )
    }

    fn holder6021() -> Vec<String> {
        check(
            "main3:H3S0:O0_N|P1_N|P2_S",
            &rows3(|m0: Option<OI0N>, m1: P1N, m2: P2S| Holder6021 { m0, m1, m2 }),
            &[("None,-,-", 'R'), ("Some,-,-", 'R')],
        )
    }

    fn holder6029() -> Vec<String> {
        check(
            "main3:H3S0:O0_N|P1_S|P2_S",
            &rows3(|m0: Option<OI0N>, m1: P1S, m2: P2S| Holder6029 { m0, m1, m2 }),
            &[("None,-,-", 'R'), ("Some,-,-", 'R')],
        )
    }

    fn holder6037() -> Vec<String> {
        check(
            "main3:H3S0:O0_S|A1_NN|P2_S",
            &rows3(|m0: Option<OI0S>, m1: A1NN, m2: P2S| Holder6037 { m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6045() -> Vec<String> {
        check(
            "main3:H3S0:O0_S|A1_SS|P2_S",
            &rows3(|m0: Option<OI0S>, m1: A1SS, m2: P2S| Holder6045 { m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6053() -> Vec<String> {
        check(
            "main3:H3S0:O0_S|E1_NN|P2_S",
            &rows3(|m0: Option<OI0S>, m1: E1NN, m2: P2S| Holder6053 { m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6061() -> Vec<String> {
        check(
            "main3:H3S0:O0_S|E1_SS|P2_S",
            &rows3(|m0: Option<OI0S>, m1: E1SS, m2: P2S| Holder6061 { m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6069() -> Vec<String> {
        check(
            "main3:H3S0:O0_S|O1_N|P2_S",
            &rows3(|m0: Option<OI0S>, m1: Option<OI1N>, m2: P2S| Holder6069 { m0, m1, m2 }),
            &[
                ("None,None,-", 'R'),
                ("None,Some,-", 'R'),
                ("Some,None,-", 'R'),
                ("Some,Some,-", 'R'),
            ],
        )
    }

    fn holder6077() -> Vec<String> {
        check(
            "main3:H3S0:O0_S|O1_S|P2_S",
            &rows3(|m0: Option<OI0S>, m1: Option<OI1S>, m2: P2S| Holder6077 { m0, m1, m2 }),
            &[
                ("None,None,-", 'R'),
                ("None,Some,-", 'R'),
                ("Some,None,-", 'R'),
                ("Some,Some,-", 'R'),
            ],
        )
    }

    fn holder6085() -> Vec<String> {
        check(
            "main3:H3S0:O0_S|P1_N|P2_S",
            &rows3(|m0: Option<OI0S>, m1: P1N, m2: P2S| Holder6085 { m0, m1, m2 }),
            &[("None,-,-", 'R'), ("Some,-,-", 'R')],
        )
    }

    fn holder6093() -> Vec<String> {
        check(
            "main3:H3S0:O0_S|P1_S|P2_S",
            &rows3(|m0: Option<OI0S>, m1: P1S, m2: P2S| Holder6093 { m0, m1, m2 }),
            &[("None,-,-", 'R'), ("Some,-,-", 'R')],
        )
    }

    fn holder6101() -> Vec<String> {
        check(
            "main3:H3S0:P0_N|A1_NN|P2_S",
            &rows3(|m0: P0N, m1: A1NN, m2: P2S| Holder6101 { m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6109() -> Vec<String> {
        check(
            "main3:H3S0:P0_N|A1_SS|P2_S",
            &rows3(|m0: P0N, m1: A1SS, m2: P2S| Holder6109 { m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6117() -> Vec<String> {
        check(
            "main3:H3S0:P0_N|E1_NN|P2_S",
            &rows3(|m0: P0N, m1: E1NN, m2: P2S| Holder6117 { m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6125() -> Vec<String> {
        check(
            "main3:H3S0:P0_N|E1_SS|P2_S",
            &rows3(|m0: P0N, m1: E1SS, m2: P2S| Holder6125 { m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6133() -> Vec<String> {
        check(
            "main3:H3S0:P0_N|O1_N|P2_S",
            &rows3(|m0: P0N, m1: Option<OI1N>, m2: P2S| Holder6133 { m0, m1, m2 }),
            &[("-,None,-", 'R'), ("-,Some,-", 'R')],
        )
    }

    fn holder6141() -> Vec<String> {
        check(
            "main3:H3S0:P0_N|O1_S|P2_S",
            &rows3(|m0: P0N, m1: Option<OI1S>, m2: P2S| Holder6141 { m0, m1, m2 }),
            &[("-,None,-", 'R'), ("-,Some,-", 'R')],
        )
    }

    fn holder6149() -> Vec<String> {
        check(
            "main3:H3S0:P0_N|P1_N|P2_S",
            &rows3(|m0: P0N, m1: P1N, m2: P2S| Holder6149 { m0, m1, m2 }),
            &[("-,-,-", 'R')],
        )
    }

    fn holder6157() -> Vec<String> {
        check(
            "main3:H3S0:P0_N|P1_S|P2_S",
            &rows3(|m0: P0N, m1: P1S, m2: P2S| Holder6157 { m0, m1, m2 }),
            &[("-,-,-", 'R')],
        )
    }

    fn holder6165() -> Vec<String> {
        check(
            "main3:H3S0:P0_S|A1_NN|P2_S",
            &rows3(|m0: P0S, m1: A1NN, m2: P2S| Holder6165 { m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6173() -> Vec<String> {
        check(
            "main3:H3S0:P0_S|A1_SS|P2_S",
            &rows3(|m0: P0S, m1: A1SS, m2: P2S| Holder6173 { m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6181() -> Vec<String> {
        check(
            "main3:H3S0:P0_S|E1_NN|P2_S",
            &rows3(|m0: P0S, m1: E1NN, m2: P2S| Holder6181 { m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6189() -> Vec<String> {
        check(
            "main3:H3S0:P0_S|E1_SS|P2_S",
            &rows3(|m0: P0S, m1: E1SS, m2: P2S| Holder6189 { m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6197() -> Vec<String> {
        check(
            "main3:H3S0:P0_S|O1_N|P2_S",
            &rows3(|m0: P0S, m1: Option<OI1N>, m2: P2S| Holder6197 { m0, m1, m2 }),
            &[("-,None,-", 'R'), ("-,Some,-", 'R')],
        )
    }

    fn holder6205() -> Vec<String> {
        check(
            "main3:H3S0:P0_S|O1_S|P2_S",
            &rows3(|m0: P0S, m1: Option<OI1S>, m2: P2S| Holder6205 { m0, m1, m2 }),
            &[("-,None,-", 'R'), ("-,Some,-", 'R')],
        )
    }

    fn holder6213() -> Vec<String> {
        check(
            "main3:H3S0:P0_S|P1_N|P2_S",
            &rows3(|m0: P0S, m1: P1N, m2: P2S| Holder6213 { m0, m1, m2 }),
            &[("-,-,-", 'R')],
        )
    }

    fn holder6221() -> Vec<String> {
        check(
            "main3:H3S0:P0_S|P1_S|P2_S",
            &rows3(|m0: P0S, m1: P1S, m2: P2S| Holder6221 { m0, m1, m2 }),
            &[("-,-,-", 'R')],
        )
    }

    fn holder6229() -> Vec<String> {
        check(
            "main3:H3S1:A0_NN|A1_NN|P2_S",
            &rows3(|m0: A0NN, m1: A1NN, m2: P2S| Holder6229 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6237() -> Vec<String> {
        check(
            "main3:H3S1:A0_NN|A1_SS|P2_S",
            &rows3(|m0: A0NN, m1: A1SS, m2: P2S| Holder6237 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6245() -> Vec<String> {
        check(
            "main3:H3S1:A0_NN|E1_NN|P2_S",
            &rows3(|m0: A0NN, m1: E1NN, m2: P2S| Holder6245 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6253() -> Vec<String> {
        check(
            "main3:H3S1:A0_NN|E1_SS|P2_S",
            &rows3(|m0: A0NN, m1: E1SS, m2: P2S| Holder6253 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6261() -> Vec<String> {
        check(
            "main3:H3S1:A0_NN|O1_N|P2_S",
            &rows3(|m0: A0NN, m1: Option<OI1N>, m2: P2S| Holder6261 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder6269() -> Vec<String> {
        check(
            "main3:H3S1:A0_NN|O1_S|P2_S",
            &rows3(|m0: A0NN, m1: Option<OI1S>, m2: P2S| Holder6269 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder6277() -> Vec<String> {
        check(
            "main3:H3S1:A0_NN|P1_N|P2_S",
            &rows3(|m0: A0NN, m1: P1N, m2: P2S| Holder6277 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder6285() -> Vec<String> {
        check(
            "main3:H3S1:A0_NN|P1_S|P2_S",
            &rows3(|m0: A0NN, m1: P1S, m2: P2S| Holder6285 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder6293() -> Vec<String> {
        check(
            "main3:H3S1:A0_SS|A1_NN|P2_S",
            &rows3(|m0: A0SS, m1: A1NN, m2: P2S| Holder6293 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6301() -> Vec<String> {
        check(
            "main3:H3S1:A0_SS|A1_SS|P2_S",
            &rows3(|m0: A0SS, m1: A1SS, m2: P2S| Holder6301 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6309() -> Vec<String> {
        check(
            "main3:H3S1:A0_SS|E1_NN|P2_S",
            &rows3(|m0: A0SS, m1: E1NN, m2: P2S| Holder6309 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6317() -> Vec<String> {
        check(
            "main3:H3S1:A0_SS|E1_SS|P2_S",
            &rows3(|m0: A0SS, m1: E1SS, m2: P2S| Holder6317 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6325() -> Vec<String> {
        check(
            "main3:H3S1:A0_SS|O1_N|P2_S",
            &rows3(|m0: A0SS, m1: Option<OI1N>, m2: P2S| Holder6325 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder6333() -> Vec<String> {
        check(
            "main3:H3S1:A0_SS|O1_S|P2_S",
            &rows3(|m0: A0SS, m1: Option<OI1S>, m2: P2S| Holder6333 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder6341() -> Vec<String> {
        check(
            "main3:H3S1:A0_SS|P1_N|P2_S",
            &rows3(|m0: A0SS, m1: P1N, m2: P2S| Holder6341 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder6349() -> Vec<String> {
        check(
            "main3:H3S1:A0_SS|P1_S|P2_S",
            &rows3(|m0: A0SS, m1: P1S, m2: P2S| Holder6349 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder6357() -> Vec<String> {
        check(
            "main3:H3S1:E0_NN|A1_NN|P2_S",
            &rows3(|m0: E0NN, m1: A1NN, m2: P2S| Holder6357 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6365() -> Vec<String> {
        check(
            "main3:H3S1:E0_NN|A1_SS|P2_S",
            &rows3(|m0: E0NN, m1: A1SS, m2: P2S| Holder6365 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6373() -> Vec<String> {
        check(
            "main3:H3S1:E0_NN|E1_NN|P2_S",
            &rows3(|m0: E0NN, m1: E1NN, m2: P2S| Holder6373 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6381() -> Vec<String> {
        check(
            "main3:H3S1:E0_NN|E1_SS|P2_S",
            &rows3(|m0: E0NN, m1: E1SS, m2: P2S| Holder6381 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6389() -> Vec<String> {
        check(
            "main3:H3S1:E0_NN|O1_N|P2_S",
            &rows3(|m0: E0NN, m1: Option<OI1N>, m2: P2S| Holder6389 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder6397() -> Vec<String> {
        check(
            "main3:H3S1:E0_NN|O1_S|P2_S",
            &rows3(|m0: E0NN, m1: Option<OI1S>, m2: P2S| Holder6397 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder6405() -> Vec<String> {
        check(
            "main3:H3S1:E0_NN|P1_N|P2_S",
            &rows3(|m0: E0NN, m1: P1N, m2: P2S| Holder6405 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder6413() -> Vec<String> {
        check(
            "main3:H3S1:E0_NN|P1_S|P2_S",
            &rows3(|m0: E0NN, m1: P1S, m2: P2S| Holder6413 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder6421() -> Vec<String> {
        check(
            "main3:H3S1:E0_SS|A1_NN|P2_S",
            &rows3(|m0: E0SS, m1: A1NN, m2: P2S| Holder6421 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6429() -> Vec<String> {
        check(
            "main3:H3S1:E0_SS|A1_SS|P2_S",
            &rows3(|m0: E0SS, m1: A1SS, m2: P2S| Holder6429 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6437() -> Vec<String> {
        check(
            "main3:H3S1:E0_SS|E1_NN|P2_S",
            &rows3(|m0: E0SS, m1: E1NN, m2: P2S| Holder6437 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6445() -> Vec<String> {
        check(
            "main3:H3S1:E0_SS|E1_SS|P2_S",
            &rows3(|m0: E0SS, m1: E1SS, m2: P2S| Holder6445 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,Dot,-", 'R'),
                ("Dot,Rect,-", 'R'),
                ("Dot,Wrap,-", 'R'),
                ("Rect,Dot,-", 'R'),
                ("Rect,Rect,-", 'R'),
                ("Rect,Wrap,-", 'R'),
                ("Wrap,Dot,-", 'R'),
                ("Wrap,Rect,-", 'R'),
                ("Wrap,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6453() -> Vec<String> {
        check(
            "main3:H3S1:E0_SS|O1_N|P2_S",
            &rows3(|m0: E0SS, m1: Option<OI1N>, m2: P2S| Holder6453 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder6461() -> Vec<String> {
        check(
            "main3:H3S1:E0_SS|O1_S|P2_S",
            &rows3(|m0: E0SS, m1: Option<OI1S>, m2: P2S| Holder6461 { own: 7, m0, m1, m2 }),
            &[
                ("Dot,None,-", 'R'),
                ("Dot,Some,-", 'R'),
                ("Rect,None,-", 'R'),
                ("Rect,Some,-", 'R'),
                ("Wrap,None,-", 'R'),
                ("Wrap,Some,-", 'R'),
            ],
        )
    }

    fn holder6469() -> Vec<String> {
        check(
            "main3:H3S1:E0_SS|P1_N|P2_S",
            &rows3(|m0: E0SS, m1: P1N, m2: P2S| Holder6469 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder6477() -> Vec<String> {
        check(
            "main3:H3S1:E0_SS|P1_S|P2_S",
            &rows3(|m0: E0SS, m1: P1S, m2: P2S| Holder6477 { own: 7, m0, m1, m2 }),
            &[("Dot,-,-", 'R'), ("Rect,-,-", 'R'), ("Wrap,-,-", 'R')],
        )
    }

    fn holder6485() -> Vec<String> {
        check(
            "main3:H3S1:O0_N|A1_NN|P2_S",
            &rows3(|m0: Option<OI0N>, m1: A1NN, m2: P2S| Holder6485 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6493() -> Vec<String> {
        check(
            "main3:H3S1:O0_N|A1_SS|P2_S",
            &rows3(|m0: Option<OI0N>, m1: A1SS, m2: P2S| Holder6493 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6501() -> Vec<String> {
        check(
            "main3:H3S1:O0_N|E1_NN|P2_S",
            &rows3(|m0: Option<OI0N>, m1: E1NN, m2: P2S| Holder6501 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6509() -> Vec<String> {
        check(
            "main3:H3S1:O0_N|E1_SS|P2_S",
            &rows3(|m0: Option<OI0N>, m1: E1SS, m2: P2S| Holder6509 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6517() -> Vec<String> {
        check(
            "main3:H3S1:O0_N|O1_N|P2_S",
            &rows3(|m0: Option<OI0N>, m1: Option<OI1N>, m2: P2S| Holder6517 { own: 7, m0, m1, m2 }),
            &[
                ("None,None,-", 'R'),
                ("None,Some,-", 'R'),
                ("Some,None,-", 'R'),
                ("Some,Some,-", 'R'),
            ],
        )
    }

    fn holder6525() -> Vec<String> {
        check(
            "main3:H3S1:O0_N|O1_S|P2_S",
            &rows3(|m0: Option<OI0N>, m1: Option<OI1S>, m2: P2S| Holder6525 { own: 7, m0, m1, m2 }),
            &[
                ("None,None,-", 'R'),
                ("None,Some,-", 'R'),
                ("Some,None,-", 'R'),
                ("Some,Some,-", 'R'),
            ],
        )
    }

    fn holder6533() -> Vec<String> {
        check(
            "main3:H3S1:O0_N|P1_N|P2_S",
            &rows3(|m0: Option<OI0N>, m1: P1N, m2: P2S| Holder6533 { own: 7, m0, m1, m2 }),
            &[("None,-,-", 'R'), ("Some,-,-", 'R')],
        )
    }

    fn holder6541() -> Vec<String> {
        check(
            "main3:H3S1:O0_N|P1_S|P2_S",
            &rows3(|m0: Option<OI0N>, m1: P1S, m2: P2S| Holder6541 { own: 7, m0, m1, m2 }),
            &[("None,-,-", 'R'), ("Some,-,-", 'R')],
        )
    }

    fn holder6549() -> Vec<String> {
        check(
            "main3:H3S1:O0_S|A1_NN|P2_S",
            &rows3(|m0: Option<OI0S>, m1: A1NN, m2: P2S| Holder6549 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6557() -> Vec<String> {
        check(
            "main3:H3S1:O0_S|A1_SS|P2_S",
            &rows3(|m0: Option<OI0S>, m1: A1SS, m2: P2S| Holder6557 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6565() -> Vec<String> {
        check(
            "main3:H3S1:O0_S|E1_NN|P2_S",
            &rows3(|m0: Option<OI0S>, m1: E1NN, m2: P2S| Holder6565 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6573() -> Vec<String> {
        check(
            "main3:H3S1:O0_S|E1_SS|P2_S",
            &rows3(|m0: Option<OI0S>, m1: E1SS, m2: P2S| Holder6573 { own: 7, m0, m1, m2 }),
            &[
                ("None,Dot,-", 'R'),
                ("None,Rect,-", 'R'),
                ("None,Wrap,-", 'R'),
                ("Some,Dot,-", 'R'),
                ("Some,Rect,-", 'R'),
                ("Some,Wrap,-", 'R'),
            ],
        )
    }

    fn holder6581() -> Vec<String> {
        check(
            "main3:H3S1:O0_S|O1_N|P2_S",
            &rows3(|m0: Option<OI0S>, m1: Option<OI1N>, m2: P2S| Holder6581 { own: 7, m0, m1, m2 }),
            &[
                ("None,None,-", 'R'),
                ("None,Some,-", 'R'),
                ("Some,None,-", 'R'),
                ("Some,Some,-", 'R'),
            ],
        )
    }

    fn holder6589() -> Vec<String> {
        check(
            "main3:H3S1:O0_S|O1_S|P2_S",
            &rows3(|m0: Option<OI0S>, m1: Option<OI1S>, m2: P2S| Holder6589 { own: 7, m0, m1, m2 }),
            &[
                ("None,None,-", 'R'),
                ("None,Some,-", 'R'),
                ("Some,None,-", 'R'),
                ("Some,Some,-", 'R'),
            ],
        )
    }

    fn holder6597() -> Vec<String> {
        check(
            "main3:H3S1:O0_S|P1_N|P2_S",
            &rows3(|m0: Option<OI0S>, m1: P1N, m2: P2S| Holder6597 { own: 7, m0, m1, m2 }),
            &[("None,-,-", 'R'), ("Some,-,-", 'R')],
        )
    }

    fn holder6605() -> Vec<String> {
        check(
            "main3:H3S1:O0_S|P1_S|P2_S",
            &rows3(|m0: Option<OI0S>, m1: P1S, m2: P2S| Holder6605 { own: 7, m0, m1, m2 }),
            &[("None,-,-", 'R'), ("Some,-,-", 'R')],
        )
    }

    fn holder6613() -> Vec<String> {
        check(
            "main3:H3S1:P0_N|A1_NN|P2_S",
            &rows3(|m0: P0N, m1: A1NN, m2: P2S| Holder6613 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6621() -> Vec<String> {
        check(
            "main3:H3S1:P0_N|A1_SS|P2_S",
            &rows3(|m0: P0N, m1: A1SS, m2: P2S| Holder6621 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6629() -> Vec<String> {
        check(
            "main3:H3S1:P0_N|E1_NN|P2_S",
            &rows3(|m0: P0N, m1: E1NN, m2: P2S| Holder6629 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6637() -> Vec<String> {
        check(
            "main3:H3S1:P0_N|E1_SS|P2_S",
            &rows3(|m0: P0N, m1: E1SS, m2: P2S| Holder6637 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6645() -> Vec<String> {
        check(
            "main3:H3S1:P0_N|O1_N|P2_S",
            &rows3(|m0: P0N, m1: Option<OI1N>, m2: P2S| Holder6645 { own: 7, m0, m1, m2 }),
            &[("-,None,-", 'R'), ("-,Some,-", 'R')],
        )
    }

    fn holder6653() -> Vec<String> {
        check(
            "main3:H3S1:P0_N|O1_S|P2_S",
            &rows3(|m0: P0N, m1: Option<OI1S>, m2: P2S| Holder6653 { own: 7, m0, m1, m2 }),
            &[("-,None,-", 'R'), ("-,Some,-", 'R')],
        )
    }

    fn holder6661() -> Vec<String> {
        check(
            "main3:H3S1:P0_N|P1_N|P2_S",
            &rows3(|m0: P0N, m1: P1N, m2: P2S| Holder6661 { own: 7, m0, m1, m2 }),
            &[("-,-,-", 'R')],
        )
    }

    fn holder6669() -> Vec<String> {
        check(
            "main3:H3S1:P0_N|P1_S|P2_S",
            &rows3(|m0: P0N, m1: P1S, m2: P2S| Holder6669 { own: 7, m0, m1, m2 }),
            &[("-,-,-", 'R')],
        )
    }

    fn holder6677() -> Vec<String> {
        check(
            "main3:H3S1:P0_S|A1_NN|P2_S",
            &rows3(|m0: P0S, m1: A1NN, m2: P2S| Holder6677 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6685() -> Vec<String> {
        check(
            "main3:H3S1:P0_S|A1_SS|P2_S",
            &rows3(|m0: P0S, m1: A1SS, m2: P2S| Holder6685 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6693() -> Vec<String> {
        check(
            "main3:H3S1:P0_S|E1_NN|P2_S",
            &rows3(|m0: P0S, m1: E1NN, m2: P2S| Holder6693 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6701() -> Vec<String> {
        check(
            "main3:H3S1:P0_S|E1_SS|P2_S",
            &rows3(|m0: P0S, m1: E1SS, m2: P2S| Holder6701 { own: 7, m0, m1, m2 }),
            &[("-,Dot,-", 'R'), ("-,Rect,-", 'R'), ("-,Wrap,-", 'R')],
        )
    }

    fn holder6709() -> Vec<String> {
        check(
            "main3:H3S1:P0_S|O1_N|P2_S",
            &rows3(|m0: P0S, m1: Option<OI1N>, m2: P2S| Holder6709 { own: 7, m0, m1, m2 }),
            &[("-,None,-", 'R'), ("-,Some,-", 'R')],
        )
    }

    fn holder6717() -> Vec<String> {
        check(
            "main3:H3S1:P0_S|O1_S|P2_S",
            &rows3(|m0: P0S, m1: Option<OI1S>, m2: P2S| Holder6717 { own: 7, m0, m1, m2 }),
            &[("-,None,-", 'R'), ("-,Some,-", 'R')],
        )
    }

    fn holder6725() -> Vec<String> {
        check(
            "main3:H3S1:P0_S|P1_N|P2_S",
            &rows3(|m0: P0S, m1: P1N, m2: P2S| Holder6725 { own: 7, m0, m1, m2 }),
            &[("-,-,-", 'R')],
        )
    }

    fn holder6733() -> Vec<String> {
        check(
            "main3:H3S1:P0_S|P1_S|P2_S",
            &rows3(|m0: P0S, m1: P1S, m2: P2S| Holder6733 { own: 7, m0, m1, m2 }),
            &[("-,-,-", 'R')],
        )
    }

    fn holder6741() -> Vec<String> {
        check(
            "mixed1:H1N0:T0_NS",
            &rows1(|m0: T0NS| Holder6741 { m0 }),
            &[("Dot", 'E'), ("Rect", 'E'), ("Wrap", 'R')],
        )
    }

    fn holder6749() -> Vec<String> {
        check(
            "mixed1:H1N1:FT0_NNS",
            &rows1(|m0: FT0NNS| Holder6749 { own: 7, m0 }),
            &[("Dot", 'E'), ("Rect", 'E'), ("Wrap", 'R')],
        )
    }

    fn holder6757() -> Vec<String> {
        check(
            "mixed2:H2N0:A0_NN|FO1_SN",
            &rows2(|m0: A0NN, m1: FO1SN| Holder6757 { m0, m1 }),
            &[("Dot,-", 'R'), ("Rect,-", 'R'), ("Wrap,-", 'R')],
        )
    }

    fn holder6765() -> Vec<String> {
        check(
            "mixed2:H2N0:A0_NN|V1_NS",
            &rows2(|m0: A0NN, m1: V1NS| Holder6765 { m0, m1 }),
            &[
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'R'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder6773() -> Vec<String> {
        check(
            "mixed2:H2N0:A0_SS|FT1_NSS",
            &rows2(|m0: A0SS, m1: FT1NSS| Holder6773 { m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder6781() -> Vec<String> {
        check(
            "mixed2:H2N0:E0_NN|FP1_SN",
            &rows2(|m0: E0NN, m1: FP1SN| Holder6781 { m0, m1 }),
            &[("Dot,-", 'R'), ("Rect,-", 'R'), ("Wrap,-", 'R')],
        )
    }

    fn holder6789() -> Vec<String> {
        check(
            "mixed2:H2N0:E0_SS|FO1_NS",
            &rows2(|m0: E0SS, m1: FO1NS| Holder6789 { m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder6797() -> Vec<String> {
        check(
            "mixed2:H2N0:E0_SS|T1_SN",
            &rows2(|m0: E0SS, m1: T1SN| Holder6797 { m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder6805() -> Vec<String> {
        check(
            "mixed2:H2N0:FO0_NS|E1_SS",
            &rows2(|m0: FO0NS, m1: E1SS| Holder6805 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder6813() -> Vec<String> {
        check(
            "mixed2:H2N0:FO0_NS|P1_N",
            &rows2(|m0: FO0NS, m1: P1N| Holder6813 { m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder6821() -> Vec<String> {
        check(
            "mixed2:H2N0:FP0_NS|E1_NN",
            &rows2(|m0: FP0NS, m1: E1NN| Holder6821 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder6829() -> Vec<String> {
        check(
            "mixed2:H2N0:FP0_NS|O1_S",
            &rows2(|m0: FP0NS, m1: Option<OI1S>| Holder6829 { m0, m1 }),
            &[("-,None", 'E'), ("-,Some", 'E')],
        )
    }

    fn holder6837() -> Vec<String> {
        check(
            "mixed2:H2N0:O0_N|FO1_SN",
            &rows2(|m0: Option<OI0N>, m1: FO1SN| Holder6837 { m0, m1 }),
            &[("None,-", 'R'), ("Some,-", 'R')],
        )
    }

    fn holder6845() -> Vec<String> {
        check(
            "mixed2:H2N0:O0_N|V1_NS",
            &rows2(|m0: Option<OI0N>, m1: V1NS| Holder6845 { m0, m1 }),
            &[
                ("None,Rect", 'E'),
                ("None,Wrap", 'R'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder6853() -> Vec<String> {
        check(
            "mixed2:H2N0:O0_S|FT1_NSS",
            &rows2(|m0: Option<OI0S>, m1: FT1NSS| Holder6853 { m0, m1 }),
            &[
                ("None,Dot", 'R'),
                ("None,Rect", 'R'),
                ("None,Wrap", 'R'),
                ("Some,Dot", 'R'),
                ("Some,Rect", 'R'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder6861() -> Vec<String> {
        check(
            "mixed2:H2N0:P0_N|FP1_SN",
            &rows2(|m0: P0N, m1: FP1SN| Holder6861 { m0, m1 }),
            &[("-,-", 'R')],
        )
    }

    fn holder6869() -> Vec<String> {
        check(
            "mixed2:H2N0:P0_S|FO1_NS",
            &rows2(|m0: P0S, m1: FO1NS| Holder6869 { m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder6877() -> Vec<String> {
        check(
            "mixed2:H2N0:P0_S|T1_SN",
            &rows2(|m0: P0S, m1: T1SN| Holder6877 { m0, m1 }),
            &[("-,Dot", 'R'), ("-,Rect", 'R'), ("-,Wrap", 'E')],
        )
    }

    fn holder6885() -> Vec<String> {
        check(
            "mixed2:H2N1:A0_NN|FO1_SN",
            &rows2(|m0: A0NN, m1: FO1SN| Holder6885 { own: 7, m0, m1 }),
            &[("Dot,-", 'R'), ("Rect,-", 'R'), ("Wrap,-", 'R')],
        )
    }

    fn holder6893() -> Vec<String> {
        check(
            "mixed2:H2N1:A0_NN|V1_NS",
            &rows2(|m0: A0NN, m1: V1NS| Holder6893 { own: 7, m0, m1 }),
            &[
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'R'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder6901() -> Vec<String> {
        check(
            "mixed2:H2N1:A0_SS|FT1_NSS",
            &rows2(|m0: A0SS, m1: FT1NSS| Holder6901 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder6909() -> Vec<String> {
        check(
            "mixed2:H2N1:E0_NN|FP1_SN",
            &rows2(|m0: E0NN, m1: FP1SN| Holder6909 { own: 7, m0, m1 }),
            &[("Dot,-", 'R'), ("Rect,-", 'R'), ("Wrap,-", 'R')],
        )
    }

    fn holder6917() -> Vec<String> {
        check(
            "mixed2:H2N1:E0_SS|FO1_NS",
            &rows2(|m0: E0SS, m1: FO1NS| Holder6917 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder6925() -> Vec<String> {
        check(
            "mixed2:H2N1:E0_SS|T1_SN",
            &rows2(|m0: E0SS, m1: T1SN| Holder6925 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder6933() -> Vec<String> {
        check(
            "mixed2:H2N1:FO0_NS|E1_SS",
            &rows2(|m0: FO0NS, m1: E1SS| Holder6933 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder6941() -> Vec<String> {
        check(
            "mixed2:H2N1:FO0_NS|P1_N",
            &rows2(|m0: FO0NS, m1: P1N| Holder6941 { own: 7, m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder6949() -> Vec<String> {
        check(
            "mixed2:H2N1:FP0_NS|E1_NN",
            &rows2(|m0: FP0NS, m1: E1NN| Holder6949 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder6957() -> Vec<String> {
        check(
            "mixed2:H2N1:FP0_NS|O1_S",
            &rows2(|m0: FP0NS, m1: Option<OI1S>| Holder6957 { own: 7, m0, m1 }),
            &[("-,None", 'E'), ("-,Some", 'E')],
        )
    }

    fn holder6965() -> Vec<String> {
        check(
            "mixed2:H2N1:O0_N|FO1_SN",
            &rows2(|m0: Option<OI0N>, m1: FO1SN| Holder6965 { own: 7, m0, m1 }),
            &[("None,-", 'R'), ("Some,-", 'R')],
        )
    }

    fn holder6973() -> Vec<String> {
        check(
            "mixed2:H2N1:O0_N|V1_NS",
            &rows2(|m0: Option<OI0N>, m1: V1NS| Holder6973 { own: 7, m0, m1 }),
            &[
                ("None,Rect", 'E'),
                ("None,Wrap", 'R'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder6981() -> Vec<String> {
        check(
            "mixed2:H2N1:O0_S|FT1_NSS",
            &rows2(|m0: Option<OI0S>, m1: FT1NSS| Holder6981 { own: 7, m0, m1 }),
            &[
                ("None,Dot", 'R'),
                ("None,Rect", 'R'),
                ("None,Wrap", 'R'),
                ("Some,Dot", 'R'),
                ("Some,Rect", 'R'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder6989() -> Vec<String> {
        check(
            "mixed2:H2N1:P0_N|FP1_SN",
            &rows2(|m0: P0N, m1: FP1SN| Holder6989 { own: 7, m0, m1 }),
            &[("-,-", 'R')],
        )
    }

    fn holder6997() -> Vec<String> {
        check(
            "mixed2:H2N1:P0_S|FO1_NS",
            &rows2(|m0: P0S, m1: FO1NS| Holder6997 { own: 7, m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder7005() -> Vec<String> {
        check(
            "mixed2:H2N1:P0_S|T1_SN",
            &rows2(|m0: P0S, m1: T1SN| Holder7005 { own: 7, m0, m1 }),
            &[("-,Dot", 'R'), ("-,Rect", 'R'), ("-,Wrap", 'E')],
        )
    }

    fn holder7013() -> Vec<String> {
        check(
            "nest1:H1N0:IN0(A0_SS)",
            &rows1(|m0: IN0A0SS| Holder7013 { m0 }),
            &[("Dot", 'E'), ("Rect", 'E'), ("Wrap", 'E')],
        )
    }

    fn holder7021() -> Vec<String> {
        check(
            "nest1:H1N0:IN0(M0_-)",
            &rows1(|m0: IN0Map| Holder7021 { m0 }),
            &[("-", 'U')],
        )
    }

    fn holder7029() -> Vec<String> {
        check(
            "nest1:H1N0:IN0(V0_SS)",
            &rows1(|m0: IN0V0SS| Holder7029 { m0 }),
            &[("Rect", 'R'), ("Wrap", 'R')],
        )
    }

    fn holder7037() -> Vec<String> {
        check(
            "nest1:H1N0:IN1(FP0_SS)",
            &rows1(|m0: IN1FP0SS| Holder7037 { m0 }),
            &[("-", 'R')],
        )
    }

    fn holder7045() -> Vec<String> {
        check(
            "nest1:H1N0:IN1(V0_NN)",
            &rows1(|m0: IN1V0NN| Holder7045 { m0 }),
            &[("Rect", 'E'), ("Wrap", 'E')],
        )
    }

    fn holder7053() -> Vec<String> {
        check(
            "nest1:H1N0:IS0(P0_N)",
            &rows1(|m0: IS0P0N| Holder7053 { m0 }),
            &[("-", 'R')],
        )
    }

    fn holder7061() -> Vec<String> {
        check(
            "nest1:H1N0:IS1(P0_N)",
            &rows1(|m0: IS1P0N| Holder7061 { m0 }),
            &[("-", 'R')],
        )
    }

    fn holder7069() -> Vec<String> {
        check(
            "nest1:H1N1:IN0(FP0_NN)",
            &rows1(|m0: IN0FP0NN| Holder7069 { own: 7, m0 }),
            &[("-", 'E')],
        )
    }

    fn holder7077() -> Vec<String> {
        check(
            "nest1:H1N1:IN0(T0_SS)",
            &rows1(|m0: IN0T0SS| Holder7077 { own: 7, m0 }),
            &[("Dot", 'R'), ("Rect", 'R'), ("Wrap", 'R')],
        )
    }

    fn holder7085() -> Vec<String> {
        check(
            "nest1:H1N1:IN1(FO0_SS)",
            &rows1(|m0: IN1FO0SS| Holder7085 { own: 7, m0 }),
            &[("-", 'R')],
        )
    }

    fn holder7093() -> Vec<String> {
        check(
            "nest1:H1N1:IN1(T0_NN)",
            &rows1(|m0: IN1T0NN| Holder7093 { own: 7, m0 }),
            &[("Dot", 'E'), ("Rect", 'E'), ("Wrap", 'E')],
        )
    }

    fn holder7101() -> Vec<String> {
        check(
            "nest1:H1N1:IS0(O0_N)",
            &rows1(|m0: IS0OptOI0N| Holder7101 { own: 7, m0 }),
            &[("None", 'R'), ("Some", 'R')],
        )
    }

    fn holder7109() -> Vec<String> {
        check(
            "nest1:H1N1:IS1(O0_N)",
            &rows1(|m0: IS1OptOI0N| Holder7109 { own: 7, m0 }),
            &[("None", 'R'), ("Some", 'R')],
        )
    }

    fn holder7117() -> Vec<String> {
        check(
            "nest2:H2N0:A0_NN|IN0(FO1_NN)",
            &rows2(|m0: A0NN, m1: IN0FO1NN| Holder7117 { m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder7125() -> Vec<String> {
        check(
            "nest2:H2N0:A0_NN|IN0(P1_S)",
            &rows2(|m0: A0NN, m1: IN0P1S| Holder7125 { m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder7133() -> Vec<String> {
        check(
            "nest2:H2N0:A0_NN|IN1(E1_SS)",
            &rows2(|m0: A0NN, m1: IN1E1SS| Holder7133 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7141() -> Vec<String> {
        check(
            "nest2:H2N0:A0_NN|IN1(P1_N)",
            &rows2(|m0: A0NN, m1: IN1P1N| Holder7141 { m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder7149() -> Vec<String> {
        check(
            "nest2:H2N0:A0_NN|IS0(E1_NN)",
            &rows2(|m0: A0NN, m1: IS0E1NN| Holder7149 { m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder7157() -> Vec<String> {
        check(
            "nest2:H2N0:A0_NN|IS1(E1_NN)",
            &rows2(|m0: A0NN, m1: IS1E1NN| Holder7157 { m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder7165() -> Vec<String> {
        check(
            "nest2:H2N0:A0_SS|IN0(E1_NN)",
            &rows2(|m0: A0SS, m1: IN0E1NN| Holder7165 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7173() -> Vec<String> {
        check(
            "nest2:H2N0:A0_SS|IN0(O1_S)",
            &rows2(|m0: A0SS, m1: IN0OptOI1S| Holder7173 { m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder7181() -> Vec<String> {
        check(
            "nest2:H2N0:A0_SS|IN1(A1_SS)",
            &rows2(|m0: A0SS, m1: IN1A1SS| Holder7181 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7189() -> Vec<String> {
        check(
            "nest2:H2N0:A0_SS|IN1(O1_N)",
            &rows2(|m0: A0SS, m1: IN1OptOI1N| Holder7189 { m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder7197() -> Vec<String> {
        check(
            "nest2:H2N0:A0_SS|IS0(A1_NN)",
            &rows2(|m0: A0SS, m1: IS0A1NN| Holder7197 { m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder7205() -> Vec<String> {
        check(
            "nest2:H2N0:A0_SS|IS1(A1_NN)",
            &rows2(|m0: A0SS, m1: IS1A1NN| Holder7205 { m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder7213() -> Vec<String> {
        check(
            "nest2:H2N0:E0_NN|IN0(A1_NN)",
            &rows2(|m0: E0NN, m1: IN0A1NN| Holder7213 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7221() -> Vec<String> {
        check(
            "nest2:H2N0:E0_NN|IN0(FT1_NNN)",
            &rows2(|m0: E0NN, m1: IN0FT1NNN| Holder7221 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7229() -> Vec<String> {
        check(
            "nest2:H2N0:E0_NN|IN0(V1_SS)",
            &rows2(|m0: E0NN, m1: IN0V1SS| Holder7229 { m0, m1 }),
            &[
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder7237() -> Vec<String> {
        check(
            "nest2:H2N0:E0_NN|IN1(FP1_SS)",
            &rows2(|m0: E0NN, m1: IN1FP1SS| Holder7237 { m0, m1 }),
            &[("Dot,-", 'R'), ("Rect,-", 'R'), ("Wrap,-", 'R')],
        )
    }

    fn holder7245() -> Vec<String> {
        check(
            "nest2:H2N0:E0_NN|IN1(V1_NN)",
            &rows2(|m0: E0NN, m1: IN1V1NN| Holder7245 { m0, m1 }),
            &[
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7253() -> Vec<String> {
        check(
            "nest2:H2N0:E0_NN|IS0(P1_N)",
            &rows2(|m0: E0NN, m1: IS0P1N| Holder7253 { m0, m1 }),
            &[("Dot,-", 'R'), ("Rect,-", 'R'), ("Wrap,-", 'R')],
        )
    }

    fn holder7261() -> Vec<String> {
        check(
            "nest2:H2N0:E0_NN|IS1(P1_N)",
            &rows2(|m0: E0NN, m1: IS1P1N| Holder7261 { m0, m1 }),
            &[("Dot,-", 'R'), ("Rect,-", 'R'), ("Wrap,-", 'R')],
        )
    }

    fn holder7269() -> Vec<String> {
        check(
            "nest2:H2N0:E0_SS|IN0(FP1_NN)",
            &rows2(|m0: E0SS, m1: IN0FP1NN| Holder7269 { m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder7277() -> Vec<String> {
        check(
            "nest2:H2N0:E0_SS|IN0(T1_SS)",
            &rows2(|m0: E0SS, m1: IN0T1SS| Holder7277 { m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder7285() -> Vec<String> {
        check(
            "nest2:H2N0:E0_SS|IN1(FO1_SS)",
            &rows2(|m0: E0SS, m1: IN1FO1SS| Holder7285 { m0, m1 }),
            &[("Dot,-", 'R'), ("Rect,-", 'R'), ("Wrap,-", 'R')],
        )
    }

    fn holder7293() -> Vec<String> {
        check(
            "nest2:H2N0:E0_SS|IN1(T1_NN)",
            &rows2(|m0: E0SS, m1: IN1T1NN| Holder7293 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7301() -> Vec<String> {
        check(
            "nest2:H2N0:E0_SS|IS0(O1_N)",
            &rows2(|m0: E0SS, m1: IS0OptOI1N| Holder7301 { m0, m1 }),
            &[
                ("Dot,None", 'R'),
                ("Dot,Some", 'R'),
                ("Rect,None", 'R'),
                ("Rect,Some", 'R'),
                ("Wrap,None", 'R'),
                ("Wrap,Some", 'R'),
            ],
        )
    }

    fn holder7309() -> Vec<String> {
        check(
            "nest2:H2N0:E0_SS|IS1(O1_N)",
            &rows2(|m0: E0SS, m1: IS1OptOI1N| Holder7309 { m0, m1 }),
            &[
                ("Dot,None", 'R'),
                ("Dot,Some", 'R'),
                ("Rect,None", 'R'),
                ("Rect,Some", 'R'),
                ("Wrap,None", 'R'),
                ("Wrap,Some", 'R'),
            ],
        )
    }

    fn holder7317() -> Vec<String> {
        check(
            "nest2:H2N0:FO0_NN|IN0(FO1_NN)",
            &rows2(|m0: FO0NN, m1: IN0FO1NN| Holder7317 { m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder7325() -> Vec<String> {
        check(
            "nest2:H2N0:FO0_NN|IN0(V1_NN)",
            &rows2(|m0: FO0NN, m1: IN0V1NN| Holder7325 { m0, m1 }),
            &[("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder7333() -> Vec<String> {
        check(
            "nest2:H2N0:FO0_NN|IN1(O1_N)",
            &rows2(|m0: FO0NN, m1: IN1OptOI1N| Holder7333 { m0, m1 }),
            &[("-,None", 'E'), ("-,Some", 'E')],
        )
    }

    fn holder7341() -> Vec<String> {
        check(
            "nest2:H2N0:FP0_NN|IN0(E1_NN)",
            &rows2(|m0: FP0NN, m1: IN0E1NN| Holder7341 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder7349() -> Vec<String> {
        check(
            "nest2:H2N0:FP0_NN|IN0(P1_S)",
            &rows2(|m0: FP0NN, m1: IN0P1S| Holder7349 { m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder7357() -> Vec<String> {
        check(
            "nest2:H2N0:FP0_NN|IN1(FP1_NN)",
            &rows2(|m0: FP0NN, m1: IN1FP1NN| Holder7357 { m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder7365() -> Vec<String> {
        check(
            "nest2:H2N0:FT0_NNN|IN0(A1_NN)",
            &rows2(|m0: FT0NNN, m1: IN0A1NN| Holder7365 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7373() -> Vec<String> {
        check(
            "nest2:H2N0:FT0_NNN|IN0(O1_S)",
            &rows2(|m0: FT0NNN, m1: IN0OptOI1S| Holder7373 { m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder7381() -> Vec<String> {
        check(
            "nest2:H2N0:FT0_NNN|IN1(E1_SS)",
            &rows2(|m0: FT0NNN, m1: IN1E1SS| Holder7381 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7389() -> Vec<String> {
        check(
            "nest2:H2N0:FT0_NNN|IN1(T1_NN)",
            &rows2(|m0: FT0NNN, m1: IN1T1NN| Holder7389 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7397() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(A0_NN)|FT1_NNN",
            &rows2(|m0: IN0A0NN, m1: FT1NNN| Holder7397 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7405() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(A0_SS)|A1_SS",
            &rows2(|m0: IN0A0SS, m1: A1SS| Holder7405 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7413() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(A0_SS)|P1_N",
            &rows2(|m0: IN0A0SS, m1: P1N| Holder7413 { m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder7421() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(E0_NN)|FO1_NN",
            &rows2(|m0: IN0E0NN, m1: FO1NN| Holder7421 { m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder7429() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(E0_NN)|V1_NN",
            &rows2(|m0: IN0E0NN, m1: V1NN| Holder7429 { m0, m1 }),
            &[
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7437() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(E0_SS)|O1_N",
            &rows2(|m0: IN0E0SS, m1: Option<OI1N>| Holder7437 { m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder7445() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(FO0_NN)|E1_NN",
            &rows2(|m0: IN0FO0NN, m1: E1NN| Holder7445 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder7453() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(FO0_NN)|P1_S",
            &rows2(|m0: IN0FO0NN, m1: P1S| Holder7453 { m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder7461() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(FP0_NN)|FP1_NN",
            &rows2(|m0: IN0FP0NN, m1: FP1NN| Holder7461 { m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder7469() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(FT0_NNN)|A1_NN",
            &rows2(|m0: IN0FT0NNN, m1: A1NN| Holder7469 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7477() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(FT0_NNN)|O1_S",
            &rows2(|m0: IN0FT0NNN, m1: Option<OI1S>| Holder7477 { m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder7485() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(O0_N)|E1_SS",
            &rows2(|m0: IN0OptOI0N, m1: E1SS| Holder7485 { m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder7493() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(O0_N)|T1_NN",
            &rows2(|m0: IN0OptOI0N, m1: T1NN| Holder7493 { m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder7501() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(O0_S)|FT1_NNN",
            &rows2(|m0: IN0OptOI0S, m1: FT1NNN| Holder7501 { m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder7509() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(P0_N)|A1_SS",
            &rows2(|m0: IN0P0N, m1: A1SS| Holder7509 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder7517() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(P0_N)|P1_N",
            &rows2(|m0: IN0P0N, m1: P1N| Holder7517 { m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder7525() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(P0_S)|FO1_NN",
            &rows2(|m0: IN0P0S, m1: FO1NN| Holder7525 { m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder7533() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(P0_S)|V1_NN",
            &rows2(|m0: IN0P0S, m1: V1NN| Holder7533 { m0, m1 }),
            &[("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder7541() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(T0_NN)|O1_N",
            &rows2(|m0: IN0T0NN, m1: Option<OI1N>| Holder7541 { m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder7549() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(V0_NN)|E1_NN",
            &rows2(|m0: IN0V0NN, m1: E1NN| Holder7549 { m0, m1 }),
            &[
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7557() -> Vec<String> {
        check(
            "nest2:H2N0:IN0(V0_NN)|P1_S",
            &rows2(|m0: IN0V0NN, m1: P1S| Holder7557 { m0, m1 }),
            &[("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder7565() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(A0_NN)|FP1_NN",
            &rows2(|m0: IN1A0NN, m1: FP1NN| Holder7565 { m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder7573() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(A0_SS)|A1_NN",
            &rows2(|m0: IN1A0SS, m1: A1NN| Holder7573 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7581() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(A0_SS)|O1_S",
            &rows2(|m0: IN1A0SS, m1: Option<OI1S>| Holder7581 { m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder7589() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(E0_NN)|E1_SS",
            &rows2(|m0: IN1E0NN, m1: E1SS| Holder7589 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7597() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(E0_NN)|T1_NN",
            &rows2(|m0: IN1E0NN, m1: T1NN| Holder7597 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7605() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(E0_SS)|FT1_NNN",
            &rows2(|m0: IN1E0SS, m1: FT1NNN| Holder7605 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7613() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(FO0_NN)|A1_SS",
            &rows2(|m0: IN1FO0NN, m1: A1SS| Holder7613 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder7621() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(FO0_NN)|P1_N",
            &rows2(|m0: IN1FO0NN, m1: P1N| Holder7621 { m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder7629() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(FP0_NN)|FO1_NN",
            &rows2(|m0: IN1FP0NN, m1: FO1NN| Holder7629 { m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder7637() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(FP0_NN)|V1_NN",
            &rows2(|m0: IN1FP0NN, m1: V1NN| Holder7637 { m0, m1 }),
            &[("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder7645() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(FT0_NNN)|O1_N",
            &rows2(|m0: IN1FT0NNN, m1: Option<OI1N>| Holder7645 { m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder7653() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(O0_N)|E1_NN",
            &rows2(|m0: IN1OptOI0N, m1: E1NN| Holder7653 { m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder7661() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(O0_N)|P1_S",
            &rows2(|m0: IN1OptOI0N, m1: P1S| Holder7661 { m0, m1 }),
            &[("None,-", 'E'), ("Some,-", 'E')],
        )
    }

    fn holder7669() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(O0_S)|FP1_NN",
            &rows2(|m0: IN1OptOI0S, m1: FP1NN| Holder7669 { m0, m1 }),
            &[("None,-", 'E'), ("Some,-", 'E')],
        )
    }

    fn holder7677() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(P0_N)|A1_NN",
            &rows2(|m0: IN1P0N, m1: A1NN| Holder7677 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder7685() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(P0_N)|O1_S",
            &rows2(|m0: IN1P0N, m1: Option<OI1S>| Holder7685 { m0, m1 }),
            &[("-,None", 'E'), ("-,Some", 'E')],
        )
    }

    fn holder7693() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(P0_S)|E1_SS",
            &rows2(|m0: IN1P0S, m1: E1SS| Holder7693 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder7701() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(P0_S)|T1_NN",
            &rows2(|m0: IN1P0S, m1: T1NN| Holder7701 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder7709() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(T0_NN)|FT1_NNN",
            &rows2(|m0: IN1T0NN, m1: FT1NNN| Holder7709 { m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7717() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(V0_NN)|A1_SS",
            &rows2(|m0: IN1V0NN, m1: A1SS| Holder7717 { m0, m1 }),
            &[
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7725() -> Vec<String> {
        check(
            "nest2:H2N0:IN1(V0_NN)|P1_N",
            &rows2(|m0: IN1V0NN, m1: P1N| Holder7725 { m0, m1 }),
            &[("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder7733() -> Vec<String> {
        check(
            "nest2:H2N0:O0_N|IN0(FO1_NN)",
            &rows2(|m0: Option<OI0N>, m1: IN0FO1NN| Holder7733 { m0, m1 }),
            &[("None,-", 'E'), ("Some,-", 'E')],
        )
    }

    fn holder7741() -> Vec<String> {
        check(
            "nest2:H2N0:O0_N|IN0(P1_S)",
            &rows2(|m0: Option<OI0N>, m1: IN0P1S| Holder7741 { m0, m1 }),
            &[("None,-", 'E'), ("Some,-", 'E')],
        )
    }

    fn holder7749() -> Vec<String> {
        check(
            "nest2:H2N0:O0_N|IN1(E1_SS)",
            &rows2(|m0: Option<OI0N>, m1: IN1E1SS| Holder7749 { m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder7757() -> Vec<String> {
        check(
            "nest2:H2N0:O0_N|IN1(P1_N)",
            &rows2(|m0: Option<OI0N>, m1: IN1P1N| Holder7757 { m0, m1 }),
            &[("None,-", 'E'), ("Some,-", 'E')],
        )
    }

    fn holder7765() -> Vec<String> {
        check(
            "nest2:H2N0:O0_N|IS0(E1_NN)",
            &rows2(|m0: Option<OI0N>, m1: IS0E1NN| Holder7765 { m0, m1 }),
            &[
                ("None,Dot", 'R'),
                ("None,Rect", 'R'),
                ("None,Wrap", 'R'),
                ("Some,Dot", 'R'),
                ("Some,Rect", 'R'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder7773() -> Vec<String> {
        check(
            "nest2:H2N0:O0_N|IS1(E1_NN)",
            &rows2(|m0: Option<OI0N>, m1: IS1E1NN| Holder7773 { m0, m1 }),
            &[
                ("None,Dot", 'R'),
                ("None,Rect", 'R'),
                ("None,Wrap", 'R'),
                ("Some,Dot", 'R'),
                ("Some,Rect", 'R'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder7781() -> Vec<String> {
        check(
            "nest2:H2N0:O0_S|IN0(E1_NN)",
            &rows2(|m0: Option<OI0S>, m1: IN0E1NN| Holder7781 { m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder7789() -> Vec<String> {
        check(
            "nest2:H2N0:O0_S|IN0(O1_S)",
            &rows2(|m0: Option<OI0S>, m1: IN0OptOI1S| Holder7789 { m0, m1 }),
            &[
                ("None,None", 'E'),
                ("None,Some", 'E'),
                ("Some,None", 'E'),
                ("Some,Some", 'E'),
            ],
        )
    }

    fn holder7797() -> Vec<String> {
        check(
            "nest2:H2N0:O0_S|IN1(A1_SS)",
            &rows2(|m0: Option<OI0S>, m1: IN1A1SS| Holder7797 { m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder7805() -> Vec<String> {
        check(
            "nest2:H2N0:O0_S|IN1(O1_N)",
            &rows2(|m0: Option<OI0S>, m1: IN1OptOI1N| Holder7805 { m0, m1 }),
            &[
                ("None,None", 'E'),
                ("None,Some", 'E'),
                ("Some,None", 'E'),
                ("Some,Some", 'E'),
            ],
        )
    }

    fn holder7813() -> Vec<String> {
        check(
            "nest2:H2N0:O0_S|IS0(A1_NN)",
            &rows2(|m0: Option<OI0S>, m1: IS0A1NN| Holder7813 { m0, m1 }),
            &[
                ("None,Dot", 'R'),
                ("None,Rect", 'R'),
                ("None,Wrap", 'R'),
                ("Some,Dot", 'R'),
                ("Some,Rect", 'R'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder7821() -> Vec<String> {
        check(
            "nest2:H2N0:O0_S|IS1(A1_NN)",
            &rows2(|m0: Option<OI0S>, m1: IS1A1NN| Holder7821 { m0, m1 }),
            &[
                ("None,Dot", 'R'),
                ("None,Rect", 'R'),
                ("None,Wrap", 'R'),
                ("Some,Dot", 'R'),
                ("Some,Rect", 'R'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder7829() -> Vec<String> {
        check(
            "nest2:H2N0:P0_N|IN0(A1_NN)",
            &rows2(|m0: P0N, m1: IN0A1NN| Holder7829 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder7837() -> Vec<String> {
        check(
            "nest2:H2N0:P0_N|IN0(FT1_NNN)",
            &rows2(|m0: P0N, m1: IN0FT1NNN| Holder7837 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder7845() -> Vec<String> {
        check(
            "nest2:H2N0:P0_N|IN0(V1_SS)",
            &rows2(|m0: P0N, m1: IN0V1SS| Holder7845 { m0, m1 }),
            &[("-,Rect", 'R'), ("-,Wrap", 'R')],
        )
    }

    fn holder7853() -> Vec<String> {
        check(
            "nest2:H2N0:P0_N|IN1(FP1_SS)",
            &rows2(|m0: P0N, m1: IN1FP1SS| Holder7853 { m0, m1 }),
            &[("-,-", 'R')],
        )
    }

    fn holder7861() -> Vec<String> {
        check(
            "nest2:H2N0:P0_N|IN1(V1_NN)",
            &rows2(|m0: P0N, m1: IN1V1NN| Holder7861 { m0, m1 }),
            &[("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder7869() -> Vec<String> {
        check(
            "nest2:H2N0:P0_N|IS0(P1_N)",
            &rows2(|m0: P0N, m1: IS0P1N| Holder7869 { m0, m1 }),
            &[("-,-", 'R')],
        )
    }

    fn holder7877() -> Vec<String> {
        check(
            "nest2:H2N0:P0_N|IS1(P1_N)",
            &rows2(|m0: P0N, m1: IS1P1N| Holder7877 { m0, m1 }),
            &[("-,-", 'R')],
        )
    }

    fn holder7885() -> Vec<String> {
        check(
            "nest2:H2N0:P0_S|IN0(FP1_NN)",
            &rows2(|m0: P0S, m1: IN0FP1NN| Holder7885 { m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder7893() -> Vec<String> {
        check(
            "nest2:H2N0:P0_S|IN0(T1_SS)",
            &rows2(|m0: P0S, m1: IN0T1SS| Holder7893 { m0, m1 }),
            &[("-,Dot", 'R'), ("-,Rect", 'R'), ("-,Wrap", 'R')],
        )
    }

    fn holder7901() -> Vec<String> {
        check(
            "nest2:H2N0:P0_S|IN1(FO1_SS)",
            &rows2(|m0: P0S, m1: IN1FO1SS| Holder7901 { m0, m1 }),
            &[("-,-", 'R')],
        )
    }

    fn holder7909() -> Vec<String> {
        check(
            "nest2:H2N0:P0_S|IN1(T1_NN)",
            &rows2(|m0: P0S, m1: IN1T1NN| Holder7909 { m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder7917() -> Vec<String> {
        check(
            "nest2:H2N0:P0_S|IS0(O1_N)",
            &rows2(|m0: P0S, m1: IS0OptOI1N| Holder7917 { m0, m1 }),
            &[("-,None", 'R'), ("-,Some", 'R')],
        )
    }

    fn holder7925() -> Vec<String> {
        check(
            "nest2:H2N0:P0_S|IS1(O1_N)",
            &rows2(|m0: P0S, m1: IS1OptOI1N| Holder7925 { m0, m1 }),
            &[("-,None", 'R'), ("-,Some", 'R')],
        )
    }

    fn holder7933() -> Vec<String> {
        check(
            "nest2:H2N0:T0_NN|IN0(FO1_NN)",
            &rows2(|m0: T0NN, m1: IN0FO1NN| Holder7933 { m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder7941() -> Vec<String> {
        check(
            "nest2:H2N0:T0_NN|IN0(V1_NN)",
            &rows2(|m0: T0NN, m1: IN0V1NN| Holder7941 { m0, m1 }),
            &[
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7949() -> Vec<String> {
        check(
            "nest2:H2N0:T0_NN|IN1(O1_N)",
            &rows2(|m0: T0NN, m1: IN1OptOI1N| Holder7949 { m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder7957() -> Vec<String> {
        check(
            "nest2:H2N0:V0_NN|IN0(E1_NN)",
            &rows2(|m0: V0NN, m1: IN0E1NN| Holder7957 { m0, m1 }),
            &[
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7965() -> Vec<String> {
        check(
            "nest2:H2N0:V0_NN|IN0(P1_S)",
            &rows2(|m0: V0NN, m1: IN0P1S| Holder7965 { m0, m1 }),
            &[("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder7973() -> Vec<String> {
        check(
            "nest2:H2N0:V0_NN|IN1(FP1_NN)",
            &rows2(|m0: V0NN, m1: IN1FP1NN| Holder7973 { m0, m1 }),
            &[("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder7981() -> Vec<String> {
        check(
            "nest2:H2N1:A0_NN|IN0(A1_NN)",
            &rows2(|m0: A0NN, m1: IN0A1NN| Holder7981 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7989() -> Vec<String> {
        check(
            "nest2:H2N1:A0_NN|IN0(FT1_NNN)",
            &rows2(|m0: A0NN, m1: IN0FT1NNN| Holder7989 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder7997() -> Vec<String> {
        check(
            "nest2:H2N1:A0_NN|IN0(V1_SS)",
            &rows2(|m0: A0NN, m1: IN0V1SS| Holder7997 { own: 7, m0, m1 }),
            &[
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder8005() -> Vec<String> {
        check(
            "nest2:H2N1:A0_NN|IN1(FP1_SS)",
            &rows2(|m0: A0NN, m1: IN1FP1SS| Holder8005 { own: 7, m0, m1 }),
            &[("Dot,-", 'R'), ("Rect,-", 'R'), ("Wrap,-", 'R')],
        )
    }

    fn holder8013() -> Vec<String> {
        check(
            "nest2:H2N1:A0_NN|IN1(V1_NN)",
            &rows2(|m0: A0NN, m1: IN1V1NN| Holder8013 { own: 7, m0, m1 }),
            &[
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8021() -> Vec<String> {
        check(
            "nest2:H2N1:A0_NN|IS0(P1_N)",
            &rows2(|m0: A0NN, m1: IS0P1N| Holder8021 { own: 7, m0, m1 }),
            &[("Dot,-", 'R'), ("Rect,-", 'R'), ("Wrap,-", 'R')],
        )
    }

    fn holder8029() -> Vec<String> {
        check(
            "nest2:H2N1:A0_NN|IS1(P1_N)",
            &rows2(|m0: A0NN, m1: IS1P1N| Holder8029 { own: 7, m0, m1 }),
            &[("Dot,-", 'R'), ("Rect,-", 'R'), ("Wrap,-", 'R')],
        )
    }

    fn holder8037() -> Vec<String> {
        check(
            "nest2:H2N1:A0_SS|IN0(FP1_NN)",
            &rows2(|m0: A0SS, m1: IN0FP1NN| Holder8037 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder8045() -> Vec<String> {
        check(
            "nest2:H2N1:A0_SS|IN0(T1_SS)",
            &rows2(|m0: A0SS, m1: IN0T1SS| Holder8045 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder8053() -> Vec<String> {
        check(
            "nest2:H2N1:A0_SS|IN1(FO1_SS)",
            &rows2(|m0: A0SS, m1: IN1FO1SS| Holder8053 { own: 7, m0, m1 }),
            &[("Dot,-", 'R'), ("Rect,-", 'R'), ("Wrap,-", 'R')],
        )
    }

    fn holder8061() -> Vec<String> {
        check(
            "nest2:H2N1:A0_SS|IN1(T1_NN)",
            &rows2(|m0: A0SS, m1: IN1T1NN| Holder8061 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8069() -> Vec<String> {
        check(
            "nest2:H2N1:A0_SS|IS0(O1_N)",
            &rows2(|m0: A0SS, m1: IS0OptOI1N| Holder8069 { own: 7, m0, m1 }),
            &[
                ("Dot,None", 'R'),
                ("Dot,Some", 'R'),
                ("Rect,None", 'R'),
                ("Rect,Some", 'R'),
                ("Wrap,None", 'R'),
                ("Wrap,Some", 'R'),
            ],
        )
    }

    fn holder8077() -> Vec<String> {
        check(
            "nest2:H2N1:A0_SS|IS1(O1_N)",
            &rows2(|m0: A0SS, m1: IS1OptOI1N| Holder8077 { own: 7, m0, m1 }),
            &[
                ("Dot,None", 'R'),
                ("Dot,Some", 'R'),
                ("Rect,None", 'R'),
                ("Rect,Some", 'R'),
                ("Wrap,None", 'R'),
                ("Wrap,Some", 'R'),
            ],
        )
    }

    fn holder8085() -> Vec<String> {
        check(
            "nest2:H2N1:E0_NN|IN0(FO1_NN)",
            &rows2(|m0: E0NN, m1: IN0FO1NN| Holder8085 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder8093() -> Vec<String> {
        check(
            "nest2:H2N1:E0_NN|IN0(P1_S)",
            &rows2(|m0: E0NN, m1: IN0P1S| Holder8093 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder8101() -> Vec<String> {
        check(
            "nest2:H2N1:E0_NN|IN1(E1_SS)",
            &rows2(|m0: E0NN, m1: IN1E1SS| Holder8101 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8109() -> Vec<String> {
        check(
            "nest2:H2N1:E0_NN|IN1(P1_N)",
            &rows2(|m0: E0NN, m1: IN1P1N| Holder8109 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder8117() -> Vec<String> {
        check(
            "nest2:H2N1:E0_NN|IS0(E1_NN)",
            &rows2(|m0: E0NN, m1: IS0E1NN| Holder8117 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder8125() -> Vec<String> {
        check(
            "nest2:H2N1:E0_NN|IS1(E1_NN)",
            &rows2(|m0: E0NN, m1: IS1E1NN| Holder8125 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder8133() -> Vec<String> {
        check(
            "nest2:H2N1:E0_SS|IN0(E1_NN)",
            &rows2(|m0: E0SS, m1: IN0E1NN| Holder8133 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8141() -> Vec<String> {
        check(
            "nest2:H2N1:E0_SS|IN0(O1_S)",
            &rows2(|m0: E0SS, m1: IN0OptOI1S| Holder8141 { own: 7, m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder8149() -> Vec<String> {
        check(
            "nest2:H2N1:E0_SS|IN1(A1_SS)",
            &rows2(|m0: E0SS, m1: IN1A1SS| Holder8149 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8157() -> Vec<String> {
        check(
            "nest2:H2N1:E0_SS|IN1(O1_N)",
            &rows2(|m0: E0SS, m1: IN1OptOI1N| Holder8157 { own: 7, m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder8165() -> Vec<String> {
        check(
            "nest2:H2N1:E0_SS|IS0(A1_NN)",
            &rows2(|m0: E0SS, m1: IS0A1NN| Holder8165 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder8173() -> Vec<String> {
        check(
            "nest2:H2N1:E0_SS|IS1(A1_NN)",
            &rows2(|m0: E0SS, m1: IS1A1NN| Holder8173 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'R'),
                ("Dot,Rect", 'R'),
                ("Dot,Wrap", 'R'),
                ("Rect,Dot", 'R'),
                ("Rect,Rect", 'R'),
                ("Rect,Wrap", 'R'),
                ("Wrap,Dot", 'R'),
                ("Wrap,Rect", 'R'),
                ("Wrap,Wrap", 'R'),
            ],
        )
    }

    fn holder8181() -> Vec<String> {
        check(
            "nest2:H2N1:FO0_NN|IN0(A1_NN)",
            &rows2(|m0: FO0NN, m1: IN0A1NN| Holder8181 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder8189() -> Vec<String> {
        check(
            "nest2:H2N1:FO0_NN|IN0(O1_S)",
            &rows2(|m0: FO0NN, m1: IN0OptOI1S| Holder8189 { own: 7, m0, m1 }),
            &[("-,None", 'E'), ("-,Some", 'E')],
        )
    }

    fn holder8197() -> Vec<String> {
        check(
            "nest2:H2N1:FO0_NN|IN1(E1_SS)",
            &rows2(|m0: FO0NN, m1: IN1E1SS| Holder8197 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder8205() -> Vec<String> {
        check(
            "nest2:H2N1:FO0_NN|IN1(T1_NN)",
            &rows2(|m0: FO0NN, m1: IN1T1NN| Holder8205 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder8213() -> Vec<String> {
        check(
            "nest2:H2N1:FP0_NN|IN0(FT1_NNN)",
            &rows2(|m0: FP0NN, m1: IN0FT1NNN| Holder8213 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder8221() -> Vec<String> {
        check(
            "nest2:H2N1:FP0_NN|IN1(A1_SS)",
            &rows2(|m0: FP0NN, m1: IN1A1SS| Holder8221 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder8229() -> Vec<String> {
        check(
            "nest2:H2N1:FP0_NN|IN1(P1_N)",
            &rows2(|m0: FP0NN, m1: IN1P1N| Holder8229 { own: 7, m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder8237() -> Vec<String> {
        check(
            "nest2:H2N1:FT0_NNN|IN0(FO1_NN)",
            &rows2(|m0: FT0NNN, m1: IN0FO1NN| Holder8237 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder8245() -> Vec<String> {
        check(
            "nest2:H2N1:FT0_NNN|IN0(V1_NN)",
            &rows2(|m0: FT0NNN, m1: IN0V1NN| Holder8245 { own: 7, m0, m1 }),
            &[
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8253() -> Vec<String> {
        check(
            "nest2:H2N1:FT0_NNN|IN1(O1_N)",
            &rows2(|m0: FT0NNN, m1: IN1OptOI1N| Holder8253 { own: 7, m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder8261() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(A0_NN)|E1_NN",
            &rows2(|m0: IN0A0NN, m1: E1NN| Holder8261 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8269() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(A0_NN)|P1_S",
            &rows2(|m0: IN0A0NN, m1: P1S| Holder8269 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder8277() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(A0_SS)|FP1_NN",
            &rows2(|m0: IN0A0SS, m1: FP1NN| Holder8277 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder8285() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(E0_NN)|A1_NN",
            &rows2(|m0: IN0E0NN, m1: A1NN| Holder8285 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8293() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(E0_NN)|O1_S",
            &rows2(|m0: IN0E0NN, m1: Option<OI1S>| Holder8293 { own: 7, m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder8301() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(E0_SS)|E1_SS",
            &rows2(|m0: IN0E0SS, m1: E1SS| Holder8301 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8309() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(E0_SS)|T1_NN",
            &rows2(|m0: IN0E0SS, m1: T1NN| Holder8309 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8317() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(FO0_NN)|FT1_NNN",
            &rows2(|m0: IN0FO0NN, m1: FT1NNN| Holder8317 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder8325() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(FP0_NN)|A1_SS",
            &rows2(|m0: IN0FP0NN, m1: A1SS| Holder8325 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder8333() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(FP0_NN)|P1_N",
            &rows2(|m0: IN0FP0NN, m1: P1N| Holder8333 { own: 7, m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder8341() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(FT0_NNN)|FO1_NN",
            &rows2(|m0: IN0FT0NNN, m1: FO1NN| Holder8341 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder8349() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(FT0_NNN)|V1_NN",
            &rows2(|m0: IN0FT0NNN, m1: V1NN| Holder8349 { own: 7, m0, m1 }),
            &[
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8357() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(O0_N)|O1_N",
            &rows2(|m0: IN0OptOI0N, m1: Option<OI1N>| Holder8357 { own: 7, m0, m1 }),
            &[
                ("None,None", 'E'),
                ("None,Some", 'E'),
                ("Some,None", 'E'),
                ("Some,Some", 'E'),
            ],
        )
    }

    fn holder8365() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(O0_S)|E1_NN",
            &rows2(|m0: IN0OptOI0S, m1: E1NN| Holder8365 { own: 7, m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder8373() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(O0_S)|P1_S",
            &rows2(|m0: IN0OptOI0S, m1: P1S| Holder8373 { own: 7, m0, m1 }),
            &[("None,-", 'E'), ("Some,-", 'E')],
        )
    }

    fn holder8381() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(P0_N)|FP1_NN",
            &rows2(|m0: IN0P0N, m1: FP1NN| Holder8381 { own: 7, m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder8389() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(P0_S)|A1_NN",
            &rows2(|m0: IN0P0S, m1: A1NN| Holder8389 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder8397() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(P0_S)|O1_S",
            &rows2(|m0: IN0P0S, m1: Option<OI1S>| Holder8397 { own: 7, m0, m1 }),
            &[("-,None", 'E'), ("-,Some", 'E')],
        )
    }

    fn holder8405() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(T0_NN)|E1_SS",
            &rows2(|m0: IN0T0NN, m1: E1SS| Holder8405 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8413() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(T0_NN)|T1_NN",
            &rows2(|m0: IN0T0NN, m1: T1NN| Holder8413 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8421() -> Vec<String> {
        check(
            "nest2:H2N1:IN0(V0_NN)|FT1_NNN",
            &rows2(|m0: IN0V0NN, m1: FT1NNN| Holder8421 { own: 7, m0, m1 }),
            &[
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8429() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(A0_NN)|A1_SS",
            &rows2(|m0: IN1A0NN, m1: A1SS| Holder8429 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8437() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(A0_NN)|P1_N",
            &rows2(|m0: IN1A0NN, m1: P1N| Holder8437 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder8445() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(A0_SS)|FO1_NN",
            &rows2(|m0: IN1A0SS, m1: FO1NN| Holder8445 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder8453() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(A0_SS)|V1_NN",
            &rows2(|m0: IN1A0SS, m1: V1NN| Holder8453 { own: 7, m0, m1 }),
            &[
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8461() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(E0_NN)|O1_N",
            &rows2(|m0: IN1E0NN, m1: Option<OI1N>| Holder8461 { own: 7, m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder8469() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(E0_SS)|E1_NN",
            &rows2(|m0: IN1E0SS, m1: E1NN| Holder8469 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8477() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(E0_SS)|P1_S",
            &rows2(|m0: IN1E0SS, m1: P1S| Holder8477 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder8485() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(FO0_NN)|FP1_NN",
            &rows2(|m0: IN1FO0NN, m1: FP1NN| Holder8485 { own: 7, m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder8493() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(FP0_NN)|A1_NN",
            &rows2(|m0: IN1FP0NN, m1: A1NN| Holder8493 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder8501() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(FP0_NN)|O1_S",
            &rows2(|m0: IN1FP0NN, m1: Option<OI1S>| Holder8501 { own: 7, m0, m1 }),
            &[("-,None", 'E'), ("-,Some", 'E')],
        )
    }

    fn holder8509() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(FT0_NNN)|E1_SS",
            &rows2(|m0: IN1FT0NNN, m1: E1SS| Holder8509 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8517() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(FT0_NNN)|T1_NN",
            &rows2(|m0: IN1FT0NNN, m1: T1NN| Holder8517 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8525() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(O0_N)|FT1_NNN",
            &rows2(|m0: IN1OptOI0N, m1: FT1NNN| Holder8525 { own: 7, m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder8533() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(O0_S)|A1_SS",
            &rows2(|m0: IN1OptOI0S, m1: A1SS| Holder8533 { own: 7, m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder8541() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(O0_S)|P1_N",
            &rows2(|m0: IN1OptOI0S, m1: P1N| Holder8541 { own: 7, m0, m1 }),
            &[("None,-", 'E'), ("Some,-", 'E')],
        )
    }

    fn holder8549() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(P0_N)|FO1_NN",
            &rows2(|m0: IN1P0N, m1: FO1NN| Holder8549 { own: 7, m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder8557() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(P0_N)|V1_NN",
            &rows2(|m0: IN1P0N, m1: V1NN| Holder8557 { own: 7, m0, m1 }),
            &[("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder8565() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(P0_S)|O1_N",
            &rows2(|m0: IN1P0S, m1: Option<OI1N>| Holder8565 { own: 7, m0, m1 }),
            &[("-,None", 'E'), ("-,Some", 'E')],
        )
    }

    fn holder8573() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(T0_NN)|E1_NN",
            &rows2(|m0: IN1T0NN, m1: E1NN| Holder8573 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8581() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(T0_NN)|P1_S",
            &rows2(|m0: IN1T0NN, m1: P1S| Holder8581 { own: 7, m0, m1 }),
            &[("Dot,-", 'E'), ("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder8589() -> Vec<String> {
        check(
            "nest2:H2N1:IN1(V0_NN)|FP1_NN",
            &rows2(|m0: IN1V0NN, m1: FP1NN| Holder8589 { own: 7, m0, m1 }),
            &[("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }

    fn holder8597() -> Vec<String> {
        check(
            "nest2:H2N1:O0_N|IN0(A1_NN)",
            &rows2(|m0: Option<OI0N>, m1: IN0A1NN| Holder8597 { own: 7, m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder8605() -> Vec<String> {
        check(
            "nest2:H2N1:O0_N|IN0(FT1_NNN)",
            &rows2(|m0: Option<OI0N>, m1: IN0FT1NNN| Holder8605 { own: 7, m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder8613() -> Vec<String> {
        check(
            "nest2:H2N1:O0_N|IN0(V1_SS)",
            &rows2(|m0: Option<OI0N>, m1: IN0V1SS| Holder8613 { own: 7, m0, m1 }),
            &[
                ("None,Rect", 'R'),
                ("None,Wrap", 'R'),
                ("Some,Rect", 'R'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder8621() -> Vec<String> {
        check(
            "nest2:H2N1:O0_N|IN1(FP1_SS)",
            &rows2(|m0: Option<OI0N>, m1: IN1FP1SS| Holder8621 { own: 7, m0, m1 }),
            &[("None,-", 'R'), ("Some,-", 'R')],
        )
    }

    fn holder8629() -> Vec<String> {
        check(
            "nest2:H2N1:O0_N|IN1(V1_NN)",
            &rows2(|m0: Option<OI0N>, m1: IN1V1NN| Holder8629 { own: 7, m0, m1 }),
            &[
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder8637() -> Vec<String> {
        check(
            "nest2:H2N1:O0_N|IS0(P1_N)",
            &rows2(|m0: Option<OI0N>, m1: IS0P1N| Holder8637 { own: 7, m0, m1 }),
            &[("None,-", 'R'), ("Some,-", 'R')],
        )
    }

    fn holder8645() -> Vec<String> {
        check(
            "nest2:H2N1:O0_N|IS1(P1_N)",
            &rows2(|m0: Option<OI0N>, m1: IS1P1N| Holder8645 { own: 7, m0, m1 }),
            &[("None,-", 'R'), ("Some,-", 'R')],
        )
    }

    fn holder8653() -> Vec<String> {
        check(
            "nest2:H2N1:O0_S|IN0(FP1_NN)",
            &rows2(|m0: Option<OI0S>, m1: IN0FP1NN| Holder8653 { own: 7, m0, m1 }),
            &[("None,-", 'E'), ("Some,-", 'E')],
        )
    }

    fn holder8661() -> Vec<String> {
        check(
            "nest2:H2N1:O0_S|IN0(T1_SS)",
            &rows2(|m0: Option<OI0S>, m1: IN0T1SS| Holder8661 { own: 7, m0, m1 }),
            &[
                ("None,Dot", 'R'),
                ("None,Rect", 'R'),
                ("None,Wrap", 'R'),
                ("Some,Dot", 'R'),
                ("Some,Rect", 'R'),
                ("Some,Wrap", 'R'),
            ],
        )
    }

    fn holder8669() -> Vec<String> {
        check(
            "nest2:H2N1:O0_S|IN1(FO1_SS)",
            &rows2(|m0: Option<OI0S>, m1: IN1FO1SS| Holder8669 { own: 7, m0, m1 }),
            &[("None,-", 'R'), ("Some,-", 'R')],
        )
    }

    fn holder8677() -> Vec<String> {
        check(
            "nest2:H2N1:O0_S|IN1(T1_NN)",
            &rows2(|m0: Option<OI0S>, m1: IN1T1NN| Holder8677 { own: 7, m0, m1 }),
            &[
                ("None,Dot", 'E'),
                ("None,Rect", 'E'),
                ("None,Wrap", 'E'),
                ("Some,Dot", 'E'),
                ("Some,Rect", 'E'),
                ("Some,Wrap", 'E'),
            ],
        )
    }

    fn holder8685() -> Vec<String> {
        check(
            "nest2:H2N1:O0_S|IS0(O1_N)",
            &rows2(|m0: Option<OI0S>, m1: IS0OptOI1N| Holder8685 { own: 7, m0, m1 }),
            &[
                ("None,None", 'R'),
                ("None,Some", 'R'),
                ("Some,None", 'R'),
                ("Some,Some", 'R'),
            ],
        )
    }

    fn holder8693() -> Vec<String> {
        check(
            "nest2:H2N1:O0_S|IS1(O1_N)",
            &rows2(|m0: Option<OI0S>, m1: IS1OptOI1N| Holder8693 { own: 7, m0, m1 }),
            &[
                ("None,None", 'R'),
                ("None,Some", 'R'),
                ("Some,None", 'R'),
                ("Some,Some", 'R'),
            ],
        )
    }

    fn holder8701() -> Vec<String> {
        check(
            "nest2:H2N1:P0_N|IN0(FO1_NN)",
            &rows2(|m0: P0N, m1: IN0FO1NN| Holder8701 { own: 7, m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder8709() -> Vec<String> {
        check(
            "nest2:H2N1:P0_N|IN0(P1_S)",
            &rows2(|m0: P0N, m1: IN0P1S| Holder8709 { own: 7, m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder8717() -> Vec<String> {
        check(
            "nest2:H2N1:P0_N|IN1(E1_SS)",
            &rows2(|m0: P0N, m1: IN1E1SS| Holder8717 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder8725() -> Vec<String> {
        check(
            "nest2:H2N1:P0_N|IN1(P1_N)",
            &rows2(|m0: P0N, m1: IN1P1N| Holder8725 { own: 7, m0, m1 }),
            &[("-,-", 'E')],
        )
    }

    fn holder8733() -> Vec<String> {
        check(
            "nest2:H2N1:P0_N|IS0(E1_NN)",
            &rows2(|m0: P0N, m1: IS0E1NN| Holder8733 { own: 7, m0, m1 }),
            &[("-,Dot", 'R'), ("-,Rect", 'R'), ("-,Wrap", 'R')],
        )
    }

    fn holder8741() -> Vec<String> {
        check(
            "nest2:H2N1:P0_N|IS1(E1_NN)",
            &rows2(|m0: P0N, m1: IS1E1NN| Holder8741 { own: 7, m0, m1 }),
            &[("-,Dot", 'R'), ("-,Rect", 'R'), ("-,Wrap", 'R')],
        )
    }

    fn holder8749() -> Vec<String> {
        check(
            "nest2:H2N1:P0_S|IN0(E1_NN)",
            &rows2(|m0: P0S, m1: IN0E1NN| Holder8749 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder8757() -> Vec<String> {
        check(
            "nest2:H2N1:P0_S|IN0(O1_S)",
            &rows2(|m0: P0S, m1: IN0OptOI1S| Holder8757 { own: 7, m0, m1 }),
            &[("-,None", 'E'), ("-,Some", 'E')],
        )
    }

    fn holder8765() -> Vec<String> {
        check(
            "nest2:H2N1:P0_S|IN1(A1_SS)",
            &rows2(|m0: P0S, m1: IN1A1SS| Holder8765 { own: 7, m0, m1 }),
            &[("-,Dot", 'E'), ("-,Rect", 'E'), ("-,Wrap", 'E')],
        )
    }

    fn holder8773() -> Vec<String> {
        check(
            "nest2:H2N1:P0_S|IN1(O1_N)",
            &rows2(|m0: P0S, m1: IN1OptOI1N| Holder8773 { own: 7, m0, m1 }),
            &[("-,None", 'E'), ("-,Some", 'E')],
        )
    }

    fn holder8781() -> Vec<String> {
        check(
            "nest2:H2N1:P0_S|IS0(A1_NN)",
            &rows2(|m0: P0S, m1: IS0A1NN| Holder8781 { own: 7, m0, m1 }),
            &[("-,Dot", 'R'), ("-,Rect", 'R'), ("-,Wrap", 'R')],
        )
    }

    fn holder8789() -> Vec<String> {
        check(
            "nest2:H2N1:P0_S|IS1(A1_NN)",
            &rows2(|m0: P0S, m1: IS1A1NN| Holder8789 { own: 7, m0, m1 }),
            &[("-,Dot", 'R'), ("-,Rect", 'R'), ("-,Wrap", 'R')],
        )
    }

    fn holder8797() -> Vec<String> {
        check(
            "nest2:H2N1:T0_NN|IN0(A1_NN)",
            &rows2(|m0: T0NN, m1: IN0A1NN| Holder8797 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8805() -> Vec<String> {
        check(
            "nest2:H2N1:T0_NN|IN0(O1_S)",
            &rows2(|m0: T0NN, m1: IN0OptOI1S| Holder8805 { own: 7, m0, m1 }),
            &[
                ("Dot,None", 'E'),
                ("Dot,Some", 'E'),
                ("Rect,None", 'E'),
                ("Rect,Some", 'E'),
                ("Wrap,None", 'E'),
                ("Wrap,Some", 'E'),
            ],
        )
    }

    fn holder8813() -> Vec<String> {
        check(
            "nest2:H2N1:T0_NN|IN1(E1_SS)",
            &rows2(|m0: T0NN, m1: IN1E1SS| Holder8813 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8821() -> Vec<String> {
        check(
            "nest2:H2N1:T0_NN|IN1(T1_NN)",
            &rows2(|m0: T0NN, m1: IN1T1NN| Holder8821 { own: 7, m0, m1 }),
            &[
                ("Dot,Dot", 'E'),
                ("Dot,Rect", 'E'),
                ("Dot,Wrap", 'E'),
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8829() -> Vec<String> {
        check(
            "nest2:H2N1:V0_NN|IN0(FT1_NNN)",
            &rows2(|m0: V0NN, m1: IN0FT1NNN| Holder8829 { own: 7, m0, m1 }),
            &[
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8837() -> Vec<String> {
        check(
            "nest2:H2N1:V0_NN|IN1(A1_SS)",
            &rows2(|m0: V0NN, m1: IN1A1SS| Holder8837 { own: 7, m0, m1 }),
            &[
                ("Rect,Dot", 'E'),
                ("Rect,Rect", 'E'),
                ("Rect,Wrap", 'E'),
                ("Wrap,Dot", 'E'),
                ("Wrap,Rect", 'E'),
                ("Wrap,Wrap", 'E'),
            ],
        )
    }

    fn holder8845() -> Vec<String> {
        check(
            "nest2:H2N1:V0_NN|IN1(P1_N)",
            &rows2(|m0: V0NN, m1: IN1P1N| Holder8845 { own: 7, m0, m1 }),
            &[("Rect,-", 'E'), ("Wrap,-", 'E')],
        )
    }
}
