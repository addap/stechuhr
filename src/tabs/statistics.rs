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

use chrono::{Date, Duration, Local, Locale, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use iced::{button, Alignment, Button, Column, Container, Element, Length, Row, Text};
use iced_aw::{
    date_picker::{self, DatePicker},
    TabLabel,
};
use iced_native::Event;
use stechuhr::date_ext::NaiveDateExt;

use crate::{Message, SharedData, StechuhrError, Tab, TAB_PADDING};
use event_eval::EventSM;
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
    minutes_weighted: i64,
    #[serde(rename = "Zeit Gewichtet Total")]
    time_weighted: String,
}

impl<'a> PersonHours<'a> {
    fn new(
        name: &'a str,
        minutes_1: i64,
        minutes_2: i64,
        minutes_3: i64,
        minutes_weighted: i64,
    ) -> Self {
        // multiply by precision before rounding to get as many decimal places
        let time_weighted_hours = minutes_weighted / 60;
        let time_weighted_minutes = minutes_weighted % 60;

        Self {
            name,
            minutes_1,
            minutes_2,
            minutes_3,
            minutes_weighted,
            time_weighted: format!(
                "{} Stunden und {} Minuten",
                time_weighted_hours, time_weighted_minutes
            ),
        }
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

    fn generate_csv(&mut self, shared: &mut SharedData) -> Result<(), StechuhrError> {
        // start and end time will be first and last day of the selected month, respectively
        let _9am = NaiveTime::from_hms(9, 0, 0);
        let start_time = self.date.naive_local().first_dom().and_time(_9am);
        let start_time_local = Local.from_local_datetime(&start_time).unwrap();

        let end_time = self.date.naive_local().last_dom().succ().and_time(_9am);
        let end_time_local = Local.from_local_datetime(&end_time).unwrap();

        shared.log_info(format!(
            "Generiere CSV f??r {}, zwischen {} und {}",
            self.date
                .format_localized("%B %Y", Locale::de_DE)
                .to_string(),
            start_time_local
                .format_localized("%d. %B (%R)", Locale::de_DE)
                .to_string(),
            end_time_local
                .format_localized("%d. %B (%R)", Locale::de_DE)
                .to_string()
        ));

        let events = db::load_events_between(start_time, end_time, &shared.connection);

        let staff_hours: Vec<PersonHours> = shared
            .staff
            .iter()
            // associate with each staff member a WorkDuration, which counts the weighted minutes of work time
            .map(|staff_member| {
                let mut event_sm = EventSM::new(staff_member);

                for event in &events {
                    event_sm.process(event)?;
                }

                event_sm.finish()
            })
            .collect::<Result<Vec<(_, _)>, StatisticsError>>()?
            .into_iter()
            // transform the calculated WorkDuration into a PersonHours struct for serialization
            .map(|(staff_member, t)| {
                let [minutes_1, minutes_2, minutes_3, minutes_weigthed] = t.num_minutes();

                PersonHours::new(
                    &staff_member.name[..],
                    minutes_1,
                    minutes_2,
                    minutes_3,
                    minutes_weigthed,
                )
            })
            .collect();

        let filename = format!(
            "./auswertung/{}.csv",
            self.date
                .format_localized("%Y-%m %B", Locale::de_DE)
                .to_string()
        );
        let mut wtr = csv::Writer::from_path(&filename)?;
        for hours in &staff_hours {
            wtr.serialize(hours)?;
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
                            Button::new(&mut self.date_button_state, Text::new("Datum ausw??hlen"))
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
                Ok(())
            }
            StatsMessage::CancelDate => {
                self.month_picker.show(false);
                Ok(())
            }
            StatsMessage::SubmitDate(date) => {
                let naive_date = NaiveDate::from(date);
                // TODO better way to get the current offset? I should be able to just specify the "Europe/Berlin" Timezone and get it from there
                let offset = *Local::now().offset();
                self.date = Date::from_utc(naive_date, offset);
                self.month_picker.show(false);
                Ok(())
            }
            StatsMessage::Generate => self.generate_csv(shared),
            // fallthrough to ignore events
            StatsMessage::HandleEvent(_) => Ok(()),
        }
    }
}

#[derive(Debug)]
pub enum StatisticsError {
    AlreadyWorking(NaiveDateTime, String),
    AlreadyAway(NaiveDateTime, String),
    OverWhileWorking(NaiveDateTime, String),
    StaffStillWorking(String),
    DurationError(Duration, Duration),
}

impl error::Error for StatisticsError {}

impl fmt::Display for StatisticsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            StatisticsError::AlreadyWorking(date, name) => format!(
                "Encountered StatusChange to Working at {} while staff {} was already working",
                date, name
            ),
            StatisticsError::AlreadyAway(date, name) => format!(
                "Encountered StatusChange to Away at {} while staff {} was already away",
                date, name
            ),
            StatisticsError::OverWhileWorking(date, name) => format!(
                "Encountered EventOver at {} while staff {} was still working",
                date, name
            ),
            StatisticsError::StaffStillWorking(name) => {
                format!(
                    "Staff {} is still working at the end of the evaluation",
                    name
                )
            }
            StatisticsError::DurationError(d1, d2) => {
                format!("Error adding durations {} and {}", d1, d2)
            }
        };
        f.write_str(&description)
    }
}
