//! Tab to add/change/get info about users
use std::{error, fmt, mem};

use iced::{
    alignment::{Horizontal, Vertical},
    button,
    scrollable::{self},
    text_input, Alignment, Button, Checkbox, Column, Container, Element, Length, Row, Scrollable,
    Space, Text, TextInput,
};
use iced_aw::{modal, Card, Modal, TabLabel};
use stechuhr::{
    icons::{eye_unicode, ICONS, TEXT_SIZE_EMOJI},
    models::*,
};

use crate::{Message, SharedData, StechuhrError, Tab, TAB_PADDING};

struct StaffMemberState {
    name_state: text_input::State,
    name_value: String,
    pin_state: text_input::State,
    pin_value: String,
    cardid_state: text_input::State,
    cardid_value: String,
    submit_state: button::State,
    #[allow(unused)]
    delete_state: button::State,

    is_visible: bool,
}

impl StaffMemberState {
    fn with_name(mut self, name: &String) -> Self {
        self.name_value.clone_from(name);
        self
    }

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
            name_state: text_input::State::default(),
            name_value: String::default(),
            pin_state: text_input::State::default(),
            pin_value: String::default(),
            cardid_state: text_input::State::default(),
            cardid_value: String::default(),
            submit_state: button::State::default(),
            delete_state: button::State::default(),
            is_visible: true,
        }
    }
}

/// Abstracts over the vector of staff members and the vector of their UI elements.
struct StaffState {
    member_states: Vec<StaffMemberState>,
}

impl From<&[StaffMember]> for StaffState {
    fn from(staff: &[StaffMember]) -> Self {
        let member_states = staff
            .iter()
            .map(|staff_member| {
                StaffMemberState::default()
                    .with_name(&staff_member.name)
                    .with_pin(&staff_member.pin)
                    .with_cardid(&staff_member.cardid)
            })
            .collect();

        StaffState::new(member_states)
    }
}

impl StaffState {
    fn new(member_states: Vec<StaffMemberState>) -> Self {
        StaffState { member_states }
    }

    fn change_name_state(&mut self, idx: usize, new_name: String) -> Result<(), StechuhrError> {
        let state = self
            .member_states
            .get_mut(idx)
            .ok_or(ManagementError::IndexError(idx))?;
        state.name_value = new_name;
        Ok(())
    }

    fn change_pin_state(&mut self, idx: usize, new_pin: String) -> Result<(), StechuhrError> {
        let state = self
            .member_states
            .get_mut(idx)
            .ok_or(ManagementError::IndexError(idx))?;
        state.pin_value = new_pin;
        Ok(())
    }

    fn change_cardid_state(&mut self, idx: usize, new_cardid: String) -> Result<(), StechuhrError> {
        let state = self
            .member_states
            .get_mut(idx)
            .ok_or(ManagementError::IndexError(idx))?;
        state.cardid_value = new_cardid;
        Ok(())
    }

    fn submit(&mut self, shared: &mut SharedData, idx: usize) -> Result<(), StechuhrError> {
        let state = self
            .member_states
            .get_mut(idx)
            .ok_or(ManagementError::IndexError(idx))?;
        let staff_member = shared
            .staff
            .get_mut(idx)
            .ok_or(ManagementError::IndexError(idx))?;

        let name = &state.name_value;
        let pin = &state.pin_value;
        let cardid = &state.cardid_value;
        let is_visible = state.is_visible;

        // use same validation as in submit_new_row
        NewStaffMember::validate(name, pin, cardid)?;
        staff_member.name.clone_from(name);
        staff_member.pin.clone_from(pin);
        staff_member.cardid.clone_from(cardid);
        staff_member.is_visible = is_visible;

        // save in db
        stechuhr::save_staff_member(staff_member, &shared.connection)?;

        let success_message = format!("Mitarbeiter {} erfolgreich geändert.", name);
        shared.log_info(success_message);

        Ok(())
    }

    fn submit_new_row(
        &mut self,
        shared: &mut SharedData,
        new_name: String,
        new_pin: String,
        new_cardid: String,
    ) -> Result<(), StechuhrError> {
        // save in DB
        let new_staff_member = NewStaffMember::new(new_name, new_pin, new_cardid)?;
        let new_staff_member = stechuhr::insert_staff(new_staff_member, &shared.connection)?;

        self.member_states.push(
            StaffMemberState::default()
                .with_name(&new_staff_member.name)
                .with_pin(&new_staff_member.pin)
                .with_cardid(&new_staff_member.cardid),
        );

        let success_message = format!(
            "Neuer Mitarbeiter {} erfolgreich hinzugefügt.",
            new_staff_member.name
        );
        shared.log_info(success_message);

        shared.staff.push(new_staff_member);

        Ok(())
    }

    fn toggle_visible(
        &mut self,
        shared: &mut SharedData,
        idx: usize,
        is_visible: bool,
    ) -> Result<(), StechuhrError> {
        let state = self
            .member_states
            .get_mut(idx)
            .ok_or(ManagementError::IndexError(idx))?;
        state.is_visible = is_visible;

        self.submit(shared, idx)?;
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
    staff_scroll_state: scrollable::State,
    staff_state: StaffState,
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
    ChangeName(usize, String),
    ChangePIN(usize, String),
    ChangeCardID(usize, String),
    SubmitRow(usize),
    ToggleVisible(usize, bool),
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
        let mut staff_scroll_state = scrollable::State::default();
        staff_scroll_state.snap_to(1.0);

        ManagementTab {
            whoami_modal_state: modal::State::default(),
            whoami_button_state: button::State::default(),
            authorized: false,
            admin_password_value: String::from(""),
            admin_password_state: text_input::State::default(),
            staff_state: StaffState::from(staff),
            staff_scroll_state,

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
    fn internal_view(&mut self) -> Element<'_, ManagementMessage> {
        const SPACING: u16 = 100;
        let mut staff_edit = Scrollable::new(&mut self.staff_scroll_state);

        for (idx, member_state) in self.staff_state.member_states.iter_mut().enumerate() {
            let staff_row = Row::new()
                .push(
                    TextInput::new(
                        &mut member_state.name_state,
                        "Name eingeben",
                        &member_state.name_value.clone(),
                        move |s| ManagementMessage::ChangeName(idx, s),
                    )
                    .width(Length::FillPortion(3)),
                )
                .push(
                    TextInput::new(
                        &mut member_state.pin_state,
                        "PIN eingeben",
                        &member_state.pin_value.clone(),
                        move |s| ManagementMessage::ChangePIN(idx, s),
                    )
                    .width(Length::FillPortion(3)),
                )
                .push(
                    TextInput::new(
                        &mut member_state.cardid_state,
                        "click & swipe RFID dongle",
                        &member_state.cardid_value.clone(),
                        move |s| ManagementMessage::ChangeCardID(idx, s),
                    )
                    .width(Length::FillPortion(3)),
                )
                .push(
                    Checkbox::new(member_state.is_visible, eye_unicode(), move |b| {
                        ManagementMessage::ToggleVisible(idx, b)
                    })
                    .font(ICONS)
                    .text_size(TEXT_SIZE_EMOJI)
                    .width(Length::FillPortion(1)),
                )
                .push(
                    Button::new(
                        &mut member_state.submit_state,
                        Text::new("Speichern").horizontal_alignment(Horizontal::Center),
                    )
                    .on_press(ManagementMessage::SubmitRow(idx))
                    .width(Length::FillPortion(2)),
                )
                .spacing(SPACING);
            staff_edit = staff_edit.push(staff_row);
        }

        // last inputs for new staff member
        {
            let new_row = Row::new()
                .push(
                    TextInput::new(
                        &mut self.new_name_state,
                        "Name eingeben",
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
                .push(Space::new(Length::FillPortion(1), Length::Shrink))
                .push(
                    Button::new(
                        &mut self.new_submit_state,
                        Text::new("Speichern").horizontal_alignment(Horizontal::Center),
                    )
                    .on_press(ManagementMessage::SubmitNewRow)
                    .width(Length::FillPortion(2)),
                )
                .spacing(SPACING);
            staff_edit = staff_edit.push(new_row);
        }

        let event_over = Button::new(
            &mut self.end_party_button_state,
            Text::new("Event beenden").horizontal_alignment(Horizontal::Center),
        )
        .on_press(ManagementMessage::EndEvent);

        let content = Column::new()
            .push(
                Container::new(staff_edit)
                    .width(Length::Fill)
                    .height(Length::FillPortion(90))
                    .center_x()
                    .align_y(Vertical::Top),
            )
            .push(
                Container::new(event_over)
                    .width(Length::Fill)
                    .height(Length::FillPortion(10))
                    .center_x()
                    .center_y(),
            )
            .spacing(20)
            .align_items(Alignment::Center);
        content.into()
    }

    fn public_view(&mut self, shared: &mut SharedData) -> Element<'_, ManagementMessage> {
        if shared.prompt_modal_state.is_shown() {
            self.admin_password_state.unfocus();
        }

        let content = Column::new()
            .push(Space::new(Length::Fill, Length::Units(100)))
            .push(
                Row::new()
                    .push(Space::new(Length::FillPortion(2), Length::Shrink))
                    .push(
                        TextInput::new(
                            &mut self.admin_password_state,
                            "Administrator Passwort",
                            &self.admin_password_value,
                            ManagementMessage::ChangePasswordInput,
                        )
                        .password()
                        .on_submit(ManagementMessage::SubmitPassword)
                        .width(Length::FillPortion(3)),
                    )
                    .push(Space::new(Length::FillPortion(2), Length::Shrink)),
            )
            .push(
                Button::new(
                    &mut self.whoami_button_state,
                    Text::new("Wem gehört dieser Dongle?").horizontal_alignment(Horizontal::Center),
                )
                .on_press(ManagementMessage::Whoami),
            )
            // .padding(100)
            .spacing(100)
            .align_items(Alignment::Center);

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

impl Tab for ManagementTab {
    type Message = ManagementMessage;

    fn title(&self) -> String {
        String::from("Verwaltung")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.title())
    }

    fn content(&mut self, shared: &mut SharedData) -> Element<'_, Message> {
        let content: Element<'_, ManagementMessage> = if self.authorized {
            self.admin_password_state.unfocus();

            self.internal_view()
        } else {
            /* Normally the textinput must be focussed.
             * But when the modal is open, we must unfocus, else it will capture an 'enter' press meant to close the modal that should be handled in the subcriptions in main.rs */
            if self.whoami_modal_state.is_shown() || shared.prompt_modal_state.is_shown() {
                self.admin_password_state.unfocus();
            } else {
                self.admin_password_state.focus();
            }

            self.public_view(shared)
        };

        let content: Element<'_, ManagementMessage> =
            Container::new(content).padding(TAB_PADDING).into();
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
            ManagementMessage::ChangeName(idx, new_name) => {
                self.staff_state.change_name_state(idx, new_name)?;
            }
            ManagementMessage::ChangePIN(idx, new_pin) => {
                self.staff_state.change_pin_state(idx, new_pin)?;
            }
            ManagementMessage::ChangeCardID(idx, new_cardid) => {
                self.staff_state.change_cardid_state(idx, new_cardid)?;
            }
            ManagementMessage::SubmitRow(idx) => {
                self.staff_state.submit(shared, idx)?;
            }
            ManagementMessage::ToggleVisible(idx, b) => {
                self.staff_state.toggle_visible(shared, idx, b)?
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
                self.staff_state.submit_new_row(
                    shared,
                    self.new_name_value.clone(),
                    self.new_pin_value.clone(),
                    self.new_cardid_value.clone(),
                )?;

                self.new_name_value.clear();
                self.new_pin_value.clear();
                self.new_cardid_value.clear();

                self.staff_scroll_state.snap_to(1.0);
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

                let msg = match cardid.parse::<Cardid>() {
                    Ok(_) => match StaffMember::get_by_card_id(&shared.staff, &cardid) {
                        Some(staff_member) => format!(
                            "Der Dongle mit ID \"{}\" gehört {}",
                            cardid,
                            staff_member.name.clone()
                        ),
                        None => format!("Der Dongle mit ID \"{}\" gehört niemandem", cardid),
                    },
                    Err(e) => format!("Ungültige Dongle-ID. {}", e),
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
