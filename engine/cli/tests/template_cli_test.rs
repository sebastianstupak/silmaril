use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper to run the CLI binary with the given arguments
fn run_cli(args: &[&str], working_dir: &PathBuf) -> Result<std::process::Output> {
    let binary_path = env!("CARGO_BIN_EXE_silm");

    let output = Command::new(binary_path).args(args).current_dir(working_dir).output()?;

    Ok(output)
}

/// Helper to check if output contains expected text
fn assert_output_contains(output: &std::process::Output, expected: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    assert!(
        combined.contains(expected),
        "Expected output to contain '{}'\nStdout: {}\nStderr: {}",
        expected,
        stdout,
        stderr
    );
}

#[test]
fn test_template_add_level() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let output = run_cli(
        &[
            "template",
            "add",
            "test_level",
            "--type",
            "level",
            "--description",
            "Test level template",
        ],
        &temp_dir.path().to_path_buf(),
    )?;

    assert!(output.status.success(), "Command should succeed");
    assert_output_contains(&output, "Creating template: test_level");
    assert_output_contains(&output, "Type: level");

    // Check that file was created
    let template_path = temp_dir.path().join("assets/templates/levels/test_level.yaml");
    assert!(template_path.exists(), "Template file should be created at {:?}", template_path);

    // Verify template content
    let content = fs::read_to_string(&template_path)?;
    assert!(content.contains("metadata:"));
    assert!(content.contains("entities:"));
    assert!(content.contains("Root:"));

    Ok(())
}

#[test]
fn test_template_add_character() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let output = run_cli(
        &["template", "add", "player", "--type", "character", "--author", "Test Author"],
        &temp_dir.path().to_path_buf(),
    )?;

    assert!(output.status.success(), "Command should succeed");
    assert_output_contains(&output, "Type: character");
    assert_output_contains(&output, "Author: Test Author");

    let template_path = temp_dir.path().join("assets/templates/characters/player.yaml");
    assert!(template_path.exists(), "Character template should be created");

    Ok(())
}

#[test]
fn test_template_add_all_types() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let types = vec![
        ("level", "levels"),
        ("character", "characters"),
        ("prop", "props"),
        ("ui", "ui"),
        ("game_state", "game_state"),
    ];

    for (template_type, folder) in types {
        let template_name = format!("test_{}", template_type);
        let output = run_cli(
            &["template", "add", &template_name, "--type", template_type],
            &temp_dir.path().to_path_buf(),
        )?;

        assert!(output.status.success(), "Command should succeed for type: {}", template_type);

        let template_path = temp_dir
            .path()
            .join(format!("assets/templates/{}/{}.yaml", folder, template_name));
        assert!(template_path.exists(), "Template should be created at {:?}", template_path);
    }

    Ok(())
}

#[test]
fn test_template_add_already_exists() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create first template
    run_cli(
        &["template", "add", "duplicate", "--type", "level"],
        &temp_dir.path().to_path_buf(),
    )?;

    // Try to create same template again
    let output = run_cli(
        &["template", "add", "duplicate", "--type", "level"],
        &temp_dir.path().to_path_buf(),
    )?;

    assert!(!output.status.success(), "Command should fail for duplicate");
    assert_output_contains(&output, "already exists");

    Ok(())
}

#[test]
fn test_template_validate_valid() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create a template first
    run_cli(
        &["template", "add", "valid_template", "--type", "level"],
        &temp_dir.path().to_path_buf(),
    )?;

    let template_path = temp_dir.path().join("assets/templates/levels/valid_template.yaml");

    // Validate it
    let output = run_cli(
        &["template", "validate", template_path.to_str().unwrap()],
        &temp_dir.path().to_path_buf(),
    )?;

    assert!(output.status.success(), "Validation should succeed");
    assert_output_contains(&output, "Template is valid");

    Ok(())
}

#[test]
fn test_template_validate_not_found() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let output =
        run_cli(&["template", "validate", "nonexistent.yaml"], &temp_dir.path().to_path_buf())?;

    assert!(!output.status.success(), "Validation should fail");
    assert_output_contains(&output, "not found");

    Ok(())
}

#[test]
fn test_template_validate_invalid_yaml() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let template_path = temp_dir.path().join("invalid.yaml");

    // Create invalid YAML
    fs::write(&template_path, "invalid: yaml: content: [")?;

    let output = run_cli(
        &["template", "validate", template_path.to_str().unwrap()],
        &temp_dir.path().to_path_buf(),
    )?;

    assert!(!output.status.success(), "Validation should fail");
    assert_output_contains(&output, "Invalid YAML");

    Ok(())
}

#[test]
fn test_template_list_empty() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let output = run_cli(&["template", "list"], &temp_dir.path().to_path_buf())?;

    assert!(output.status.success(), "List should succeed");
    assert_output_contains(&output, "No templates");

    Ok(())
}

#[test]
fn test_template_list_with_templates() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create several templates
    run_cli(
        &["template", "add", "level1", "--type", "level"],
        &temp_dir.path().to_path_buf(),
    )?;
    run_cli(
        &["template", "add", "player", "--type", "character"],
        &temp_dir.path().to_path_buf(),
    )?;
    run_cli(&["template", "add", "chest", "--type", "prop"], &temp_dir.path().to_path_buf())?;

    let output = run_cli(&["template", "list"], &temp_dir.path().to_path_buf())?;

    assert!(output.status.success(), "List should succeed");
    assert_output_contains(&output, "level1.yaml");
    assert_output_contains(&output, "player.yaml");
    assert_output_contains(&output, "chest.yaml");

    Ok(())
}

#[test]
fn test_template_tree() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create a template
    run_cli(
        &["template", "add", "hierarchy", "--type", "level"],
        &temp_dir.path().to_path_buf(),
    )?;

    let template_path = temp_dir.path().join("assets/templates/levels/hierarchy.yaml");

    let output = run_cli(
        &["template", "tree", template_path.to_str().unwrap()],
        &temp_dir.path().to_path_buf(),
    )?;

    assert!(output.status.success(), "Tree should succeed");
    assert_output_contains(&output, "Template hierarchy");
    assert_output_contains(&output, "Root");

    Ok(())
}

#[test]
fn test_template_rename() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create a template
    run_cli(
        &["template", "add", "old_name", "--type", "level"],
        &temp_dir.path().to_path_buf(),
    )?;

    let old_path = temp_dir.path().join("assets/templates/levels/old_name.yaml");
    let new_path = temp_dir.path().join("assets/templates/levels/new_name.yaml");

    assert!(old_path.exists(), "Original file should exist");

    // Rename it
    let output = run_cli(
        &["template", "rename", old_path.to_str().unwrap(), "new_name"],
        &temp_dir.path().to_path_buf(),
    )?;

    assert!(output.status.success(), "Rename should succeed");
    assert_output_contains(&output, "Renamed to");

    assert!(!old_path.exists(), "Old file should not exist");
    assert!(new_path.exists(), "New file should exist");

    Ok(())
}

#[test]
fn test_template_rename_conflict() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create two templates
    run_cli(
        &["template", "add", "template1", "--type", "level"],
        &temp_dir.path().to_path_buf(),
    )?;
    run_cli(
        &["template", "add", "template2", "--type", "level"],
        &temp_dir.path().to_path_buf(),
    )?;

    let template1_path = temp_dir.path().join("assets/templates/levels/template1.yaml");

    // Try to rename template1 to template2 (which already exists)
    let output = run_cli(
        &["template", "rename", template1_path.to_str().unwrap(), "template2"],
        &temp_dir.path().to_path_buf(),
    )?;

    assert!(!output.status.success(), "Rename should fail");
    assert_output_contains(&output, "already exists");

    Ok(())
}

#[test]
fn test_template_delete_with_confirmation() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create a template
    run_cli(
        &["template", "add", "to_delete", "--type", "level"],
        &temp_dir.path().to_path_buf(),
    )?;

    let template_path = temp_dir.path().join("assets/templates/levels/to_delete.yaml");

    assert!(template_path.exists(), "Template should exist");

    // Delete with --yes flag (skip confirmation)
    let output = run_cli(
        &["template", "delete", template_path.to_str().unwrap(), "--yes"],
        &temp_dir.path().to_path_buf(),
    )?;

    assert!(output.status.success(), "Delete should succeed");
    assert_output_contains(&output, "deleted");

    assert!(!template_path.exists(), "Template should be deleted");

    Ok(())
}

#[test]
fn test_template_delete_not_found() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let output = run_cli(
        &["template", "delete", "nonexistent.yaml", "--yes"],
        &temp_dir.path().to_path_buf(),
    )?;

    assert!(!output.status.success(), "Delete should fail");
    assert_output_contains(&output, "not found");

    Ok(())
}

#[test]
fn test_template_compile() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create a template
    run_cli(
        &["template", "add", "to_compile", "--type", "level"],
        &temp_dir.path().to_path_buf(),
    )?;

    let template_path = temp_dir.path().join("assets/templates/levels/to_compile.yaml");

    // Compile it
    let output = run_cli(
        &["template", "compile", template_path.to_str().unwrap()],
        &temp_dir.path().to_path_buf(),
    )?;

    assert!(output.status.success(), "Compile should succeed");
    assert_output_contains(&output, "compile");

    Ok(())
}

#[test]
fn test_template_compile_with_output() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create a template
    run_cli(
        &["template", "add", "to_compile2", "--type", "level"],
        &temp_dir.path().to_path_buf(),
    )?;

    let template_path = temp_dir.path().join("assets/templates/levels/to_compile2.yaml");
    let output_path = temp_dir.path().join("compiled.bin");

    // Compile with output path
    let output = run_cli(
        &[
            "template",
            "compile",
            template_path.to_str().unwrap(),
            "--output",
            output_path.to_str().unwrap(),
        ],
        &temp_dir.path().to_path_buf(),
    )?;

    assert!(output.status.success(), "Compile with output should succeed");
    assert_output_contains(&output, "compile");

    Ok(())
}

#[test]
fn test_template_help() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let output = run_cli(&["template", "--help"], &temp_dir.path().to_path_buf())?;

    assert!(output.status.success(), "Help should succeed");
    assert_output_contains(&output, "template");
    assert_output_contains(&output, "add");
    assert_output_contains(&output, "validate");
    assert_output_contains(&output, "compile");
    assert_output_contains(&output, "list");
    assert_output_contains(&output, "tree");
    assert_output_contains(&output, "rename");
    assert_output_contains(&output, "delete");

    Ok(())
}

#[test]
fn test_template_add_help() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let output = run_cli(&["template", "add", "--help"], &temp_dir.path().to_path_buf())?;

    assert!(output.status.success(), "Add help should succeed");
    assert_output_contains(&output, "Create a new template");
    assert_output_contains(&output, "--type");
    assert_output_contains(&output, "--description");
    assert_output_contains(&output, "--author");

    Ok(())
}
