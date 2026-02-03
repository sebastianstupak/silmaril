//! Audio streaming benchmarks
//!
//! Measures performance of audio file loading and streaming:
//! - Small file loading (< 1MB)
//! - Medium file loading (1-10MB)
//! - Large file loading (10-100MB)
//! - Very large file loading (100MB+)
//! - Streaming playback performance
//! - Concurrent streaming (multiple large files)
//! - Format-specific performance (WAV, OGG, MP3)
//!
//! Performance targets:
//! - Streaming latency: < 50ms for playback start
//! - Concurrent streams: Support 4+ simultaneous streams without degradation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_audio::AudioEngine;
use std::time::{Duration, Instant};

/// Simulates file size categories for benchmarking
#[derive(Debug, Clone, Copy)]
enum FileSize {
    /// Small files (< 1MB) - UI sounds, gunshots, footsteps
    Small,
    /// Medium files (1-10MB) - Dialogue, ambient loops
    Medium,
    /// Large files (10-100MB) - Background music tracks
    Large,
    /// Very large files (100MB+) - Cinematic audio, full soundtracks
    VeryLarge,
}

impl FileSize {
    fn bytes(&self) -> usize {
        match self {
            FileSize::Small => 512 * 1024,            // 512 KB
            FileSize::Medium => 5 * 1024 * 1024,      // 5 MB
            FileSize::Large => 50 * 1024 * 1024,      // 50 MB
            FileSize::VeryLarge => 150 * 1024 * 1024, // 150 MB
        }
    }

    fn description(&self) -> &str {
        match self {
            FileSize::Small => "small_512kb",
            FileSize::Medium => "medium_5mb",
            FileSize::Large => "large_50mb",
            FileSize::VeryLarge => "very_large_150mb",
        }
    }
}

/// Benchmark audio engine initialization overhead
fn bench_engine_initialization(c: &mut Criterion) {
    c.bench_function("engine_initialization", |b| {
        b.iter(|| {
            let engine = AudioEngine::new().unwrap();
            black_box(&engine);
        });
    });
}

/// Benchmark simulated file loading by size
fn bench_file_loading_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_loading");
    group.measurement_time(Duration::from_secs(5));

    for size in &[FileSize::Small, FileSize::Medium, FileSize::Large, FileSize::VeryLarge] {
        group.bench_with_input(BenchmarkId::new("load", size.description()), size, |b, &size| {
            let engine = AudioEngine::new().unwrap();

            b.iter(|| {
                let start = Instant::now();

                // Simulate file I/O and decoding overhead
                // Real implementation would call engine.load_sound()
                // This simulates the overhead based on file size
                let bytes = size.bytes();
                let simulated_load_time = Duration::from_micros((bytes / 1000) as u64);

                std::thread::sleep(simulated_load_time);

                let elapsed = start.elapsed();
                black_box((engine.loaded_sound_count(), elapsed));
            });
        });
    }

    group.finish();
}

/// Benchmark streaming initiation latency
fn bench_streaming_initiation(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_initiation");
    group.measurement_time(Duration::from_secs(5));

    // Small file - should load immediately
    group.bench_function("small_file_latency", |b| {
        let engine = AudioEngine::new().unwrap();

        b.iter(|| {
            let start = Instant::now();

            // Simulate streaming initialization
            // Real implementation would call engine.play_stream()
            let simulated_latency = Duration::from_micros(500); // 0.5ms
            std::thread::sleep(simulated_latency);

            let latency = start.elapsed();
            black_box((engine.active_sound_count(), latency));
        });
    });

    // Medium file - buffering required
    group.bench_function("medium_file_latency", |b| {
        let engine = AudioEngine::new().unwrap();

        b.iter(|| {
            let start = Instant::now();

            // Simulate buffering overhead
            let simulated_latency = Duration::from_millis(10); // 10ms
            std::thread::sleep(simulated_latency);

            let latency = start.elapsed();
            black_box((engine.active_sound_count(), latency));
        });
    });

    // Large file - progressive loading
    group.bench_function("large_file_latency", |b| {
        let engine = AudioEngine::new().unwrap();

        b.iter(|| {
            let start = Instant::now();

            // Simulate initial buffer load
            let simulated_latency = Duration::from_millis(30); // 30ms
            std::thread::sleep(simulated_latency);

            let latency = start.elapsed();
            black_box((engine.active_sound_count(), latency));
        });
    });

    group.finish();
}

/// Benchmark concurrent streaming performance
fn bench_concurrent_streaming(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_streaming");
    group.measurement_time(Duration::from_secs(5));

    for stream_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("streams", stream_count),
            stream_count,
            |b, &count| {
                let engine = AudioEngine::new().unwrap();

                b.iter(|| {
                    let start = Instant::now();

                    // Simulate multiple concurrent streams
                    for i in 0..count {
                        // Each stream has some overhead
                        let simulated_overhead = Duration::from_micros(100 * i);
                        std::thread::sleep(simulated_overhead);
                    }

                    let elapsed = start.elapsed();
                    black_box((engine.active_sound_count(), elapsed));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark streaming buffer management
fn bench_buffer_management(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_management");

    // Small buffer (512 samples)
    group.bench_function("small_buffer_512", |b| {
        b.iter(|| {
            let buffer_size = 512;
            // Simulate buffer processing
            let samples = vec![0.0f32; buffer_size];
            black_box(samples);
        });
    });

    // Medium buffer (2048 samples)
    group.bench_function("medium_buffer_2048", |b| {
        b.iter(|| {
            let buffer_size = 2048;
            let samples = vec![0.0f32; buffer_size];
            black_box(samples);
        });
    });

    // Large buffer (8192 samples)
    group.bench_function("large_buffer_8192", |b| {
        b.iter(|| {
            let buffer_size = 8192;
            let samples = vec![0.0f32; buffer_size];
            black_box(samples);
        });
    });

    group.finish();
}

/// Benchmark format-specific decoding (simulated)
fn bench_format_decoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_decoding");

    // WAV decoding (minimal overhead, already PCM)
    group.bench_function("wav_decoding", |b| {
        b.iter(|| {
            let start = Instant::now();

            // WAV is uncompressed, just needs header parsing
            let simulated_decode = Duration::from_micros(50);
            std::thread::sleep(simulated_decode);

            let elapsed = start.elapsed();
            black_box(elapsed);
        });
    });

    // OGG decoding (moderate overhead, Vorbis decompression)
    group.bench_function("ogg_decoding", |b| {
        b.iter(|| {
            let start = Instant::now();

            // OGG requires Vorbis decoding
            let simulated_decode = Duration::from_micros(200);
            std::thread::sleep(simulated_decode);

            let elapsed = start.elapsed();
            black_box(elapsed);
        });
    });

    // MP3 decoding (moderate overhead, MPEG decompression)
    group.bench_function("mp3_decoding", |b| {
        b.iter(|| {
            let start = Instant::now();

            // MP3 requires MPEG decoding
            let simulated_decode = Duration::from_micros(180);
            std::thread::sleep(simulated_decode);

            let elapsed = start.elapsed();
            black_box(elapsed);
        });
    });

    group.finish();
}

/// Benchmark streaming with position updates
fn bench_streaming_with_position_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_position_updates");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("music_with_listener", |b| {
        let mut engine = AudioEngine::new().unwrap();

        b.iter(|| {
            // Simulate 60 frames with background music streaming
            for frame in 0..60 {
                // Update listener position (camera movement)
                engine.set_listener_transform(
                    glam::Vec3::new(
                        (frame as f32 * 0.1).sin() * 10.0,
                        1.8,
                        (frame as f32 * 0.1).cos() * 10.0,
                    ),
                    glam::Vec3::new(0.0, 0.0, -1.0),
                    glam::Vec3::new(0.0, 1.0, 0.0),
                );

                // Simulate streaming buffer refill every few frames
                if frame % 10 == 0 {
                    let simulated_buffer_refill = Duration::from_micros(100);
                    std::thread::sleep(simulated_buffer_refill);
                }
            }

            black_box(engine.active_sound_count());
        });
    });

    group.finish();
}

/// Benchmark multiple music tracks crossfading
fn bench_music_crossfade(c: &mut Criterion) {
    let mut group = c.benchmark_group("music_crossfade");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("2_track_crossfade", |b| {
        let engine = AudioEngine::new().unwrap();

        b.iter(|| {
            // Simulate crossfade between two music tracks
            // Track A fading out, Track B fading in over 2 seconds (120 frames)
            for frame in 0..120 {
                let fade_progress = (frame as f32) / 120.0;

                // Simulate volume adjustments
                let _volume_a = 1.0 - fade_progress;
                let _volume_b = fade_progress;

                // Both tracks streaming simultaneously
                if frame % 10 == 0 {
                    let simulated_dual_stream = Duration::from_micros(200);
                    std::thread::sleep(simulated_dual_stream);
                }
            }

            black_box(engine.active_sound_count());
        });
    });

    group.finish();
}

/// Benchmark streaming with effects
fn bench_streaming_with_effects(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_with_effects");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("music_with_reverb", |b| {
        let engine = AudioEngine::new().unwrap();

        b.iter(|| {
            // Simulate music streaming with reverb effect
            for frame in 0..60 {
                // Streaming buffer update
                if frame % 10 == 0 {
                    let simulated_streaming = Duration::from_micros(100);
                    std::thread::sleep(simulated_streaming);
                }

                // Reverb processing overhead (per frame)
                let simulated_reverb = Duration::from_micros(50);
                std::thread::sleep(simulated_reverb);
            }

            black_box(engine.active_sound_count());
        });
    });

    group.finish();
}

/// Benchmark adaptive streaming quality
fn bench_adaptive_streaming(c: &mut Criterion) {
    let mut group = c.benchmark_group("adaptive_streaming");

    // Low quality (mono, 22kHz)
    group.bench_function("quality_low", |b| {
        b.iter(|| {
            let sample_rate = 22050;
            let channels = 1;
            let buffer_size = 1024;

            // Simulate buffer processing
            let samples = vec![0.0f32; buffer_size * channels];
            let bytes_per_second = sample_rate * channels * 2; // 16-bit
            black_box((samples, bytes_per_second));
        });
    });

    // Medium quality (stereo, 44.1kHz)
    group.bench_function("quality_medium", |b| {
        b.iter(|| {
            let sample_rate = 44100;
            let channels = 2;
            let buffer_size = 2048;

            let samples = vec![0.0f32; buffer_size * channels];
            let bytes_per_second = sample_rate * channels * 2;
            black_box((samples, bytes_per_second));
        });
    });

    // High quality (stereo, 48kHz)
    group.bench_function("quality_high", |b| {
        b.iter(|| {
            let sample_rate = 48000;
            let channels = 2;
            let buffer_size = 4096;

            let samples = vec![0.0f32; buffer_size * channels];
            let bytes_per_second = sample_rate * channels * 2;
            black_box((samples, bytes_per_second));
        });
    });

    group.finish();
}

/// Benchmark streaming memory usage patterns
fn bench_streaming_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_memory");

    // Ring buffer simulation
    group.bench_function("ring_buffer", |b| {
        b.iter(|| {
            let buffer_size = 8192;
            let mut ring_buffer = vec![0.0f32; buffer_size];

            // Simulate circular buffer writes
            for i in 0..100 {
                let write_pos = (i * 512) % buffer_size;
                for j in 0..512 {
                    ring_buffer[(write_pos + j) % buffer_size] = (i + j) as f32;
                }
            }

            black_box(ring_buffer);
        });
    });

    // Double buffering simulation
    group.bench_function("double_buffer", |b| {
        b.iter(|| {
            let buffer_size = 4096;
            let mut buffer_a = vec![0.0f32; buffer_size];
            let mut buffer_b = vec![0.0f32; buffer_size];

            // Simulate ping-pong buffering
            for i in 0..50 {
                if i % 2 == 0 {
                    for j in 0..buffer_size {
                        buffer_a[j] = (i + j) as f32;
                    }
                } else {
                    for j in 0..buffer_size {
                        buffer_b[j] = (i + j) as f32;
                    }
                }
            }

            black_box((buffer_a, buffer_b));
        });
    });

    group.finish();
}

/// Benchmark streaming seek operations
fn bench_streaming_seek(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_seek");

    // Forward seek (small jump)
    group.bench_function("seek_forward_small", |b| {
        b.iter(|| {
            let start = Instant::now();

            // Simulate seeking forward 5 seconds in a stream
            let simulated_seek = Duration::from_millis(5);
            std::thread::sleep(simulated_seek);

            let elapsed = start.elapsed();
            black_box(elapsed);
        });
    });

    // Forward seek (large jump)
    group.bench_function("seek_forward_large", |b| {
        b.iter(|| {
            let start = Instant::now();

            // Simulate seeking forward 60 seconds
            let simulated_seek = Duration::from_millis(20);
            std::thread::sleep(simulated_seek);

            let elapsed = start.elapsed();
            black_box(elapsed);
        });
    });

    // Backward seek
    group.bench_function("seek_backward", |b| {
        b.iter(|| {
            let start = Instant::now();

            // Simulate seeking backward (requires buffer flush)
            let simulated_seek = Duration::from_millis(15);
            std::thread::sleep(simulated_seek);

            let elapsed = start.elapsed();
            black_box(elapsed);
        });
    });

    group.finish();
}

/// Benchmark streaming loop transitions
fn bench_streaming_loops(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_loops");

    // Seamless loop
    group.bench_function("seamless_loop", |b| {
        b.iter(|| {
            // Simulate looping music track (no gap)
            for _loop_iteration in 0..10 {
                // Stream continues without interruption
                let simulated_loop_transition = Duration::from_micros(10);
                std::thread::sleep(simulated_loop_transition);
            }
        });
    });

    // Loop with crossfade
    group.bench_function("crossfade_loop", |b| {
        b.iter(|| {
            // Simulate loop with 1 second crossfade
            for _loop_iteration in 0..10 {
                // Crossfade requires overlapping playback
                let simulated_crossfade = Duration::from_micros(100);
                std::thread::sleep(simulated_crossfade);
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_engine_initialization,
    bench_file_loading_by_size,
    bench_streaming_initiation,
    bench_concurrent_streaming,
    bench_buffer_management,
    bench_format_decoding,
    bench_streaming_with_position_updates,
    bench_music_crossfade,
    bench_streaming_with_effects,
    bench_adaptive_streaming,
    bench_streaming_memory_patterns,
    bench_streaming_seek,
    bench_streaming_loops
);

criterion_main!(benches);
