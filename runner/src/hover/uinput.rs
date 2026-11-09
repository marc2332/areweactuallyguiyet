use crate::{
    Error,
    hover::{
        geometry::{AXIS_MAX, Geometry, Point},
        output,
    },
};
use benchmark_common::Event;
use evdev::{
    AbsInfo, AbsoluteAxisCode, AttributeSet, EventType, InputEvent, KeyCode, PropType,
    RelativeAxisCode, UinputAbsSetup, uinput::VirtualDevice,
};
use std::io;

pub struct Driver {
    // Closing evdev's owned uinput descriptor destroys the device on every exit path.
    device: VirtualDevice,
    geometry: Geometry,
}

impl Driver {
    pub fn connect() -> Result<(Self, Event), Error> {
        let (geometry, event) = output::query()?;
        let device = Self::create().map_err(|error| {
            io::Error::new(error.kind(), format!(
                "Cannot create Hover absolute mouse through /dev/uinput: {error}. Arrange read/write uinput access outside the runner and use a compositor that accepts absolute mouse motion. No privileges or device permissions were changed"
            ))
        })?;
        Ok((Self { device, geometry }, event))
    }

    pub fn output_dimensions(&self) -> Result<(u32, u32), Error> {
        Ok((self.geometry.width, self.geometry.height))
    }

    pub fn validate_viewport(&self, width: f32, height: f32) -> Result<(), Error> {
        self.geometry.validate_viewport(width, height)
    }

    fn create() -> io::Result<VirtualDevice> {
        let info = AbsInfo::new(0, 0, AXIS_MAX, 0, 0, 0);
        VirtualDevice::builder()?
            .name("gui-bench-hover")
            .with_properties(&AttributeSet::from_iter([PropType::POINTER]))?
            .with_keys(&AttributeSet::from_iter([KeyCode::BTN_LEFT]))?
            .with_relative_axes(&AttributeSet::from_iter([
                RelativeAxisCode::REL_X,
                RelativeAxisCode::REL_Y,
            ]))?
            .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_X, info))?
            .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_Y, info))?
            .build()
    }

    fn move_to(&mut self, position: Point) -> Result<(), Error> {
        if position
            .iter()
            .any(|coordinate| !(0..=AXIS_MAX).contains(coordinate))
        {
            return Err("Hover absolute mouse position is outside device range".into());
        }
        self.device.emit(&[
            InputEvent::new(
                EventType::ABSOLUTE.0,
                AbsoluteAxisCode::ABS_X.0,
                position[0],
            ),
            InputEvent::new(
                EventType::ABSOLUTE.0,
                AbsoluteAxisCode::ABS_Y.0,
                position[1],
            ),
        ])?;
        Ok(())
    }

    pub fn park(&mut self) -> Result<(), Error> {
        self.move_to(self.geometry.park()?)
    }

    pub fn submit(&mut self, cell: usize) -> Result<(), Error> {
        self.move_to(self.geometry.cell(cell)?)
    }
}
