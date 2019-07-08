use crate::{
    cal_machine::retriever::{self, EventsResponse},
    copyable, err, stm,
};
use chrono::{format::ParseError, prelude::*};
use std::cmp::Ordering;
use Machine::*;

stm!(ev_stm, Machine, [] => Uninitialised(), {
        [Uninitialised] => OneCreator(Email);
        [OneCreator] => NotOneCreator()
    });

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodMarker {
    Start,
    End,
}

impl PeriodMarker {
    pub fn select(
        &self,
        date_time: &Option<String>,
        date: &Option<String>,
    ) -> Result<DateTime<Local>, Error> {
        if let Some(inner_date_time) = date_time {
            Ok(inner_date_time.parse()?)
        } else if let Some(inner_date) = date {
            Ok(inner_date.parse()?)
        } else {
            Err(MissingDateTimeError(*self).into())
        }
    }
}

copyable!(MissingDateTimeError, PeriodMarker);

err!(Error {
        Chrono(ParseError),
        MissingDateTime(MissingDateTimeError)
    });

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Email(pub String);

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

impl From<&retriever::Event> for Result<Event, Error> {
    fn from(ev: &retriever::Event) -> Result<Event, Error> {
        Ok(Event {
            summary: ev.summary.to_string(),
            description: ev.description.clone(),
            start: PeriodMarker::Start.select(&ev.start.date_time, &ev.start.date)?,
            end: PeriodMarker::End.select(&ev.end.date_time, &ev.end.date)?,
        })
    }
}

pub struct Appointments {
    pub events: Vec<Event>,
    state: Machine,
}

impl Appointments {
    pub fn new() -> Appointments {
        Appointments {
            events: Vec::new(),
            state: Uninitialised(ev_stm::Uninitialised),
        }
    }

    pub fn email(&self) -> Option<Email> {
        match self.state {
            Uninitialised(_) => None,
            OneCreator(_, ref email) => Some(email.clone()),
            NotOneCreator(_) => None,
        }
    }

    pub fn add(&mut self, received: &EventsResponse) -> Result<(), Error> {
        use std::mem::replace;

        let mut state = replace(&mut self.state, Uninitialised(ev_stm::Uninitialised));
        for ev in received.items.iter() {
            state = match state {
                Uninitialised(st) => OneCreator(st.into(), Email(ev.creator.email.clone())),
                OneCreator(st, Email(email)) => {
                    if email == ev.creator.email {
                        OneCreator(st, Email(email))
                    } else {
                        NotOneCreator(st.into())
                    }
                }
                NotOneCreator(st) => NotOneCreator(st),
            };
            let ev_res: Result<Event, Error> = ev.into();
            let typed_ev = ev_res?;
            self.events.push(typed_ev);
        }
        replace(&mut self.state, state);

        Ok(())
    }
}
