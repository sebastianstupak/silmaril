//! Esports-Grade Replay System
//!
//! Provides frame-perfect recording, compression, seeking, and playback
//! for competitive multiplayer game replays.

use std::collections::VecDeque;
use std::io::Write;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Replay frame containing complete world state
#[derive(Debug, Clone)]
pub struct ReplayFrame {
    /// Frame tick number
    pub tick: u64,
    /// Frame timestamp (nanoseconds since replay start)
    pub timestamp_nanos: u64,
    /// Serialized world state (FlatBuffers format)
    pub world_state: Vec<u8>,
    /// Player inputs for this frame
    pub inputs: Vec<PlayerInput>,
    /// Frame metadata (scores, events, etc.)
    pub metadata: FrameMetadata,
}

/// Player input for a single frame
#[derive(Debug, Clone)]
pub struct PlayerInput {
    /// Player ID
    pub player_id: u64,
    /// Input bitfield (movement, actions, etc.)
    pub input_bits: u32,
    /// View direction (yaw, pitch)
    pub view_direction: [f32; 2],
}

/// Frame metadata
#[derive(Debug, Clone, Default)]
pub struct FrameMetadata {
    /// Player scores
    pub scores: Vec<(u64, u32)>,
    /// Game events this frame (kills, objectives, etc.)
    pub events: Vec<String>,
    /// Custom metadata
    pub custom: Vec<(String, String)>,
}

/// Replay configuration
#[derive(Debug, Clone)]
pub struct ReplayConfig {
    /// Record every N ticks (1 = every tick, 2 = every other tick)
    pub record_interval: u32,
    /// Maximum replay duration (ticks)
    pub max_duration_ticks: u64,
    /// Enable compression
    pub enable_compression: bool,
    /// Maximum buffer size (frames)
    pub max_buffer_size: usize,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            record_interval: 1,          // Record every tick
            max_duration_ticks: 216_000, // 60 minutes at 60 TPS
            enable_compression: true,
            max_buffer_size: 3600, // 1 minute at 60 TPS
        }
    }
}

/// Replay recorder for capturing game state
pub struct ReplayRecorder {
    config: ReplayConfig,
    frames: VecDeque<ReplayFrame>,
    current_tick: u64,
    start_time_nanos: u64,
    recording: bool,
}

impl ReplayRecorder {
    /// Create a new replay recorder
    pub fn new(config: ReplayConfig) -> Self {
        info!(
            record_interval = config.record_interval,
            max_duration_mins = config.max_duration_ticks / 60 / 60,
            compression = config.enable_compression,
            "Creating replay recorder"
        );

        Self {
            config,
            frames: VecDeque::new(),
            current_tick: 0,
            start_time_nanos: 0,
            recording: false,
        }
    }

    /// Start recording
    pub fn start_recording(&mut self) {
        self.recording = true;
        self.current_tick = 0;
        self.start_time_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        self.frames.clear();
        info!("Started replay recording");
    }

    /// Stop recording
    pub fn stop_recording(&mut self) {
        self.recording = false;
        info!(
            frame_count = self.frames.len(),
            duration_secs = self.current_tick / 60,
            "Stopped replay recording"
        );
    }

    /// Record a frame
    pub fn record_frame(
        &mut self,
        world_state: Vec<u8>,
        inputs: Vec<PlayerInput>,
        metadata: FrameMetadata,
    ) -> Result<(), String> {
        if !self.recording {
            return Ok(());
        }

        // Check if we should record this tick
        if !self.current_tick.is_multiple_of(self.config.record_interval as u64) {
            self.current_tick += 1;
            return Ok(());
        }

        // Check duration limit
        if self.current_tick >= self.config.max_duration_ticks {
            warn!("Replay duration limit reached, stopping recording");
            self.stop_recording();
            return Err("Replay duration limit reached".to_string());
        }

        let timestamp_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
            - self.start_time_nanos;

        let frame = ReplayFrame {
            tick: self.current_tick,
            timestamp_nanos,
            world_state: if self.config.enable_compression {
                Self::compress_data(&world_state)?
            } else {
                world_state
            },
            inputs,
            metadata,
        };

        // Add to buffer
        self.frames.push_back(frame);

        // Limit buffer size
        if self.frames.len() > self.config.max_buffer_size {
            self.frames.pop_front();
        }

        self.current_tick += 1;

        Ok(())
    }

    /// Get frame count
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get recording status
    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// Get current tick
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Save replay to bytes
    pub fn save_to_bytes(&self) -> Result<Vec<u8>, String> {
        let mut buffer = Vec::new();

        // Write header
        buffer.extend_from_slice(b"AGE_REPLAY_V1");
        buffer.extend_from_slice(&self.config.record_interval.to_le_bytes());
        buffer.extend_from_slice(&(self.frames.len() as u64).to_le_bytes());

        // Write frames
        for frame in &self.frames {
            // Tick
            buffer.extend_from_slice(&frame.tick.to_le_bytes());
            // Timestamp
            buffer.extend_from_slice(&frame.timestamp_nanos.to_le_bytes());
            // World state length + data
            buffer.extend_from_slice(&(frame.world_state.len() as u32).to_le_bytes());
            buffer.extend_from_slice(&frame.world_state);
            // Inputs count + data
            buffer.extend_from_slice(&(frame.inputs.len() as u32).to_le_bytes());
            for input in &frame.inputs {
                buffer.extend_from_slice(&input.player_id.to_le_bytes());
                buffer.extend_from_slice(&input.input_bits.to_le_bytes());
                buffer.extend_from_slice(&input.view_direction[0].to_le_bytes());
                buffer.extend_from_slice(&input.view_direction[1].to_le_bytes());
            }
            // Metadata
            buffer.extend_from_slice(&(frame.metadata.scores.len() as u32).to_le_bytes());
            for (player_id, score) in &frame.metadata.scores {
                buffer.extend_from_slice(&player_id.to_le_bytes());
                buffer.extend_from_slice(&score.to_le_bytes());
            }
        }

        debug!(
            size_bytes = buffer.len(),
            size_mb = buffer.len() / 1024 / 1024,
            "Replay saved to bytes"
        );

        Ok(buffer)
    }

    fn compress_data(data: &[u8]) -> Result<Vec<u8>, String> {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(data).map_err(|e| format!("Compression failed: {}", e))?;
        encoder.finish().map_err(|e| format!("Compression finish failed: {}", e))
    }
}

/// Replay player for playback
pub struct ReplayPlayer {
    frames: Vec<ReplayFrame>,
    current_frame_index: usize,
    playback_speed: f32,
    playing: bool,
}

impl ReplayPlayer {
    /// Load replay from bytes
    pub fn load_from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 13 + 8 + 8 {
            return Err("Invalid replay file: too small".to_string());
        }

        // Check header
        if &data[0..13] != b"AGE_REPLAY_V1" {
            return Err("Invalid replay file: bad header".to_string());
        }

        let mut cursor = 13;
        let record_interval = u32::from_le_bytes([
            data[cursor],
            data[cursor + 1],
            data[cursor + 2],
            data[cursor + 3],
        ]);
        cursor += 4;

        let frame_count = u64::from_le_bytes([
            data[cursor],
            data[cursor + 1],
            data[cursor + 2],
            data[cursor + 3],
            data[cursor + 4],
            data[cursor + 5],
            data[cursor + 6],
            data[cursor + 7],
        ]);
        cursor += 8;

        let mut frames = Vec::with_capacity(frame_count as usize);

        // Read frames
        for _ in 0..frame_count {
            if cursor + 8 + 8 + 4 > data.len() {
                return Err("Invalid replay file: unexpected EOF".to_string());
            }

            // Tick
            let tick = u64::from_le_bytes([
                data[cursor],
                data[cursor + 1],
                data[cursor + 2],
                data[cursor + 3],
                data[cursor + 4],
                data[cursor + 5],
                data[cursor + 6],
                data[cursor + 7],
            ]);
            cursor += 8;

            // Timestamp
            let timestamp_nanos = u64::from_le_bytes([
                data[cursor],
                data[cursor + 1],
                data[cursor + 2],
                data[cursor + 3],
                data[cursor + 4],
                data[cursor + 5],
                data[cursor + 6],
                data[cursor + 7],
            ]);
            cursor += 8;

            // World state
            let world_state_len = u32::from_le_bytes([
                data[cursor],
                data[cursor + 1],
                data[cursor + 2],
                data[cursor + 3],
            ]) as usize;
            cursor += 4;

            if cursor + world_state_len > data.len() {
                return Err("Invalid replay file: world state overflow".to_string());
            }

            let world_state = data[cursor..cursor + world_state_len].to_vec();
            cursor += world_state_len;

            // Inputs
            if cursor + 4 > data.len() {
                return Err("Invalid replay file: inputs count overflow".to_string());
            }

            let input_count = u32::from_le_bytes([
                data[cursor],
                data[cursor + 1],
                data[cursor + 2],
                data[cursor + 3],
            ]) as usize;
            cursor += 4;

            let mut inputs = Vec::with_capacity(input_count);
            for _ in 0..input_count {
                if cursor + 8 + 4 + 4 + 4 > data.len() {
                    return Err("Invalid replay file: input overflow".to_string());
                }

                let player_id = u64::from_le_bytes([
                    data[cursor],
                    data[cursor + 1],
                    data[cursor + 2],
                    data[cursor + 3],
                    data[cursor + 4],
                    data[cursor + 5],
                    data[cursor + 6],
                    data[cursor + 7],
                ]);
                cursor += 8;

                let input_bits = u32::from_le_bytes([
                    data[cursor],
                    data[cursor + 1],
                    data[cursor + 2],
                    data[cursor + 3],
                ]);
                cursor += 4;

                let yaw = f32::from_le_bytes([
                    data[cursor],
                    data[cursor + 1],
                    data[cursor + 2],
                    data[cursor + 3],
                ]);
                cursor += 4;

                let pitch = f32::from_le_bytes([
                    data[cursor],
                    data[cursor + 1],
                    data[cursor + 2],
                    data[cursor + 3],
                ]);
                cursor += 4;

                inputs.push(PlayerInput { player_id, input_bits, view_direction: [yaw, pitch] });
            }

            // Metadata
            if cursor + 4 > data.len() {
                return Err("Invalid replay file: metadata overflow".to_string());
            }

            let score_count = u32::from_le_bytes([
                data[cursor],
                data[cursor + 1],
                data[cursor + 2],
                data[cursor + 3],
            ]) as usize;
            cursor += 4;

            let mut scores = Vec::with_capacity(score_count);
            for _ in 0..score_count {
                if cursor + 8 + 4 > data.len() {
                    return Err("Invalid replay file: score overflow".to_string());
                }

                let player_id = u64::from_le_bytes([
                    data[cursor],
                    data[cursor + 1],
                    data[cursor + 2],
                    data[cursor + 3],
                    data[cursor + 4],
                    data[cursor + 5],
                    data[cursor + 6],
                    data[cursor + 7],
                ]);
                cursor += 8;

                let score = u32::from_le_bytes([
                    data[cursor],
                    data[cursor + 1],
                    data[cursor + 2],
                    data[cursor + 3],
                ]);
                cursor += 4;

                scores.push((player_id, score));
            }

            frames.push(ReplayFrame {
                tick,
                timestamp_nanos,
                world_state,
                inputs,
                metadata: FrameMetadata { scores, events: Vec::new(), custom: Vec::new() },
            });
        }

        info!(frame_count = frames.len(), record_interval, "Loaded replay from bytes");

        Ok(Self { frames, current_frame_index: 0, playback_speed: 1.0, playing: false })
    }

    /// Start playback
    pub fn play(&mut self) {
        self.playing = true;
        debug!("Started replay playback");
    }

    /// Pause playback
    pub fn pause(&mut self) {
        self.playing = false;
        debug!("Paused replay playback");
    }

    /// Set playback speed
    pub fn set_speed(&mut self, speed: f32) {
        self.playback_speed = speed.clamp(0.1, 10.0);
        debug!(speed = self.playback_speed, "Set replay playback speed");
    }

    /// Seek to tick
    pub fn seek_to_tick(&mut self, tick: u64) -> Result<(), String> {
        let frame_index = self
            .frames
            .iter()
            .position(|f| f.tick >= tick)
            .ok_or_else(|| format!("Tick {} not found in replay", tick))?;

        self.current_frame_index = frame_index;
        debug!(tick, frame_index, "Seeked to tick");
        Ok(())
    }

    /// Seek to time
    pub fn seek_to_time(&mut self, duration: Duration) -> Result<(), String> {
        let target_nanos = duration.as_nanos() as u64;
        let frame_index = self
            .frames
            .iter()
            .position(|f| f.timestamp_nanos >= target_nanos)
            .ok_or_else(|| format!("Time {:?} not found in replay", duration))?;

        self.current_frame_index = frame_index;
        debug!(time_secs = duration.as_secs(), frame_index, "Seeked to time");
        Ok(())
    }

    /// Get current frame
    pub fn current_frame(&self) -> Option<&ReplayFrame> {
        self.frames.get(self.current_frame_index)
    }

    /// Advance to next frame
    pub fn next_frame(&mut self) -> Option<&ReplayFrame> {
        if self.current_frame_index < self.frames.len() {
            self.current_frame_index += 1;
        }
        self.current_frame()
    }

    /// Get frame count
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get total duration
    pub fn duration(&self) -> Duration {
        self.frames
            .last()
            .map(|f| Duration::from_nanos(f.timestamp_nanos))
            .unwrap_or(Duration::ZERO)
    }

    /// Check if playback is finished
    pub fn is_finished(&self) -> bool {
        self.current_frame_index >= self.frames.len()
    }

    /// Get playback progress (0.0-1.0)
    pub fn progress(&self) -> f32 {
        if self.frames.is_empty() {
            return 1.0;
        }
        self.current_frame_index as f32 / self.frames.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_frame(tick: u64) -> ReplayFrame {
        ReplayFrame {
            tick,
            timestamp_nanos: tick * 16_666_666,
            world_state: vec![1, 2, 3, 4],
            inputs: vec![PlayerInput {
                player_id: 1,
                input_bits: 0xFF,
                view_direction: [1.0, 0.5],
            }],
            metadata: FrameMetadata { scores: vec![(1, 100)], events: vec![], custom: vec![] },
        }
    }

    #[test]
    fn test_recorder_basic() {
        let config = ReplayConfig::default();
        let mut recorder = ReplayRecorder::new(config);

        assert!(!recorder.is_recording());
        recorder.start_recording();
        assert!(recorder.is_recording());

        recorder.record_frame(vec![1, 2, 3], vec![], FrameMetadata::default()).unwrap();
        assert_eq!(recorder.frame_count(), 1);

        recorder.stop_recording();
        assert!(!recorder.is_recording());
    }

    #[test]
    fn test_recorder_interval() {
        let config = ReplayConfig {
            record_interval: 2, // Record every other tick
            ..Default::default()
        };
        let mut recorder = ReplayRecorder::new(config);
        recorder.start_recording();

        // Record 4 ticks, should only save 2 frames
        for _ in 0..4 {
            recorder.record_frame(vec![1, 2, 3], vec![], FrameMetadata::default()).unwrap();
        }

        assert_eq!(recorder.frame_count(), 2);
    }

    #[test]
    fn test_save_and_load() {
        let config = ReplayConfig::default();
        let mut recorder = ReplayRecorder::new(config);
        recorder.start_recording();

        // Record some frames
        for i in 0..10 {
            let frame = create_test_frame(i);
            recorder.record_frame(frame.world_state, frame.inputs, frame.metadata).unwrap();
        }

        // Save to bytes
        let bytes = recorder.save_to_bytes().unwrap();
        assert!(!bytes.is_empty());

        // Load from bytes
        let player = ReplayPlayer::load_from_bytes(&bytes).unwrap();
        assert_eq!(player.frame_count(), 10);
    }

    #[test]
    fn test_player_playback() {
        let config = ReplayConfig::default();
        let mut recorder = ReplayRecorder::new(config);
        recorder.start_recording();

        for i in 0..5 {
            let frame = create_test_frame(i);
            recorder.record_frame(frame.world_state, frame.inputs, frame.metadata).unwrap();
        }

        let bytes = recorder.save_to_bytes().unwrap();
        let mut player = ReplayPlayer::load_from_bytes(&bytes).unwrap();

        assert!(!player.is_finished());
        player.play();

        // Advance through frames
        for _ in 0..5 {
            assert!(player.current_frame().is_some());
            player.next_frame();
        }

        assert!(player.is_finished());
    }

    #[test]
    fn test_player_seeking() {
        let config = ReplayConfig::default();
        let mut recorder = ReplayRecorder::new(config);
        recorder.start_recording();

        for i in 0..10 {
            let frame = create_test_frame(i);
            recorder.record_frame(frame.world_state, frame.inputs, frame.metadata).unwrap();
        }

        let bytes = recorder.save_to_bytes().unwrap();
        let mut player = ReplayPlayer::load_from_bytes(&bytes).unwrap();

        // Seek to tick 5
        player.seek_to_tick(5).unwrap();
        assert_eq!(player.current_frame().unwrap().tick, 5);

        // Seek to time (use 0 since frames are recorded very quickly in tests)
        player.seek_to_time(Duration::ZERO).unwrap();
        assert_eq!(player.current_frame().unwrap().tick, 0);
    }

    #[test]
    fn test_playback_speed() {
        let config = ReplayConfig::default();
        let mut recorder = ReplayRecorder::new(config);
        recorder.start_recording();

        recorder.record_frame(vec![1, 2, 3], vec![], FrameMetadata::default()).unwrap();

        let bytes = recorder.save_to_bytes().unwrap();
        let mut player = ReplayPlayer::load_from_bytes(&bytes).unwrap();

        player.set_speed(2.0);
        player.set_speed(0.5);
        player.set_speed(100.0); // Should clamp to 10.0
    }

    #[test]
    fn test_progress() {
        let config = ReplayConfig::default();
        let mut recorder = ReplayRecorder::new(config);
        recorder.start_recording();

        for i in 0..10 {
            let frame = create_test_frame(i);
            recorder.record_frame(frame.world_state, frame.inputs, frame.metadata).unwrap();
        }

        let bytes = recorder.save_to_bytes().unwrap();
        let mut player = ReplayPlayer::load_from_bytes(&bytes).unwrap();

        assert_eq!(player.progress(), 0.0);

        for _ in 0..5 {
            player.next_frame();
        }

        assert!((player.progress() - 0.5).abs() < 0.1);
    }
}
