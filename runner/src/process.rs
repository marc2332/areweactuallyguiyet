pub mod hover;

use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    process::{Child, Command},
    time::Instant,
};

pub struct ChildGuard(pub Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            let _ = Command::new("kill")
                .args(["-KILL", "--", &format!("-{}", self.0.id())])
                .output();
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[derive(Debug, Clone)]
struct ProcessStat {
    parent: u32,
    ticks: u64,
    start: u64,
}

fn parse_stat(contents: &str) -> Option<ProcessStat> {
    let closing = contents.rfind(')')?;
    let fields: Vec<_> = contents.get(closing + 2..)?.split_whitespace().collect();
    Some(ProcessStat {
        parent: fields.get(1)?.parse().ok()?,
        ticks: fields
            .get(11)?
            .parse::<u64>()
            .ok()?
            .checked_add(fields.get(12)?.parse::<u64>().ok()?)?,
        start: fields.get(19)?.parse().ok()?,
    })
}

#[derive(Clone)]
pub(crate) struct ProcessTree {
    processes: BTreeMap<u32, ProcessStat>,
    started: Instant,
    completed: Instant,
}

impl ProcessTree {
    fn discover(root: u32) -> io::Result<Self> {
        let started = Instant::now();
        let mut processes = BTreeMap::new();
        for entry in fs::read_dir("/proc")? {
            let entry = entry?;
            let Ok(identifier) = entry.file_name().to_string_lossy().parse::<u32>() else {
                continue;
            };
            if let Ok(contents) = fs::read_to_string(entry.path().join("stat")) {
                if let Some(stat) = parse_stat(&contents) {
                    processes.insert(identifier, stat);
                }
            }
        }
        let mut descendants = BTreeSet::from([root]);
        loop {
            let before = descendants.len();
            for (identifier, stat) in &processes {
                if descendants.contains(&stat.parent) {
                    descendants.insert(*identifier);
                }
            }
            if before == descendants.len() {
                break;
            }
        }
        processes.retain(|identifier, _| descendants.contains(identifier));
        Ok(Self {
            processes,
            started,
            completed: Instant::now(),
        })
    }
}

fn memory_kib(contents: &str, field: &str) -> Option<u64> {
    contents.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key != field {
            return None;
        }
        value.split_whitespace().next()?.parse().ok()
    })
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub elapsed_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_started_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_completed_ms: Option<f64>,
    pub rss_mib: Option<f64>,
    pub pss_mib: Option<f64>,
    pub process_count: usize,
}

pub struct ProcessSampler {
    root: u32,
    ticks_per_second: f64,
    previous: BTreeMap<(u32, u64), u64>,
    cpu_ticks: u64,
    baseline: bool,
    pub samples: Vec<Sample>,
}

impl ProcessSampler {
    pub fn new(root: u32) -> io::Result<Self> {
        if !cfg!(target_os = "linux") {
            return Err(io::Error::other(
                "Process measurements currently require Linux procfs",
            ));
        }
        let output = Command::new("getconf").arg("CLK_TCK").output()?;
        if !output.status.success() {
            return Err(io::Error::other("getconf CLK_TCK failed"));
        }
        let ticks_per_second = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<f64>()
            .map_err(io::Error::other)?;
        if !ticks_per_second.is_finite() || ticks_per_second <= 0.0 {
            return Err(io::Error::other("Invalid CLK_TCK"));
        }
        Ok(Self {
            root,
            ticks_per_second,
            previous: BTreeMap::new(),
            cpu_ticks: 0,
            baseline: true,
            samples: Vec::new(),
        })
    }

    pub fn sample(&mut self, elapsed_ms: f64) -> io::Result<()> {
        let tree = ProcessTree::discover(self.root)?;
        let mut rss_total = 0u64;
        let mut pss_total = 0u64;
        let mut rss_available = true;
        let mut pss_available = true;
        let mut count = 0;
        for (identifier, stat) in tree.processes {
            count += 1;
            let key = (identifier, stat.start);
            let previous = self.previous.insert(key, stat.ticks);
            if let Some(previous) = previous {
                self.cpu_ticks += stat.ticks.saturating_sub(previous);
            } else if !self.baseline {
                self.cpu_ticks += stat.ticks;
            }
            let status = fs::read_to_string(format!("/proc/{identifier}/status")).ok();
            if let Some(rss) = status
                .as_deref()
                .and_then(|contents| memory_kib(contents, "VmRSS"))
            {
                rss_total += rss;
            } else {
                rss_available = false;
            }
            let rollup = fs::read_to_string(format!("/proc/{identifier}/smaps_rollup")).ok();
            if let Some(pss) = rollup
                .as_deref()
                .and_then(|contents| memory_kib(contents, "Pss"))
            {
                pss_total += pss;
            } else {
                pss_available = false;
            }
        }
        self.baseline = false;
        self.samples.push(Sample {
            elapsed_ms,
            collection_started_ms: None,
            collection_completed_ms: None,
            rss_mib: (rss_available && count > 0).then_some(rss_total as f64 / 1024.0),
            pss_mib: (pss_available && count > 0).then_some(pss_total as f64 / 1024.0),
            process_count: count,
        });
        Ok(())
    }

    pub fn cpu_ms(&self) -> f64 {
        self.cpu_ticks as f64 / self.ticks_per_second * 1000.0
    }
}

#[cfg(test)]
mod tests {
    use crate::process::{memory_kib, parse_stat};

    #[test]
    fn process_names_can_contain_parentheses_and_spaces() {
        let stat =
            parse_stat("123 (name with ) space) S 42 0 0 0 0 0 0 0 0 0 10 20 0 0 0 0 0 0 100 0 0")
                .unwrap();
        assert_eq!(stat.parent, 42);
        assert_eq!(stat.ticks, 30);
        assert_eq!(stat.start, 100);
    }

    #[test]
    fn memory_fields_are_exact() {
        assert_eq!(memory_kib("SwapPss: 9 kB\nPss: 123 kB\n", "Pss"), Some(123));
        assert_eq!(memory_kib("VmSize: 12 kB", "VmRSS"), None);
    }
}
