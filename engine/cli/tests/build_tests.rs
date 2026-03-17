//! Comprehensive tests for the build command foundation layer (Tasks 1-3).

use silm::commands::build::env::{merge_env, parse_build_env, parse_build_section, parse_env_file};
use silm::commands::build::package::{generate_dockerfile, zip_filename};
use silm::commands::build::{
    check_tool, dist_dir_name, host_target_triple, platform_from_str, BuildKind, BuildTool,
    KNOWN_PLATFORMS,
};

// ============================================================================
// parse_env_file
// ============================================================================

#[test]
fn test_parse_env_file_basic() {
    let content = "FOO=bar\nBAZ=qux";
    let result = parse_env_file(content);
    assert_eq!(
        result,
        vec![
            ("FOO".into(), "bar".into()),
            ("BAZ".into(), "qux".into()),
        ]
    );
}

#[test]
fn test_parse_env_file_comments_and_blanks() {
    let content = "# a comment\n\nKEY=value\n  \n# another comment\nKEY2=val2\n";
    let result = parse_env_file(content);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0, "KEY");
    assert_eq!(result[1].0, "KEY2");
}

#[test]
fn test_parse_env_file_blank_value() {
    let content = "EMPTY=";
    let result = parse_env_file(content);
    assert_eq!(result, vec![("EMPTY".into(), "".into())]);
}

#[test]
fn test_parse_env_file_duplicate_keys() {
    let content = "DUP=first\nDUP=second";
    let result = parse_env_file(content);
    // Both appear; merge_env will handle dedup via HashMap insertion order
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].1, "first");
    assert_eq!(result[1].1, "second");
}

#[test]
fn test_parse_env_file_no_equals_lines_skipped() {
    let content = "GOOD=yes\nno-equals-here\nALSO_GOOD=yep";
    let result = parse_env_file(content);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0, "GOOD");
    assert_eq!(result[1].0, "ALSO_GOOD");
}

#[test]
fn test_parse_env_file_value_with_equals() {
    let content = "URL=https://example.com?a=1&b=2";
    let result = parse_env_file(content);
    assert_eq!(result[0].1, "https://example.com?a=1&b=2");
}

#[test]
fn test_parse_env_file_value_not_trimmed() {
    let content = "SPACED=  hello  ";
    let result = parse_env_file(content);
    assert_eq!(result[0].1, "  hello  ");
}

// ============================================================================
// parse_build_env
// ============================================================================

#[test]
fn test_parse_build_env_basic() {
    let content = r#"
[build.env]
RUST_LOG = "debug"
SERVER_PORT = "7777"
"#;
    let result = parse_build_env(content);
    assert!(result.iter().any(|(k, v)| k == "RUST_LOG" && v == "debug"));
    assert!(result
        .iter()
        .any(|(k, v)| k == "SERVER_PORT" && v == "7777"));
}

#[test]
fn test_parse_build_env_ignores_other_sections() {
    let content = r#"
[package]
name = "my-game"

[build.env]
ONLY_THIS = "yes"

[other]
not_this = "no"
"#;
    let result = parse_build_env(content);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, "ONLY_THIS");
}

#[test]
fn test_parse_build_env_empty_when_absent() {
    let content = r#"
[package]
name = "my-game"
"#;
    let result = parse_build_env(content);
    assert!(result.is_empty());
}

#[test]
fn test_parse_build_env_integer_value() {
    let content = r#"
[build.env]
PORT = 8080
"#;
    let result = parse_build_env(content);
    assert_eq!(result.len(), 1);
    // Integer values get stringified
    assert_eq!(result[0].1, "8080");
}

// ============================================================================
// parse_build_section
// ============================================================================

#[test]
fn test_parse_build_section_platforms_list() {
    let content = r#"
[build]
platforms = ["native", "linux-x86_64", "wasm"]
"#;
    let result = parse_build_section(content).unwrap();
    assert_eq!(result, vec!["native", "linux-x86_64", "wasm"]);
}

#[test]
fn test_parse_build_section_absent() {
    let content = r#"
[package]
name = "my-game"
"#;
    assert!(parse_build_section(content).is_none());
}

#[test]
fn test_parse_build_section_many_platforms() {
    let content = r#"
[build]
platforms = ["native", "server", "windows-x86_64", "linux-x86_64", "linux-arm64", "macos-x86_64", "macos-arm64", "wasm"]
"#;
    let result = parse_build_section(content).unwrap();
    assert_eq!(result.len(), 8);
}

#[test]
fn test_parse_build_section_no_platforms_key() {
    let content = r#"
[build]
optimization = "release"
"#;
    assert!(parse_build_section(content).is_none());
}

// ============================================================================
// merge_env
// ============================================================================

#[test]
fn test_merge_env_priority_order() {
    let build_env = vec![("KEY".into(), "from_build".into())];
    let dotenv = vec![("KEY".into(), "from_dotenv".into())];
    let env_file = vec![("KEY".into(), "from_file".into())];

    let result = merge_env(&build_env, &dotenv, &env_file);
    assert_eq!(result.get("KEY").unwrap(), "from_file");
}

#[test]
fn test_merge_env_build_env_lowest() {
    let build_env = vec![("A".into(), "build".into())];
    let dotenv = vec![("A".into(), "dotenv".into())];
    let env_file: Vec<(String, String)> = vec![];

    let result = merge_env(&build_env, &dotenv, &env_file);
    assert_eq!(result.get("A").unwrap(), "dotenv");
}

#[test]
fn test_merge_env_disjoint_keys() {
    let build_env = vec![("A".into(), "1".into())];
    let dotenv = vec![("B".into(), "2".into())];
    let env_file = vec![("C".into(), "3".into())];

    let result = merge_env(&build_env, &dotenv, &env_file);
    assert_eq!(result.len(), 3);
    assert_eq!(result["A"], "1");
    assert_eq!(result["B"], "2");
    assert_eq!(result["C"], "3");
}

#[test]
fn test_merge_env_shell_wins() {
    // Use a unique key unlikely to exist in the real environment
    let key = "_SILM_BUILD_TEST_SHELL_WINS";
    unsafe {
        std::env::set_var(key, "from_shell");
    }

    let build_env = vec![(key.into(), "from_build".into())];
    let dotenv: Vec<(String, String)> = vec![];
    let env_file: Vec<(String, String)> = vec![];

    let result = merge_env(&build_env, &dotenv, &env_file);
    // Shell var exists, so the key should be filtered out
    assert!(!result.contains_key(key));

    // Clean up
    unsafe {
        std::env::remove_var(key);
    }
}

// ============================================================================
// platform_from_str
// ============================================================================

#[test]
fn test_platform_from_str_native() {
    let p = platform_from_str("native").unwrap();
    assert_eq!(p.name(), "native");
    assert_eq!(p.tool(), BuildTool::Cargo);
    assert_eq!(p.kind(), BuildKind::ServerAndClient);
    assert!(!p.experimental());
    // target triple should match host
    assert_eq!(p.target_triple(), host_target_triple());
}

#[test]
fn test_platform_from_str_server() {
    let p = platform_from_str("server").unwrap();
    assert_eq!(p.name(), "server");
    assert_eq!(p.kind(), BuildKind::ServerOnly);
    assert_eq!(p.tool(), BuildTool::Cargo);
}

#[test]
fn test_platform_from_str_windows_x86_64() {
    let p = platform_from_str("windows-x86_64").unwrap();
    assert_eq!(p.name(), "windows-x86_64");
    assert!(p.uses_exe_extension());
    assert!(!p.experimental());
    // Tool depends on host
    if std::env::consts::OS == "windows" {
        assert_eq!(p.tool(), BuildTool::Cargo);
        assert_eq!(p.target_triple(), "x86_64-pc-windows-msvc");
    } else {
        assert_eq!(p.tool(), BuildTool::Cross);
        assert_eq!(p.target_triple(), "x86_64-pc-windows-gnu");
    }
}

#[test]
fn test_platform_from_str_linux_x86_64() {
    let p = platform_from_str("linux-x86_64").unwrap();
    assert_eq!(p.target_triple(), "x86_64-unknown-linux-gnu");
    assert_eq!(p.tool(), BuildTool::Cross);
    assert!(!p.uses_exe_extension());
}

#[test]
fn test_platform_from_str_linux_arm64() {
    let p = platform_from_str("linux-arm64").unwrap();
    assert_eq!(p.target_triple(), "aarch64-unknown-linux-gnu");
    assert_eq!(p.tool(), BuildTool::Cross);
}

#[test]
fn test_platform_from_str_macos_x86_64() {
    let p = platform_from_str("macos-x86_64").unwrap();
    assert_eq!(p.target_triple(), "x86_64-apple-darwin");
    assert!(p.experimental());
}

#[test]
fn test_platform_from_str_macos_arm64() {
    let p = platform_from_str("macos-arm64").unwrap();
    assert_eq!(p.target_triple(), "aarch64-apple-darwin");
    assert!(p.experimental());
}

#[test]
fn test_platform_from_str_wasm() {
    let p = platform_from_str("wasm").unwrap();
    assert_eq!(p.target_triple(), "wasm32-unknown-unknown");
    assert_eq!(p.tool(), BuildTool::Trunk);
    assert_eq!(p.kind(), BuildKind::ClientOnly);
    assert!(!p.uses_exe_extension());
}

#[test]
fn test_platform_from_str_unknown() {
    let result = platform_from_str("playstation-5");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("Unknown platform"));
    assert!(msg.contains("playstation-5"));
}

#[test]
fn test_all_known_platforms_resolve() {
    for name in KNOWN_PLATFORMS {
        let result = platform_from_str(name);
        assert!(result.is_ok(), "Failed to resolve platform: {name}");
    }
}

// ============================================================================
// dist_dir_name
// ============================================================================

#[test]
fn test_dist_dir_name_all_platforms() {
    for name in KNOWN_PLATFORMS {
        let p = platform_from_str(name).unwrap();
        assert_eq!(dist_dir_name(&p), *name);
    }
}

// ============================================================================
// host_target_triple
// ============================================================================

#[test]
fn test_host_target_triple_not_empty() {
    let triple = host_target_triple();
    assert!(!triple.is_empty());
    // Should contain arch and os info
    assert!(triple.contains('-'));
}

// ============================================================================
// zip_filename
// ============================================================================

#[test]
fn test_zip_filename_basic() {
    assert_eq!(
        zip_filename("my-game", "1.0.0", "linux-x86_64"),
        "my-game-v1.0.0-linux-x86_64.zip"
    );
}

#[test]
fn test_zip_filename_various() {
    assert_eq!(
        zip_filename("silmaril", "0.2.3", "wasm"),
        "silmaril-v0.2.3-wasm.zip"
    );
}

// ============================================================================
// generate_dockerfile
// ============================================================================

#[test]
fn test_generate_dockerfile_basic() {
    let env = vec![("RUST_LOG".into(), "info".into())];
    let df = generate_dockerfile(&env);
    assert!(df.contains("FROM debian:bookworm-slim"));
    assert!(df.contains("COPY server /usr/local/bin/server"));
    assert!(df.contains("EXPOSE 7777/udp"));
    assert!(df.contains("ENV RUST_LOG=info"));
    assert!(df.contains("ENTRYPOINT [\"/usr/local/bin/server\"]"));
}

#[test]
fn test_generate_dockerfile_multiple_env() {
    let env = vec![
        ("A".into(), "1".into()),
        ("B".into(), "2".into()),
        ("C".into(), "3".into()),
    ];
    let df = generate_dockerfile(&env);
    assert!(df.contains("ENV A=1"));
    assert!(df.contains("ENV B=2"));
    assert!(df.contains("ENV C=3"));
}

#[test]
fn test_generate_dockerfile_no_env() {
    let df = generate_dockerfile(&[]);
    assert!(df.contains("FROM debian:bookworm-slim"));
    assert!(df.contains("ENTRYPOINT"));
    assert!(!df.contains("ENV "));
}

// ============================================================================
// check_tool (error path)
// ============================================================================

#[test]
fn test_check_tool_nonexistent() {
    let result = check_tool("__silm_nonexistent_tool_xyz__");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("not found"));
    assert!(msg.contains("cargo install"));
}
