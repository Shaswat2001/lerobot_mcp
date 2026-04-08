//! Async HTTP client for the Hugging Face Hub REST API and datasets-server API.
//!
//! This is the core I/O layer. Every method here makes one HTTP call and
//! deserializes the response into a typed struct from `hub::types`.
//!
//! No caching here -- that's handled by `CachedClient` in a later phase.

use crate::error::{AppError, Result};
use crate::hub::types::*;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};

/// Client for the Hugging Face Hub REST API and the datasets-server API.
///
/// Holds a `reqwest::Client` (which internally uses `Arc`, so cloning
/// this struct is cheap -- it shares the connection pool).
#[derive(Debug, Clone)]
pub struct HubClient {
    http: reqwest::Client,
    hf_api_base: String,
    ds_server_base: String,
}

impl HubClient {
    /// Create a new HubClient.
    ///
    /// - `token`: Optional HF API token for accessing private/gated datasets.
    ///
    /// The `reqwest::Client` is configured with:
    /// - Custom User-Agent identifying this tool
    /// - Optional Bearer token
    /// - 30s timeout
    pub fn new(token: Option<&str>) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(concat!("lerobot-mcp/", env!("CARGO_PKG_VERSION"))),
        );
        if let Some(tok) = token {
            let val = HeaderValue::from_str(&format!("Bearer {tok}"))
                .map_err(|e| AppError::InvalidConfig {
                    message: format!("Invalid HF token: {e}"),
                })?;
            headers.insert(AUTHORIZATION, val);
        }

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AppError::InvalidConfig {
                message: format!("Failed to build HTTP client: {e}"),
            })?;

        Ok(Self {
            http,
            hf_api_base: "https://huggingface.co/api".to_string(),
            ds_server_base: "https://datasets-server.huggingface.co".to_string(),
        })
    }

    // ─── Hub REST API ────────────────────────────────────────────────

    /// Search for LeRobot datasets on the Hub.
    ///
    /// Calls: `GET /api/datasets?search={query}&filter=other:LeRobot&sort=downloads&direction=-1&limit={limit}&full=true&cardData=true`
    #[tracing::instrument(skip(self), fields(query, limit))]
    pub async fn search_datasets(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<DatasetListItem>> {
        let url = format!(
            "{}/datasets?search={}&filter=LeRobot&sort=downloads&direction=-1&limit={}&full=true&cardData=true",
            self.hf_api_base,
            urlencoding::encode(query),
            limit,
        );
        tracing::debug!(url = %url, "Searching datasets");

        let resp = self.http.get(&url).send().await.map_err(AppError::Http)?;
        let status = resp.status();

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(self.map_error(status.as_u16(), &url, body));
        }

        let items: Vec<DatasetListItem> = resp.json().await.map_err(AppError::Http)?;
        tracing::info!(count = items.len(), "Search returned results");
        Ok(items)
    }

    /// Get detailed info for a single dataset from the Hub REST API.
    ///
    /// Calls: `GET /api/datasets/{repo_id}?full=true`
    #[tracing::instrument(skip(self), fields(repo_id))]
    pub async fn dataset_detail(&self, repo_id: &str) -> Result<DatasetDetail> {
        let url = format!(
            "{}/datasets/{}?full=true",
            self.hf_api_base,
            urlencoding::encode(repo_id),
        );

        let resp = self.http.get(&url).send().await.map_err(AppError::Http)?;
        let status = resp.status();

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(self.map_error(status.as_u16(), &url, body));
        }

        resp.json().await.map_err(AppError::Http)
    }

    // ─── Datasets-server API ─────────────────────────────────────────

    /// Get column schemas and split info from the datasets-server.
    ///
    /// Calls: `GET /info?dataset={repo_id}`
    #[tracing::instrument(skip(self), fields(repo_id))]
    pub async fn dataset_server_info(&self, repo_id: &str) -> Result<DatasetServerInfo> {
        let url = format!(
            "{}/info?dataset={}",
            self.ds_server_base,
            urlencoding::encode(repo_id),
        );

        let resp = self.http.get(&url).send().await.map_err(AppError::Http)?;
        let status = resp.status();

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(self.map_error(status.as_u16(), &url, body));
        }

        resp.json().await.map_err(AppError::Http)
    }

    /// Get size info (rows, bytes) from the datasets-server.
    ///
    /// Calls: `GET /size?dataset={repo_id}`
    #[tracing::instrument(skip(self), fields(repo_id))]
    pub async fn dataset_size(&self, repo_id: &str) -> Result<DatasetSize> {
        let url = format!(
            "{}/size?dataset={}",
            self.ds_server_base,
            urlencoding::encode(repo_id),
        );

        let resp = self.http.get(&url).send().await.map_err(AppError::Http)?;
        let status = resp.status();

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(self.map_error(status.as_u16(), &url, body));
        }

        resp.json().await.map_err(AppError::Http)
    }

    /// Get row data for a specific slice of a dataset.
    ///
    /// Calls: `GET /rows?dataset={repo_id}&config={config}&split={split}&offset={offset}&length={length}`
    #[tracing::instrument(skip(self), fields(repo_id, config, split, offset, length))]
    pub async fn dataset_rows(
        &self,
        repo_id: &str,
        config: &str,
        split: &str,
        offset: u64,
        length: u32,
    ) -> Result<DatasetRows> {
        let url = format!(
            "{}/rows?dataset={}&config={}&split={}&offset={}&length={}",
            self.ds_server_base,
            urlencoding::encode(repo_id),
            urlencoding::encode(config),
            urlencoding::encode(split),
            offset,
            length.min(100), // datasets-server caps at 100
        );

        let resp = self.http.get(&url).send().await.map_err(AppError::Http)?;
        let status = resp.status();

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(self.map_error(status.as_u16(), &url, body));
        }

        resp.json().await.map_err(AppError::Http)
    }

    /// Filter rows by a column value (used for episode preview).
    ///
    /// Calls: `GET /filter?dataset={repo_id}&config={config}&split={split}&where={where_clause}&offset={offset}&length={length}`
    #[tracing::instrument(skip(self), fields(repo_id, where_clause))]
    pub async fn filter_rows(
        &self,
        repo_id: &str,
        config: &str,
        split: &str,
        where_clause: &str,
        offset: u64,
        length: u32,
    ) -> Result<DatasetRows> {
        let url = format!(
            "{}/filter?dataset={}&config={}&split={}&where={}&offset={}&length={}",
            self.ds_server_base,
            urlencoding::encode(repo_id),
            urlencoding::encode(config),
            urlencoding::encode(split),
            urlencoding::encode(where_clause),
            offset,
            length.min(100),
        );

        let resp = self.http.get(&url).send().await.map_err(AppError::Http)?;
        let status = resp.status();

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(self.map_error(status.as_u16(), &url, body));
        }

        resp.json().await.map_err(AppError::Http)
    }

    // ─── Error mapping ───────────────────────────────────────────────

    fn map_error(&self, status: u16, url: &str, body: String) -> AppError {
        match status {
            404 => {
                // Try to extract repo_id from the URL for a nicer error
                let repo_id = url
                    .split("/datasets/")
                    .nth(1)
                    .and_then(|s| s.split('?').next())
                    .unwrap_or("unknown")
                    .to_string();
                AppError::NotFound { repo_id }
            }
            429 => {
                // Parse retry-after from body if available
                AppError::RateLimited {
                    retry_after_secs: 60,
                }
            }
            _ => AppError::HubApi {
                status,
                url: url.to_string(),
                body,
            },
        }
    }
}