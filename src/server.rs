use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router,
};

use crate::hub::client::HubClient;
use crate::tools::search;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EchoParams {
    #[schemars(description = "The message to echo back")]
    pub message: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchDatasetsParams {
    #[schemars(description = "Search query string")]
    pub query: String,
    #[schemars(description = "Filter by robot type (e.g. 'so100', 'aloha', 'widowx', 'koch')")]
    pub robot_type: Option<String>,
    #[schemars(description = "Minimum number of episodes")]
    pub min_episodes: Option<u32>,
    #[schemars(description = "Maximum number of results to return (default 10, max 50)")]
    pub limit: Option<u32>,
}

/// The MCP server for LeRobot dataset operations.
#[derive(Clone)]
pub struct LeRobotServer {
    client: Arc<HubClient>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl LeRobotServer {
    pub fn new(client: HubClient) -> Self {
        Self {
            client: Arc::new(client),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Echo back the input message. Used for testing connectivity.")]
    async fn echo(
        &self,
        Parameters(EchoParams { message }): Parameters<EchoParams>,
    ) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(format!(
            "echo: {message}"
        ))]))
    }

    #[tool(
        description = "Search for LeRobot robotics datasets on the Hugging Face Hub. \
        Returns datasets matching your query with metadata like robot type, \
        episode count, FPS, and task descriptions. Use robot_type to filter \
        by a specific robot (e.g. 'so100', 'aloha', 'widowx', 'koch')."
    )]
    async fn search_datasets(
        &self,
        Parameters(params): Parameters<SearchDatasetsParams>,
    ) -> Result<CallToolResult, McpError> {
        let query = params.query.trim();
        if query.is_empty() {
            return Err(McpError::invalid_params(
                "query parameter cannot be empty",
                None,
            ));
        }
        let limit = params.limit.unwrap_or(10).min(50);
        let result = search::execute_search(
            &self.client,
            query,
            params.robot_type.as_deref(),
            params.min_episodes,
            limit,
        )
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(
            result.to_markdown(),
        )]))
    }
}

#[tool_handler]
impl ServerHandler for LeRobotServer {
    fn get_info(&self) -> ServerInfo {
        let mut __info__ = ServerInfo::default();
        __info__.protocol_version = ProtocolVersion::V_2025_06_18;
        __info__.capabilities = ServerCapabilities::builder()
            .enable_tools()
            .build();
        __info__.server_info = Implementation::from_build_env();
        __info__.instructions = Some(
            "LeRobot Dataset MCP Server. Provides tools for searching, inspecting, \
             and comparing LeRobot robotics datasets on the Hugging Face Hub."
                .into(),
        );
        __info__
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_creates_without_panic() {
        let client = HubClient::new(None).unwrap();
        let _server = LeRobotServer::new(client);
    }
}