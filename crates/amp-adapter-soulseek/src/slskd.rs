//! A client for slskd's REST API (`/api/v0`, `X-API-Key`). slskd is a separate Soulseek client;
//! multiplay never speaks the Soulseek protocol itself (product decision, TASK-162).

#![forbid(unsafe_code)]

use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

/// What can go wrong talking to slskd.
#[derive(Debug, thiserror::Error)]
pub enum SlskdError {
    #[error("slskd is not running at this address ({0})")]
    NotRunning(String),
    #[error("slskd refused the API key")]
    Unauthorized,
    #[error("slskd answered with an error: {0}")]
    Api(String),
}

/// One file in a search response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchFile {
    pub filename: String,
    #[serde(default)]
    pub size: u64,
    pub bit_rate: Option<u32>,
    /// Seconds.
    pub length: Option<u32>,
    #[serde(default)]
    pub extension: Option<String>,
}

/// One peer's answer to a search.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub username: String,
    #[serde(default)]
    pub has_free_upload_slot: bool,
    #[serde(default)]
    pub upload_speed: u64,
    #[serde(default)]
    pub queue_length: u64,
    #[serde(default)]
    pub files: Vec<SearchFile>,
}

#[derive(Debug, Deserialize)]
struct SearchState {
    #[serde(default)]
    state: String,
    #[serde(default)]
    responses: Vec<SearchResponse>,
}

/// A download as slskd reports it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    pub filename: String,
    /// Flags as text, e.g. `"InProgress"`, `"Completed, Succeeded"`, `"Completed, Errored"`.
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub percent_complete: f64,
    #[serde(default)]
    pub exception: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserTransfers {
    #[serde(default)]
    directories: Vec<TransferDirectory>,
}

#[derive(Debug, Deserialize)]
struct TransferDirectory {
    #[serde(default)]
    files: Vec<Transfer>,
}

/// A connection to one slskd.
#[derive(Debug, Clone)]
pub struct SlskdClient {
    base: String,
    api_key: String,
    http: reqwest::Client,
}

impl SlskdClient {
    /// `base` like `http://127.0.0.1:5030`.
    ///
    /// # Errors
    /// When the HTTP client cannot be built.
    pub fn new(base: &str, api_key: &str) -> Result<Self, SlskdError> {
        amp_adapter_http::install_ring_provider();
        let http = reqwest::Client::builder()
            .user_agent(amp_domain::about::user_agent())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| SlskdError::Api(e.to_string()))?;
        Ok(Self {
            base: base.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            http,
        })
    }

    /// The slskd address.
    #[must_use]
    pub fn base(&self) -> &str {
        &self.base
    }

    async fn send(&self, req: reqwest::RequestBuilder) -> Result<reqwest::Response, SlskdError> {
        let resp = req
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| SlskdError::NotRunning(format!("{}: {e}", self.base)))?;
        match resp.status().as_u16() {
            401 | 403 => Err(SlskdError::Unauthorized),
            s if s >= 400 => Err(SlskdError::Api(format!(
                "{s}: {}",
                resp.text().await.unwrap_or_default()
            ))),
            _ => Ok(resp),
        }
    }

    async fn json<T: serde::de::DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<T, SlskdError> {
        self.send(req)
            .await?
            .json()
            .await
            .map_err(|e| SlskdError::Api(e.to_string()))
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/v0/{path}", self.base)
    }

    /// slskd's version; the cheapest "is it there and does the key work" check.
    ///
    /// # Errors
    /// Not running, wrong key, or an unexpected answer.
    pub async fn version(&self) -> Result<String, SlskdError> {
        self.json(self.http.get(self.url("application/version")))
            .await
    }

    /// The folder completed downloads are moved to.
    ///
    /// # Errors
    /// As [`Self::version`].
    pub async fn downloads_dir(&self) -> Result<PathBuf, SlskdError> {
        let options: serde_json::Value = self.json(self.http.get(self.url("options"))).await?;
        options["directories"]["downloads"]
            .as_str()
            .map(PathBuf::from)
            .ok_or_else(|| SlskdError::Api("no downloads directory in slskd's options".into()))
    }

    /// Searches the Soulseek network and waits up to `wait` for answers.
    ///
    /// # Errors
    /// As [`Self::version`].
    pub async fn search(
        &self,
        text: &str,
        wait: Duration,
    ) -> Result<Vec<SearchResponse>, SlskdError> {
        let id = uuid::Uuid::new_v4().to_string();
        let _: serde_json::Value = self
            .json(self.http.post(self.url("searches")).json(&json!({
                "id": id,
                "searchText": text,
                "searchTimeout": wait.as_millis().min(30_000) as u64,
                "filterResponses": true,
            })))
            .await?;
        let deadline = tokio::time::Instant::now() + wait;
        loop {
            let state: SearchState = self
                .json(
                    self.http
                        .get(self.url(&format!("searches/{id}?includeResponses=true"))),
                )
                .await?;
            if state.state.contains("Completed") || tokio::time::Instant::now() >= deadline {
                return Ok(state.responses);
            }
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
    }

    /// Asks slskd to download `filename` from `username` into `destination` (relative to its
    /// downloads folder).
    ///
    /// # Errors
    /// As [`Self::version`], or when slskd lists the file as failed.
    pub async fn download(
        &self,
        username: &str,
        filename: &str,
        size: u64,
        destination: &str,
    ) -> Result<(), SlskdError> {
        let answer: serde_json::Value = self
            .json(
                self.http
                    .post(self.url("transfers/downloads/batches"))
                    .json(&json!({
                        "username": username,
                        "files": [ { "filename": filename, "size": size } ],
                        "options": { "destination": destination },
                    })),
            )
            .await?;
        match answer["failures"].as_array().and_then(|f| f.first()) {
            Some(failure) => Err(SlskdError::Api(
                failure["message"]
                    .as_str()
                    .unwrap_or("download refused")
                    .to_string(),
            )),
            None => Ok(()),
        }
    }

    /// The current state of the download of `filename` from `username`.
    ///
    /// # Errors
    /// As [`Self::version`].
    pub async fn transfer(
        &self,
        username: &str,
        filename: &str,
    ) -> Result<Option<Transfer>, SlskdError> {
        let user: UserTransfers = self
            .json(
                self.http
                    .get(self.url(&format!("transfers/downloads/{}", urlencode(username)))),
            )
            .await?;
        Ok(user
            .directories
            .into_iter()
            .flat_map(|d| d.files)
            .find(|t| t.filename == filename))
    }
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
