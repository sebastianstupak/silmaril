//! SVG viewport — 2D scene visualization for the editor.
//!
//! Generates an SVG representation of the scene with a grid, entity
//! positions, and selection highlighting.  This serves as the visual
//! scaffold while real Vulkan integration is developed separately.

pub mod picking;

use serde::{Deserialize, Serialize};
use std::fmt::Write;

/// View of a single entity to be rendered in the viewport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityView {
    pub id: u64,
    pub name: String,
    /// Normalised position in \[0, 1\] space.
    pub x: f32,
    /// Normalised position in \[0, 1\] space.
    pub y: f32,
    /// CSS colour string, e.g. `"#e06c75"`.
    pub color: String,
}

/// Camera state for panning / zooming the viewport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportCamera {
    pub offset_x: f32,
    pub offset_y: f32,
    pub zoom: f32,
}

impl Default for ViewportCamera {
    fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom: 1.0,
        }
    }
}

/// Colour palette for entity circles.
const ENTITY_COLORS: &[&str] = &[
    "#e06c75", // red
    "#61afef", // blue
    "#98c379", // green
    "#e5c07b", // yellow
    "#c678dd", // purple
    "#56b6c2", // cyan
    "#d19a66", // orange
    "#be5046", // dark-red
];

/// Pick a colour for entity index `i`.
pub fn entity_color(i: usize) -> &'static str {
    ENTITY_COLORS[i % ENTITY_COLORS.len()]
}

// SVG colour constants used in format strings.
// Kept here so we can pass them as named arguments to `write!()` without
// embedding literal `#` inside raw strings (which would prematurely close
// the `r#"..."#` delimiter).
const BG_COLOR: &str = "#1a1a2e";
const GRID_COLOR: &str = "#252545";
const TEXT_DIM_COLOR: &str = "#555";
const TEXT_COLOR: &str = "#ccc";
const AXIS_X_COLOR: &str = "#e06c7566";
const AXIS_Y_COLOR: &str = "#98c37966";
const SEL_RING_COLOR: &str = "#61afef";
const SEL_STROKE: &str = "#ffffff";
const DEFAULT_STROKE: &str = "#aaa";
const FONT: &str = "sans-serif";

/// Generate a complete SVG string for the viewport.
///
/// * `width` / `height` — pixel dimensions of the viewport container.
/// * `entities`         — entities to draw.
/// * `selected_id`      — entity id to highlight, if any.
/// * `camera`           — pan/zoom state.
/// * `active_tool`      — current tool (`"select"`, `"move"`, `"rotate"`, `"scale"`).
pub fn generate_viewport_svg(
    width: u32,
    height: u32,
    entities: &[EntityView],
    selected_id: Option<u64>,
    camera: &ViewportCamera,
    active_tool: &str,
) -> String {
    let mut svg = String::with_capacity(4096);

    // Root element
    let _ = write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" \
         viewBox=\"0 0 {w} {h}\" style=\"display:block\">",
        w = width,
        h = height,
    );

    // -- Definitions (selection glow filter) --
    svg.push_str(concat!(
        "<defs>",
        "<filter id=\"sel-glow\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\">",
        "<feGaussianBlur stdDeviation=\"4\" result=\"blur\"/>",
        "<feMerge><feMergeNode in=\"blur\"/><feMergeNode in=\"SourceGraphic\"/></feMerge>",
        "</filter>",
        "</defs>",
    ));

    // -- Background --
    let _ = write!(
        svg,
        "<rect width=\"{w}\" height=\"{h}\" fill=\"{bg}\"/>",
        w = width,
        h = height,
        bg = BG_COLOR,
    );

    // -- Grid --
    write_grid(&mut svg, width, height, camera);

    // -- Origin crosshair --
    write_origin(&mut svg, width, height, camera);

    // -- Entities --
    for entity in entities {
        let is_selected = selected_id == Some(entity.id);
        write_entity(&mut svg, width, height, entity, is_selected, camera);

        // Draw gizmo overlay on the selected entity
        if is_selected && active_tool != "select" {
            let half_w = width as f32 / 2.0;
            let half_h = height as f32 / 2.0;
            let px = half_w
                + (entity.x - 0.5) * width as f32 * camera.zoom
                + camera.offset_x * camera.zoom;
            let py = half_h
                + (entity.y - 0.5) * height as f32 * camera.zoom
                + camera.offset_y * camera.zoom;

            match active_tool {
                "move" => write_move_gizmo(&mut svg, px, py),
                "rotate" => write_rotate_gizmo(&mut svg, px, py),
                "scale" => write_scale_gizmo(&mut svg, px, py),
                _ => {}
            }
        }
    }

    // -- "No entities" hint --
    if entities.is_empty() {
        let cx = width / 2;
        let cy = height / 2;
        let _ = write!(
            svg,
            "<text x=\"{cx}\" y=\"{cy}\" fill=\"{c}\" font-family=\"{f}\" \
             font-size=\"14\" text-anchor=\"middle\" dominant-baseline=\"middle\">\
             No entities in scene</text>",
            c = TEXT_DIM_COLOR,
            f = FONT,
        );
    }

    svg.push_str("</svg>");
    svg
}

// ---------------------------------------------------------------------------
// Internal drawing helpers
// ---------------------------------------------------------------------------

fn write_grid(svg: &mut String, width: u32, height: u32, camera: &ViewportCamera) {
    let spacing = 50.0 * camera.zoom;
    if spacing < 8.0 {
        return; // too zoomed out, skip grid
    }

    let ox = (camera.offset_x * camera.zoom) % spacing;
    let oy = (camera.offset_y * camera.zoom) % spacing;

    let mut x = ox;
    while x < width as f32 {
        let _ = write!(
            svg,
            "<line x1=\"{x:.1}\" y1=\"0\" x2=\"{x:.1}\" y2=\"{h}\" \
             stroke=\"{c}\" stroke-width=\"0.5\"/>",
            h = height,
            c = GRID_COLOR,
        );
        x += spacing;
    }

    let mut y = oy;
    while y < height as f32 {
        let _ = write!(
            svg,
            "<line x1=\"0\" y1=\"{y:.1}\" x2=\"{w}\" y2=\"{y:.1}\" \
             stroke=\"{c}\" stroke-width=\"0.5\"/>",
            w = width,
            c = GRID_COLOR,
        );
        y += spacing;
    }
}

fn write_origin(svg: &mut String, width: u32, height: u32, camera: &ViewportCamera) {
    let cx = (width as f32) / 2.0 + camera.offset_x * camera.zoom;
    let cy = (height as f32) / 2.0 + camera.offset_y * camera.zoom;

    // Horizontal axis (red)
    let _ = write!(
        svg,
        "<line x1=\"0\" y1=\"{cy:.1}\" x2=\"{w}\" y2=\"{cy:.1}\" \
         stroke=\"{c}\" stroke-width=\"1\"/>",
        w = width,
        c = AXIS_X_COLOR,
    );
    // Vertical axis (green)
    let _ = write!(
        svg,
        "<line x1=\"{cx:.1}\" y1=\"0\" x2=\"{cx:.1}\" y2=\"{h}\" \
         stroke=\"{c}\" stroke-width=\"1\"/>",
        h = height,
        c = AXIS_Y_COLOR,
    );
}

fn write_entity(
    svg: &mut String,
    width: u32,
    height: u32,
    entity: &EntityView,
    selected: bool,
    camera: &ViewportCamera,
) {
    let half_w = width as f32 / 2.0;
    let half_h = height as f32 / 2.0;

    // Map normalised [0,1] position -> pixel, centered around viewport middle
    let px =
        half_w + (entity.x - 0.5) * width as f32 * camera.zoom + camera.offset_x * camera.zoom;
    let py =
        half_h + (entity.y - 0.5) * height as f32 * camera.zoom + camera.offset_y * camera.zoom;

    let radius = if selected { 14.0 } else { 10.0 };

    // Selection ring
    if selected {
        let _ = write!(
            svg,
            "<circle cx=\"{px:.1}\" cy=\"{py:.1}\" r=\"{r}\" fill=\"none\" \
             stroke=\"{c}\" stroke-width=\"2.5\" filter=\"url(#sel-glow)\"/>",
            r = radius + 5.0,
            c = SEL_RING_COLOR,
        );
    }

    // Entity circle
    let stroke = if selected { SEL_STROKE } else { DEFAULT_STROKE };
    let sw = if selected { 2.0 } else { 0.8 };
    let _ = write!(
        svg,
        "<circle cx=\"{px:.1}\" cy=\"{py:.1}\" r=\"{radius}\" fill=\"{col}\" \
         stroke=\"{stroke}\" stroke-width=\"{sw}\" opacity=\"0.9\" \
         data-entity-id=\"{id}\"/>",
        col = entity.color,
        id = entity.id,
    );

    // Label below circle
    let _ = write!(
        svg,
        "<text x=\"{px:.1}\" y=\"{ty:.1}\" fill=\"{c}\" font-family=\"{f}\" \
         font-size=\"10\" text-anchor=\"middle\">{name}</text>",
        ty = py + radius + 14.0,
        c = TEXT_COLOR,
        f = FONT,
        name = html_escape(&entity.name),
    );
}

// ---------------------------------------------------------------------------
// Gizmo drawing helpers
// ---------------------------------------------------------------------------

const GIZMO_X_COLOR: &str = "#ff4444";
const GIZMO_Y_COLOR: &str = "#44ff44";
const GIZMO_Z_COLOR: &str = "#4444ff";

/// Draw a move (translate) gizmo: three coloured arrows (X, Y, Z) with arrowheads
/// and a centre square for free-plane movement.
fn write_move_gizmo(svg: &mut String, cx: f32, cy: f32) {
    let len = 40.0;

    // X axis (red) — rightward arrow
    let _ = write!(
        svg,
        "<line x1=\"{cx:.1}\" y1=\"{cy:.1}\" x2=\"{x2:.1}\" y2=\"{cy:.1}\" \
         stroke=\"{c}\" stroke-width=\"2\"/>",
        x2 = cx + len,
        c = GIZMO_X_COLOR,
    );
    let _ = write!(
        svg,
        "<polygon points=\"{x1:.1},{y1:.1} {x2:.1},{y2:.1} {x3:.1},{y3:.1}\" fill=\"{c}\"/>",
        x1 = cx + len,
        y1 = cy - 4.0,
        x2 = cx + len,
        y2 = cy + 4.0,
        x3 = cx + len + 8.0,
        y3 = cy,
        c = GIZMO_X_COLOR,
    );

    // Y axis (green) — upward arrow
    let _ = write!(
        svg,
        "<line x1=\"{cx:.1}\" y1=\"{cy:.1}\" x2=\"{cx:.1}\" y2=\"{y2:.1}\" \
         stroke=\"{c}\" stroke-width=\"2\"/>",
        y2 = cy - len,
        c = GIZMO_Y_COLOR,
    );
    let _ = write!(
        svg,
        "<polygon points=\"{x1:.1},{y1:.1} {x2:.1},{y2:.1} {x3:.1},{y3:.1}\" fill=\"{c}\"/>",
        x1 = cx - 4.0,
        y1 = cy - len,
        x2 = cx + 4.0,
        y2 = cy - len,
        x3 = cx,
        y3 = cy - len - 8.0,
        c = GIZMO_Y_COLOR,
    );

    // Z axis (blue) — diagonal dashed line (perspective hint)
    let _ = write!(
        svg,
        "<line x1=\"{cx:.1}\" y1=\"{cy:.1}\" x2=\"{x2:.1}\" y2=\"{y2:.1}\" \
         stroke=\"{c}\" stroke-width=\"2\" stroke-dasharray=\"4,2\"/>",
        x2 = cx - len * 0.5,
        y2 = cy + len * 0.5,
        c = GIZMO_Z_COLOR,
    );

    // Centre square (free-plane movement)
    let _ = write!(
        svg,
        "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"8\" height=\"8\" fill=\"yellow\" opacity=\"0.6\"/>",
        x = cx - 4.0,
        y = cy - 4.0,
    );
}

/// Draw a rotate gizmo: three coloured ellipses (X, Y rotation planes) and an
/// outer screen-space ring.
fn write_rotate_gizmo(svg: &mut String, cx: f32, cy: f32) {
    let r = 35.0;

    // X rotation ellipse (red — horizontal)
    let _ = write!(
        svg,
        "<ellipse cx=\"{cx:.1}\" cy=\"{cy:.1}\" rx=\"{rx:.1}\" ry=\"{ry:.1}\" \
         fill=\"none\" stroke=\"{c}\" stroke-width=\"1.5\"/>",
        rx = r,
        ry = r * 0.3,
        c = GIZMO_X_COLOR,
    );

    // Y rotation ellipse (green — vertical)
    let _ = write!(
        svg,
        "<ellipse cx=\"{cx:.1}\" cy=\"{cy:.1}\" rx=\"{rx:.1}\" ry=\"{ry:.1}\" \
         fill=\"none\" stroke=\"{c}\" stroke-width=\"1.5\"/>",
        rx = r * 0.3,
        ry = r,
        c = GIZMO_Y_COLOR,
    );

    // Screen-space rotation ring (white outer circle)
    let _ = write!(
        svg,
        "<circle cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{r:.1}\" \
         fill=\"none\" stroke=\"white\" stroke-width=\"1\" opacity=\"0.5\"/>",
        r = r + 5.0,
    );
}

/// Draw a scale gizmo: three coloured lines with cube endpoints and a centre cube
/// for uniform scaling.
fn write_scale_gizmo(svg: &mut String, cx: f32, cy: f32) {
    let len = 35.0;
    let cube = 5.0;

    // X axis (red) with cube endpoint
    let _ = write!(
        svg,
        "<line x1=\"{cx:.1}\" y1=\"{cy:.1}\" x2=\"{x2:.1}\" y2=\"{cy:.1}\" \
         stroke=\"{c}\" stroke-width=\"2\"/>",
        x2 = cx + len,
        c = GIZMO_X_COLOR,
    );
    let _ = write!(
        svg,
        "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{s}\" height=\"{s}\" fill=\"{c}\"/>",
        x = cx + len - cube / 2.0,
        y = cy - cube / 2.0,
        s = cube,
        c = GIZMO_X_COLOR,
    );

    // Y axis (green) with cube endpoint
    let _ = write!(
        svg,
        "<line x1=\"{cx:.1}\" y1=\"{cy:.1}\" x2=\"{cx:.1}\" y2=\"{y2:.1}\" \
         stroke=\"{c}\" stroke-width=\"2\"/>",
        y2 = cy - len,
        c = GIZMO_Y_COLOR,
    );
    let _ = write!(
        svg,
        "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{s}\" height=\"{s}\" fill=\"{c}\"/>",
        x = cx - cube / 2.0,
        y = cy - len - cube / 2.0,
        s = cube,
        c = GIZMO_Y_COLOR,
    );

    // Centre cube (uniform scale)
    let _ = write!(
        svg,
        "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{s}\" height=\"{s}\" fill=\"white\" opacity=\"0.8\"/>",
        x = cx - cube / 2.0,
        y = cy - cube / 2.0,
        s = cube,
    );
}

/// Minimal HTML entity escaping for SVG text content.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_scene_produces_valid_svg() {
        let svg =
            generate_viewport_svg(800, 600, &[], None, &ViewportCamera::default(), "select");
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("No entities in scene"));
    }

    #[test]
    fn entities_appear_in_svg() {
        let entities = vec![
            EntityView {
                id: 1,
                name: "Player".to_string(),
                x: 0.5,
                y: 0.5,
                color: "#e06c75".to_string(),
            },
            EntityView {
                id: 2,
                name: "Enemy".to_string(),
                x: 0.3,
                y: 0.7,
                color: "#61afef".to_string(),
            },
        ];

        let svg = generate_viewport_svg(
            800,
            600,
            &entities,
            Some(1),
            &ViewportCamera::default(),
            "select",
        );
        assert!(svg.contains("data-entity-id=\"1\""));
        assert!(svg.contains("data-entity-id=\"2\""));
        assert!(svg.contains("Player"));
        assert!(svg.contains("Enemy"));
        // Selected entity should have glow filter
        assert!(svg.contains("sel-glow"));
    }

    #[test]
    fn move_gizmo_appears_for_selected_entity() {
        let entities = vec![EntityView {
            id: 1,
            name: "Player".to_string(),
            x: 0.5,
            y: 0.5,
            color: "#e06c75".to_string(),
        }];
        let svg = generate_viewport_svg(
            800,
            600,
            &entities,
            Some(1),
            &ViewportCamera::default(),
            "move",
        );
        // Move gizmo draws red and green arrows
        assert!(svg.contains("#ff4444"));
        assert!(svg.contains("#44ff44"));
        assert!(svg.contains("polygon"));
    }

    #[test]
    fn rotate_gizmo_appears_for_selected_entity() {
        let entities = vec![EntityView {
            id: 1,
            name: "Player".to_string(),
            x: 0.5,
            y: 0.5,
            color: "#e06c75".to_string(),
        }];
        let svg = generate_viewport_svg(
            800,
            600,
            &entities,
            Some(1),
            &ViewportCamera::default(),
            "rotate",
        );
        assert!(svg.contains("ellipse"));
    }

    #[test]
    fn scale_gizmo_appears_for_selected_entity() {
        let entities = vec![EntityView {
            id: 1,
            name: "Player".to_string(),
            x: 0.5,
            y: 0.5,
            color: "#e06c75".to_string(),
        }];
        let svg = generate_viewport_svg(
            800,
            600,
            &entities,
            Some(1),
            &ViewportCamera::default(),
            "scale",
        );
        // Scale gizmo draws cubes (rects) at axis endpoints
        assert!(svg.contains("opacity=\"0.8\""));
    }

    #[test]
    fn no_gizmo_for_select_tool() {
        let entities = vec![EntityView {
            id: 1,
            name: "Player".to_string(),
            x: 0.5,
            y: 0.5,
            color: "#e06c75".to_string(),
        }];
        let svg = generate_viewport_svg(
            800,
            600,
            &entities,
            Some(1),
            &ViewportCamera::default(),
            "select",
        );
        // Should NOT contain gizmo colours
        assert!(!svg.contains("#ff4444"));
        assert!(!svg.contains("#44ff44"));
    }

    #[test]
    fn html_escape_works() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("A & B"), "A &amp; B");
    }
}
