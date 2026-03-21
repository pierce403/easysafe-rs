use anyhow::{Context, Result};
use easysafe_core::{HeadRef, HealthResponse, WorkspaceConfig};
use reqwest::{Client, StatusCode};

#[derive(Clone)]
pub struct NodeClient {
    base_url: String,
    client: Client,
}

impl NodeClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: Client::new(),
        }
    }

    pub async fn health(&self) -> Result<HealthResponse> {
        let url = format!("{}/api/health", self.base_url);
        Ok(self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    pub async fn put_config(&self, config: &WorkspaceConfig) -> Result<()> {
        let url = format!(
            "{}/api/workspaces/{}/config",
            self.base_url, config.workspace_id
        );
        self.client
            .put(url)
            .json(config)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn get_config(&self, workspace_id: &str) -> Result<Option<WorkspaceConfig>> {
        let url = format!("{}/api/workspaces/{workspace_id}/config", self.base_url);
        let response = self.client.get(url).send().await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        Ok(Some(response.error_for_status()?.json().await?))
    }

    pub async fn put_head(&self, head_ref: &HeadRef) -> Result<()> {
        let url = format!(
            "{}/api/workspaces/{}/head",
            self.base_url, head_ref.workspace_id
        );
        self.client
            .put(url)
            .json(head_ref)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn get_head(&self, workspace_id: &str) -> Result<Option<HeadRef>> {
        let url = format!("{}/api/workspaces/{workspace_id}/head", self.base_url);
        let response = self.client.get(url).send().await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        Ok(Some(response.error_for_status()?.json().await?))
    }

    pub async fn put_object(&self, object_id: &str, bytes: &[u8]) -> Result<()> {
        let url = format!("{}/api/objects/{object_id}", self.base_url);
        self.client
            .put(url)
            .body(bytes.to_vec())
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn get_object(&self, object_id: &str) -> Result<Vec<u8>> {
        let url = format!("{}/api/objects/{object_id}", self.base_url);
        let response = self.client.get(url).send().await?;
        response
            .error_for_status_ref()
            .with_context(|| format!("failed to download object {object_id}"))?;
        Ok(response.bytes().await?.to_vec())
    }
}
