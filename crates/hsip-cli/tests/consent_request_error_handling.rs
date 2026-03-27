//! Integration tests for consent request error handling
//! Verifies that CLI exits cleanly instead of panicking on invalid input

use std::process::Command;

#[test]
fn test_missing_request_file_no_panic() {
    use std::env;

    // Use cross-platform temp directory
    let temp_dir = env::temp_dir();
    let nonexistent_path = temp_dir.join("nonexistent_request_file_hsip_test.json");

    let output = Command::new(env!("CARGO_BIN_EXE_hsip-cli"))
        .args([
            "consent-send-request",
            "--to",
            "127.0.0.1:40404",
            "--file",
            nonexistent_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute CLI");

    // Should exit with error code (not 0)
    assert!(!output.status.success(), "should fail with non-zero exit code");

    // Should NOT panic (panic would show "panicked at" in stderr)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked at"),
        "should not panic; stderr: {}",
        stderr
    );

    // Should contain helpful error message
    assert!(
        stderr.contains("failed to read request file"),
        "should show clear error message; stderr: {}",
        stderr
    );
}

#[test]
fn test_invalid_json_request_file_no_panic() {
    use std::env;
    use std::fs;
    use std::io::Write;

    // Create a temp file with invalid JSON (cross-platform)
    let temp_dir = env::temp_dir();
    let temp_path = temp_dir.join("hsip_test_invalid.json");
    let mut f = fs::File::create(&temp_path).expect("create temp file");
    f.write_all(b"{ invalid json ").expect("write invalid json");
    drop(f);

    let output = Command::new(env!("CARGO_BIN_EXE_hsip-cli"))
        .args([
            "consent-send-request",
            "--to",
            "127.0.0.1:40404",
            "--file",
            temp_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute CLI");

    // Clean up
    let _ = fs::remove_file(&temp_path);

    // Should exit with error code
    assert!(!output.status.success(), "should fail with non-zero exit code");

    // Should NOT panic
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked at"),
        "should not panic; stderr: {}",
        stderr
    );

    // Should contain JSON parse error message
    assert!(
        stderr.contains("failed to parse request JSON"),
        "should show JSON parse error; stderr: {}",
        stderr
    );
}

#[test]
fn test_empty_request_file_no_panic() {
    use std::env;
    use std::fs;

    // Create empty file (cross-platform)
    let temp_dir = env::temp_dir();
    let temp_path = temp_dir.join("hsip_test_empty.json");
    fs::File::create(&temp_path).expect("create empty file");

    let output = Command::new(env!("CARGO_BIN_EXE_hsip-cli"))
        .args([
            "consent-send-request",
            "--to",
            "127.0.0.1:40404",
            "--file",
            temp_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute CLI");

    // Clean up
    let _ = fs::remove_file(&temp_path);

    // Should exit with error code
    assert!(!output.status.success(), "should fail with non-zero exit code");

    // Should NOT panic
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked at"),
        "should not panic; stderr: {}",
        stderr
    );

    // Should contain parse error (empty file is invalid JSON)
    assert!(
        stderr.contains("failed to parse request JSON") || stderr.contains("EOF"),
        "should show parse error; stderr: {}",
        stderr
    );
}
