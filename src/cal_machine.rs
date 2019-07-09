pub mod evs;
mod retriever;

use crate::{
    cal_display::{Error as CalDisplayError, RefreshType, Renderer},
    cal_machine::{
        evs::{Appointments, Error as EvError},
        instant_types::{DownloadedAt, RefreshedAt},
    },
    display::{self},
    err,
    formatter::{self, GlyphRow},
    gpio_in::{
        self, Button, DetectableDuration, Error as GPIO_Error, LongButtonEvent, LongPressButton,
        LongReleaseDuration, Pin, GPIO, SW1_GPIO, SW2_GPIO, SW3_GPIO, SW4_GPIO,
    },
    stm,
};
use chrono::{format::ParseError, prelude::*};
use nix::{unistd::*, Error as NixError};
use retriever::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{
    ffi::CString,
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering as AtomicOrdering},
        Arc,
    },
    thread,
};

stm!(cal_stm, Machine, [ErrorWait] => Load(), {
    [DisplayError] => ErrorWait(DownloadedAt);
    [ErrorWait, Load, Wait] => RequestCodes();
    [Load, Wait] => Refresh(RefreshToken);
    [Refresh, Save, Wait] => ReadFirst(Authenticators, RefreshedAt);
    [RequestCodes] => Poll(String, PeriodSeconds);
    [Load, Page, Poll, ReadFirst, Refresh, RequestCodes] => DisplayError(String);
    [Poll] => Save(Authenticators);
    [ReadFirst] => Page(Authenticators, Option<PageToken>, Appointments, RefreshedAt, DownloadedAt);
    [Page, Wait] => Display(Authenticators, Appointments, RefreshedAt, DownloadedAt, RefreshType, GlyphRow);
    [Display] => Wait(Authenticators, Appointments, RefreshedAt, DownloadedAt, GlyphRow)
});

type PeriodSeconds = u64;
type AuthTokens = (RefreshToken, RefreshResponse);

const FOUR_MINS: Duration = Duration::from_secs(240);
const RECHECK_PERIOD: Duration = Duration::from_secs(300);
const BUTTON_POLL_PERIOD: Duration = Duration::from_millis(25);

err!(Error {
    Chrono(ParseError),
    CalDisplayError(CalDisplayError),
    Display(display::Error),
    IO(io::Error),
    Ev(EvError),
    Reqwest(reqwest::Error),
    GPIO(GPIO_Error),
    Nix(NixError)
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
    instant!(DownloadedAt);
}

impl RefreshToken {
    pub fn load(path: &Path) -> io::Result<Option<Self>> {
        match File::open(path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                let text: String = reader.lines().collect::<io::Result<String>>()?;
                let credentials = serde_json::from_str(&text)?;
                println!("after token file load");
                Ok(Some(credentials))
            }
            Err(_error) => {
                return Ok(None);
            }
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        println!("before token file save");
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        let serialised = serde_json::to_string(self)?;
        writer.write_all(serialised.as_bytes())?;
        writer.flush()
    }
}

#[derive(Debug)]
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

fn opt_filter<T>(val: &Option<T>, pred: impl Fn(&T) -> bool) -> bool {
    match val {
        None => false,
        Some(inner) => pred(inner),
    }
}

pub fn render_stms() -> Result<(), Error> {
    let mut f = File::create("docs/cal_machine.dot")?;
    Machine::render_to(&mut f);
    f.flush()?;

    f = File::create("docs/ev_stm.dot")?;
    evs::Machine::render_to(&mut f);
    f.flush()?;

    f = File::create("docs/long_press_button_stm.dot")?;
    gpio_in::LongPressMachine::render_to(&mut f);
    f.flush()?;

    f = File::create("docs/tokenising_stm.dot")?;
    formatter::Machine::render_to(&mut f);
    f.flush()?;

    Ok(())
}

fn shutdown() -> Result<(), NixError> {
    println!("shutting down...");
    execvp(
        &CString::new("halt").expect("Invalid CString: halt"),
        &[
            CString::new("--halt").expect("invalid arg --halt"),
            CString::new("-ffn").expect("invalid arg -ffn"),
        ],
    )?;
    println!("shutdown failed");
    Ok(())
}

//    loader: impl Fn() -> io::Result<Option<RefreshToken>>,
pub fn run(
    renderer: &mut Renderer,
    quitter: &Arc<AtomicBool>,
    config_file: &Path,
    saver: impl Fn(&RefreshToken, &mut Renderer) -> Result<(), Error>,
) -> Result<(), Error> {
    use Machine::*;

    use reqwest::{Response, StatusCode};

    const HTTP_ERROR: &str = "HTTP error";
    const LOAD_FAILED: &str = "Failed to load credentials";
    const QUOTA_EXCEEDED: &str = "Quota Exceeded";
    const ACCESS_DENIED: &str = "User has refused to grant access to this calendar";
    const UNRECOGNISED_TOKEN_TYPE: &str = "Unrecognised token type";
    const LONGISH_DURATION: Duration = Duration::from_millis(2000);
    const LONG_DURATION: Duration = Duration::from_secs(4);

    let mut today = Local::today().and_hms(0, 0, 0);
    let mut display_date = today;
    let retriever = EventRetriever::inst();
    let mut mach: Machine = Load(cal_stm::Load);
    let mut gpio = GPIO::new()?;
    let mut reset_button = LongPressButton::new(
        Pin(SW3_GPIO),
        DetectableDuration(LONG_DURATION),
        LongReleaseDuration(LONGISH_DURATION),
    );
    let mut back_button = LongPressButton::new(
        Pin(SW4_GPIO),
        DetectableDuration(LONG_DURATION),
        LongReleaseDuration(LONGISH_DURATION),
    );
    let mut next_button = LongPressButton::new(
        Pin(SW1_GPIO),
        DetectableDuration(LONG_DURATION),
        LongReleaseDuration(LONGISH_DURATION),
    );
    let mut scroll_button = LongPressButton::new(
        Pin(SW2_GPIO),
        DetectableDuration(LONG_DURATION),
        LongReleaseDuration(LONGISH_DURATION),
    );

    while !quitter.load(AtomicOrdering::SeqCst) {
        mach = match mach {
            Load(st) => match RefreshToken::load(&config_file) {
                //Load(st) => match loader() {
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
                        let body: DeviceUserCodeResponse = resp.json()?;
                        println!("Body is next... {:?}", body);
                        renderer.display_user_code(
                            &body.user_code,
                            &(Local::now() + chrono::Duration::seconds(body.expires_in)),
                            &body.verification_url,
                        )?;

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
                                (RefreshToken(refresh_token.clone()), credentials_tokens).into();
                            ReadFirst(st.into(), credentials, RefreshedAt::now())
                        }
                    }
                    other_status => {
                        let err_msg = format!("When refreshing status: {:?}", other_status);
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
                    other_status => match resp.json::<PollErrorResponse>() {
                        Ok(body) => {
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
                        Err(error) => DisplayError(
                            st.into(),
                            format!("HTTP error: {}, {:?}", other_status.as_u16(), error),
                        ),
                    },
                }
            }
            Save(st, credentials) => {
                //credentials.refresh_token.save(&config_file)?;
                saver(&credentials.refresh_token, renderer)?;
                ReadFirst(st.into(), credentials, RefreshedAt::now())
            }
            ReadFirst(st, credentials_tokens, refreshed_at) => {
                let mut resp: Response = retriever.read(
                    &format!("Bearer {}", credentials_tokens.volatiles.access_token),
                    &display_date,
                    &(display_date + chrono::Duration::days(1) - chrono::Duration::seconds(1)),
                    &Option::<PageToken>::None,
                )?;
                let status = resp.status();
                match status {
                    StatusCode::OK => {
                        let events_resp: EventsResponse = resp.json()?;
                        let mut new_events = evs::Appointments::new();
                        new_events.add(&events_resp)?;
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
                            DownloadedAt::now(),
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
            Page(st, credentials_tokens, page_token, mut events, refreshed_at, downloaded_at) => {
                if let None = page_token {
                    Display(
                        st.into(),
                        credentials_tokens,
                        events,
                        refreshed_at,
                        downloaded_at,
                        RefreshType::Full,
                        GlyphRow(0),
                    )
                } else {
                    let mut resp: Response = retriever.read(
                        &format!("Bearer {}", credentials_tokens.volatiles.access_token),
                        &display_date,
                        &(display_date + chrono::Duration::days(1) - chrono::Duration::seconds(1)),
                        &page_token,
                    )?;
                    let status = resp.status();
                    match status {
                        StatusCode::OK => {
                            let events_resp: EventsResponse = resp.json()?;
                            events.add(&events_resp)?;
                            let page_token = match events_resp.next_page_token {
                                None => None,
                                Some(next_page) => Some(PageToken(next_page)),
                            };

                            Page(
                                st,
                                credentials_tokens,
                                page_token,
                                events,
                                refreshed_at,
                                downloaded_at,
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
            }
            Display(st, credentials, apps, refreshed_at, downloaded_at, refresh_type, v_pos) => {
                println!("Retrieved events: {:?}", apps.events);
                renderer.display_events(
                    &display_date,
                    &apps,
                    refresh_type,
                    |num_event_rows, screen_height| {
                        let max_row_offset = if screen_height.0 <= num_event_rows.0 {
                            num_event_rows.0 - screen_height.0
                        } else {
                            0
                        };
                        GlyphRow(v_pos.0 % (max_row_offset + 1))
                    },
                )?;
                Wait(
                    st.into(),
                    credentials,
                    apps,
                    refreshed_at,
                    downloaded_at,
                    v_pos,
                )
            }
            Wait(st, credentials, apps, refreshed_at, started_wait_at, v_pos) => {
                let waiting_for = started_wait_at.instant().elapsed();
                let elapsed_since_token_refresh = refreshed_at.instant().elapsed();

                if (elapsed_since_token_refresh + FOUR_MINS).as_secs()
                    >= credentials.volatiles.expires_in
                {
                    Refresh(st.into(), credentials.refresh_token)
                } else {
                    renderer.heartbeat(if (elapsed_since_token_refresh.as_secs() & 2) == 2 {
                        true
                    } else {
                        false
                    })?;

                    thread::sleep(BUTTON_POLL_PERIOD);
                    let reset_event = reset_button.event(&mut gpio)?;
                    let back_event = back_button.event(&mut gpio)?;
                    let next_event = next_button.event(&mut gpio)?;
                    let scroll_event = scroll_button.event(&mut gpio)?;

                    let short_check = |e: &LongButtonEvent| e.is_short_press();
                    let release_check = |e: &LongButtonEvent| e.is_release();
                    let long_check = |e: &LongButtonEvent| e.is_long_press();

                    let new_today = Local::today().and_hms(0, 0, 0);

                    if opt_filter(&reset_event, long_check) {
                        RequestCodes(st.into())
                    } else if opt_filter(&reset_event, short_check) {
                        shutdown()?;
                        Wait(st, credentials, apps, refreshed_at, started_wait_at, v_pos)
                    } else if today.date() != new_today.date()
                        || opt_filter(&scroll_event, long_check)
                    {
                        println!("full display & date refresh");
                        today = new_today;
                        display_date = today;
                        ReadFirst(st.into(), credentials, refreshed_at)
                    } else if opt_filter(&scroll_event, short_check) {
                        Display(
                            st.into(),
                            credentials,
                            apps,
                            refreshed_at,
                            started_wait_at,
                            RefreshType::Partial,
                            GlyphRow(v_pos.0 + 2),
                        )
                    } else if opt_filter(&back_event, release_check)
                        || opt_filter(&next_event, release_check)
                        || waiting_for >= RECHECK_PERIOD
                    {
                        println!("full display refresh");
                        ReadFirst(st.into(), credentials, refreshed_at)
                    } else if opt_filter(&back_event, short_check) {
                        display_date = display_date - chrono::Duration::days(1);
                        println!("New date: {:?}", display_date);
                        renderer.refresh_date(&display_date)?;
                        Wait(st, credentials, apps, refreshed_at, started_wait_at, v_pos)
                    } else if opt_filter(&next_event, short_check) {
                        display_date = display_date + chrono::Duration::days(1);
                        renderer.refresh_date(&display_date)?;
                        Wait(st, credentials, apps, refreshed_at, started_wait_at, v_pos)
                    } else {
                        Wait(st, credentials, apps, refreshed_at, started_wait_at, v_pos)
                    }
                }
            }
            ErrorWait(st, started_wait_at) => {
                let waiting_for = started_wait_at.instant().elapsed();
                if waiting_for >= RECHECK_PERIOD {
                    Load(st.into())
                } else {
                    thread::sleep(BUTTON_POLL_PERIOD);
                    let reset_event = reset_button.event(&mut gpio)?;
                    if opt_filter(&reset_event, |e| e.is_long_press()) {
                        RequestCodes(st.into())
                    } else if opt_filter(&reset_event, |e| e.is_short_press()) {
                        shutdown()?;
                        ErrorWait(st, started_wait_at)
                    } else {
                        ErrorWait(st, started_wait_at)
                    }
                }
            }
            DisplayError(st, message) => {
                eprintln!("Error: {}", message);
                ErrorWait(st.into(), DownloadedAt::now())
            }
        };
    }
    Ok(())
}
