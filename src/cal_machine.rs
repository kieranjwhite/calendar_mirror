pub mod evs;
mod retriever;

use crate::{
    cal_display::{self, Error as CalDisplayError, RefreshType, Renderer, Status},
    cal_machine::{
        evs::{Appointments, Error as EvError, Now},
        instant_types::*,
    },
    cloneable,
    display::{self},
    err,
    formatter::{self, GlyphYCnt},
    gpio_in::{
        self, Button, DetectableDuration, Error as GPIO_Error, LongButtonEvent, LongPressButton,
        LongReleaseDuration, Pin, GPIO, SW1_GPIO, SW2_GPIO, SW3_GPIO, SW4_GPIO,
    },
    stm,
};
use chrono::{format::ParseError, prelude::*};
use log::{trace};
use nix::{unistd::*, Error as NixError};
use retriever::*;
use serde::{Deserialize, Serialize};
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
    time::Duration,
};

//trace_macros!(true);
stm!(machine cal_stm, Machine, CalsAtEnd, CalTerminals, [ErrorWait] => LoadAuth() |end|, {
    [DisplayError] => ErrorWait(DownloadedAt) |end|;
    [ErrorWait, LoadAuth, NetworkOutage, PollEvents] => RequestCodes() |end|;
    [LoadAuth, NetworkOutage, PageEvents, PollEvents] => RefreshAuth(RefreshToken, PendingDisplayDate) |end|;
    [DeviceAuthPoll, RefreshAuth, PollEvents] => ReadFirstEvents(Authenticators, RefreshedAt, RefreshType, PendingDisplayDate) |end|;
    [RequestCodes] => DeviceAuthPoll(String, PeriodSeconds) |end|;
    [LoadAuth, PageEvents, DeviceAuthPoll, ReadFirstEvents, RefreshAuth, RequestCodes] => DisplayError(String) |end|;
    [ReadFirstEvents] => PageEvents(Authenticators, Option<PageToken>, Appointments, RefreshedAt, DownloadedAt, RefreshType, PendingDisplayDate) |end|;
    [PageEvents] => PollEvents(Authenticators, RefreshedAt, DownloadedAt, TimeUpdatedAt, PendingDisplayDate) |end|;
    [RefreshAuth, ReadFirstEvents, PageEvents] => CachedDisplay(RefreshToken, LastNetErrorAt) |end|;
    [CachedDisplay] => NetworkOutage(RefreshToken, LastNetErrorAt, TimeUpdatedAt) |end|
});
//trace_macros!(false);

type PeriodSeconds = u64;
type AuthTokens = (RefreshToken, RefreshResponse);

const PREEMPTIVE_REFRESH_OFFSET_MINS: Duration = Duration::from_secs(240);
const RECHECK_PERIOD: Duration = Duration::from_secs(300);
const TIME_UPDATE_PERIOD: Duration = Duration::from_secs(60);
const BUTTON_POLL_PERIOD: Duration = Duration::from_millis(25);
const V_POS_INC: usize = 5;

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
        pub struct $name(pub Instant);

        impl $name {
            pub fn now() -> $name {
                $name(Instant::now())
            }
        }

        impl AsRef<Instant> for $name {
            fn as_ref(&self) -> &Instant {
                &self.0
            }
        }
    };
}

mod instant_types {
    use std::time::Instant;
    instant!(RefreshedAt);
    instant!(DownloadedAt);
    instant!(TimeUpdatedAt);
    instant!(LastNetErrorAt);
}

//pub struct PendingDisplayDate(DateTime<Local>);
cloneable!(PendingDisplayDate, DateTime<Local>);

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
    formatter::FormattingMachine::render_to(&mut f);
    f.flush()?;

    f = File::create("docs/display_stm.dot")?;
    cal_display::DisplayMachine::render_to(&mut f);
    f.flush()?;

    f = File::create("docs/appointments_stm.dot")?;
    cal_display::AppMachine::render_to(&mut f);
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

fn max_row_offset(num_event_rows: GlyphYCnt, screen_height: GlyphYCnt) -> GlyphYCnt {
    if screen_height.0 <= num_event_rows.0 {
        GlyphYCnt(num_event_rows.0 - screen_height.0)
    } else {
        GlyphYCnt(0)
    }
}

fn prev_v_pos(v_pos: GlyphYCnt) -> GlyphYCnt {
    if V_POS_INC > v_pos.0 {
        GlyphYCnt(0)
    } else {
        GlyphYCnt((v_pos.0 - V_POS_INC) as usize)
    }
}

fn new_pos(v_pos: GlyphYCnt, num_event_rows: GlyphYCnt, screen_height: GlyphYCnt) -> GlyphYCnt {
    let max_row_offset = max_row_offset(num_event_rows, screen_height);
    let prev_v_pos = prev_v_pos(v_pos);

    if prev_v_pos.0 + screen_height.0 >= num_event_rows.0 {
        GlyphYCnt(0)
    } else if v_pos.0 > max_row_offset.0 {
        max_row_offset
    } else {
        v_pos
    }
}

//    loader: impl Fn() -> io::Result<Option<RefreshToken>>,
pub fn run(
    renderer: &mut Renderer,
    quitter: &Arc<AtomicBool>,
    config_file: &Path,
    saver: impl Fn(&RefreshToken, &mut Renderer) -> Result<(), Error>,
) -> Result<(), Error> {
    use Machine::{
        CachedDisplay, DeviceAuthPoll, DisplayError, ErrorWait, LoadAuth, NetworkOutage,
        PageEvents, PollEvents, ReadFirstEvents, RefreshAuth, RequestCodes,
    };

    use reqwest::{Response, StatusCode};

    const HTTP_ERROR: &str = "HTTP error";
    const LOAD_FAILED: &str = "Failed to load credentials";
    const QUOTA_EXCEEDED: &str = "Quota Exceeded";
    const ACCESS_DENIED: &str = "User has refused to grant access to this calendar";
    const UNRECOGNISED_TOKEN_TYPE: &str = "Unrecognised token type";
    const LONGISH_DURATION: Duration = Duration::from_millis(1500);
    const LONG_DURATION: Duration = Duration::from_secs(4);
    const GLYPH_Y_ORIGIN: GlyphYCnt = GlyphYCnt(0);

    let mut today = Local::today().and_hms(0, 0, 0);
    let mut display_date = today; //don't delete this variable -- it's needed after a network outage to display events from that last date we navigated to, while at the same time reverting date changes due to the previous failed date navigation operation
    let mut v_pos: GlyphYCnt = GLYPH_Y_ORIGIN;
    let retriever = EventRetriever::inst();
    let mut mach = Machine::new((), Box::new(|mach| {
        trace!("dropping cal_machine Machine: {:?}", mach);
        match mach {
            CalsAtEnd::LoadAuth(st) =>CalTerminals::LoadAuth(st),
            CalsAtEnd::ErrorWait(st) =>CalTerminals::ErrorWait(st),
            CalsAtEnd::RequestCodes(st) =>CalTerminals::RequestCodes(st),
            CalsAtEnd::RefreshAuth(st) =>CalTerminals::RefreshAuth(st),
            CalsAtEnd::ReadFirstEvents(st)=>CalTerminals::ReadFirstEvents(st),
            CalsAtEnd::DeviceAuthPoll(st) =>CalTerminals::DeviceAuthPoll(st),
            CalsAtEnd::DisplayError(st)=>CalTerminals::DisplayError(st),
            CalsAtEnd::PageEvents(st)=>CalTerminals::PageEvents(st),
            CalsAtEnd::PollEvents(st)=>CalTerminals::PollEvents(st),
            CalsAtEnd::CachedDisplay(st)=>CalTerminals::CachedDisplay(st),
            CalsAtEnd::NetworkOutage(st)=>CalTerminals::NetworkOutage(st),
        }
    }));
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
            LoadAuth(st) => match RefreshToken::load(&config_file) {
                Err(error_msg) => DisplayError(
                    st.into(),
                    format!("{}: {}", LOAD_FAILED, error_msg.to_string()),
                ),
                Ok(None) => RequestCodes(st.into()),
                Ok(Some(refresh_token)) => {
                    RefreshAuth(st.into(), refresh_token, PendingDisplayDate(today))
                }
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

                        DeviceAuthPoll(
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
            RefreshAuth(st, RefreshToken(refresh_token), pending_display_date) => {
                renderer.display_status(Status::NetworkPending, true)?;
                match retriever.refresh(&refresh_token) {
                    Ok(mut resp) => {
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
                                        (RefreshToken(refresh_token.clone()), credentials_tokens)
                                            .into();
                                    ReadFirstEvents(
                                        st.into(),
                                        credentials,
                                        RefreshedAt::now(),
                                        RefreshType::Full,
                                        pending_display_date,
                                    )
                                }
                            }
                            other_status => {
                                let err_msg = format!("When refreshing status: {:?}", other_status);
                                DisplayError(st.into(), err_msg)
                            }
                        }
                    }

                    Err(err) => {
                        eprintln!("Error refreshing: {:?}", err);
                        CachedDisplay(
                            st.into(),
                            RefreshToken(refresh_token),
                            LastNetErrorAt::now(),
                        )
                    }
                }
            }
            DeviceAuthPoll(st, device_code, delay_s) => {
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
                            let auth: Authenticators = credentials_tokens.into();
                            saver(&auth.refresh_token, renderer)?;
                            ReadFirstEvents(
                                st.into(),
                                auth,
                                RefreshedAt::now(),
                                RefreshType::Full,
                                PendingDisplayDate(today),
                            )
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
                                    DeviceAuthPoll(st, device_code, delay_s)
                                }
                                StatusCode::PRECONDITION_REQUIRED
                                    if body.error == AUTHORISATION_PENDING_ERROR =>
                                {
                                    DeviceAuthPoll(st, device_code, delay_s)
                                }
                                StatusCode::TOO_MANY_REQUESTS
                                    if body.error == POLLING_TOO_FREQUENTLY_ERROR =>
                                {
                                    DeviceAuthPoll(st, device_code, delay_s * 2)
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
            ReadFirstEvents(
                st,
                credentials_tokens,
                refreshed_at,
                refresh_type,
                pending_display_date,
            ) => {
                renderer.display_status(Status::NetworkPending, true)?;
                match retriever.read(
                    &format!("Bearer {}", credentials_tokens.volatiles.access_token),
                    &pending_display_date.0,
                    &(*pending_display_date.as_ref() + chrono::Duration::days(1)
                        - chrono::Duration::seconds(1)),
                    &Option::<PageToken>::None,
                ) {
                    Ok(mut resp) => {
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
                                PageEvents(
                                    st.into(),
                                    credentials_tokens,
                                    page_token,
                                    new_events,
                                    refreshed_at,
                                    DownloadedAt::now(),
                                    refresh_type,
                                    pending_display_date,
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
                    Err(err) => {
                        eprintln!("Error refreshing: {:?}", err);
                        CachedDisplay(
                            st.into(),
                            credentials_tokens.refresh_token,
                            LastNetErrorAt::now(),
                        )
                    }
                }
            }
            PageEvents(
                st,
                credentials_tokens,
                page_token,
                mut events,
                refreshed_at,
                downloaded_at,
                refresh_type,
                pending_display_date,
            ) => {
                if let None = page_token {
                    let now = Local::now();
                    let new_today = Local::today().and_hms(0, 0, 0);
                    if new_today != today && new_today != pending_display_date.0 {
                        RefreshAuth(
                            st.into(),
                            credentials_tokens.refresh_token,
                            PendingDisplayDate(new_today),
                        )
                    } else {
                        today = new_today;
                        display_date = pending_display_date.0;
                        println!("PageEvents. before display {:?}", v_pos);
                        let pos_calculator =
                            |num_event_rows: GlyphYCnt, screen_height: GlyphYCnt| {
                                v_pos = new_pos(v_pos, num_event_rows, screen_height);
                                v_pos
                            };
                        renderer.display_events(
                            display_date.clone(),
                            events.finalise(),
                            refresh_type,
                            Now(now),
                            pos_calculator,
                        )?;
                        println!("PageEvents. after display {:?}", v_pos);
                        PollEvents(
                            st.into(),
                            credentials_tokens,
                            refreshed_at,
                            downloaded_at,
                            TimeUpdatedAt::now(),
                            pending_display_date,
                        )
                    }
                } else {
                    renderer.display_status(Status::NetworkPending, true)?;
                    match retriever.read(
                        &format!("Bearer {}", credentials_tokens.volatiles.access_token),
                        &pending_display_date.0,
                        &(pending_display_date.0 + chrono::Duration::days(1)
                            - chrono::Duration::seconds(1)),
                        &page_token,
                    ) {
                        Ok(mut resp) => {
                            let status = resp.status();
                            match status {
                                StatusCode::OK => {
                                    let events_resp: EventsResponse = resp.json()?;
                                    events.add(&events_resp)?;
                                    let page_token = match events_resp.next_page_token {
                                        None => None,
                                        Some(next_page) => Some(PageToken(next_page)),
                                    };

                                    PageEvents(
                                        st,
                                        credentials_tokens,
                                        page_token,
                                        events,
                                        refreshed_at,
                                        downloaded_at,
                                        refresh_type,
                                        pending_display_date,
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
                        Err(err) => {
                            eprintln!("Error refreshing: {:?}", err);
                            CachedDisplay(
                                st.into(),
                                credentials_tokens.refresh_token,
                                LastNetErrorAt::now(),
                            )
                        }
                    }
                }
            }
            PollEvents(
                st,
                credentials,
                refreshed_at,
                started_wait_at,
                time_updated_at,
                pending_display_date,
            ) => {
                let waiting_for = started_wait_at.as_ref().elapsed();
                let same_time_for = time_updated_at.as_ref().elapsed();
                let elapsed_since_token_refresh = refreshed_at.as_ref().elapsed();

                let seconds_since_refresh = elapsed_since_token_refresh.as_secs();
                renderer.display_status(
                    Status::AllOk,
                    if (seconds_since_refresh & 2) == 2 {
                        true
                    } else {
                        false
                    },
                )?;

                if seconds_since_refresh + PREEMPTIVE_REFRESH_OFFSET_MINS.as_secs()
                    >= credentials.volatiles.expires_in
                {
                    RefreshAuth(st.into(), credentials.refresh_token, pending_display_date)
                } else {
                    thread::sleep(BUTTON_POLL_PERIOD);
                    let reset_event = reset_button.event(&mut gpio)?;
                    let back_event = back_button.event(&mut gpio)?;
                    let next_event = next_button.event(&mut gpio)?;
                    let scroll_event = scroll_button.event(&mut gpio)?;

                    let short_check = |e: &LongButtonEvent| e.is_short_press();
                    let release_check = |e: &LongButtonEvent| e.is_release();
                    let long_check = |e: &LongButtonEvent| e.is_long_press();

                    if opt_filter(&reset_event, long_check) {
                        RequestCodes(st.into())
                    } else if opt_filter(&reset_event, short_check) {
                        shutdown()?;
                        PollEvents(
                            st,
                            credentials,
                            refreshed_at,
                            started_wait_at,
                            time_updated_at,
                            pending_display_date,
                        )
                    } else if opt_filter(&scroll_event, long_check) {
                        println!("full display & date refresh");
                        ReadFirstEvents(
                            st.into(),
                            credentials,
                            refreshed_at,
                            RefreshType::Full,
                            PendingDisplayDate(Local::today().and_hms(0, 0, 0)),
                        )
                    } else if opt_filter(&scroll_event, short_check) {
                        println!("PollEvents. before scroll v_pos: {:?}", v_pos);
                        v_pos = GlyphYCnt(v_pos.0 + V_POS_INC).into();
                        let pos_calculator =
                            |num_event_rows: GlyphYCnt, screen_height: GlyphYCnt| {
                                v_pos = new_pos(v_pos, num_event_rows, screen_height);
                                v_pos
                            };
                        renderer.scroll_events(Now(Local::now()), pos_calculator)?;
                        println!("PollEvents. after scroll v_pos: {:?}", v_pos);
                        PollEvents(
                            st.into(),
                            credentials,
                            refreshed_at,
                            started_wait_at,
                            time_updated_at,
                            pending_display_date,
                        )
                    } else if waiting_for >= RECHECK_PERIOD {
                        println!("full display refresh due");
                        ReadFirstEvents(
                            st.into(),
                            credentials,
                            refreshed_at,
                            RefreshType::Full,
                            pending_display_date,
                        )
                    } else if same_time_for >= TIME_UPDATE_PERIOD {
                        println!("time update due");
                        let pos_calculator =
                            |_num_event_rows: GlyphYCnt, _screen_height: GlyphYCnt| v_pos;
                        renderer.scroll_events(Now(Local::now()), pos_calculator)?;
                        PollEvents(
                            st.into(),
                            credentials,
                            refreshed_at,
                            started_wait_at,
                            TimeUpdatedAt::now(),
                            pending_display_date,
                        )
                    } else if opt_filter(&back_event, release_check)
                        || opt_filter(&next_event, release_check)
                    {
                        v_pos = GLYPH_Y_ORIGIN.clone().into();
                        println!(
                            "partial display refresh after date change. v_pos: {:?}",
                            v_pos
                        );
                        ReadFirstEvents(
                            st.into(),
                            credentials,
                            refreshed_at,
                            RefreshType::Partial,
                            pending_display_date,
                        )
                    } else if opt_filter(&back_event, short_check) {
                        let new_display_date = pending_display_date.0 - chrono::Duration::days(1);
                        renderer.refresh_date(&new_display_date)?;
                        PollEvents(
                            st,
                            credentials,
                            refreshed_at,
                            started_wait_at,
                            time_updated_at,
                            PendingDisplayDate(new_display_date),
                        )
                    } else if opt_filter(&next_event, short_check) {
                        let new_display_date = pending_display_date.0 + chrono::Duration::days(1);
                        renderer.refresh_date(&new_display_date)?;
                        PollEvents(
                            st,
                            credentials,
                            refreshed_at,
                            started_wait_at,
                            time_updated_at,
                            PendingDisplayDate(new_display_date),
                        )
                    } else {
                        PollEvents(
                            st,
                            credentials,
                            refreshed_at,
                            started_wait_at,
                            time_updated_at,
                            pending_display_date,
                        )
                    }
                }
            }
            CachedDisplay(st, refresh_token, net_error_at) => {
                let pos_calculator = |_num_event_rows: GlyphYCnt, _screen_height: GlyphYCnt| v_pos;
                renderer.scroll_events(Now(Local::now()), pos_calculator)?;
                NetworkOutage(
                    st.into(),
                    refresh_token,
                    net_error_at,
                    TimeUpdatedAt(*TimeUpdatedAt::now().as_ref() - TIME_UPDATE_PERIOD),
                )
            }
            NetworkOutage(st, refresh_token, net_error_at, time_updated_at) => {
                let elapsed_since_outage = net_error_at.as_ref().elapsed();
                let seconds_since_outage = elapsed_since_outage.as_secs();
                renderer.display_status(
                    Status::NetworkDown,
                    if (seconds_since_outage & 2) == 2 {
                        true
                    } else {
                        false
                    },
                )?;

                if (seconds_since_outage & 8) == 8 {
                    RefreshAuth(st.into(), refresh_token, PendingDisplayDate(display_date))
                } else {
                    thread::sleep(BUTTON_POLL_PERIOD);
                    let reset_event = reset_button.event(&mut gpio)?;
                    let scroll_event = scroll_button.event(&mut gpio)?;

                    let short_check = |e: &LongButtonEvent| e.is_short_press();
                    let long_check = |e: &LongButtonEvent| e.is_long_press();

                    if opt_filter(&reset_event, long_check) {
                        println!("network outage. auth reset event");
                        RequestCodes(st.into())
                    } else if opt_filter(&reset_event, short_check) {
                        println!("network outage. shutdown event");
                        shutdown()?;
                        NetworkOutage(st, refresh_token, net_error_at, time_updated_at)
                    } else if opt_filter(&scroll_event, short_check) {
                        println!("NetworkOutage. before scroll. v_pos: {:?}", v_pos);
                        v_pos = GlyphYCnt(v_pos.0 + V_POS_INC).into();
                        let pos_calculator =
                            |num_event_rows: GlyphYCnt, screen_height: GlyphYCnt| {
                                v_pos = new_pos(v_pos, num_event_rows, screen_height);
                                println!("pos_calculator. v_pos {:?} num_event_rows {:?} screen_height {:?} v_pos {:?}", v_pos, num_event_rows, screen_height, v_pos);
                                v_pos
                            };
                        renderer.scroll_events(Now(Local::now()), pos_calculator)?;
                        println!("NetworkOutage. after scroll. v_pos: {:?}", v_pos);
                        NetworkOutage(st.into(), refresh_token, net_error_at, time_updated_at)
                    } else {
                        NetworkOutage(st, refresh_token, net_error_at, time_updated_at)
                    }
                }
            }
            ErrorWait(st, started_wait_at) => {
                let waiting_for = started_wait_at.as_ref().elapsed();
                if waiting_for >= RECHECK_PERIOD {
                    LoadAuth(st.into())
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
