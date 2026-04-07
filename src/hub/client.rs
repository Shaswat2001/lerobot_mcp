//! Async HTTP client for the Hugging Face Hub REST API and datasets-server API.
//!
//! This is the core I/O layer. Every method here makes one HTTP call and
//! deserializes the response into a typed struct from `hub::types`.
//!
//! No caching here -- that's handled by `CachedClient` in a later phase.

use crate::error::{Error, Result};
use crate::hub::types::*;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};

#[derive(Debug, Clone)]
pub struct HubClient {
    
}