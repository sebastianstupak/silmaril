//! Entity picking — resolve viewport click to entity id.

use super::{EntityView, ViewportCamera};

/// Find the entity under the click position `(click_x, click_y)` in pixel
/// coordinates, given the viewport dimensions and camera state.
///
/// Returns `Some(entity_id)` of the closest (top-most) entity whose circle
/// contains the click point, or `None` if nothing was hit.
pub fn pick_entity(
    click_x: f32,
    click_y: f32,
    width: u32,
    height: u32,
    entities: &[EntityView],
    camera: &ViewportCamera,
) -> Option<u64> {
    let half_w = width as f32 / 2.0;
    let half_h = height as f32 / 2.0;
    let hit_radius = 14.0_f32; // generous click target

    let mut best: Option<(u64, f32)> = None;

    for entity in entities {
        let px =
            half_w + (entity.x - 0.5) * width as f32 * camera.zoom + camera.offset_x * camera.zoom;
        let py = half_h
            + (entity.y - 0.5) * height as f32 * camera.zoom
            + camera.offset_y * camera.zoom;

        let dx = click_x - px;
        let dy = click_y - py;
        let dist_sq = dx * dx + dy * dy;

        if dist_sq <= hit_radius * hit_radius {
            match best {
                Some((_, prev_dist)) if dist_sq < prev_dist => {
                    best = Some((entity.id, dist_sq));
                }
                None => {
                    best = Some((entity.id, dist_sq));
                }
                _ => {}
            }
        }
    }

    best.map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity(id: u64, x: f32, y: f32) -> EntityView {
        EntityView {
            id,
            name: format!("E{id}"),
            x,
            y,
            color: "#fff".to_string(),
        }
    }

    #[test]
    fn pick_center_entity() {
        let entities = vec![make_entity(1, 0.5, 0.5)];
        let cam = ViewportCamera::default();
        // Click at viewport center (400, 300) for 800x600 viewport
        let result = pick_entity(400.0, 300.0, 800, 600, &entities, &cam);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn pick_miss() {
        let entities = vec![make_entity(1, 0.5, 0.5)];
        let cam = ViewportCamera::default();
        // Click far from center
        let result = pick_entity(10.0, 10.0, 800, 600, &entities, &cam);
        assert_eq!(result, None);
    }

    #[test]
    fn pick_closest_when_overlapping() {
        let entities = vec![make_entity(1, 0.5, 0.5), make_entity(2, 0.505, 0.505)];
        let cam = ViewportCamera::default();
        // Click near entity 2's center
        let e2_x = 400.0 + (0.505 - 0.5) * 800.0;
        let e2_y = 300.0 + (0.505 - 0.5) * 600.0;
        let result = pick_entity(e2_x, e2_y, 800, 600, &entities, &cam);
        assert_eq!(result, Some(2));
    }
}
