use std::time::Duration;

use reqwest::{Client, Response};
use serde::Serialize;

static BASE_URL: &str = "http://localhost:8093";

pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub async fn new() -> Self {
        let client = Client::default();

        for _ in 0..10 {
            match client.get(format!("{BASE_URL}")).send().await {
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

        Self { client }
    }

    pub async fn post<T: Serialize>(&self, path: &str, body: T) -> Response {
        self.client
            .post(format!("{BASE_URL}{path}"))
            .body(serde_json::to_string_pretty(&body).unwrap())
            .send()
            .await
            .unwrap()
    }
}
