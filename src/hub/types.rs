use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetListItem {
    pub id: String,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub likes: u64,
    pub last_modified: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub description: Option<String>,
}