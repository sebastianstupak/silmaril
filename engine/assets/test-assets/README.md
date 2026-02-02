# Test Assets

This directory contains test assets used by the asset system tests and examples.

## Materials

- `brick.yaml` - Brick material with albedo and normal maps
- `metal.yaml` - Metallic material with roughness maps

## Audio

Audio files can be generated programmatically in tests using `hound` crate for WAV files.

## Usage in Tests

```rust
use engine_assets::MaterialData;
use std::fs;

let yaml_content = fs::read_to_string("test-assets/brick.yaml").unwrap();
let material = MaterialData::from_yaml(&yaml_content).unwrap();
```
