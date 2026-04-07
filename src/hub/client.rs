use crate::error::{AppError, Result};
use crate::hub::types::DatasetListItem;

pub struct HubClient {
    http: reqwest::Client,
}

impl HubClient {
    pub fn new(token: Option<&str>) -> Self {
        let mut builder = reqwest::Client::builder()
            .user_agent("lerobot-mcp/0.1.0")
            .timeout(std::time::Duration::from_secs(30));

        // If we have a token, set it as default auth header
        if let Some(t) = token {
            let mut headers = reqwest::header::HeaderMap::new();
            let val = format!("Bearer {}", t);
            if let Ok(header_val) = reqwest::header::HeaderValue::from_str(&val) {
                headers.insert(reqwest::header::AUTHORIZATION, header_val);
            }
            builder = builder.default_headers(headers);
        }

        HubClient {
            http: builder.build().expect("Failed to build HTTP client"),
        }
    }

    /// Search for LeRobot datasets on the Hub.
    pub async fn search_datasets(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<DatasetListItem>> {
        let url = format!(
            "https://huggingface.co/api/datasets?search={}&filter=other:LeRobot&sort=downloads&direction=-1&limit={}&full=true",
            query, limit
        );

        let resp = self.http.get(&url).send().await.map_err(AppError::from)?;
        let status = resp.status().as_u16();

        if status == 429 {
            return Err(AppError::RateLimited { retry_after_secs: 60 });
        }
        if status >= 400 {
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::HubApi { status, url, body });
        }

        let items: Vec<DatasetListItem> = resp.json().await.map_err(AppError::from)?;
        Ok(items)
    }
}