//! Scrollbar geometry shared by painting and pointer handling.
#[derive(Debug)]
pub struct Geometry {
    pub thumb_height: f32,
    pub thumb_top: f32,
    travel: f32,
    max_scroll: f32,
}

impl Geometry {
    pub fn scaled(viewport: f32, max_scroll: f32, offset: f32, scale: f32) -> Self {
        let viewport = viewport.max(0.0);
        let max_scroll = max_scroll.max(0.0);
        let thumb_height = if max_scroll == 0.0 {
            viewport
        } else {
            (viewport * viewport / (viewport + max_scroll))
                .max(28.0 * scale)
                .min(viewport)
        };
        let travel = viewport - thumb_height;
        let thumb_top = if max_scroll > 0.0 {
            (-offset / max_scroll).clamp(0.0, 1.0) * travel
        } else {
            0.0
        };
        Self {
            thumb_height,
            thumb_top,
            travel,
            max_scroll,
        }
    }

    pub fn offset_for_pointer(&self, pointer: f32, grab: f32) -> f32 {
        if self.travel <= 0.0 {
            0.0
        } else {
            -((pointer - grab) / self.travel).clamp(0.0, 1.0) * self.max_scroll
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fits_without_scrolling() {
        let geometry = Geometry::scaled(400.0, 0.0, 0.0, 1.0);
        assert_eq!(geometry.thumb_height, 400.0);
        assert_eq!(geometry.offset_for_pointer(150.0, 0.0), 0.0);
        assert_eq!(
            Geometry::scaled(0.0, 100.0, 0.0, 1.0).offset_for_pointer(10.0, 0.0),
            0.0
        );
    }
    #[test]
    fn maps_thumb_to_content_and_clamps_drag() {
        let geometry = Geometry::scaled(400.0, 1200.0, -600.0, 1.0);
        assert_eq!(geometry.thumb_height, 100.0);
        assert_eq!(geometry.thumb_top, 150.0);
        assert_eq!(geometry.offset_for_pointer(170.0, 20.0), -600.0);
        assert_eq!(geometry.offset_for_pointer(-50.0, 20.0), 0.0);
        assert_eq!(geometry.offset_for_pointer(999.0, 20.0), -1200.0);
    }

    #[test]
    fn scaled_thumb_preserves_pointer_mapping_at_supported_zoom_levels() {
        for factor in [0.8, 1.0, 1.2] {
            let geometry = Geometry::scaled(400.0, 10000.0, -5000.0, factor);
            assert!((geometry.thumb_height - 28.0 * factor).abs() < 0.001);
            assert!(
                (geometry.offset_for_pointer(geometry.thumb_top + 5.0 * factor, 5.0 * factor)
                    + 5000.0)
                    .abs()
                    < 0.01
            );
        }
    }
}
