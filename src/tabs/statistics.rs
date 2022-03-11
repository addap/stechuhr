// generate a csv file with user work times for a given series of months
//
// 1. choose a datetime from a datetime picker (use the time widget for now)
// 2. db query that
// 2.1 filters events from that timeframe
// 2.2 groups by staff uuid
// 2.3 orders according to increasing timestamp
// 2.4 joins with staff table to add staff names
// 3. go through events and compute sum of timeslices between Working-Away pairs
// 4. dump the result in csv

use chrono::{Date, Duration, Local, Locale, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use iced::{button, Button, Column, Container, Element, Length, Row, Space, Text};
use iced_aw::{
    date_picker::{self, DatePicker},
    TabLabel,
};
use std::cmp::min;
use stechuhr::date_ext::NaiveDateExt;
use stechuhr::models::*;

use crate::{Message, SharedData, Tab};

pub struct StatsTab {
    date: Date<Local>,
    // widget states
    month_picker: date_picker::State,
    date_button_state: button::State,
    generate_button_state: button::State,
}

#[derive(Debug, Clone)]
pub enum StatsMessage {
    ChooseDate,
    CancelDate,
    SubmitDate(date_picker::Date),
    Generate,
}

type Secs = u32;
const SECS_PER_HOUR: Secs = 60 * 60;

enum DurationSMLabel {
    L9_22,
    L22_24,
    L24_9,
}

impl DurationSMLabel {
    /* Compute the number of seconds in one time period */
    fn to_duration_seconds(&self) -> Secs {
        match self {
            Self::L9_22 => (22 - 9) * SECS_PER_HOUR,
            Self::L22_24 => (24 - 22) * SECS_PER_HOUR,
            Self::L24_9 => 9 * SECS_PER_HOUR,
        }
    }

    /* Compute the first second of each time period */
    fn to_start_seconds(&self) -> Secs {
        match self {
            Self::L9_22 => 9 * SECS_PER_HOUR,
            Self::L22_24 => 22 * SECS_PER_HOUR,
            Self::L24_9 => 0 * SECS_PER_HOUR,
        }
    }

    /* Compute a label for a number of seconds between midnight and midnight of the following day */
    fn from_absolute_seconds(s: Secs) -> Self {
        assert!(s < 24 * SECS_PER_HOUR);

        if s < 9 * SECS_PER_HOUR {
            Self::L24_9
        } else if s < 22 * SECS_PER_HOUR {
            Self::L9_22
        } else {
            Self::L22_24
        }
    }
}

struct DurationSM {
    buckets: [Secs; 3],
    label: DurationSMLabel,
    current_seconds: Secs, /* offset within the current time period (only used at start if starting time is not aligned) */
}

impl DurationSM {
    /* Initialize a state machine from an initial seconds value to choose the starting label. */
    fn new(start_seconds: Secs) -> Self {
        assert!(start_seconds < 24 * SECS_PER_HOUR);
        let label = DurationSMLabel::from_absolute_seconds(start_seconds);
        let current_seconds = start_seconds - label.to_start_seconds();

        Self {
            buckets: [0, 0, 0],
            label,
            current_seconds,
        }
    }

    /* Advance to the next time period. */
    fn next_step(&mut self) {
        match self.label {
            DurationSMLabel::L9_22 => self.label = DurationSMLabel::L22_24,
            DurationSMLabel::L22_24 => self.label = DurationSMLabel::L24_9,
            DurationSMLabel::L24_9 => self.label = DurationSMLabel::L9_22,
        }
    }

    /* Returns the number of seconds in the current time period. */
    fn get_current_seconds(&self) -> Secs {
        self.label.to_duration_seconds() - self.current_seconds
    }

    /* Compute the number of time that can be added in the current time period and add it to the current bucket.
     * The time that can be added must be less or equal to the iven total number of seconds left. */
    fn add_time(&mut self, s: Secs) {
        match self.label {
            DurationSMLabel::L9_22 => self.buckets[0] += s,
            DurationSMLabel::L22_24 => self.buckets[1] += s,
            DurationSMLabel::L24_9 => self.buckets[2] += s,
        }
        self.current_seconds = 0;
    }

    /* Convert to a WorkDuration */
    fn to_work_duration(&self) -> WorkDuration {
        let [s1, s2, s3] = self.buckets;
        WorkDuration([
            Duration::seconds(s1 as i64),
            Duration::seconds(s2 as i64),
            Duration::seconds(s3 as i64),
        ])
    }
}

#[derive(Debug)]
struct WorkDuration([Duration; 3]);

impl WorkDuration {
    fn zero() -> Self {
        WorkDuration([Duration::zero(), Duration::zero(), Duration::zero()])
    }

    fn checked_add(&self, rhs: &Self) -> Option<Self> {
        let WorkDuration([t1, t2, t3]) = self;
        let WorkDuration([s1, s2, s3]) = rhs;

        Some(WorkDuration([
            s1.checked_add(t1).unwrap(),
            s2.checked_add(t2).unwrap(),
            s3.checked_add(t3).unwrap(),
        ]))
    }

    fn from_start_end_time(start_time: NaiveDateTime, end_time: NaiveDateTime) -> Self {
        // TODO ensure that naivedatetime is in correct timezone
        // 9 Uhr - 22 Uhr -> bucket 1
        // 22 Uhr - 24 Uhr -> bucket 2
        // 24 Uhr - 9 Uhr -> bucket 3
        //
        // like in os
        // compute total number of seconds in duration
        // get start seconds in day
        // while total_seconds > 0
        //   get seconds until next threshold
        //   put then into respective bucket
        //   subtract from total
        assert!(start_time < end_time);

        let current_seconds = start_time.num_seconds_from_midnight();
        let mut seconds_remaining =
            u32::try_from(end_time.signed_duration_since(start_time).num_seconds()).unwrap();
        let mut sm = DurationSM::new(current_seconds);

        while seconds_remaining > 0 {
            let s = min(seconds_remaining, sm.get_current_seconds());
            seconds_remaining -= s;
            sm.add_time(s);
            sm.next_step();
        }

        sm.to_work_duration()
    }

    fn num_minutes(&self) -> [i64; 4] {
        // TODO round up minutes
        let WorkDuration([t1, t2, t3]) = self;
        let minutes_1 = t1.num_minutes();
        let minutes_2 = t2.num_minutes();
        let minutes_3 = t3.num_minutes();
        let minutes_weigthed = (1.0 * (minutes_1 as f64)
            + 1.25 * (minutes_2 as f64)
            + 1.40 * (minutes_3 as f64)) as i64;

        [minutes_1, minutes_2, minutes_3, minutes_weigthed]
    }
}

enum EventSMLabel {
    Working(NaiveDateTime),
    Away,
}

struct EventSM {
    duration: WorkDuration,
    label: EventSMLabel,
}

impl EventSM {
    fn new() -> Self {
        Self {
            duration: WorkDuration::zero(),
            label: EventSMLabel::Away,
        }
    }

    fn add_time(&mut self, start_time: NaiveDateTime, end_time: NaiveDateTime) {
        let additional_work_time = WorkDuration::from_start_end_time(start_time, end_time);
        self.duration = self.duration.checked_add(&additional_work_time).unwrap();
    }

    fn process(&mut self, sm_uuid: i32, event: &WorkEventT) {
        match self.label {
            EventSMLabel::Away => match event.event {
                WorkEvent::StatusChange(uuid, _, WorkStatus::Working) => {
                    if sm_uuid == uuid {
                        self.label = EventSMLabel::Working(event.created_at)
                    }
                }
                _ => {}
            },
            EventSMLabel::Working(start_time) => match event.event {
                WorkEvent::StatusChange(uuid, _, WorkStatus::Away) if sm_uuid == uuid => {
                    self.add_time(start_time, event.created_at);
                    self.label = EventSMLabel::Away;
                }
                WorkEvent::EventOver => {
                    self.add_time(start_time, event.created_at);
                    self.label = EventSMLabel::Away;
                }
                _ => {}
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct PersonHours<'a> {
    #[serde(rename = "Name")]
    name: &'a str,
    #[serde(rename = "Minuten 9 - 22 Uhr")]
    minutes_1: i64,
    #[serde(rename = "Minuten 22 - 24 Uhr")]
    minutes_2: i64,
    #[serde(rename = "Minuten 24 - 9 Uhr")]
    minutes_3: i64,
    #[serde(rename = "Minuten Gewichtet Total")]
    minutes_weigthed: i64,
}

impl StatsTab {
    pub fn new() -> Self {
        StatsTab {
            date: Local::today(),
            month_picker: date_picker::State::now(),
            date_button_state: button::State::default(),
            generate_button_state: button::State::default(),
        }
    }

    pub fn update(&mut self, shared: &mut SharedData, message: StatsMessage) {
        match message {
            StatsMessage::ChooseDate => {
                self.month_picker.reset();
                self.month_picker.show(true);
            }
            StatsMessage::CancelDate => {
                self.month_picker.show(false);
            }
            StatsMessage::SubmitDate(date) => {
                let naive_date = NaiveDate::from(date);
                // TODO better way to get the current offset? I should be able to just specify the "Europe/Berlin" Timezone and get it from there
                let offset = *Local::now().offset();
                self.date = Date::from_utc(naive_date, offset);
                self.month_picker.show(false);
            }
            StatsMessage::Generate => {
                // start and end time will be first and last day of the selected month, respectively
                let _9am = NaiveTime::from_hms(9, 0, 0);
                let start_time = self.date.naive_local().first_dom().and_time(_9am);
                let end_time = self.date.naive_local().last_dom().succ().and_time(_9am);
                println!(
                    "Generating CSV for month {}, between {} and {}",
                    self.date.format("%B %Y").to_string(),
                    start_time,
                    end_time
                );

                let events = stechuhr::load_events(start_time, end_time, &shared.connection);

                let staff_hours: Vec<PersonHours> = shared
                    .staff
                    .iter()
                    // associate with each staff member a WorkDuration, which counts the weighted minutes of work time
                    .map(|staff_member| {
                        let mut event_sm = EventSM::new();

                        for event in &events {
                            event_sm.process(staff_member.uuid(), event);
                        }

                        // sanity check that during evaluation, no staff member is still working
                        match event_sm.label {
                            EventSMLabel::Working(_) => {
                                panic!("Staff still working");
                            }
                            _ => {}
                        }

                        (staff_member, event_sm.duration)
                    })
                    // transform the calculated WorkDuration into a PersonHours struct for serialization
                    .map(|(staff_member, t)| {
                        let name = staff_member.name.as_ref();
                        let [minutes_1, minutes_2, minutes_3, minutes_weigthed] = t.num_minutes();

                        PersonHours {
                            name,
                            minutes_1,
                            minutes_2,
                            minutes_3,
                            minutes_weigthed,
                        }
                    })
                    .collect();

                let filename = format!(
                    "{}.csv",
                    self.date
                        .format_localized("%B-%Y", Locale::de_DE)
                        .to_string()
                );
                let mut wtr = csv::Writer::from_path(filename).unwrap();
                for hours in &staff_hours {
                    wtr.serialize(hours).unwrap();
                }
                wtr.flush().unwrap();
                println!("ok");
            }
        }
    }
}

impl<'a: 'b, 'b> Tab<'a, 'b> for StatsTab {
    fn title(&self) -> String {
        String::from("Auswertung")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.title())
    }

    fn content(&mut self, _shared: &mut SharedData) -> Element<'_, Message> {
        let dummy = Space::new(Length::Fill, Length::Fill);
        let datepicker = DatePicker::new(
            &mut self.month_picker,
            dummy,
            StatsMessage::CancelDate,
            StatsMessage::SubmitDate,
        );

        let content: Element<'_, StatsMessage> = Container::new(
            Row::new().push(datepicker).push(
                Column::new()
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .push(Text::new(
                        self.date
                            .format_localized("%B %Y", Locale::de_DE)
                            .to_string(),
                    ))
                    .push(
                        Button::new(&mut self.date_button_state, Text::new("Datum auswählen"))
                            .on_press(StatsMessage::ChooseDate),
                    )
                    .push(
                        Button::new(&mut self.generate_button_state, Text::new("CSV Generieren"))
                            .on_press(StatsMessage::Generate),
                    ),
            ),
        )
        .padding(20)
        .into();

        content.map(Message::Statistics)
    }
}
