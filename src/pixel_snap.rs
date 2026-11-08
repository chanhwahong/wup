//! Row backgrounds are separated by reserved layout space, not thin borders.
#[derive(Clone, Copy, Debug)]
pub struct RowMetrics {
    pub content_height: f32,
    pub separator_height: f32,
}

pub fn row_metrics(base_height: f32, zoom: f32, dpi: f32) -> RowMetrics {
    let dpi = if dpi.is_finite() && dpi > 0.0 {
        dpi
    } else {
        1.0
    };
    let zoom = if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    };
    let base_height = if base_height.is_finite() && base_height > 0.0 {
        base_height
    } else {
        64.0
    };
    // At least two device pixels and one logical pixel. The latter keeps the
    // gap nonzero even if the layout engine rounds logical coordinates.
    let separator = dpi.ceil().max(2.0);
    let total = (base_height * zoom * dpi)
        .round()
        .max(separator + dpi.ceil());
    RowMetrics {
        content_height: (total - separator) / dpi,
        separator_height: separator / dpi,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn separator_remains_visible_after_fractional_position_and_layout_rounding() {
        for dpi in [1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0] {
            for percent in (80..=120).step_by(5) {
                let m = row_metrics(64.0, percent as f32 * 9.0 / 1000.0, dpi);
                let total = m.content_height + m.separator_height;
                assert!(m.content_height > 0.0 && m.separator_height >= 1.0);
                assert!((total * dpi - (total * dpi).round()).abs() < 0.001);
                for row in 0..100 {
                    for phase in [0.0, 0.1, 0.25, 0.49, 0.5, 0.75, 0.99] {
                        for scroll in [0.0, 17.35, 83.72] {
                            let start = 55.3 + row as f32 * total + phase - scroll;
                            let inner_end = (start + m.content_height).round();
                            let outer_end = (start + total).round();
                            assert!((outer_end - inner_end) * dpi >= 0.999);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn invalid_inputs_have_finite_nonzero_fallbacks() {
        for (height, zoom, dpi) in [
            (0.0, 0.0, 0.0),
            (f32::NAN, 1.0, f32::NAN),
            (64.0, -1.0, -1.0),
        ] {
            let m = row_metrics(height, zoom, dpi);
            assert!(m.content_height.is_finite() && m.content_height > 0.0);
            assert!(m.separator_height.is_finite() && m.separator_height >= 1.0);
        }
    }
}
