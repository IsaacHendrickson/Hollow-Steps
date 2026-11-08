use std::path::PathBuf;

/// Fixed input data for behavioral tests; never copy the editable game assets.
pub fn fixture_assets() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}
