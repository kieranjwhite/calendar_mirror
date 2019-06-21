mod retriever;

use crate::stm;
use chrono::{format::ParseError, prelude::*};
use reqwest::{Response, StatusCode};
use retriever::*;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::Path,
    thread,
    time::Duration,
};

stm!(cal_stm, Machine, Load(), {
    [Load, Wipe], RequestCodes();
    [Load, Wait], Refresh(Authenticator);
    [Save], ReadFirst(Authenticator);
    [RequestCodes], Poll(String, u64);
    [Load, Page, Poll, ReadFirst, Refresh, RequestCodes], DisplayError(String);
    [Poll, Refresh], Save(Authenticator);
    [ReadFirst], Page(Authenticator, Option<PageToken>, Vec<Event>);
    [Page], Display(Vec<Event>);
    [DisplayError, Display], Wait();
    [Wait], Wipe()
});

const HTTP_ERROR: &str = "HTTP error";
const LOAD_FAILED: &str = "Failed to load credentials";
const QUOTA_EXCEEDED: &str = "Quota Exceeded";
const ACCESS_DENIED: &str = "User has refused to grant access to this calendar";
const UNRECOGNISED_TOKEN_TYPE: &str = "Unrecognised token type";

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    IO(io::Error),
    Chrono(ParseError),
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

impl From<ParseError> for Error {
    fn from(orig: ParseError) -> Error {
        Error::Chrono(orig)
    }
}

struct RefreshToken(String);

#[derive(Serialize, Deserialize, Debug)]
pub struct Authenticator {
    pub access_token: String,
    refresh_token: String,
    expires_in: u32,
}

impl Authenticator {
    pub fn load(path: &Path) -> io::Result<Option<Self>> {
        match File::open(path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                let text: String = reader.lines().collect::<io::Result<String>>()?;
                let credentials = serde_json::from_str(&text)?;
                Ok(Some(credentials))
            }
            Err(_error) => {
                return Ok(None);
            }
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        println!("before file ");
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        let serialised = serde_json::to_string(self)?;
        writer.write_all(serialised.as_bytes())?;
        writer.flush()
    }
}

impl From<PollResponse> for Authenticator {
    fn from(resp: PollResponse) -> Authenticator {
        Authenticator {
            access_token: resp.access_token,
            refresh_token: resp.refresh_token,
            expires_in: resp.expires_in,
        }
    }
}

type AuthTokens = (RefreshToken, RefreshResponse);

impl From<(AuthTokens)> for Authenticator {
    fn from(token_response: AuthTokens) -> Authenticator {
        let (RefreshToken(refresh_token), refresh_response) = token_response;
        Authenticator {
            access_token: refresh_response.access_token,
            refresh_token: refresh_token,
            expires_in: refresh_response.expires_in,
        }
    }
}

#[derive(Debug)]
pub struct Event {
    summary: String,
    description: Option<String>,
    start: DateTime<Local>,
    end: DateTime<Local>,
}

impl From<&retriever::Event> for Result<Event, ParseError> {
    fn from(ev: &retriever::Event) -> Result<Event, ParseError> {
        Ok(Event {
            summary: ev.summary.to_string(),
            description: ev.description.clone(),
            start: ev.start.date_time.parse()?,
            end: ev.end.date_time.parse()?,
        })
    }
}

fn add_events(received: &EventsResponse) -> Result<Vec<Event>, ParseError> {
    let mut events=Vec::new();
    for ev in received.items.iter() {
        let ev_res: Result<Event, ParseError> = ev.into();
        let typed_ev = ev_res?;
        events.push(typed_ev);
    }
    Ok(events)
}

pub fn run() -> Result<(), Error> {
    use Machine::*;
    let today = Local::today().and_hms(0, 0, 0);
    let config_file = Path::new("/home/kieran/projects/rust/calendar_mirror/config.json");
    let retriever = EventRetriever::inst();
    let mut mach: Machine = Load(cal_stm::Load);

    loop {
        mach = match mach {
            Load(st) => match Authenticator::load(config_file) {
                Err(error_msg) => DisplayError(
                    st.into(),
                    format!("{}: {}", LOAD_FAILED, error_msg.to_string()),
                ),
                Ok(None) => RequestCodes(st.into()),
                Ok(Some(creds)) => Refresh(st.into(), creds),
            },
            RequestCodes(st) => {
                let mut resp: Response = retriever.retrieve_dev_and_code()?;
                let status = resp.status();
                match status {
                    StatusCode::OK => {
                        println!("Headers: {:#?}", resp.headers());
                        let body: DeviceUserCodeResponse = resp.json()?;
                        println!("Body is next... {:?}", body);

                        Poll(st.into(), String::from(body.device_code), body.interval as u64)
                    }
                    other_status => {
                        let body: DeviceUserCodeErrorResponse = resp.json()?;
                        eprintln!("Error when getting request code: {:?}", body);
                        match other_status {
                            StatusCode::FORBIDDEN
                                if body.error_code == QUOTA_EXCEEDED_ERROR_CODE =>
                            {
                                DisplayError(st.into(), QUOTA_EXCEEDED.to_string())
                            }
                            _otherwise => DisplayError(
                                st.into(),
                                format!(
                                    "{}: {}, {}",
                                    HTTP_ERROR,
                                    other_status.as_u16(),
                                    body.error_code
                                ),
                            ),
                        }
                    }
                }
            }
            Refresh(st, credential_tokens) => {
                let mut resp: Response = retriever.refresh(&credential_tokens.refresh_token)?;
                let status = resp.status();
                match status {
                    StatusCode::OK => {
                        println!("Headers: {:#?}", resp.headers());
                        let credentials_tokens: RefreshResponse = resp.json()?;

                        let token_type = credentials_tokens.token_type.clone();
                        if token_type != TOKEN_TYPE {
                            DisplayError(st.into(), format!("{}: {}", UNRECOGNISED_TOKEN_TYPE, token_type))
                        } else {
                            println!("Body is next... {:?}", credentials_tokens);
                            let credentials: Authenticator = (
                                RefreshToken(credential_tokens.refresh_token.clone()),
                                credentials_tokens,
                            )
                                .into();
                            Save(st.into(), credentials)
                        }
                    }
                    other_status => {
                        let body: PollErrorResponse = resp.json()?;
                        let err_msg = format!(
                            "When refreshing status: {:?} body: {:?}",
                            other_status, body
                        );
                        DisplayError(st.into(), err_msg)
                    }
                }
            }
            Poll(st, device_code, delay_s) => {
                thread::sleep(Duration::from_secs(delay_s));
                let mut resp: Response = retriever.poll(&device_code)?;
                let status = resp.status();
                match status {
                    StatusCode::OK => {
                        println!("Headers: {:#?}", resp.headers());
                        let credentials_tokens: PollResponse = resp.json()?;

                        let token_type = credentials_tokens.token_type.clone();
                        if token_type != TOKEN_TYPE {
                            DisplayError(st.into(), format!("{}: {}", UNRECOGNISED_TOKEN_TYPE, token_type))
                        } else {
                            println!("Body is next... {:?}", credentials_tokens);
                            Save(st.into(), credentials_tokens.into())
                        }
                    }
                    other_status => {
                        let body: PollErrorResponse = resp.json()?;
                        eprintln!("Error when polling: {:?}", body);
                        match other_status {
                            StatusCode::FORBIDDEN if body.error == ACCESS_DENIED_ERROR => {
                                DisplayError(st.into(), ACCESS_DENIED.to_string())
                            }
                            StatusCode::BAD_REQUEST
                                if body.error == AUTHORISATION_PENDING_ERROR =>
                            {
                                Poll(st, device_code, delay_s)
                            }
                            StatusCode::PRECONDITION_REQUIRED
                                if body.error == AUTHORISATION_PENDING_ERROR =>
                            {
                                Poll(st, device_code, delay_s)
                            }
                            StatusCode::TOO_MANY_REQUESTS
                                if body.error == POLLING_TOO_FREQUENTLY_ERROR =>
                            {
                                Poll(st, device_code, delay_s*2)
                            }
                            _otherwise => {
                                DisplayError(st.into(), format!(
                                    "HTTP error: {}, {}, {}",
                                    other_status.as_u16(),
                                    body.error,
                                    body.error_description
                                ))
                            }
                        }
                    }
                }
            }
            Save(st, credentials_tokens) => {
                credentials_tokens.save(config_file)?;
                ReadFirst(st.into(), credentials_tokens)
            }
            ReadFirst(st, credentials_tokens) => {
                let mut resp: Response = retriever.read(
                    &format!("Bearer {}", credentials_tokens.access_token),
                    &today,
                    &(today + chrono::Duration::days(1) - chrono::Duration::seconds(1)),
                    &Option::<PageToken>::None,
                )?;
                let status = resp.status();
                match status {
                    StatusCode::OK => {
                        println!("Event Headers: {:#?}", resp.headers());
                        let events_resp: EventsResponse = resp.json()?;
                        let new_events=add_events(&events_resp)?;
                        let page_token = match events_resp.next_page_token {
                            None => None,
                            Some(next_page) => Some(PageToken(next_page)),
                        };
                        Page(st.into(), credentials_tokens, page_token, new_events)
                    }
                    _other_status => {
                        println!("Event Headers: {:#?}", resp.headers());
                        println!("Event is next... {:?}", resp.text()?);
                        DisplayError(st.into(), format!("in readfirst. http status: {:?}", status))
                    }
                }
            }
            Page(st, credentials_tokens, page_token, mut events) => {
                if let None = page_token {
                    Display(st.into(), events)
                } else {
                    let mut resp: Response = retriever.read(
                        &format!("Bearer {}", credentials_tokens.access_token),
                        &today,
                        &(today + chrono::Duration::days(1) - chrono::Duration::seconds(1)),
                        &page_token,
                    )?;
                    let status = resp.status();
                    match status {
                        StatusCode::OK => {
                            println!("Event Headers: {:#?}", resp.headers());
                            let events_resp: EventsResponse = resp.json()?;
                            let new_events=add_events(&events_resp)?;
                            let page_token = match events_resp.next_page_token {
                                None => None,
                                Some(next_page) => Some(PageToken(next_page)),
                            };

                            events.extend(new_events);
                            Page(st, credentials_tokens, page_token, events)
                        }
                        _other_status => {
                            println!("Event Headers: {:#?}", resp.headers());
                            println!("Event is next... {:?}", resp.text()?);
                            DisplayError(st.into(), format!("in readfirst. http status: {:?}", status))
                        }
                    }
                }
            }
            Display(st, events) => {
                println!("Retrieved events: {:?}", events);
                Wait(st.into())
            }
            Wait(st) => Wait(st),
            Wipe(st) => Wipe(st),
            DisplayError(st, message) => {
                eprintln!("Error: {}", message);
                Wait(st.into())
            }
        };
    }
}
