use reqwest::{Client, Response};
use serde::Serialize;

static BASE_URL: &str = "http://localhost:8093";

pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: Client::default(),
        }
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
