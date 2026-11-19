use super::file_error;
use std::path::{Path, MAIN_SEPARATOR};

#[test]
fn file_errors_use_native_separators_for_caller_paths() {
    let expected = Path::new("assets").join("rooms").join("second.ron");
    // Simulate a caller supplying mixed separators on Windows.
    let caller = expected.to_string_lossy().replacen(MAIN_SEPARATOR, "/", 1);
    let error = file_error(Path::new(&caller), "invalid patrol interval");
    let (reported, reason) = error.split_once(": ").unwrap();
    assert_eq!(Path::new(reported), expected);
    assert_eq!(Path::new(reported).file_name(), Some("second.ron".as_ref()));
    assert!(reported
        .chars()
        .filter(|c| matches!(c, '/' | '\\'))
        .all(|c| c == MAIN_SEPARATOR));
    assert_eq!(reason, "invalid patrol interval");
}
