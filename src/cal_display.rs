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

const HEADING_SIZE: u32 = 16;
const EMAIL_SIZE: u32 = 10;
const EVENTS_SIZE: u32 = 16;

const DATE_FORMAT: &str = "%e %b";
const TIME_FORMAT: &str = "%H:%M";

const NO_EMAIL: &str = "Multiple accounts";
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

    pub fn display(&mut self, date: &DateTime<Local>, apps: &Appointments) -> Result<(), Error> {
        let mut ops: Vec<Op> = Vec::with_capacity(apps.events.len() + 4);
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
        ops.push(Op::WriteAll);

        self.pipe.send(ops.iter())?;

        Ok(())
    }
}
