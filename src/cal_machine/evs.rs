use crate::{
    cal_machine::retriever::{self, EventsResponse},
    cloneable, copyable, err, stm,
};
use chrono::{
    format::{strftime::StrftimeItems, DelayedFormat, ParseError},
    offset::LocalResult,
    prelude::*,
};
use std::cmp::Ordering;
use Machine::*;

err!(Error {
    Chrono(ParseError),
    MissingDateTime(MissingDateTimeError),
    TimeZoneInvalid(TimeZoneInvalidError),
    TimeZoneAmbiguous(TimeZoneAmbiguousError)
    });

stm!(ev_stm, Machine, [] => Uninitialised(), {
        [Uninitialised] => OneCreator(Email);
        [OneCreator] => NotOneCreator()
    });

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodMarker {
    Start(NaiveTime),
    End(NaiveTime),
}

impl PeriodMarker {
    pub fn select(
        &self,
        date_time: &Option<String>,
        date: &Option<String>,
    ) -> Result<DateTime<Local>, Error> {
        let time = match self {
            PeriodMarker::Start(time) => time,
            PeriodMarker::End(time) => time,
        };

        if let Some(inner_date_time) = date_time {
            Ok(inner_date_time.parse()?)
        } else if let Some(inner_date) = date {
            println!("inner_date: {:?}", inner_date);

            let date_only =
                NaiveDate::parse_from_str(inner_date, "%Y-%m-%d").expect("failed to parse");
            println!("{}", date_only);
            let date_time = NaiveDateTime::new(date_only, *time);
            let date_time_tz: DateTime<Local> = match Local.from_local_datetime(&date_time) {
                LocalResult::None => Err(TimeZoneInvalidError())?,
                LocalResult::Single(dt) => dt,
                LocalResult::Ambiguous(dt_1, dt_2) => Err(TimeZoneAmbiguousError((dt_1, dt_2)))?,
            };
            println!("date: {:?}", date_time_tz);

            Ok(date_time_tz)
        } else {
            Err(MissingDateTimeError(*self).into())
        }
    }
}

type TwoDateTimes = (DateTime<Local>, DateTime<Local>);
copyable!(MissingDateTimeError, PeriodMarker);
cloneable!(TimeZoneAmbiguousError, TwoDateTimes);
cloneable!(StartDate, DateTime<Local>);
cloneable!(EndDate, DateTime<Local>);
cloneable!(Now, DateTime<Local>);

macro_rules! delegate_date_time {
    ($outer_type:ident) => {
        impl $outer_type {
            pub fn format<'a>(&self, fmt: &'a str) -> DelayedFormat<StrftimeItems<'a>> {
                self.0.format(fmt)
            }

            pub fn cmp(&self, other: &Self) -> Ordering {
                self.0.cmp(&other.0)
            }

            pub fn cmp_date_time(&self, other: &DateTime<Local>) -> Ordering {
                self.0.cmp(&other)
            }
}
    };
}

delegate_date_time!(StartDate);
delegate_date_time!(EndDate);
delegate_date_time!(Now);

#[derive(Debug)]
pub struct TimeZoneInvalidError();

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Email(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    pub summary: String,
    pub description: Option<String>,
    pub all_consuming: bool, //if an event is all consuming it is treated as being something that a person will be occupied with between the start and end date and the display should indicate that. Most events are treated as all consuming. All-day events aren't.
    pub start: StartDate,
    pub end: EndDate,
}

impl Event {
    pub fn in_progress(&self, now: &Now) -> bool {
        return now.cmp_date_time(&self.start.0)!=Ordering::Less && now.cmp_date_time(&self.end.0)==Ordering::Less
    }
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
            all_consuming: !(ev.start.date_time.is_none() && ev.end.date_time.is_none()),
            start: StartDate(
                PeriodMarker::Start(NaiveTime::from_hms(0, 0, 0))
                    .select(&ev.start.date_time, &ev.start.date)?,
            ),
            end: EndDate(
                PeriodMarker::End(NaiveTime::from_hms(23, 59, 59))
                    .select(&ev.end.date_time, &ev.end.date)?,
            ),
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
