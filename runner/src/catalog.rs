use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Adapter {
    pub id: String,
    pub name: Option<String>,
    pub base_id: Option<String>,
    pub version: String,
    pub package: String,
    pub manifest: String,
    pub scenarios: BTreeMap<String, Capability>,
}

#[derive(Debug, Deserialize)]
pub struct Capability {
    pub status: String,
    pub reason: Option<String>,
}

pub fn load(root: &Path) -> Result<Vec<Adapter>, Box<dyn std::error::Error>> {
    let path = root.join("apps/catalog.json");
    let adapters: Vec<Adapter> = serde_json::from_slice(&fs::read(&path)?)?;
    let mut identifiers = std::collections::BTreeSet::new();
    for adapter in &adapters {
        if !identifiers.insert(&adapter.id) {
            return Err(format!("Duplicate adapter {}", adapter.id).into());
        }
        let manifest = Path::new(&adapter.manifest);
        if manifest.is_absolute()
            || manifest
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(format!("Invalid manifest path for {}", adapter.id).into());
        }
        for capability in adapter.scenarios.values() {
            if !["implemented", "unsupported", "unavailable"].contains(&capability.status.as_str())
            {
                return Err(format!("Invalid capability status for {}", adapter.id).into());
            }
        }
    }
    Ok(adapters)
}
