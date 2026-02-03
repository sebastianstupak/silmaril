//! Generate test audio assets for singleplayer example
//!
//! Creates simple WAV files for testing audio functionality:
//! - footstep.wav: 440Hz sine wave (0.1s)
//! - ambient.wav: 220Hz sine wave (2s, looping)
//! - explosion.wav: White noise burst (0.3s)
//! - music.wav: Multi-tone melody (5s, looping)

use hound::{WavSpec, WavWriter};
use std::f32::consts::PI;
use std::path::Path;

const SAMPLE_RATE: u32 = 44100;

fn main() {
    println!("Generating test audio assets...");

    let assets_dir = Path::new("assets/audio");
    std::fs::create_dir_all(assets_dir).expect("Failed to create assets directory");

    generate_footstep(assets_dir);
    generate_ambient(assets_dir);
    generate_explosion(assets_dir);
    generate_music(assets_dir);

    println!("✓ Audio assets generated successfully");
}

/// Generate footstep sound (440Hz sine wave, 0.1s)
fn generate_footstep(dir: &Path) {
    let path = dir.join("footstep.wav");
    println!("  Generating footstep.wav...");

    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(&path, spec).expect("Failed to create footstep.wav");

    let duration = 0.1; // 100ms
    let frequency = 440.0;

    for i in 0..(SAMPLE_RATE as f32 * duration) as usize {
        let t = i as f32 / SAMPLE_RATE as f32;
        let sample = (t * frequency * 2.0 * PI).sin();

        // Apply envelope for natural sound
        let envelope = if t < 0.01 {
            t / 0.01 // Attack
        } else if t > duration - 0.02 {
            (duration - t) / 0.02 // Release
        } else {
            1.0 // Sustain
        };

        let amplitude = (sample * envelope * i16::MAX as f32) as i16;
        writer.write_sample(amplitude).expect("Failed to write sample");
    }

    writer.finalize().expect("Failed to finalize footstep.wav");
}

/// Generate ambient sound (220Hz sine wave, 2s, looping)
fn generate_ambient(dir: &Path) {
    let path = dir.join("ambient.wav");
    println!("  Generating ambient.wav...");

    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(&path, spec).expect("Failed to create ambient.wav");

    let duration = 2.0; // 2 seconds
    let frequency = 220.0;

    for i in 0..(SAMPLE_RATE as f32 * duration) as usize {
        let t = i as f32 / SAMPLE_RATE as f32;

        // Mix two frequencies for richer sound
        let sample =
            (t * frequency * 2.0 * PI).sin() * 0.5 + (t * frequency * 1.5 * 2.0 * PI).sin() * 0.3;

        let amplitude = (sample * 0.7 * i16::MAX as f32) as i16;
        writer.write_sample(amplitude).expect("Failed to write sample");
    }

    writer.finalize().expect("Failed to finalize ambient.wav");
}

/// Generate explosion sound (white noise burst, 0.3s)
fn generate_explosion(dir: &Path) {
    let path = dir.join("explosion.wav");
    println!("  Generating explosion.wav...");

    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(&path, spec).expect("Failed to create explosion.wav");

    let duration = 0.3; // 300ms

    for i in 0..(SAMPLE_RATE as f32 * duration) as usize {
        let t = i as f32 / SAMPLE_RATE as f32;

        // White noise (using deterministic random)
        let noise = ((i * 1103515245 + 12345) as i32 >> 16) as f32 / 32768.0;

        // Exponential decay envelope
        let envelope = (-t * 8.0).exp();

        let amplitude = (noise * envelope * i16::MAX as f32) as i16;
        writer.write_sample(amplitude).expect("Failed to write sample");
    }

    writer.finalize().expect("Failed to finalize explosion.wav");
}

/// Generate music (multi-tone melody, 5s, looping)
fn generate_music(dir: &Path) {
    let path = dir.join("music.wav");
    println!("  Generating music.wav...");

    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(&path, spec).expect("Failed to create music.wav");

    let duration = 5.0; // 5 seconds

    // Simple melody: C4, E4, G4, C5 (repeated)
    let notes = [261.63, 329.63, 392.00, 523.25];
    let note_duration = duration / notes.len() as f32;

    for i in 0..(SAMPLE_RATE as f32 * duration) as usize {
        let t = i as f32 / SAMPLE_RATE as f32;
        let note_index = (t / note_duration) as usize % notes.len();
        let note_t = t % note_duration;

        let frequency = notes[note_index];
        let sample = (note_t * frequency * 2.0 * PI).sin();

        // Apply envelope for each note
        let envelope = if note_t < 0.05 {
            note_t / 0.05 // Attack
        } else if note_t > note_duration - 0.1 {
            (note_duration - note_t) / 0.1 // Release
        } else {
            1.0 // Sustain
        };

        let amplitude = (sample * envelope * 0.5 * i16::MAX as f32) as i16;
        writer.write_sample(amplitude).expect("Failed to write sample");
    }

    writer.finalize().expect("Failed to finalize music.wav");
}
