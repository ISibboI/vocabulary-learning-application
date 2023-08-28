use std::time::Duration;

use reqwest::{Client, ClientBuilder, Response, StatusCode};
use serde::Serialize;

static BASE_URL: &str = "http://localhost:8093";

pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub async fn new() -> Self {
        let client = ClientBuilder::new().cookie_store(true).build().unwrap();

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

        Self { client }
    }

    pub async fn post<T: Serialize>(&self, path: &str, body: T) -> Response {
        self.client
            .post(format!("{BASE_URL}{path}"))
            .json(&body)
            .send()
            .await
            .unwrap()
    }

    pub async fn post_empty(&self, path: &str) -> Response {
        self.client
            .post(format!("{BASE_URL}{path}"))
            .send()
            .await
            .unwrap()
    }
}

pub async fn assert_response_status(response: Response, status: StatusCode) {
    assert_eq!(
        response.status(),
        status,
        "unexpected response:\n{:?}\n",
        std::str::from_utf8(response.bytes().await.unwrap().as_ref()),
    );
}
