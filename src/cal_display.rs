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

use crate::display::Operation as Op;
use crate::{
    cal_machine::evs::{
        AppsReadonly, ArgumentOutOfRange, DisplayableOccasion, Email, Error as EventError, Minute,
        Now, TIME_FORMAT,
    },
    cloneable,
    display::{Error as DisplayError, PartialUpdate, Pos, RenderPipeline},
    err,
    formatter::{self, Dims, GlyphXCnt, GlyphYCnt, LeftFormatter},
    stm,
};
use AppMachine::*;
use DisplayMachine::*;

use chrono::prelude::*;
use core::{cmp::Ordering, fmt::Debug};
use log::{trace,error};
use std::iter::from_fn;

const HEADING_ID: &str = "heading";
const PULSE_ID: &str = "pulse";
const EMAIL_ID: &str = "email";
const EVENTS_ID: &str = "events";

const HEADING_POS: Pos = Pos(10, 0);
const PULSE_POS: Pos = Pos(0, 0);
const EMAIL_POS: Pos = Pos(96, 4);
const EVENTS_POS: Pos = Pos(0, 20);
const INSTR1_POS: Pos = Pos(24, 24);
const CODE_POS: Pos = Pos(64, 48);
const INSTR2_POS: Pos = Pos(20, 108);
const EXPIRY_POS: Pos = Pos(82, 122);

const LARGE_SIZE: u32 = 24;
const SMALL_SIZE: u32 = 12;
const INSTR_SIZE: u32 = 16;
const HEADING_SIZE: u32 = 16;
const PULSE_SIZE: u32 = 16;
const EMAIL_SIZE: u32 = 10;
const EVENTS_SIZE: u32 = 16;

const DATE_FORMAT: &str = "%e %b";
const NO_EVENTS: &str = "No events";
const NO_EMAIL: &str = "E-mail not listed";
const END_DELIMITER: &str = " ";
const IN_PROGRESS_DELIMITER: &str = "<";

const STATUS_FLASH_OFF: &str = " ";

stm!(machine display_stm, DisplayMachine, DisplayAtEnd, DisplayTerminals, [Empty, SaveWarning, UserCode, Events] => Unknown() |end|, {
    [Empty, UserCode, Events, Unknown] => SaveWarning() |end|;
    [Empty, SaveWarning, Events, Unknown] => UserCode()  |end|;
    [Empty, SaveWarning, UserCode, Unknown] => Events() |end|;
    [SaveWarning, UserCode, Events, Unknown] => Empty() |end|;
});

stm!(machine app_stm, AppMachine, AppAtEnd, AppTerminals,
     []=> Before(), {
         [Before] => InProgress();
         [Before, InProgress] => After();
         [Before, InProgress, After] => Chained() |end|;
         [InProgress, After] => Error() |end|
     });

#[derive(Debug)]
pub struct InvalidStateError(&'static str);

err!(Error {
    Events(EventError),
    Display(DisplayError),
    Format(formatter::Error),
    InvalidState(InvalidStateError),
    ArgumentOutOfRange(ArgumentOutOfRange)
});

const SCREEN_DIMS: Dims = Dims(GlyphXCnt(26), GlyphYCnt(9));

#[derive(PartialEq)]
pub enum RefreshType {
    Full,
    Partial,
}

pub struct Renderer {
    pipe: RenderPipeline,
    state: Option<DisplayMachine>,
    status: Status,
    pulse_on: bool,
    formatter: LeftFormatter,
    dims: Dims,
    events: Option<EventContent>,
}

struct EventContent {
    date: DateTime<Local>,
    apps: AppsReadonly,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Status {
    AllOk,
    NetworkDown,
    NetworkPending,
}

impl Status {
    pub fn repr(&self, flash_on: bool) -> &'static str {
        if !flash_on {
            STATUS_FLASH_OFF
        } else {
            match self {
                Status::AllOk => " ",
                Status::NetworkDown => "i",
                Status::NetworkPending => "G",
            }
        }
    }
}

cloneable!(EventDescription, String);
cloneable!(NowDescription, String);

enum DisplayAction {
    Event,
    TimeAndEvent,
}

#[derive(Debug)]
enum DisplayRecord {
    Event(EventDescription),
    Time(NowDescription),
    TimeAndEvent(NowDescription, EventDescription),
}

impl Renderer {
    pub fn new() -> Result<Renderer, Error> {
        Ok(Renderer {
            pipe: RenderPipeline::new()?,
            state: Some(DisplayMachine::new(
                (),
                Box::new(|mach: DisplayAtEnd| {
                    trace!("dropping DisplayMachine: {:?}", mach);

                    match mach {
                        DisplayAtEnd::Empty(st) => DisplayTerminals::Empty(st),
                        DisplayAtEnd::SaveWarning(st) => DisplayTerminals::SaveWarning(st),
                        DisplayAtEnd::UserCode(st) => DisplayTerminals::UserCode(st),
                        DisplayAtEnd::Events(st) => DisplayTerminals::Events(st),
                        DisplayAtEnd::Unknown(st) => DisplayTerminals::Unknown(st),
                    }
                }),
            )),
            status: Status::AllOk,
            pulse_on: false,
            formatter: LeftFormatter::new(SCREEN_DIMS),
            dims: SCREEN_DIMS,
            events: None,
        })
    }

    pub fn wait_for_server() -> Result<Renderer, Error> {
        RenderPipeline::wait_for_server()?;
        Renderer::new()
    }

    pub fn disconnect_quits_server(&mut self) -> Result<(), Error> {
        let mut ops: Vec<Op> = Vec::with_capacity(1);
        ops.push(Op::QuitWhenDone);
        self.pipe.send(ops.iter(), false)?;

        Ok(())
    }

    fn format<E: DisplayableOccasion + Debug>(event: &E, now: &Now) -> (Option<Ordering>, String) {
        let mut event_str = String::with_capacity(40);

        event_str.push_str(&event.period());

        let ordering = event.partial_chron_cmp(now);
        if let Some(Ordering::Equal) = ordering {
            event_str.push_str(IN_PROGRESS_DELIMITER);
        } else {
            event_str.push_str(END_DELIMITER);
        }

        event_str.push_str(&event.description());
        event_str.push('\n');

        (ordering, event_str)
    }

    fn unset_state(&mut self) {
        self.state = Some(
            match self
                .state
                .take()
                .expect("no state in Renderer.unset_state()")
            {
                Empty(st) => Unknown(st.into()),
                SaveWarning(st) => Unknown(st.into()),
                UserCode(st) => Unknown(st.into()),
                Events(st) => Unknown(st.into()),
                Unknown(st) => Unknown(st),
            },
        );
    }

    pub fn clear(&mut self) -> Result<(), Error> {
        self.unset_state();
        self.events = None;
        let mut ops: Vec<Op> = Vec::with_capacity(1);
        ops.push(Op::Clear);
        self.pipe.send(ops.iter(), false)?;

        self.state = Some(
            match self.state.take().expect("no state in Renderer.clear()") {
                Empty(st) => Empty(st),
                SaveWarning(st) => Empty(st.into()),
                UserCode(st) => Empty(st.into()),
                Events(st) => Empty(st.into()),
                Unknown(st) => Empty(st.into()),
            },
        );

        Ok(())
    }

    fn events_displayed(&mut self) -> bool {
        let mut displayed = false;
        match self
            .state
            .as_ref()
            .expect("no state in Renderer.events_displayed()")
        {
            Events(_) => displayed = true,
            _ => (),
        };

        return displayed;
    }

    pub fn display_status(&mut self, status: Status, on: bool) -> Result<(), Error> {
        if on == self.pulse_on && self.status == status {
            return Ok(());
        }

        if !self.events_displayed() {
            return Ok(());
        }

        self.pulse_on = on;
        self.status = status;

        let mut ops: Vec<Op> = Vec::with_capacity(3);
        ops.push(Op::UpdateText(
            PULSE_ID.to_string(),
            status.repr(self.pulse_on).to_string(),
        ));
        ops.push(Op::WriteAll(PartialUpdate(true)));

        self.pipe.send(ops.iter(), false)?;
        Ok(())
    }

    pub fn display_save_warning(&mut self) -> Result<(), Error> {
        self.unset_state();
        self.events = None;
        let mut ops: Vec<Op> = Vec::with_capacity(4);
        ops.push(Op::Clear);
        ops.push(Op::AddText(
            "SAVING!".to_string(),
            Pos(78, 48),
            LARGE_SIZE,
            "Code".to_string(),
        ));
        ops.push(Op::AddText(
            "Do not disconnect power.".to_string(),
            Pos(40, 108),
            SMALL_SIZE,
            "Instr2".to_string(),
        ));
        ops.push(Op::WriteAll(PartialUpdate(false)));

        self.pipe.send(ops.iter(), true)?;

        self.state = Some(
            match self
                .state
                .take()
                .expect("no state in Renderer.display_save_warning()")
            {
                Empty(st) => SaveWarning(st.into()),
                SaveWarning(st) => SaveWarning(st),
                UserCode(st) => SaveWarning(st.into()),
                Events(st) => SaveWarning(st.into()),
                Unknown(st) => SaveWarning(st.into()),
            },
        );

        Ok(())
    }

    pub fn display_user_code(
        &mut self,
        user_code: &str,
        expires_at: &DateTime<Local>,
        url: &str,
    ) -> Result<(), Error> {
        self.unset_state();
        self.events = None;
        let mut ops: Vec<Op> = Vec::with_capacity(6);
        ops.push(Op::Clear);
        ops.push(Op::AddText(
            "Please enter the code:".to_string(),
            INSTR1_POS,
            INSTR_SIZE,
            "Instr1".to_string(),
        ));
        ops.push(Op::AddText(
            user_code.to_string(),
            CODE_POS,
            LARGE_SIZE,
            "Code".to_string(),
        ));
        ops.push(Op::AddText(
            format!("at {}", url),
            INSTR2_POS,
            SMALL_SIZE,
            "Instr2".to_string(),
        ));
        ops.push(Op::AddText(
            format!("before {}", expires_at.format(TIME_FORMAT).to_string()),
            EXPIRY_POS,
            SMALL_SIZE,
            "Expiry".to_string(),
        ));
        ops.push(Op::WriteAll(PartialUpdate(false)));

        self.pipe.send(ops.iter(), false)?;

        self.state = Some(
            match self
                .state
                .take()
                .expect("no state in Renderer.display_user_mode()")
            {
                Empty(st) => UserCode(st.into()),
                SaveWarning(st) => UserCode(st.into()),
                UserCode(st) => UserCode(st),
                Events(st) => UserCode(st.into()),
                Unknown(st) => UserCode(st.into()),
            },
        );

        Ok(())
    }

    pub fn refresh_date(&mut self, date: &DateTime<Local>) -> Result<(), Error> {
        if !self.events_displayed() {
            return Ok(());
        }

        let mut ops: Vec<Op> = Vec::with_capacity(5);

        let heading = date.format(DATE_FORMAT).to_string();
        ops.push(Op::UpdateText(HEADING_ID.to_string(), heading));

        ops.push(Op::UpdateText(EMAIL_ID.to_string(), "".to_string()));
        ops.push(Op::UpdateText(EVENTS_ID.to_string(), "".to_string()));
        ops.push(Op::WriteAll(PartialUpdate(true)));

        self.pipe.send(ops.iter(), false)?;
        Ok(())
    }

    pub fn scroll_events(
        &mut self,
        now: Now,
        pos_calculator: impl FnMut(GlyphYCnt, GlyphYCnt) -> GlyphYCnt,
    ) -> Result<(), Error> {
        self.render_events(RefreshType::Partial, now, pos_calculator)
    }

    fn date_start(date: &DateTime<Local>) -> Result<DateTime<Local>, Error> {
        date.with_hour(0)
            .and_then(|date| date.with_minute(0))
            .and_then(|date| date.with_second(0))
            .and_then(|date| date.with_nanosecond(0))
            .ok_or(Error::ArgumentOutOfRange(ArgumentOutOfRange()))
    }

    fn render_events(
        &mut self,
        render_type: RefreshType,
        now: Now,
        mut pos_calculator: impl FnMut(GlyphYCnt, GlyphYCnt) -> GlyphYCnt,
    ) -> Result<(), Error> {
        if render_type == RefreshType::Partial {
            if !self.events_displayed() {
                return Ok(());
            }
        } else {
            self.unset_state();
        }

        let _all_displayable = if let Some(ref content) = self.events {
            let display_date = Renderer::date_start(&content.date)?;
            let today = Renderer::date_start(&now.as_ref())?;
            let mut ops: Vec<Op> = Vec::with_capacity(6);
            let _events_queued = if display_date != today && content.apps.events().len() == 0 {
                let displayable_events = NO_EVENTS.to_string();
                if render_type == RefreshType::Full {
                    ops.push(Op::Clear);
                    ops.push(Op::AddText(
                        displayable_events,
                        EVENTS_POS,
                        EVENTS_SIZE,
                        EVENTS_ID.to_string(),
                    ));
                } else {
                    ops.push(Op::UpdateText(EVENTS_ID.to_string(), displayable_events));
                }
            } else {
                let mut events = content.apps.events();
                events.sort();

                let display_date_start = Minute::new(&Now(display_date))?;

                let following_date_start = Minute::new(&Now(
                    *display_date_start.time.as_ref() + chrono::Duration::days(1)
                ))?;

                let time_displayable: String = self
                    .formatter
                    .just(&Renderer::format(&Minute::new(&now)?, &now).1)?;

                let mut app_mach = AppMachine::new(
                    (),
                    Box::new(|mach: AppAtEnd| loop {
                        trace!("dropping AppMachine: {:?}", mach);

                        match mach {
                            AppAtEnd::Chained(st) => return AppTerminals::Chained(st),
                            AppAtEnd::Error(st) => return AppTerminals::Error(st),
                            _ => {
                                panic!("Illegal AppMachine state: {:?}", mach);
                            }
                        };
                    }),
                );

                //Dealing with an event which straddles 2 days when we're displaying the 2nd of those days
                //which happens to be tomorrow
                if let Some(Ordering::Greater) = display_date_start.partial_chron_cmp(&now) {
                    app_mach = if let Before(st) = app_mach {
                        InProgress(st.into())
                    } else {
                        panic!(
                            "appointments state machine has the wrong starting state: {:?}",
                            app_mach.state()
                        );
                    };
                };
                let mut mach_opt = Some(app_mach);

                let displayed_events =
                    events
                    .iter()
                    .map(|ev| {
                        let (partial_ordering, ev_displayable) = Renderer::format(ev, &now);

                        let mut display_action = DisplayAction::Event;
                        mach_opt = Some(match mach_opt.take()
                                                .expect("missing state in appointments stm") {
                                Before(st) => match partial_ordering {
                                    Some(Ordering::Less) => {
                                        Before(st)
                                    }
                                    Some(Ordering::Equal) => InProgress(st.into()),
                                    Some(Ordering::Greater) => {
                                        display_action = DisplayAction::TimeAndEvent;
                                        After(st.into())
                                    }
                                    None => Before(st),
                                },
                                InProgress(st) => match partial_ordering {
                                    Some(Ordering::Less) => {
                                        error!(
                                            "overlapping events in cal_display from InProgress when less: {:?}",
                                            content.date);
                                        AppMachine::Error(st.into())
                                    },
                                    Some(Ordering::Equal) => InProgress(st),
                                    Some(Ordering::Greater) => After(st.into()),
                                    None => InProgress(st),
                                },
                                After(st) => match partial_ordering {
                                    Some(Ordering::Less) => {
                                        error!(
                                            "overlapping events in cal_display from After when less: {:?}",
                                            content.date
                                        );
                                        AppMachine::Error(st.into())
                                    },
                                    Some(Ordering::Equal) => {
                                        error!(
                                            "overlapping events in cal_display from After when equal: {:?}",
                                            content.date);
                                        AppMachine::Error(st.into())
                                    },
                                    Some(Ordering::Greater) => After(st),
                                    None => After(st),
                                },
                                Chained(st) => Chained(st),
                                AppMachine::Error(st) => AppMachine::Error(st),
                            });

                        let result=match self.formatter.just(&ev_displayable) {
                            Ok(formatted) => {
                                let event = EventDescription(formatted);
                                match display_action {
                                    DisplayAction::Event => Ok(DisplayRecord::Event(event)),
                                    DisplayAction::TimeAndEvent => {
                                        Ok(DisplayRecord::TimeAndEvent(
                                            NowDescription(time_displayable.clone()),
                                            event,
                                        ))
                                    }
                                }
                            }
                            Err(error) => return Err(error.into()),
                        };
                        result
                    }).collect::<Vec<Result<DisplayRecord,Error>>>();

                let joined = {
                    displayed_events
                        .into_iter()
                        .chain(from_fn(|| {
                            let mut out = None;

                            mach_opt = Some(
                                match mach_opt.take().expect("missing state in appointments stm") {
                                    Before(st) => {
                                        if let Some(Ordering::Greater) =
                                            following_date_start.partial_chron_cmp(&now)
                                        {
                                            out = Some(Ok(DisplayRecord::Time(NowDescription(
                                                time_displayable.clone(),
                                            ))));
                                            After(st.into())
                                        } else {
                                            Chained(st.into())
                                        }
                                    }
                                    InProgress(st) => Chained(st.into()),
                                    After(st) => Chained(st.into()),
                                    Chained(st) => Chained(st),
                                    Error(st) => Error(st),
                                },
                            );
                            //}
                            out
                        }))
                        .flat_map(|action| match action {
                            Ok(DisplayRecord::TimeAndEvent(now, event)) => {
                                vec![Ok(now.as_ref().clone()), Ok(event.as_ref().clone())]
                                    .into_iter()
                            }
                            Ok(DisplayRecord::Event(event)) => {
                                vec![Ok(event.as_ref().clone())].into_iter()
                            }
                            Ok(DisplayRecord::Time(now)) => {
                                vec![Ok(now.as_ref().clone())].into_iter()
                            }
                            Err(error) => vec![Err(error)].into_iter(),
                        })
                        .collect::<Result<Vec<String>, Error>>()?
                        .join("\n")
                };
                let lines = joined.lines().collect::<Vec<&str>>();
                let pos = pos_calculator(GlyphYCnt(lines.len()), self.dims.1);
                let justified_events = lines[pos.0..].join("\n");

                if render_type == RefreshType::Full {
                    ops.push(Op::Clear);

                    ops.push(Op::AddText(
                        justified_events,
                        EVENTS_POS,
                        EVENTS_SIZE,
                        EVENTS_ID.to_string(),
                    ));
                } else {
                    ops.push(Op::UpdateText(EVENTS_ID.to_string(), justified_events));
                }
            };

            let heading = content.date.format(DATE_FORMAT).to_string();
            let displayable_email = if let Some(Email(email_address)) = content.apps.email() {
                email_address
            } else {
                NO_EMAIL.to_string()
            };
            if render_type == RefreshType::Full {
                ops.push(Op::AddText(
                    heading,
                    HEADING_POS,
                    HEADING_SIZE,
                    HEADING_ID.to_string(),
                ));

                ops.push(Op::AddText(
                    STATUS_FLASH_OFF.to_string(),
                    PULSE_POS,
                    PULSE_SIZE,
                    PULSE_ID.to_string(),
                ));

                ops.push(Op::AddText(
                    displayable_email,
                    EMAIL_POS,
                    EMAIL_SIZE,
                    EMAIL_ID.to_string(),
                ));

                ops.push(Op::WriteAll(PartialUpdate(false)));
            } else {
                ops.push(Op::UpdateText(HEADING_ID.to_string(), heading));
                ops.push(Op::UpdateText(
                    PULSE_ID.to_string(),
                    STATUS_FLASH_OFF.to_string(),
                ));
                ops.push(Op::UpdateText(EMAIL_ID.to_string(), displayable_email));
                ops.push(Op::WriteAll(PartialUpdate(true)));
            }

            self.pipe.send(ops.iter(), false)?;
            self.state = Some(
                match self
                    .state
                    .take()
                    .expect("no state in Renderer.render_events()")
                {
                    Empty(st) => Events(st.into()),
                    SaveWarning(st) => Events(st.into()),
                    UserCode(st) => Events(st.into()),
                    Events(st) => Events(st),
                    Unknown(st) => Events(st.into()),
                },
            );
        } else {
            error!("no events");
        };

        Ok(())
    }

    pub fn display_events(
        &mut self,
        date: DateTime<Local>,
        apps: AppsReadonly,
        render_type: RefreshType,
        now: Now,
        pos_calculator: impl FnMut(GlyphYCnt, GlyphYCnt) -> GlyphYCnt,
    ) -> Result<(), Error> {
        let content = EventContent { date, apps };
        self.events = Some(content);
        self.render_events(render_type, now, pos_calculator)
    }
}
