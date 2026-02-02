//! Font data structures and loaders
//!
//! Pure data structures for font assets (TTF/OTF fonts).
//! No text rendering or GPU dependencies - can be used by server, tools, or client.

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};

define_error! {
    pub enum FontError {
        InvalidFormat { reason: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        MissingTable { table: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        ParseError { reason: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
        UnsupportedFormat { format: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
    }
}

/// Font style enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FontStyle {
    /// Normal/Regular style
    Normal,
    /// Italic style
    Italic,
    /// Oblique style (slanted)
    Oblique,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self::Normal
    }
}

/// Font weight enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FontWeight {
    /// Thin weight (100)
    Thin,
    /// Extra-light weight (200)
    ExtraLight,
    /// Light weight (300)
    Light,
    /// Normal/Regular weight (400)
    Normal,
    /// Medium weight (500)
    Medium,
    /// Semi-bold weight (600)
    SemiBold,
    /// Bold weight (700)
    Bold,
    /// Extra-bold weight (800)
    ExtraBold,
    /// Black weight (900)
    Black,
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::Normal
    }
}

impl FontWeight {
    /// Convert from numeric weight value
    pub fn from_value(value: u16) -> Self {
        match value {
            0..=150 => Self::Thin,
            151..=250 => Self::ExtraLight,
            251..=350 => Self::Light,
            351..=450 => Self::Normal,
            451..=550 => Self::Medium,
            551..=650 => Self::SemiBold,
            651..=750 => Self::Bold,
            751..=850 => Self::ExtraBold,
            _ => Self::Black,
        }
    }

    /// Convert to numeric weight value
    pub fn to_value(self) -> u16 {
        match self {
            Self::Thin => 100,
            Self::ExtraLight => 200,
            Self::Light => 300,
            Self::Normal => 400,
            Self::Medium => 500,
            Self::SemiBold => 600,
            Self::Bold => 700,
            Self::ExtraBold => 800,
            Self::Black => 900,
        }
    }
}

/// Font metrics (measurements)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FontMetrics {
    /// Ascent above baseline (in font units)
    pub ascent: i16,
    /// Descent below baseline (in font units, typically negative)
    pub descent: i16,
    /// Line gap spacing (in font units)
    pub line_gap: i16,
    /// Units per em (resolution of the font coordinate system)
    pub units_per_em: u16,
}

impl FontMetrics {
    /// Create new font metrics
    pub fn new(ascent: i16, descent: i16, line_gap: i16, units_per_em: u16) -> Self {
        Self { ascent, descent, line_gap, units_per_em }
    }

    /// Calculate total line height
    pub fn line_height(&self) -> i16 {
        self.ascent - self.descent + self.line_gap
    }
}

/// Font data (CPU-side font data)
///
/// Pure data structure - no text rendering or GPU state.
/// Rendering backends create text rendering resources from this data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontData {
    /// Font family name (e.g., "Arial", "Roboto")
    pub family: String,
    /// Font style (normal, italic, oblique)
    pub style: FontStyle,
    /// Font weight (thin, normal, bold, etc.)
    pub weight: FontWeight,
    /// Raw font file data (TTF/OTF bytes)
    pub data: Vec<u8>,
    /// Font metrics
    pub metrics: FontMetrics,
}

impl FontData {
    /// Create a new font data structure
    pub fn new(
        family: String,
        style: FontStyle,
        weight: FontWeight,
        data: Vec<u8>,
        metrics: FontMetrics,
    ) -> Self {
        Self { family, style, weight, data, metrics }
    }

    /// Load font from TTF (TrueType Font) file data
    #[instrument(skip(ttf_data))]
    pub fn from_ttf(ttf_data: &[u8]) -> Result<Self, FontError> {
        info!("Loading TTF font");
        Self::load_from_bytes(ttf_data, "TTF")
    }

    /// Load font from OTF (OpenType Font) file data
    #[instrument(skip(otf_data))]
    pub fn from_otf(otf_data: &[u8]) -> Result<Self, FontError> {
        info!("Loading OTF font");
        Self::load_from_bytes(otf_data, "OTF")
    }

    /// Internal method to load font from bytes using ttf-parser
    fn load_from_bytes(data: &[u8], format: &str) -> Result<Self, FontError> {
        // Parse font using ttf-parser
        let face = ttf_parser::Face::parse(data, 0).map_err(|e| FontError::InvalidFormat {
            reason: format!("Failed to parse {} font: {:?}", format, e),
        })?;

        // Extract family name
        let family = Self::extract_family_name(&face)?;

        // Extract style
        let style = Self::extract_style(&face);

        // Extract weight
        let weight = Self::extract_weight(&face);

        // Extract metrics
        let metrics = Self::extract_metrics(&face)?;

        info!(
            family = %family,
            style = ?style,
            weight = ?weight,
            "Font loaded successfully"
        );

        Ok(Self { family, style, weight, data: data.to_vec(), metrics })
    }

    /// Extract font family name from font face
    fn extract_family_name(face: &ttf_parser::Face) -> Result<String, FontError> {
        // Try to get family name from name table
        for name in face.names() {
            if name.name_id == ttf_parser::name_id::FAMILY {
                if let Some(family) = name.to_string() {
                    return Ok(family);
                }
            }
        }

        // Fallback: try full font name
        for name in face.names() {
            if name.name_id == ttf_parser::name_id::FULL_NAME {
                if let Some(full_name) = name.to_string() {
                    warn!("Using full name as family name: {}", full_name);
                    return Ok(full_name);
                }
            }
        }

        Err(FontError::MissingTable { table: "name (family)".to_string() })
    }

    /// Extract font style from font face
    fn extract_style(face: &ttf_parser::Face) -> FontStyle {
        // Check if font is italic or oblique
        if face.is_italic() {
            FontStyle::Italic
        } else if face.is_oblique() {
            FontStyle::Oblique
        } else {
            FontStyle::Normal
        }
    }

    /// Extract font weight from font face
    fn extract_weight(face: &ttf_parser::Face) -> FontWeight {
        FontWeight::from_value(face.weight().to_number())
    }

    /// Extract font metrics from font face
    fn extract_metrics(face: &ttf_parser::Face) -> Result<FontMetrics, FontError> {
        let ascent = face.ascender();
        let descent = face.descender();
        let line_gap = face.line_gap();
        let units_per_em = face.units_per_em();

        Ok(FontMetrics { ascent, descent, line_gap, units_per_em })
    }

    /// Get the number of glyphs in the font
    pub fn glyph_count(&self) -> u16 {
        // Re-parse to get glyph count
        if let Ok(face) = ttf_parser::Face::parse(&self.data, 0) {
            face.number_of_glyphs()
        } else {
            0
        }
    }

    /// Get font memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.data.len() + self.family.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_weight_conversion() {
        assert_eq!(FontWeight::from_value(100), FontWeight::Thin);
        assert_eq!(FontWeight::from_value(400), FontWeight::Normal);
        assert_eq!(FontWeight::from_value(700), FontWeight::Bold);
        assert_eq!(FontWeight::from_value(900), FontWeight::Black);

        assert_eq!(FontWeight::Thin.to_value(), 100);
        assert_eq!(FontWeight::Normal.to_value(), 400);
        assert_eq!(FontWeight::Bold.to_value(), 700);
    }

    #[test]
    fn test_font_metrics_line_height() {
        let metrics = FontMetrics::new(800, -200, 100, 1000);
        assert_eq!(metrics.line_height(), 1100); // 800 - (-200) + 100
    }

    #[test]
    fn test_font_style_default() {
        assert_eq!(FontStyle::default(), FontStyle::Normal);
    }

    #[test]
    fn test_font_weight_default() {
        assert_eq!(FontWeight::default(), FontWeight::Normal);
    }

    #[test]
    fn test_font_data_memory_usage() {
        let data = vec![0u8; 1000];
        let font = FontData::new(
            "Test".to_string(),
            FontStyle::Normal,
            FontWeight::Normal,
            data,
            FontMetrics::new(800, -200, 100, 1000),
        );

        // Should include size of struct + data + family string
        let usage = font.memory_usage();
        assert!(usage >= 1000); // At least the data size
    }
}
