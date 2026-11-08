use crate::process::Sample;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metric {
    pub value: f64,
    pub unit: String,
}

impl Metric {
    pub fn new(value: f64, unit: &str) -> Self {
        Self {
            value,
            unit: unit.to_owned(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Repetition {
    pub index: usize,
    pub startup_boundary: String,
    pub metrics: BTreeMap<String, Metric>,
    pub samples: Vec<Sample>,
    pub events: Vec<benchmark_common::Event>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultEntry {
    pub framework: String,
    pub scenario: String,
    pub scenario_version: u32,
    pub status: String,
    pub reason: Option<String>,
    pub metrics: BTreeMap<String, Metric>,
    pub repetitions: Vec<Repetition>,
}

impl ResultEntry {
    pub fn pending(framework: &str, scenario: benchmark_common::Scenario) -> Self {
        Self {
            framework: framework.into(),
            scenario: scenario.id().into(),
            scenario_version: scenario.version(),
            status: "ok".into(),
            reason: None,
            metrics: BTreeMap::new(),
            repetitions: Vec::new(),
        }
    }

    pub fn summarize(&mut self) {
        if self.status != "ok" {
            self.metrics.clear();
            return;
        }
        let mut values: BTreeMap<String, Vec<&Metric>> = BTreeMap::new();
        for repetition in &self.repetitions {
            for (name, metric) in &repetition.metrics {
                values.entry(name.clone()).or_default().push(metric);
            }
        }
        for (name, mut metrics) in values {
            if metrics.len() != self.repetitions.len() {
                continue;
            }
            metrics.sort_by(|left, right| left.value.total_cmp(&right.value));
            let middle = metrics.len() / 2;
            let median = if metrics.len() % 2 == 0 {
                (metrics[middle - 1].value + metrics[middle].value) / 2.0
            } else {
                metrics[middle].value
            };
            self.metrics
                .insert(name, Metric::new(median, &metrics[middle].unit));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::results::{Metric, Repetition, ResultEntry};
    use std::collections::BTreeMap;

    #[test]
    fn summaries_use_repetition_medians_and_require_full_coverage() {
        let mut entry = ResultEntry::pending("egui", benchmark_common::Scenario::Scroll);
        for (index, value) in [1.0, 9.0, 2.0].into_iter().enumerate() {
            let mut metrics = BTreeMap::from([("cpu_ms".into(), Metric::new(value, "ms"))]);
            if index == 0 {
                metrics.insert("pss_mib".into(), Metric::new(12.0, "MiB"));
            }
            entry.repetitions.push(Repetition {
                index,
                startup_boundary: "ui-built".into(),
                metrics,
                samples: vec![],
                events: vec![],
            });
        }
        entry.summarize();
        assert_eq!(entry.metrics["cpu_ms"].value, 2.0);
        assert!(!entry.metrics.contains_key("pss_mib"));
        entry.status = "failed".into();
        entry.summarize();
        assert!(entry.metrics.is_empty());
    }
}
