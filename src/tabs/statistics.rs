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

use std::{error, fmt};

use chrono::{Date, Duration, Local, Locale, NaiveDate, NaiveDateTime, TimeZone};
use iced::{button, window, Alignment, Button, Column, Container, Element, Length, Row, Text};
use iced_aw::{
    date_picker::{self, DatePicker},
    TabLabel,
};
use iced_native::Event;
use stechuhr::models::StaffMember;

use crate::{Message, SharedData, StechuhrError, Tab, TAB_PADDING};
use stechuhr::TEXT_SIZE_BIG;

use self::time_eval::WorkDuration;

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

/// The result of the computation done by EventSM.
#[derive(Debug)]
pub struct PersonHours<'a> {
    staff_member: &'a StaffMember,
    duration: WorkDuration,
}

impl<'a> PersonHours<'a> {
    fn new(staff_member: &'a StaffMember) -> Self {
        Self {
            staff_member,
            duration: WorkDuration::zero(),
        }
    }

    fn staff_member(&self) -> &StaffMember {
        &self.staff_member
    }

    fn duration(&self) -> &WorkDuration {
        &self.duration
    }
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
        let [minutes_1, minutes_2, minutes_3] = hours.duration().num_minutes();

        Self {
            name: hours.staff_member().name.clone(),
            minutes_1,
            minutes_2,
            minutes_3,
        }
    }
}

#[derive(Debug)]
pub struct StaffHours {
    hours_csv: Vec<PersonHoursCSV>,
    soft_errors: Vec<SoftStatisticsError>,
}

impl StaffHours {
    pub(self) fn hours(&self) -> &[PersonHoursCSV] {
        &self.hours_csv
    }
    pub(self) fn errors(&self) -> &[SoftStatisticsError] {
        &self.soft_errors
    }
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

    fn generate_csv(
        shared: &mut SharedData,
        date: Date<Local>,
        staff_hours: StaffHours,
    ) -> Result<(), StechuhrError> {
        // TODO create auswertung directory

        // Write everyting into a CSV file.
        let filename = format!(
            "./auswertung/{}.csv",
            date.format_localized("%Y-%m %B", Locale::de_DE).to_string()
        );

        let mut wtr = csv::WriterBuilder::new()
            // enable flexible writer since errors are just one field
            .flexible(true)
            .from_path(&filename)?;

        for hours in staff_hours.hours() {
            wtr.serialize(hours)?;
        }
        for error in staff_hours.errors() {
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
                // Set windowed to help people find the generated CSV.
                shared.window_mode = window::Mode::Windowed;
                let hours = event_eval::evaluate_hours_for_month(shared, self.date)?;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoftStatisticsError {
    AlreadyWorking(NaiveDateTime, String),
    AlreadyAway(NaiveDateTime, String),
    StaffStillWorking(NaiveDateTime, String),
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
                "Um {} wurde der Status von {} auf 'Arbeiten' gesetzt während er/sie schon am Arbeiten war. Inkonsistente Datenbank, bitte Adrian Bescheid sagen.",
                date, name
            ),
            Self::AlreadyAway(date, name) => format!(
                "Um {} wurde der Status von {} auf 'Pause' gesetzt während er/sie schon in der Pause war. Inkonsistente Datenbank, bitte Adrian Bescheid sagen.",
                date, name
            ),
            Self::StaffStillWorking(date, name) => format!(
                "Um {} arbeitet {} noch um 6 Uhr morgens. Es wurde wahrscheinlich vergessen sich abzumelden.",
                date, name
            ),
        };
        f.write_str(&description)
    }
}
