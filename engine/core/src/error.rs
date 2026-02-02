//! Error infrastructure for the agent game engine.
//!
//! This module provides the foundation for structured error handling across the entire engine.
//! All engine errors implement the `EngineError` trait, which provides:
//! - Error codes for programmatic handling
//! - Severity levels for filtering and alerting
//! - Automatic structured logging via tracing
//!
//! # Error Code Ranges
//!
//! Error codes are organized by subsystem:
//! - 1000-1099: Core ECS
//! - 1100-1199: Serialization
//! - 1200-1299: Platform
//! - 1300-1399: Rendering
//! - 1400-1499: Networking
//! - 1500-1599: Physics
//! - 1600-1699: Audio
//! - 1700-1799: LOD
//! - 1800-1899: Interest Management
//! - 1900-1999: Auto-update

use std::fmt;

/// Error codes organized by subsystem.
///
/// Each subsystem has a range of 100 codes. This allows for programmatic
/// error handling and monitoring/alerting based on error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ErrorCode {
    // Core ECS (1000-1099)
    /// Entity not found in the world
    EntityNotFound = 1000,
    /// Component not found on entity
    ComponentNotFound = 1001,
    /// Archetype not found in world storage
    ArchetypeNotFound = 1002,
    /// Invalid entity ID provided
    InvalidEntityId = 1003,
    /// Component already exists on entity
    ComponentAlreadyExists = 1004,

    // Serialization (1100-1199)
    /// General serialization failure
    SerializationFailed = 1100,
    /// General deserialization failure
    DeserializationFailed = 1101,
    /// YAML serialization failed
    YamlSerializeFailed = 1102,
    /// YAML deserialization failed
    YamlDeserializeFailed = 1103,
    /// Bincode serialization failed
    BincodeSerializeFailed = 1104,
    /// Bincode deserialization failed
    BincodeDeserializeFailed = 1105,
    /// FlatBuffers serialization failed
    FlatbuffersSerializeFailed = 1106,
    /// FlatBuffers deserialization failed
    FlatbuffersDeserializeFailed = 1107,
    /// Invalid data format
    InvalidFormat = 1108,
    /// Version mismatch in serialized data
    VersionMismatch = 1109,

    // Platform (1200-1299)
    /// Window creation failed
    WindowCreationFailed = 1200,
    /// Vulkan surface creation failed
    SurfaceCreationFailed = 1201,
    /// Input system initialization failed
    InputInitFailed = 1202,
    /// Time system initialization failed
    TimeInitFailed = 1203,
    /// Filesystem operation failed
    FileSystemError = 1204,
    /// Threading operation failed
    ThreadingError = 1205,
    /// Platform not supported
    PlatformNotSupported = 1206,

    // Rendering (1300-1399)
    /// Vulkan initialization failed
    VulkanInitFailed = 1300,
    /// Vulkan instance creation failed
    InstanceCreationFailed = 1301,
    /// No suitable GPU found
    NoSuitableGpu = 1302,
    /// Physical device enumeration failed
    DeviceEnumerationFailed = 1303,
    /// Logical device creation failed
    LogicalDeviceCreationFailed = 1304,
    /// Queue family not found
    QueueFamilyNotFound = 1305,
    /// Extension not supported
    ExtensionNotSupported = 1306,
    /// Validation layer not available
    ValidationLayerNotAvailable = 1307,
    /// Debug messenger creation failed
    DebugMessengerCreationFailed = 1308,
    /// Memory allocation failed
    MemoryAllocationFailed = 1309,
    /// Buffer creation failed
    BufferCreationFailed = 1310,
    /// Image creation failed
    ImageCreationFailed = 1311,
    /// Image view creation failed
    ImageViewCreationFailed = 1312,
    /// Swapchain creation failed
    SwapchainCreationFailed = 1313,
    /// Surface creation failed (moved from Platform)
    SurfaceCreationFailedRenderer = 1314,
    /// Surface capabilities query failed
    SurfaceCapabilitiesQueryFailed = 1315,
    /// Surface format query failed
    SurfaceFormatQueryFailed = 1316,
    /// Present mode query failed
    PresentModeQueryFailed = 1317,
    /// Command pool creation failed
    CommandPoolCreationFailed = 1318,
    /// Command buffer allocation failed
    CommandBufferAllocationFailed = 1319,
    /// Shader compilation failed
    ShaderCompileFailed = 1320,
    /// Shader module creation failed
    ShaderModuleCreationFailed = 1321,
    /// Pipeline creation failed
    PipelineCreationFailed = 1322,
    /// Pipeline cache creation failed
    PipelineCacheCreationFailed = 1323,
    /// Render pass creation failed
    RenderPassCreationFailed = 1324,
    /// Framebuffer creation failed
    FramebufferCreationFailed = 1325,
    /// Descriptor set layout creation failed
    DescriptorSetLayoutCreationFailed = 1326,
    /// Descriptor pool creation failed
    DescriptorPoolCreationFailed = 1327,
    /// Descriptor set allocation failed
    DescriptorSetAllocationFailed = 1328,
    /// Texture loading failed
    TextureLoadFailed = 1329,
    /// Mesh loading failed
    MeshLoadFailed = 1330,
    /// Material loading failed
    MaterialLoadFailed = 1331,
    /// Asset loading failed (general)
    AssetLoadFailed = 1339,
    /// Synchronization object creation failed
    SyncObjectCreationFailed = 1340,
    /// GPU memory mapping failed
    MemoryMappingFailed = 1341,
    /// Command buffer recording failed
    CommandBufferRecordingFailed = 1333,
    /// Queue submission failed
    QueueSubmissionFailed = 1334,
    /// Present failed
    PresentFailed = 1335,
    /// Swapchain out of date
    SwapchainOutOfDate = 1336,
    /// Swapchain suboptimal
    SwapchainSuboptimal = 1337,
    /// Device lost
    DeviceLost = 1338,
    /// Invalid mesh data (empty or malformed)
    InvalidMeshData = 1342,
    /// Debug snapshot validation failed
    DebugSnapshotValidationFailed = 1343,
    /// Invalid timestamp in debug snapshot
    InvalidTimestamp = 1344,
    /// Invalid viewport dimensions
    InvalidViewport = 1345,
    /// Invalid draw call data
    InvalidDrawCall = 1346,
    /// Invalid transform matrix (NaN or Inf)
    InvalidTransform = 1347,
    /// Draw call has zero vertices
    ZeroVertices = 1348,
    /// Debug data export failed (I/O error)
    DebugExportIo = 1349,
    /// Debug data serialization failed
    DebugExportSerialization = 1350,
    /// Debug database operation failed
    DebugExportDatabase = 1351,
    /// PNG encoding failed
    DebugExportPngEncoding = 1352,
    /// Failed to read color buffer for frame capture
    DebugCaptureColorBufferReadFailed = 1353,
    /// Failed to read depth buffer for frame capture
    DebugCaptureDepthBufferReadFailed = 1354,
    /// Frame dimensions mismatch
    DebugCaptureDimensionMismatch = 1355,
    /// Invalid frame data
    DebugCaptureInvalidFrameData = 1356,

    // Networking (1400-1499)
    /// Network connection failed
    ConnectionFailed = 1400,
    /// Socket bind failed
    BindFailed = 1401,
    /// Network send failed
    SendFailed = 1402,
    /// Network receive failed
    ReceiveFailed = 1403,
    /// Network protocol error
    ProtocolError = 1404,
    /// TLS handshake failed
    TlsHandshakeFailed = 1405,
    /// TLS certificate error
    TlsCertificateError = 1406,
    /// TLS configuration error
    TlsConfigError = 1407,
    /// TLS connection error
    TlsConnectionError = 1408,
    /// TLS encryption error
    TlsEncryptionError = 1409,
    /// TLS decryption error
    TlsDecryptionError = 1410,
    /// Certificate validation failed
    CertificateValidationFailed = 1411,
    /// Certificate expired
    CertificateExpired = 1412,
    /// Certificate not yet valid
    CertificateNotYetValid = 1413,
    /// Invalid certificate
    InvalidCertificate = 1414,
    /// Certificate chain error
    CertificateChainError = 1415,
    /// ACME protocol error
    AcmeError = 1416,
    /// Certificate renewal failed
    CertificateRenewalFailed = 1417,
    /// DTLS handshake failed
    DtlsHandshakeFailed = 1418,
    /// DTLS packet error
    DtlsPacketError = 1419,

    // Physics (1500-1599)
    /// Physics system initialization failed
    PhysicsInitFailed = 1500,
    /// Collision detection failed
    CollisionDetectionFailed = 1501,

    // Audio (1600-1699)
    /// Audio system initialization failed
    AudioInitFailed = 1600,
    /// Sound file load failed
    SoundLoadFailed = 1601,

    // LOD (1700-1799)
    /// LOD system initialization failed
    LodInitFailed = 1700,

    // Interest Management (1800-1899)
    /// Interest management initialization failed
    InterestInitFailed = 1800,

    // Auto-update (1900-1999)
    /// Update check failed
    UpdateCheckFailed = 1900,
    /// Update download failed
    UpdateDownloadFailed = 1901,
    /// Update installation failed
    UpdateInstallFailed = 1902,
}

impl ErrorCode {
    /// Get the subsystem this error code belongs to.
    pub fn subsystem(&self) -> &'static str {
        let code = *self as u32;
        match code {
            1000..=1099 => "Core ECS",
            1100..=1199 => "Serialization",
            1200..=1299 => "Platform",
            1300..=1399 => "Rendering",
            1400..=1499 => "Networking",
            1500..=1599 => "Physics",
            1600..=1699 => "Audio",
            1700..=1799 => "LOD",
            1800..=1899 => "Interest Management",
            1900..=1999 => "Auto-update",
            _ => "Unknown",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.subsystem(), *self as u32)
    }
}

/// Error severity levels for filtering and alerting.
///
/// - Warning: Non-fatal issues that don't prevent operation
/// - Error: Failures that prevent a specific operation but don't crash the engine
/// - Critical: Failures that require engine shutdown or restart
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorSeverity {
    /// Non-fatal issues that don't prevent operation
    Warning,
    /// Failures that prevent a specific operation but don't crash the engine
    Error,
    /// Failures that require engine shutdown or restart
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Warning => write!(f, "WARNING"),
            ErrorSeverity::Error => write!(f, "ERROR"),
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Core trait for all engine errors.
///
/// This trait extends `std::error::Error` with structured error codes,
/// severity levels, and automatic logging via `tracing`.
///
/// # Implementation
///
/// Use the `define_error!` macro from `engine-macros` to automatically
/// implement this trait:
///
/// ```ignore
/// use engine_macros::define_error;
///
/// define_error! {
///     pub enum MyError {
///         NotFound { id: u32 } = ErrorCode::EntityNotFound, ErrorSeverity::Error,
///         InvalidData { reason: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
///     }
/// }
/// ```
pub trait EngineError: std::error::Error + Send + Sync {
    /// Get the error code for programmatic handling.
    fn code(&self) -> ErrorCode;

    /// Get the severity level.
    fn severity(&self) -> ErrorSeverity;

    /// Get the backtrace for this error, if available.
    ///
    /// Backtraces are only captured when the `backtrace` feature is enabled.
    /// This provides detailed information about where the error occurred.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(backtrace) = err.backtrace() {
    ///     println!("Error occurred at:\n{}", backtrace);
    /// }
    /// ```
    fn backtrace(&self) -> Option<&std::backtrace::Backtrace> {
        None
    }

    /// Log this error using structured logging.
    ///
    /// This method is automatically called by the error handling infrastructure.
    /// The default implementation uses `tracing` to emit structured log events.
    /// When the `backtrace` feature is enabled, backtraces are included in logs.
    fn log(&self) {
        use tracing::{error, warn};

        match self.severity() {
            ErrorSeverity::Warning => {
                if let Some(bt) = self.backtrace() {
                    warn!(
                        error_code = %self.code(),
                        error_message = %self,
                        subsystem = self.code().subsystem(),
                        backtrace = %bt,
                        "Engine warning"
                    );
                } else {
                    warn!(
                        error_code = %self.code(),
                        error_message = %self,
                        subsystem = self.code().subsystem(),
                        "Engine warning"
                    );
                }
            }
            ErrorSeverity::Error => {
                if let Some(bt) = self.backtrace() {
                    error!(
                        error_code = %self.code(),
                        error_message = %self,
                        subsystem = self.code().subsystem(),
                        backtrace = %bt,
                        "Engine error"
                    );
                } else {
                    error!(
                        error_code = %self.code(),
                        error_message = %self,
                        subsystem = self.code().subsystem(),
                        "Engine error"
                    );
                }
            }
            ErrorSeverity::Critical => {
                if let Some(bt) = self.backtrace() {
                    error!(
                        error_code = %self.code(),
                        error_message = %self,
                        subsystem = self.code().subsystem(),
                        backtrace = %bt,
                        "CRITICAL ENGINE ERROR"
                    );
                } else {
                    error!(
                        error_code = %self.code(),
                        error_message = %self,
                        subsystem = self.code().subsystem(),
                        "CRITICAL ENGINE ERROR"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_subsystem() {
        assert_eq!(ErrorCode::EntityNotFound.subsystem(), "Core ECS");
        assert_eq!(ErrorCode::SerializationFailed.subsystem(), "Serialization");
        assert_eq!(ErrorCode::WindowCreationFailed.subsystem(), "Platform");
        assert_eq!(ErrorCode::VulkanInitFailed.subsystem(), "Rendering");
        assert_eq!(ErrorCode::ConnectionFailed.subsystem(), "Networking");
    }

    #[test]
    fn test_error_code_display() {
        let code = ErrorCode::EntityNotFound;
        let display = format!("{}", code);
        assert!(display.contains("Core ECS"));
        assert!(display.contains("1000"));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(ErrorSeverity::Warning < ErrorSeverity::Error);
        assert!(ErrorSeverity::Error < ErrorSeverity::Critical);
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", ErrorSeverity::Warning), "WARNING");
        assert_eq!(format!("{}", ErrorSeverity::Error), "ERROR");
        assert_eq!(format!("{}", ErrorSeverity::Critical), "CRITICAL");
    }

    #[test]
    fn test_error_code_ranges() {
        // Verify each subsystem is in correct range
        assert!((ErrorCode::EntityNotFound as u32) >= 1000);
        assert!((ErrorCode::EntityNotFound as u32) < 1100);

        assert!((ErrorCode::SerializationFailed as u32) >= 1100);
        assert!((ErrorCode::SerializationFailed as u32) < 1200);

        assert!((ErrorCode::WindowCreationFailed as u32) >= 1200);
        assert!((ErrorCode::WindowCreationFailed as u32) < 1300);
    }
}
