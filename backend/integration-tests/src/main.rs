use api_commands::CreateAccount;
use log::info;
use reqwest::StatusCode;
use simplelog::TermLogger;

use crate::util::HttpClient;

mod util;

fn initialise_logging() {
    TermLogger::new(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    );

    info!("Logging initialised");
}

#[tokio::main]
async fn main() {
    initialise_logging();
    let client = HttpClient::new().await;

    test_user_account_creation(&client).await;

    info!("Finished");
}

async fn test_user_account_creation(client: &HttpClient) {
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                name: "anne".to_owned(),
                password: "frank".to_owned().into(),
            },
        )
        .await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "unexpected response: {response:?}"
    );
}
