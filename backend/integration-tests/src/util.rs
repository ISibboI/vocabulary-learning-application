use std::time::Duration;

use anyhow::bail;
use reqwest::{Client, ClientBuilder, Response, StatusCode};
use serde::Serialize;

static BASE_URL: &str = "http://localhost:8093";

pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub async fn new() -> anyhow::Result<Self> {
        let client = ClientBuilder::new().cookie_store(true).build()?;

        for _ in 0..10 {
            match client.get(BASE_URL).send().await {
                Ok(_) => break,
                Err(error) => {
                    if !error.is_connect() {
                        panic!("{error}");
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;

        Ok(Self { client })
    }

    pub async fn post<T: Serialize>(&self, path: &str, body: T) -> anyhow::Result<Response> {
        Ok(self
            .client
            .post(format!("{BASE_URL}{path}"))
            .json(&body)
            .send()
            .await?)
    }

    pub async fn post_empty(&self, path: &str) -> anyhow::Result<Response> {
        Ok(self.client.post(format!("{BASE_URL}{path}")).send().await?)
    }

    pub async fn delete(&self, path: &str) -> anyhow::Result<Response> {
        Ok(self
            .client
            .delete(format!("{BASE_URL}{path}"))
            .send()
            .await?)
    }
}

pub async fn assert_response_status(response: Response, status: StatusCode) -> anyhow::Result<()> {
    if response.status() != status {
        bail!(
            "unexpected response: {}\nexpected: {}\n{:?}\n",
            response.status(),
            status,
            std::str::from_utf8(response.bytes().await.unwrap().as_ref()),
        );
    } else {
        Ok(())
    }
}
