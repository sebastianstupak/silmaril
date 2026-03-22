//! Editor contribution points — forward declaration for the module bridge.
//!
//! Rust cargo modules implement [`EditorContributor`] to declare which panels
//! and inspector fields they contribute. The frontend companion for each module
//! registers the Svelte component under the matching panel ID.
//!
//! The Tauri command `list_module_contributions` (not yet implemented) will
//! collect all registered contributors and return their metadata to the frontend.

/// Metadata for a panel contributed by a Rust module.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PanelMeta {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: Option<&'static str>,
}

/// Metadata for an inspector field renderer contributed by a Rust module.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InspectorFieldMeta {
    /// ECS component type name this renderer handles, e.g. `"NetworkTransform"`.
    pub component_type: &'static str,
    pub source: &'static str,
}

/// Implement this trait on a unit struct in each Rust module that contributes
/// editor panels or inspector fields.
///
/// # Example
/// ```rust
/// use crate::bridge::contributions::{EditorContributor, PanelMeta, InspectorFieldMeta};
///
/// pub struct NetworkingContributions;
///
/// impl EditorContributor for NetworkingContributions {
///     fn panels(&self) -> Vec<PanelMeta> {
///         vec![PanelMeta { id: "networking-monitor", title: "Network Monitor", icon: None }]
///     }
///     fn inspector_fields(&self) -> Vec<InspectorFieldMeta> { vec![] }
/// }
/// ```
pub trait EditorContributor: Send + Sync {
    fn panels(&self) -> Vec<PanelMeta>;
    fn inspector_fields(&self) -> Vec<InspectorFieldMeta>;
}
