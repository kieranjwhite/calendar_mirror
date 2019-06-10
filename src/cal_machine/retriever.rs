use reqwest::{Client, Response, Result};
use serde::{Deserialize, Serialize};

const DEVICE_CODE_URL: &str = "https://accounts.google.com/o/oauth2/device/code";

const CLIENT_ID_KEY: &str = "client_id";
const CLIENT_ID_VAL: &str =
    "873648397769-eba22ohhel0t30e37dib506540vjdb25.apps.googleusercontent.com";

const SCOPE_KEY: &str = "scope";
const SCOPE_VAL: &str = "https://www.googleapis.com/auth/calendar.readonly";

const DEVICE_CODE_KEY: &str = "device_code";
const USER_CODE_KEY: &str = "user_code";
const EXPIRES_IN_KEY: &str = "expires_in";
const INTERVAL_KEY: &str = "interval";
const VERIFICATION_URL_KEY: &str = "verification_url";

pub struct EventRetriever {
    client: Client,
}

impl EventRetriever {
    pub fn inst() -> EventRetriever {
        let client = Client::new();

        EventRetriever { client }
    }

    pub fn retrieve_dev_and_code(&self) -> Result<Response> {
        let post_args = [(CLIENT_ID_KEY, CLIENT_ID_VAL), (SCOPE_KEY, SCOPE_VAL)];
        self.client.post(DEVICE_CODE_URL).form(&post_args).send()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviceUserCodeResponse {
    device_code: String,
    user_code: String,
    expires_in: u32,
    interval: u32,
    verification_url: String,
}
