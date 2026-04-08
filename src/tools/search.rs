//! search_datasets MCP tool.
//!
//! Searches for LeRobot datasets on the Hugging Face Hub with optional
//! filtering by robot type and minimum episode count.
use crate::hub::client::HubClient;
use crate::hub::types::DatasetListItem;

/// Result of a dataset search, formatted for LLM consumption.
pub struct SearchResult {
    pub items: Vec<SearchResultItem>,
    pub total_found: usize,
    pub total_after_filter: usize,
}

pub struct SearchResultItem {
    pub repo_id: String,
    pub robot_type: Option<String>,
    pub num_episodes: Option<u64>,
    pub downloads: Option<u64>,
    pub likes: Option<u64>,
    pub description: Option<String>,
    pub fps: Option<u32>,
    pub tasks: Vec<String>,
}

impl SearchResult {
    /// Format as a markdown table for LLM readability. 
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();

        if self.items.is_empty() {
            out.push_str("No datasets found matching your criteria.\n");
            return out;
        }

        out.push_str(&format!(
            "Found {} datasets ({} after filtering):\n\n",
            self.total_found, self.total_after_filter
        ));


        out.push_str("| Dataset | Robot | Episodes | Downloads | FPS | Tasks |\n");
        out.push_str("|---------|-------|----------|-----------|-----|-------|\n");

        for item in &self.items {
            let robot = item.robot_type.as_deref().unwrap_or("-");
            let episodes = item.num_episodes.map(|s| s.to_string()).unwrap_or_else(|| "-".to_string());
            let downloads = item.downloads.map(|s| s.to_string()).unwrap_or_else(|| "-".to_string());
            let fps = item.fps.map(|s| s.to_string()).unwrap_or_else(|| "-".to_string());
            let tasks = if item.tasks.is_empty() {
                "-".to_string()
            } else {
                item.tasks.join(", ")
            };

            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                item.repo_id, robot, episodes, downloads, fps, tasks
            ));

        }

        out
    }
}

/// Execute a dataset search with optional post-filtering.
///
/// The HF Hub API handles text search and the LeRobot tag filter.
/// We do robot_type and min_episodes filtering client-side because
/// these are stored in the YAML frontmatter (cardData), not as
/// first-class Hub API filter parameters.
pub async fn execute_search(
    client: &HubClient,
    query: &str,
    robot_type: Option<&str>,
    min_episodes: Option<u32>,
    limit: u32,
) -> crate::error::Result<SearchResult> {
    // Fetch more than requested to account for post-filtering
    let fetch_limit = if robot_type.is_some() || min_episodes.is_some() {
        (limit * 3).min(100) // over-fetch, cap at API max
    } else {
        limit
    };
 
    let items: Vec<DatasetListItem> = client.search_datasets(query, fetch_limit).await?;
    let total_found = items.len();
 
    // Post-filter and transform
    let filtered: Vec<SearchResultItem> = items
        .into_iter()
        .filter_map(|item| {
            let meta = item
                .card_data
                .as_ref()
                .map(|cd| cd.lerobot_metadata())
                .unwrap_or_default();
 
            // Filter by robot_type if specified
            if let Some(rt) = robot_type {
                let rt_lower = rt.to_lowercase();
                let matches = meta
                    .robot_type
                    .as_ref()
                    .map(|r| r.to_lowercase().contains(&rt_lower))
                    .unwrap_or(false);
                if !matches {
                    // Also check tags for robot type
                    let tag_match = item
                        .tags
                        .as_ref()
                        .map(|tags| {
                            tags.iter()
                                .any(|t| t.to_lowercase().contains(&rt_lower))
                        })
                        .unwrap_or(false);
                    if !tag_match {
                        return None;
                    }
                }
            }
 
            // Filter by min_episodes if specified
            if let Some(min_ep) = min_episodes {
                if let Some(num_ep) = meta.num_episodes {
                    if num_ep < min_ep as u64 {
                        return None;
                    }
                }
                // If num_episodes is unknown, keep the item (don't filter out)
            }
 
            Some(SearchResultItem {
                repo_id: item.id,
                robot_type: meta.robot_type,
                num_episodes: meta.num_episodes,
                downloads: item.downloads,
                likes: item.likes,
                description: item.description,
                fps: meta.fps,
                tasks: meta.tasks,
            })
        })
        .take(limit as usize)
        .collect();
 
    let total_after_filter = filtered.len();
 
    Ok(SearchResult {
        items: filtered,
        total_found,
        total_after_filter,
    })
}