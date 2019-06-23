mod retriever;

use crate::cal_display::Renderer;
use crate::cal_machine::instant_types::{RefreshedAt, WaitingFrom};
use crate::display;
use crate::err;
use crate::stm;
use chrono::{format::ParseError, prelude::*};
use retriever::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{
    cmp::{Ord, Ordering},
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::Path,
    thread,
};

stm!(cal_stm, Machine, Start(), {
    [DisplayError] => ErrorWait(WaitingFrom);
    [Start,ErrorWait] => Load();
    [Load, Wipe] => RequestCodes();
    [Load, Wait] => Refresh(RefreshToken);
    [Refresh, Save, Wait] => ReadFirst(Authenticators, RefreshedAt);
    [RequestCodes] => Poll(String, PeriodSeconds);
    [Load, Page, Poll, ReadFirst, Refresh, RequestCodes] => DisplayError(String);
    [Poll] => Save(Authenticators);
    [ReadFirst] => Page(Authenticators, Option<PageToken>, Vec<Event>, RefreshedAt);
    [Page] => Display(Authenticators, Vec<Event>, RefreshedAt);
    [Display] => Wait(Authenticators, RefreshedAt, WaitingFrom);
    [Wait] => Wipe()
});

type PeriodSeconds = u64;
type AuthTokens = (RefreshToken, RefreshResponse);

const TWO_MINS_S: PeriodSeconds = 120;
const REFRESH_PERIOD_S: PeriodSeconds = 300;

err!(Error {
    Chrono(ParseError),
    Display(display::Error),
    IO(io::Error),
    Reqwest(reqwest::Error)
});

#[derive(Serialize, Deserialize, Debug)]
pub struct VolatileAuthenticator {
    pub access_token: String,
    expires_in: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RefreshToken(String);

macro_rules! instant {
    ($name:ident) => {
        #[derive(Debug)]
        pub struct $name(Instant);
        
        impl $name {
            pub fn now() -> $name {
                $name(Instant::now())
            }
            
            pub fn instant(&self) -> Instant {
                self.0
            }
        }
    };
}

mod instant_types {
    use std::time::Instant;
    instant!(RefreshedAt);
    instant!(WaitingFrom);
}

impl RefreshToken {
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

pub struct Authenticators {
    refresh_token: RefreshToken,
    volatiles: VolatileAuthenticator,
}

impl From<PollResponse> for Authenticators {
    fn from(resp: PollResponse) -> Authenticators {
        Authenticators {
            refresh_token: RefreshToken(resp.refresh_token),
            volatiles: VolatileAuthenticator {
                access_token: resp.access_token,
                expires_in: resp.expires_in,
            },
        }
    }
}

impl From<(AuthTokens)> for Authenticators {
    fn from((refresh_token, refresh_response): AuthTokens) -> Authenticators {
        Authenticators {
            refresh_token,
            volatiles: VolatileAuthenticator {
                access_token: refresh_response.access_token,
                expires_in: refresh_response.expires_in,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    pub summary: String,
    pub description: Option<String>,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.start.cmp(&other.start) {
            Ordering::Equal => match self.end.cmp(&other.end) {
                Ordering::Equal => match self.summary.cmp(&other.summary) {
                    Ordering::Equal => self.description.cmp(&other.description),
                    other => other,
                },
                other => other,
            },
            other => other,
        }
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
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
    let mut events = Vec::new();
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
    let config_file = Path::new("config.json");
    let retriever = EventRetriever::inst();
    let mut mach: Machine = Start(cal_stm::Start);

    if cfg!(feature = "render_stm") {
        let mut f = File::create("docs/cal_machine.dot")?;
        Machine::render_to(&mut f);
        f.flush()?;
        Ok(())
    } else {
        use reqwest::{Response, StatusCode};

        const HTTP_ERROR: &str = "HTTP error";
        const LOAD_FAILED: &str = "Failed to load credentials";
        const QUOTA_EXCEEDED: &str = "Quota Exceeded";
        const ACCESS_DENIED: &str = "User has refused to grant access to this calendar";
        const UNRECOGNISED_TOKEN_TYPE: &str = "Unrecognised token type";

        let mut renderer = Renderer::new()?;

        loop {
            mach = match mach {
                Start(st) => Load(st.into()),
                Load(st) => match RefreshToken::load(config_file) {
                    Err(error_msg) => DisplayError(
                        st.into(),
                        format!("{}: {}", LOAD_FAILED, error_msg.to_string()),
                    ),
                    Ok(None) => RequestCodes(st.into()),
                    Ok(Some(refresh_token)) => Refresh(st.into(), refresh_token),
                },
                RequestCodes(st) => {
                    let mut resp: Response = retriever.retrieve_dev_and_code()?;
                    let status = resp.status();
                    match status {
                        StatusCode::OK => {
                            println!("Headers: {:#?}", resp.headers());
                            let body: DeviceUserCodeResponse = resp.json()?;
                            println!("Body is next... {:?}", body);

                            Poll(
                                st.into(),
                                String::from(body.device_code),
                                body.interval as PeriodSeconds,
                            )
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
                Refresh(st, RefreshToken(refresh_token)) => {
                    let mut resp: Response = retriever.refresh(&refresh_token)?;
                    let status = resp.status();
                    match status {
                        StatusCode::OK => {
                            println!("Headers: {:#?}", resp.headers());
                            let credentials_tokens: RefreshResponse = resp.json()?;

                            let token_type = credentials_tokens.token_type.clone();
                            if token_type != TOKEN_TYPE {
                                DisplayError(
                                    st.into(),
                                    format!("{}: {}", UNRECOGNISED_TOKEN_TYPE, token_type),
                                )
                            } else {
                                println!("Body is next... {:?}", credentials_tokens);
                                let credentials: Authenticators =
                                    (RefreshToken(refresh_token.clone()), credentials_tokens)
                                        .into();
                                ReadFirst(st.into(), credentials, RefreshedAt::now())
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
                                DisplayError(
                                    st.into(),
                                    format!("{}: {}", UNRECOGNISED_TOKEN_TYPE, token_type),
                                )
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
                                    Poll(st, device_code, delay_s * 2)
                                }
                                _otherwise => DisplayError(
                                    st.into(),
                                    format!(
                                        "HTTP error: {}, {}, {}",
                                        other_status.as_u16(),
                                        body.error,
                                        body.error_description
                                    ),
                                ),
                            }
                        }
                    }
                }
                Save(st, credentials) => {
                    credentials.refresh_token.save(config_file)?;
                    ReadFirst(st.into(), credentials, RefreshedAt::now())
                }
                ReadFirst(st, credentials_tokens, refreshed_at) => {
                    let mut resp: Response = retriever.read(
                        &format!("Bearer {}", credentials_tokens.volatiles.access_token),
                        &today,
                        &(today + chrono::Duration::days(1) - chrono::Duration::seconds(1)),
                        &Option::<PageToken>::None,
                    )?;
                    let status = resp.status();
                    match status {
                        StatusCode::OK => {
                            println!("Event Headers: {:#?}", resp.headers());
                            let events_resp: EventsResponse = resp.json()?;
                            let new_events = add_events(&events_resp)?;
                            let page_token = match events_resp.next_page_token {
                                None => None,
                                Some(next_page) => Some(PageToken(next_page)),
                            };
                            Page(
                                st.into(),
                                credentials_tokens,
                                page_token,
                                new_events,
                                refreshed_at,
                            )
                        }
                        _other_status => {
                            println!("Event Headers: {:#?}", resp.headers());
                            println!("Event is next... {:?}", resp.text()?);
                            DisplayError(
                                st.into(),
                                format!("in readfirst. http status: {:?}", status),
                            )
                        }
                    }
                }
                Page(st, credentials_tokens, page_token, mut events, refreshed_at) => {
                    if let None = page_token {
                        Display(st.into(), credentials_tokens, events, refreshed_at)
                    } else {
                        let mut resp: Response = retriever.read(
                            &format!("Bearer {}", credentials_tokens.volatiles.access_token),
                            &today,
                            &(today + chrono::Duration::days(1) - chrono::Duration::seconds(1)),
                            &page_token,
                        )?;
                        let status = resp.status();
                        match status {
                            StatusCode::OK => {
                                println!("Event Headers: {:#?}", resp.headers());
                                let events_resp: EventsResponse = resp.json()?;
                                let new_events = add_events(&events_resp)?;
                                let page_token = match events_resp.next_page_token {
                                    None => None,
                                    Some(next_page) => Some(PageToken(next_page)),
                                };

                                events.extend(new_events);
                                Page(st, credentials_tokens, page_token, events, refreshed_at)
                            }
                            _other_status => {
                                println!("Event Headers: {:#?}", resp.headers());
                                println!("Event is next... {:?}", resp.text()?);
                                DisplayError(
                                    st.into(),
                                    format!("in readfirst. http status: {:?}", status),
                                )
                            }
                        }
                    }
                }
                Display(st, credentials, events, refreshed_at) => {
                    println!("Retrieved events: {:?}", events);
                    renderer.display(&today, &events)?;
                    Wait(st.into(), credentials, refreshed_at, WaitingFrom::now())
                }
                Wait(st, credentials, refreshed_at, started_wait_at) => {
                    let waiting_for = started_wait_at.instant().elapsed().as_secs();
                    if waiting_for >= REFRESH_PERIOD_S {
                        let elapsed_since_token_refresh =
                            refreshed_at.instant().elapsed().as_secs();
                        let expires_in = credentials.volatiles.expires_in;
                        if expires_in < TWO_MINS_S
                            || elapsed_since_token_refresh
                                >= credentials.volatiles.expires_in - TWO_MINS_S
                        {
                            Refresh(st.into(), credentials.refresh_token)
                        } else {
                            ReadFirst(st.into(), credentials, refreshed_at)
                        }
                    } else {
                        Wait(st, credentials, refreshed_at, started_wait_at)
                    }
                }
                ErrorWait(st, started_wait_at) => {
                    let waiting_for = started_wait_at.instant().elapsed().as_secs();
                    if waiting_for >= REFRESH_PERIOD_S {
                        Load(st.into())
                    } else {
                        ErrorWait(st, started_wait_at)
                    }
                }
                Wipe(st) => Wipe(st),
                DisplayError(st, message) => {
                    eprintln!("Error: {}", message);
                    ErrorWait(st.into(), WaitingFrom::now())
                }
            };
        }
    }
}
