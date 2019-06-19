use reqwest::{self, Client, Response};
use serde::{Deserialize, Serialize};

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
const GRANT_TYPE_REFRESH_VAL: &str="refresh_token";
const REFRESH_TOKEN_KEY: &str = "refresh_token";

const TIME_MIN_KEY: &str = "timeMin";
const TIME_MAX_KEY: &str = "timeMax";
const MAX_RESULTS_KEY: &str = "maxResults";
const SINGLE_EVENTS_KEY: &str = "singleEvents";

const AUTHORISATION_HEADER: &str = "Authorization";
const ACCEPT_HEADER: &str = "Accept";
const ACCEPT_JSON: &str = "application/json";
pub const TOKEN_TYPE: &str = "Bearer";
//const USER_CODE_KEY: &str = "user_code";
//const EXPIRES_IN_KEY: &str = "expires_in";
//const INTERVAL_KEY: &str = "interval";
//const VERIFICATION_URL_KEY: &str = "verification_url";

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

    pub fn read(&self, bearer: &str) -> reqwest::Result<Response> {
        let args = [(MAX_RESULTS_KEY, "1"), (SINGLE_EVENTS_KEY, "true")];
        let request = self
            .client
            .get(READ_URL)
            .header(ACCEPT_HEADER, ACCEPT_JSON)
            .header(AUTHORISATION_HEADER, bearer)
            .query(&args);
        println!("cal read request: {:?}", request);
        request.send()
    }

    pub fn refresh(&self, refresh_token: &str) -> reqwest::Result<Response> {
        let post_args = [
            (CLIENT_ID_KEY, CLIENT_ID_VAL),
            (CLIENT_SECRET_KEY, CLIENT_SECRET_VAL),
            (REFRESH_TOKEN_KEY, refresh_token),
            (GRANT_TYPE_KEY, GRANT_TYPE_REFRESH_VAL)
        ];
        println!("refresh request args: {:?}", post_args);
        let request = self.client.post(AUTHORISATION_URL).form(&post_args);
        println!("refresh token request: {:?}", request);
        request.send()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceUserCodeResponse {
    pub device_code: String,
    user_code: String,
    expires_in: u32,
    pub interval: u32,
    verification_url: String,
}

pub const QUOTA_EXCEEDED_ERROR_CODE: &str = "rate_limit_exceeded";
#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceUserCodeErrorResponse {
    pub error_code: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PollResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u32,
    pub token_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RefreshResponse {
    pub access_token: String,
    pub expires_in: u32,
    pub token_type: String,
}

pub const ACCESS_DENIED_ERROR: &str = "access_denied";
pub const AUTHORISATION_PENDING_ERROR: &str = "authorization_pending";
pub const POLLING_TOO_FREQUENTLY_ERROR: &str = "slow_down";

#[derive(Serialize, Deserialize, Debug)]
pub struct PollErrorResponse {
    pub error: String,
    pub error_description: String,
}
