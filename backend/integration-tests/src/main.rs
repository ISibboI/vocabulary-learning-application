use std::process::ExitCode;

use api_commands::{CreateAccount, Login};
use log::info;
use reqwest::StatusCode;
use simplelog::TermLogger;
use tokio::spawn;

use crate::util::{assert_response_status, HttpClient};

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
async fn main() -> ExitCode {
    initialise_logging();

    let tasks = [
        spawn(test_user_account_creation()),
        spawn(test_duplicate_user_account_creation()),
        spawn(test_user_account_deletion()),
        spawn(test_login_logout()),
        spawn(test_wrong_password()),
        spawn(test_too_long_username()),
        spawn(test_too_long_password()),
        spawn(test_too_short_username()),
        spawn(test_too_short_password()),
        spawn(test_wrong_username_login()),
        spawn(test_too_long_password_login()),
    ];

    let mut has_error = false;
    for task in tasks {
        let result = task.await;
        has_error |= result.is_err();
        info!("{:?}", result);
    }

    if has_error {
        info!("Finished with errors");
        ExitCode::FAILURE
    } else {
        info!("Finished successfully");
        ExitCode::SUCCESS
    }
}

async fn test_user_account_creation() {
    let client = HttpClient::new().await;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                name: "anne".to_owned(),
                password: "frank😀😀😀".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::CREATED).await
}

async fn test_duplicate_user_account_creation() {
    let client = HttpClient::new().await;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                name: "rosa".to_owned(),
                password: "luxemburg".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::CREATED).await;

    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                name: "rosa".to_owned(),
                password: "luxemburg".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::CONFLICT).await;
}

async fn test_user_account_deletion() {
    let client = HttpClient::new().await;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                name: "claus".to_owned(),
                password: "von stauffenberg".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::CREATED).await;

    let response = client.post_empty("/accounts/delete").await;

    assert_response_status(response, StatusCode::UNAUTHORIZED).await;

    let response = client
        .post(
            "/accounts/login",
            Login {
                name: "claus".to_owned(),
                password: "von stauffenberg".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::NO_CONTENT).await;

    let response = client.post_empty("/accounts/delete").await;

    assert_response_status(response, StatusCode::NO_CONTENT).await;
}

async fn test_login_logout() {
    let client = HttpClient::new().await;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                name: "orli".to_owned(),
                password: "wald😀😀😀😀".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::CREATED).await;

    let response = client.post_empty("/accounts/logout").await;

    assert_response_status(response, StatusCode::UNAUTHORIZED).await;

    let response = client
        .post(
            "/accounts/login",
            Login {
                name: "orli".to_owned(),
                password: "wald😀😀😀😀".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::NO_CONTENT).await;

    let response = client.post_empty("/accounts/logout").await;

    assert_response_status(response, StatusCode::NO_CONTENT).await;

    let response = client.post_empty("/accounts/logout").await;

    assert_response_status(response, StatusCode::UNAUTHORIZED).await;
}

async fn test_wrong_password() {
    let client = HttpClient::new().await;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                name: "lothar".to_owned(),
                password: "kreyssig".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::CREATED).await;

    // wrong password
    let response = client
        .post(
            "/accounts/login",
            Login {
                name: "lothar".to_owned(),
                password: "anders".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::BAD_REQUEST).await;

    // correct password
    let response = client
        .post(
            "/accounts/login",
            Login {
                name: "lothar".to_owned(),
                password: "kreyssig".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::NO_CONTENT).await;

    // using a wrong password logs out
    let response = client
        .post(
            "/accounts/login",
            Login {
                name: "lothar".to_owned(),
                password: "anders".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::BAD_REQUEST).await;

    let response = client.post_empty("/accounts/logout").await;

    assert_response_status(response, StatusCode::UNAUTHORIZED).await;

    // does normal login-logout still work?
    let response = client
        .post(
            "/accounts/login",
            Login {
                name: "lothar".to_owned(),
                password: "kreyssig".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::NO_CONTENT).await;

    let response = client.post_empty("/accounts/logout").await;

    assert_response_status(response, StatusCode::NO_CONTENT).await;

    let response = client.post_empty("/accounts/logout").await;

    assert_response_status(response, StatusCode::UNAUTHORIZED).await;
}

async fn test_too_long_username() {
    let client = HttpClient::new().await;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                name: "else-else-else-else-else-else-else-else-else-else-else-else-else-else-else"
                    .to_owned(),
                password: "hirsch😀😀".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::BAD_REQUEST).await;
}

async fn test_too_long_password() {
    let client = HttpClient::new().await;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                name: "josef"
                    .to_owned(),
                password: "höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::BAD_REQUEST).await;
}

async fn test_too_short_username() {
    let client = HttpClient::new().await;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                name: "K.".to_owned(),
                password: "ibach😀😀😀".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::BAD_REQUEST).await;
}

async fn test_too_short_password() {
    let client = HttpClient::new().await;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                name: "hans".to_owned(),
                password: "ils".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::BAD_REQUEST).await;
}

async fn test_wrong_username_login() {
    let client = HttpClient::new().await;
    let response = client
        .post(
            "/accounts/login",
            Login {
                name: "peter".to_owned(),
                password: "hüppeler".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::BAD_REQUEST).await;
}

async fn test_too_long_password_login() {
    let client = HttpClient::new().await;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                name: "alois".to_owned(),
                password: "hundhammer".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::CREATED).await;

    let response = client
        .post(
            "/accounts/login",
            Login {
                name: "alois".to_owned(),
                password: "hundhammer-hundhammer-hundhammer-hundhammer-hundhammer-hundhammer-hundhammer-hundhammer-hundhammer-hundhammer-hundhammer-hundhammer".to_owned().into(),
            },
        )
        .await;

    assert_response_status(response, StatusCode::BAD_REQUEST).await;
}
