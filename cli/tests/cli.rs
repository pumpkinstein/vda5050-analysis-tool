use std::{
    error::Error,
    fs,
    path::PathBuf,
    process::{Command, Output},
};

fn sample_log() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../log-file-parser/test-data/sample.log")
}

fn sample_manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../log-file-parser/test-data/sample.vda5050-expected.yaml")
}

fn cli_output(arguments: &[&str]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_cli"))
        .args(arguments)
        .output()?)
}

#[test]
fn quick_view_preserves_the_report_path() -> Result<(), Box<dyn Error>> {
    let file = sample_log();
    let file = file.to_string_lossy();
    let output = cli_output(["quick-view", "--file", &file].as_slice())?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("Parsing file:"));
    assert!(stdout.contains("=== Index DataFrame ==="));
    assert!(stdout.contains("=== Summary Statistics ==="));
    Ok(())
}

#[test]
fn parser_errors_return_nonzero_status() -> Result<(), Box<dyn Error>> {
    let output = cli_output(["check", "--file", "/definitely/missing/vda5050.log"].as_slice())?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("No such file"));
    Ok(())
}

#[test]
fn wrong_source_hash_is_reported_without_rewriting_the_manifest() -> Result<(), Box<dyn Error>> {
    let manifest_path = std::env::temp_dir().join(format!(
        "vda5050-cli-wrong-hash-{}.yaml",
        std::process::id()
    ));
    let original = fs::read_to_string(sample_manifest())?;
    let wrong = original.replace(
        "a72e2b063b7e412db9f649144e38153e6593dcdd80fb8c7d4ee22eb8973d7510",
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
    fs::write(&manifest_path, &wrong)?;
    let file = sample_log();
    let file = file.to_string_lossy();
    let manifest = manifest_path.to_string_lossy();

    let output = cli_output(["check", "--file", &file, "--expect", &manifest].as_slice())?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("source.sha256"));
    assert_eq!(fs::read_to_string(&manifest_path)?, wrong);

    fs::remove_file(manifest_path)?;
    Ok(())
}

#[test]
fn changed_aggregate_expectation_is_reported() -> Result<(), Box<dyn Error>> {
    let manifest_path = std::env::temp_dir().join(format!(
        "vda5050-cli-wrong-aggregate-{}.yaml",
        std::process::id()
    ));
    let original = fs::read_to_string(sample_manifest())?;
    let wrong = original.replace("unique_robots: 3", "unique_robots: 99");
    fs::write(&manifest_path, wrong)?;
    let file = sample_log();
    let file = file.to_string_lossy();
    let manifest = manifest_path.to_string_lossy();

    let output = cli_output(["check", "--file", &file, "--expect", &manifest].as_slice())?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("expected.unique_robots"));

    fs::remove_file(manifest_path)?;
    Ok(())
}

#[test]
fn snapshot_refuses_overwrite_without_explicit_opt_in() -> Result<(), Box<dyn Error>> {
    let output_path =
        std::env::temp_dir().join(format!("vda5050-cli-snapshot-{}.yaml", std::process::id()));
    let _ = fs::remove_file(&output_path);
    let file = sample_log();
    let file = file.to_string_lossy();
    let output = output_path.to_string_lossy();

    let first = cli_output(["snapshot", "--file", &file, "--output", &output].as_slice())?;
    assert!(first.status.success());
    let original = fs::read(&output_path)?;

    let second = cli_output(["snapshot", "--file", &file, "--output", &output].as_slice())?;
    assert!(!second.status.success());
    assert!(String::from_utf8_lossy(&second.stderr).contains("refusing to overwrite"));
    assert_eq!(fs::read(&output_path)?, original);

    fs::remove_file(output_path)?;
    Ok(())
}
