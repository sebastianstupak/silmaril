//! Integration tests for material and audio assets

use engine_assets::{AudioData, AudioError, MaterialData, MaterialError};
use hound::WavSpec;
use std::io::Cursor;

#[test]
fn test_material_yaml_load_and_save() {
    let yaml = r#"
name: "test_material"
base_color_texture: "color.png"
metallic_roughness_texture: null
normal_texture: "normal.png"
emissive_texture: null
base_color_factor: [0.5, 0.6, 0.7, 1.0]
metallic_factor: 0.2
roughness_factor: 0.8
emissive_factor: [0.0, 0.0, 0.0]
"#;

    let material = MaterialData::from_yaml(yaml).expect("Failed to load material");
    assert_eq!(material.name, "test_material");
    assert_eq!(material.base_color_texture, Some("color.png".to_string()));
    assert_eq!(material.normal_texture, Some("normal.png".to_string()));
    assert_eq!(material.metallic_factor, 0.2);
    assert_eq!(material.roughness_factor, 0.8);

    // Test roundtrip
    let yaml_out = material.to_yaml().expect("Failed to serialize");
    let material2 = MaterialData::from_yaml(&yaml_out).expect("Failed to deserialize");
    assert_eq!(material, material2);
}

#[test]
fn test_wav_audio_load() {
    // Create a test WAV file in memory
    let spec = WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        for i in 0..1000 {
            writer.write_sample(i as i16).unwrap();
        }
        writer.finalize().unwrap();
    }

    let wav_data = cursor.into_inner();
    let audio = AudioData::from_wav(&wav_data).expect("Failed to load WAV");

    assert_eq!(audio.sample_rate, 44100);
    assert_eq!(audio.channels, 2);
    assert_eq!(audio.sample_count(), 500); // 1000 samples / 2 channels
}

#[test]
fn test_audio_duration_calculation() {
    // 1 second of stereo audio at 44100 Hz
    let audio =
        AudioData::new(44100, 2, engine_assets::AudioFormat::PCM16, vec![0u8; 44100 * 2 * 2]);

    let duration = audio.duration();
    assert!((duration - 1.0).abs() < 0.01, "Duration should be approximately 1 second");
}

#[test]
fn test_invalid_wav_data() {
    let invalid_data = b"not a wav file";
    let result = AudioData::from_wav(invalid_data);
    assert!(result.is_err());
    assert!(matches!(result, Err(AudioError::InvalidWavFormat { .. })));
}

#[test]
fn test_invalid_ogg_data() {
    let invalid_data = b"not an ogg file";
    let result = AudioData::from_ogg(invalid_data);
    assert!(result.is_err());
    assert!(matches!(result, Err(AudioError::InvalidOggFormat { .. })));
}

#[test]
fn test_material_default() {
    let material = MaterialData::default();
    assert_eq!(material.name, "default");
    assert_eq!(material.base_color_factor, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(material.metallic_factor, 0.0);
    assert_eq!(material.roughness_factor, 0.5);
}

#[test]
fn test_material_new() {
    let material = MaterialData::new("my_material");
    assert_eq!(material.name, "my_material");
    assert!(material.base_color_texture.is_none());
    assert!(material.normal_texture.is_none());
}

#[test]
fn test_invalid_yaml_material() {
    let invalid_yaml = "this is { not [ valid yaml";
    let result = MaterialData::from_yaml(invalid_yaml);
    assert!(result.is_err());
    assert!(matches!(result, Err(MaterialError::InvalidYamlFormat { .. })));
}
