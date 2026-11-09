use crate::Error;
use benchmark_common::hover;

pub const AXIS_MAX: i32 = 65535;
pub const PARK: [i32; 2] = [hover::PARK.0 as i32, hover::PARK.1 as i32];
pub type Point = [i32; 2];

#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    pub width: u32,
    pub height: u32,
}

impl Geometry {
    pub fn new(width: u32, height: u32) -> Result<Self, Error> {
        if width <= height || height <= PARK[1] as u32 {
            return Err(format!(
                "Hover requires a landscape output wider than its height with both dimensions >740 pixels, got {width}x{height}. Use one unrotated scale1 Wayland output with room for the grid and park"
            ).into());
        }
        Ok(Self { width, height })
    }

    pub fn validate_viewport(self, width: f32, height: f32) -> Result<(), Error> {
        if f64::from(width) != f64::from(self.width) || f64::from(height) != f64::from(self.height)
        {
            return Err(format!(
                "Hover actual logical viewport {width}x{height} must equal the single Wayland output {}x{}. Request fullscreen and report the actual client dimensions at scale1",
                self.width, self.height
            ).into());
        }
        Ok(())
    }

    pub fn point(self, position: Point) -> Result<Point, Error> {
        let mut normalized = [0; 2];
        for (axis, dimension) in [self.width, self.height].into_iter().enumerate() {
            if position[axis] < 0 || position[axis] as u32 >= dimension {
                return Err("Hover local position is outside the fullscreen output".into());
            }
            normalized[axis] = (f64::from(position[axis]) * f64::from(AXIS_MAX)
                / f64::from(dimension))
            .round() as i32;
        }
        Ok(normalized)
    }

    pub fn cell(self, index: usize) -> Result<Point, Error> {
        let (x, y) = hover::center(index).ok_or("Invalid hover cell index")?;
        self.point([i32::from(x), i32::from(y)])
    }

    pub fn park(self) -> Result<Point, Error> {
        self.point(PARK)
    }
}

#[cfg(test)]
mod tests {
    use crate::hover::geometry::{AXIS_MAX, Geometry};
    use benchmark_common::hover;

    #[test]
    fn fullscreen_mapping_uses_output_dimensions_for_first_last_cells_and_park() {
        let geometry = Geometry::new(1920, 1080).unwrap();
        assert_eq!(geometry.point([0, 0]).unwrap(), [0, 0]);
        assert_eq!(geometry.cell(0).unwrap(), [410, 728]);
        assert_eq!(geometry.cell(899).unwrap(), [24166, 42962]);
        assert_eq!(geometry.cell(hover::target(899)).unwrap(), [410, 42962]);
        assert_eq!(geometry.park().unwrap(), [25258, 44904]);
        for index in 0..hover::COUNT {
            assert!(
                geometry
                    .cell(index)
                    .unwrap()
                    .iter()
                    .all(|axis| (0..=AXIS_MAX).contains(axis))
            );
        }
        assert!(geometry.cell(hover::COUNT).is_err());
        for point in [[-1, 0], [0, -1], [1920, 0], [0, 1080], [i32::MAX, 0]] {
            assert!(geometry.point(point).is_err());
        }
        assert!(
            geometry
                .point([1919, 1079])
                .unwrap()
                .iter()
                .all(|axis| (0..=AXIS_MAX).contains(axis))
        );
    }

    #[test]
    fn fullscreen_geometry_rejects_small_portrait_square_and_mismatched_viewports() {
        for (width, height) in [(0, 0), (1280, 740), (720, 800), (1080, 1920), (800, 800)] {
            assert!(Geometry::new(width, height).is_err());
        }
        let geometry = Geometry::new(1920, 1080).unwrap();
        geometry.validate_viewport(1920.0, 1080.0).unwrap();
        for (width, height) in [
            (1280.0, 800.0),
            (1920.0, 1079.0),
            (f32::NAN, 1080.0),
            (f32::INFINITY, 1080.0),
        ] {
            assert!(geometry.validate_viewport(width, height).is_err());
        }
        Geometry::new(742, 741).unwrap().park().unwrap();
    }
}
