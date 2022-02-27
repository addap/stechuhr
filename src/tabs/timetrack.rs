use chrono::Locale;
use iced::{
    button, text_input, Button, Column, Container, Element, HorizontalAlignment, Length, Row, Text,
    TextInput,
};
use iced_aw::{modal, Card, Modal, TabLabel};
use stechuhr::models::*;

use crate::{Message, SharedData, Tab};

pub struct TimetrackTab {
    break_input_value: String,
    break_input_uuid: Option<i32>,
    // widget states
    end_party_button_state: button::State,
    break_input_state: text_input::State,
    break_modal_state: modal::State<BreakModalState>,
}

#[derive(Default)]
struct BreakModalState {
    confirm_state: button::State,
    cancel_state: button::State,
}

#[derive(Debug, Clone)]
pub enum TimetrackMessage {
    ChangeBreakInput(String),
    SubmitBreakInput,
    ConfirmSubmitBreakInput,
    CancelSubmitBreakInput,
    EndEvent,
}

impl TimetrackTab {
    pub fn new() -> Self {
        TimetrackTab {
            break_input_value: String::new(),
            break_input_uuid: None,
            end_party_button_state: button::State::default(),
            break_input_state: text_input::State::default(),
            // TODO why does State not take the type argument <BreakModalState> here?
            break_modal_state: modal::State::default(),
        }
    }

    pub fn update(&mut self, shared: &mut SharedData, message: TimetrackMessage) {
        // always focus the input
        self.break_input_state.focus();

        match message {
            TimetrackMessage::ChangeBreakInput(value) => {
                self.break_input_value = value;
            }
            TimetrackMessage::SubmitBreakInput => {
                let input = self.break_input_value.trim();

                if input.len() == 4 || input.len() == 6 {
                    if let Some(staff_member) =
                        StaffMember::get_by_pin_or_card_id(&shared.staff, input)
                    {
                        self.break_modal_state.show(true);
                        self.break_input_uuid = Some(staff_member.uuid());
                    } else {
                        println!("No matching staff member found for input {}.", input);
                    }
                } else {
                    println!("Malformed input {}.", input);
                }
            }
            TimetrackMessage::ConfirmSubmitBreakInput => {
                if let Some(break_uuid) = self.break_input_uuid {
                    let staff_member = StaffMember::get_by_uuid_mut(&mut shared.staff, break_uuid)
                        .expect("uuid does not yield a staff member");
                    let new_status = staff_member.status.toggle();
                    staff_member.status = new_status;
                    shared.log_event(WorkEvent::StatusChange(break_uuid, new_status));
                    self.break_modal_state.show(false);
                    self.break_input_uuid = None;
                    self.break_input_value.clear();
                }
            }
            TimetrackMessage::CancelSubmitBreakInput => {
                self.break_modal_state.show(false);
                self.break_input_uuid = None;
                self.break_input_value.clear();
            }
            TimetrackMessage::EndEvent => {
                shared.log_event(WorkEvent::EventOver);
            }
        }
    }
}

impl<'a: 'b, 'b> Tab<'a, 'b> for TimetrackTab {
    // TODO what is this for?
    // type Message = Message;

    fn title(&self) -> String {
        String::from("Timetrack")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.title())
    }

    fn content(&mut self, shared: &mut SharedData) -> Element<'_, Message> {
        //view
        let mut staff_view = Column::new();
        for staff_member in shared.staff.iter() {
            staff_view = staff_view.push(Text::new(format!(
                "{}: {}",
                staff_member.name,
                staff_member.status.to_string()
            )));
        }

        let content = Container::new(
            Column::new()
                .padding(20)
                .push(Text::new(
                    shared
                        .current_time
                        .format_localized("%T, %A, %e. %B %Y", Locale::de_DE)
                        .to_string(),
                ))
                .push(staff_view)
                .push(
                    TextInput::new(
                        &mut self.break_input_state,
                        "Deine PIN/Dongle swipen",
                        &self.break_input_value,
                        TimetrackMessage::ChangeBreakInput,
                    )
                    .on_submit(TimetrackMessage::SubmitBreakInput),
                )
                .push(
                    Button::new(
                        &mut self.end_party_button_state,
                        Text::new("Event beenden")
                            .horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .on_press(TimetrackMessage::EndEvent),
                ),
        );

        let break_modal_value = if let Some(break_uuid) = self.break_input_uuid {
            let staff_member = StaffMember::get_by_uuid_mut(&mut shared.staff, break_uuid)
                .expect("uuid does not yield a staff member");
            format!(
                "{} wird auf '{}' gesetzt. Korrekt?",
                staff_member.name,
                staff_member.status.toggle()
            )
        } else {
            String::from("Warnung: kein Mitarbeiter ausgewählt. Bitte Adrian Bescheid geben.")
        };

        let modal = Modal::new(&mut self.break_modal_state, content, move |state| {
            Card::new(
                Text::new("Änderung des Arbeitsstatus"),
                Text::new(break_modal_value.clone()),
            )
            .foot(
                Row::new()
                    .spacing(10)
                    .padding(5)
                    .width(Length::Fill)
                    .push(
                        Button::new(
                            &mut state.confirm_state,
                            Text::new("Ok").horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .width(Length::Fill)
                        .on_press(TimetrackMessage::ConfirmSubmitBreakInput),
                    )
                    .push(
                        Button::new(
                            &mut state.cancel_state,
                            Text::new("Zurück").horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .width(Length::Fill)
                        .on_press(TimetrackMessage::CancelSubmitBreakInput),
                    ),
            )
            .max_width(300)
            .width(Length::Shrink)
            .on_close(TimetrackMessage::CancelSubmitBreakInput)
            .into()
        })
        .backdrop(TimetrackMessage::CancelSubmitBreakInput)
        .on_esc(TimetrackMessage::CancelSubmitBreakInput);

        let content: Element<'_, TimetrackMessage> = Container::new(modal).into();

        content.map(Message::Timetrack)
    }
}