use crate::display::Operation as Op;
use crate::{
    cal_machine::{evs::Appointments, Email, Event},
    display::*,
};
use chrono::prelude::*;

pub struct Renderer {
    pipe: RenderPipeline,
}

const HEADING_ID: &str = "heading";
const EMAIL_ID: &str = "email";
const EVENTS_ID: &str = "events";

const HEADING_POS: Pos = Pos(0, 0);
const EMAIL_POS: Pos = Pos(100, 4);
const EVENTS_POS: Pos = Pos(0, 24);
const INSTR1_POS: Pos = Pos(24, 24);
const CODE_POS: Pos = Pos(64, 48);
const INSTR2_POS: Pos = Pos(20, 108);
const EXPIRY_POS: Pos = Pos(82, 122);

const LARGE_SIZE: u32 = 24;
const SMALL_SIZE: u32 = 12;
const INSTR_SIZE: u32 = 16;
const HEADING_SIZE: u32 = 16;
const EMAIL_SIZE: u32 = 10;
const EVENTS_SIZE: u32 = 16;

const DATE_FORMAT: &str = "%e %b";
const TIME_FORMAT: &str = "%H:%M";

const NO_EMAIL: &str = "Email unknown";
const NO_EVENTS: &str = "No events";
const START_DELIMITER: &str = "-";
const END_DELIMITER: &str = " ";
const SUMMARY_DELIMITER: &str = "";
//const DESC_DELIMITER: &str = "> ";

const TIME_LEN: usize = 5;

impl Renderer {
    pub fn new() -> Result<Renderer, Error> {
        Ok(Renderer {
            pipe: RenderPipeline::new()?,
        })
    }

    pub fn wait_for_server() -> Result<Renderer, Error> {
        RenderPipeline::wait_for_server()?;
        Renderer::new()
    }

    pub fn disconnect_quits_server(&mut self) -> Result<(), Error> {
        let mut ops: Vec<Op> = Vec::with_capacity(1);
        ops.push(Op::QuitWhenDone);
        self.pipe.send(ops.iter())?;

        Ok(())
    }
    
    fn format(event: &Event) -> String {
        let mut event_str = String::with_capacity(
            TIME_LEN
                + START_DELIMITER.len()
                + TIME_LEN
                + END_DELIMITER.len()
                + event.summary.len()
                + SUMMARY_DELIMITER.len()
                //+ if let Some(ref desc) = event.description {
                //    desc.len()
                //} else {
                //    0
                //}
                //+ DESC_DELIMITER.len()
                + 1,
        );

        event_str.push_str(&event.start.format(TIME_FORMAT).to_string());
        event_str.push_str(START_DELIMITER);
        event_str.push_str(&event.end.format(TIME_FORMAT).to_string());
        event_str.push_str(END_DELIMITER);
        event_str.push_str(&event.summary);
        event_str.push_str(SUMMARY_DELIMITER);
        //if let Some(ref desc) = event.description {
        //    event_str.push_str(&desc);
        //}
        //event_str.push_str(DESC_DELIMITER);
        event_str.push('\n');

        event_str
    }

    pub fn display_user_code(
        &mut self,
        user_code: &str,
        expires_at: &DateTime<Local>,
        url: &str,
    ) -> Result<(), Error> {
        let mut ops: Vec<Op> = Vec::with_capacity(5);
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

        self.pipe.send(ops.iter())?;

        Ok(())
    }

    pub fn refresh_date(&mut self, date: &DateTime<Local>) -> Result<(), Error> {
        let mut ops: Vec<Op> = Vec::with_capacity(5);

        let heading = date.format(DATE_FORMAT).to_string();
        ops.push(Op::UpdateText(HEADING_ID.to_string(), heading));

        ops.push(Op::UpdateText(EMAIL_ID.to_string(), "".to_string()));
        ops.push(Op::UpdateText(EVENTS_ID.to_string(), "".to_string()));
        ops.push(Op::WriteAll(PartialUpdate(true)));

        self.pipe.send(ops.iter())
    }

    pub fn display_events(
        &mut self,
        date: &DateTime<Local>,
        apps: &Appointments,
    ) -> Result<(), Error> {
        let mut ops: Vec<Op> = Vec::with_capacity(5);
        ops.push(Op::Clear);

        let heading = date.format(DATE_FORMAT).to_string();
        ops.push(Op::AddText(
            heading,
            HEADING_POS,
            HEADING_SIZE,
            HEADING_ID.to_string(),
        ));

        if let Some(Email(email_address)) = apps.email() {
            ops.push(Op::AddText(
                email_address,
                EMAIL_POS,
                EMAIL_SIZE,
                EMAIL_ID.to_string(),
            ));
        } else {
            ops.push(Op::AddText(
                NO_EMAIL.to_string(),
                EMAIL_POS,
                EMAIL_SIZE,
                EMAIL_ID.to_string(),
            ));
        }

        if apps.events.len() == 0 {
            ops.push(Op::AddText(
                NO_EVENTS.to_string(),
                EVENTS_POS,
                EVENTS_SIZE,
                EVENTS_ID.to_string(),
            ));
        } else {
            let mut events = apps.events.clone();
            events.sort();

            let ev_strs: Vec<String> = events.iter().map(|ev| Renderer::format(&ev)).collect();
            let ev_ref_strs = &ev_strs;
            let event_len = ev_ref_strs.iter().fold(0, |s, ev| s + ev.len());
            let mut all_events = String::with_capacity(event_len);
            for ev in ev_strs {
                all_events.push_str(&ev);
            }

            ops.push(Op::AddText(
                all_events,
                EVENTS_POS,
                EVENTS_SIZE,
                EVENTS_ID.to_string(),
            ));
        }
        ops.push(Op::WriteAll(PartialUpdate(false)));

        self.pipe.send(ops.iter())
    }
}

