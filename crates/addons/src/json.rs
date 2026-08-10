//! Serde payloads for the FFI JSON contract (ADDON_SPEC §12.2).

use std::collections::BTreeMap;

use serde::Serialize;

use crate::package::AddonDescriptor;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonDescriptor {
    pub id: String,
    pub name: String,
    pub version: String,
    pub entry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    pub api_version: [u32; 2],
    pub provider: String,
    pub root_kind: String,
    pub size_bytes: u64,
    pub has_icons: BTreeMap<String, bool>,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct ListPayload {
    pub addons: Vec<JsonDescriptor>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SinglePayload {
    pub addon: JsonDescriptor,
}

impl JsonDescriptor {
    pub fn from_descriptor(d: &AddonDescriptor) -> Self {
        Self {
            id: d.id.clone(),
            name: d.name.clone(),
            version: d.version.to_string(),
            entry: d.entry.clone(),
            description: d.description.clone(),
            author: d.author.clone(),
            license: d.license.clone(),
            api_version: [d.api_version.0, d.api_version.1],
            provider: d.provider.clone(),
            root_kind: d.root_kind.as_str().to_string(),
            size_bytes: d.size_bytes,
            has_icons: d.has_icons.clone(),
            state: d.state.clone(),
            error: d.error.clone(),
        }
    }
}

pub fn list_json(descriptors: &[AddonDescriptor]) -> String {
    let payload = ListPayload {
        addons: descriptors
            .iter()
            .map(JsonDescriptor::from_descriptor)
            .collect(),
    };
    serde_json::to_string(&payload).expect("descriptor serialization cannot fail")
}

pub fn single_json(d: &AddonDescriptor) -> String {
    let payload = SinglePayload {
        addon: JsonDescriptor::from_descriptor(d),
    };
    serde_json::to_string(&payload).expect("descriptor serialization cannot fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::{AddonDescriptor, RootKind};

    fn sample() -> AddonDescriptor {
        AddonDescriptor {
            id: "example.addon".into(),
            name: "Example".into(),
            version: semver::Version::new(1, 0, 0),
            entry: "src/main.lua".into(),
            description: None,
            author: None,
            license: None,
            api_version: (1, 0),
            provider: "local".into(),
            root_kind: RootKind::Directory,
            size_bytes: 42,
            has_icons: BTreeMap::from([("128".into(), true)]),
            state: "installed".into(),
            error: None,
        }
    }

    #[test]
    fn serializes_expected_fields() {
        let v: serde_json::Value = serde_json::from_str(&single_json(&sample())).unwrap();
        assert_eq!(v["addon"]["id"], "example.addon");
        assert_eq!(v["addon"]["version"], "1.0.0");
        assert_eq!(v["addon"]["rootKind"], "directory");
        assert_eq!(v["addon"]["state"], "installed");
        assert!(v["addon"].get("description").is_none());
    }

    #[test]
    fn list_payload_is_array() {
        let v: serde_json::Value = serde_json::from_str(&list_json(&[sample()])).unwrap();
        assert_eq!(v["addons"].as_array().unwrap().len(), 1);
        assert_eq!(v["addons"][0]["provider"], "local");
    }
}
