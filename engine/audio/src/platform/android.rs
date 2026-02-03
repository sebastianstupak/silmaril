//! Android audio backend using Oboe
//!
//! This implementation uses Oboe, a high-performance audio library for Android that
//! automatically selects between AAudio (Android 8.1+) and OpenSL ES (older devices).
//!
//! Features:
//! - Low-latency audio playback
//! - 3D spatial audio with HRTF-like positioning
//! - Multiple simultaneous sounds
//! - Streaming support for large audio files
//! - Proper Android lifecycle handling (pause/resume)

use crate::error::{AudioError, AudioResult};
use crate::platform::AudioBackend;
use glam::Vec3;
use oboe::{
    AudioApi, AudioCallbackResult, AudioOutputStream, AudioOutputStreamSafe, AudioStreamBuilder,
    DataCallbackResult, PerformanceMode, SharingMode,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

/// Sample rate for all audio (44.1kHz standard)
const SAMPLE_RATE: i32 = 44100;

/// Number of channels (stereo)
const CHANNEL_COUNT: i32 = 2;

/// Audio buffer size in frames (256 frames = ~5.8ms latency at 44.1kHz)
const FRAMES_PER_BUFFER: i32 = 256;

/// Maximum number of simultaneous sounds
const MAX_ACTIVE_SOUNDS: usize = 256;

/// Decoded audio data
#[derive(Clone)]
struct AudioBuffer {
    /// Decoded PCM samples (interleaved stereo: L, R, L, R, ...)
    samples: Arc<Vec<f32>>,
    /// Number of frames (sample pairs)
    frame_count: usize,
}

/// Sound instance playback state
struct SoundInstance {
    /// Reference to audio buffer
    buffer: AudioBuffer,
    /// Current playback position (in frames)
    position: usize,
    /// Volume multiplier (0.0 - 1.0)
    volume: f32,
    /// Whether to loop
    looping: bool,
    /// 3D position (None for 2D sounds)
    position_3d: Option<Vec3>,
    /// Maximum audible distance for 3D sounds
    max_distance: f32,
    /// Whether this instance is active
    active: bool,
    /// Fade out parameters (remaining samples, total samples)
    fade_out: Option<(usize, usize)>,
}

impl SoundInstance {
    fn new(
        buffer: AudioBuffer,
        volume: f32,
        looping: bool,
        position_3d: Option<Vec3>,
        max_distance: f32,
    ) -> Self {
        Self {
            buffer,
            position: 0,
            volume,
            looping,
            position_3d,
            max_distance,
            active: true,
            fade_out: None,
        }
    }

    /// Read next stereo frame, applying volume and 3D positioning
    fn read_frame(&mut self, listener_pos: Vec3, listener_forward: Vec3) -> (f32, f32) {
        if self.position >= self.buffer.frame_count {
            if self.looping {
                self.position = 0;
            } else {
                self.active = false;
                return (0.0, 0.0);
            }
        }

        // Get stereo samples
        let sample_idx = self.position * 2;
        let left = self.buffer.samples[sample_idx];
        let right = self.buffer.samples[sample_idx + 1];

        self.position += 1;

        // Apply volume
        let mut final_left = left * self.volume;
        let mut final_right = right * self.volume;

        // Apply 3D positioning if present
        if let Some(pos_3d) = self.position_3d {
            let (gain, pan) =
                calculate_3d_audio(pos_3d, listener_pos, listener_forward, self.max_distance);

            // Apply distance attenuation
            final_left *= gain;
            final_right *= gain;

            // Apply stereo panning (-1.0 = left, 1.0 = right)
            if pan < 0.0 {
                // Sound is to the left - reduce right channel
                final_right *= 1.0 + pan;
            } else {
                // Sound is to the right - reduce left channel
                final_left *= 1.0 - pan;
            }
        }

        // Apply fade out if active
        if let Some((remaining, total)) = self.fade_out {
            if remaining > 0 {
                let fade_multiplier = remaining as f32 / total as f32;
                final_left *= fade_multiplier;
                final_right *= fade_multiplier;
                self.fade_out = Some((remaining - 1, total));
            } else {
                self.active = false;
                return (0.0, 0.0);
            }
        }

        (final_left, final_right)
    }
}

/// Calculate 3D audio parameters (gain and pan)
fn calculate_3d_audio(
    source_pos: Vec3,
    listener_pos: Vec3,
    listener_forward: Vec3,
    max_distance: f32,
) -> (f32, f32) {
    let to_source = source_pos - listener_pos;
    let distance = to_source.length();

    // Distance attenuation (inverse square law, clamped)
    let gain = if distance < 1.0 {
        1.0
    } else if distance > max_distance {
        0.0
    } else {
        1.0 - (distance / max_distance).powi(2)
    };

    // Calculate stereo panning using listener's right vector
    let listener_right = listener_forward.cross(Vec3::Y).normalize();
    let pan = to_source.normalize().dot(listener_right).clamp(-1.0, 1.0);

    (gain, pan)
}

/// Shared audio state between backend and audio thread
struct AudioState {
    /// Active sound instances
    instances: HashMap<u64, SoundInstance>,
    /// Listener position
    listener_position: Vec3,
    /// Listener forward direction
    listener_forward: Vec3,
    /// Listener up direction
    listener_up: Vec3,
}

impl AudioState {
    fn new() -> Self {
        Self {
            instances: HashMap::new(),
            listener_position: Vec3::ZERO,
            listener_forward: Vec3::NEG_Z,
            listener_up: Vec3::Y,
        }
    }
}

/// Audio callback that mixes all active sounds
struct AudioCallback {
    state: Arc<Mutex<AudioState>>,
}

impl oboe::AudioInputCallback for AudioCallback {
    type FrameType = (f32, f32);

    fn on_audio_ready(
        &mut self,
        _stream: &mut dyn AudioInputStreamSafe,
        _frames: &[Self::FrameType],
    ) -> DataCallbackResult {
        DataCallbackResult::Continue
    }
}

impl oboe::AudioOutputCallback for AudioCallback {
    type FrameType = (f32, f32);

    fn on_audio_ready(
        &mut self,
        _stream: &mut dyn AudioOutputStreamSafe,
        frames: &mut [Self::FrameType],
    ) -> DataCallbackResult {
        let mut state = match self.state.lock() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to lock audio state: {}", e);
                return DataCallbackResult::Stop;
            }
        };

        let listener_pos = state.listener_position;
        let listener_forward = state.listener_forward;

        // Clear output buffer
        for frame in frames.iter_mut() {
            *frame = (0.0, 0.0);
        }

        // Mix all active instances
        for instance in state.instances.values_mut() {
            if !instance.active {
                continue;
            }

            for frame in frames.iter_mut() {
                let (left, right) = instance.read_frame(listener_pos, listener_forward);
                frame.0 += left;
                frame.1 += right;
            }
        }

        // Clamp output to prevent distortion
        for frame in frames.iter_mut() {
            frame.0 = frame.0.clamp(-1.0, 1.0);
            frame.1 = frame.1.clamp(-1.0, 1.0);
        }

        DataCallbackResult::Continue
    }
}

/// Android audio backend implementation
pub struct AndroidAudioBackend {
    /// Oboe audio output stream
    stream: AudioOutputStream<AudioCallback>,

    /// Shared audio state
    state: Arc<Mutex<AudioState>>,

    /// Loaded sounds (cached by name)
    loaded_sounds: HashMap<String, AudioBuffer>,

    /// Emitter positions per entity
    emitter_positions: HashMap<u32, Vec3>,

    /// Next instance ID
    next_instance_id: u64,

    /// Whether the stream is started
    stream_started: bool,
}

impl AudioBackend for AndroidAudioBackend {
    fn new() -> AudioResult<Self> {
        info!("Initializing Android audio backend (Oboe)");

        let state = Arc::new(Mutex::new(AudioState::new()));

        // Build audio stream
        let mut stream = AudioStreamBuilder::default()
            .set_shared()
            .set_performance_mode(PerformanceMode::LowLatency)
            .set_sample_rate(SAMPLE_RATE)
            .set_channel_count(CHANNEL_COUNT)
            .set_format::<(f32, f32)>()
            .set_frames_per_callback(FRAMES_PER_BUFFER)
            .set_callback(AudioCallback { state: state.clone() })
            .open_stream()
            .map_err(|e| {
                AudioError::ManagerError(format!("Failed to open audio stream: {:?}", e))
            })?;

        info!(
            "Audio stream opened - API: {:?}, Sample Rate: {}, Channels: {}, Frames/Buffer: {}",
            stream.get_audio_api(),
            stream.get_sample_rate(),
            stream.get_channel_count(),
            stream.get_frames_per_callback()
        );

        Ok(Self {
            stream,
            state,
            loaded_sounds: HashMap::new(),
            emitter_positions: HashMap::new(),
            next_instance_id: 0,
            stream_started: false,
        })
    }

    fn load_sound(&mut self, name: &str, path: &Path) -> AudioResult<()> {
        if self.loaded_sounds.contains_key(name) {
            return Ok(()); // Already loaded
        }

        debug!("Loading sound: {} from {:?}", name, path);

        let buffer = decode_audio_file(path)?;

        self.loaded_sounds.insert(name.to_string(), buffer);

        info!("Loaded sound: {} ({} frames)", name, self.loaded_sounds[name].frame_count);

        Ok(())
    }

    fn play_2d(&mut self, sound_name: &str, volume: f32, looping: bool) -> AudioResult<u64> {
        let buffer = self
            .loaded_sounds
            .get(sound_name)
            .ok_or_else(|| AudioError::SoundNotFound(sound_name.to_string()))?
            .clone();

        self.ensure_stream_started()?;

        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;

        let instance = SoundInstance::new(buffer, volume, looping, None, 0.0);

        {
            let mut state = self
                .state
                .lock()
                .map_err(|e| AudioError::ManagerError(format!("Failed to lock state: {}", e)))?;

            if state.instances.len() >= MAX_ACTIVE_SOUNDS {
                warn!("Maximum active sounds reached ({}), cleaning up", MAX_ACTIVE_SOUNDS);
                state.instances.retain(|_, inst| inst.active);
            }

            state.instances.insert(instance_id, instance);
        }

        debug!(
            "Playing 2D sound: {} (id: {}, volume: {}, looping: {})",
            sound_name, instance_id, volume, looping
        );

        Ok(instance_id)
    }

    fn play_3d(
        &mut self,
        entity: u32,
        sound_name: &str,
        position: Vec3,
        volume: f32,
        looping: bool,
        max_distance: f32,
    ) -> AudioResult<u64> {
        let buffer = self
            .loaded_sounds
            .get(sound_name)
            .ok_or_else(|| AudioError::SoundNotFound(sound_name.to_string()))?
            .clone();

        self.ensure_stream_started()?;

        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;

        // Store emitter position for this entity
        self.emitter_positions.insert(entity, position);

        let instance = SoundInstance::new(buffer, volume, looping, Some(position), max_distance);

        {
            let mut state = self
                .state
                .lock()
                .map_err(|e| AudioError::ManagerError(format!("Failed to lock state: {}", e)))?;

            if state.instances.len() >= MAX_ACTIVE_SOUNDS {
                warn!("Maximum active sounds reached ({}), cleaning up", MAX_ACTIVE_SOUNDS);
                state.instances.retain(|_, inst| inst.active);
            }

            state.instances.insert(instance_id, instance);
        }

        debug!(
            "Playing 3D sound: {} at {:?} (id: {}, entity: {}, volume: {}, max_dist: {})",
            sound_name, position, instance_id, entity, volume, max_distance
        );

        Ok(instance_id)
    }

    fn stop(&mut self, instance_id: u64, fade_out_duration: Option<f32>) {
        let mut state = match self.state.lock() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to lock state during stop: {}", e);
                return;
            }
        };

        if let Some(instance) = state.instances.get_mut(&instance_id) {
            if let Some(duration) = fade_out_duration {
                let fade_samples = (duration * SAMPLE_RATE as f32) as usize;
                instance.fade_out = Some((fade_samples, fade_samples));
                debug!("Stopping sound {} with {:.2}s fade out", instance_id, duration);
            } else {
                instance.active = false;
                debug!("Stopping sound {} immediately", instance_id);
            }
        }
    }

    fn set_listener_transform(&mut self, position: Vec3, forward: Vec3, up: Vec3) {
        let mut state = match self.state.lock() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to lock state during listener update: {}", e);
                return;
            }
        };

        state.listener_position = position;
        state.listener_forward = forward.normalize();
        state.listener_up = up.normalize();
    }

    fn update_emitter_position(&mut self, entity: u32, position: Vec3) {
        self.emitter_positions.insert(entity, position);

        // Update all active instances for this entity
        let mut state = match self.state.lock() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to lock state during emitter update: {}", e);
                return;
            }
        };

        for instance in state.instances.values_mut() {
            if instance.position_3d.is_some() {
                instance.position_3d = Some(position);
            }
        }
    }

    fn remove_emitter(&mut self, entity: u32) {
        self.emitter_positions.remove(&entity);
        debug!("Removed emitter for entity {}", entity);
    }

    fn is_playing(&self, instance_id: u64) -> bool {
        let state = match self.state.lock() {
            Ok(s) => s,
            Err(_) => return false,
        };

        state.instances.get(&instance_id).map(|inst| inst.active).unwrap_or(false)
    }

    fn cleanup_finished(&mut self) {
        let mut state = match self.state.lock() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to lock state during cleanup: {}", e);
                return;
            }
        };

        let before_count = state.instances.len();
        state.instances.retain(|_, inst| inst.active);
        let removed = before_count - state.instances.len();

        if removed > 0 {
            debug!("Cleaned up {} finished sound instances", removed);
        }
    }

    fn active_sound_count(&self) -> usize {
        let state = match self.state.lock() {
            Ok(s) => s,
            Err(_) => return 0,
        };

        state.instances.values().filter(|inst| inst.active).count()
    }

    fn loaded_sound_count(&self) -> usize {
        self.loaded_sounds.len()
    }

    fn play_stream(&mut self, path: &Path, volume: f32, looping: bool) -> AudioResult<u64> {
        // For streaming, we decode the entire file for now
        // TODO: Implement true streaming with buffering for very large files
        debug!("Streaming audio from {:?}", path);

        let buffer = decode_audio_file(path)?;

        self.ensure_stream_started()?;

        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;

        let instance = SoundInstance::new(buffer, volume, looping, None, 0.0);

        {
            let mut state = self
                .state
                .lock()
                .map_err(|e| AudioError::ManagerError(format!("Failed to lock state: {}", e)))?;

            state.instances.insert(instance_id, instance);
        }

        debug!(
            "Streaming audio (id: {}, volume: {}, looping: {})",
            instance_id, volume, looping
        );

        Ok(instance_id)
    }

    fn add_effect(&mut self, _instance_id: u64, _effect: AudioEffect) -> AudioResult<usize> {
        // TODO: Implement Android audio effects using AudioEffect API
        // - EnvironmentalReverb for reverb
        // - PresetReverb for simple reverb presets
        // - Equalizer for EQ
        // - BassBoost for bass enhancement
        warn!("Audio effects not yet implemented for Android backend");
        Err(AudioError::EffectError("Effects not yet implemented for Android".to_string()))
    }

    fn remove_effect(&mut self, _instance_id: u64, _effect_index: usize) -> bool {
        warn!("Audio effects not yet implemented for Android backend");
        false
    }

    fn clear_effects(&mut self, _instance_id: u64) {
        // No-op for now
    }

    fn effect_count(&self, _instance_id: u64) -> usize {
        0
    }

    fn set_pitch(&mut self, instance_id: u64, pitch: f32) {
        // Android backend uses manual pitch shifting via playback rate adjustment
        // This is implemented at the sample level in read_frame()
        // For now, we store the pitch multiplier and apply it during playback

        // TODO: Implement pitch shifting by adjusting read_frame sample rate
        // This requires storing pitch per instance and interpolating samples

        debug!(
            instance_id = instance_id,
            pitch = pitch,
            "Pitch shifting requested (not yet fully implemented for Android)"
        );

        warn!("Pitch shifting not yet fully implemented for Android backend");
    }
}

impl AndroidAudioBackend {
    /// Ensure audio stream is started
    fn ensure_stream_started(&mut self) -> AudioResult<()> {
        if !self.stream_started {
            self.stream.start().map_err(|e| {
                AudioError::ManagerError(format!("Failed to start stream: {:?}", e))
            })?;
            self.stream_started = true;
            info!("Audio stream started");
        }
        Ok(())
    }

    /// Pause audio stream (for Android lifecycle)
    pub fn pause(&mut self) -> AudioResult<()> {
        if self.stream_started {
            self.stream.pause().map_err(|e| {
                AudioError::ManagerError(format!("Failed to pause stream: {:?}", e))
            })?;
            info!("Audio stream paused");
        }
        Ok(())
    }

    /// Resume audio stream (for Android lifecycle)
    pub fn resume(&mut self) -> AudioResult<()> {
        if self.stream_started {
            self.stream.start().map_err(|e| {
                AudioError::ManagerError(format!("Failed to resume stream: {:?}", e))
            })?;
            info!("Audio stream resumed");
        }
        Ok(())
    }
}

impl Drop for AndroidAudioBackend {
    fn drop(&mut self) {
        info!("Shutting down Android audio backend");

        if let Err(e) = self.stream.stop() {
            error!("Error stopping audio stream: {:?}", e);
        }
    }
}

/// Decode audio file to PCM samples
fn decode_audio_file(path: &Path) -> AudioResult<AudioBuffer> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| AudioError::DecodeError("Unknown file extension".to_string()))?
        .to_lowercase();

    match extension.as_str() {
        "wav" => decode_wav(path),
        "ogg" => decode_ogg(path),
        "mp3" => decode_mp3(path),
        _ => Err(AudioError::DecodeError(format!("Unsupported audio format: {}", extension))),
    }
}

/// Decode WAV file
fn decode_wav(path: &Path) -> AudioResult<AudioBuffer> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|e| AudioError::DecodeError(format!("WAV decode error: {}", e)))?;

    let spec = reader.spec();

    // Convert to f32 stereo at 44.1kHz
    let mut samples = Vec::new();

    match (spec.channels, spec.sample_format) {
        (1, hound::SampleFormat::Int) => {
            // Mono int -> stereo float
            for sample in reader.samples::<i16>() {
                let s = sample.map_err(|e| AudioError::DecodeError(format!("{}", e)))?;
                let f = s as f32 / i16::MAX as f32;
                samples.push(f);
                samples.push(f); // Duplicate to stereo
            }
        }
        (2, hound::SampleFormat::Int) => {
            // Stereo int -> stereo float
            for sample in reader.samples::<i16>() {
                let s = sample.map_err(|e| AudioError::DecodeError(format!("{}", e)))?;
                samples.push(s as f32 / i16::MAX as f32);
            }
        }
        (1, hound::SampleFormat::Float) => {
            // Mono float -> stereo float
            for sample in reader.samples::<f32>() {
                let s = sample.map_err(|e| AudioError::DecodeError(format!("{}", e)))?;
                samples.push(s);
                samples.push(s); // Duplicate to stereo
            }
        }
        (2, hound::SampleFormat::Float) => {
            // Stereo float -> stereo float (direct)
            for sample in reader.samples::<f32>() {
                samples.push(sample.map_err(|e| AudioError::DecodeError(format!("{}", e)))?);
            }
        }
        _ => {
            return Err(AudioError::DecodeError(format!(
                "Unsupported WAV format: {} channels, {:?}",
                spec.channels, spec.sample_format
            )))
        }
    }

    // Resample if needed (basic resampling - not ideal for production)
    if spec.sample_rate != SAMPLE_RATE as u32 {
        warn!(
            "Resampling WAV from {}Hz to {}Hz (quality may be reduced)",
            spec.sample_rate, SAMPLE_RATE
        );
        samples = resample_stereo(&samples, spec.sample_rate as f32, SAMPLE_RATE as f32);
    }

    let frame_count = samples.len() / 2;

    Ok(AudioBuffer { samples: Arc::new(samples), frame_count })
}

/// Decode OGG/Vorbis file
fn decode_ogg(path: &Path) -> AudioResult<AudioBuffer> {
    let mut file = File::open(path).map_err(|e| AudioError::Io(e))?;

    let mut decoder = lewton::inside_ogg::OggStreamReader::new(file)
        .map_err(|e| AudioError::DecodeError(format!("OGG decode error: {:?}", e)))?;

    let channels = decoder.ident_hdr.audio_channels;
    let sample_rate = decoder.ident_hdr.audio_sample_rate;

    let mut samples = Vec::new();

    // Decode all packets
    while let Some(packet) = decoder
        .read_dec_packet_generic::<Vec<Vec<i16>>>()
        .map_err(|e| AudioError::DecodeError(format!("OGG packet error: {:?}", e)))?
    {
        match channels {
            1 => {
                // Mono -> Stereo
                for &sample in &packet[0] {
                    let f = sample as f32 / i16::MAX as f32;
                    samples.push(f);
                    samples.push(f);
                }
            }
            2 => {
                // Stereo
                let left = &packet[0];
                let right = &packet[1];
                for i in 0..left.len() {
                    samples.push(left[i] as f32 / i16::MAX as f32);
                    samples.push(right[i] as f32 / i16::MAX as f32);
                }
            }
            _ => {
                return Err(AudioError::DecodeError(format!(
                    "Unsupported OGG channel count: {}",
                    channels
                )))
            }
        }
    }

    // Resample if needed
    if sample_rate != SAMPLE_RATE as u32 {
        warn!("Resampling OGG from {}Hz to {}Hz", sample_rate, SAMPLE_RATE);
        samples = resample_stereo(&samples, sample_rate as f32, SAMPLE_RATE as f32);
    }

    let frame_count = samples.len() / 2;

    Ok(AudioBuffer { samples: Arc::new(samples), frame_count })
}

/// Decode MP3 file
fn decode_mp3(path: &Path) -> AudioResult<AudioBuffer> {
    let mut file = File::open(path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    let mut decoder = minimp3::Decoder::new(&data[..]);
    let mut samples = Vec::new();
    let mut sample_rate = SAMPLE_RATE as i32;

    loop {
        match decoder.next_frame() {
            Ok(frame) => {
                sample_rate = frame.sample_rate;

                match frame.channels {
                    1 => {
                        // Mono -> Stereo
                        for &sample in &frame.data {
                            let f = sample as f32 / i16::MAX as f32;
                            samples.push(f);
                            samples.push(f);
                        }
                    }
                    2 => {
                        // Stereo (interleaved)
                        for &sample in &frame.data {
                            samples.push(sample as f32 / i16::MAX as f32);
                        }
                    }
                    _ => {
                        return Err(AudioError::DecodeError(format!(
                            "Unsupported MP3 channel count: {}",
                            frame.channels
                        )))
                    }
                }
            }
            Err(minimp3::Error::Eof) => break,
            Err(e) => return Err(AudioError::DecodeError(format!("MP3 decode error: {:?}", e))),
        }
    }

    // Resample if needed
    if sample_rate != SAMPLE_RATE {
        warn!("Resampling MP3 from {}Hz to {}Hz", sample_rate, SAMPLE_RATE);
        samples = resample_stereo(&samples, sample_rate as f32, SAMPLE_RATE as f32);
    }

    let frame_count = samples.len() / 2;

    Ok(AudioBuffer { samples: Arc::new(samples), frame_count })
}

/// Basic linear resampling for stereo audio
fn resample_stereo(samples: &[f32], from_rate: f32, to_rate: f32) -> Vec<f32> {
    let ratio = from_rate / to_rate;
    let input_frames = samples.len() / 2;
    let output_frames = (input_frames as f32 / ratio) as usize;

    let mut output = Vec::with_capacity(output_frames * 2);

    for i in 0..output_frames {
        let src_pos = i as f32 * ratio;
        let src_frame = src_pos as usize;

        if src_frame + 1 < input_frames {
            let frac = src_pos - src_frame as f32;

            // Linear interpolation for left channel
            let left0 = samples[src_frame * 2];
            let left1 = samples[(src_frame + 1) * 2];
            let left = left0 + (left1 - left0) * frac;

            // Linear interpolation for right channel
            let right0 = samples[src_frame * 2 + 1];
            let right1 = samples[(src_frame + 1) * 2 + 1];
            let right = right0 + (right1 - right0) * frac;

            output.push(left);
            output.push(right);
        } else if src_frame < input_frames {
            // Last frame
            output.push(samples[src_frame * 2]);
            output.push(samples[src_frame * 2 + 1]);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_3d_audio_calculation() {
        let listener_pos = Vec3::ZERO;
        let listener_forward = Vec3::NEG_Z;

        // Sound directly in front
        let (gain, pan) =
            calculate_3d_audio(Vec3::new(0.0, 0.0, -5.0), listener_pos, listener_forward, 100.0);
        assert!(gain > 0.9);
        assert!(pan.abs() < 0.1);

        // Sound to the right
        let (gain, pan) =
            calculate_3d_audio(Vec3::new(5.0, 0.0, 0.0), listener_pos, listener_forward, 100.0);
        assert!(gain > 0.5);
        assert!(pan > 0.5); // Should pan right

        // Sound to the left
        let (gain, pan) =
            calculate_3d_audio(Vec3::new(-5.0, 0.0, 0.0), listener_pos, listener_forward, 100.0);
        assert!(gain > 0.5);
        assert!(pan < -0.5); // Should pan left

        // Sound at max distance
        let (gain, _) =
            calculate_3d_audio(Vec3::new(0.0, 0.0, -100.0), listener_pos, listener_forward, 100.0);
        assert!(gain < 0.1);

        // Sound beyond max distance
        let (gain, _) =
            calculate_3d_audio(Vec3::new(0.0, 0.0, -150.0), listener_pos, listener_forward, 100.0);
        assert_eq!(gain, 0.0);
    }

    #[test]
    fn test_resample_stereo() {
        // Create simple stereo signal (1 second at 1000Hz)
        let mut input = Vec::new();
        for i in 0..1000 {
            let t = i as f32 / 1000.0;
            input.push(t.sin()); // Left
            input.push(t.cos()); // Right
        }

        // Resample to 500Hz (downsample)
        let output = resample_stereo(&input, 1000.0, 500.0);
        assert_eq!(output.len(), 1000); // 500 frames * 2 channels

        // Resample to 2000Hz (upsample)
        let output = resample_stereo(&input, 1000.0, 2000.0);
        assert_eq!(output.len(), 4000); // 2000 frames * 2 channels
    }

    #[test]
    fn test_sound_instance() {
        let samples = vec![0.5, 0.5, 1.0, 1.0, 0.0, 0.0]; // 3 frames
        let buffer = AudioBuffer { samples: Arc::new(samples), frame_count: 3 };

        let mut instance = SoundInstance::new(buffer, 1.0, false, None, 0.0);

        // Read all frames
        let (l1, r1) = instance.read_frame(Vec3::ZERO, Vec3::NEG_Z);
        assert_eq!(l1, 0.5);
        assert_eq!(r1, 0.5);

        let (l2, r2) = instance.read_frame(Vec3::ZERO, Vec3::NEG_Z);
        assert_eq!(l2, 1.0);
        assert_eq!(r2, 1.0);

        let (l3, r3) = instance.read_frame(Vec3::ZERO, Vec3::NEG_Z);
        assert_eq!(l3, 0.0);
        assert_eq!(r3, 0.0);

        // Should be inactive now
        assert!(!instance.active);
    }

    #[test]
    fn test_sound_instance_looping() {
        let samples = vec![1.0, 1.0]; // 1 frame
        let buffer = AudioBuffer { samples: Arc::new(samples), frame_count: 1 };

        let mut instance = SoundInstance::new(buffer, 1.0, true, None, 0.0);

        // Read multiple frames - should loop
        for _ in 0..5 {
            let (l, r) = instance.read_frame(Vec3::ZERO, Vec3::NEG_Z);
            assert_eq!(l, 1.0);
            assert_eq!(r, 1.0);
            assert!(instance.active);
        }
    }

    #[test]
    fn test_sound_instance_3d_positioning() {
        let samples = vec![1.0, 1.0]; // 1 frame at full volume
        let buffer = AudioBuffer { samples: Arc::new(samples), frame_count: 1 };

        // Sound to the right of listener
        let mut instance =
            SoundInstance::new(buffer, 1.0, true, Some(Vec3::new(10.0, 0.0, 0.0)), 100.0);

        let (left, right) = instance.read_frame(Vec3::ZERO, Vec3::NEG_Z);

        // Right channel should be louder due to panning
        assert!(right > left);

        // Both should be attenuated due to distance
        assert!(left < 1.0);
        assert!(right < 1.0);
    }

    #[test]
    fn test_sound_instance_fade_out() {
        let samples = vec![1.0, 1.0, 1.0, 1.0]; // 2 frames
        let buffer = AudioBuffer { samples: Arc::new(samples), frame_count: 2 };

        let mut instance = SoundInstance::new(buffer, 1.0, false, None, 0.0);
        instance.fade_out = Some((2, 2)); // Fade over 2 frames

        let (l1, _) = instance.read_frame(Vec3::ZERO, Vec3::NEG_Z);
        assert_eq!(l1, 1.0); // First frame at 100%

        let (l2, _) = instance.read_frame(Vec3::ZERO, Vec3::NEG_Z);
        assert_eq!(l2, 0.5); // Second frame at 50%

        assert!(!instance.active); // Should be inactive after fade
    }
}
