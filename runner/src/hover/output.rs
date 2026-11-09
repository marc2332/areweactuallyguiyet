use crate::{Error, hover::geometry::Geometry};
use benchmark_common::Event;
use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum,
    protocol::{wl_output, wl_registry},
};

struct Output {
    global: u32,
    mode: Option<(i32, i32)>,
    scale: i32,
    transform: Option<WEnum<wl_output::Transform>>,
}

impl Default for Output {
    fn default() -> Self {
        Self {
            global: 0,
            mode: None,
            // wl_output defines scale1 when no scale event is sent.
            scale: 1,
            transform: None,
        }
    }
}

#[derive(Default)]
struct Outputs(Vec<Output>);

impl Outputs {
    fn geometry(&self) -> Result<Geometry, Error> {
        if self.0.len() != 1 {
            return Err(format!(
                "Hover requires exactly one advertised connected wl_output, found {}. Disable extra outputs before running Hover",
                self.0.len()
            ).into());
        }
        let output = &self.0[0];
        if output.scale != 1 || output.transform != Some(WEnum::Value(wl_output::Transform::Normal))
        {
            return Err("Hover requires wl_output scale1 and normal unrotated transform. Set the single Wayland output to scale1 without rotation or reflection".into());
        }
        let (width, height) = output.mode.ok_or(
            "Hover requires a wl_output current mode. Enable the single output with a current landscape mode",
        )?;
        Geometry::new(
            u32::try_from(width).map_err(|_| "Invalid Wayland output width")?,
            u32::try_from(height).map_err(|_| "Invalid Wayland output height")?,
        )
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for Outputs {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        queue: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } if interface == "wl_output" => {
                state.0.push(Output {
                    global: name,
                    ..Output::default()
                });
                registry.bind::<wl_output::WlOutput, _, _>(name, version.min(4), queue, name);
            }
            wl_registry::Event::GlobalRemove { name } => {
                state.0.retain(|output| output.global != name);
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, u32> for Outputs {
    fn event(
        state: &mut Self,
        _: &wl_output::WlOutput,
        event: wl_output::Event,
        global: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(output) = state.0.iter_mut().find(|output| output.global == *global) else {
            return;
        };
        match event {
            wl_output::Event::Geometry { transform, .. } => output.transform = Some(transform),
            wl_output::Event::Scale { factor } => output.scale = factor,
            wl_output::Event::Mode {
                flags: WEnum::Value(flags),
                width,
                height,
                ..
            } if flags.contains(wl_output::Mode::Current) => output.mode = Some((width, height)),
            _ => {}
        }
    }
}

pub fn query() -> Result<(Geometry, Event), Error> {
    let connection = Connection::connect_to_env().map_err(|error| {
        format!(
            "Hover requires a Linux Wayland desktop with an accessible WAYLAND_DISPLAY: {error}"
        )
    })?;
    let mut queue = connection.new_event_queue();
    let _registry = connection.display().get_registry(&queue.handle(), ());
    let mut outputs = Outputs::default();
    queue.roundtrip(&mut outputs)?;
    queue.roundtrip(&mut outputs)?;
    let geometry = outputs.geometry()?;
    Ok((
        geometry,
        Event::HoverGeometry {
            output_global: outputs.0[0].global,
            output_width: geometry.width,
            output_height: geometry.height,
            output_scale: outputs.0[0].scale,
            output_transform: "normal".into(),
            logical_width: geometry.width as f32,
            logical_height: geometry.height as f32,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::hover::output::{Output, Outputs};
    use wayland_client::{WEnum, protocol::wl_output};

    fn supported_output() -> Output {
        Output {
            global: 1,
            mode: Some((1920, 1080)),
            scale: 1,
            transform: Some(WEnum::Value(wl_output::Transform::Normal)),
        }
    }

    #[test]
    fn wayland_output_requires_exactly_one_current_scale1_normal_landscape_output() {
        assert!(Outputs::default().geometry().is_err());
        assert!(
            Outputs(vec![supported_output(), supported_output()])
                .geometry()
                .is_err()
        );
        assert!(Outputs(vec![supported_output()]).geometry().is_ok());
        let default_scale = Output {
            mode: Some((1920, 1080)),
            transform: Some(WEnum::Value(wl_output::Transform::Normal)),
            ..Output::default()
        };
        assert!(Outputs(vec![default_scale]).geometry().is_ok());
        for scale in [0, 2] {
            let mut output = supported_output();
            output.scale = scale;
            assert!(Outputs(vec![output]).geometry().is_err());
        }
        for transform in [
            None,
            Some(WEnum::Value(wl_output::Transform::_90)),
            Some(WEnum::Value(wl_output::Transform::Flipped)),
            Some(WEnum::Unknown(99)),
        ] {
            let mut output = supported_output();
            output.transform = transform;
            assert!(Outputs(vec![output]).geometry().is_err());
        }
        for mode in [
            None,
            Some((0, 0)),
            Some((-1, 1080)),
            Some((1080, 1920)),
            Some((1280, 720)),
        ] {
            let mut output = supported_output();
            output.mode = mode;
            assert!(Outputs(vec![output]).geometry().is_err());
        }
    }
}
