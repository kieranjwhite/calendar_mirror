use crate::display::Operation as Op;
use crate::{
    cal_machine::evs::{Appointments, Email, EndDate, Event, Now, StartDate},
    display::{Error as DisplayError, PartialUpdate, Pos, RenderPipeline},
    err,
    formatter::{self, Dims, GlyphXCnt, GlyphYCnt, LeftFormatter},
};
use chrono::prelude::*;

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
const TIME_FORMAT: &str = "%H:%M";

const NO_EMAIL: &str = "Email unknown";
const NO_EVENTS: &str = "No events";
const START_DELIMITER: &str = "-";
const END_DELIMITER: &str = " ";
const IN_PROGRESS_DELIMITER: &str = "<";
const SUMMARY_DELIMITER: &str = "";

const STATUS_FLASH_OFF: &str = " ";

const TIME_LEN: usize = 5;

#[derive(Debug)]
pub struct InvalidStateError(&'static str);

err!(Error {
    Display(DisplayError),
    Format(formatter::Error),
    InvalidState(InvalidStateError)
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

pub struct DatedDescription {
    pub desc: String,
    pub start: StartDate,
    pub end: EndDate,
}

impl DatedDescription {
    pub fn new(start: &StartDate, end: &EndDate, desc: String) -> DatedDescription {
        DatedDescription { desc, start:start.clone(), end:end.clone() }
    }

    pub fn description(&self) -> &str {
        &self.desc
    }
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

    fn format(event: &Event, now: &Now) -> String {
        let mut event_str = String::with_capacity(
            TIME_LEN
                + START_DELIMITER.len()
                + TIME_LEN
                + END_DELIMITER.len()
                + event.summary.len()
                + SUMMARY_DELIMITER.len()
                + 1,
        );

        event_str.push_str(&event.start.0.format(TIME_FORMAT).to_string());
        event_str.push_str(START_DELIMITER);
        event_str.push_str(&event.end.format(TIME_FORMAT).to_string());

        if event.in_progress(now) {
            event_str.push_str(IN_PROGRESS_DELIMITER);
        } else {
            event_str.push_str(END_DELIMITER);
        }
        
        event_str.push_str(&event.summary);
        event_str.push_str(SUMMARY_DELIMITER);
        event_str.push('\n');

        event_str
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
            } else {
                let mut events = content.apps.events.clone();
                events.sort();

                let evs: Vec<DatedDescription> = events
                    .iter()
                    .map(|ev| {
                        Ok(DatedDescription::new(&ev.start, &ev.end, self.formatter.just(&Renderer::format(&ev, &now))?))
                    })
                    .collect::<Result<Vec<DatedDescription>, Error>>()?;

                let joined=evs.iter().map(|ev| ev.description()).collect::<Vec<&str>>().join("\n");
                let lines= joined.lines().collect::<Vec<&str>>();
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
            }

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
