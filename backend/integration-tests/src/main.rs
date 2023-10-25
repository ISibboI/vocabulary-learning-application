use anyhow::{bail, Context};
use api_commands::{CreateAccount, Login, SecBytes};
use log::{debug, error, info};
use reqwest::StatusCode;
use simplelog::TermLogger;
use tokio::spawn;

use crate::util::{assert_response_status, HttpClient};

mod util;

fn initialise_logging() -> anyhow::Result<()> {
    TermLogger::init(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;

    info!("Logging initialised");
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    initialise_logging()?;

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
    let test_amount = tasks.len();

    let mut results = Vec::new();
    for task in tasks {
        let result = task.await;
        let Ok(result) = result else {
            error!("{result:?}");
            continue;
        };
        results.push(result);
    }

    let mut error_count = 0;
    for result in results {
        if result.is_err() {
            error_count += 1;
            error!("{:?}", result);
        } else {
            debug!("{:?}", result);
        }
    }

    if error_count > 0 {
        error!(
            "Finished with errors in {}/{} tests",
            error_count, test_amount
        );
        bail!("Finished with errors");
    } else {
        info!("Finished successfully");
        Ok(())
    }
}

async fn test_user_account_creation() -> anyhow::Result<()> {
    let client = HttpClient::new().await?;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                username: "anne".to_owned(),
                password: "frank😀😀😀".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::CREATED)
}

async fn test_duplicate_user_account_creation() -> anyhow::Result<()> {
    let client = HttpClient::new().await?;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                username: "rosa".to_owned(),
                password: "luxemburg".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::CREATED)?;

    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                username: "rosa".to_owned(),
                password: "luxemburg".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::CONFLICT)
}

async fn test_user_account_deletion() -> anyhow::Result<()> {
    let client = HttpClient::new().await?;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                username: "claus".to_owned(),
                password: "von stauffenberg".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::CREATED)?;

    let response = client.delete("/accounts/delete").await?;

    assert_response_status!(response, StatusCode::UNAUTHORIZED)?;

    let response = client
        .post(
            "/accounts/login",
            Login {
                username: "claus".to_owned(),
                password: "von stauffenberg".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::NO_CONTENT)?;

    let response = client.delete("/accounts/delete").await?;

    assert_response_status!(response, StatusCode::NO_CONTENT)
}

async fn test_login_logout() -> anyhow::Result<()> {
    let client = HttpClient::new().await?;
    let password = SecBytes::from("wald😀😀😀😀".to_owned());

    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                username: "orli".to_owned(),
                password: password.clone(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::CREATED)?;

    let response = client.post_empty("/accounts/logout").await?;

    assert_response_status!(response, StatusCode::UNAUTHORIZED)?;

    let response = client
        .post(
            "/accounts/login",
            Login {
                username: "orli".to_owned(),
                password: password.clone(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::NO_CONTENT)?;

    let response = client.post_empty("/accounts/logout").await?;

    assert_response_status!(response, StatusCode::NO_CONTENT)?;

    let response = client.post_empty("/accounts/logout").await?;

    assert_response_status!(response, StatusCode::UNAUTHORIZED)
}

async fn test_wrong_password() -> anyhow::Result<()> {
    let client = HttpClient::new().await?;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                username: "lothar".to_owned(),
                password: "kreyssig".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::CREATED)?;

    // wrong password
    let response = client
        .post(
            "/accounts/login",
            Login {
                username: "lothar".to_owned(),
                password: "anders++".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::BAD_REQUEST)?;

    // correct password
    let response = client
        .post(
            "/accounts/login",
            Login {
                username: "lothar".to_owned(),
                password: "kreyssig".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::NO_CONTENT)?;

    // using a wrong password logs out
    let response = client
        .post(
            "/accounts/login",
            Login {
                username: "lothar".to_owned(),
                password: "anders".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::BAD_REQUEST)?;

    let response = client.post_empty("/accounts/logout").await?;

    assert_response_status!(response, StatusCode::UNAUTHORIZED)?;

    // does normal login-logout still work?
    let response = client
        .post(
            "/accounts/login",
            Login {
                username: "lothar".to_owned(),
                password: "kreyssig".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::NO_CONTENT)?;

    let response = client.post_empty("/accounts/logout").await?;

    assert_response_status!(response, StatusCode::NO_CONTENT)?;

    let response = client.post_empty("/accounts/logout").await?;

    assert_response_status!(response, StatusCode::UNAUTHORIZED)
}

async fn test_too_long_username() -> anyhow::Result<()> {
    let client = HttpClient::new().await?;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                username:
                    "else-else-else-else-else-else-else-else-else-else-else-else-else-else-else"
                        .to_owned(),
                password: "hirsch😀😀".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::BAD_REQUEST)
}

async fn test_too_long_password() -> anyhow::Result<()> {
    let client = HttpClient::new().await?;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                username: "josef"
                    .to_owned(),
                password: "höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn-höhn".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::BAD_REQUEST)
}

async fn test_too_short_username() -> anyhow::Result<()> {
    let client = HttpClient::new().await?;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                username: "K.".to_owned(),
                password: "ibach😀😀😀".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::BAD_REQUEST)
}

async fn test_too_short_password() -> anyhow::Result<()> {
    let client = HttpClient::new().await?;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                username: "hans".to_owned(),
                password: "ils".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::BAD_REQUEST)
}

async fn test_wrong_username_login() -> anyhow::Result<()> {
    let client = HttpClient::new().await?;
    let response = client
        .post(
            "/accounts/login",
            Login {
                username: "peter".to_owned(),
                password: "hüppeler".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::BAD_REQUEST)
}

async fn test_too_long_password_login() -> anyhow::Result<()> {
    let client = HttpClient::new().await?;
    let response = client
        .post(
            "/accounts/create",
            CreateAccount {
                username: "alois".to_owned(),
                password: "hundhammer".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::CREATED)?;

    let response = client
        .post(
            "/accounts/login",
            Login {
                username: "alois".to_owned(),
                password: "hundhammer-hundhammer-hundhammer-hundhammer-hundhammer-hundhammer-hundhammer-hundhammer-hundhammer-hundhammer-hundhammer-hundhammer".to_owned().into(),
            },
        )
        .await?;

    assert_response_status!(response, StatusCode::BAD_REQUEST)
}
