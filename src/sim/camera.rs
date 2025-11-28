//! Camera system for defining views into world space

/// Camera defines a view into world space
/// Bounds directly define what the camera sees - no separate zoom
#[derive(Debug, Clone)]
pub struct Camera {
    /// World space bounds this camera views
    /// Changing bounds = zooming in/out
    pub bounds: Bounds,
}

impl Camera {
    /// Create camera with explicit world bounds
    pub fn new(min: [f32; 2], max: [f32; 2]) -> Self {
        Self {
            bounds: Bounds { min, max },
        }
    }

    /// Create camera centered at origin with given size
    pub fn centered(width: f32, height: f32) -> Self {
        let half_w = width / 2.0;
        let half_h = height / 2.0;
        Self::new([-half_w, -half_h], [half_w, half_h])
    }

    /// Get the current view bounds
    pub fn view_bounds(&self) -> &Bounds {
        &self.bounds
    }

    /// Pan the camera by delta in world units
    pub fn pan(&mut self, delta: [f32; 2]) {
        self.bounds.min[0] += delta[0];
        self.bounds.min[1] += delta[1];
        self.bounds.max[0] += delta[0];
        self.bounds.max[1] += delta[1];
    }

    /// Zoom in/out by changing bounds size around center
    /// scale > 1.0 = zoom out, scale < 1.0 = zoom in
    pub fn zoom(&mut self, scale: f32) {
        let center = self.bounds.center();
        let width = self.bounds.width() * scale;
        let height = self.bounds.height() * scale;

        self.bounds.min = [center[0] - width / 2.0, center[1] - height / 2.0];
        self.bounds.max = [center[0] + width / 2.0, center[1] + height / 2.0];
    }

    /// Set camera to view specific bounds
    pub fn set_bounds(&mut self, min: [f32; 2], max: [f32; 2]) {
        self.bounds.min = min;
        self.bounds.max = max;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl Bounds {
    pub fn new(min: [f32; 2], max: [f32; 2]) -> Self {
        Self { min, max }
    }

    pub fn width(&self) -> f32 {
        self.max[0] - self.min[0]
    }

    pub fn height(&self) -> f32 {
        self.max[1] - self.min[1]
    }

    pub fn center(&self) -> [f32; 2] {
        [
            (self.min[0] + self.max[0]) / 2.0,
            (self.min[1] + self.max[1]) / 2.0,
        ]
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width() / self.height()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_bounds_and_dimensions() {
        let camera = Camera::new([-1.0, -2.0], [3.0, 4.0]);
        assert_eq!(camera.bounds.width(), 4.0);
        assert_eq!(camera.bounds.height(), 6.0);
        assert_eq!(camera.bounds.center(), [1.0, 1.0]);
        assert!((camera.bounds.aspect_ratio() - (4.0 / 6.0)).abs() < 0.001);
    }

    #[test]
    fn test_camera_centered() {
        let camera = Camera::centered(4.0, 6.0);
        assert_eq!(camera.bounds.min, [-2.0, -3.0]);
        assert_eq!(camera.bounds.max, [2.0, 3.0]);
        assert_eq!(camera.bounds.center(), [0.0, 0.0]);
    }

    #[test]
    fn test_camera_pan() {
        let mut camera = Camera::centered(4.0, 6.0);
        camera.pan([1.0, -2.0]);
        assert_eq!(camera.bounds.min, [-1.0, -5.0]);
        assert_eq!(camera.bounds.max, [3.0, 1.0]);
        assert_eq!(camera.bounds.center(), [1.0, -2.0]);
    }

    #[test]
    fn test_camera_zoom() {
        let mut camera = Camera::centered(4.0, 6.0);
        camera.zoom(2.0); // Zoom out
        assert_eq!(camera.bounds.width(), 8.0);
        assert_eq!(camera.bounds.height(), 12.0);
        assert_eq!(camera.bounds.center(), [0.0, 0.0]); // Center unchanged
    }

    #[test]
    fn test_camera_zoom_in() {
        let mut camera = Camera::centered(4.0, 6.0);
        camera.zoom(0.5); // Zoom in
        assert_eq!(camera.bounds.width(), 2.0);
        assert_eq!(camera.bounds.height(), 3.0);
        assert_eq!(camera.bounds.center(), [0.0, 0.0]); // Center unchanged
    }

    #[test]
    fn test_camera_set_bounds() {
        let mut camera = Camera::centered(4.0, 6.0);
        camera.set_bounds([-10.0, -20.0], [10.0, 20.0]);
        assert_eq!(camera.bounds.min, [-10.0, -20.0]);
        assert_eq!(camera.bounds.max, [10.0, 20.0]);
    }
}
