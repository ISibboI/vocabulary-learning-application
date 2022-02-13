use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::{IntoUrl, StatusCode};
use rvoc_backend::{ApiCommand, ApiResponseData, LoginCommand, LogoutCommand, SignupCommand};
use serde::Serialize;
use std::collections::hash_map::RandomState;
use std::collections::HashSet;
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

static URL: &str = "http://localhost:2374/api/command";
static LOGIN_URL: &str = "http://localhost:2374/api/login";
static LOGOUT_URL: &str = "http://localhost:2374/api/logout";
static SIGNUP_URL: &str = "http://localhost:2374/api/signup";

struct ClientWithCookies {
    client: Client,
    cookie: Option<String>,
}

impl Default for ClientWithCookies {
    fn default() -> Self {
        Self {
            client: Client::new(),
            cookie: None,
        }
    }
}

impl ClientWithCookies {
    fn post<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        let builder = self.client.post(url);
        if let Some(cookie) = &self.cookie {
            builder.header("Cookie", cookie)
        } else {
            builder
        }
    }

    fn set_cookie(&mut self, response: &Response) {
        assert_eq!(response.cookies().count(), 1); // This method is only designed for a single cookie.
        let cookie = response.cookies().next().unwrap();
        assert!(cookie.secure());
        assert!(cookie.http_only());
        assert!(cookie.same_site_strict());
        self.cookie = Some(format!("{}={}", cookie.name(), cookie.value()));
    }
}

fn expect_error_from_str(client: &ClientWithCookies, json: &str, http_status_code: u16) {
    let response = client.post(URL).json(json).send().unwrap();
    assert_eq!(
        response.status(),
        StatusCode::from_u16(http_status_code).unwrap()
    );
}

fn expect_error(client: &ClientWithCookies, api_command: &ApiCommand, http_status_code: u16) {
    let response = client.post(URL).json(api_command).send().unwrap();
    assert_eq!(
        response.status(),
        StatusCode::from_u16(http_status_code).unwrap()
    );
}

fn expect_ok_with_url<Command: Serialize>(
    client: &mut ClientWithCookies,
    command: &Command,
    url: &str,
) -> ApiResponseData {
    let response = client.post(url).json(command).send().unwrap();
    assert_eq!(
        response.status(),
        StatusCode::from_str("200").unwrap(),
        "Error!\n{:#?}",
        response
    );
    // Set cookies only if cookies exist.
    if response.cookies().next().is_some() {
        client.set_cookie(&response);
    }
    let response: ApiResponseData = response.json().unwrap();
    assert!(!response.is_error(), "Error:\n{:#?}", response);
    response
}

fn expect_ok(client: &mut ClientWithCookies, api_command: &ApiCommand) -> ApiResponseData {
    expect_ok_with_url(client, api_command, URL)
}

fn expect_login_ok(
    client: &mut ClientWithCookies,
    login_command: &LoginCommand,
) -> ApiResponseData {
    expect_ok_with_url(client, login_command, LOGIN_URL)
}

fn expect_logout_ok(
    client: &mut ClientWithCookies,
    logout_command: &LogoutCommand,
) -> ApiResponseData {
    expect_ok_with_url(client, logout_command, LOGOUT_URL)
}

fn expect_signup_ok(
    client: &mut ClientWithCookies,
    signup_command: &SignupCommand,
) -> ApiResponseData {
    expect_ok_with_url(client, signup_command, SIGNUP_URL)
}

/// Sign up and log in a user with the given name, and return a [Client](reqwest::blocking::Client) with the session cookie set.
/// Email will be <login_name>@test.com.
/// Password will be the same as the login name.
fn signup_and_login(login_name: &str) -> ClientWithCookies {
    let mut client = ClientWithCookies::default();
    let response = expect_signup_ok(
        &mut client,
        &SignupCommand {
            login_name: login_name.to_string(),
            password: login_name.to_string(),
            email: format!("{}@test.com", login_name),
        },
    );
    assert_eq!(response, ApiResponseData::Ok);

    let response = expect_login_ok(
        &mut client,
        &LoginCommand {
            login_name: login_name.to_string(),
            password: login_name.to_string(),
        },
    );
    assert_eq!(response, ApiResponseData::Ok);
    client
}

#[test]
fn test_empty_command() {
    let client = signup_and_login("test_empty_command");
    expect_error_from_str(&client, "{}", 500);
}

#[test]
fn test_language_commands() {
    let mut client = signup_and_login("test_language_commands");
    let languages = vec!["French", "German", "English"];
    for language in &languages {
        expect_ok(
            &mut client,
            &ApiCommand::AddLanguage {
                name: language.to_string(),
            },
        );
    }

    let mut list_languages = |limit| {
        if let ApiResponseData::ListLanguages(listed_languages) =
            expect_ok(&mut client, &ApiCommand::ListLanguages { limit })
        {
            let listed_languages: HashSet<_, RandomState> =
                HashSet::from_iter(listed_languages.into_iter().map(|l| l.name));
            assert_eq!(listed_languages.len(), limit);

            for listed_language in &listed_languages {
                assert!(languages.contains(&listed_language.as_str()));
            }
        } else {
            panic!();
        }
    };

    for limit in 1..=3 {
        list_languages(limit);
    }
}

#[test]
fn test_signup_and_login() {
    let mut client = signup_and_login("test_signup_and_login");
    let response = expect_ok(&mut client, &ApiCommand::IsLoggedIn);
    assert_eq!(response, ApiResponseData::Ok);
}

#[test]
fn test_session_expiry() {
    let client = signup_and_login("test_session_expiry");
    // Wait until session is expired
    // (make sure that the session cookie is set to expire after 15 seconds in the test instance of the backend)
    sleep(Duration::from_secs(20));
    expect_error(&client, &ApiCommand::IsLoggedIn, 403);
}

/// Check if a logged in user is logged out if someone else tries to brute-force their password at the same time.
#[test]
fn test_user_not_kicked_out_on_wrong_login() {
    let login_name = "test_user_not_kicked_out_on_wrong_login";
    let mut client = signup_and_login(login_name);

    // Wrong login.
    let response = ClientWithCookies::default()
        .post(LOGIN_URL)
        .json(&LoginCommand {
            login_name: login_name.to_string(),
            password: "abc".to_string(),
        })
        .send()
        .unwrap();
    assert_eq!(response.status(), StatusCode::from_u16(200).unwrap());
    match response.json().unwrap() {
        ApiResponseData::Error(error) => {
            println!("{error}");
        }
        other => panic!("Expected error, but got {other:?}"),
    }

    // Check if user is still logged in.
    expect_ok(&mut client, &ApiCommand::IsLoggedIn);
}

/// Check if a logged in user is logged out if someone else tries to create an account with their name at the same time.
#[test]
fn test_user_not_kicked_out_on_duplicate_signup() {
    let login_name = "test_user_not_kicked_out_on_duplicate_signup";
    let mut client = signup_and_login(login_name);

    // Wrong signup.
    let response = ClientWithCookies::default()
        .post(SIGNUP_URL)
        .json(&SignupCommand {
            login_name: login_name.to_string(),
            password: login_name.to_string(),
            email: format!("{}@test.com", login_name),
        })
        .send()
        .unwrap();
    assert_eq!(response.status(), StatusCode::from_u16(200).unwrap());
    match response.json().unwrap() {
        ApiResponseData::Error(error) => {
            println!("{error}");
        }
        other => panic!("Expected error, but got {other:?}"),
    }

    // Check if user is still logged in.
    expect_ok(&mut client, &ApiCommand::IsLoggedIn);
}

/// Test if two different clients can have a session at the same time.
#[test]
fn test_parallel_session() {
    let session_count = 10;

    let login_names: Vec<String> = (0..session_count)
        .map(|i| format!("test_parallel_session_{i}"))
        .collect();
    let mut clients: Vec<_> = login_names
        .iter()
        .map(String::as_str)
        .map(signup_and_login)
        .collect();

    let mut cookie_set = HashSet::new();
    for client in &mut clients {
        let response = expect_ok(client, &ApiCommand::IsLoggedIn);
        assert_eq!(response, ApiResponseData::Ok);
        assert!(cookie_set.insert(client.cookie.clone().unwrap())); // Assert that each session id is unique.
    }
}

/// Check if logging out sessions one by one works.
#[test]
fn test_single_logout() {
    let session_count = 10;

    let login_names: Vec<String> = (0..session_count)
        .map(|i| format!("test_single_logout_{i}"))
        .collect();
    let mut clients: Vec<_> = login_names
        .iter()
        .map(String::as_str)
        .map(signup_and_login)
        .collect();

    for client in &mut clients {
        let response = expect_ok(client, &ApiCommand::IsLoggedIn);
        assert_eq!(response, ApiResponseData::Ok);
    }

    let mut clients = clients.as_mut_slice();
    while !clients.is_empty() {
        let response = expect_logout_ok(&mut clients[0], &LogoutCommand::ThisSession);
        assert_eq!(response, ApiResponseData::Ok);

        expect_error(&mut clients[0], &ApiCommand::IsLoggedIn, 403);

        clients = &mut clients[1..];
        for client in clients.iter_mut() {
            assert_eq!(
                expect_ok(client, &ApiCommand::IsLoggedIn),
                ApiResponseData::Ok
            );
        }
    }
}
