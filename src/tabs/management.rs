// add users and change user names/pin/cardid

use std::{error, fmt, mem};

use iced::{
    button, text_input, Button, Column, Element, HorizontalAlignment, Length, Row, Text, TextInput,
};
use iced_aw::{modal, Card, Modal, TabLabel};
use stechuhr::models::*;

use crate::{Message, SharedData, StechuhrError, Tab};

struct StaffMemberState {
    pin_state: text_input::State,
    pin_value: String,
    cardid_state: text_input::State,
    cardid_value: String,
    submit_state: button::State,
    delete_state: button::State,
}

impl StaffMemberState {
    fn with_pin(mut self, pin: &String) -> Self {
        self.pin_value.clone_from(pin);
        self
    }

    fn with_cardid(mut self, cardid: &String) -> Self {
        self.cardid_value.clone_from(cardid);
        self
    }
}

impl Default for StaffMemberState {
    fn default() -> Self {
        Self {
            pin_state: text_input::State::default(),
            pin_value: String::default(),
            cardid_state: text_input::State::default(),
            cardid_value: String::default(),
            submit_state: button::State::default(),
            delete_state: button::State::default(),
        }
    }
}

impl StaffMemberState {
    fn new_from_staff(staff: &[StaffMember]) -> Vec<Self> {
        staff
            .iter()
            .map(|staff_member| {
                StaffMemberState::default()
                    .with_pin(&staff_member.pin)
                    .with_cardid(&staff_member.cardid)
            })
            .collect()
    }
}
/* Abstracts over the vector of staff members and the vector of their UI elements. */
struct MemberRow<'a> {
    shared: &'a mut SharedData,
    states: &'a mut Vec<StaffMemberState>,
}

impl<'a> MemberRow<'a> {
    fn from(shared: &'a mut SharedData, states: &'a mut Vec<StaffMemberState>) -> Self {
        MemberRow { shared, states }
    }

    fn change_pin_state(&mut self, idx: usize, new_pin: String) -> Result<(), StechuhrError> {
        let state = self
            .states
            .get_mut(idx)
            .ok_or(ManagementError::IndexError(idx))?;
        state.cardid_value = new_pin;
        Ok(())
    }

    fn change_cardid_state(&mut self, idx: usize, new_cardid: String) -> Result<(), StechuhrError> {
        let state = self
            .states
            .get_mut(idx)
            .ok_or(ManagementError::IndexError(idx))?;
        state.cardid_value = new_cardid;
        Ok(())
    }

    fn submit(&mut self, idx: usize) -> Result<(), StechuhrError> {
        let state = self
            .states
            .get_mut(idx)
            .ok_or(ManagementError::IndexError(idx))?;
        let staff_member = self
            .shared
            .staff
            .get_mut(idx)
            .ok_or(ManagementError::IndexError(idx))?;

        let pin = &state.pin_value;
        let cardid = &state.cardid_value;
        let _ = pin.parse::<PIN>()?;
        let _ = cardid.parse::<Cardid>()?;

        staff_member.pin.clone_from(pin);
        staff_member.cardid.clone_from(cardid);

        // save in db
        stechuhr::update_staff_member(staff_member, &self.shared.connection)?;
        Ok(())
    }

    fn submit_new_row(
        &mut self,
        new_name: String,
        new_pin: String,
        new_cardid: String,
    ) -> Result<(), StechuhrError> {
        self.states.push(
            StaffMemberState::default()
                .with_pin(&new_pin)
                .with_cardid(&new_cardid),
        );

        // have to declare the message here since we move the string and the new_staff_member before we can use it in the call to log_info
        let success_message = format!("Neuer Mitarbeiter {} hinzugefügt", new_name);

        // save in DB
        let new_staff_member = NewStaffMember::new(new_name, new_pin, new_cardid)?;
        let new_staff_member = stechuhr::insert_staff(new_staff_member, &self.shared.connection)?;
        self.shared.staff.push(new_staff_member);

        self.shared.log_info(success_message);
        Ok(())
    }

    // fn delete(&mut self, idx: usize) {
    //     self.states.remove(idx);
    //     self.staff.remove(idx);
    // }
}

pub struct ManagementTab {
    whoami_modal_state: modal::State<WhoamiModalState>,
    whoami_button_state: button::State,
    /* wether we are logged in */
    authorized: bool,
    admin_password_value: String,
    admin_password_state: text_input::State,
    /* management of staff */
    staff_states: Vec<StaffMemberState>,
    /* adding new staff */
    new_name_state: text_input::State,
    new_name_value: String,
    new_pin_state: text_input::State,
    new_pin_value: String,
    new_cardid_state: text_input::State,
    new_cardid_value: String,
    new_submit_state: button::State,
    end_party_button_state: button::State,
}

#[derive(Debug, Default)]
struct WhoamiModalState {
    input_value: String,
    input_state: text_input::State,
}

#[derive(Debug, Clone)]
pub enum ManagementMessage {
    Whoami,
    ChangeWhoami(String),
    SubmitWhoami,
    CancelWhoami,
    /* Pre Login */
    ChangePasswordInput(String),
    SubmitPassword,
    /* After Login */
    ChangePIN(usize, String),
    ChangeCardID(usize, String),
    SubmitRow(usize),
    // DeleteRow(usize),
    ChangeNewRow(Option<String>, Option<String>, Option<String>),
    SubmitNewRow,
    EndEvent,
}

impl ManagementTab {
    fn auth(&mut self) {
        self.authorized = true;
    }

    pub fn deauth(&mut self) {
        self.authorized = false;
    }

    pub fn new(staff: &[StaffMember]) -> Self {
        ManagementTab {
            whoami_modal_state: modal::State::default(),
            whoami_button_state: button::State::default(),
            authorized: false,
            admin_password_value: String::from(""),
            admin_password_state: text_input::State::default(),
            staff_states: StaffMemberState::new_from_staff(staff),

            new_name_state: text_input::State::default(),
            new_name_value: String::from(""),
            new_pin_state: text_input::State::default(),
            new_pin_value: String::from(""),
            new_cardid_state: text_input::State::default(),
            new_cardid_value: String::from(""),
            new_submit_state: button::State::default(),

            end_party_button_state: button::State::default(),
        }
    }
}

impl ManagementTab {
    fn internal_view(&mut self, shared: &mut SharedData) -> Element<'_, ManagementMessage> {
        let mut staff_edit = Column::new().padding(20);

        for (idx, (staff_member, state)) in shared
            .staff
            .iter_mut()
            .zip(self.staff_states.iter_mut())
            .enumerate()
        {
            let staff_row = Row::new()
                .push(Text::new(&staff_member.name).width(Length::FillPortion(3)))
                .push(
                    TextInput::new(
                        &mut state.pin_state,
                        "PIN eingeben",
                        &state.pin_value.clone(),
                        move |s| ManagementMessage::ChangePIN(idx, s),
                    )
                    .width(Length::FillPortion(3)),
                )
                .push(
                    TextInput::new(
                        &mut state.cardid_state,
                        "click & swipe RFID dongle",
                        &state.cardid_value.clone(),
                        move |s| ManagementMessage::ChangeCardID(idx, s),
                    )
                    .width(Length::FillPortion(3)),
                )
                .push(
                    Button::new(&mut state.submit_state, Text::new("Speichern"))
                        .on_press(ManagementMessage::SubmitRow(idx))
                        .width(Length::FillPortion(1)),
                );
            staff_edit = staff_edit.push(staff_row);
        }

        // last inputs for new staff member
        {
            let new_row = Row::new()
                .push(
                    TextInput::new(
                        &mut self.new_name_state,
                        "Name",
                        &self.new_name_value,
                        |s| ManagementMessage::ChangeNewRow(Some(s), None, None),
                    )
                    .width(Length::FillPortion(3)),
                )
                .push(
                    TextInput::new(
                        &mut self.new_pin_state,
                        "PIN eingeben",
                        &self.new_pin_value,
                        |s| ManagementMessage::ChangeNewRow(None, Some(s), None),
                    )
                    .width(Length::FillPortion(3)),
                )
                .push(
                    TextInput::new(
                        &mut self.new_cardid_state,
                        "click & swipe RFID dongle",
                        &self.new_cardid_value,
                        move |s| ManagementMessage::ChangeNewRow(None, None, Some(s)),
                    )
                    .width(Length::FillPortion(3)),
                )
                .push(
                    Button::new(&mut self.new_submit_state, Text::new("Hinzufügen"))
                        .on_press(ManagementMessage::SubmitNewRow)
                        .width(Length::FillPortion(1)),
                );
            staff_edit = staff_edit.push(new_row);
        }

        let event_over = Button::new(
            &mut self.end_party_button_state,
            Text::new("Event beenden").horizontal_alignment(HorizontalAlignment::Center),
        )
        .on_press(ManagementMessage::EndEvent);

        let content = Column::new().push(staff_edit).push(event_over);
        content.into()
    }

    fn public_view(&mut self, shared: &mut SharedData) -> Element<'_, ManagementMessage> {
        if shared.prompt_modal_state.is_shown() {
            self.admin_password_state.unfocus();
        }

        let content = Column::new()
            .push(
                TextInput::new(
                    &mut self.admin_password_state,
                    "Administrator Passwort",
                    &self.admin_password_value,
                    ManagementMessage::ChangePasswordInput,
                )
                .password()
                .on_submit(ManagementMessage::SubmitPassword),
            )
            .push(
                Button::new(
                    &mut self.whoami_button_state,
                    Text::new("Wem gehört dieser Dongle?")
                        .horizontal_alignment(HorizontalAlignment::Center),
                )
                .on_press(ManagementMessage::Whoami),
            );

        let whoami_modal = Modal::new(&mut self.whoami_modal_state, content, move |state| {
            Card::new(Text::new("Dongle Abfrage"), {
                state.input_state.focus();
                TextInput::new(
                    &mut state.input_state,
                    "",
                    &state.input_value,
                    ManagementMessage::ChangeWhoami,
                )
                .on_submit(ManagementMessage::SubmitWhoami)
            })
            .max_width(300)
            .width(Length::Fill)
            .on_close(ManagementMessage::CancelWhoami)
            .into()
        })
        .backdrop(ManagementMessage::CancelWhoami)
        .on_esc(ManagementMessage::CancelWhoami);

        whoami_modal.into()
    }
}

impl<'a: 'b, 'b> Tab<'a, 'b> for ManagementTab {
    type Message = ManagementMessage;

    fn title(&self) -> String {
        String::from("Verwaltung")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.title())
    }

    fn content(&mut self, shared: &mut SharedData) -> Element<'_, Message> {
        let content: Element<'_, ManagementMessage> = if self.authorized {
            self.internal_view(shared)
        } else {
            self.public_view(shared)
        };

        content.map(Message::Management)
    }

    fn update_result(
        &mut self,
        shared: &mut SharedData,
        message: ManagementMessage,
    ) -> Result<(), StechuhrError> {
        match message {
            ManagementMessage::ChangePasswordInput(password) => {
                self.admin_password_value = password;
            }
            ManagementMessage::SubmitPassword => {
                if stechuhr::verify_password(self.admin_password_value.trim(), &shared.connection) {
                    self.admin_password_value.clear();
                    self.auth();
                } else {
                    self.admin_password_value.clear();
                    return Err(ManagementError::InvalidPassword.into());
                }
            }
            ManagementMessage::ChangePIN(idx, new_pin) => {
                MemberRow::from(shared, &mut self.staff_states).change_pin_state(idx, new_pin)?;
            }
            ManagementMessage::ChangeCardID(idx, new_cardid) => {
                MemberRow::from(shared, &mut self.staff_states)
                    .change_cardid_state(idx, new_cardid)?;
            }
            ManagementMessage::SubmitRow(idx) => {
                MemberRow::from(shared, &mut self.staff_states).submit(idx)?;
            }
            // ManagementMessage::DeleteRow(idx) => {
            //     MemberRow::from(&mut shared.staff, &mut self.staff_states)
            //         .delete(idx)
            // }
            ManagementMessage::ChangeNewRow(name, pin, cardid) => {
                if let Some(name) = name {
                    self.new_name_value = name;
                }
                if let Some(pin) = pin {
                    self.new_pin_value = pin;
                }
                if let Some(cardid) = cardid {
                    self.new_cardid_value = cardid;
                }
            }
            ManagementMessage::SubmitNewRow => {
                MemberRow::from(shared, &mut self.staff_states).submit_new_row(
                    self.new_name_value.clone(),
                    self.new_pin_value.clone(),
                    self.new_cardid_value.clone(),
                )?;

                self.new_name_value.clear();
                self.new_pin_value.clear();
                self.new_cardid_value.clear();
            }
            ManagementMessage::Whoami => {
                self.whoami_modal_state.show(true);
            }
            ManagementMessage::CancelWhoami => {
                self.whoami_modal_state.inner_mut().input_value.clear();
                self.whoami_modal_state.show(false);
            }
            ManagementMessage::ChangeWhoami(cardid) => {
                self.whoami_modal_state.inner_mut().input_value = cardid;
            }
            ManagementMessage::SubmitWhoami => {
                let cardid = mem::replace(
                    &mut self.whoami_modal_state.inner_mut().input_value,
                    String::from(""),
                );
                self.whoami_modal_state.show(false);

                let msg = match StaffMember::get_by_card_id(&shared.staff, &cardid) {
                    Some(staff_member) => format!(
                        "Der Dongle mit ID \"{}\" gehört {}",
                        cardid,
                        staff_member.name.clone()
                    ),
                    None => format!("Der Dongle mit ID \"{}\" gehört niemandem", cardid),
                };
                shared.prompt_message(msg);
            }
            ManagementMessage::EndEvent => {
                let sign_off_events: Vec<_> = shared
                    .staff
                    .iter_mut()
                    .filter(|staff_member| staff_member.status == WorkStatus::Working)
                    .map(|staff_member| {
                        let uuid = staff_member.uuid();
                        let name = staff_member.name.clone();
                        let new_status = WorkStatus::Away;
                        staff_member.status = new_status;
                        WorkEvent::StatusChange(uuid, name, new_status)
                    })
                    .collect();

                for event in sign_off_events.into_iter() {
                    shared.log_event(event);
                }
                shared.log_event(WorkEvent::EventOver);
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum ManagementError {
    IndexError(usize),
    InvalidPassword,
}

impl error::Error for ManagementError {}

impl fmt::Display for ManagementError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            ManagementError::IndexError(idx) => {
                format!("Index out of range: {}", idx)
            }
            ManagementError::InvalidPassword => String::from("Ungültiges Passwort"),
        };
        f.write_str(&description)
    }
}
