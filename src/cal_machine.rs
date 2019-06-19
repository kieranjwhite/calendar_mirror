mod retriever;

use crate::stm;
use reqwest::{Response, StatusCode};
use retriever::*;
use std::{io, path::Path, thread, time::Duration};

stm!(cal_stm, Load, {
    [ReadFirst, Wipe], RequestCodes;
    [Load, Wait], Refresh;
    [Refresh, Save], ReadFirst;
    [RequestCodes], Poll;
    [Load, Poll, ReadFirst, RequestCodes, Save], DisplayError;
    [Poll], Save;
    [ReadFirst], Page;
    [Page], Display;
    [DisplayError, Display], Wait;
    [Wait], Wipe
});

const MISSING_CREDENTIALS: &str="Missing credentials";
const LOAD_FAILED: &str = "Failed to load credentials";
const QUOTA_EXCEEDED: &str = "Quota Exceeded";
const ACCESS_DENIED: &str = "User has refused to grant access to this calendar";
const PENDING_REQUEST: &str = "The user has not yet granted you access to this calender";
const SLOW_DOWN: &str = "This device is checking for access too frequently";
const UNRECOGNISED_TOKEN_TYPE: &str = "Unrecognised token type";

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    IO(io::Error),
}

impl From<reqwest::Error> for Error {
    fn from(orig: reqwest::Error) -> Error {
        Error::Reqwest(orig)
    }
}

impl From<io::Error> for Error {
    fn from(orig: io::Error) -> Error {
        Error::IO(orig)
    }
}

pub fn run() -> Result<(), Error> {
    use cal_stm::Machine;
    use cal_stm::Machine::*;

    let config_file = Path::new("/home/kieran/projects/rust/calendar_mirror/config.json");
    let retriever = EventRetriever::inst();
    let mut mach: Machine = Machine::new_stm();
    let mut error: Option<String> = None;
    let mut device_code = String::new();
    let mut delay_s: u64 = 1;
    let mut credentials = None;
    loop {
        mach = match mach {
            Load(st) => match PollResponse::load(config_file) {
                Err(error_msg) => {
                    error = Some(format!("{}: {}", LOAD_FAILED, error_msg.to_string()));
                    DisplayError(st.into())
                }
                Ok(creds) => {
                    credentials = creds;
                    Refresh(st.into())
                }
            },
            RequestCodes(st) => {
                let mut resp: Response = retriever.retrieve_dev_and_code()?;
                let status = resp.status();
                match status {
                    StatusCode::OK => {
                        println!("Headers: {:#?}", resp.headers());
                        let body: DeviceUserCodeResponse = resp.json()?;
                        println!("Body is next... {:?}", body);

                        device_code = String::from(body.device_code);
                        delay_s = body.interval as u64;
                        Poll(st.into())
                    }
                    other_status => {
                        let body: DeviceUserCodeErrorResponse = resp.json()?;
                        eprintln!("Error when getting request code: {:?}", body);
                        match other_status {
                            StatusCode::FORBIDDEN
                                if body.error_code == QUOTA_EXCEEDED_ERROR_CODE =>
                            {
                                error = Some(QUOTA_EXCEEDED.to_string());
                                DisplayError(st.into())
                            }
                            _otherwise => {
                                error = Some(format!(
                                    "HTTP error: {}, {}",
                                    other_status.as_u16(),
                                    body.error_code
                                ));
                                DisplayError(st.into())
                            }
                        }
                    }
                }
            }
            Refresh(st) => ReadFirst(st.into()),
            Poll(st) => {
                thread::sleep(Duration::from_secs(delay_s));
                let mut resp: Response = retriever.poll(&device_code)?;
                let status = resp.status();
                match status {
                    StatusCode::OK => {
                        println!("Headers: {:#?}", resp.headers());
                        let credentials_tokens: PollResponse=resp.json()?;
                        credentials=Some(credentials_tokens);
                        Save(st.into())
                    }
                    other_status => {
                        let body: PollErrorResponse = resp.json()?;
                        eprintln!("Error when polling: {:?}", body);
                        match other_status {
                            StatusCode::FORBIDDEN if body.error == ACCESS_DENIED_ERROR => {
                                error = Some(ACCESS_DENIED.to_string());
                                DisplayError(st.into())
                            }
                            StatusCode::BAD_REQUEST
                                if body.error == AUTHORISATION_PENDING_ERROR =>
                            {
                                Poll(st)
                            }
                            StatusCode::PRECONDITION_REQUIRED
                                if body.error == AUTHORISATION_PENDING_ERROR =>
                            {
                                Poll(st)
                            }
                            StatusCode::TOO_MANY_REQUESTS
                                if body.error == POLLING_TOO_FREQUENTLY_ERROR =>
                            {
                                delay_s *= 2;
                                Poll(st)
                            }
                            _otherwise => {
                                error = Some(format!(
                                    "HTTP error: {}, {}, {}",
                                    other_status.as_u16(),
                                    body.error,
                                    body.error_description
                                ));
                                DisplayError(st.into())
                            }
                        }
                    }
                }
            }
            Save(st) => match credentials {
                None => {
                    error = Some(MISSING_CREDENTIALS.to_string());
                    DisplayError(st.into())
                },
                Some(ref credential_tokens) => {
                    let token_type = credential_tokens.token_type.clone();
                    if token_type != TOKEN_TYPE {
                        error = Some(format!("{}: {}", UNRECOGNISED_TOKEN_TYPE, token_type));
                        DisplayError(st.into())
                    } else {
                        println!("Body is next... {:?}", credential_tokens);
                        credential_tokens.save(config_file)?;
                        ReadFirst(st.into())
                    }
                }
            },
            ReadFirst(st) => {
                match credentials {
                    None => RequestCodes(st.into()),
                    Some(ref credentials_tokens) => {
                        let mut resp: Response =
                            retriever.read(&format!("Bearer {}", credentials_tokens.access_token))?;
                        let status = resp.status();
                        match status {
                            StatusCode::OK => {
                                println!("Event Headers: {:#?}", resp.headers());
                                //let body: ReadResponse = resp.json()?;
                                println!("Event is next... {:?}", resp.text());
                                Page(st.into())
                            }
                            _other_status => {
                                println!("Event Headers: {:#?}", resp.headers());
                                println!("Event is next... {:?}", resp.text());
                                error = Some(format!("in readfirst. http status: {:?}", status));
                                DisplayError(st.into())
                            }
                        }
                    }
                }
            }
            Page(st) => Page(st),
            Display(st) => Display(st),
            Wait(st) => Wait(st),
            Wipe(st) => Wipe(st),
            DisplayError(st) => {
                let message = match error.take() {
                    None => "Uninitialised".to_string(),
                    Some(error_msg) => error_msg,
                };
                eprintln!("Error: {}", message);
                Wait(st.into())
            }
        };
    }
}
