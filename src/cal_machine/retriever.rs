/*
Copyright [2019] [Kieran White]

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

use chrono::prelude::*;
use reqwest::{self, Client, Response};
use serde::Deserialize;

const DEVICE_CODE_URL: &str = "https://accounts.google.com/o/oauth2/device/code";
const AUTHORISATION_URL: &str = "https://www.googleapis.com/oauth2/v4/token";
const READ_URL: &str = "https://www.googleapis.com/calendar/v3/calendars/primary/events";
const CLIENT_ID_KEY: &str = "client_id";
const CLIENT_ID_VAL: &str =
    "873648397769-eba22ohhel0t30e37dib506540vjdb25.apps.googleusercontent.com";
const SCOPE_KEY: &str = "scope";
const SCOPE_VAL: &str = "https://www.googleapis.com/auth/calendar.readonly";
const CLIENT_SECRET_KEY: &str = "client_secret";
const CLIENT_SECRET_VAL: &str = "miaNRI8ECPVbiAaKWgiw6a3S";
const CODE_KEY: &str = "code";
const GRANT_TYPE_KEY: &str = "grant_type";
const GRANT_TYPE_POLL_VAL: &str = "http://oauth.net/grant_type/device/1.0";
const GRANT_TYPE_REFRESH_VAL: &str = "refresh_token";
const REFRESH_TOKEN_KEY: &str = "refresh_token";
const PAGE_TOKEN_KEY: &str = "pageToken";

const TIME_MIN_KEY: &str = "timeMin";
const TIME_MAX_KEY: &str = "timeMax";
const MAX_RESULTS_KEY: &str = "maxResults";
const SINGLE_EVENTS_KEY: &str = "singleEvents";

const AUTHORISATION_HEADER: &str = "Authorization";
const ACCEPT_HEADER: &str = "Accept";
const ACCEPT_JSON: &str = "application/json";
pub const TOKEN_TYPE: &str = "Bearer";

pub const ACCESS_DENIED_ERROR: &str = "access_denied";
pub const AUTHORISATION_PENDING_ERROR: &str = "authorization_pending";
pub const POLLING_TOO_FREQUENTLY_ERROR: &str = "slow_down";

#[derive(Debug)]
pub struct PageToken(pub String);

pub struct EventRetriever {
    client: Client,
}

impl EventRetriever {
    pub fn inst() -> EventRetriever {
        let client = Client::new();

        EventRetriever { client }
    }

    pub fn retrieve_dev_and_code(&self) -> reqwest::Result<Response> {
        let post_args = [(CLIENT_ID_KEY, CLIENT_ID_VAL), (SCOPE_KEY, SCOPE_VAL)];
        println!("device code args: {:?}", post_args);
        let request = self.client.post(DEVICE_CODE_URL).form(&post_args);
        println!("device code request: {:?}", request);
        request.send()
    }

    pub fn poll(&self, code: &str) -> reqwest::Result<Response> {
        let post_args = [
            (CLIENT_ID_KEY, CLIENT_ID_VAL),
            (CLIENT_SECRET_KEY, CLIENT_SECRET_VAL),
            (CODE_KEY, code),
            (GRANT_TYPE_KEY, GRANT_TYPE_POLL_VAL),
        ];
        self.client.post(AUTHORISATION_URL).form(&post_args).send()
    }

    pub fn read(
        &self,
        bearer: &str,
        min_time: &DateTime<Local>,
        max_time: &DateTime<Local>,
        page_token: &Option<PageToken>,
    ) -> reqwest::Result<Response> {
        let min = &min_time.format("%+").to_string().clone();
        let max = &max_time.format("%+").to_string().clone();
        let request = self
            .client
            .get(READ_URL)
            .header(ACCEPT_HEADER, ACCEPT_JSON)
            .header(AUTHORISATION_HEADER, bearer);
        let request = match page_token {
            None => request.query(&[
                (TIME_MIN_KEY, min),
                (TIME_MAX_KEY, max),
                (MAX_RESULTS_KEY, &String::from("1")),
                (SINGLE_EVENTS_KEY, &String::from("true")),
            ]),
            Some(PageToken(token)) => request.query(&[
                (TIME_MIN_KEY, min),
                (TIME_MAX_KEY, max),
                (MAX_RESULTS_KEY, &String::from("1")),
                (SINGLE_EVENTS_KEY, &String::from("true")),
                (PAGE_TOKEN_KEY, token),
            ]),
        };
        println!("cal read request: {:?}", request);
        request.send()
    }

    pub fn refresh(&self, refresh_token: &str) -> reqwest::Result<Response> {
        let post_args = [
            (CLIENT_ID_KEY, CLIENT_ID_VAL),
            (CLIENT_SECRET_KEY, CLIENT_SECRET_VAL),
            (REFRESH_TOKEN_KEY, refresh_token),
            (GRANT_TYPE_KEY, GRANT_TYPE_REFRESH_VAL),
        ];
        println!("refresh request args: {:?}", post_args);
        let request = self.client.post(AUTHORISATION_URL).form(&post_args);
        println!("refresh token request: {:?}", request);
        request.send()
    }
}

#[derive(Deserialize, Debug)]
pub struct DeviceUserCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub expires_in: i64,
    pub interval: u32,
    pub verification_url: String,
}

pub const QUOTA_EXCEEDED_ERROR_CODE: &str = "rate_limit_exceeded";
#[derive(Deserialize, Debug)]
pub struct DeviceUserCodeErrorResponse {
    pub error_code: String,
}

#[derive(Deserialize, Debug)]
pub struct PollResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
}

#[derive(Deserialize, Debug)]
pub struct RefreshResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub token_type: String,
}

#[derive(Deserialize, Debug)]
pub struct PollErrorResponse {
    pub error: String,
    pub error_description: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DateTimeStamp {
    //#[serde(rename = "dateTime")]
    pub date_time: Option<String>, //most events provide date and time
    pub date: Option<String> //but all-day events only provide the date
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
pub struct PersonalIdentifier {
    pub email: String,
}

#[derive(Deserialize, Debug)]
pub struct Event {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub creator: PersonalIdentifier,
    pub start: DateTimeStamp,
    pub end: DateTimeStamp,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventsResponse {
    pub next_page_token: Option<String>,
    pub items: Vec<Event>,
}
