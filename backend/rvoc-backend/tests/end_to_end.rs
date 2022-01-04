use reqwest::blocking::Client;
use reqwest::StatusCode;
use rvoc_backend::{ApiCommand, ApiResponseData};
use std::collections::hash_map::RandomState;
use std::collections::HashSet;
use std::str::FromStr;

static URL: &str = "http://localhost:2374/api/command";

fn expect_error(json: &str, error: &str) {
    let client = Client::new();
    let response = client.post(URL).json(json).send().unwrap();
    assert_eq!(response.status(), StatusCode::from_str(error).unwrap());
}

fn expect_ok(api_command: &ApiCommand) -> ApiResponseData {
    let client = Client::new();
    let response = client.post(URL).json(api_command).send().unwrap();
    assert_eq!(response.status(), StatusCode::from_str("200").unwrap());
    let response: ApiResponseData = response.json().unwrap();
    assert!(!response.is_error(), "Error:\n{:#?}", response);
    response
}

#[test]
fn test_empty_command() {
    expect_error("{}", "400");
}

#[test]
fn test_language_commands() {
    let languages = vec!["French", "German", "English"];
    for language in &languages {
        expect_ok(&ApiCommand::AddLanguage {
            name: language.to_string(),
        });
    }

    let list_languages = |limit| {
        if let ApiResponseData::ListLanguages(listed_languages) =
            expect_ok(&ApiCommand::ListLanguages { limit })
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
fn test_login() {}
