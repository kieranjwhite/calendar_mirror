use crate::{
    cal_machine::retriever::{self, EventsResponse},
    cloneable, copyable, err, stm,
};
use chrono::{format::ParseError, offset::LocalResult, prelude::*, Duration};
use std::cmp::Ordering;
use Machine::*;

#[derive(Debug)]
pub struct ArgumentOutOfRange();

err!(Error {
    Chrono(ParseError),
    ArgumentOutOfRange(ArgumentOutOfRange),
    MissingDateTime(MissingDateTimeError),
    TimeZoneInvalid(TimeZoneInvalidError),
    TimeZoneAmbiguous(TimeZoneAmbiguousError)
    });

stm!(ev_stm, Machine, [] => Uninitialised(), {
        [Uninitialised] => OneCreator(Email) |end|;
        [OneCreator] => NotOneCreator() |end|
    });

pub const TIME_FORMAT: &str = "%H:%M";

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

#[derive(Debug)]
pub struct TimeZoneInvalidError();

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Email(pub String);

pub trait DisplayableOccasion {
    fn description(&self) -> String;
    fn period(&self) -> String;
    //fn in_progress(&self, now: &Now) -> bool;
    fn partial_chron_cmp(&self, other: &Now) -> Option<Ordering>;
    //fn all_consuming(&self) -> bool;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Minute {
    //pub label: String,
    pub time: Now,
    pub after: DateTime<Local>,
}

impl Minute {
    pub fn new(time: &Now) -> Result<Minute, Error> {
        let minute_start = Now(time
            .as_ref()
            .with_second(0)
            .and_then(|today| today.with_nanosecond(0))
            .ok_or(Error::ArgumentOutOfRange(ArgumentOutOfRange()))?);
        Ok(Minute {
            time: minute_start.clone(),
            after: *minute_start.as_ref() + Duration::minutes(1),
        })
    }
}

impl DisplayableOccasion for Minute {
    fn description(&self) -> String {
        "".to_string()
    }

    fn period(&self) -> String {
        self.time.as_ref().format(TIME_FORMAT).to_string() + "< < < "
    }

    fn partial_chron_cmp(&self, other: &Now) -> Option<Ordering> {
        if let Some(Ordering::Greater) = self.time.as_ref().partial_cmp(other.as_ref()) {
            Some(Ordering::Greater)
        } else {
            match self.after.partial_cmp(other.as_ref()) {
                Some(Ordering::Greater) => Some(Ordering::Equal),
                None => None,
                _ => Some(Ordering::Less)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    pub summary: String,
    pub description: Option<String>,
    pub all_consuming: bool, //if an event is all consuming it is treated as being something that a person will be occupied with between the start and end date and the display should indicate that. Most events are treated as all consuming. All-day events aren't.
    pub start: StartDate,
    pub end: EndDate,
}

impl DisplayableOccasion for Event {
    fn description(&self) -> String {
        self.summary.clone()
    }

    fn period(&self) -> String {
        let mut result = String::new();
        result.push_str(&self.start.as_ref().format(TIME_FORMAT).to_string());
        result.push_str("-");
        result.push_str(&self.end.as_ref().format(TIME_FORMAT).to_string());
        result
    }

    fn partial_chron_cmp(&self, other: &Now) -> Option<Ordering> {
        if self.all_consuming {
            match self.start.as_ref().cmp(other.as_ref()) {
                Ordering::Greater => Some(Ordering::Greater),
                _ => match self.end.as_ref().cmp(other.as_ref()) {
                    Ordering::Greater => Some(Ordering::Equal),
                    _ => Some(Ordering::Less),
                },
            }
        } else {
            None
        }
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.start.as_ref().cmp(other.start.as_ref()) {
            Ordering::Equal => match self.end.as_ref().cmp(other.end.as_ref()) {
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
