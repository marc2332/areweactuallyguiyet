use crate::process::{ProcessSampler, ProcessStat, ProcessTree, Sample, memory_kib, parse_stat};
use benchmark_common::{Event, hover::ResourceSampleTiming};
use std::{
    collections::BTreeMap,
    fs, io,
    sync::mpsc::Sender,
    time::{Duration, Instant},
};

pub const MAX_MEMORY_LATENESS: Duration = Duration::from_secs(2);

#[derive(Clone)]
struct Collection {
    started: Instant,
    completed: Instant,
    process_count: usize,
}

impl Collection {
    fn validate_memory(&self, origin: Instant, endpoint: Option<Duration>) -> io::Result<()> {
        if self.completed.duration_since(self.started) > MAX_MEMORY_LATENESS {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Hover memory collection exceeded its 2s latency bound",
            ));
        }
        if endpoint.is_some_and(|endpoint| {
            self.completed.duration_since(origin) > endpoint + MAX_MEMORY_LATENESS
        }) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Hover final memory completed more than 2s after input endpoint",
            ));
        }
        Ok(())
    }

    fn timing(&self, origin: Instant) -> ResourceSampleTiming {
        ResourceSampleTiming {
            started_ms: elapsed_ms(self.started, origin),
            completed_ms: elapsed_ms(self.completed, origin),
            process_count: self.process_count,
        }
    }
}

fn elapsed_ms(instant: Instant, origin: Instant) -> f64 {
    if instant >= origin {
        instant.duration_since(origin).as_secs_f64() * 1000.0
    } else {
        -origin.duration_since(instant).as_secs_f64() * 1000.0
    }
}

fn read_stat(identifier: u32) -> io::Result<ProcessStat> {
    let contents = fs::read_to_string(format!("/proc/{identifier}/stat"))?;
    parse_stat(&contents).ok_or_else(|| io::Error::other("Invalid process CPU counters"))
}

pub(crate) struct CpuTracker {
    root: u32,
    root_start: u64,
    new_process_start: u64,
    previous: BTreeMap<(u32, u64), u64>,
    members: ProcessTree,
    baseline: Collection,
    endpoint: Option<Collection>,
    membership_scans: Vec<Collection>,
    ticks: u64,
    ticks_per_second: f64,
}

impl CpuTracker {
    fn start(root: u32, tree: ProcessTree, ticks_per_second: f64) -> io::Result<Self> {
        let started = Instant::now();
        let uptime = fs::read_to_string("/proc/uptime")?;
        let uptime: f64 = uptime
            .split_whitespace()
            .next()
            .ok_or_else(|| io::Error::other("Missing uptime"))?
            .parse()
            .map_err(io::Error::other)?;
        if !uptime.is_finite() || uptime < 0.0 {
            return Err(io::Error::other("Invalid uptime"));
        }
        // Uptime is truncated to centiseconds, so use an upper bound for new-process credit.
        let new_process_start = ((uptime + 0.01) * ticks_per_second).ceil() as u64;
        let root_start = tree
            .processes
            .get(&root)
            .ok_or_else(|| io::Error::other("Hover root process disappeared"))?
            .start;
        let counters = Self::read_members(root, root_start, &tree)?;
        let baseline = Collection {
            started,
            completed: Instant::now(),
            process_count: counters.len(),
        };
        let membership_scans = vec![Self::scan(&tree)];
        Ok(Self {
            root,
            root_start,
            new_process_start,
            previous: counters
                .iter()
                .map(|(&identifier, stat)| ((identifier, stat.start), stat.ticks))
                .collect(),
            members: tree,
            baseline,
            endpoint: None,
            membership_scans,
            ticks: 0,
            ticks_per_second,
        })
    }

    fn scan(tree: &ProcessTree) -> Collection {
        Collection {
            started: tree.started,
            completed: tree.completed,
            process_count: tree.processes.len(),
        }
    }

    fn read_members(
        root: u32,
        root_start: u64,
        tree: &ProcessTree,
    ) -> io::Result<BTreeMap<u32, ProcessStat>> {
        Self::read_members_with(root, root_start, tree, read_stat)
    }

    fn read_members_with(
        root: u32,
        root_start: u64,
        tree: &ProcessTree,
        mut read: impl FnMut(u32) -> io::Result<ProcessStat>,
    ) -> io::Result<BTreeMap<u32, ProcessStat>> {
        let mut counters = BTreeMap::new();
        for (&identifier, expected) in &tree.processes {
            match read(identifier) {
                Ok(stat) if stat.start == expected.start => {
                    counters.insert(identifier, stat);
                }
                _ if identifier == root => {
                    return Err(io::Error::other(
                        "Hover root process exited or changed identity",
                    ));
                }
                _ => {}
            }
        }
        if counters
            .get(&root)
            .is_none_or(|stat| stat.start != root_start)
        {
            return Err(io::Error::other("Hover root process identity is missing"));
        }
        Ok(counters)
    }

    fn account(&mut self, counters: &BTreeMap<u32, ProcessStat>) -> io::Result<()> {
        if counters
            .get(&self.root)
            .is_none_or(|stat| stat.start != self.root_start)
        {
            return Err(io::Error::other(
                "Hover root process exited or changed identity",
            ));
        }
        for (&identifier, stat) in counters {
            if let Some(previous) = self.previous.insert((identifier, stat.start), stat.ticks) {
                self.ticks += stat.ticks.saturating_sub(previous);
            } else if stat.start >= self.new_process_start {
                self.ticks += stat.ticks;
            }
        }
        Ok(())
    }

    pub(crate) fn update(&mut self, tree: ProcessTree) -> io::Result<()> {
        self.account(&tree.processes)?;
        self.membership_scans.push(Self::scan(&tree));
        self.members = tree;
        Ok(())
    }

    pub(crate) fn finish(&mut self) -> io::Result<()> {
        let started = Instant::now();
        let counters = Self::read_members(self.root, self.root_start, &self.members);
        self.endpoint = Some(Collection {
            started,
            completed: Instant::now(),
            process_count: counters.as_ref().map_or(0, BTreeMap::len),
        });
        self.account(&counters?)
    }

    pub(crate) fn endpoint_elapsed(&self, origin: Instant) -> Option<Duration> {
        self.endpoint
            .as_ref()
            .map(|collection| collection.completed.saturating_duration_since(origin))
    }
}

pub struct Resources<'a> {
    sampler: &'a mut ProcessSampler,
    pub(crate) cpu: Option<CpuTracker>,
    tree: ProcessTree,
    memory: Vec<Collection>,
    final_memory_completed: Option<Instant>,
}

impl<'a> Resources<'a> {
    pub fn prepare(sampler: &'a mut ProcessSampler) -> io::Result<Self> {
        let tree = ProcessTree::discover(sampler.root)?;
        let mut resources = Self {
            sampler,
            cpu: None,
            tree,
            memory: Vec::new(),
            final_memory_completed: None,
        };
        resources.memory(Instant::now())?;
        resources.cpu = Some(CpuTracker::start(
            resources.sampler.root,
            resources.tree.clone(),
            resources.sampler.ticks_per_second,
        )?);
        Ok(resources)
    }

    pub(crate) fn set_origin(&mut self, origin: Instant) {
        for (sample, collection) in self.sampler.samples.iter_mut().zip(&self.memory) {
            Self::stamp(sample, collection, origin);
        }
    }

    fn stamp(sample: &mut Sample, collection: &Collection, origin: Instant) {
        sample.elapsed_ms = elapsed_ms(collection.completed, origin);
        sample.collection_started_ms = Some(elapsed_ms(collection.started, origin));
        sample.collection_completed_ms = Some(sample.elapsed_ms);
    }

    pub(crate) fn periodic(
        &mut self,
        origin: Instant,
        duration: Duration,
        updates: &Sender<ProcessTree>,
    ) -> io::Result<()> {
        self.tree = ProcessTree::discover(self.sampler.root)?;
        // The scheduler may already have stopped while discovery was in progress.
        let _ = updates.send(self.tree.clone());
        if origin.elapsed() < duration {
            self.memory(origin)?;
        }
        Ok(())
    }

    fn memory(&mut self, origin: Instant) -> io::Result<()> {
        let started = Instant::now();
        let mut rss = 0;
        let mut pss = 0;
        let mut available_rss = true;
        let mut available_pss = true;
        for (&identifier, expected) in &self.tree.processes {
            if !read_stat(identifier).is_ok_and(|stat| stat.start == expected.start) {
                if identifier == self.sampler.root {
                    return Err(io::Error::other(
                        "Hover root process exited or changed identity during memory sampling",
                    ));
                }
                available_rss = false;
                available_pss = false;
                continue;
            }
            let status = fs::read_to_string(format!("/proc/{identifier}/status")).ok();
            if let Some(value) = status.as_deref().and_then(|text| memory_kib(text, "VmRSS")) {
                rss += value;
            } else {
                available_rss = false;
            }
            let rollup = fs::read_to_string(format!("/proc/{identifier}/smaps_rollup")).ok();
            if let Some(value) = rollup.as_deref().and_then(|text| memory_kib(text, "Pss")) {
                pss += value;
            } else {
                available_pss = false;
            }
        }
        let collection = Collection {
            started,
            completed: Instant::now(),
            process_count: self.tree.processes.len(),
        };
        let mut sample = Sample {
            elapsed_ms: 0.0,
            collection_started_ms: None,
            collection_completed_ms: None,
            rss_mib: available_rss.then_some(rss as f64 / 1024.0),
            pss_mib: available_pss.then_some(pss as f64 / 1024.0),
            process_count: collection.process_count,
        };
        Self::stamp(&mut sample, &collection, origin);
        let validation = collection.validate_memory(origin, None);
        self.sampler.samples.push(sample);
        self.memory.push(collection);
        validation
    }

    pub(crate) fn finish_memory(&mut self, origin: Instant, duration: Duration) -> io::Result<()> {
        let previous = self.memory.len();
        let result = self.memory(origin);
        self.final_memory_completed = self
            .memory
            .get(previous)
            .map(|collection| collection.completed);
        result?;
        self.memory
            .get(previous)
            .ok_or_else(|| io::Error::other("Missing Hover final memory sample"))?
            .validate_memory(origin, Some(duration))
    }

    pub(crate) fn event(
        &mut self,
        phase: &str,
        origin: Instant,
        cpu: &CpuTracker,
        verified: bool,
    ) -> Event {
        self.sampler.cpu_ticks = cpu.ticks;
        Event::HoverResources {
            phase: phase.into(),
            root_process_id: cpu.root,
            root_start_ticks: cpu.root_start,
            new_process_start_ticks: cpu.new_process_start,
            cpu_baseline: cpu.baseline.timing(origin),
            cpu_endpoint: cpu
                .endpoint
                .as_ref()
                .map(|collection| collection.timing(origin)),
            membership_scans: cpu
                .membership_scans
                .iter()
                .map(|collection| collection.timing(origin))
                .collect(),
            memory_samples: self
                .memory
                .iter()
                .map(|collection| collection.timing(origin))
                .collect(),
            final_memory_completed_ms: self
                .final_memory_completed
                .map(|instant| elapsed_ms(instant, origin)),
            maximum_memory_lateness_ms: MAX_MEMORY_LATENESS.as_secs_f64() * 1000.0,
            cpu_ms: cpu.ticks as f64 / cpu.ticks_per_second * 1000.0,
            verified,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::process::{
        ProcessSampler, ProcessStat, ProcessTree, Sample,
        hover::{Collection, CpuTracker, Resources},
    };
    use benchmark_common::Event;
    use std::{
        collections::BTreeMap,
        io,
        time::{Duration, Instant},
    };

    fn stat(start: u64, ticks: u64) -> ProcessStat {
        ProcessStat {
            parent: 1,
            ticks,
            start,
        }
    }

    fn tree(processes: BTreeMap<u32, ProcessStat>, instant: Instant) -> ProcessTree {
        ProcessTree {
            processes,
            started: instant,
            completed: instant,
        }
    }

    fn tracker(origin: Instant) -> CpuTracker {
        let members = tree(
            BTreeMap::from([(1, stat(10, 100)), (2, stat(20, 50))]),
            origin - Duration::from_millis(10),
        );
        CpuTracker {
            root: 1,
            root_start: 10,
            new_process_start: 100,
            previous: members
                .processes
                .iter()
                .map(|(&identifier, stat)| ((identifier, stat.start), stat.ticks))
                .collect(),
            baseline: Collection {
                started: origin - Duration::from_millis(5),
                completed: origin - Duration::from_millis(4),
                process_count: 2,
            },
            endpoint: None,
            membership_scans: vec![CpuTracker::scan(&members)],
            members,
            ticks: 0,
            ticks_per_second: 100.0,
        }
    }

    #[test]
    fn cpu_membership_accounts_new_children_without_crediting_pre_baseline_work_or_pid_reuse() {
        let origin = Instant::now();
        let mut cpu = tracker(origin);
        let processes = BTreeMap::from([
            (1, stat(10, 105)),
            (2, stat(20, 52)),
            (3, stat(30, 900)),
            (4, stat(110, 3)),
        ]);
        cpu.update(tree(processes, origin + Duration::from_millis(100)))
            .unwrap();
        assert_eq!(cpu.ticks, 10);
        let processes = BTreeMap::from([
            (1, stat(10, 110)),
            (2, stat(20, 52)),
            (3, stat(30, 904)),
            (4, stat(110, 6)),
        ]);
        cpu.update(tree(processes, origin + Duration::from_millis(200)))
            .unwrap();
        assert_eq!(cpu.ticks, 22);
        let mut processes = cpu.members.processes.clone();
        processes.insert(3, stat(120, 2));
        cpu.account(&processes).unwrap();
        assert_eq!(cpu.ticks, 24);
        processes.insert(1, stat(130, 1000));
        assert!(cpu.account(&processes).is_err());
        assert_eq!(cpu.ticks, 24);
        processes.remove(&1);
        assert!(cpu.account(&processes).is_err());
        assert_eq!(cpu.membership_scans.len(), 3);
    }

    #[test]
    fn endpoint_counter_reads_validate_root_and_skip_missing_or_reused_descendants() {
        let origin = Instant::now();
        let cpu = tracker(origin);
        let counters = CpuTracker::read_members_with(1, 10, &cpu.members, |identifier| {
            if identifier == 1 {
                Ok(stat(10, 105))
            } else {
                Ok(stat(999, 900))
            }
        })
        .unwrap();
        assert_eq!(counters.len(), 1);
        assert_eq!(counters[&1].ticks, 105);
        assert!(
            CpuTracker::read_members_with(1, 10, &cpu.members, |_| Ok(stat(999, 900))).is_err()
        );
        assert!(
            CpuTracker::read_members_with(1, 10, &cpu.members, |_| Err(io::Error::other("exited")))
                .is_err()
        );
    }

    #[test]
    fn resource_events_keep_cpu_endpoint_separate_from_later_memory_and_real_completion_times() {
        let origin = Instant::now();
        let mut cpu = tracker(origin);
        cpu.ticks = 12;
        cpu.endpoint = Some(Collection {
            started: origin + Duration::from_millis(1000),
            completed: origin + Duration::from_millis(1002),
            process_count: 2,
        });
        let mut sampler = ProcessSampler {
            root: 1,
            ticks_per_second: 100.0,
            previous: BTreeMap::new(),
            cpu_ticks: 0,
            baseline: true,
            samples: Vec::new(),
        };
        let collection = Collection {
            started: origin + Duration::from_millis(1003),
            completed: origin + Duration::from_millis(1250),
            process_count: 2,
        };
        let mut sample = Sample {
            elapsed_ms: 0.0,
            collection_started_ms: None,
            collection_completed_ms: None,
            rss_mib: Some(8.0),
            pss_mib: None,
            process_count: 2,
        };
        Resources::stamp(&mut sample, &collection, origin);
        assert_eq!(sample.elapsed_ms, 1250.0);
        assert!((sample.collection_started_ms.unwrap() - 1003.0).abs() < 1e-9);
        assert_eq!(sample.collection_completed_ms, Some(1250.0));
        sampler.samples.push(sample);
        let mut resources = Resources {
            sampler: &mut sampler,
            cpu: None,
            tree: cpu.members.clone(),
            memory: vec![collection],
            final_memory_completed: Some(origin + Duration::from_millis(1250)),
        };
        let event = resources.event("measurement", origin, &cpu, true);
        let value = serde_json::to_value(event).unwrap();
        assert_eq!(value["event"], "hover-resources");
        assert_eq!(value["cpu_baseline"]["started_ms"], -5.0);
        assert_eq!(value["cpu_baseline"]["completed_ms"], -4.0);
        assert_eq!(value["cpu_endpoint"]["started_ms"], 1000.0);
        assert_eq!(value["cpu_endpoint"]["completed_ms"], 1002.0);
        assert_eq!(value["final_memory_completed_ms"], 1250.0);
        assert_eq!(value["memory_samples"][0]["completed_ms"], 1250.0);
        assert_eq!(value["membership_scans"][0]["process_count"], 2);
        assert_eq!(value["cpu_ms"], 120.0);
        assert!(matches!(
            serde_json::from_value::<Event>(value).unwrap(),
            Event::HoverResources { verified: true, .. }
        ));
        assert_eq!(resources.sampler.cpu_ms(), 120.0);
        assert_eq!(
            cpu.endpoint_elapsed(origin),
            Some(Duration::from_millis(1002))
        );
    }

    #[test]
    fn memory_latency_is_bounded_without_relabeling_it_as_the_cpu_endpoint() {
        let origin = Instant::now();
        let duration = Duration::from_secs(1);
        let mut collection = Collection {
            started: origin + duration,
            completed: origin + Duration::from_secs(3),
            process_count: 1,
        };
        collection.validate_memory(origin, Some(duration)).unwrap();
        collection.completed += Duration::from_nanos(1);
        assert!(collection.validate_memory(origin, Some(duration)).is_err());
        collection.started += Duration::from_secs(1);
        assert!(collection.validate_memory(origin, Some(duration)).is_err());
        collection.validate_memory(origin, None).unwrap();
        collection.completed += Duration::from_secs(2);
        assert!(collection.validate_memory(origin, None).is_err());
    }
}
