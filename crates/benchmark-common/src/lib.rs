pub mod dashboard;
pub mod hover;

pub use hover::HoverObserver;

use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};

pub const PROTOCOL_VERSION: u32 = 1;
pub const WIDTH: f32 = 1280.0;
pub const HEIGHT: f32 = 800.0;
pub const ROW_HEIGHT: f32 = 48.0;
pub const SWITCH_COLUMNS: usize = 30;
pub const SWITCH_ROWS: usize = 30;
pub const SWITCH_COUNT: usize = SWITCH_COLUMNS * SWITCH_ROWS;
pub const SWITCH_CELL_SIZE: f32 = 30.0;
pub const SWITCH_ON_COLOR: (u8, u8, u8) = (47, 158, 68);
pub const SWITCH_OFF_COLOR: (u8, u8, u8) = (134, 142, 150);
pub const SWITCH_TEXT_COLOR: (u8, u8, u8) = (255, 255, 255);
pub const SWITCH_FONT_SIZE: f32 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scenario {
    Startup,
    Scroll,
    VirtualScroll,
    Switches,
    Hover,
    Memory,
}

impl Scenario {
    pub const ALL: [Self; 6] = [
        Self::Startup,
        Self::Scroll,
        Self::VirtualScroll,
        Self::Switches,
        Self::Hover,
        Self::Memory,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Scroll => "scroll",
            Self::VirtualScroll => "virtual-scroll",
            Self::Switches => "switches",
            Self::Hover => "hover",
            Self::Memory => "memory",
        }
    }

    pub fn version(self) -> u32 {
        match self {
            Self::Hover => 3,
            Self::Startup | Self::Switches | Self::Memory => 2,
            _ => 1,
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|scenario| scenario.id() == value)
            .ok_or_else(|| format!("Unknown scenario: {value}"))
    }

    pub fn row_count(self) -> usize {
        match self {
            Self::VirtualScroll => 100_000,
            Self::Scroll => 2_000,
            Self::Memory => 200,
            Self::Switches | Self::Hover => 0,
            Self::Startup => 20,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SwitchMode {
    Single,
    #[default]
    Batch,
    All,
}

impl SwitchMode {
    pub fn batch_size(self) -> usize {
        match self {
            Self::Single => 1,
            Self::Batch => 90,
            Self::All => SWITCH_COUNT,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Command {
    Prepare,
    Start {
        duration_ms: u64,
        switch_mode: SwitchMode,
    },
    Reset,
    FinishHover {
        submitted_motions: u64,
    },
    Stop,
    Quit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "kebab-case")]
pub enum Event {
    Hello {
        protocol: u32,
        scenario: Scenario,
        process_id: u32,
    },
    Ready {
        boundary: String,
    },
    Started,
    HoverReset,
    HoverReady {
        backend: String,
        scale_factor: f64,
        logical_width: f32,
        logical_height: f32,
    },
    HoverGeometry {
        output_global: u32,
        output_width: u32,
        output_height: u32,
        output_scale: i32,
        output_transform: String,
        logical_width: f32,
        logical_height: f32,
    },
    HoverCompleted {
        elapsed_ms: f64,
        observations: hover::Observations,
        verified: bool,
    },
    HoverResources {
        phase: String,
        root_process_id: u32,
        root_start_ticks: u64,
        new_process_start_ticks: u64,
        cpu_baseline: hover::ResourceSampleTiming,
        cpu_endpoint: Option<hover::ResourceSampleTiming>,
        membership_scans: Vec<hover::ResourceSampleTiming>,
        memory_samples: Vec<hover::ResourceSampleTiming>,
        final_memory_completed_ms: Option<f64>,
        maximum_memory_lateness_ms: f64,
        cpu_ms: f64,
        verified: bool,
    },
    HoverInput {
        phase: String,
        backend: String,
        expected_motions: u64,
        attempted_motions: u64,
        submitted_motions: u64,
        submitted_cells: Vec<usize>,
        submitted_elapsed_ms: Vec<f64>,
        lateness_ms: Vec<f64>,
        /// Legacy field storing CPU-counter completion in the Hover v2 resource contract.
        #[serde(default)]
        endpoint_sample_elapsed_ms: Option<f64>,
        #[serde(default)]
        endpoint_elapsed_ms: Option<f64>,
        #[serde(default)]
        finish_request_elapsed_ms: Option<f64>,
        verified: bool,
    },
    Progress {
        elapsed_ms: f64,
        scheduled_updates: u64,
        applied_updates: u64,
        scroll_offset: f32,
    },
    Completed {
        elapsed_ms: f64,
        scheduled_updates: u64,
        applied_updates: u64,
    },
    Dashboard {
        snapshot: dashboard::Snapshot,
    },
    Failure {
        message: String,
    },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FrameState {
    pub running: bool,
    pub should_quit: bool,
    pub elapsed: Duration,
    pub update_index: u64,
    pub scroll_offset: f32,
    pub reset_generation: u64,
}

pub struct Harness {
    stream: Option<TcpStream>,
    input: Vec<u8>,
    output: VecDeque<u8>,
    scenario: Scenario,
    started: Option<Instant>,
    duration: Duration,
    mode: SwitchMode,
    state: FrameState,
    ready: bool,
    last_progress: Instant,
    last_dashboard: Instant,
    applied_updates: u64,
    hover_observer: HoverObserver,
    hover_finish: Option<(u64, Instant)>,
}

impl Harness {
    pub fn connect(scenario: Scenario) -> io::Result<Self> {
        let stream = match std::env::var("GUI_BENCH_ADDR") {
            Ok(address) => {
                let stream = TcpStream::connect(address)?;
                stream.set_nodelay(true)?;
                stream.set_nonblocking(true)?;
                Some(stream)
            }
            Err(std::env::VarError::NotPresent) => None,
            Err(error) => return Err(io::Error::new(io::ErrorKind::InvalidInput, error)),
        };
        let mut harness = Self {
            stream,
            input: Vec::new(),
            output: VecDeque::new(),
            scenario,
            started: None,
            duration: Duration::from_secs(10),
            mode: SwitchMode::Batch,
            state: FrameState::default(),
            ready: false,
            last_progress: Instant::now(),
            last_dashboard: Instant::now(),
            applied_updates: 0,
            hover_observer: HoverObserver::default(),
            hover_finish: None,
        };
        harness.send(Event::Hello {
            protocol: PROTOCOL_VERSION,
            scenario,
            process_id: std::process::id(),
        })?;
        Ok(harness)
    }

    pub fn controlled(&self) -> bool {
        self.stream.is_some()
    }

    pub fn mark_ready(&mut self, boundary: &str) -> io::Result<()> {
        if !self.ready {
            self.ready = true;
            self.send(Event::Ready {
                boundary: boundary.to_owned(),
            })?;
        }
        Ok(())
    }

    pub fn hover_observer(&self) -> HoverObserver {
        self.hover_observer.clone()
    }

    /// Report the public native backend or "unspecified", and the actual exposed scale.
    /// Logical dimensions must come from the actual native client area or root layout.
    pub fn mark_hover_ready(
        &mut self,
        boundary: &str,
        backend: &str,
        scale_factor: f64,
        logical_width: f32,
        logical_height: f32,
    ) -> io::Result<()> {
        hover::validate_viewport(backend, scale_factor, logical_width, logical_height)
            .map_err(|message| io::Error::new(io::ErrorKind::Unsupported, message))?;
        if !self.ready {
            self.send(Event::HoverReady {
                backend: backend.to_owned(),
                scale_factor,
                logical_width,
                logical_height,
            })?;
        }
        self.mark_ready(boundary)
    }

    pub fn switch_mode(&self) -> SwitchMode {
        self.mode
    }

    pub fn poll(&mut self) -> io::Result<FrameState> {
        if self.scenario == Scenario::Hover {
            self.poll_hover_completion(Instant::now())?;
        } else if self.started.is_some() && self.state.elapsed >= self.duration {
            self.complete()?;
        }
        self.flush()?;
        if let Some(stream) = &mut self.stream {
            let mut buffer = [0; 4096];
            loop {
                match stream.read(&mut buffer) {
                    Ok(0) => {
                        self.state.should_quit = true;
                        break;
                    }
                    Ok(length) => self.input.extend_from_slice(&buffer[..length]),
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                    Err(error) => return Err(error),
                }
                if self.input.len() > 65_536 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Control message too large",
                    ));
                }
            }
        }
        while let Some(newline) = self.input.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = self.input.drain(..=newline).collect();
            let command = serde_json::from_slice::<Command>(&line)?;
            match command {
                Command::Prepare | Command::Reset => {
                    self.started = None;
                    self.applied_updates = 0;
                    self.hover_observer.reset(false);
                    self.hover_finish = None;
                    self.state = FrameState {
                        reset_generation: self.state.reset_generation + 1,
                        ..FrameState::default()
                    };
                    if self.scenario == Scenario::Hover {
                        self.send(Event::HoverReset)?;
                    }
                }
                Command::Start {
                    duration_ms,
                    switch_mode,
                } => {
                    if duration_ms == 0 || duration_ms > 300_000 {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "Duration must be in 1..=300000 ms",
                        ));
                    }
                    self.started = Some(Instant::now());
                    self.duration = Duration::from_millis(duration_ms);
                    self.mode = switch_mode;
                    self.state.running = true;
                    self.state.elapsed = Duration::ZERO;
                    self.state.update_index = 0;
                    self.applied_updates = 0;
                    self.hover_observer.reset(self.scenario == Scenario::Hover);
                    self.hover_finish = None;
                    self.send(Event::Started)?;
                }
                Command::FinishHover { submitted_motions } => {
                    if self.scenario != Scenario::Hover
                        || self.started.is_none()
                        || self.hover_finish.is_some()
                        || submitted_motions != hover::motion_count(self.duration)
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Invalid hover finish command",
                        ));
                    }
                    self.hover_finish = Some((submitted_motions, Instant::now()));
                }
                Command::Stop if self.scenario == Scenario::Hover => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Hover requires native input verification",
                    ));
                }
                Command::Stop => self.complete()?,
                Command::Quit => self.state.should_quit = true,
            }
        }
        if let Some(started) = self.started {
            self.state.elapsed = started.elapsed().min(self.duration);
            self.state.update_index = if self.scenario == Scenario::Hover {
                0
            } else {
                (self.state.elapsed.as_secs_f64() * 30.0).floor() as u64
            };
            let maximum = (self.scenario.row_count() as f32 * ROW_HEIGHT - HEIGHT).max(0.0);
            self.state.scroll_offset = scroll_offset(self.state.elapsed, maximum);
        }
        Ok(self.state)
    }

    pub fn report_applied(&mut self, applied_updates: u64) -> io::Result<()> {
        self.applied_updates = applied_updates;
        if self.state.running && self.last_progress.elapsed() >= Duration::from_millis(250) {
            self.last_progress = Instant::now();
            self.send(Event::Progress {
                elapsed_ms: self.state.elapsed.as_secs_f64() * 1000.0,
                scheduled_updates: self.state.update_index,
                applied_updates,
                scroll_offset: self.state.scroll_offset,
            })?;
        }
        Ok(())
    }

    pub fn report_dashboard(&mut self, dashboard: &dashboard::Dashboard) -> io::Result<()> {
        if self.last_dashboard.elapsed() >= Duration::from_secs(1) {
            self.last_dashboard = Instant::now();
            self.send(Event::Dashboard {
                snapshot: dashboard.snapshot(),
            })?;
        }
        Ok(())
    }

    fn poll_hover_completion(&mut self, now: Instant) -> io::Result<()> {
        if let Some((submitted_motions, requested)) = self.hover_finish {
            let observations = self.hover_observer.snapshot();
            if now.duration_since(requested) >= hover::DRAIN_TIMEOUT {
                self.hover_finish = None;
                self.started = None;
                self.state.running = false;
                self.hover_observer.reset(false);
                let verification = observations.verify(submitted_motions);
                self.send(Event::HoverCompleted {
                    elapsed_ms: self.state.elapsed.as_secs_f64() * 1000.0,
                    observations,
                    verified: verification.is_ok(),
                })?;
                if let Err(message) = verification {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, message));
                }
            }
        }
        Ok(())
    }

    fn complete(&mut self) -> io::Result<()> {
        if self.started.take().is_some() {
            self.state.running = false;
            self.send(Event::Completed {
                elapsed_ms: self.state.elapsed.as_secs_f64() * 1000.0,
                scheduled_updates: self.state.update_index,
                applied_updates: self.applied_updates,
            })?;
        }
        Ok(())
    }

    fn send(&mut self, event: Event) -> io::Result<()> {
        if self.stream.is_some() {
            self.output.extend(serde_json::to_vec(&event)?);
            self.output.push_back(b'\n');
            self.flush()?;
        }
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(stream) = &mut self.stream {
            while !self.output.is_empty() {
                let (first, _) = self.output.as_slices();
                match stream.write(first) {
                    Ok(0) => {
                        return Err(io::Error::new(
                            io::ErrorKind::WriteZero,
                            "Control connection closed",
                        ));
                    }
                    Ok(length) => {
                        self.output.drain(..length);
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(())
    }
}

pub fn scroll_offset(elapsed: Duration, maximum: f32) -> f32 {
    if maximum <= 0.0 {
        return 0.0;
    }
    let distance = (elapsed.as_secs_f64() * 2000.0) % (2.0 * f64::from(maximum));
    if distance > f64::from(maximum) {
        (2.0 * f64::from(maximum) - distance) as f32
    } else {
        distance as f32
    }
}

pub fn row_title(index: usize) -> String {
    format!("Record {index:06}")
}

pub fn row_body(index: usize) -> String {
    format!(
        "Item {index}: shared deterministic text for comparing scrolling, clipping and component state."
    )
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Switches {
    words: [u64; 15],
    applied_updates: u64,
}

impl Switches {
    pub fn get(&self, index: usize) -> bool {
        index < SWITCH_COUNT && self.words[index / 64] & (1 << (index % 64)) != 0
    }

    pub fn toggle(&mut self, index: usize) {
        if index < SWITCH_COUNT {
            self.words[index / 64] ^= 1 << (index % 64);
        }
    }

    pub fn advance(&mut self, update_index: u64, mode: SwitchMode) {
        for update in self.applied_updates..update_index {
            let batch_size = mode.batch_size();
            let start = (update % SWITCH_COUNT as u64) as usize * batch_size % SWITCH_COUNT;
            for offset in 0..batch_size {
                self.toggle((start + offset) % SWITCH_COUNT);
            }
        }
        self.applied_updates = update_index;
    }

    pub fn advance_changed(
        &mut self,
        update_index: u64,
        mode: SwitchMode,
        mut apply: impl FnMut(usize, bool),
    ) {
        let previous = self.clone();
        self.advance(update_index, mode);
        for index in 0..SWITCH_COUNT {
            let value = self.get(index);
            if previous.get(index) != value {
                apply(index, value);
            }
        }
    }

    pub fn applied_updates(&self) -> u64 {
        self.applied_updates
    }
}

#[macro_export]
macro_rules! scenario_main {
    () => {
        #[cfg(not(any(
            feature = "startup",
            feature = "scroll",
            feature = "virtual-scroll",
            feature = "switches",
            feature = "memory",
            feature = "hover"
        )))]
        compile_error!(
            "Enable exactly one scenario: startup, scroll, virtual-scroll, switches, memory, hover"
        );
        #[cfg(any(
            all(
                feature = "startup",
                any(
                    feature = "hover",
                    feature = "scroll",
                    feature = "virtual-scroll",
                    feature = "switches",
                    feature = "memory"
                )
            ),
            all(
                feature = "scroll",
                any(
                    feature = "virtual-scroll",
                    feature = "switches",
                    feature = "memory",
                    feature = "hover"
                )
            ),
            all(
                feature = "virtual-scroll",
                any(feature = "switches", feature = "memory", feature = "hover")
            ),
            all(feature = "switches", any(feature = "memory", feature = "hover")),
            all(feature = "memory", feature = "hover")
        ))]
        compile_error!("Enable only one benchmark scenario at a time");
        #[cfg(feature = "hover")]
        #[path = "benchmark-hover.rs"]
        mod selected_hover;
        #[cfg(feature = "memory")]
        #[path = "benchmark-memory.rs"]
        mod selected_memory;
        #[cfg(feature = "scroll")]
        #[path = "benchmark-scroll.rs"]
        mod selected_scroll;
        #[cfg(feature = "startup")]
        #[path = "benchmark-startup.rs"]
        mod selected_startup;
        #[cfg(feature = "switches")]
        #[path = "benchmark-switches.rs"]
        mod selected_switches;
        #[cfg(feature = "virtual-scroll")]
        #[path = "benchmark-virtual-scroll.rs"]
        mod selected_virtual_scroll;
        fn main() -> Result<(), Box<dyn std::error::Error>> {
            #[cfg(feature = "hover")]
            selected_hover::run()?;
            #[cfg(feature = "startup")]
            selected_startup::run()?;
            #[cfg(feature = "scroll")]
            selected_scroll::run()?;
            #[cfg(feature = "virtual-scroll")]
            selected_virtual_scroll::run()?;
            #[cfg(feature = "switches")]
            selected_switches::run()?;
            #[cfg(feature = "memory")]
            selected_memory::run()?;
            Ok(())
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::{Command, Harness, Scenario, SwitchMode, Switches, scroll_offset};
    use std::time::Duration;

    #[test]
    fn scenarios_round_trip() {
        for scenario in Scenario::ALL {
            assert_eq!(Scenario::parse(scenario.id()), Ok(scenario));
        }
        assert!(Scenario::parse("unknown").is_err());
    }

    #[test]
    fn scenario_versions_match_current_workloads() {
        assert_eq!(Scenario::Switches.version(), 2);
        assert_eq!(Scenario::Startup.version(), 2);
        assert_eq!(Scenario::Scroll.version(), 1);
    }

    #[test]
    fn changed_callbacks_only_receive_final_changed_cells() {
        let mut switches = Switches::default();
        let mut changes = Vec::new();
        switches.advance_changed(1, SwitchMode::Single, |index, value| {
            changes.push((index, value))
        });
        assert_eq!(changes, vec![(0, true)]);
        changes.clear();
        switches.advance_changed(1, SwitchMode::Single, |index, value| {
            changes.push((index, value))
        });
        assert!(changes.is_empty());
        let mut switches = Switches::default();
        switches.advance_changed(2, SwitchMode::All, |index, value| {
            changes.push((index, value))
        });
        assert!(changes.is_empty());
        assert_eq!(switches.applied_updates(), 2);
    }

    #[test]
    fn scrolling_reflects_at_the_end() {
        assert_eq!(scroll_offset(Duration::from_secs(1), 3000.0), 2000.0);
        assert_eq!(scroll_offset(Duration::from_secs(2), 3000.0), 2000.0);
        assert_eq!(scroll_offset(Duration::from_secs(3), 3000.0), 0.0);
    }

    #[test]
    fn completion_allows_the_final_update_before_reporting() {
        let mut harness = Harness::connect(Scenario::Switches).unwrap();
        harness.input.extend(
            serde_json::to_vec(&Command::Start {
                duration_ms: 1000,
                switch_mode: SwitchMode::All,
            })
            .unwrap(),
        );
        harness.input.push(b'\n');
        assert!(harness.poll().unwrap().running);
        harness.started = Some(std::time::Instant::now() - Duration::from_secs(2));
        let final_frame = harness.poll().unwrap();
        assert!(final_frame.running);
        assert_eq!(final_frame.update_index, 30);
        harness.report_applied(30).unwrap();
        assert!(!harness.poll().unwrap().running);
        assert_eq!(harness.applied_updates, 30);
        harness.input.extend(b"{\"command\":\"reset\"}\n");
        let reset = harness.poll().unwrap();
        assert_eq!(reset.update_index, 0);
        assert_eq!(reset.reset_generation, 1);
        assert_eq!(harness.applied_updates, 0);
    }

    fn hover_harness() -> Harness {
        Harness {
            stream: None,
            input: Vec::new(),
            output: std::collections::VecDeque::new(),
            scenario: Scenario::Hover,
            started: Some(std::time::Instant::now()),
            duration: Duration::from_secs(1),
            mode: SwitchMode::Batch,
            state: crate::FrameState {
                running: true,
                elapsed: Duration::from_secs(1),
                ..Default::default()
            },
            ready: false,
            last_progress: std::time::Instant::now(),
            last_dashboard: std::time::Instant::now(),
            applied_updates: 0,
            hover_observer: crate::HoverObserver::default(),
            hover_finish: None,
        }
    }

    #[test]
    fn hover_readiness_requires_sensible_actual_dimensions_and_scale() {
        let mut harness = hover_harness();
        for (backend, scale, width, height) in [
            ("unspecified", 1.0, 740.0, 1080.0),
            ("unspecified", 1.0, 1920.0, 720.0),
            ("unspecified", 1.0, f32::NAN, 1080.0),
            ("unspecified", 1.0, 1920.0, f32::INFINITY),
            ("unspecified", 2.0, 1920.0, 1080.0),
            ("", 1.0, 1920.0, 1080.0),
        ] {
            assert!(
                harness
                    .mark_hover_ready("native", backend, scale, width, height)
                    .is_err()
            );
            assert!(!harness.ready);
        }
        harness
            .mark_hover_ready("native", "wayland", 1.0, 1920.0, 1080.0)
            .unwrap();
        assert!(harness.ready);
        harness
            .mark_hover_ready("native", "wayland", 1.0, 1000.0, 780.0)
            .unwrap();
    }

    #[test]
    fn hover_completion_waits_for_bounded_native_drain_and_never_uses_applied_ticks() {
        let mut harness = hover_harness();
        harness.hover_observer.reset(true);
        harness.report_applied(60).unwrap();
        assert!(harness.poll().unwrap().running);
        assert_eq!(harness.state.update_index, 0);
        let requested = std::time::Instant::now();
        harness.hover_finish = Some((60, requested));
        for motion in 0..59 {
            harness.hover_observer.enter(crate::hover::target(motion));
        }
        harness
            .poll_hover_completion(
                requested + crate::hover::DRAIN_TIMEOUT - Duration::from_nanos(1),
            )
            .unwrap();
        assert!(harness.state.running);
        harness.hover_observer.enter(crate::hover::target(59));
        harness
            .poll_hover_completion(requested + crate::hover::DRAIN_TIMEOUT)
            .unwrap();
        assert!(!harness.state.running);
        assert!(harness.started.is_none());
    }

    #[test]
    fn hover_drain_rejects_missing_native_input_and_stop_shortcut() {
        let mut harness = hover_harness();
        harness.hover_observer.reset(true);
        let requested = std::time::Instant::now();
        harness.hover_finish = Some((60, requested));
        assert!(
            harness
                .poll_hover_completion(requested + crate::hover::DRAIN_TIMEOUT)
                .is_err()
        );
        assert!(!harness.state.running);
        assert!(harness.hover_observer.snapshot().cells.is_empty());
        let mut harness = hover_harness();
        harness.input.extend(b"{\"command\":\"stop\"}\n");
        assert!(harness.poll().is_err());
        let mut harness = hover_harness();
        harness
            .input
            .extend(b"{\"command\":\"finish-hover\",\"submitted_motions\":1}\n");
        assert!(harness.poll().is_err());
    }

    #[test]
    fn hover_reset_clears_observations_and_extra_drain_events_fail() {
        let mut harness = hover_harness();
        let observer = harness.hover_observer();
        observer.reset(true);
        observer.enter(0);
        harness.input.extend(b"{\"command\":\"reset\"}\n");
        assert_eq!(harness.poll().unwrap().reset_generation, 1);
        observer.enter(1);
        assert!(observer.snapshot().cells.is_empty());
        harness
            .input
            .extend(b"{\"command\":\"start\",\"duration_ms\":1000,\"switch_mode\":\"batch\"}\n");
        assert!(harness.poll().unwrap().running);
        observer.enter(0);
        assert_eq!(observer.snapshot().cells, vec![0]);
        for motion in 1..61 {
            observer.enter(crate::hover::target(motion));
        }
        let requested = std::time::Instant::now();
        harness.hover_finish = Some((60, requested));
        assert!(
            harness
                .poll_hover_completion(requested + crate::hover::DRAIN_TIMEOUT)
                .is_err()
        );
    }

    #[test]
    fn hover_reset_stays_idle_until_started() {
        let mut harness = hover_harness();
        harness.input.extend(b"{\"command\":\"reset\"}\n");
        assert!(!harness.poll().unwrap().running);
        harness
            .input
            .extend(b"{\"command\":\"start\",\"duration_ms\":1000,\"switch_mode\":\"batch\"}\n");
        assert!(harness.poll().unwrap().running);
        harness.hover_observer.enter(0);
        assert_eq!(harness.hover_observer.snapshot().cells, vec![0]);
    }

    #[test]
    fn hover_resource_protocol_keeps_distinct_cpu_and_memory_times() {
        use crate::{Event, hover::ResourceSampleTiming};
        let event = Event::HoverResources {
            phase: "measurement".into(),
            root_process_id: 42,
            root_start_ticks: 100,
            new_process_start_ticks: 500,
            cpu_baseline: ResourceSampleTiming {
                started_ms: -2.0,
                completed_ms: -1.0,
                process_count: 1,
            },
            cpu_endpoint: Some(ResourceSampleTiming {
                started_ms: 1000.0,
                completed_ms: 1002.0,
                process_count: 1,
            }),
            membership_scans: vec![ResourceSampleTiming {
                started_ms: 900.0,
                completed_ms: 904.0,
                process_count: 1,
            }],
            memory_samples: vec![ResourceSampleTiming {
                started_ms: 1003.0,
                completed_ms: 1250.0,
                process_count: 1,
            }],
            final_memory_completed_ms: Some(1250.0),
            maximum_memory_lateness_ms: 2000.0,
            cpu_ms: 250.0,
            verified: true,
        };
        let value = serde_json::to_value(event).unwrap();
        assert_eq!(value["cpu_endpoint"]["completed_ms"], 1002.0);
        assert_eq!(value["final_memory_completed_ms"], 1250.0);
        let decoded: Event = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), value);
    }

    #[test]
    fn hover_protocol_messages_round_trip() {
        use crate::{Event, hover::Observations};
        assert_eq!(Scenario::Hover.id(), "hover");
        assert_eq!(Scenario::parse("hover"), Ok(Scenario::Hover));
        assert!(Scenario::ALL.contains(&Scenario::Hover));
        assert_eq!(Scenario::Hover.version(), 3);
        assert_eq!(Scenario::Hover.row_count(), 0);
        let old = serde_json::from_str::<Event>(r#"{"event":"completed","elapsed_ms":1000,"scheduled_updates":30,"applied_updates":30}"#).unwrap();
        assert!(matches!(
            old,
            Event::Completed {
                applied_updates: 30,
                ..
            }
        ));
        for event in [
            Event::HoverGeometry {
                output_global: 1,
                output_width: 1920,
                output_height: 1080,
                output_scale: 1,
                output_transform: "normal".into(),
                logical_width: 1920.0,
                logical_height: 1080.0,
            },
            Event::HoverReset,
            Event::HoverReady {
                backend: "x11".into(),
                scale_factor: 1.0,
                logical_width: crate::WIDTH,
                logical_height: crate::HEIGHT,
            },
            Event::HoverCompleted {
                elapsed_ms: 1000.0,
                observations: Observations {
                    cells: vec![0, 1],
                    invalid: false,
                },
                verified: true,
            },
        ] {
            let value = serde_json::to_value(&event).unwrap();
            let decoded: Event = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(decoded).unwrap(), value);
        }
        let command = Command::FinishHover {
            submitted_motions: 60,
        };
        let encoded = serde_json::to_string(&command).unwrap();
        assert!(matches!(
            serde_json::from_str::<Command>(&encoded).unwrap(),
            Command::FinishHover {
                submitted_motions: 60
            }
        ));
    }

    #[test]
    fn missed_switch_updates_are_not_discarded() {
        let mut switches = Switches::default();
        switches.advance(2, SwitchMode::Batch);
        assert!((0..180).all(|index| switches.get(index)));
        assert!((180..900).all(|index| !switches.get(index)));
        assert_eq!(switches.applied_updates(), 2);
        switches.advance(2, SwitchMode::Batch);
        assert!(switches.get(0));
    }
}
