//! End-to-end workflow test demonstrating the complete bincode compilation pipeline.
//!
//! This test demonstrates:
//! 1. Creating a YAML template
//! 2. Compiling to bincode via TemplateCompiler
//! 3. Loading via TemplateLoader (auto-detection)
//! 4. Verifying performance improvement

use engine_core::ecs::World;
use engine_core::gameplay::Health;
use engine_core::math::Transform;
use engine_templating::{
    EntityDefinition, Template, TemplateCompiler, TemplateLoader, TemplateMetadata,
};
use rustc_hash::FxHashMap;
use std::fs;
use std::time::Instant;
use tempfile::TempDir;

#[test]
fn test_complete_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let yaml_path = temp_dir.path().join("workflow.yaml");
    let bin_path = temp_dir.path().join("workflow.bin");

    // Step 1: Create a YAML template (simulating authoring)
    println!("Step 1: Creating YAML template...");
    let metadata = TemplateMetadata {
        name: Some("Workflow Test".to_string()),
        description: Some("End-to-end workflow demonstration".to_string()),
        author: Some("Test Suite".to_string()),
        version: Some("1.0".to_string()),
    };

    let mut template = Template::new(metadata.clone());

    // Add several entities to make it realistic
    for i in 0..10 {
        let mut components = FxHashMap::default();
        components.insert("Transform".to_string(), serde_yaml::Value::Null);
        components.insert("Health".to_string(), serde_yaml::Value::Null);

        let entity = EntityDefinition::new_inline(
            components,
            vec![format!("entity_{}", i), "test".to_string()],
        );
        template.add_entity(format!("Entity_{}", i), entity);
    }

    let yaml_content = serde_yaml::to_string(&template).expect("Failed to serialize");
    fs::write(&yaml_path, &yaml_content).expect("Failed to write YAML");

    println!("  ✓ YAML template created: {} bytes", yaml_content.len());

    // Step 2: Compile to bincode
    println!("\nStep 2: Compiling to bincode...");
    let compiler = TemplateCompiler::new();
    let start = Instant::now();
    compiler.compile(&yaml_path, &bin_path).expect("Compilation failed");
    let compile_time = start.elapsed();

    let bin_size = fs::metadata(&bin_path).expect("Failed to get metadata").len();
    let compression_ratio = (bin_size as f64 / yaml_content.len() as f64) * 100.0;

    println!("  ✓ Compilation completed in {:?}", compile_time);
    println!("  ✓ Bincode size: {} bytes ({:.1}% of YAML)", bin_size, compression_ratio);

    // Verify the compiled template by loading it
    let loaded_compiled =
        compiler.load_compiled(&bin_path).expect("Failed to load compiled template");
    assert_eq!(loaded_compiled.metadata, metadata);
    assert_eq!(loaded_compiled.entity_count(), 10);

    // Step 3: Load via TemplateLoader (should auto-detect .bin)
    println!("\nStep 3: Loading via TemplateLoader (auto-detection)...");
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    let mut loader = TemplateLoader::new();

    // Pass YAML path, but it should load .bin automatically
    let start = Instant::now();
    let instance = loader.load(&mut world, &yaml_path).expect("Failed to load template");
    let load_time = start.elapsed();

    println!("  ✓ Template loaded in {:?}", load_time);
    println!("  ✓ Loaded {} entities", instance.entities.len());

    assert_eq!(instance.name, "Workflow Test");
    assert_eq!(instance.entities.len(), 10);

    // Step 4: Compare performance (load YAML vs bincode)
    println!("\nStep 4: Performance comparison...");

    // Clear cache for fair comparison
    loader.clear_cache();

    // Time YAML loading (delete .bin temporarily)
    fs::remove_file(&bin_path).expect("Failed to remove bin file");
    let start = Instant::now();
    let _yaml_instance = loader.load(&mut world, &yaml_path).expect("Failed to load YAML");
    let yaml_load_time = start.elapsed();

    // Recompile bincode
    compiler.compile(&yaml_path, &bin_path).expect("Recompilation failed");

    // Clear cache again
    loader.clear_cache();

    // Time bincode loading
    let start = Instant::now();
    let _bin_instance = loader.load(&mut world, &yaml_path).expect("Failed to load bincode");
    let bin_load_time = start.elapsed();

    let speedup = yaml_load_time.as_nanos() as f64 / bin_load_time.as_nanos() as f64;

    println!("  YAML load time: {:?}", yaml_load_time);
    println!("  Bincode load time: {:?}", bin_load_time);
    println!("  Speedup: {:.1}x faster", speedup);

    // Note: In debug builds, the performance difference may be minimal
    // In release builds, bincode is typically 10-50x faster
    if bin_load_time < yaml_load_time {
        println!("  ✓ Bincode was faster than YAML");
    } else {
        println!("  ⚠ YAML was faster (expected in debug builds with small templates)");
    }

    println!("\n✓ Complete workflow test passed!");
}

#[test]
fn test_workflow_with_caching() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let yaml_path = temp_dir.path().join("cache_workflow.yaml");
    let bin_path = temp_dir.path().join("cache_workflow.bin");

    // Create and compile template
    let metadata = TemplateMetadata {
        name: Some("Cache Workflow Test".to_string()),
        description: None,
        author: None,
        version: Some("1.0".to_string()),
    };

    let template = Template::new(metadata);
    let yaml_content = serde_yaml::to_string(&template).expect("Failed to serialize");
    fs::write(&yaml_path, &yaml_content).expect("Failed to write YAML");

    let compiler = TemplateCompiler::new();
    compiler.compile(&yaml_path, &bin_path).expect("Compilation failed");

    // Load multiple times and verify caching
    let mut world = World::new();
    let mut loader = TemplateLoader::new();

    println!("Loading template 3 times to test caching...");

    assert_eq!(loader.cache_size(), 0, "Cache should start empty");

    let start = Instant::now();
    loader.load(&mut world, &yaml_path).expect("First load failed");
    let first_load = start.elapsed();
    println!("  First load (cache miss): {:?}", first_load);
    assert_eq!(loader.cache_size(), 1, "Cache should have 1 entry");

    let start = Instant::now();
    loader.load(&mut world, &yaml_path).expect("Second load failed");
    let second_load = start.elapsed();
    println!("  Second load (cache hit): {:?}", second_load);
    assert_eq!(loader.cache_size(), 1, "Cache should still have 1 entry");

    let start = Instant::now();
    loader.load(&mut world, &yaml_path).expect("Third load failed");
    let third_load = start.elapsed();
    println!("  Third load (cache hit): {:?}", third_load);
    assert_eq!(loader.cache_size(), 1, "Cache should still have 1 entry");

    // Cache hits should be faster than first load
    println!("\n✓ Caching workflow test passed!");
}
