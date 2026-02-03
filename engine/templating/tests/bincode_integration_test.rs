//! Integration tests for bincode template compilation and loading.
//!
//! These tests verify the complete workflow:
//! 1. YAML → Bincode compilation
//! 2. Bincode template loading
//! 3. Roundtrip validation
//! 4. Auto-detection of .bin vs .yaml
//! 5. Checksum validation

use engine_core::ecs::World;
use engine_templating::{
    EntityDefinition, Template, TemplateCompiler, TemplateLoader, TemplateMetadata,
};
use rustc_hash::FxHashMap;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_compile_yaml_to_bincode() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let yaml_path = temp_dir.path().join("test.yaml");
    let bin_path = temp_dir.path().join("test.bin");

    // Create a test template
    let metadata = TemplateMetadata {
        name: Some("Test Template".to_string()),
        description: Some("A test template for bincode compilation".to_string()),
        author: Some("Test Suite".to_string()),
        version: Some("1.0".to_string()),
    };

    let mut template = Template::new(metadata.clone());
    let entity = EntityDefinition::new_inline(FxHashMap::default(), vec!["test".to_string()]);
    template.add_entity("Root".to_string(), entity);

    // Write YAML
    let yaml = serde_yaml::to_string(&template).expect("Failed to serialize");
    fs::write(&yaml_path, yaml).expect("Failed to write YAML");

    // Compile to bincode
    let compiler = TemplateCompiler::new();
    let result = compiler.compile(&yaml_path, &bin_path);

    assert!(result.is_ok(), "Compilation should succeed");
    assert!(bin_path.exists(), "Bincode file should be created");

    // Verify by loading the compiled template
    let loaded = compiler.load_compiled(&bin_path).expect("Failed to load compiled template");
    assert_eq!(loaded.metadata.name, Some("Test Template".to_string()));
    assert_eq!(loaded.entity_count(), 1);
}

#[test]
fn test_load_compiled_template() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let yaml_path = temp_dir.path().join("test.yaml");
    let bin_path = temp_dir.path().join("test.bin");

    // Create and compile a template
    let metadata = TemplateMetadata {
        name: Some("Load Test".to_string()),
        description: None,
        author: None,
        version: Some("1.0".to_string()),
    };

    let mut template = Template::new(metadata.clone());
    template.add_entity(
        "Entity1".to_string(),
        EntityDefinition::new_inline(FxHashMap::default(), vec![]),
    );
    template.add_entity(
        "Entity2".to_string(),
        EntityDefinition::new_inline(FxHashMap::default(), vec![]),
    );

    let yaml = serde_yaml::to_string(&template).expect("Failed to serialize");
    fs::write(&yaml_path, yaml).expect("Failed to write YAML");

    let compiler = TemplateCompiler::new();
    compiler.compile(&yaml_path, &bin_path).expect("Compilation failed");

    // Load the compiled template
    let loaded = compiler.load_compiled(&bin_path).expect("Failed to load");

    assert_eq!(loaded.metadata.name, Some("Load Test".to_string()));
    assert_eq!(loaded.entity_count(), 2);
}

#[test]
fn test_roundtrip_yaml_bincode_template() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let yaml_path = temp_dir.path().join("roundtrip.yaml");
    let bin_path = temp_dir.path().join("roundtrip.bin");

    // Create original template
    let original_metadata = TemplateMetadata {
        name: Some("Roundtrip Test".to_string()),
        description: Some("Testing data integrity".to_string()),
        author: Some("Test Suite".to_string()),
        version: Some("2.0".to_string()),
    };

    let mut original = Template::new(original_metadata.clone());

    for i in 0..5 {
        let entity = EntityDefinition::new_inline(
            FxHashMap::default(),
            vec![format!("tag_{}", i), "test".to_string()],
        );
        original.add_entity(format!("Entity_{}", i), entity);
    }

    // Write YAML
    let yaml = serde_yaml::to_string(&original).expect("Failed to serialize");
    fs::write(&yaml_path, &yaml).expect("Failed to write YAML");

    // Compile to bincode
    let compiler = TemplateCompiler::new();
    compiler.compile(&yaml_path, &bin_path).expect("Compilation failed");

    // Load from bincode
    let loaded = compiler.load_compiled(&bin_path).expect("Failed to load");

    // Verify data integrity
    assert_eq!(loaded.metadata.name, original.metadata.name);
    assert_eq!(loaded.metadata.description, original.metadata.description);
    assert_eq!(loaded.metadata.author, original.metadata.author);
    assert_eq!(loaded.metadata.version, original.metadata.version);
    assert_eq!(loaded.entity_count(), original.entity_count());

    // Verify entities match
    for (name, entity) in &original.entities {
        let loaded_entity = loaded.get_entity(name).expect("Entity should exist");
        assert_eq!(loaded_entity.is_inline(), entity.is_inline());
    }
}

#[test]
fn test_auto_detection_bin_preferred_over_yaml() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let yaml_path = temp_dir.path().join("autodetect.yaml");
    let bin_path = temp_dir.path().join("autodetect.bin");

    // Create YAML template with specific metadata
    let yaml_metadata = TemplateMetadata {
        name: Some("YAML Version".to_string()),
        description: Some("This is the YAML version".to_string()),
        author: None,
        version: Some("1.0".to_string()),
    };

    let yaml_template = Template::new(yaml_metadata);
    let yaml_content = serde_yaml::to_string(&yaml_template).expect("Failed to serialize");
    fs::write(&yaml_path, yaml_content).expect("Failed to write YAML");

    // Create bincode template with different metadata
    let bin_metadata = TemplateMetadata {
        name: Some("Bincode Version".to_string()),
        description: Some("This is the bincode version".to_string()),
        author: None,
        version: Some("2.0".to_string()),
    };

    let bin_template = Template::new(bin_metadata);
    let compiler = TemplateCompiler::new();

    // Write bincode directly
    let yaml_temp = serde_yaml::to_string(&bin_template).expect("Failed to serialize");
    let temp_yaml = temp_dir.path().join("temp.yaml");
    fs::write(&temp_yaml, yaml_temp).expect("Failed to write temp YAML");
    compiler.compile(&temp_yaml, &bin_path).expect("Failed to compile");

    // Load using TemplateLoader (should prefer .bin)
    let mut world = World::new();
    let mut loader = TemplateLoader::new();

    // Pass the YAML path, but it should load the .bin file
    let instance = loader.load(&mut world, &yaml_path).expect("Failed to load template");

    // Verify it loaded the bincode version
    assert_eq!(instance.name, "Bincode Version");
}

#[test]
fn test_auto_detection_fallback_to_yaml() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let yaml_path = temp_dir.path().join("yaml_only.yaml");

    // Create only YAML template (no .bin)
    let metadata = TemplateMetadata {
        name: Some("YAML Only".to_string()),
        description: Some("Only YAML exists".to_string()),
        author: None,
        version: Some("1.0".to_string()),
    };

    let template = Template::new(metadata);
    let yaml_content = serde_yaml::to_string(&template).expect("Failed to serialize");
    fs::write(&yaml_path, yaml_content).expect("Failed to write YAML");

    // Load using TemplateLoader (should fall back to YAML)
    let mut world = World::new();
    let mut loader = TemplateLoader::new();

    let instance = loader.load(&mut world, &yaml_path).expect("Failed to load template");

    assert_eq!(instance.name, "YAML Only");
}

#[test]
fn test_checksum_validation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let yaml_path = temp_dir.path().join("checksum.yaml");
    let bin_path = temp_dir.path().join("checksum.bin");

    // Create and compile template
    let metadata = TemplateMetadata {
        name: Some("Checksum Test".to_string()),
        description: None,
        author: None,
        version: Some("1.0".to_string()),
    };

    let template = Template::new(metadata);
    let yaml_content = serde_yaml::to_string(&template).expect("Failed to serialize");
    fs::write(&yaml_path, yaml_content).expect("Failed to write YAML");

    let compiler = TemplateCompiler::new();
    compiler.compile(&yaml_path, &bin_path).expect("Compilation failed");

    // Load should succeed with valid checksum
    let result = compiler.load_compiled(&bin_path);
    assert!(result.is_ok(), "Loading valid template should succeed");

    // Corrupt the bincode file
    let mut bincode_data = fs::read(&bin_path).expect("Failed to read bincode");
    // Flip some bits in the middle of the data
    if bincode_data.len() > 100 {
        bincode_data[50] ^= 0xFF;
        bincode_data[75] ^= 0xFF;
        fs::write(&bin_path, bincode_data).expect("Failed to write corrupted data");

        // Load should fail with corrupted data
        let result = compiler.load_compiled(&bin_path);
        assert!(result.is_err(), "Loading corrupted template should fail checksum validation");
    }
}

#[test]
fn test_compile_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create multiple YAML templates
    for i in 0..5 {
        let yaml_path = temp_dir.path().join(format!("template_{}.yaml", i));
        let metadata = TemplateMetadata {
            name: Some(format!("Template {}", i)),
            description: None,
            author: None,
            version: Some("1.0".to_string()),
        };
        let template = Template::new(metadata);
        let yaml_content = serde_yaml::to_string(&template).expect("Failed to serialize");
        fs::write(yaml_path, yaml_content).expect("Failed to write YAML");
    }

    // Compile directory
    let compiler = TemplateCompiler::new();
    let count = compiler
        .compile_directory(temp_dir.path())
        .expect("Failed to compile directory");

    assert_eq!(count, 5, "Should compile all 5 templates");

    // Verify all .bin files exist
    for i in 0..5 {
        let bin_path = temp_dir.path().join(format!("template_{}.bin", i));
        assert!(bin_path.exists(), "Bincode file {} should exist", bin_path.display());
    }
}

#[test]
fn test_bincode_smaller_than_yaml() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let yaml_path = temp_dir.path().join("size_test.yaml");
    let bin_path = temp_dir.path().join("size_test.bin");

    // Create a moderately complex template
    let metadata = TemplateMetadata {
        name: Some("Size Comparison Test".to_string()),
        description: Some(
            "This template is used to verify that bincode is smaller than YAML".to_string(),
        ),
        author: Some("Test Suite".to_string()),
        version: Some("1.0".to_string()),
    };

    let mut template = Template::new(metadata);

    // Add multiple entities
    for i in 0..20 {
        let entity = EntityDefinition::new_inline(
            FxHashMap::default(),
            vec![format!("tag_{}", i), "test".to_string(), "entity".to_string()],
        );
        template.add_entity(format!("Entity_{}", i), entity);
    }

    let yaml_content = serde_yaml::to_string(&template).expect("Failed to serialize");
    fs::write(&yaml_path, &yaml_content).expect("Failed to write YAML");

    // Compile
    let compiler = TemplateCompiler::new();
    compiler.compile(&yaml_path, &bin_path).expect("Compilation failed");

    // Compare sizes
    let yaml_size = fs::metadata(&yaml_path).expect("Failed to get YAML metadata").len();
    let bin_size = fs::metadata(&bin_path).expect("Failed to get bincode metadata").len();

    assert!(
        bin_size < yaml_size,
        "Bincode should be smaller than YAML (bincode: {}, YAML: {})",
        bin_size,
        yaml_size
    );

    let compression_ratio = (bin_size as f64 / yaml_size as f64) * 100.0;
    println!(
        "Compression ratio: {:.1}% (bincode: {} bytes, YAML: {} bytes)",
        compression_ratio, bin_size, yaml_size
    );

    // Bincode should be 50-80% of YAML size (ideally)
    assert!(
        compression_ratio < 90.0,
        "Bincode should be significantly smaller (got {:.1}%)",
        compression_ratio
    );
}

#[test]
fn test_loader_cache_with_bincode() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let yaml_path = temp_dir.path().join("cache_test.yaml");
    let bin_path = temp_dir.path().join("cache_test.bin");

    // Create and compile template
    let metadata = TemplateMetadata {
        name: Some("Cache Test".to_string()),
        description: None,
        author: None,
        version: Some("1.0".to_string()),
    };

    let template = Template::new(metadata);
    let yaml_content = serde_yaml::to_string(&template).expect("Failed to serialize");
    fs::write(&yaml_path, yaml_content).expect("Failed to write YAML");

    let compiler = TemplateCompiler::new();
    compiler.compile(&yaml_path, &bin_path).expect("Compilation failed");

    // Load template multiple times
    let mut world = World::new();
    let mut loader = TemplateLoader::new();

    assert_eq!(loader.cache_size(), 0, "Cache should start empty");

    loader.load(&mut world, &yaml_path).expect("First load failed");
    assert_eq!(loader.cache_size(), 1, "Cache should have 1 entry after first load");

    loader.load(&mut world, &yaml_path).expect("Second load failed");
    assert_eq!(
        loader.cache_size(),
        1,
        "Cache should still have 1 entry after second load (cache hit)"
    );

    loader.load(&mut world, &yaml_path).expect("Third load failed");
    assert_eq!(
        loader.cache_size(),
        1,
        "Cache should still have 1 entry after third load (cache hit)"
    );
}
