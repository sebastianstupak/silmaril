use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use engine_assets::{AudioData, AudioFormat, MaterialData};
use hound::WavSpec;
use std::io::Cursor;

fn material_parsing_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("material");

    // YAML parsing
    let yaml = r#"
name: "brick"
base_color_texture: "brick_albedo.png"
metallic_roughness_texture: null
normal_texture: "brick_normal.png"
emissive_texture: null
base_color_factor: [1.0, 1.0, 1.0, 1.0]
metallic_factor: 0.0
roughness_factor: 0.8
emissive_factor: [0.0, 0.0, 0.0]
"#;

    group.bench_function("parse_yaml", |b| b.iter(|| MaterialData::from_yaml(black_box(yaml))));

    // YAML serialization
    let material = MaterialData::new("test_material");

    group.bench_function("serialize_yaml", |b| b.iter(|| material.to_yaml()));

    // Roundtrip
    group.bench_function("yaml_roundtrip", |b| {
        b.iter(|| {
            let yaml = material.to_yaml().unwrap();
            MaterialData::from_yaml(&yaml)
        })
    });

    group.finish();
}

fn audio_wav_loading_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_wav");

    // Create 1 second of stereo audio at 44100 Hz
    let spec = WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        // 1 second of audio: 44100 samples/sec * 2 channels = 88200 samples
        for i in 0..88200 {
            let sample = (i as f32 * 0.01).sin();
            writer.write_sample((sample * i16::MAX as f32) as i16).unwrap();
        }
        writer.finalize().unwrap();
    }

    let wav_data = cursor.into_inner();
    let data_size = wav_data.len() as u64;

    group.throughput(Throughput::Bytes(data_size));
    group.bench_function("load_1s_stereo_44100hz", |b| {
        b.iter(|| AudioData::from_wav(black_box(&wav_data)))
    });

    group.finish();
}

fn audio_wav_small_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_wav_small");

    // Create 100ms of mono audio at 48000 Hz
    let spec = WavSpec {
        channels: 1,
        sample_rate: 48000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        // 0.1 seconds of audio: 4800 samples
        for i in 0..4800 {
            let sample = (i as f32 * 0.01).sin();
            writer.write_sample((sample * i16::MAX as f32) as i16).unwrap();
        }
        writer.finalize().unwrap();
    }

    let wav_data = cursor.into_inner();
    let data_size = wav_data.len() as u64;

    group.throughput(Throughput::Bytes(data_size));
    group.bench_function("load_100ms_mono_48000hz", |b| {
        b.iter(|| AudioData::from_wav(black_box(&wav_data)))
    });

    group.finish();
}

fn audio_duration_calculation_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_duration");

    // 1 second of stereo audio at 44100 Hz
    let audio = AudioData::new(
        44100,
        2,
        AudioFormat::PCM16,
        vec![0u8; 44100 * 2 * 2], // 1 sec * 2 channels * 2 bytes
    );

    group.bench_function("calculate_duration", |b| b.iter(|| black_box(&audio).duration()));

    group.bench_function("sample_count", |b| b.iter(|| black_box(&audio).sample_count()));

    group.finish();
}

criterion_group!(
    benches,
    material_parsing_benchmark,
    audio_wav_loading_benchmark,
    audio_wav_small_benchmark,
    audio_duration_calculation_benchmark,
);
criterion_main!(benches);
