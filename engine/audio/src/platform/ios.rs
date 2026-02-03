//! iOS audio backend using Core Audio and AVFoundation
//!
//! This backend provides full 3D spatial audio support on iOS devices using:
//! - AVAudioEngine for audio playback and mixing
//! - AVAudio3DMixerNode for spatial audio positioning
//! - AVAudioPlayerNode for individual sound playback
//! - AVAudioSession for audio session management
//!
//! # Architecture
//!
//! The iOS backend manages:
//! - Audio session configuration (playback category, interruption handling)
//! - Main audio engine (AVAudioEngine)
//! - 3D mixer node for spatial audio
//! - Player nodes for each active sound
//! - Audio file buffers loaded into memory
//! - Emitter tracking for entity-based spatial audio

use crate::effects::AudioEffect;
use crate::error::{AudioError, AudioResult};
use crate::platform::AudioBackend;
use glam::Vec3;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

#[cfg(target_os = "ios")]
mod ffi {
    //! FFI bindings to iOS Core Audio and AVFoundation
    //!
    //! This module provides Rust bindings to the Objective-C APIs needed for audio.
    //! We use objc crate to call Objective-C methods from Rust.

    use block::ConcreteBlock;
    use objc::runtime::{Class, Object};
    use objc::{class, msg_send, sel, sel_impl};
    use objc_foundation::{INSString, NSString};
    use std::ffi::CString;
    use std::os::raw::c_void;
    use std::ptr;

    /// FFI representation of 3D position (AVAudio3DPoint)
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct AVAudio3DPoint {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    impl AVAudio3DPoint {
        pub fn new(x: f32, y: f32, z: f32) -> Self {
            Self { x, y, z }
        }
    }

    /// FFI representation of 3D vector orientation
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct AVAudio3DVectorOrientation {
        pub forward: AVAudio3DPoint,
        pub up: AVAudio3DPoint,
    }

    /// Audio session wrapper
    pub struct AudioSession {
        session: *mut Object,
    }

    unsafe impl Send for AudioSession {}
    unsafe impl Sync for AudioSession {}

    impl AudioSession {
        pub fn shared() -> Result<Self, String> {
            unsafe {
                let session_class = class!(AVAudioSession);
                let session: *mut Object = msg_send![session_class, sharedInstance];
                if session.is_null() {
                    return Err("Failed to get AVAudioSession".to_string());
                }
                Ok(Self { session })
            }
        }

        pub fn set_category(&self, category: &str) -> Result<(), String> {
            unsafe {
                let category_str = NSString::from_str(category);
                let mut error: *mut Object = ptr::null_mut();
                let success: bool =
                    msg_send![self.session, setCategory:category_str.as_ptr() error:&mut error];
                if !success {
                    return Err("Failed to set audio session category".to_string());
                }
                Ok(())
            }
        }

        pub fn set_active(&self, active: bool) -> Result<(), String> {
            unsafe {
                let mut error: *mut Object = ptr::null_mut();
                let success: bool = msg_send![self.session, setActive:active error:&mut error];
                if !success {
                    return Err("Failed to activate audio session".to_string());
                }
                Ok(())
            }
        }
    }

    impl Drop for AudioSession {
        fn drop(&mut self) {
            // AVAudioSession is managed by the system, no manual cleanup needed
        }
    }

    /// AVAudioEngine wrapper
    pub struct AudioEngine {
        engine: *mut Object,
    }

    unsafe impl Send for AudioEngine {}
    unsafe impl Sync for AudioEngine {}

    impl AudioEngine {
        pub fn new() -> Result<Self, String> {
            unsafe {
                let engine_class = class!(AVAudioEngine);
                let engine: *mut Object = msg_send![engine_class, alloc];
                let engine: *mut Object = msg_send![engine, init];
                if engine.is_null() {
                    return Err("Failed to create AVAudioEngine".to_string());
                }
                Ok(Self { engine })
            }
        }

        pub fn main_mixer_node(&self) -> *mut Object {
            unsafe { msg_send![self.engine, mainMixerNode] }
        }

        pub fn attach(&self, node: *mut Object) {
            unsafe {
                let _: () = msg_send![self.engine, attachNode: node];
            }
        }

        pub fn detach(&self, node: *mut Object) {
            unsafe {
                let _: () = msg_send![self.engine, detachNode: node];
            }
        }

        pub fn connect(
            &self,
            source: *mut Object,
            destination: *mut Object,
            format: *mut Object,
        ) -> Result<(), String> {
            unsafe {
                let _: () = msg_send![
                    self.engine,
                    connect:source
                    to:destination
                    format:format
                ];
                Ok(())
            }
        }

        pub fn start(&self) -> Result<(), String> {
            unsafe {
                let mut error: *mut Object = ptr::null_mut();
                let success: bool = msg_send![self.engine, startAndReturnError:&mut error];
                if !success {
                    return Err("Failed to start audio engine".to_string());
                }
                Ok(())
            }
        }

        pub fn stop(&self) {
            unsafe {
                let _: () = msg_send![self.engine, stop];
            }
        }

        pub fn as_ptr(&self) -> *mut Object {
            self.engine
        }
    }

    impl Drop for AudioEngine {
        fn drop(&mut self) {
            self.stop();
            unsafe {
                let _: () = msg_send![self.engine, release];
            }
        }
    }

    /// AVAudioPlayerNode wrapper
    pub struct PlayerNode {
        node: *mut Object,
    }

    unsafe impl Send for PlayerNode {}
    unsafe impl Sync for PlayerNode {}

    impl PlayerNode {
        pub fn new() -> Result<Self, String> {
            unsafe {
                let node_class = class!(AVAudioPlayerNode);
                let node: *mut Object = msg_send![node_class, alloc];
                let node: *mut Object = msg_send![node, init];
                if node.is_null() {
                    return Err("Failed to create AVAudioPlayerNode".to_string());
                }
                Ok(Self { node })
            }
        }

        pub fn schedule_buffer(&self, buffer: *mut Object, looping: bool) {
            unsafe {
                if looping {
                    let _: () = msg_send![self.node, scheduleBuffer:buffer atTime:ptr::null::<Object>() options:1u32 completionHandler:ptr::null::<c_void>()];
                } else {
                    let _: () = msg_send![self.node, scheduleBuffer:buffer atTime:ptr::null::<Object>() options:0u32 completionHandler:ptr::null::<c_void>()];
                }
            }
        }

        pub fn play(&self) {
            unsafe {
                let _: () = msg_send![self.node, play];
            }
        }

        pub fn stop(&self) {
            unsafe {
                let _: () = msg_send![self.node, stop];
            }
        }

        pub fn is_playing(&self) -> bool {
            unsafe { msg_send![self.node, isPlaying] }
        }

        pub fn volume(&self) -> f32 {
            unsafe { msg_send![self.node, volume] }
        }

        pub fn set_volume(&self, volume: f32) {
            unsafe {
                let _: () = msg_send![self.node, setVolume: volume];
            }
        }

        pub fn as_ptr(&self) -> *mut Object {
            self.node
        }
    }

    impl Drop for PlayerNode {
        fn drop(&mut self) {
            self.stop();
            unsafe {
                let _: () = msg_send![self.node, release];
            }
        }
    }

    /// AVAudio3DMixerNode wrapper for spatial audio
    pub struct Audio3DMixerNode {
        node: *mut Object,
    }

    unsafe impl Send for Audio3DMixerNode {}
    unsafe impl Sync for Audio3DMixerNode {}

    impl Audio3DMixerNode {
        pub fn new() -> Result<Self, String> {
            unsafe {
                let node_class = class!(AVAudio3DMixerNode);
                let node: *mut Object = msg_send![node_class, alloc];
                let node: *mut Object = msg_send![node, init];
                if node.is_null() {
                    return Err("Failed to create AVAudio3DMixerNode".to_string());
                }
                Ok(Self { node })
            }
        }

        pub fn set_listener_position(&self, position: AVAudio3DPoint) {
            unsafe {
                let _: () = msg_send![self.node, setListenerPosition: position];
            }
        }

        pub fn set_listener_orientation(&self, orientation: AVAudio3DVectorOrientation) {
            unsafe {
                let _: () = msg_send![self.node, setListenerAngularOrientation: orientation];
            }
        }

        pub fn set_source_position(&self, bus: u32, position: AVAudio3DPoint) {
            unsafe {
                let _: () = msg_send![self.node, setPosition:position forBus:bus];
            }
        }

        pub fn set_distance_attenuation_parameters(
            &self,
            bus: u32,
            ref_distance: f32,
            max_distance: f32,
            rolloff_factor: f32,
        ) {
            unsafe {
                // Get the distance parameters object for this bus
                let params: *mut Object =
                    msg_send![self.node, distanceAttenuationParametersForBus: bus];
                if !params.is_null() {
                    let _: () = msg_send![params, setReferenceDistance: ref_distance];
                    let _: () = msg_send![params, setMaximumDistance: max_distance];
                    let _: () = msg_send![params, setRolloffFactor: rolloff_factor];
                }
            }
        }

        pub fn as_ptr(&self) -> *mut Object {
            self.node
        }
    }

    impl Drop for Audio3DMixerNode {
        fn drop(&mut self) {
            unsafe {
                let _: () = msg_send![self.node, release];
            }
        }
    }

    /// AVAudioPCMBuffer wrapper
    pub struct AudioBuffer {
        buffer: *mut Object,
    }

    unsafe impl Send for AudioBuffer {}
    unsafe impl Sync for AudioBuffer {}

    impl AudioBuffer {
        pub fn from_file(path: &str) -> Result<Self, String> {
            unsafe {
                // Create NSURL from file path
                let url_class = class!(NSURL);
                let path_str = NSString::from_str(path);
                let url: *mut Object = msg_send![url_class, fileURLWithPath: path_str.as_ptr()];
                if url.is_null() {
                    return Err(format!("Failed to create URL for path: {}", path));
                }

                // Create AVAudioFile
                let file_class = class!(AVAudioFile);
                let mut error: *mut Object = ptr::null_mut();
                let audio_file: *mut Object = msg_send![file_class, alloc];
                let audio_file: *mut Object =
                    msg_send![audio_file, initForReading:url error:&mut error];
                if audio_file.is_null() {
                    return Err(format!("Failed to load audio file: {}", path));
                }

                // Get audio format and length
                let format: *mut Object = msg_send![audio_file, processingFormat];
                let length: u32 = msg_send![audio_file, length];

                // Create buffer
                let buffer_class = class!(AVAudioPCMBuffer);
                let buffer: *mut Object = msg_send![buffer_class, alloc];
                let buffer: *mut Object =
                    msg_send![buffer, initWithPCMFormat:format frameCapacity:length];
                if buffer.is_null() {
                    let _: () = msg_send![audio_file, release];
                    return Err("Failed to create audio buffer".to_string());
                }

                // Read audio data into buffer
                let success: bool = msg_send![audio_file, readIntoBuffer:buffer error:&mut error];
                if !success {
                    let _: () = msg_send![buffer, release];
                    let _: () = msg_send![audio_file, release];
                    return Err("Failed to read audio data".to_string());
                }

                // Clean up file (we have the data in buffer now)
                let _: () = msg_send![audio_file, release];

                Ok(Self { buffer })
            }
        }

        pub fn as_ptr(&self) -> *mut Object {
            self.buffer
        }

        pub fn format(&self) -> *mut Object {
            unsafe { msg_send![self.buffer, format] }
        }
    }

    impl Drop for AudioBuffer {
        fn drop(&mut self) {
            unsafe {
                let _: () = msg_send![self.buffer, release];
            }
        }
    }

    impl Clone for AudioBuffer {
        fn clone(&self) -> Self {
            unsafe {
                // Retain the buffer for the new reference
                let _: () = msg_send![self.buffer, retain];
                Self { buffer: self.buffer }
            }
        }
    }
}

#[cfg(not(target_os = "ios"))]
mod ffi {
    //! Mock FFI for non-iOS platforms (for testing/compilation)
    use super::*;

    #[derive(Debug, Clone, Copy)]
    pub struct AVAudio3DPoint {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    impl AVAudio3DPoint {
        pub fn new(x: f32, y: f32, z: f32) -> Self {
            Self { x, y, z }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct AVAudio3DVectorOrientation {
        pub forward: AVAudio3DPoint,
        pub up: AVAudio3DPoint,
    }

    pub struct AudioSession;
    impl AudioSession {
        pub fn shared() -> Result<Self, String> {
            Err("iOS audio only available on iOS".to_string())
        }
        pub fn set_category(&self, _category: &str) -> Result<(), String> {
            Ok(())
        }
        pub fn set_active(&self, _active: bool) -> Result<(), String> {
            Ok(())
        }
    }

    pub struct AudioEngine;
    impl AudioEngine {
        pub fn new() -> Result<Self, String> {
            Err("iOS audio only available on iOS".to_string())
        }
        pub fn main_mixer_node(&self) -> usize {
            0
        }
        pub fn attach(&self, _node: usize) {}
        pub fn detach(&self, _node: usize) {}
        pub fn connect(
            &self,
            _source: usize,
            _destination: usize,
            _format: usize,
        ) -> Result<(), String> {
            Ok(())
        }
        pub fn start(&self) -> Result<(), String> {
            Ok(())
        }
        pub fn stop(&self) {}
        pub fn as_ptr(&self) -> usize {
            0
        }
    }

    pub struct PlayerNode;
    impl PlayerNode {
        pub fn new() -> Result<Self, String> {
            Err("iOS audio only available on iOS".to_string())
        }
        pub fn schedule_buffer(&self, _buffer: usize, _looping: bool) {}
        pub fn play(&self) {}
        pub fn stop(&self) {}
        pub fn is_playing(&self) -> bool {
            false
        }
        pub fn volume(&self) -> f32 {
            1.0
        }
        pub fn set_volume(&self, _volume: f32) {}
        pub fn as_ptr(&self) -> usize {
            0
        }
    }

    pub struct Audio3DMixerNode;
    impl Audio3DMixerNode {
        pub fn new() -> Result<Self, String> {
            Err("iOS audio only available on iOS".to_string())
        }
        pub fn set_listener_position(&self, _position: AVAudio3DPoint) {}
        pub fn set_listener_orientation(&self, _orientation: AVAudio3DVectorOrientation) {}
        pub fn set_source_position(&self, _bus: u32, _position: AVAudio3DPoint) {}
        pub fn set_distance_attenuation_parameters(
            &self,
            _bus: u32,
            _ref_distance: f32,
            _max_distance: f32,
            _rolloff_factor: f32,
        ) {
        }
        pub fn as_ptr(&self) -> usize {
            0
        }
    }

    #[derive(Clone)]
    pub struct AudioBuffer;
    impl AudioBuffer {
        pub fn from_file(_path: &str) -> Result<Self, String> {
            Err("iOS audio only available on iOS".to_string())
        }
        pub fn as_ptr(&self) -> usize {
            0
        }
        pub fn format(&self) -> usize {
            0
        }
    }
}

/// Sound instance tracking
struct SoundInstance {
    player: Arc<ffi::PlayerNode>,
    is_3d: bool,
    entity_id: Option<u32>,
    bus: Option<u32>,
}

/// Spatial emitter for 3D audio
struct Emitter {
    bus: u32,
    position: Vec3,
}

/// iOS Core Audio backend implementation
pub struct IOSAudioBackend {
    /// Audio session (manages system audio)
    session: Option<ffi::AudioSession>,

    /// Main audio engine
    engine: Option<ffi::AudioEngine>,

    /// 3D mixer node for spatial audio
    mixer_3d: Option<Arc<ffi::Audio3DMixerNode>>,

    /// Loaded sound buffers (cached by name)
    loaded_sounds: HashMap<String, Arc<ffi::AudioBuffer>>,

    /// Active sound instances (keyed by instance ID)
    active_sounds: HashMap<u64, SoundInstance>,

    /// Spatial emitters per entity
    emitters: HashMap<u32, Emitter>,

    /// Next instance ID
    next_instance_id: u64,

    /// Next mixer bus ID for 3D sounds
    next_bus_id: u32,

    /// Listener position and orientation
    listener_position: Vec3,
    listener_forward: Vec3,
    listener_up: Vec3,
}

impl IOSAudioBackend {
    /// Initialize the audio session with optimal settings for gaming
    fn initialize_audio_session() -> AudioResult<ffi::AudioSession> {
        let session = ffi::AudioSession::shared()
            .map_err(|e| AudioError::ManagerError(format!("Failed to get audio session: {}", e)))?;

        // Set playback category for gaming audio
        // AVAudioSessionCategoryPlayback allows background audio and respects silent switch
        session.set_category("AVAudioSessionCategoryPlayback").map_err(|e| {
            AudioError::ManagerError(format!("Failed to set audio category: {}", e))
        })?;

        // Activate the session
        session.set_active(true).map_err(|e| {
            AudioError::ManagerError(format!("Failed to activate audio session: {}", e))
        })?;

        info!("iOS audio session initialized");

        Ok(session)
    }
}

impl AudioBackend for IOSAudioBackend {
    fn new() -> AudioResult<Self> {
        #[cfg(not(target_os = "ios"))]
        {
            return Err(AudioError::ManagerError(
                "iOS audio backend only available on iOS".to_string(),
            ));
        }

        #[cfg(target_os = "ios")]
        {
            // Initialize audio session
            let session = Self::initialize_audio_session()?;

            // Create audio engine
            let engine = ffi::AudioEngine::new().map_err(|e| {
                AudioError::ManagerError(format!("Failed to create audio engine: {}", e))
            })?;

            // Create 3D mixer node
            let mixer_3d = Arc::new(ffi::Audio3DMixerNode::new().map_err(|e| {
                AudioError::ManagerError(format!("Failed to create 3D mixer node: {}", e))
            })?);

            // Attach mixer to engine
            engine.attach(mixer_3d.as_ptr());

            // Connect mixer to main mixer node
            let main_mixer = engine.main_mixer_node();
            // Get format from main mixer
            #[cfg(target_os = "ios")]
            let format = unsafe {
                use objc::{msg_send, sel, sel_impl};
                let format: *mut objc::runtime::Object =
                    msg_send![mixer_3d.as_ptr(), outputFormatForBus: 0u32];
                format
            };
            #[cfg(not(target_os = "ios"))]
            let format = 0;

            engine
                .connect(mixer_3d.as_ptr(), main_mixer, format)
                .map_err(|e| AudioError::ManagerError(format!("Failed to connect mixer: {}", e)))?;

            // Start the engine
            engine.start().map_err(|e| {
                AudioError::ManagerError(format!("Failed to start audio engine: {}", e))
            })?;

            info!("iOS audio backend initialized successfully");

            Ok(Self {
                session: Some(session),
                engine: Some(engine),
                mixer_3d: Some(mixer_3d),
                loaded_sounds: HashMap::new(),
                active_sounds: HashMap::new(),
                emitters: HashMap::new(),
                next_instance_id: 0,
                next_bus_id: 0,
                listener_position: Vec3::ZERO,
                listener_forward: Vec3::new(0.0, 0.0, -1.0),
                listener_up: Vec3::new(0.0, 1.0, 0.0),
            })
        }
    }

    fn load_sound(&mut self, name: &str, path: &Path) -> AudioResult<()> {
        if self.loaded_sounds.contains_key(name) {
            debug!("Sound already loaded: {}", name);
            return Ok(());
        }

        let path_str = path.to_str().ok_or_else(|| {
            AudioError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path"))
        })?;

        let buffer = ffi::AudioBuffer::from_file(path_str)
            .map_err(|e| AudioError::DecodeError(format!("Failed to load audio: {}", e)))?;

        self.loaded_sounds.insert(name.to_string(), Arc::new(buffer));

        info!("Loaded sound: {} from {}", name, path_str);

        Ok(())
    }

    fn play_2d(&mut self, sound_name: &str, volume: f32, looping: bool) -> AudioResult<u64> {
        let buffer = self
            .loaded_sounds
            .get(sound_name)
            .ok_or_else(|| AudioError::SoundNotFound(sound_name.to_string()))?
            .clone();

        let engine = self
            .engine
            .as_ref()
            .ok_or_else(|| AudioError::ManagerError("Engine not initialized".to_string()))?;

        // Create player node
        let player = ffi::PlayerNode::new().map_err(|e| {
            AudioError::ManagerError(format!("Failed to create player node: {}", e))
        })?;

        player.set_volume(volume);

        // Attach player to engine
        engine.attach(player.as_ptr());

        // Connect player to main mixer
        let main_mixer = engine.main_mixer_node();
        engine
            .connect(player.as_ptr(), main_mixer, buffer.format())
            .map_err(|e| AudioError::ManagerError(format!("Failed to connect player: {}", e)))?;

        // Schedule buffer
        player.schedule_buffer(buffer.as_ptr(), looping);

        // Start playback
        player.play();

        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;

        self.active_sounds.insert(
            instance_id,
            SoundInstance { player: Arc::new(player), is_3d: false, entity_id: None, bus: None },
        );

        debug!("Playing 2D sound: {} (id: {})", sound_name, instance_id);

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

        let engine = self
            .engine
            .as_ref()
            .ok_or_else(|| AudioError::ManagerError("Engine not initialized".to_string()))?;

        let mixer_3d = self
            .mixer_3d
            .as_ref()
            .ok_or_else(|| AudioError::ManagerError("3D mixer not initialized".to_string()))?
            .clone();

        // Allocate bus for this sound
        let bus = self.next_bus_id;
        self.next_bus_id += 1;

        // Create player node
        let player = ffi::PlayerNode::new().map_err(|e| {
            AudioError::ManagerError(format!("Failed to create player node: {}", e))
        })?;

        player.set_volume(volume);

        // Attach player to engine
        engine.attach(player.as_ptr());

        // Connect player to 3D mixer on the allocated bus
        engine
            .connect(player.as_ptr(), mixer_3d.as_ptr(), buffer.format())
            .map_err(|e| AudioError::ManagerError(format!("Failed to connect player: {}", e)))?;

        // Set 3D position on mixer bus
        let av_position = ffi::AVAudio3DPoint::new(position.x, position.y, position.z);
        mixer_3d.set_source_position(bus, av_position);

        // Set distance attenuation parameters
        let ref_distance = 1.0;
        let rolloff_factor = 1.0;
        mixer_3d.set_distance_attenuation_parameters(
            bus,
            ref_distance,
            max_distance,
            rolloff_factor,
        );

        // Schedule buffer
        player.schedule_buffer(buffer.as_ptr(), looping);

        // Start playback
        player.play();

        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;

        // Track emitter
        if !self.emitters.contains_key(&entity) {
            self.emitters.insert(entity, Emitter { bus, position });
        }

        self.active_sounds.insert(
            instance_id,
            SoundInstance {
                player: Arc::new(player),
                is_3d: true,
                entity_id: Some(entity),
                bus: Some(bus),
            },
        );

        debug!(
            "Playing 3D sound: {} at {:?} (id: {}, bus: {})",
            sound_name, position, instance_id, bus
        );

        Ok(instance_id)
    }

    fn stop(&mut self, instance_id: u64, fade_out_duration: Option<f32>) {
        if let Some(instance) = self.active_sounds.remove(&instance_id) {
            // Note: AVAudioPlayerNode doesn't support fade out directly
            // For production, we'd need to use AVAudioUnitEQ or similar for fading
            if fade_out_duration.is_some() {
                warn!("Fade out not yet implemented for iOS backend");
            }

            instance.player.stop();

            if let Some(engine) = &self.engine {
                engine.detach(instance.player.as_ptr());
            }

            debug!("Stopped sound (id: {})", instance_id);
        }
    }

    fn set_listener_transform(&mut self, position: Vec3, forward: Vec3, up: Vec3) {
        self.listener_position = position;
        self.listener_forward = forward;
        self.listener_up = up;

        if let Some(mixer_3d) = &self.mixer_3d {
            let av_position = ffi::AVAudio3DPoint::new(position.x, position.y, position.z);
            mixer_3d.set_listener_position(av_position);

            let orientation = ffi::AVAudio3DVectorOrientation {
                forward: ffi::AVAudio3DPoint::new(forward.x, forward.y, forward.z),
                up: ffi::AVAudio3DPoint::new(up.x, up.y, up.z),
            };
            mixer_3d.set_listener_orientation(orientation);
        }
    }

    fn update_emitter_position(&mut self, entity: u32, position: Vec3) {
        if let Some(emitter) = self.emitters.get_mut(&entity) {
            emitter.position = position;

            if let Some(mixer_3d) = &self.mixer_3d {
                let av_position = ffi::AVAudio3DPoint::new(position.x, position.y, position.z);
                mixer_3d.set_source_position(emitter.bus, av_position);
            }
        }
    }

    fn remove_emitter(&mut self, entity: u32) {
        if self.emitters.remove(&entity).is_some() {
            debug!("Removed emitter for entity {}", entity);
        }
    }

    fn is_playing(&self, instance_id: u64) -> bool {
        if let Some(instance) = self.active_sounds.get(&instance_id) {
            instance.player.is_playing()
        } else {
            false
        }
    }

    fn cleanup_finished(&mut self) {
        let initial_count = self.active_sounds.len();

        self.active_sounds.retain(|id, instance| {
            let is_playing = instance.player.is_playing();
            if !is_playing {
                // Detach player node from engine
                if let Some(engine) = &self.engine {
                    engine.detach(instance.player.as_ptr());
                }
                debug!("Cleaned up finished sound (id: {})", id);
            }
            is_playing
        });

        let cleaned = initial_count - self.active_sounds.len();
        if cleaned > 0 {
            debug!("Cleaned up {} finished sounds", cleaned);
        }
    }

    fn active_sound_count(&self) -> usize {
        self.active_sounds.len()
    }

    fn loaded_sound_count(&self) -> usize {
        self.loaded_sounds.len()
    }

    fn play_stream(&mut self, path: &Path, volume: f32, looping: bool) -> AudioResult<u64> {
        // For iOS, we use the same mechanism as regular sounds
        // AVAudioFile handles streaming automatically for large files
        let path_str = path.to_str().ok_or_else(|| {
            AudioError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path"))
        })?;

        let buffer = ffi::AudioBuffer::from_file(path_str)
            .map_err(|e| AudioError::DecodeError(format!("Failed to load audio: {}", e)))?;

        let engine = self
            .engine
            .as_ref()
            .ok_or_else(|| AudioError::ManagerError("Engine not initialized".to_string()))?;

        let player = ffi::PlayerNode::new().map_err(|e| {
            AudioError::ManagerError(format!("Failed to create player node: {}", e))
        })?;

        player.set_volume(volume);

        engine.attach(player.as_ptr());

        let main_mixer = engine.main_mixer_node();
        engine
            .connect(player.as_ptr(), main_mixer, buffer.format())
            .map_err(|e| AudioError::ManagerError(format!("Failed to connect player: {}", e)))?;

        player.schedule_buffer(buffer.as_ptr(), looping);
        player.play();

        let instance_id = self.next_instance_id;
        self.next_instance_id += 1;

        self.active_sounds.insert(
            instance_id,
            SoundInstance { player: Arc::new(player), is_3d: false, entity_id: None, bus: None },
        );

        debug!("Streaming audio from {} (id: {})", path_str, instance_id);

        Ok(instance_id)
    }

    fn add_effect(&mut self, _instance_id: u64, _effect: AudioEffect) -> AudioResult<usize> {
        // TODO: Implement iOS audio effects using AVAudioUnitEffect
        // - AVAudioUnitReverb for reverb
        // - AVAudioUnitDelay for echo
        // - AVAudioUnitEQ for equalization
        // - AVAudioUnitDistortion for filter effects
        warn!("Audio effects not yet implemented for iOS backend");
        Err(AudioError::EffectError("Effects not yet implemented for iOS".to_string()))
    }

    fn remove_effect(&mut self, _instance_id: u64, _effect_index: usize) -> bool {
        warn!("Audio effects not yet implemented for iOS backend");
        false
    }

    fn clear_effects(&mut self, _instance_id: u64) {
        // No-op for now
    }

    fn effect_count(&self, _instance_id: u64) -> usize {
        0
    }

    fn set_pitch(&mut self, instance_id: u64, pitch: f32) {
        // iOS backend uses Core Audio's AVAudioEngine
        // Pitch shifting can be implemented using AVAudioUnitTimePitch
        // For now, we'll log and plan for future implementation

        // TODO: Implement pitch shifting using AVAudioUnitTimePitch
        // - Create AVAudioUnitTimePitch unit
        // - Set pitch property (in cents, where 100 cents = 1 semitone)
        // - Insert into audio processing graph

        debug!(
            instance_id = instance_id,
            pitch = pitch,
            "Pitch shifting requested (not yet fully implemented for iOS)"
        );

        warn!("Pitch shifting not yet fully implemented for iOS backend");
    }
}

impl Drop for IOSAudioBackend {
    fn drop(&mut self) {
        // Stop all active sounds
        for (id, instance) in self.active_sounds.drain() {
            instance.player.stop();
            if let Some(engine) = &self.engine {
                engine.detach(instance.player.as_ptr());
            }
            debug!("Stopped sound {} during cleanup", id);
        }

        // Stop engine
        if let Some(engine) = &self.engine {
            if let Some(mixer_3d) = &self.mixer_3d {
                engine.detach(mixer_3d.as_ptr());
            }
            engine.stop();
        }

        // Deactivate audio session
        if let Some(session) = &self.session {
            let _ = session.set_active(false);
        }

        info!("iOS audio backend shut down");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ios_backend_creation() {
        // This will only succeed on iOS
        let result = IOSAudioBackend::new();

        #[cfg(target_os = "ios")]
        assert!(result.is_ok(), "Failed to create iOS audio backend");

        #[cfg(not(target_os = "ios"))]
        assert!(result.is_err(), "iOS backend should fail on non-iOS platforms");
    }

    #[test]
    fn test_sound_tracking() {
        // Test that we can track multiple sounds correctly
        #[cfg(not(target_os = "ios"))]
        {
            // On non-iOS, just verify the struct compiles
            let backend = IOSAudioBackend {
                session: None,
                engine: None,
                mixer_3d: None,
                loaded_sounds: HashMap::new(),
                active_sounds: HashMap::new(),
                emitters: HashMap::new(),
                next_instance_id: 0,
                next_bus_id: 0,
                listener_position: Vec3::ZERO,
                listener_forward: Vec3::new(0.0, 0.0, -1.0),
                listener_up: Vec3::new(0.0, 1.0, 0.0),
            };

            assert_eq!(backend.active_sound_count(), 0);
            assert_eq!(backend.loaded_sound_count(), 0);
        }
    }

    #[test]
    fn test_emitter_management() {
        #[cfg(not(target_os = "ios"))]
        {
            let mut backend = IOSAudioBackend {
                session: None,
                engine: None,
                mixer_3d: None,
                loaded_sounds: HashMap::new(),
                active_sounds: HashMap::new(),
                emitters: HashMap::new(),
                next_instance_id: 0,
                next_bus_id: 0,
                listener_position: Vec3::ZERO,
                listener_forward: Vec3::new(0.0, 0.0, -1.0),
                listener_up: Vec3::new(0.0, 1.0, 0.0),
            };

            // Simulate emitter creation
            backend.emitters.insert(1, Emitter { bus: 0, position: Vec3::ZERO });

            assert_eq!(backend.emitters.len(), 1);

            backend.remove_emitter(1);
            assert_eq!(backend.emitters.len(), 0);
        }
    }

    #[test]
    fn test_listener_transform() {
        #[cfg(not(target_os = "ios"))]
        {
            let mut backend = IOSAudioBackend {
                session: None,
                engine: None,
                mixer_3d: None,
                loaded_sounds: HashMap::new(),
                active_sounds: HashMap::new(),
                emitters: HashMap::new(),
                next_instance_id: 0,
                next_bus_id: 0,
                listener_position: Vec3::ZERO,
                listener_forward: Vec3::new(0.0, 0.0, -1.0),
                listener_up: Vec3::new(0.0, 1.0, 0.0),
            };

            let pos = Vec3::new(1.0, 2.0, 3.0);
            let forward = Vec3::new(0.0, 0.0, -1.0);
            let up = Vec3::new(0.0, 1.0, 0.0);

            backend.set_listener_transform(pos, forward, up);

            assert_eq!(backend.listener_position, pos);
            assert_eq!(backend.listener_forward, forward);
            assert_eq!(backend.listener_up, up);
        }
    }
}
