//! Line drawing primitives for debug visualization
//!
//! Provides basic line, wireframe box, and arrow rendering utilities.

use engine_math::Vec3;

/// A single debug line segment
#[derive(Debug, Clone, Copy)]
pub struct DebugLine {
    /// Start point
    pub start: Vec3,
    /// End point
    pub end: Vec3,
    /// Color (RGB, 0.0-1.0 range)
    pub color: [f32; 3],
}

impl DebugLine {
    /// Create a new debug line
    pub fn new(start: Vec3, end: Vec3, color: [f32; 3]) -> Self {
        Self { start, end, color }
    }
}

/// Generate lines for a wireframe axis-aligned bounding box
///
/// Returns 12 lines forming the edges of a box.
pub fn wireframe_box(min: Vec3, max: Vec3, color: [f32; 3]) -> [DebugLine; 12] {
    // Bottom face (4 lines)
    let line0 = DebugLine::new(Vec3::new(min.x, min.y, min.z), Vec3::new(max.x, min.y, min.z), color);
    let line1 = DebugLine::new(Vec3::new(max.x, min.y, min.z), Vec3::new(max.x, min.y, max.z), color);
    let line2 = DebugLine::new(Vec3::new(max.x, min.y, max.z), Vec3::new(min.x, min.y, max.z), color);
    let line3 = DebugLine::new(Vec3::new(min.x, min.y, max.z), Vec3::new(min.x, min.y, min.z), color);

    // Top face (4 lines)
    let line4 = DebugLine::new(Vec3::new(min.x, max.y, min.z), Vec3::new(max.x, max.y, min.z), color);
    let line5 = DebugLine::new(Vec3::new(max.x, max.y, min.z), Vec3::new(max.x, max.y, max.z), color);
    let line6 = DebugLine::new(Vec3::new(max.x, max.y, max.z), Vec3::new(min.x, max.y, max.z), color);
    let line7 = DebugLine::new(Vec3::new(min.x, max.y, max.z), Vec3::new(min.x, max.y, min.z), color);

    // Vertical edges (4 lines)
    let line8 = DebugLine::new(Vec3::new(min.x, min.y, min.z), Vec3::new(min.x, max.y, min.z), color);
    let line9 = DebugLine::new(Vec3::new(max.x, min.y, min.z), Vec3::new(max.x, max.y, min.z), color);
    let line10 = DebugLine::new(Vec3::new(max.x, min.y, max.z), Vec3::new(max.x, max.y, max.z), color);
    let line11 = DebugLine::new(Vec3::new(min.x, min.y, max.z), Vec3::new(min.x, max.y, max.z), color);

    [
        line0, line1, line2, line3, // Bottom
        line4, line5, line6, line7, // Top
        line8, line9, line10, line11, // Vertical
    ]
}

/// Generate lines for an arrow
///
/// Returns 4 lines: 1 shaft + 3 arrowhead lines
pub fn arrow(start: Vec3, end: Vec3, color: [f32; 3]) -> [DebugLine; 4] {
    let direction = (end - start).normalize();
    let length = (end - start).length();

    // Shaft
    let shaft = DebugLine::new(start, end, color);

    // Arrowhead (20% of total length)
    let head_length = length * 0.2;
    let head_start = end - direction * head_length;

    // Find perpendicular vectors for arrowhead
    let up = if direction.y.abs() < 0.9 {
        Vec3::Y
    } else {
        Vec3::X
    };
    let right = direction.cross(up).normalize();
    let up = right.cross(direction).normalize();

    let head_width = head_length * 0.5;

    // Arrowhead lines (3 lines forming a cone)
    let head1 = DebugLine::new(end, head_start + right * head_width, color);
    let head2 = DebugLine::new(end, head_start - right * head_width, color);
    let head3 = DebugLine::new(end, head_start + up * head_width, color);

    [shaft, head1, head2, head3]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wireframe_box_line_count() {
        let lines = wireframe_box(Vec3::ZERO, Vec3::ONE, [1.0, 0.0, 0.0]);
        assert_eq!(lines.len(), 12, "Box should have 12 edges");
    }

    #[test]
    fn test_wireframe_box_closure() {
        let lines = wireframe_box(Vec3::ZERO, Vec3::ONE, [1.0, 0.0, 0.0]);

        // Verify that lines form a closed box (each vertex connects to 3 edges)
        let mut vertices = std::collections::HashMap::new();
        for line in &lines {
            *vertices.entry((line.start.x, line.start.y, line.start.z)).or_insert(0) += 1;
            *vertices.entry((line.end.x, line.end.y, line.end.z)).or_insert(0) += 1;
        }

        // Box has 8 vertices, each connects to 3 edges
        assert_eq!(vertices.len(), 8, "Box should have 8 unique vertices");
        for count in vertices.values() {
            assert_eq!(*count, 3, "Each vertex should connect to exactly 3 edges");
        }
    }

    #[test]
    fn test_arrow_line_count() {
        let lines = arrow(Vec3::ZERO, Vec3::Y, [0.0, 1.0, 0.0]);
        assert_eq!(lines.len(), 4, "Arrow should have 4 lines (1 shaft + 3 head)");
    }

    #[test]
    fn test_arrow_direction() {
        let start = Vec3::ZERO;
        let end = Vec3::new(0.0, 10.0, 0.0);
        let lines = arrow(start, end, [0.0, 1.0, 0.0]);

        // Shaft should go from start to end
        assert_eq!(lines[0].start, start);
        assert_eq!(lines[0].end, end);

        // Arrowhead lines should start at end point
        assert_eq!(lines[1].start, end);
        assert_eq!(lines[2].start, end);
        assert_eq!(lines[3].start, end);
    }
}
