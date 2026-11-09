#[cfg(target_os = "linux")]
mod uinput;

#[cfg(any(target_os = "linux", test))]
mod geometry;
#[cfg(target_os = "linux")]
mod output;

use crate::{
    Error, check_child,
    process::{
        ChildGuard,
        hover::{CpuTracker, Resources},
    },
    protocol::Connection,
};
use benchmark_common::{Command, Event, hover};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[cfg(target_os = "linux")]
pub use crate::hover::uinput::Driver;

#[cfg(not(target_os = "linux"))]
pub struct Driver;

#[cfg(not(target_os = "linux"))]
impl Driver {
    pub fn connect() -> Result<(Self, Event), Error> {
        Err("Native hover input requires a Linux Wayland desktop and uinput".into())
    }

    pub fn output_dimensions(&self) -> Result<(u32, u32), Error> {
        Err("Native hover input requires a Linux Wayland desktop and uinput".into())
    }

    pub fn validate_viewport(&self, _: f32, _: f32) -> Result<(), Error> {
        Err("Native hover input requires a Linux Wayland desktop and uinput".into())
    }

    pub fn park(&mut self) -> Result<(), Error> {
        Err("Native hover input requires a Linux Wayland desktop and uinput".into())
    }

    pub fn submit(&mut self, _: usize) -> Result<(), Error> {
        self.park()
    }
}

pub fn validate_ready(
    backend: &str,
    scale_factor: f64,
    logical_width: f32,
    logical_height: f32,
) -> Result<(), Error> {
    hover::validate_viewport(backend, scale_factor, logical_width, logical_height)
        .map_err(|message| io::Error::new(io::ErrorKind::Unsupported, message).into())
}

#[derive(Default)]
struct Schedule {
    attempted_motions: u64,
    submitted_cells: Vec<usize>,
    submitted_elapsed_ms: Vec<f64>,
    lateness_ms: Vec<f64>,
    endpoint_sample_elapsed_ms: Option<f64>,
    endpoint_elapsed_ms: Option<f64>,
    finish_request_elapsed_ms: Option<f64>,
}

impl Schedule {
    fn due(&self, elapsed: Duration, duration: Duration) -> Result<Option<usize>, Error> {
        let next = self.submitted_cells.len() as u64;
        if next == hover::motion_count(duration) {
            return Ok(None);
        }
        let deadline = hover::deadline(next);
        if elapsed >= duration || elapsed.saturating_sub(deadline) > hover::MAX_LATENESS {
            return Err(
                "Hover input deadline missed by more than 5ms, catch-up motions are forbidden"
                    .into(),
            );
        }
        Ok((elapsed >= deadline).then(|| hover::target(next)))
    }

    fn submitted(&mut self, cell: usize, elapsed: Duration) -> Result<(), Error> {
        let next = self.submitted_cells.len() as u64;
        if cell != hover::target(next) {
            return Err("Invalid submitted hover target".into());
        }
        let lateness = elapsed.saturating_sub(hover::deadline(next));
        self.submitted_cells.push(cell);
        self.submitted_elapsed_ms
            .push(elapsed.as_secs_f64() * 1000.0);
        self.lateness_ms.push(lateness.as_secs_f64() * 1000.0);
        if lateness > hover::MAX_LATENESS {
            return Err("Hover input submission exceeded 5ms lateness tolerance".into());
        }
        Ok(())
    }

    fn finish(&self, elapsed: Duration, duration: Duration) -> Result<(), Error> {
        if elapsed < duration || elapsed - duration > hover::MAX_LATENESS {
            return Err("Hover phase endpoint missed the 5ms timing tolerance".into());
        }
        if self.submitted_cells.len() as u64 != hover::motion_count(duration) {
            return Err("Incomplete hover trajectory at phase deadline".into());
        }
        Ok(())
    }

    fn finish_sampling(&mut self, elapsed: Duration, duration: Duration) -> Result<(), Error> {
        self.endpoint_sample_elapsed_ms = Some(elapsed.as_secs_f64() * 1000.0);
        self.finish(elapsed, duration)
            .map_err(|error| format!("Hover CPU snapshot: {error}").into())
    }

    fn run(
        &mut self,
        duration: Duration,
        mut elapsed: impl FnMut() -> Duration,
        mut submit: impl FnMut(usize) -> Result<(), Error>,
        mut service: impl FnMut() -> Result<(), Error>,
        mut idle: impl FnMut(),
    ) -> Result<(), Error> {
        loop {
            service()?;
            if let Some(cell) = self.due(elapsed(), duration)? {
                self.attempted_motions += 1;
                submit(cell)?;
                self.submitted(cell, elapsed())?;
            }
            let current = elapsed();
            if current >= duration {
                self.endpoint_elapsed_ms = Some(current.as_secs_f64() * 1000.0);
                return self.finish(current, duration);
            }
            idle();
        }
    }

    fn verify_completion(
        &self,
        observations: &hover::Observations,
        duration: Duration,
    ) -> Result<(), Error> {
        if self.submitted_cells.len() as u64 != hover::motion_count(duration) {
            return Err("Hover phase ended before every native motion was submitted".into());
        }
        observations.verify(self.submitted_cells.len() as u64)?;
        if observations.cells != self.submitted_cells {
            return Err("Native hover observations differ from actual submitted cells".into());
        }
        Ok(())
    }

    fn event(&self, phase: &str, duration: Duration, verified: bool) -> Event {
        Event::HoverInput {
            phase: phase.to_owned(),
            backend: "linux-wayland-uinput-absolute-fullscreen-v3-60hz-scale1".into(),
            expected_motions: hover::motion_count(duration),
            attempted_motions: self.attempted_motions,
            submitted_motions: self.submitted_cells.len() as u64,
            submitted_cells: self.submitted_cells.clone(),
            submitted_elapsed_ms: self.submitted_elapsed_ms.clone(),
            lateness_ms: self.lateness_ms.clone(),
            endpoint_sample_elapsed_ms: self.endpoint_sample_elapsed_ms,
            endpoint_elapsed_ms: self.endpoint_elapsed_ms,
            finish_request_elapsed_ms: self.finish_request_elapsed_ms,
            verified,
        }
    }
}

pub fn reset_phase(
    connection: &mut Connection,
    child: &mut ChildGuard,
    driver: &mut Driver,
) -> Result<(), Error> {
    driver.park()?;
    connection.send(&Command::Reset)?;
    let waiting = Instant::now();
    let mut acknowledged = None;
    loop {
        for event in connection.receive()? {
            match event {
                Event::HoverReset => acknowledged = Some(Instant::now()),
                Event::Failure { message } => return Err(message.into()),
                Event::Started | Event::Completed { .. } | Event::HoverCompleted { .. } => {
                    return Err("Unexpected phase event during untimed hover reset".into());
                }
                _ => {}
            }
        }
        if acknowledged.is_some_and(|reset| reset.elapsed() >= hover::DRAIN_TIMEOUT) {
            return Ok(());
        }
        check_child(child, waiting, Duration::from_secs(3))?;
        thread::sleep(Duration::from_millis(1));
    }
}

struct InputOutcome {
    schedule: Schedule,
    cpu: Option<CpuTracker>,
    result: Result<(), String>,
}

pub fn wait_phase(
    connection: &mut Connection,
    child: &mut ChildGuard,
    driver: &mut Driver,
    duration: Duration,
    phase: &str,
    mut resources: Option<Resources<'_>>,
) -> Result<(), Error> {
    let waiting = Instant::now();
    let mut started = None;
    let mut finishing = None;
    let mut next_sample = Duration::from_millis(100);
    let cancelled = AtomicBool::new(false);
    let (start_sender, start_receiver) = mpsc::channel::<Instant>();
    let (updates, snapshots) = mpsc::channel();
    let (completed, completion) = mpsc::channel();
    let mut cpu = match resources.as_mut() {
        Some(resources) => Some(
            resources
                .cpu
                .take()
                .ok_or("Missing prepared Hover CPU baseline")?,
        ),
        None => None,
    };
    let mut outcome: Option<InputOutcome> = None;
    let result = thread::scope(|scope| {
        let cancelled = &cancelled;
        let scheduler = thread::Builder::new()
            .name("hover-input".into())
            .spawn_scoped(scope, move || {
                let mut schedule = Schedule::default();
                let result = (|| -> Result<(), Error> {
                    let origin = start_receiver
                        .recv()
                        .map_err(|_| "Hover input start cancelled")?;
                    schedule.run(
                        duration,
                        || origin.elapsed(),
                        |cell| driver.submit(cell),
                        || {
                            if cancelled.load(Ordering::Relaxed) {
                                return Err("Hover input cancelled".into());
                            }
                            if let Some(cpu) = &mut cpu {
                                for snapshot in snapshots.try_iter() {
                                    cpu.update(snapshot)?;
                                }
                            }
                            Ok(())
                        },
                        || thread::sleep(Duration::from_millis(1)),
                    )?;
                    if let Some(cpu) = &mut cpu {
                        cpu.finish()?;
                        schedule.finish_sampling(
                            cpu.endpoint_elapsed(origin)
                                .ok_or("Missing Hover CPU endpoint")?,
                            duration,
                        )?;
                    }
                    Ok(())
                })()
                .map_err(|error| error.to_string());
                let _ = completed.send(InputOutcome {
                    schedule,
                    cpu,
                    result,
                });
            })?;
        let result = (|| -> Result<(), Error> {
            loop {
                for event in connection.receive()? {
                    match event {
                        Event::Started if started.is_none() => {
                            let origin = Instant::now();
                            started = Some(origin);
                            if let Some(resources) = &mut resources {
                                resources.set_origin(origin);
                            }
                            start_sender
                                .send(origin)
                                .map_err(|_| "Hover scheduler disconnected before start")?;
                        }
                        Event::Started => return Err("Duplicate Hover Started event".into()),
                        Event::HoverCompleted {
                            elapsed_ms,
                            observations,
                            verified,
                        } => {
                            if finishing.is_none()
                                || elapsed_ms < duration.as_secs_f64() * 1000.0 - 1.0
                            {
                                return Err(
                                    "Hover completed outside native input finish boundary".into()
                                );
                            }
                            if !verified {
                                return Err(
                                    "Adapter rejected native hover observation verification".into(),
                                );
                            }
                            outcome
                                .as_ref()
                                .ok_or("Missing Hover scheduler result")?
                                .schedule
                                .verify_completion(&observations, duration)?;
                            return Ok(());
                        }
                        Event::Completed { .. } => {
                            return Err(
                                "Programmatic completion cannot verify native hover input".into()
                            );
                        }
                        Event::Failure { message } => return Err(message.into()),
                        _ => {}
                    }
                }
                if let Some(origin) = started {
                    if outcome.is_none() {
                        match completion.try_recv() {
                            Ok(value) => outcome = Some(value),
                            Err(mpsc::TryRecvError::Empty) => {}
                            Err(mpsc::TryRecvError::Disconnected) => {
                                return Err("Hover scheduler disconnected".into());
                            }
                        }
                        if let Some(outcome) = &mut outcome {
                            if let Err(message) = &outcome.result {
                                return Err(message.clone().into());
                            }
                            connection.send(&Command::FinishHover {
                                submitted_motions: outcome.schedule.submitted_cells.len() as u64,
                            })?;
                            let requested = Instant::now();
                            outcome.schedule.finish_request_elapsed_ms =
                                Some(requested.duration_since(origin).as_secs_f64() * 1000.0);
                            finishing = Some(requested);
                            if let Some(resources) = &mut resources {
                                resources.finish_memory(origin, duration)?;
                            }
                        } else if origin.elapsed() >= next_sample && origin.elapsed() < duration {
                            if let Some(resources) = &mut resources {
                                resources.periodic(origin, duration, &updates)?;
                            }
                            while next_sample <= origin.elapsed() {
                                next_sample += Duration::from_millis(100);
                            }
                        }
                    }
                }
                if finishing.is_some_and(|requested| {
                    requested.elapsed() > hover::DRAIN_TIMEOUT + Duration::from_secs(2)
                }) {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "Hover native observation drain timed out",
                    )
                    .into());
                }
                check_child(child, waiting, duration + Duration::from_secs(10))?;
                thread::sleep(Duration::from_millis(1));
            }
        })();
        cancelled.store(true, Ordering::Relaxed);
        drop(start_sender);
        if scheduler.join().is_err() {
            return Err("Hover scheduler panicked".into());
        }
        if outcome.is_none() {
            outcome = completion.try_recv().ok();
        }
        result
    });
    if let Some(outcome) = outcome {
        connection
            .events
            .push(outcome.schedule.event(phase, duration, result.is_ok()));
        if let (Some(resources), Some(cpu), Some(origin)) = (&mut resources, outcome.cpu, started) {
            connection
                .events
                .push(resources.event(phase, origin, &cpu, result.is_ok()));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::hover::Schedule;
    use benchmark_common::{Event, hover};
    use std::time::Duration;

    #[test]
    fn scoped_scheduler_keeps_virtual_sixty_hz_while_resource_collection_waits() {
        use std::{cell::Cell, sync::mpsc, thread};
        let (resource_started, resource_waiting) = mpsc::channel();
        let (finished, input_finished) = mpsc::channel();
        thread::scope(|scope| {
            scope.spawn(move || {
                resource_waiting.recv().unwrap();
                let elapsed = Cell::new(Duration::ZERO);
                let mut schedule = Schedule::default();
                schedule
                    .run(
                        Duration::from_secs(1),
                        || elapsed.get(),
                        |_| Ok(()),
                        || Ok(()),
                        || elapsed.set(elapsed.get() + Duration::from_millis(1)),
                    )
                    .unwrap();
                finished.send(schedule).unwrap();
            });
            resource_started.send(()).unwrap();
            // Resource acquisition remains blocked until the independent input phase finishes.
            let schedule = input_finished.recv().unwrap();
            assert_eq!(schedule.submitted_cells.len(), 60);
            assert_eq!(schedule.endpoint_elapsed_ms, Some(1000.0));
            assert!(schedule.lateness_ms.iter().all(|lateness| *lateness <= 1.0));
        });
    }

    #[test]
    fn scheduler_rejects_driver_overruns_and_cancels_without_catch_up() {
        use std::cell::Cell;
        let elapsed = Cell::new(Duration::ZERO);
        let mut schedule = Schedule::default();
        assert!(
            schedule
                .run(
                    Duration::from_secs(1),
                    || elapsed.get(),
                    |_| {
                        elapsed.set(elapsed.get() + Duration::from_millis(6));
                        Ok(())
                    },
                    || Ok(()),
                    || {}
                )
                .is_err()
        );
        assert_eq!(schedule.attempted_motions, 1);
        assert_eq!(schedule.submitted_cells, vec![0]);
        assert_eq!(schedule.lateness_ms, vec![6.0]);
        let mut schedule = Schedule::default();
        assert!(
            schedule
                .run(
                    Duration::from_secs(1),
                    || Duration::ZERO,
                    |_| Ok(()),
                    || Err("cancelled".into()),
                    || {}
                )
                .is_err()
        );
        assert_eq!(schedule.attempted_motions, 0);
        assert!(schedule.endpoint_elapsed_ms.is_none());
    }

    #[test]
    fn readiness_accepts_actual_fullscreen_dimensions_and_requires_scale1() {
        for backend in ["wayland", "unspecified"] {
            crate::hover::validate_ready(backend, 1.0, 1920.0, 1080.0).unwrap();
        }
        for backend in ["", " "] {
            assert!(crate::hover::validate_ready(backend, 1.0, 1920.0, 1080.0).is_err());
        }
        for scale in [0.0, 1.5, 2.0, f64::NAN, f64::INFINITY] {
            assert!(crate::hover::validate_ready("wayland", scale, 1920.0, 1080.0).is_err());
        }
        assert!(crate::hover::validate_ready("wayland", 1.0, 740.0, 1080.0).is_err());
    }

    #[test]
    fn deadlines_fail_late_without_catch_up_and_record_actual_cadence() {
        let duration = Duration::from_secs(1);
        let mut schedule = Schedule::default();
        assert_eq!(schedule.due(Duration::ZERO, duration).unwrap(), Some(0));
        schedule.submitted(0, Duration::from_millis(1)).unwrap();
        assert_eq!(
            schedule.due(Duration::from_millis(10), duration).unwrap(),
            None
        );
        assert_eq!(schedule.due(hover::deadline(1), duration).unwrap(), Some(1));
        assert!(
            schedule
                .due(
                    hover::deadline(1) + hover::MAX_LATENESS + Duration::from_nanos(1),
                    duration
                )
                .is_err()
        );
        assert!(schedule.due(duration, duration).is_err());
        assert_eq!(schedule.submitted_elapsed_ms, vec![1.0]);
        assert_eq!(schedule.lateness_ms, vec![1.0]);
        let mut late = Schedule::default();
        assert!(late.submitted(0, Duration::from_millis(6)).is_err());
        assert_eq!(late.submitted_cells, vec![0]);
    }

    #[test]
    fn endpoint_cpu_sampling_records_actual_time_and_rejects_overruns() {
        let duration = Duration::from_millis(50);
        let mut schedule = Schedule::default();
        for motion in 0..3 {
            schedule
                .submitted(hover::target(motion), hover::deadline(motion))
                .unwrap();
        }
        let sampled_elapsed = duration + Duration::from_millis(2);
        schedule.finish_sampling(sampled_elapsed, duration).unwrap();
        assert_eq!(schedule.endpoint_sample_elapsed_ms, Some(52.0));
        schedule
            .finish_sampling(duration + hover::MAX_LATENESS, duration)
            .unwrap();
        assert!(
            schedule
                .finish_sampling(duration - Duration::from_nanos(1), duration)
                .is_err()
        );
        assert!(
            schedule
                .finish_sampling(duration + Duration::from_millis(6), duration)
                .is_err()
        );
        let event = schedule.event("measurement", duration, false);
        let value = serde_json::to_value(event).unwrap();
        assert_eq!(value["endpoint_sample_elapsed_ms"], 56.0);
        assert_eq!(value["verified"], false);
    }

    #[test]
    fn uinput_schedule_records_driver_not_toolkit_backend_and_requires_native_delivery() {
        let mut schedule = Schedule::default();
        schedule.submitted(0, Duration::ZERO).unwrap();
        let duration = Duration::from_millis(1);
        let value = serde_json::to_value(schedule.event("measurement", duration, false)).unwrap();
        assert_eq!(
            value["backend"],
            "linux-wayland-uinput-absolute-fullscreen-v3-60hz-scale1"
        );
        assert_eq!(value["verified"], false);
        assert!(
            schedule
                .verify_completion(&hover::Observations::default(), duration)
                .is_err()
        );
        assert!(
            schedule
                .verify_completion(
                    &hover::Observations {
                        cells: vec![0],
                        invalid: false
                    },
                    duration
                )
                .is_ok()
        );
    }

    #[test]
    fn completion_requires_submissions_and_exact_native_order() {
        let duration = Duration::from_millis(50);
        let mut schedule = Schedule::default();
        let observations = hover::Observations {
            cells: vec![0, 1, 2],
            invalid: false,
        };
        assert!(schedule.verify_completion(&observations, duration).is_err());
        assert!(schedule.finish(duration, duration).is_err());
        for motion in 0..3 {
            schedule
                .submitted(hover::target(motion), hover::deadline(motion))
                .unwrap();
        }
        assert_eq!(schedule.due(duration, duration).unwrap(), None);
        schedule.verify_completion(&observations, duration).unwrap();
        schedule.finish(duration, duration).unwrap();
        schedule
            .finish(duration + hover::MAX_LATENESS, duration)
            .unwrap();
        assert!(
            schedule
                .finish(duration - Duration::from_nanos(1), duration)
                .is_err()
        );
        assert!(
            schedule
                .finish(
                    duration + hover::MAX_LATENESS + Duration::from_nanos(1),
                    duration
                )
                .is_err()
        );
        assert!(
            schedule
                .verify_completion(
                    &hover::Observations {
                        cells: vec![0, 2, 1],
                        invalid: false
                    },
                    duration
                )
                .is_err()
        );
        assert!(
            schedule
                .verify_completion(
                    &hover::Observations {
                        cells: vec![0, 1],
                        invalid: false
                    },
                    duration
                )
                .is_err()
        );
        let event = schedule.event("measurement", duration, true);
        let value = serde_json::to_value(&event).unwrap();
        assert_eq!(value["event"], "hover-input");
        assert_eq!(value["submitted_cells"], serde_json::json!([0, 1, 2]));
        assert!(matches!(
            serde_json::from_value::<Event>(value).unwrap(),
            Event::HoverInput { verified: true, .. }
        ));
    }
}
