mod catalog;
mod hover;
mod process;
mod protocol;
mod results;

use benchmark_common::{Command as Control, Event, PROTOCOL_VERSION, Scenario, SwitchMode};
use catalog::Adapter;
use clap::{Parser, Subcommand, ValueEnum};
use process::{ChildGuard, ProcessSampler};
use protocol::Connection;
use results::{Metric, Repetition, ResultEntry};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs, io,
    net::TcpListener,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

type Error = Box<dyn std::error::Error>;

#[derive(Parser)]
#[command(about = "Local Rust GUI benchmark runner")]
struct Arguments {
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    List,
    Doctor,
    Build {
        #[arg(long)]
        framework: String,
        #[arg(long)]
        scenario: String,
        #[arg(long, value_enum, default_value_t = Profile::Release)]
        profile: Profile,
    },
    Run(RunArguments),
}

#[derive(Clone, Copy, ValueEnum)]
enum Mode {
    Single,
    Batch,
    All,
}

#[derive(Clone, Copy, ValueEnum)]
enum Profile {
    Dev,
    Release,
}

impl Profile {
    fn id(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Release => "release",
        }
    }
}

#[derive(clap::Args)]
struct RunArguments {
    #[arg(long, value_delimiter = ',', required = true)]
    framework: Vec<String>,
    #[arg(long, value_delimiter = ',', required = true)]
    scenario: Vec<String>,
    #[arg(long, value_enum, default_value_t = Profile::Release)]
    profile: Profile,
    #[arg(long, default_value = "reference")]
    machine: String,
    #[arg(long, default_value = "Local reference machine")]
    label: String,
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..=100))]
    repetitions: u32,
    #[arg(
        long,
        conflicts_with = "repetitions",
        help = "Run one repetition per framework and scenario without changing the workload"
    )]
    fast: bool,
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u32).range(1..=300))]
    duration: u32,
    #[arg(long, default_value_t = 2, value_parser = clap::value_parser!(u32).range(1..=60))]
    warmup: u32,
    #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u32).range(0..=300))]
    cooldown: u32,
    #[arg(long, value_enum, default_value_t = Mode::Batch)]
    switch_mode: Mode,
    #[arg(long, default_value = "results")]
    output: PathBuf,
    #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(u32).range(5..=300))]
    startup_timeout: u32,
}

impl RunArguments {
    fn repetition_count(&self) -> u32 {
        if self.fast { 1 } else { self.repetitions }
    }
}

fn main() -> Result<(), Error> {
    let arguments = Arguments::parse();
    let root = arguments.root.canonicalize()?;
    let adapters = catalog::load(&root)?;
    match arguments.command {
        Action::List => {
            for adapter in &adapters {
                println!("{}", adapter.id);
                for (scenario, capability) in &adapter.scenarios {
                    println!(
                        "  {scenario}: {}{}",
                        capability.status,
                        capability
                            .reason
                            .as_ref()
                            .map(|reason| format!(" ({reason})"))
                            .unwrap_or_default()
                    );
                }
            }
        }
        Action::Doctor => {
            println!("OS: {}", std::env::consts::OS);
            for name in ["DISPLAY", "WAYLAND_DISPLAY", "XDG_SESSION_TYPE"] {
                println!(
                    "{name}: {}",
                    std::env::var(name).unwrap_or_else(|_| "not set".into())
                );
            }
            for tool in ["cargo", "rustc", "getconf", "mangohud", "xdotool", "lspci"] {
                let available = Command::new("sh")
                    .args(["-c", &format!("command -v {tool}")])
                    .output()
                    .is_ok_and(|output| output.status.success());
                println!(
                    "{tool}: {}",
                    if available {
                        "available"
                    } else {
                        "unavailable"
                    }
                );
            }
            println!("Frame and GPU execution timings are not collected by this runner yet.");
            println!(
                "Visible focused windows, display settings and power profile must be controlled by the operator."
            );
        }
        Action::Build {
            framework,
            scenario,
            profile,
        } => {
            Scenario::parse(&scenario)?;
            let adapter = find_adapter(&adapters, &framework)?;
            require_implemented(adapter, &scenario)?;
            println!("{}", build(&root, adapter, &scenario, profile)?.display());
        }
        Action::Run(arguments) => run(&root, &adapters, arguments)?,
    }
    Ok(())
}

fn find_adapter<'a>(adapters: &'a [Adapter], identifier: &str) -> Result<&'a Adapter, Error> {
    adapters
        .iter()
        .find(|adapter| adapter.id == identifier)
        .ok_or_else(|| format!("Unknown framework: {identifier}").into())
}

fn require_implemented(adapter: &Adapter, scenario: &str) -> Result<(), Error> {
    let capability = adapter
        .scenarios
        .get(scenario)
        .ok_or_else(|| format!("{} has no {scenario} scenario", adapter.id))?;
    if capability.status != "implemented" {
        return Err(format!(
            "{} / {scenario}: {}",
            adapter.id,
            capability.reason.as_deref().unwrap_or(&capability.status)
        )
        .into());
    }
    Ok(())
}

fn build(
    root: &Path,
    adapter: &Adapter,
    scenario: &str,
    profile: Profile,
) -> Result<PathBuf, Error> {
    eprintln!("Building {} / {scenario}", adapter.id);
    let output = Command::new("cargo")
        .current_dir(root)
        .args([
            "build",
            "--profile",
            profile.id(),
            "--locked",
            "--manifest-path",
            &adapter.manifest,
            "--package",
            &adapter.package,
            "--no-default-features",
            "--features",
            scenario,
            "--message-format=json",
        ])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "Build failed for {} / {scenario}:\n{}",
            adapter.id,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let executable = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find_map(|message| {
            if message["reason"] == "compiler-artifact"
                && message["target"]["name"] == adapter.package
            {
                message["executable"].as_str().map(PathBuf::from)
            } else {
                None
            }
        })
        .ok_or("Cargo did not report the application executable")?;
    let directory = root.join("target/staged").join(&adapter.id).join(scenario);
    fs::create_dir_all(&directory)?;
    let destination = directory.join(format!("app{}", std::env::consts::EXE_SUFFIX));
    fs::copy(executable, &destination)?;
    Ok(destination)
}

fn run(root: &Path, adapters: &[Adapter], arguments: RunArguments) -> Result<(), Error> {
    if !cfg!(target_os = "linux") {
        return Err("Local resource sampling currently requires Linux".into());
    }
    if !safe_identifier(&arguments.machine) {
        return Err(
            "Machine ID must contain only ASCII letters, digits, hyphens and underscores".into(),
        );
    }
    if arguments
        .scenario
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != arguments.scenario.len()
        || arguments
            .framework
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != arguments.framework.len()
    {
        return Err("Framework and scenario lists must not contain duplicates".into());
    }
    for scenario in &arguments.scenario {
        Scenario::parse(scenario)?;
    }
    for framework in &arguments.framework {
        find_adapter(adapters, framework)?;
    }
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let identifier = format!("{}-{}", epoch.as_millis(), std::process::id());
    let directory = root
        .join(&arguments.output)
        .join(&arguments.machine)
        .join(&identifier);
    fs::create_dir_all(&directory)?;
    let mut entries = Vec::new();
    let mut executables = BTreeMap::new();
    let mut provenance = BTreeMap::new();
    let mut framework_versions = BTreeMap::new();
    let mut framework_labels = BTreeMap::new();
    let mut framework_base_ids = BTreeMap::new();
    for framework in &arguments.framework {
        let adapter = find_adapter(adapters, framework)?;
        framework_versions.insert(framework.clone(), adapter.version.clone());
        if let Some(name) = &adapter.name {
            framework_labels.insert(framework.clone(), name.clone());
        }
        if let Some(base_id) = &adapter.base_id {
            framework_base_ids.insert(framework.clone(), base_id.clone());
        }
        let manifest = root.join(&adapter.manifest);
        if let Ok(contents) = fs::read_to_string(&manifest) {
            provenance.insert(format!("{framework}/Cargo.toml"), contents);
        }
        if let Some(parent) = manifest.parent() {
            if let Ok(contents) = fs::read_to_string(parent.join("Cargo.lock")) {
                provenance.insert(format!("{framework}/Cargo.lock"), contents);
            }
        }
        for scenario in &arguments.scenario {
            let mut entry = ResultEntry::pending(framework, Scenario::parse(scenario)?);
            match adapter.scenarios.get(scenario) {
                Some(capability) if capability.status == "implemented" => {
                    entry.reason = capability.reason.clone();
                    match build(root, adapter, scenario, arguments.profile) {
                        Ok(executable) => {
                            executables.insert((framework.clone(), scenario.clone()), executable);
                        }
                        Err(error) => {
                            entry.status = "failed".into();
                            entry.reason = Some(error.to_string());
                        }
                    }
                }
                Some(capability) => {
                    entry.status = capability.status.clone();
                    entry.reason = capability.reason.clone();
                }
                None => {
                    entry.status = "unsupported".into();
                    entry.reason = Some("No adapter for this scenario".into());
                }
            }
            entries.push(entry);
        }
    }
    if arguments.cooldown > 0 {
        eprintln!(
            "Cooling down for {} seconds after compilation",
            arguments.cooldown
        );
        thread::sleep(Duration::from_secs(u64::from(arguments.cooldown)));
    }
    let mut seed = epoch.as_nanos() as u64;
    for repetition in 0..arguments.repetition_count() as usize {
        let mut order: Vec<_> = (0..entries.len()).collect();
        for index in (1..order.len()).rev() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            order.swap(index, (seed % (index as u64 + 1)) as usize);
        }
        for index in order {
            let entry = &mut entries[index];
            if entry.status != "ok" {
                continue;
            }
            let executable = executables
                .get(&(entry.framework.clone(), entry.scenario.clone()))
                .ok_or("Missing staged executable")?;
            eprintln!(
                "Running {} / {}, repetition {}",
                entry.framework,
                entry.scenario,
                repetition + 1
            );
            let prefix = directory.join(format!(
                "{}-{}-{repetition}",
                entry.framework, entry.scenario
            ));
            match measure(
                executable,
                Scenario::parse(&entry.scenario)?,
                repetition,
                &arguments,
                &prefix,
            ) {
                Ok(result) => entry.repetitions.push(result),
                Err(error) => {
                    entry.status = match error.downcast_ref::<io::Error>().map(io::Error::kind) {
                        Some(io::ErrorKind::TimedOut) => "timed-out",
                        Some(io::ErrorKind::InvalidData) => "invalid-output",
                        Some(io::ErrorKind::Unsupported) => "unavailable",
                        _ => "failed",
                    }
                    .into();
                    entry.reason = Some(error.to_string());
                }
            }
        }
    }
    for entry in &mut entries {
        entry.summarize();
    }
    let failed = entries
        .iter()
        .any(|entry| ["failed", "timed-out", "invalid-output"].contains(&entry.status.as_str()));
    let hover_input_metadata = json!({
        "workloadVersion": Scenario::Hover.version(),
        "backend": "linux-wayland-uinput-absolute-fullscreen-v3-60hz-scale1",
        "trajectory": "serpentine-v1 client cell centers from (0,0), 30x30 at 24 logical pixels",
        "motionsPerSecond": 60, "cycleSeconds": 15,
        "durationNote": "Default 10s measures a prefix, full sweep requires duration >=15s",
        "scale": 1, "maximumLatenessMs": 5, "finalDrainMs": 250, "finalDrainProtocolGraceMs": 2000,
        "phaseSetup": "two Wayland registry/output round trips and one persistent uinput device, park at local (740,740) before each phase, then HoverReset acknowledgment and 250ms settle before CPU baseline and Start",
        "verification": "exact ordered native callback cells must match submitted cells in each phase",
        "geometry": "single advertised connected wl_output current landscape mode, scale1 and normal transform, actual adapter logical viewport must equal the full output before input creation, local origin (0,0) mapped directly to normalized 0..65535 using output dimensions",
        "fullscreen": "adapter requests fullscreen, runner verifies actual viewport dimensions against output, no independent native fullscreen-state inspection",
        "geometryProvenance": "HoverReady retains actual backend/scale/viewport, HoverGeometry retains wl_output global/current dimensions/scale/transform and matching actual viewport",
        "interferencePolicy": "operator must keep the entire grid visible, window stationary and desktop untouched, exact timed native target observations required",
        "interferenceLimit": "Wayland exposes no global focus, occlusion or physical-device attribution checks here. Output geometry and exact native sequences cannot detect all transient interference",
        "backendSelection": "child native environment preserved, actual public toolkit backend reported or unspecified, input driver is separate",
        "timing": "absolute monotonic deadlines from received Started, no catch-up, submission timestamps record kernel writes, not compositor delivery acknowledgments",
        "measurementBoundary": "CPU baseline counters after initial memory acquisition and before measured Start, endpoint tracked-member CPU counters complete within 5ms of fixed input endpoint, memory acquisition and observation drain never extend CPU attribution",
        "resourceScheduling": "scoped input scheduler independent of main-thread process discovery and memory reads, nominal 100ms discovery with missed resource slots skipped rather than fabricated",
        "cpuMembership": "baseline and periodic tracked process identities, root identity validated, children born after last membership discovery and short-lived children may be missed, pre-existing newly discovered members get no pre-baseline CPU credit",
        "memoryBoundary": "initial pre-CPU-baseline sample, periodic samples and final sample after input endpoint, actual acquisition start/completion timestamps, final completion at most 2000ms late and individual acquisition at most 2000ms",
        "timingProvenance": "HoverResources retains CPU baseline/endpoint read spans, root identity and new-process cutoff, used membership scan spans/counts and memory spans, HoverInput endpoint_sample_elapsed_ms is CPU-counter completion, endpoint_elapsed_ms is input stop, finish_request_elapsed_ms is control request completion"
    });
    let run = json!({
        "schemaVersion": 1, "id": identifier, "collectedAt": utc_now()?,
        "machine": machine_metadata(&arguments.machine, &arguments.label),
        "results": entries,
        "metadata": {
            "durationSeconds": arguments.duration, "warmupSeconds": arguments.warmup,
            "repetitions": arguments.repetition_count(), "fastMode": arguments.fast, "aggregation": "median of repetition summaries",
            "buildProfile": arguments.profile.id(), "comparisonValidated": false,
            "cooldownSeconds": arguments.cooldown,
            "gpuiFastViewRetention": std::env::var("GPUI_VIEW_RETENTION").ok(),
            "comparability": "Prototype workloads need visual review and normalization of fonts, controls and animations before comparative ranking",
            "switchMode": match arguments.switch_mode { Mode::Single => "single", Mode::Batch => "batch", Mode::All => "all" },
            "inputMode": if arguments.scenario.iter().any(|scenario| scenario == "hover") { "scenario-specific, hover uses Linux uinput absolute mouse" } else { "programmatic" },
            "hoverInput": hover_input_metadata,
            "samplingIntervalMs": 100,
            "outputValidation": "unverified, operator screenshot review required",
            "frameTiming": "unavailable", "gpuTiming": "unavailable",
            "startup": "fresh process, warm filesystem cache, post-renderer adapter boundary, startup-scene memory sampled after settling",
            "startupMemorySettleSeconds": arguments.warmup,
            "startupMemorySampling": "single endpoint after idle settling, not initialization peak",
            "freyaRendererOverride": std::env::var("FREYA_RENDERER").ok(),
            "startupWorkloadVersion": Scenario::Startup.version(),
            "cpuCaveat": "Sampled process-tree CPU, short-lived subprocesses can be missed",
            "rssCaveat": "Summed process-tree RSS can double-count shared pages",
            "memoryCaveat": "Peak is sampled, GPU allocations excluded",
            "rustc": tool_output("rustc", &["--version"]),
            "gitRevision": tool_output_at(root, "git", &["rev-parse", "HEAD"]),
            "gitStatus": tool_output_at(root, "git", &["status", "--short"]),
            "dependencyManifests": provenance,
            "frameworkVersions": framework_versions,
            "frameworkLabels": framework_labels,
            "frameworkBaseIds": framework_base_ids,
            "controlWorkloadVersion": Scenario::Switches.version(),
            "memoryWorkloadVersion": Scenario::Memory.version(),
            "controlDesign": "30x30 equal rectangles, ON/OFF, shared colors, 12px DejaVu Sans, no animations"
        }
    });
    let path = directory.join("run.json");
    fs::write(&path, serde_json::to_vec_pretty(&run)?)?;
    println!("Saved {}", path.display());
    if failed {
        return Err("One or more benchmarks failed, partial results were saved".into());
    }
    Ok(())
}

fn measure(
    executable: &Path,
    scenario: Scenario,
    index: usize,
    arguments: &RunArguments,
    prefix: &Path,
) -> Result<Repetition, Error> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let mut command = Command::new(executable);
    command
        .env("GUI_BENCH_ADDR", listener.local_addr()?.to_string())
        .stdout(fs::File::create(prefix.with_extension("stdout.log"))?)
        .stderr(fs::File::create(prefix.with_extension("stderr.log"))?);
    let (mut hover_driver, hover_geometry) = if scenario == Scenario::Hover {
        let (driver, geometry) = hover::Driver::connect()
            .map_err(|error| io::Error::new(io::ErrorKind::Unsupported, error.to_string()))?;
        let (width, height) = driver.output_dimensions()?;
        command
            .env("GUI_BENCH_HOVER_WIDTH", width.to_string())
            .env("GUI_BENCH_HOVER_HEIGHT", height.to_string());
        (Some(driver), Some(geometry))
    } else {
        (None, None)
    };
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let launched = Instant::now();
    let mut child = ChildGuard(command.spawn()?);
    let timeout = Duration::from_secs(u64::from(arguments.startup_timeout));
    let mut connection = loop {
        match listener.accept() {
            Ok((stream, _)) => break Connection::new(stream)?,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.into()),
        }
        check_child(&mut child, launched, timeout)?;
        thread::sleep(Duration::from_millis(5));
    };
    let mut boundary = None;
    let mut hello = false;
    let mut hover_viewport = None;
    loop {
        for event in connection.receive()? {
            match event {
                Event::Hello {
                    protocol,
                    scenario: received,
                    process_id,
                } => {
                    if protocol != PROTOCOL_VERSION
                        || received != scenario
                        || process_id != child.0.id()
                    {
                        return Err("Invalid adapter handshake".into());
                    }
                    hello = true;
                }
                Event::Ready { boundary: value } => boundary = Some(value),
                Event::HoverReady {
                    backend,
                    scale_factor,
                    logical_width,
                    logical_height,
                } => {
                    if scenario != Scenario::Hover {
                        return Err("Unexpected HoverReady for another scenario".into());
                    }
                    hover::validate_ready(&backend, scale_factor, logical_width, logical_height)?;
                    hover_viewport = Some((logical_width, logical_height));
                }
                Event::Failure { message } => return Err(message.into()),
                _ => {}
            }
        }
        if hello && boundary.is_some() {
            break;
        }
        check_child(&mut child, launched, timeout)?;
        thread::sleep(Duration::from_millis(5));
    }
    if scenario == Scenario::Hover && hover_viewport.is_none() {
        return Err(
            "Hover adapter did not declare native observation capability and actual backend/scale"
                .into(),
        );
    }
    if let Some(driver) = &hover_driver {
        let (width, height) = hover_viewport.ok_or("Missing Hover viewport")?;
        driver
            .validate_viewport(width, height)
            .map_err(|error| io::Error::new(io::ErrorKind::Unsupported, error.to_string()))?;
    }
    if let Some(geometry) = hover_geometry {
        connection.events.push(geometry);
    }
    let startup_ms = launched.elapsed().as_secs_f64() * 1000.0;
    let mut metrics = BTreeMap::from([("startup_ms".into(), Metric::new(startup_ms, "ms"))]);
    let mut sampler = ProcessSampler::new(child.0.id())?;
    if scenario != Scenario::Startup {
        let mode = match arguments.switch_mode {
            Mode::Single => SwitchMode::Single,
            Mode::Batch => SwitchMode::Batch,
            Mode::All => SwitchMode::All,
        };
        if let Some(driver) = &mut hover_driver {
            let result = hover::reset_phase(&mut connection, &mut child, driver);
            fs::write(
                prefix.with_extension("events.json"),
                serde_json::to_vec_pretty(&connection.events)?,
            )?;
            result?;
        } else {
            connection.send(&Control::Prepare)?;
        }
        connection.send(&Control::Start {
            duration_ms: u64::from(arguments.warmup) * 1000,
            switch_mode: mode,
        })?;
        if let Some(driver) = &mut hover_driver {
            let result = hover::wait_phase(
                &mut connection,
                &mut child,
                driver,
                Duration::from_secs(u64::from(arguments.warmup)),
                "warmup",
                None,
            );
            fs::write(
                prefix.with_extension("events.json"),
                serde_json::to_vec_pretty(&connection.events)?,
            )?;
            result?;
        } else {
            wait_phase(
                &mut connection,
                &mut child,
                Duration::from_secs(u64::from(arguments.warmup)),
                None,
            )?;
        }
        if let Some(driver) = &mut hover_driver {
            let result = hover::reset_phase(&mut connection, &mut child, driver);
            fs::write(
                prefix.with_extension("events.json"),
                serde_json::to_vec_pretty(&connection.events)?,
            )?;
            result?;
        } else {
            connection.send(&Control::Reset)?;
        }
        if let Some(driver) = &mut hover_driver {
            let resources = process::hover::Resources::prepare(&mut sampler)?;
            connection.send(&Control::Start {
                duration_ms: u64::from(arguments.duration) * 1000,
                switch_mode: mode,
            })?;
            let result = hover::wait_phase(
                &mut connection,
                &mut child,
                driver,
                Duration::from_secs(u64::from(arguments.duration)),
                "measurement",
                Some(resources),
            );
            fs::write(
                prefix.with_extension("events.json"),
                serde_json::to_vec_pretty(&connection.events)?,
            )?;
            result?;
        } else {
            sampler.sample(0.0)?;
            connection.send(&Control::Start {
                duration_ms: u64::from(arguments.duration) * 1000,
                switch_mode: mode,
            })?;
            wait_phase(
                &mut connection,
                &mut child,
                Duration::from_secs(u64::from(arguments.duration)),
                Some(&mut sampler),
            )?;
        }
        if scenario == Scenario::Memory {
            let verified = connection.events.iter().any(|event| {
                matches!(event,
                Event::Dashboard { snapshot }
                    if snapshot.records == benchmark_common::dashboard::RECORD_COUNT
                    && snapshot.fields == benchmark_common::dashboard::FIELD_COUNT
                    && snapshot.thumbnails == benchmark_common::dashboard::THUMBNAIL_COUNT
                    && snapshot.detail_characters == benchmark_common::dashboard::DETAIL_CHARACTERS
                    && snapshot.edited_fields > 0 && snapshot.active_records > 0
                    && snapshot.applied_updates > 0)
            });
            if !verified {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Memory dashboard did not report the complete shared fixture and live state changes").into());
            }
        }
        metrics.insert("cpu_ms".into(), Metric::new(sampler.cpu_ms(), "ms"));
    } else {
        let settling = Instant::now();
        let settle_duration = Duration::from_secs(u64::from(arguments.warmup));
        while settling.elapsed() < settle_duration {
            for event in connection.receive()? {
                if let Event::Failure { message } = event {
                    return Err(message.into());
                }
            }
            check_child(
                &mut child,
                settling,
                settle_duration + Duration::from_secs(10),
            )?;
            thread::sleep(Duration::from_millis(5));
        }
        sampler.sample(settling.elapsed().as_secs_f64() * 1000.0)?;
    }
    let rss: Vec<_> = sampler
        .samples
        .iter()
        .filter_map(|sample| sample.rss_mib)
        .collect();
    let pss: Vec<_> = sampler
        .samples
        .iter()
        .filter_map(|sample| sample.pss_mib)
        .collect();
    let endpoint_rss = if scenario == Scenario::Hover {
        sampler.samples.last().and_then(|sample| sample.rss_mib)
    } else {
        rss.last().copied()
    };
    if let Some(last) = endpoint_rss {
        metrics.insert("rss_mib".into(), Metric::new(last, "MiB"));
    }
    if let Some(peak) = rss.into_iter().reduce(f64::max) {
        metrics.insert("peak_rss_mib".into(), Metric::new(peak, "MiB"));
    }
    let endpoint_pss = if scenario == Scenario::Hover {
        sampler.samples.last().and_then(|sample| sample.pss_mib)
    } else {
        pss.last().copied()
    };
    if let Some(last) = endpoint_pss {
        metrics.insert("pss_mib".into(), Metric::new(last, "MiB"));
    }
    connection.send(&Control::Quit)?;
    let closing = Instant::now();
    while child.0.try_wait()?.is_none() && closing.elapsed() < Duration::from_secs(5) {
        thread::sleep(Duration::from_millis(10));
    }
    match child.0.try_wait()? {
        Some(status) if status.success() => {}
        Some(status) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Adapter failed during shutdown: {status}"),
            )
            .into());
        }
        None => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Adapter did not exit after the quit command",
            )
            .into());
        }
    }
    Ok(Repetition {
        index,
        startup_boundary: boundary.ok_or("Missing startup boundary")?,
        metrics,
        samples: sampler.samples,
        events: connection.events,
    })
}

fn wait_phase(
    connection: &mut Connection,
    child: &mut ChildGuard,
    expected_duration: Duration,
    mut sampler: Option<&mut ProcessSampler>,
) -> Result<(), Error> {
    let waiting = Instant::now();
    let mut started = None;
    let mut last_sample = Instant::now();
    loop {
        for event in connection.receive()? {
            match event {
                Event::Started => started = Some(Instant::now()),
                Event::Completed {
                    elapsed_ms,
                    applied_updates,
                    scheduled_updates,
                } => {
                    if started.is_none() {
                        return Err("Adapter completed without a Started event".into());
                    }
                    if elapsed_ms < expected_duration.as_secs_f64() * 1000.0 - 1.0
                        || waiting.elapsed() < expected_duration.mul_f64(0.95)
                        || applied_updates != scheduled_updates
                        || scheduled_updates != (expected_duration.as_millis() * 30 / 1000) as u64
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Adapter did not complete every scheduled workload update",
                        )
                        .into());
                    }
                    if let Some(sampler) = sampler.as_deref_mut() {
                        sampler.sample(
                            started
                                .map(|instant| instant.elapsed().as_secs_f64() * 1000.0)
                                .unwrap_or(0.0),
                        )?;
                    }
                    return Ok(());
                }
                Event::Failure { message } => return Err(message.into()),
                _ => {}
            }
        }
        if let Some(started) = started {
            if last_sample.elapsed() >= Duration::from_millis(100) {
                if let Some(sampler) = sampler.as_deref_mut() {
                    sampler.sample(started.elapsed().as_secs_f64() * 1000.0)?;
                }
                last_sample = Instant::now();
            }
        }
        check_child(child, waiting, expected_duration + Duration::from_secs(10))?;
        thread::sleep(Duration::from_millis(5));
    }
}

fn check_child(child: &mut ChildGuard, started: Instant, timeout: Duration) -> Result<(), Error> {
    if let Some(status) = child.0.try_wait()? {
        return Err(format!("Adapter exited early: {status}, inspect stderr.log").into());
    }
    if started.elapsed() >= timeout {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "Adapter exceeded its phase timeout",
        )
        .into());
    }
    Ok(())
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn tool_output(tool: &str, arguments: &[&str]) -> Option<String> {
    Command::new(tool)
        .args(arguments)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn tool_output_at(directory: &Path, tool: &str, arguments: &[&str]) -> Option<String> {
    Command::new(tool)
        .current_dir(directory)
        .args(arguments)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn utc_now() -> Result<String, Error> {
    tool_output("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .ok_or_else(|| "Could not obtain UTC time".into())
}

fn machine_metadata(identifier: &str, label: &str) -> Value {
    let cpu = fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|contents| {
            contents.lines().find_map(|line| {
                let (key, value) = line.split_once(':')?;
                (key.trim() == "model name").then(|| value.trim().to_owned())
            })
        });
    let graphics = tool_output("lspci", &[]).map(|contents| {
        contents
            .lines()
            .filter(|line| {
                line.contains("VGA")
                    || line.contains("3D controller")
                    || line.contains("Display controller")
            })
            .collect::<Vec<_>>()
            .join("\n")
    });
    json!({ "id": identifier, "label": label, "os": tool_output("uname", &["-sr"]), "cpu": cpu, "gpu": graphics,
        "memory": fs::read_to_string("/proc/meminfo").ok().and_then(|contents| contents.lines().next().map(str::to_owned)),
        "session": std::env::var("XDG_SESSION_TYPE").ok(),
        "displaySettings": "not automatically captured, operator must record separately" })
}

#[cfg(test)]
mod tests {
    use crate::{Action, Arguments, RunArguments};
    use clap::Parser;

    fn run_arguments(options: &[&str]) -> RunArguments {
        let arguments = Arguments::try_parse_from(
            [
                "gui-bench-runner",
                "run",
                "--framework",
                "egui,freya",
                "--scenario",
                "hover,scroll",
            ]
            .into_iter()
            .chain(options.iter().copied()),
        )
        .unwrap();
        let Action::Run(arguments) = arguments.command else {
            panic!("Expected run arguments");
        };
        arguments
    }

    #[test]
    fn fast_mode_runs_once_without_changing_workload() {
        let regular = run_arguments(&["--duration", "15", "--warmup", "2"]);
        let fast = run_arguments(&["--fast", "--duration", "15", "--warmup", "2"]);
        assert_eq!(regular.repetition_count(), 5);
        assert_eq!(fast.repetition_count(), 1);
        assert_eq!(fast.duration, regular.duration);
        assert_eq!(fast.warmup, regular.warmup);
        assert_eq!(fast.framework, regular.framework);
        assert_eq!(fast.scenario, regular.scenario);
    }

    #[test]
    fn fast_mode_preserves_explicit_repetitions_when_disabled() {
        let arguments = run_arguments(&["--repetitions", "3"]);
        assert!(!arguments.fast);
        assert_eq!(arguments.repetition_count(), 3);
    }

    #[test]
    fn fast_mode_conflicts_with_explicit_repetitions() {
        for options in [
            ["--fast", "--repetitions", "3"],
            ["--repetitions", "1", "--fast"],
        ] {
            let result = Arguments::try_parse_from(
                [
                    "gui-bench-runner",
                    "run",
                    "--framework",
                    "egui",
                    "--scenario",
                    "hover",
                ]
                .into_iter()
                .chain(options),
            );
            assert_eq!(
                result.err().unwrap().kind(),
                clap::error::ErrorKind::ArgumentConflict
            );
        }
    }
}
