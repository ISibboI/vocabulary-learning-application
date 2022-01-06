use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::{IntoUrl, StatusCode};
use rvoc_backend::{ApiCommand, ApiResponseData, LoginCommand, SignupCommand};
use std::collections::hash_map::RandomState;
use std::collections::HashSet;
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

static URL: &str = "http://localhost:2374/api/command";
static LOGIN_URL: &str = "http://localhost:2374/api/login";
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
        let cookie = response.cookies().next().unwrap();
        assert!(cookie.secure());
        assert!(cookie.http_only());
        assert!(cookie.same_site_strict());
        self.cookie = Some(format!("{}={}", cookie.name(), cookie.value()));
    }
}

/// Sign up and log in a user with the given name, and return a [Client](reqwest::blocking::Client) with the session cookie set.
/// Email will be <login_name>@test.com.
/// Password will be the same as the login name.
fn signup_and_login(login_name: &str) -> ClientWithCookies {
    let mut client = ClientWithCookies::default();
    let response = client
        .post(SIGNUP_URL)
        .json(&SignupCommand {
            login_name: login_name.to_string(),
            password: login_name.to_string(),
            email: format!("{}@test.com", login_name),
        })
        .send()
        .unwrap();
    assert_eq!(response.status(), StatusCode::from_u16(200).unwrap());
    assert_eq!(
        response.json::<ApiResponseData>().unwrap(),
        ApiResponseData::Ok
    );

    let response = client
        .post(LOGIN_URL)
        .json(&LoginCommand {
            login_name: login_name.to_string(),
            password: login_name.to_string(),
        })
        .send()
        .unwrap();
    client.set_cookie(&response);
    assert_eq!(response.status(), StatusCode::from_u16(200).unwrap());
    assert_eq!(
        response.json::<ApiResponseData>().unwrap(),
        ApiResponseData::Ok
    );
    client
}

fn expect_error(client: &ClientWithCookies, json: &str, error: &str) {
    let response = client.post(URL).json(json).send().unwrap();
    assert_eq!(response.status(), StatusCode::from_str(error).unwrap());
}

fn expect_ok(client: &ClientWithCookies, api_command: &ApiCommand) -> ApiResponseData {
    let response = client.post(URL).json(api_command).send().unwrap();
    assert_eq!(
        response.status(),
        StatusCode::from_str("200").unwrap(),
        "Error!\n{:#?}",
        response
    );
    let response: ApiResponseData = response.json().unwrap();
    assert!(!response.is_error(), "Error:\n{:#?}", response);
    response
}

#[test]
fn test_empty_command() {
    let client = signup_and_login("test_empty_command");
    expect_error(&client, "{}", "400");
}

#[test]
fn test_language_commands() {
    let client = signup_and_login("test_language_commands");
    let languages = vec!["French", "German", "English"];
    for language in &languages {
        expect_ok(
            &client,
            &ApiCommand::AddLanguage {
                name: language.to_string(),
            },
        );
    }

    let list_languages = |limit| {
        if let ApiResponseData::ListLanguages(listed_languages) =
            expect_ok(&client, &ApiCommand::ListLanguages { limit })
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
    let client = signup_and_login("test_signup_and_login");
    let response = expect_ok(&client, &ApiCommand::IsLoggedIn);
    assert_eq!(response, ApiResponseData::Ok);
}

#[test]
fn test_session_expiry() {
    let client = signup_and_login("test_session_expiry");
    // Wait until session is expired
    // (make sure that the session cookie is set to expire after 15 seconds in the test instance of the backend)
    sleep(Duration::from_secs(20));
    expect_error(
        &client,
        &serde_json::to_string(&ApiCommand::IsLoggedIn).unwrap(),
        "403",
    );
}
