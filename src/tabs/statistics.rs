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

use chrono::{Duration, NaiveDate, NaiveDateTime};
use iced::{
    button, text_input, Button, Column, Container, Element, HorizontalAlignment, Length, Row, Text,
    TextInput,
};
use iced_aw::{modal, Card, Modal, TabLabel};
use std::collections::HashMap;
use stechuhr::models::*;

use crate::{Message, SharedData, Tab};

pub struct StatsTab {
    // widget states
    generate_button_state: button::State,
}

#[derive(Debug, Clone)]
pub enum StatsMessage {
    Generate,
}

enum EvalState {
    Working(NaiveDateTime),
    Away,
}

impl StatsTab {
    pub fn new() -> Self {
        StatsTab {
            generate_button_state: button::State::default(),
        }
    }

    fn add_time(
        hm: &mut HashMap<i32, Duration>,
        uuid: i32,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
    ) {
        let time = hm.entry(uuid).or_insert(Duration::zero());

        // todo compute time
        let duration = end_time.signed_duration_since(start_time);
        *time = time.checked_add(&duration).unwrap();
    }

    pub fn update(&mut self, shared: &mut SharedData, message: StatsMessage) {
        match message {
            StatsMessage::Generate => {
                let staff = &shared.staff;
                let mut hours_by_staff: HashMap<i32, Duration> = HashMap::new();
                let events = stechuhr::load_events(&shared.connection);

                for staff_member in staff.iter() {
                    let mut eval_state = EvalState::Away;

                    for event in events.iter() {
                        match eval_state {
                            EvalState::Away => match event.event {
                                WorkEvent::StatusChange(uuid, WorkStatus::Working) => {
                                    if staff_member.uuid().eq(&uuid) {
                                        eval_state = EvalState::Working(event.created_at)
                                    }
                                }
                                _ => {}
                            },
                            EvalState::Working(start_time) => match event.event {
                                WorkEvent::StatusChange(uuid, WorkStatus::Away)
                                    if staff_member.uuid().eq(&uuid) =>
                                {
                                    StatsTab::add_time(
                                        &mut hours_by_staff,
                                        staff_member.uuid(),
                                        start_time,
                                        event.created_at,
                                    );
                                    eval_state = EvalState::Away;
                                }
                                WorkEvent::EventOver => {
                                    StatsTab::add_time(
                                        &mut hours_by_staff,
                                        staff_member.uuid(),
                                        start_time,
                                        event.created_at,
                                    );
                                    eval_state = EvalState::Away;
                                }
                                _ => {}
                            },
                        }
                    }

                    match eval_state {
                        EvalState::Working(_) => {
                            panic!("Staff still working");
                        }
                        _ => {}
                    }
                }

                println! {"Times: {:?}", hours_by_staff};
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

    fn content(&mut self, shared: &mut SharedData) -> Element<'_, Message> {
        let content: Element<'_, StatsMessage> = Container::new(
            Column::new().padding(20).push(
                Button::new(&mut self.generate_button_state, Text::new("CSV Generieren"))
                    .on_press(StatsMessage::Generate),
            ),
        )
        .into();

        content.map(Message::Statistics)
    }
}
