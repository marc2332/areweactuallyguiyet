pub mod dashboard;

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
    Memory,
}

impl Scenario {
    pub const ALL: [Self; 5] = [
        Self::Startup,
        Self::Scroll,
        Self::VirtualScroll,
        Self::Switches,
        Self::Memory,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Scroll => "scroll",
            Self::VirtualScroll => "virtual-scroll",
            Self::Switches => "switches",
            Self::Memory => "memory",
        }
    }

    pub fn version(self) -> u32 {
        match self {
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
            Self::Switches => 0,
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

    pub fn switch_mode(&self) -> SwitchMode {
        self.mode
    }

    pub fn poll(&mut self) -> io::Result<FrameState> {
        if self.started.is_some() && self.state.elapsed >= self.duration {
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
                    self.state = FrameState {
                        reset_generation: self.state.reset_generation + 1,
                        ..FrameState::default()
                    };
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
                    self.send(Event::Started)?;
                }
                Command::Stop => self.complete()?,
                Command::Quit => self.state.should_quit = true,
            }
        }
        if let Some(started) = self.started {
            self.state.elapsed = started.elapsed().min(self.duration);
            self.state.update_index = (self.state.elapsed.as_secs_f64() * 30.0).floor() as u64;
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
            feature = "memory"
        )))]
        compile_error!(
            "Enable exactly one scenario: startup, scroll, virtual-scroll, switches, memory"
        );
        #[cfg(any(
            all(
                feature = "startup",
                any(
                    feature = "scroll",
                    feature = "virtual-scroll",
                    feature = "switches",
                    feature = "memory"
                )
            ),
            all(
                feature = "scroll",
                any(feature = "virtual-scroll", feature = "switches", feature = "memory")
            ),
            all(
                feature = "virtual-scroll",
                any(feature = "switches", feature = "memory")
            ),
            all(feature = "switches", feature = "memory")
        ))]
        compile_error!("Enable only one benchmark scenario at a time");
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
    fn dummy_controls_have_a_distinct_workload_version() {
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
