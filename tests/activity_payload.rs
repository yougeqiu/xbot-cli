//! Isolated managed helper regression. No ShadowBot assembly or live client is loaded.
#[test]
fn activity_payload_install_and_metadata_guards() {
    let output = std::process::Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/scripts/test-activity-payload.ps1"
        ))
        .output()
        .expect("PowerShell 7 is required for isolated payload tests");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS:"));
}
