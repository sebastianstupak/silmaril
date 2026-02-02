//! Audio data structures and loaders
//!
//! Pure data structures for audio assets. No audio playback dependencies.
//! Can be used by server (for validation), tools, or client.

use crate::validation::{compute_hash, AssetValidator, ValidationError};
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use tracing::{info, instrument};

define_error! {
    pub enum AudioError {
        InvalidWavFormat { reason: String } = ErrorCode::SoundLoadFailed, ErrorSeverity::Error,
        InvalidOggFormat { reason: String } = ErrorCode::SoundLoadFailed, ErrorSeverity::Error,
        UnsupportedFormat { format: String } = ErrorCode::SoundLoadFailed, ErrorSeverity::Error,
        IoError { reason: String } = ErrorCode::SoundLoadFailed, ErrorSeverity::Error,
    }
}

/// Audio sample format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    /// 16-bit PCM (signed integer)
    PCM16,
    /// Vorbis compressed audio
    Vorbis,
    /// Opus compressed audio
    Opus,
}

/// Audio data (CPU-side audio samples)
///
/// Pure data structure - no audio playback state.
/// Audio backends create playback resources from this data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioData {
    /// Sample rate in Hz (e.g., 44100, 48000)
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo)
    pub channels: u16,
    /// Audio format
    pub format: AudioFormat,
    /// Raw audio data
    pub data: Vec<u8>,
}

impl AudioData {
    /// Create a new audio data structure
    pub fn new(sample_rate: u32, channels: u16, format: AudioFormat, data: Vec<u8>) -> Self {
        Self { sample_rate, channels, format, data }
    }

    /// Calculate the duration of the audio in seconds
    pub fn duration(&self) -> f64 {
        match self.format {
            AudioFormat::PCM16 => {
                let samples = self.data.len() / 2; // 2 bytes per sample for 16-bit
                let frames = samples / self.channels as usize;
                frames as f64 / self.sample_rate as f64
            }
            AudioFormat::Vorbis | AudioFormat::Opus => {
                // For compressed formats, this is an estimate
                // Actual duration would need to be decoded
                0.0
            }
        }
    }

    /// Get the number of samples (per channel)
    pub fn sample_count(&self) -> usize {
        match self.format {
            AudioFormat::PCM16 => {
                let samples = self.data.len() / 2; // 2 bytes per sample
                samples / self.channels as usize
            }
            AudioFormat::Vorbis | AudioFormat::Opus => 0, // Compressed formats need decoding
        }
    }

    /// Load audio from WAV file data
    #[instrument(skip(wav_data))]
    pub fn from_wav(wav_data: &[u8]) -> Result<Self, AudioError> {
        info!("Loading WAV audio");

        let cursor = Cursor::new(wav_data);
        let reader = hound::WavReader::new(cursor)
            .map_err(|e| AudioError::InvalidWavFormat { reason: e.to_string() })?;

        let spec = reader.spec();

        // Only support 16-bit PCM for now
        if spec.bits_per_sample != 16 {
            return Err(AudioError::UnsupportedFormat {
                format: format!("{}-bit WAV", spec.bits_per_sample),
            });
        }

        if spec.sample_format != hound::SampleFormat::Int {
            return Err(AudioError::UnsupportedFormat {
                format: "non-integer WAV format".to_string(),
            });
        }

        let samples: Result<Vec<i16>, _> = reader.into_samples().collect();
        let samples =
            samples.map_err(|e| AudioError::InvalidWavFormat { reason: e.to_string() })?;

        let sample_count = samples.len();

        // Convert samples to bytes
        let mut data = Vec::with_capacity(sample_count * 2);
        for sample in samples {
            data.extend_from_slice(&sample.to_le_bytes());
        }

        info!(
            sample_rate = spec.sample_rate,
            channels = spec.channels,
            samples = sample_count,
            "WAV loaded successfully"
        );

        Ok(Self {
            sample_rate: spec.sample_rate,
            channels: spec.channels,
            format: AudioFormat::PCM16,
            data,
        })
    }

    /// Load audio from OGG Vorbis file data
    #[instrument(skip(ogg_data))]
    pub fn from_ogg(ogg_data: &[u8]) -> Result<Self, AudioError> {
        info!("Loading OGG Vorbis audio");

        let cursor = Cursor::new(ogg_data);
        let mut reader = lewton::inside_ogg::OggStreamReader::new(cursor)
            .map_err(|e| AudioError::InvalidOggFormat { reason: e.to_string() })?;

        let sample_rate = reader.ident_hdr.audio_sample_rate;
        let channels = reader.ident_hdr.audio_channels as u16;

        let mut pcm_data: Vec<i16> = Vec::new();

        // Decode all packets
        while let Some(packet) = reader
            .read_dec_packet_itl()
            .map_err(|e| AudioError::InvalidOggFormat { reason: e.to_string() })?
        {
            pcm_data.extend(packet);
        }

        // Convert samples to bytes
        let mut data = Vec::with_capacity(pcm_data.len() * 2);
        for sample in pcm_data {
            data.extend_from_slice(&sample.to_le_bytes());
        }

        info!(
            sample_rate = sample_rate,
            channels = channels,
            samples = data.len() / 2,
            "OGG loaded and decoded successfully"
        );

        Ok(Self { sample_rate, channels, format: AudioFormat::PCM16, data })
    }
}

// ============================================================================
// Validation Implementation
// ============================================================================

impl AssetValidator for AudioData {
    /// Validate audio format (WAV/OGG headers)
    fn validate_format(data: &[u8]) -> Result<(), ValidationError> {
        if data.is_empty() {
            return Err(ValidationError::emptydata());
        }

        // Check for WAV header
        if data.len() >= 4 && &data[0..4] == b"RIFF" {
            // Validate WAV structure
            if data.len() < 44 {
                return Err(ValidationError::invaliddimensions("WAV file too small".to_string()));
            }
            return Ok(());
        }

        // Check for OGG header
        if data.len() >= 4 && &data[0..4] == b"OggS" {
            return Ok(());
        }

        // Unknown format
        Err(ValidationError::invaliddimensions(
            "Unknown audio format (expected WAV or OGG)".to_string(),
        ))
    }

    /// Validate audio data integrity
    fn validate_data(&self) -> Result<(), ValidationError> {
        // Validate sample rate
        if self.sample_rate == 0 {
            return Err(ValidationError::invaliddimensions(
                "Sample rate cannot be zero".to_string(),
            ));
        }

        // Common sample rates: 8000, 11025, 16000, 22050, 44100, 48000, 96000
        const MAX_SAMPLE_RATE: u32 = 192000;
        if self.sample_rate > MAX_SAMPLE_RATE {
            return Err(ValidationError::invaliddimensions(format!(
                "Sample rate too high: {} (max {})",
                self.sample_rate, MAX_SAMPLE_RATE
            )));
        }

        // Validate channels
        if self.channels == 0 || self.channels > 8 {
            return Err(ValidationError::invaliddimensions(format!(
                "Invalid channel count: {} (expected 1-8)",
                self.channels
            )));
        }

        // Validate data size
        if self.data.is_empty() {
            return Err(ValidationError::emptydata());
        }

        // For PCM16, validate data size is even (2 bytes per sample)
        if self.format == AudioFormat::PCM16 && self.data.len() % 2 != 0 {
            return Err(ValidationError::invaliddimensions(format!(
                "PCM16 data size must be even, got {}",
                self.data.len()
            )));
        }

        Ok(())
    }

    /// Validate checksum
    fn validate_checksum(&self, expected: &[u8; 32]) -> Result<(), ValidationError> {
        let actual = self.compute_checksum();
        if &actual != expected {
            return Err(ValidationError::checksummismatch(*expected, actual));
        }
        Ok(())
    }

    /// Compute Blake3 checksum of audio data
    fn compute_checksum(&self) -> [u8; 32] {
        compute_hash(&self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_audio_with_pcm_data() {
        let data = vec![0u8; 1000];
        let audio = AudioData::new(44100, 2, AudioFormat::PCM16, data);

        assert_eq!(audio.sample_rate, 44100);
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.format, AudioFormat::PCM16);
        assert_eq!(audio.data.len(), 1000);
    }

    #[test]
    fn test_query_sample_rate() {
        let audio = AudioData::new(48000, 2, AudioFormat::PCM16, vec![0u8; 1000]);
        assert_eq!(audio.sample_rate, 48000);
    }

    #[test]
    fn test_query_channels() {
        let mono = AudioData::new(44100, 1, AudioFormat::PCM16, vec![0u8; 1000]);
        assert_eq!(mono.channels, 1);

        let stereo = AudioData::new(44100, 2, AudioFormat::PCM16, vec![0u8; 1000]);
        assert_eq!(stereo.channels, 2);
    }

    #[test]
    fn test_duration_calculation() {
        // 1 second of stereo audio at 44100 Hz
        // 44100 samples/sec * 2 channels * 2 bytes/sample = 176400 bytes
        let audio = AudioData::new(44100, 2, AudioFormat::PCM16, vec![0u8; 176400]);
        let duration = audio.duration();

        // Should be approximately 1.0 second
        assert!((duration - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_sample_count() {
        // 1000 samples (stereo) = 2000 total samples = 4000 bytes
        let audio = AudioData::new(44100, 2, AudioFormat::PCM16, vec![0u8; 4000]);
        assert_eq!(audio.sample_count(), 1000);
    }

    #[test]
    fn test_load_16bit_wav() {
        // Create a simple WAV file in memory
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
            // Write 100 samples (200 total for stereo)
            for i in 0..200 {
                writer.write_sample(i as i16).unwrap();
            }
            writer.finalize().unwrap();
        }

        let wav_data = cursor.into_inner();
        let audio = AudioData::from_wav(&wav_data).expect("Failed to load WAV");

        assert_eq!(audio.sample_rate, 44100);
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.format, AudioFormat::PCM16);
        assert_eq!(audio.data.len(), 400); // 200 samples * 2 bytes
    }

    #[test]
    fn test_wav_sample_rate_correct() {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
            for i in 0..100 {
                writer.write_sample(i as i16).unwrap();
            }
            writer.finalize().unwrap();
        }

        let wav_data = cursor.into_inner();
        let audio = AudioData::from_wav(&wav_data).expect("Failed to load WAV");

        assert_eq!(audio.sample_rate, 48000);
    }

    #[test]
    fn test_wav_channels_correct() {
        let mono_spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, mono_spec).unwrap();
            for i in 0..100 {
                writer.write_sample(i as i16).unwrap();
            }
            writer.finalize().unwrap();
        }

        let wav_data = cursor.into_inner();
        let audio = AudioData::from_wav(&wav_data).expect("Failed to load WAV");

        assert_eq!(audio.channels, 1);
    }

    #[test]
    fn test_invalid_wav_returns_error() {
        let invalid_data = b"this is not a wav file";
        let result = AudioData::from_wav(invalid_data);

        assert!(result.is_err());
        match result {
            Err(AudioError::InvalidWavFormat { .. }) => (),
            _ => panic!("Expected InvalidWavFormat error"),
        }
    }

    #[test]
    fn test_audio_format_variants() {
        assert_eq!(AudioFormat::PCM16, AudioFormat::PCM16);
        assert_ne!(AudioFormat::PCM16, AudioFormat::Vorbis);
        assert_ne!(AudioFormat::Vorbis, AudioFormat::Opus);
    }

    // ========================================================================
    // Validation Tests
    // ========================================================================

    use crate::validation::{AssetValidator, ValidationError};

    #[test]
    fn test_valid_audio_passes_validation() {
        let audio = AudioData::new(44100, 2, AudioFormat::PCM16, vec![0u8; 1000]);
        let report = audio.validate_all();
        assert!(report.is_valid());
    }

    #[test]
    fn test_validate_data_zero_sample_rate() {
        let audio = AudioData::new(0, 2, AudioFormat::PCM16, vec![0u8; 100]);
        let result = audio.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::InvalidDimensions { .. }) => {}
            _ => panic!("Expected InvalidDimensions error"),
        }
    }

    #[test]
    fn test_validate_data_excessive_sample_rate() {
        let audio = AudioData::new(999999, 2, AudioFormat::PCM16, vec![0u8; 100]);
        let result = audio.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::InvalidDimensions { reason }) => {
                assert!(reason.contains("too high"));
            }
            _ => panic!("Expected InvalidDimensions error"),
        }
    }

    #[test]
    fn test_validate_data_zero_channels() {
        let audio = AudioData::new(44100, 0, AudioFormat::PCM16, vec![0u8; 100]);
        let result = audio.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::InvalidDimensions { reason }) => {
                assert!(reason.contains("channel"));
            }
            _ => panic!("Expected InvalidDimensions error"),
        }
    }

    #[test]
    fn test_validate_data_too_many_channels() {
        let audio = AudioData::new(44100, 10, AudioFormat::PCM16, vec![0u8; 100]);
        let result = audio.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::InvalidDimensions { reason }) => {
                assert!(reason.contains("channel"));
            }
            _ => panic!("Expected InvalidDimensions error"),
        }
    }

    #[test]
    fn test_validate_data_empty_audio() {
        let audio = AudioData::new(44100, 2, AudioFormat::PCM16, vec![]);
        let result = audio.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::EmptyData {}) => {}
            _ => panic!("Expected EmptyData error"),
        }
    }

    #[test]
    fn test_validate_data_pcm16_odd_size() {
        let audio = AudioData::new(44100, 2, AudioFormat::PCM16, vec![0u8; 99]); // Odd size
        let result = audio.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::InvalidDimensions { reason }) => {
                assert!(reason.contains("even"));
            }
            _ => panic!("Expected InvalidDimensions error"),
        }
    }

    #[test]
    fn test_validate_format_wav_header() {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
            for i in 0..100 {
                writer.write_sample(i as i16).unwrap();
            }
            writer.finalize().unwrap();
        }

        let wav_data = cursor.into_inner();
        assert!(AudioData::validate_format(&wav_data).is_ok());
    }

    #[test]
    fn test_validate_format_empty_data() {
        let result = AudioData::validate_format(&[]);
        assert!(result.is_err());
        match result {
            Err(ValidationError::EmptyData {}) => {}
            _ => panic!("Expected EmptyData error"),
        }
    }

    #[test]
    fn test_checksum_validation_passes() {
        let audio = AudioData::new(44100, 2, AudioFormat::PCM16, vec![128u8; 1000]);
        let checksum = audio.compute_checksum();
        assert!(audio.validate_checksum(&checksum).is_ok());
    }

    #[test]
    fn test_checksum_validation_fails() {
        let audio = AudioData::new(44100, 2, AudioFormat::PCM16, vec![128u8; 1000]);
        let wrong_checksum = [0u8; 32];
        let result = audio.validate_checksum(&wrong_checksum);
        assert!(result.is_err());
    }

    #[test]
    fn test_checksum_deterministic() {
        let audio = AudioData::new(44100, 2, AudioFormat::PCM16, vec![200u8; 1000]);
        let hash1 = audio.compute_checksum();
        let hash2 = audio.compute_checksum();
        assert_eq!(hash1, hash2);
    }
}
