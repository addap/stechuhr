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

mod event_eval;
mod time_eval;

use std::{borrow::Cow, error, fmt};

use chrono::{
    naive::MIN_DATETIME, Date, Duration, Local, Locale, NaiveDate, NaiveDateTime, NaiveTime,
    TimeZone,
};
use iced::{button, Alignment, Button, Column, Container, Element, Length, Row, Text};
use iced_aw::{
    date_picker::{self, DatePicker},
    TabLabel,
};
use iced_native::Event;
use stechuhr::{
    date_ext::NaiveDateExt,
    models::{DBStaffMember, StaffMember, WorkEventT, WorkStatus},
};

use crate::{Message, SharedData, StechuhrError, Tab, TAB_PADDING};
use event_eval::{EventSM, PersonHours};
use stechuhr::{db, TEXT_SIZE_BIG};

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
    HandleEvent(Event),
}

#[derive(Debug, Serialize)]
struct PersonHoursCSV {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Minuten 6 - 22 Uhr")]
    minutes_1: i64,
    #[serde(rename = "Minuten 22 - 24 Uhr")]
    minutes_2: i64,
    #[serde(rename = "Minuten 24 - 6 Uhr")]
    minutes_3: i64,
}

impl<'a> From<PersonHours<'a>> for PersonHoursCSV {
    fn from(hours: PersonHours<'a>) -> Self {
        let [minutes_1, minutes_2, minutes_3] = hours.duration.num_minutes();

        Self {
            name: hours.staff_member.name.clone(),
            minutes_1,
            minutes_2,
            minutes_3,
        }
    }
}

#[derive(Debug)]
struct StaffHours {
    hours: Vec<PersonHoursCSV>,
    errors: Vec<SoftStatisticsError>,
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

    /// Create a EventSM state machine and feed all WorkEventT events to it to compute the StaffMemberHours.
    fn generate_hours_for_staff_member<'a>(
        events: &'a Vec<WorkEventT>,
        start_time: NaiveDateTime,
    ) -> impl 'a + Fn(&StaffMember) -> Result<(PersonHours, Vec<SoftStatisticsError>), StatisticsError>
    {
        move |staff_member| {
            let initial_start_time = if staff_member.status == WorkStatus::Working {
                Some(start_time)
            } else {
                None
            };

            let mut event_sm = EventSM::new(staff_member, initial_start_time);

            for event in events {
                event_sm.process(event)?;
            }

            Ok(event_sm.finish())
        }
    }

    fn evaluate_hours_for_month(
        shared: &mut SharedData,
        date: Date<Local>,
    ) -> Result<StaffHours, StechuhrError> {
        // start and end time will be first and last day of the selected month, respectively
        let _6am = NaiveTime::from_hms(6, 0, 0);
        let start_time = date.naive_local().first_dom().and_time(_6am);
        let end_time = date.naive_local().last_dom().succ().and_time(_6am);

        let start_time_local = Local.from_local_datetime(&start_time).unwrap();
        let end_time_local = Local.from_local_datetime(&end_time).unwrap();

        shared.log_info(format!(
            "Starte Auswertung für {}, zwischen {} und {}",
            date.format_localized("%B %Y", Locale::de_DE).to_string(),
            start_time_local
                .format_localized("%d. %B (%R)", Locale::de_DE)
                .to_string(),
            end_time_local
                .format_localized("%d. %B (%R)", Locale::de_DE)
                .to_string()
        ));

        StatsTab::evaluate_hours(shared, start_time, end_time)
    }

    fn evaluate_hours(
        shared: &SharedData,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
    ) -> Result<StaffHours, StechuhrError> {
        // Load events before the evaluation period in order to set the correct initial status for staff members.
        let previous_events = db::load_events_between(MIN_DATETIME, start_time, &shared.connection);
        let events = db::load_events_between(start_time, end_time, &shared.connection);

        // Set the initial status for staff members.
        // Atm we only do evaluation starting at 6am on the 1st of the month, so no one will be working as we set everyone to non-working at 6am.
        let evaluation_staff = shared
            .staff
            .iter()
            // turn everyone into DBStaffMember to forget the working status
            .map(|staff_member| DBStaffMember::from(Cow::Borrowed(staff_member)))
            // compute the initial status
            .map(|staff_member| {
                db::staff_member_compute_status(staff_member, &previous_events, start_time)
            })
            .collect::<Vec<_>>();

        StatsTab::evaluate_hours_for_events(evaluation_staff, events, start_time)
    }

    fn evaluate_hours_for_events(
        staff: Vec<StaffMember>,
        events: Vec<WorkEventT>,
        start_time: NaiveDateTime,
    ) -> Result<StaffHours, StechuhrError> {
        let (hours_raw, soft_errors): (Vec<PersonHours>, Vec<Vec<SoftStatisticsError>>) = staff
            .iter()
            // Associate with each staff member a WorkDuration, which counts the minutes of work time
            .map(StatsTab::generate_hours_for_staff_member(
                &events, start_time,
            ))
            .collect::<Result<Vec<(PersonHours, Vec<SoftStatisticsError>)>, StatisticsError>>()?
            .into_iter()
            .unzip();

        let hours: Vec<PersonHoursCSV> = hours_raw
            .into_iter()
            // transform the calculated WorkDuration into a PersonHours struct for serialization
            .map(PersonHoursCSV::from)
            .collect();

        Ok(StaffHours {
            hours,
            errors: soft_errors.into_iter().flatten().collect(),
        })
    }

    fn generate_csv(
        shared: &mut SharedData,
        date: Date<Local>,
        staff_hours: StaffHours,
    ) -> Result<(), StechuhrError> {
        // Write everyting into a CSV file.
        let filename = format!(
            "./auswertung/{}.csv",
            date.format_localized("%Y-%m %B", Locale::de_DE).to_string()
        );

        let mut wtr = csv::WriterBuilder::new()
            // enable flexible writer since errors are just one field
            .flexible(true)
            .from_path(&filename)?;

        for hours in &staff_hours.hours {
            wtr.serialize(hours)?;
        }
        for error in &staff_hours.errors {
            shared.log_error(error.to_string());
            // pad with units to put errors into a separate column
            wtr.serialize(((), (), (), (), (), (), error.to_string()))?;
        }
        wtr.flush()?;

        shared.prompt_message(format!(
            "Arbeitszeit wurde in der Datei {} gespeichert",
            filename,
        ));
        opener::open(filename)?;
        Ok(())
    }
}

impl Tab for StatsTab {
    type Message = StatsMessage;

    fn title(&self) -> String {
        String::from("Auswertung")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.title())
    }

    fn content(&mut self, _shared: &mut SharedData) -> Element<'_, Message> {
        let date = Container::new(
            Text::new(
                self.date
                    .format_localized("%B %Y", Locale::de_DE)
                    .to_string(),
            )
            .size(TEXT_SIZE_BIG),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y();

        let datepicker = DatePicker::new(
            &mut self.month_picker,
            date,
            StatsMessage::CancelDate,
            StatsMessage::SubmitDate,
        );

        let content = Row::new()
            .push(datepicker)
            .push(
                Container::new(
                    Column::new()
                        .push(
                            Button::new(&mut self.date_button_state, Text::new("Datum auswählen"))
                                .on_press(StatsMessage::ChooseDate),
                        )
                        .push(
                            Button::new(
                                &mut self.generate_button_state,
                                Text::new("CSV Generieren"),
                            )
                            .on_press(StatsMessage::Generate),
                        )
                        .spacing(20),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y(),
            )
            .align_items(Alignment::Center);

        let content: Element<'_, StatsMessage> =
            Container::new(content).padding(TAB_PADDING).into();
        content.map(Message::Statistics)
    }

    fn update_result(
        &mut self,
        shared: &mut SharedData,
        message: StatsMessage,
    ) -> Result<(), StechuhrError> {
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
                self.date = Local.from_local_date(&naive_date).unwrap();
                self.month_picker.show(false);
            }
            StatsMessage::Generate => {
                let hours = StatsTab::evaluate_hours_for_month(shared, self.date)?;
                StatsTab::generate_csv(shared, self.date, hours)?;
            }
            // fallthrough to ignore events
            StatsMessage::HandleEvent(_) => (),
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum StatisticsError {
    DurationError(Duration, Duration),
}

#[derive(Debug, Clone)]
pub enum SoftStatisticsError {
    AlreadyWorking(NaiveDateTime, String),
    AlreadyAway(NaiveDateTime, String),
    OverWhileWorking(NaiveDateTime, String),
    StaffStillWorking(String),
}

impl error::Error for StatisticsError {}
impl error::Error for SoftStatisticsError {}

impl fmt::Display for StatisticsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            Self::DurationError(d1, d2) => {
                format!("Error adding durations {} and {}", d1, d2)
            }
        };
        f.write_str(&description)
    }
}

impl fmt::Display for SoftStatisticsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            Self::AlreadyWorking(date, name) => format!(
                "Um {} wurde der Status von {} auf 'Arbeiten' gesetzt während sie schon am Arbeiten war. Inkonsistente Datenbank, bitte Adrian Bescheid sagen.",
                date, name
            ),
            Self::AlreadyAway(date, name) => format!(
                "Um {} wurde der Status von {} auf 'Pause' gesetzt während sie schon in der Pause war. Inkonsistente Datenbank oder Mitarbeiter hat über eine Monatsgrenze gearbeitet, bitte Adrian Bescheid sagen.",
                date, name
            ),
            Self::OverWhileWorking(date, name) => format!(
                "Um {} wurde eine Hochzeit beendet als {} noch gearbeitet hat. Inkonsistente Datenbank, bitte Adrian Bescheid sagen.",
                date, name
            ),
            Self::StaffStillWorking(name) => format!(
                "{} arbeitet noch am Ende der Auswertung am 1. des nächsten Monats um 6 Uhr morgens. Es wurde wahrscheinlich vergessen sich abzumelden.",
                name
            ),
        };
        f.write_str(&description)
    }
}
