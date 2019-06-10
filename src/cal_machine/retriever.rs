use hyper::client::{connect::HttpConnector,
                    ResponseFuture};
use hyper::rt::{self, Future, Stream};
use hyper::Body;
use hyper::Client;
use hyper::header::HeaderMap;
use hyper::header::HeaderValue;
use hyper::Request;
use hyper_tls::HttpsConnector;
use serde::{Deserialize,Serialize};
use serde_json::Result;
use std::io::Write;
use url::form_urlencoded;
//use std::io::{self, Write};

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
    client: Client<HttpsConnector<HttpConnector>, Body>,
}

impl EventRetriever {
    pub fn inst() -> EventRetriever {
        let https = HttpsConnector::new(1).expect("TLS initialization failed");
        let client = Client::builder().build::<_, hyper::Body>(https);

        EventRetriever { client }
    }

    pub fn retrieve_dev_and_code(&self) -> ResponseFuture {
        let post_args=vec![(String::from(CLIENT_ID_KEY), String::from(CLIENT_ID_VAL)),
                           (String::from(SCOPE_KEY), String::from(SCOPE_VAL))];
        let args_encoded=form_urlencoded::Serializer::new(String::new())
            .extend_pairs(post_args.into_iter()).finish();
        
        let mut builder = Request::post(DEVICE_CODE_URL);
        let headers:&mut HeaderMap<HeaderValue>=builder.headers_mut().expect("unable to access headers");
        headers.insert(hyper::header::CONTENT_TYPE, HeaderValue::from_static("application/x-www-form-urlencoded"));
        let req=builder.body(Body::from(args_encoded))
            .expect("request builder");
        
        self.client.request(req)
    }
}

#[derive(Serialize,Deserialize)]
pub struct DeviceUserCodeResponse {
    device_code: String,
    user_code: String,
    expires_in: u32,
    interval: u32,
    verification_url: String
}
