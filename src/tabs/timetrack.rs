use std::cmp::min;

use chrono::Locale;
use iced::{
    alignment::Horizontal, button, text_input, Alignment, Button, Column, Container, Element,
    Image, Length, Row, Space, Text, TextInput,
};
use iced_aw::{modal, Card, Modal, TabLabel};
use stechuhr::models::*;

use crate::{Message, SharedData, StechuhrError, Tab, TAB_PADDING, TEXT_SIZE, TEXT_SIZE_BIG};

const PIN_LENGTH: usize = 4;
const CARDID_LENGTH: usize = 10;

pub struct TimetrackTab {
    break_input_value: String,
    break_input_uuid: Option<i32>,
    // widget states
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
}

impl TimetrackTab {
    pub fn new() -> Self {
        TimetrackTab {
            break_input_value: String::new(),
            break_input_uuid: None,
            break_input_state: text_input::State::default(),
            // TODO why does State not take the type argument <BreakModalState> here?
            break_modal_state: modal::State::default(),
        }
    }

    fn handle_confirm_submit_break_input(&mut self, shared: &mut SharedData) {
        if let Some(break_uuid) = self.break_input_uuid {
            let staff_member = StaffMember::get_by_uuid_mut(&mut shared.staff, break_uuid)
                .expect("uuid does not yield a staff member");
            let name = staff_member.name.clone();
            let new_status = staff_member.status.toggle();
            staff_member.status = new_status;
            shared.log_event(WorkEvent::StatusChange(break_uuid, name, new_status));
            self.break_modal_state.show(false);
            self.break_input_uuid = None;
            self.break_input_value.clear();
        }
    }

    /// Generate a column of names and icons signalling their work status.
    /// Have to annotate return type as 'static, else it takes the argument's lifetime
    fn get_staff_column(staff: &[StaffMember]) -> Element<'static, TimetrackMessage> {
        let names = Column::new()
            .width(Length::FillPortion(80))
            .spacing(10)
            .align_items(Alignment::End);
        let icons = Column::new()
            .width(Length::FillPortion(20))
            .spacing(10)
            .align_items(Alignment::Start);

        let (names, icons) = staff
            .iter()
            .fold((names, icons), |(names, icons), staff_member| {
                let img = Image::new(staff_member.status.to_emoji())
                    .width(Length::Units(TEXT_SIZE))
                    .height(Length::Units(TEXT_SIZE));
                (
                    names.push(
                        Text::new(format!(
                            "{}: {}",
                            staff_member.name,
                            staff_member.status.to_string()
                        ))
                        .size(TEXT_SIZE),
                    ),
                    icons.push(img),
                )
            });

        Row::new()
            .push(names)
            .push(icons)
            .width(Length::FillPortion(10))
            .spacing(10)
            .into()
    }

    /// Generate the timetrack dashboard composed of columns of names and icons signalling their work status.
    /// Have to annotate return type as 'static, else it takes the argument's lifetime
    fn get_staff_view(staff: &[StaffMember]) -> Container<'static, TimetrackMessage> {
        const COLUMNS: usize = 3;
        let column_size = staff.len() / COLUMNS;
        let mut extra = staff.len() % COLUMNS;

        let padding1 = Space::new(Length::Shrink, Length::Fill);
        let padding2 = Space::new(Length::Shrink, Length::Fill);

        let mut staff_view = Row::new().spacing(50).push(padding1);
        let mut start = 0;

        for _ in 0..COLUMNS {
            let end = start
                + column_size
                + if extra > 0 {
                    extra -= 1;
                    1
                } else {
                    0
                };
            let end = min(staff.len(), end);
            let staff_column = TimetrackTab::get_staff_column(&staff[start..end]);
            staff_view = staff_view.push(staff_column);

            start = end;
        }
        Container::new(staff_view.push(padding2))
    }
}

impl SharedData {}

impl Tab for TimetrackTab {
    type Message = TimetrackMessage;

    fn title(&self) -> String {
        String::from("Stechuhr")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.title())
    }

    fn content(&mut self, shared: &mut SharedData) -> Element<'_, Message> {
        /* Normally the textinput must be focussed so that we can just swipe a rfid tag anytime.
         * But when the modal is open, we must unfocus, else it will capture an 'enter' press meant to close the modal that should be handled in the subcriptions in main.rs */
        if self.break_modal_state.is_shown() || shared.prompt_modal_state.is_shown() {
            self.break_input_state.unfocus();
        } else {
            self.break_input_state.focus();
        }

        // big clock at the top
        let clock = Text::new(
            shared
                .current_time
                .format_localized("%A, %e. %B - %T", Locale::de_DE)
                .to_string(),
        )
        .horizontal_alignment(Horizontal::Center)
        .size(TEXT_SIZE_BIG);

        let staff_view = TimetrackTab::get_staff_view(&shared.staff);

        let dongle_input = TextInput::new(
            &mut self.break_input_state,
            "PIN eingeben/Dongle swipen",
            &self.break_input_value,
            TimetrackMessage::ChangeBreakInput,
        )
        .on_submit(TimetrackMessage::SubmitBreakInput)
        .size(TEXT_SIZE)
        .width(Length::Units(300));

        let content = Column::new()
            .align_items(Alignment::Center)
            .width(Length::Fill)
            .padding(TAB_PADDING)
            .spacing(10)
            .push(clock.height(Length::FillPortion(10)))
            .push(staff_view.height(Length::FillPortion(70)))
            .push(dongle_input);

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
                            Text::new("Ok").horizontal_alignment(Horizontal::Center),
                        )
                        .width(Length::Shrink)
                        .on_press(TimetrackMessage::ConfirmSubmitBreakInput),
                    )
                    .push(
                        Button::new(
                            &mut state.cancel_state,
                            Text::new("Zurück").horizontal_alignment(Horizontal::Center),
                        )
                        .width(Length::Shrink)
                        .on_press(TimetrackMessage::CancelSubmitBreakInput),
                    ),
            )
            // .max_width(300)
            .width(Length::Shrink)
            .on_close(TimetrackMessage::CancelSubmitBreakInput)
            .into()
        })
        .backdrop(TimetrackMessage::CancelSubmitBreakInput)
        .on_esc(TimetrackMessage::CancelSubmitBreakInput);

        let content: Element<'_, TimetrackMessage> = modal.into();
        content.map(Message::Timetrack)
    }

    fn update_result(
        &mut self,
        shared: &mut SharedData,
        message: TimetrackMessage,
    ) -> Result<(), StechuhrError> {
        match message {
            TimetrackMessage::ChangeBreakInput(value) => {
                self.break_input_value = value;
            }
            TimetrackMessage::SubmitBreakInput => {
                let input = self.break_input_value.trim().to_owned();

                if input.len() == PIN_LENGTH || input.len() == CARDID_LENGTH {
                    if let Some(staff_member) =
                        StaffMember::get_by_pin_or_card_id(&shared.staff, &input)
                    {
                        self.break_modal_state.show(true);
                        self.break_input_uuid = Some(staff_member.uuid());
                    } else {
                        self.break_input_value.clear();
                        return Err(StechuhrError::Str(String::from("Unbekannte PIN/Dongle")));
                    }
                } else {
                    self.break_input_value.clear();
                    return Err(StechuhrError::Str(format!(
                        "\"{}\" is weder eine PIN noch ein Dongle",
                        input
                    )));
                }
            }
            TimetrackMessage::ConfirmSubmitBreakInput => {
                self.handle_confirm_submit_break_input(shared);
            }
            TimetrackMessage::CancelSubmitBreakInput => {
                self.break_modal_state.show(false);
                self.break_input_uuid = None;
                self.break_input_value.clear();
            }
        }
        Ok(())
    }
}
