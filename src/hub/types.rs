//! Serde types for Hugging Face Hub API and datasets-server API responses.
//!
//! Every struct here matches a real API response shape. If a field is optional
//! in the API (missing from some responses), it's `Option<T>` here.
//! Use `#[serde(default)]` liberally because community LeRobot datasets
//! have wildly inconsistent metadata.

use serde::Deserialize;
use std::collections::HashMap;

// ─── Hub REST API: GET /api/datasets?search=...&full=true&cardData=true ───
 
/// A single dataset from the Hub listing/search endpoint.
///
/// Endpoint: `GET https://huggingface.co/api/datasets?search={q}&filter=other:LeRobot&full=true&cardData=true`
/// The response is a JSON array of these objects.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetListItem {
    /// Full repo ID, e.g. "lerobot/aloha_mobile_cabinet"
    pub id: String,
 
    /// Display name (often same as the repo name portion of id)
    #[serde(default)]
    pub author: Option<String>,
 
    /// Number of likes on the Hub
    #[serde(default)]
    pub likes: Option<u64>,
 
    /// Number of downloads (last 30 days)
    #[serde(default)]
    pub downloads: Option<u64>,
 
    /// Total all-time downloads
    #[serde(default)]
    pub downloads_all_time: Option<u64>,
 
    /// ISO timestamp of last modification
    #[serde(default)]
    pub last_modified: Option<String>,
 
    /// ISO timestamp of creation
    #[serde(default)]
    pub created_at: Option<String>,
 
    /// Tags applied to the dataset (e.g. ["LeRobot", "robotics"])
    #[serde(default)]
    pub tags: Option<Vec<String>>,
 
    /// Whether the dataset is private
    #[serde(default)]
    pub private: Option<bool>,
 
    /// Dataset card YAML frontmatter, parsed into JSON.
    /// Only present when `cardData=true` in the request.
    /// This is where LeRobot-specific metadata lives.
    #[serde(default)]
    pub card_data: Option<CardData>,
 
    /// Git SHA of the latest commit
    #[serde(default)]
    pub sha: Option<String>,
 
    /// Short description from the dataset card
    #[serde(default)]
    pub description: Option<String>,
 
    /// Citation string
    #[serde(default)]
    pub citation: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CardData {
    /// Tags from YAML frontmatter (e.g. ["LeRobot", "so100", "tutorial"])
    pub tags: Option<Vec<String>>,
 
    /// Task categories (e.g. ["robotics"])
    pub task_categories: Option<Vec<String>>,
 
    /// License (e.g. "apache-2.0")
    pub license: Option<String>,
 
    /// Dataset configs block. In LeRobot datasets, this contains
    /// info about which Parquet files belong to which split.
    pub configs: Option<Vec<DatasetConfig>>,
 
    /// Additional fields we don't explicitly model.
    /// Captures things like `robot_type`, `fps`, `features`, etc.
    /// that some LeRobot datasets put in their card YAML.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A config entry from the dataset card YAML.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DatasetConfig {
    pub config_name: Option<String>,
    pub data_files: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default)]
pub struct LeRobotMetadata {
    /// Robot type (e.g. "so100", "aloha", "widowx", "koch")
    pub robot_type: Option<String>,
 
    /// Frames per second
    pub fps: Option<u32>,
 
    /// Number of episodes (from card metadata, not always present)
    pub num_episodes: Option<u64>,
 
    /// Number of total frames
    pub num_frames: Option<u64>,
 
    /// Task descriptions
    pub tasks: Vec<String>,
 
    /// Feature names (e.g. "observation.state", "action", "observation.images.top")
    pub features: Vec<String>,
 
    /// Environment type (e.g. "real_world", "sim_transfer_cube")
    pub env_type: Option<String>,
}

impl CardData {
    /// Extract LeRobot-specific metadata from the card data.
    /// This does best-effort parsing since community datasets are inconsistent.
    pub fn lerobot_metadata(&self) -> LeRobotMetadata {
        let mut meta = LeRobotMetadata::default();
 
        // robot_type: sometimes in extra as a direct key
        if let Some(val) = self.extra.get("robot_type") {
            meta.robot_type = val.as_str().map(|s| s.to_string());
        }
 
        // fps
        if let Some(val) = self.extra.get("fps") {
            meta.fps = val.as_u64().map(|v| v as u32);
        }
 
        // num_episodes
        if let Some(val) = self.extra.get("num_episodes") {
            meta.num_episodes = val.as_u64();
        }
 
        // num_frames
        if let Some(val) = self.extra.get("num_frames") {
            meta.num_frames = val.as_u64();
        }
 
        // env_type
        if let Some(val) = self.extra.get("env_type") {
            meta.env_type = val.as_str().map(|s| s.to_string());
        }
 
        // tasks: can be a string or array
        if let Some(val) = self.extra.get("tasks") {
            if let Some(arr) = val.as_array() {
                meta.tasks = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            } else if let Some(s) = val.as_str() {
                meta.tasks = vec![s.to_string()];
            }
        }
 
        // features: typically a map of feature_name -> {type, shape}
        if let Some(val) = self.extra.get("features") {
            if let Some(obj) = val.as_object() {
                meta.features = obj.keys().cloned().collect();
            }
        }

        meta
    }
}

impl DatasetListItem {
    /// Extract LeRobot metadata from the description field.
    /// The description often contains a dump of meta/info.json with
    /// fields like robot_type, fps, total_episodes, total_frames.
    pub fn lerobot_metadata(&self) -> LeRobotMetadata {
        // First try cardData
        let mut meta = self
            .card_data
            .as_ref()
            .map(|cd| cd.lerobot_metadata())
            .unwrap_or_default();

        // Then try parsing meta/info.json from the description
        if let Some(desc) = &self.description {
            if let Some(start) = desc.find('{') {
                // Find the matching closing brace for the JSON block
                if let Some(json_str) = extract_json_block(&desc[start..]) {
                    if let Ok(info) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        if meta.robot_type.is_none() {
                            meta.robot_type = info
                                .get("robot_type")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                        if meta.fps.is_none() {
                            meta.fps = info.get("fps").and_then(|v| v.as_u64()).map(|v| v as u32);
                        }
                        if meta.num_episodes.is_none() {
                            meta.num_episodes =
                                info.get("total_episodes").and_then(|v| v.as_u64());
                        }
                        if meta.num_frames.is_none() {
                            meta.num_frames = info.get("total_frames").and_then(|v| v.as_u64());
                        }
                        if meta.features.is_empty() {
                            if let Some(obj) = info.get("features").and_then(|v| v.as_object()) {
                                meta.features = obj.keys().cloned().collect();
                            }
                        }
                    }
                }
            }
        }

        meta
    }
}

/// Extract a JSON object from a string, handling nested braces.
fn extract_json_block(s: &str) -> Option<String> {
    let mut depth = 0;
    let mut end = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    if end > 0 {
        Some(s[..end].to_string())
    } else {
        None
    }
}
// ─── Hub REST API: GET /api/datasets/{repo_id}?full=true ─────────────────
 
/// Detailed info for a single dataset. Same fields as DatasetListItem
/// but guaranteed to be fully populated (unlike search results where
/// some fields may be missing).
pub type DatasetDetail = DatasetListItem;

#[derive(Debug, Clone, Deserialize)]
pub struct DatasetServerInfo {
    /// Map from config name to info for that config.
    pub dataset_info: HashMap<String, ConfigInfo>,
}
 
/// Info for a single config (subset) of a dataset.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct ConfigInfo {
    /// Description of this config
    pub description: Option<String>,
 
    /// Citation
    pub citation: Option<String>,
 
    /// Features (column schemas).
    /// Keys are column names, values describe the type.
    pub features: Option<HashMap<String, serde_json::Value>>,
 
    /// Splits info (train, test, etc.)
    pub splits: Option<HashMap<String, SplitInfo>>,
 
    /// Builder name
    pub builder_name: Option<String>,
 
    /// Config name
    pub config_name: Option<String>,
 
    /// Dataset name
    pub dataset_name: Option<String>,
 
    /// Version
    pub version: Option<serde_json::Value>,
 
    /// Download size
    pub download_size: Option<u64>,
 
    /// Dataset size
    pub dataset_size: Option<u64>,
}

/// Info about a single split (e.g. "train").
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct SplitInfo {
    pub name: Option<String>,
    pub num_bytes: Option<u64>,
    pub num_examples: Option<u64>,
    pub dataset_name: Option<String>,
}

// ─── Datasets-server API: GET /size?dataset={repo_id} ────────────────────
 
/// Response from the datasets-server `/size` endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct DatasetSize {
    pub size: SizePayload,
}
 
#[derive(Debug, Clone, Deserialize)]
pub struct SizePayload {
    pub dataset: DatasetSizeInfo,
    #[serde(default)]
    pub configs: Vec<ConfigSizeInfo>,
    #[serde(default)]
    pub splits: Vec<SplitSizeInfo>,
}
 
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct DatasetSizeInfo {
    pub dataset: Option<String>,
    pub num_bytes_original_files: Option<u64>,
    pub num_bytes_parquet_files: Option<u64>,
    pub num_bytes_memory: Option<u64>,
    pub num_rows: Option<u64>,
}
 
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct ConfigSizeInfo {
    pub dataset: Option<String>,
    pub config: Option<String>,
    pub num_bytes_original_files: Option<u64>,
    pub num_bytes_parquet_files: Option<u64>,
    pub num_bytes_memory: Option<u64>,
    pub num_rows: Option<u64>,
    pub num_columns: Option<u64>,
}
 
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct SplitSizeInfo {
    pub dataset: Option<String>,
    pub config: Option<String>,
    pub split: Option<String>,
    pub num_bytes_parquet_files: Option<u64>,
    pub num_bytes_memory: Option<u64>,
    pub num_rows: Option<u64>,
    pub num_columns: Option<u64>,
}
 
// ─── Datasets-server API: GET /rows?dataset=...&config=...&split=... ─────
 
/// Response from the datasets-server `/rows` or `/filter` endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct DatasetRows {
    /// Column schemas
    pub features: Vec<FeatureInfo>,
 
    /// The actual row data
    pub rows: Vec<RowEntry>,
 
    /// Total number of rows (for the filtered/full split)
    #[serde(default)]
    pub num_rows_total: Option<u64>,
}
 
#[derive(Debug, Clone, Deserialize)]
pub struct FeatureInfo {
    pub feature_idx: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub dtype: serde_json::Value,
}
 
#[derive(Debug, Clone, Deserialize)]
pub struct RowEntry {
    pub row_idx: u64,
    pub row: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub truncated_cells: Option<Vec<String>>,
}
 
// ─── Utility functions ───────────────────────────────────────────────────
 
/// Format bytes into human-readable string (e.g. "1.2 GB")
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;
 
    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
 
#[cfg(test)]
mod tests {
    use super::*;
 
    #[test]
    fn format_bytes_works() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1_500_000), "1.4 MB");
        assert_eq!(format_bytes(2_500_000_000), "2.3 GB");
    }
 
    #[test]
    fn card_data_deserializes_with_defaults() {
        let json = r#"{}"#;
        let cd: CardData = serde_json::from_str(json).unwrap();
        assert!(cd.tags.is_none());
        assert!(cd.extra.is_empty());
    }
 
    #[test]
    fn card_data_extracts_lerobot_metadata() {
        let json = r#"{
            "tags": ["LeRobot", "so100"],
            "robot_type": "so100",
            "fps": 30,
            "num_episodes": 50,
            "features": {
                "observation.state": {"type": "float32", "shape": [6]},
                "action": {"type": "float32", "shape": [6]}
            }
        }"#;
        let cd: CardData = serde_json::from_str(json).unwrap();
        let meta = cd.lerobot_metadata();
        assert_eq!(meta.robot_type.as_deref(), Some("so100"));
        assert_eq!(meta.fps, Some(30));
        assert_eq!(meta.num_episodes, Some(50));
        assert_eq!(meta.features.len(), 2);
        assert!(meta.features.contains(&"action".to_string()));
    }
 
    #[test]
    fn dataset_list_item_deserializes_partial() {
        // Simulates a minimal response with missing optional fields
        let json = r#"{"id": "user/my-dataset"}"#;
        let item: DatasetListItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, "user/my-dataset");
        assert!(item.likes.is_none());
        assert!(item.card_data.is_none());
    }
 
    #[test]
    fn dataset_size_deserializes() {
        let json = r#"{
            "size": {
                "dataset": {
                    "dataset": "lerobot/aloha",
                    "num_bytes_original_files": 58710973,
                    "num_bytes_parquet_files": 58710973,
                    "num_bytes_memory": 1060742354,
                    "num_rows": 187213
                },
                "configs": [],
                "splits": []
            }
        }"#;
        let size: DatasetSize = serde_json::from_str(json).unwrap();
        assert_eq!(size.size.dataset.num_rows, Some(187213));
    }
 
    #[test]
    fn dataset_rows_deserializes() {
        let json = r#"{
            "features": [
                {"feature_idx": 0, "name": "action", "type": {"dtype": "float32", "_type": "Value"}}
            ],
            "rows": [
                {"row_idx": 0, "row": {"action": 1.5}}
            ]
        }"#;
        let rows: DatasetRows = serde_json::from_str(json).unwrap();
        assert_eq!(rows.features.len(), 1);
        assert_eq!(rows.features[0].name, "action");
        assert_eq!(rows.rows.len(), 1);
    }
}