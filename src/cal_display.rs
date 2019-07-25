use crate::display::Operation as Op;
use crate::{
    cal_machine::evs::{
        Appointments, ArgumentOutOfRange, DisplayableOccasion, Email, Error as EventError, Minute,
        Now, TIME_FORMAT,
    },
    cloneable,
    display::{Error as DisplayError, PartialUpdate, Pos, RenderPipeline},
    err,
    formatter::{self, Dims, GlyphXCnt, GlyphYCnt, LeftFormatter},
    stm,
};
use Machine::*;

use chrono::prelude::*;
use core::{cmp::Ordering, fmt::Debug};

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

const NO_EMAIL: &str = "Email unknown";
const END_DELIMITER: &str = " ";
const IN_PROGRESS_DELIMITER: &str = "<";

const STATUS_FLASH_OFF: &str = " ";

stm!(appointment_stm, Machine, []=> Before(), {
    [Before] => InProgress();
    [Before, InProgress] => After();
    [InProgress, After] => Error()
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
    status: Status,
    pulse_on: bool,
    formatter: LeftFormatter,
    dims: Dims,
    events: Option<EventContent>,
}

struct EventContent {
    date: DateTime<Local>,
    apps: Appointments,
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
                Status::AllOk => "!",
                Status::NetworkDown => "i",
                Status::NetworkPending => "G",
            }
        }
    }
}
/*
pub struct DatedDescription {
    pub desc: String,
    pub start: StartDate,
    pub end: EndDate,
}

impl DatedDescription {
    pub fn new(start: &StartDate, end: &EndDate, desc: String) -> DatedDescription {
        DatedDescription {
            desc,
            start: start.clone(),
            end: end.clone(),
        }
    }

    pub fn description(&self) -> &str {
        &self.desc
    }
}
*/
cloneable!(EventDescription, String);
cloneable!(NowDescription, String);

enum DisplayAction {
    Event,
    TimeAndEvent,
    EventAndTime,
}

#[derive(Debug)]
enum DisplayRecord {
    Event(EventDescription),
    TimeAndEvent(NowDescription, EventDescription),
    EventAndTime(EventDescription, NowDescription),
}

struct MutableMachineWrapper {
    mach_opt: Option<Machine>,
}

impl Renderer {
    pub fn new() -> Result<Renderer, Error> {
        Ok(Renderer {
            pipe: RenderPipeline::new()?,
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
        println!(
            "format: compared: {:?} with {:?}. Result: {:?}",
            event, now, ordering
        );
        if let Some(Ordering::Equal) = ordering {
            event_str.push_str(IN_PROGRESS_DELIMITER);
        } else {
            event_str.push_str(END_DELIMITER);
        }

        event_str.push_str(&event.description());
        event_str.push('\n');

        (ordering, event_str)
    }

    pub fn clear(&mut self) -> Result<(), Error> {
        self.events = None;
        let mut ops: Vec<Op> = Vec::with_capacity(1);
        ops.push(Op::Clear);
        self.pipe.send(ops.iter(), false)?;
        Ok(())
    }

    pub fn display_status(&mut self, status: Status, on: bool) -> Result<(), Error> {
        if on == self.pulse_on && self.status == status {
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
        Ok(())
    }

    pub fn display_user_code(
        &mut self,
        user_code: &str,
        expires_at: &DateTime<Local>,
        url: &str,
    ) -> Result<(), Error> {
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

        Ok(())
    }

    pub fn refresh_date(&mut self, date: &DateTime<Local>) -> Result<(), Error> {
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

    fn render_events(
        &mut self,
        render_type: RefreshType,
        now: Now,
        mut pos_calculator: impl FnMut(GlyphYCnt, GlyphYCnt) -> GlyphYCnt,
    ) -> Result<(), Error> {
        if let Some(ref content) = self.events {
            let mut ops: Vec<Op> = Vec::with_capacity(6);
            /*
            if content.apps.events.len() == 0 {
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
            } else { */
            println!("render_events. now: {:?}", now);

            let mut events = content.apps.events.clone();
            events.sort();

            let display_date_start = Minute::new(&Now(content
                .date
                .with_hour(0)
                .and_then(|t| t.with_minute(0))
                .ok_or(Error::ArgumentOutOfRange(ArgumentOutOfRange()))?))?;

            let following_date_start = Minute::new(&Now(
                *display_date_start.time.as_ref() + chrono::Duration::days(1)
            ))?;

            let mut mach = Before(appointment_stm::Before);

            if let Some(Ordering::Greater) = display_date_start.partial_chron_cmp(&now) {
                mach = if let Before(st) = mach {
                    After(st.into())
                } else {
                    mach
                };
            };

            let wrapper = MutableMachineWrapper {
                mach_opt: Some(mach),
            };

            //let mut busy_now = false;
            let num_events = events.len();
            let joined = events
                    .iter()
                    .enumerate()
                    .scan(wrapper, |wrapper, (idx, ev)| {
                        println!("before event vs now comparison: now {:?}", now);
                        let (partial_ordering, ev_displayable) = Renderer::format(ev, &now);

                        let mut display_action = DisplayAction::Event;
                        let mach_opt = wrapper.mach_opt.take();
                        wrapper.mach_opt = if let Some(mach) = mach_opt {
                            Some(match mach {
                                Before(st) => match partial_ordering {
                                    Some(Ordering::Less) => {
                                        if idx + 1 == num_events {
                                            if let Some(Ordering::Greater) = following_date_start.partial_chron_cmp(&now) {

                                                println!("will append time at end");
                                                display_action = DisplayAction::EventAndTime;
                                                After(st.into())
                                            } else {
                                                Before(st)
                                            }
                                        } else {
                                            Before(st)
                                        }
                                    }
                                    Some(Ordering::Equal) => InProgress(st.into()),
                                    Some(Ordering::Greater) => {
                                        println!("will insert time before: {:?}", ev);
                                        display_action = DisplayAction::TimeAndEvent;
                                        After(st.into())
                                    }
                                    None => Before(st),
                                },
                                InProgress(st) => match partial_ordering {
                                    Some(Ordering::Less) => {
                                        eprintln!(
                                            "overlapping events in cal_display from InProgress when less: {:?}",
                                            content.date);
                                        Error(st.into())
                                    },
                                    Some(Ordering::Equal) => InProgress(st),
                                    Some(Ordering::Greater) => After(st.into()),
                                    None => InProgress(st),
                                },
                                After(st) => match partial_ordering {
                                    Some(Ordering::Less) => {
                                        eprintln!(
                                            "overlapping events in cal_display from After when less: {:?}",
                                            content.date
                                        );
                                        Error(st.into())
                                    },
                                    Some(Ordering::Equal) => {
                                        eprintln!(
                                            "overlapping events in cal_display from After when equal: {:?}",
                                            content.date);
                                        Error(st.into())
                                    },
                                    Some(Ordering::Greater) => After(st),
                                    None => After(st),
                                },
                                Error(st) => {
                                    Error(st)
                                }
                            })
                        } else {
                            None
                        };

                        let time_displayable: String=match Minute::new(&now)
                            .or_else(|e| Err(Error::from(e)))
                            .and_then(|n|self.formatter.just(&Renderer::format(&n, &now).1)
                                      .or_else(|e| Err(Error::from(e))))
                             {
                                Ok(just_time) => just_time,
                                Err(error) => return Some(Err(error))
                            };

                        let result=match self.formatter.just(&ev_displayable) {
                            Ok(formatted) => {
                                let event = EventDescription(formatted);
                                match display_action {
                                    DisplayAction::Event => Some(Ok(DisplayRecord::Event(event))),
                                    DisplayAction::EventAndTime => {
                                        Some(Ok(DisplayRecord::EventAndTime(
                                            event,
                                            NowDescription(time_displayable),
                                        )))
                                    }
                                    DisplayAction::TimeAndEvent => {
                                        Some(Ok(DisplayRecord::TimeAndEvent(
                                            NowDescription(time_displayable),
                                            event,
                                        )))
                                    }
                                }
                            }
                            Err(error) => return Some(Err(error.into())),
                        };
                        println!("result of scan: {:?}", result);
                        result
                    })
                    .flat_map(|action| match action {
                        Ok(DisplayRecord::EventAndTime(event, now)) => {
                            vec![Ok(event.as_ref().clone()), Ok(now.as_ref().clone())].into_iter()
                        }
                        Ok(DisplayRecord::TimeAndEvent(now, event)) => {
                            vec![Ok(now.as_ref().clone()), Ok(event.as_ref().clone())].into_iter()
                        }
                        Ok(DisplayRecord::Event(event)) => {
                            vec![Ok(event.as_ref().clone())].into_iter()
                        }
                        Err(error) => vec![Err(error)].into_iter(),
                    })
                    .collect::<Result<Vec<String>, Error>>()?
                    .join("\n");
            println!("joined: {:?}", joined);
            let lines = joined.lines().collect::<Vec<&str>>();
            println!("lines: {:?}", lines);
            let pos = pos_calculator(GlyphYCnt(lines.len()), self.dims.1);
            let justified_events = lines[pos.0..].join("\n");
            println!("justified events: {:?}", justified_events);

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
            //}

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
        } else {
            println!("no events");
            //Err(Error::InvalidState(InvalidStateError("self.render_events should have been the last rendering optation to have been invoked")))
        }
        Ok(())
    }

    pub fn display_events(
        &mut self,
        date: DateTime<Local>,
        apps: Appointments,
        render_type: RefreshType,
        now: Now,
        pos_calculator: impl FnMut(GlyphYCnt, GlyphYCnt) -> GlyphYCnt,
    ) -> Result<(), Error> {
        let content = EventContent { date, apps };
        self.events = Some(content);
        self.render_events(render_type, now, pos_calculator)
    }
}
