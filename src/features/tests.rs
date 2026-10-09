//! Unit test of the feature detection.

use super::*;

#[test]
fn test_feature_detection() {
    let enabled = Features::enabled_features();
    log::debug!("Enabled features: {enabled:?}");

    #[cfg(all(
        feature = "serde",
        feature = "zod",
        feature = "jsonschema",
        feature = "mongodb",
        feature = "typescript"
    ))]
    {
        assert!(Features::has_serde());
        assert!(Features::has_zod());
        assert!(Features::has_jsonschema());
        assert!(Features::has_mongodb());
        assert!(Features::has_typescript());
    }
}
