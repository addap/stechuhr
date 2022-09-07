//! Tab to add/change/get info about users
use std::{error, fmt, mem};

use chrono::Local;
use iced::{
    alignment::{Horizontal, Vertical},
    button, keyboard, scrollable, text_input, Alignment, Button, Checkbox, Column, Container,
    Element, Length, Row, Scrollable, Space, Text, TextInput,
};
use iced_aw::{modal, Card, Modal, TabLabel};
use iced_native::Event;
use stechuhr::{
    db,
    icons::{self, TEXT_SIZE_EMOJI},
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

    fn with_visible(mut self, is_visible: bool) -> Self {
        self.is_visible = is_visible;
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
                    .with_visible(staff_member.is_visible)
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
        db::save_staff_member(staff_member, &mut shared.connection)?;

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
        let new_staff_member = db::insert_staff(new_staff_member, &mut shared.connection)?;

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

    fn delete_row(&mut self, shared: &mut SharedData, idx: usize) -> Result<(), StechuhrError> {
        if idx >= self.member_states.len() {
            return Err(ManagementError::IndexError(idx).into());
        }
        self.member_states.remove(idx);
        let staff_member = shared.staff.remove(idx);

        db::delete_staff_member(staff_member, &mut shared.connection)?;

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

    delete_modal_state: modal::State<DeleteModalState>,
    delete_idx: Option<usize>,
}

#[derive(Default)]
struct DeleteModalState {
    delete_confirm_state: button::State,
    delete_cancel_state: button::State,
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
    DeleteRow(usize),
    ConfirmDeleteRow,
    CancelDeleteRow,
    ChangeNewRow(Option<String>, Option<String>, Option<String>),
    SubmitNewRow,
    EndEvent,
    GenericSubmit,
    HandleEvent(Event),
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

            delete_modal_state: modal::State::default(),
            delete_idx: None,
        }
    }

    fn submit_new_row(&mut self, shared: &mut SharedData) -> Result<(), StechuhrError> {
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

        Ok(())
    }
}

impl ManagementTab {
    fn text_input<'a, F>(
        state: &'a mut text_input::State,
        placeholder: &str,
        value: &str,
        f: F,
    ) -> TextInput<'a, ManagementMessage>
    where
        F: 'a + Fn(String) -> ManagementMessage,
    {
        stechuhr::style::text_input(state, placeholder, value, f)
            .on_submit(ManagementMessage::GenericSubmit)
            .width(Length::FillPortion(3))
    }

    fn internal_view(&mut self, shared: &mut SharedData) -> Element<'_, ManagementMessage> {
        const SPACING: u16 = 1;
        let mut staff_edit = Scrollable::new(&mut self.staff_scroll_state);
        let mut even = true;

        for (idx, member_state) in self.staff_state.member_states.iter_mut().enumerate() {
            let staff_row = Container::new(
                Row::new()
                    .push(
                        ManagementTab::text_input(
                            &mut member_state.name_state,
                            "Name eingeben",
                            &member_state.name_value.clone(),
                            move |s| ManagementMessage::ChangeName(idx, s),
                        )
                        .width(Length::FillPortion(25)),
                    )
                    .push(Space::new(Length::FillPortion(SPACING), Length::Shrink))
                    .push(
                        ManagementTab::text_input(
                            &mut member_state.pin_state,
                            "PIN eingeben",
                            &member_state.pin_value.clone(),
                            move |s| ManagementMessage::ChangePIN(idx, s),
                        )
                        .width(Length::FillPortion(25)),
                    )
                    .push(Space::new(Length::FillPortion(SPACING), Length::Shrink))
                    .push(
                        ManagementTab::text_input(
                            &mut member_state.cardid_state,
                            "Dongle swipen",
                            &member_state.cardid_value.clone(),
                            move |s| ManagementMessage::ChangeCardID(idx, s),
                        )
                        .width(Length::FillPortion(25)),
                    )
                    .push(Space::new(Length::FillPortion(5), Length::Shrink))
                    .push(
                        Checkbox::new(
                            member_state.is_visible,
                            icons::emoji::eye.codepoint,
                            move |b| ManagementMessage::ToggleVisible(idx, b),
                        )
                        .font(icons::FONT_SYMBOLA)
                        .text_size(TEXT_SIZE_EMOJI)
                        .width(Length::FillPortion(8)),
                    )
                    .push(
                        Button::new(
                            &mut member_state.delete_state,
                            icons::icon(icons::emoji::trashcan),
                        )
                        .on_press(ManagementMessage::DeleteRow(idx))
                        .width(Length::FillPortion(5)),
                    )
                    .push(
                        Button::new(
                            &mut member_state.submit_state,
                            icons::icon(icons::emoji::floppydisk),
                        )
                        .on_press(ManagementMessage::SubmitRow(idx))
                        .width(Length::FillPortion(5)),
                    )
                    .push(Space::new(Length::FillPortion(2), Length::Shrink)),
            )
            .style(stechuhr::style::management_row(&mut even));
            staff_edit = staff_edit.push(staff_row);
        }

        // last inputs for new staff member
        {
            let new_row = Container::new(
                Row::new()
                    .push(
                        ManagementTab::text_input(
                            &mut self.new_name_state,
                            "Name eingeben",
                            &self.new_name_value,
                            |s| ManagementMessage::ChangeNewRow(Some(s), None, None),
                        )
                        .width(Length::FillPortion(25)),
                    )
                    .push(Space::new(Length::FillPortion(SPACING), Length::Shrink))
                    .push(
                        ManagementTab::text_input(
                            &mut self.new_pin_state,
                            "PIN eingeben",
                            &self.new_pin_value,
                            |s| ManagementMessage::ChangeNewRow(None, Some(s), None),
                        )
                        .width(Length::FillPortion(25)),
                    )
                    .push(Space::new(Length::FillPortion(SPACING), Length::Shrink))
                    .push(
                        ManagementTab::text_input(
                            &mut self.new_cardid_state,
                            "click & swipe RFID dongle",
                            &self.new_cardid_value,
                            move |s| ManagementMessage::ChangeNewRow(None, None, Some(s)),
                        )
                        .width(Length::FillPortion(25)),
                    )
                    .push(Space::new(Length::FillPortion(5), Length::Shrink))
                    .push(Space::new(Length::FillPortion(13), Length::Shrink))
                    .push(
                        Button::new(
                            &mut self.new_submit_state,
                            icons::icon(icons::emoji::floppydisk),
                        )
                        .on_press(ManagementMessage::SubmitNewRow)
                        .width(Length::FillPortion(5)),
                    )
                    .push(Space::new(Length::FillPortion(2), Length::Shrink)),
            )
            .style(stechuhr::style::management_row(&mut even));
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

        let delete_modal_value = if let Some(delete_idx) = self.delete_idx {
            if let Some(staff_member) = shared.staff.get(delete_idx) {
                format!("{} wird gelöscht. Korrekt?", staff_member.name,)
            } else {
                String::from("Warnung: das solltest du nicht sehen. Bitte Adrian Bescheid geben.")
            }
        } else {
            String::from("Warnung: das solltest du nicht sehen. Bitte Adrian Bescheid geben.")
        };

        let modal = Modal::new(&mut self.delete_modal_state, content, move |state| {
            Card::new(
                Text::new("Löschen eines Mitarbeiters"),
                Text::new(&delete_modal_value),
            )
            .foot(
                Row::new()
                    .spacing(10)
                    .padding(5)
                    .width(Length::Fill)
                    .push(
                        Button::new(
                            &mut state.delete_confirm_state,
                            Text::new("Ok").horizontal_alignment(Horizontal::Center),
                        )
                        .width(Length::Shrink)
                        .on_press(ManagementMessage::ConfirmDeleteRow),
                    )
                    .push(
                        Button::new(
                            &mut state.delete_cancel_state,
                            Text::new("Zurück").horizontal_alignment(Horizontal::Center),
                        )
                        .width(Length::Shrink)
                        .on_press(ManagementMessage::CancelDeleteRow),
                    ),
            )
            .width(Length::Shrink)
            .on_close(ManagementMessage::CancelDeleteRow)
            .into()
        })
        .backdrop(ManagementMessage::CancelDeleteRow)
        .on_esc(ManagementMessage::CancelDeleteRow);

        modal.into()
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
                        stechuhr::style::text_input(
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
                stechuhr::style::text_input(
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

    fn collect_inputs(&mut self) -> (Option<usize>, Vec<&mut text_input::State>) {
        let mut inputs = Vec::with_capacity(3 * (self.staff_state.member_states.len()));

        for staff_member_state in &mut self.staff_state.member_states {
            inputs.push(&mut staff_member_state.name_state);
            inputs.push(&mut staff_member_state.pin_state);
            inputs.push(&mut staff_member_state.cardid_state);
        }

        inputs.push(&mut self.new_name_state);
        inputs.push(&mut self.new_pin_state);
        inputs.push(&mut self.new_cardid_state);

        let focus_idx =
            inputs
                .iter()
                .enumerate()
                .find_map(|(i, input)| if input.is_focused() { Some(i) } else { None });

        (focus_idx, inputs)
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
        let (_, inputs) = self.collect_inputs();
        if shared.prompt_modal_state.is_shown() {
            inputs.into_iter().for_each(|input| input.unfocus());
        }

        let content: Element<'_, ManagementMessage> = if self.authorized {
            self.admin_password_state.unfocus();

            self.internal_view(shared)
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
                if db::verify_password(self.admin_password_value.trim(), &mut shared.connection) {
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
                self.staff_state.toggle_visible(shared, idx, b)?;
            }
            ManagementMessage::DeleteRow(idx) => {
                self.delete_idx = Some(idx);
                self.delete_modal_state.show(true);
            }
            ManagementMessage::CancelDeleteRow => {
                self.delete_idx = None;
                self.delete_modal_state.show(false);
            }
            ManagementMessage::ConfirmDeleteRow => {
                if let Some(delete_idx) = self.delete_idx {
                    self.staff_state.delete_row(shared, delete_idx)?;

                    self.delete_idx = None;
                    self.delete_modal_state.show(false);
                }
            }
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
                self.submit_new_row(shared)?;
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
                let sign_off_time = Local::now().naive_local();
                let sign_off_events = shared.sign_off_all_staff(sign_off_time);
                for eventt in sign_off_events.into_iter() {
                    shared.log_eventt(eventt);
                }
                shared.create_event(WorkEvent::EventOver);
            }
            ManagementMessage::GenericSubmit => {
                let (focus_idx, _) = self.collect_inputs();

                if let Some(focus_idx) = focus_idx {
                    let row_idx = focus_idx / 3;

                    if row_idx == self.staff_state.member_states.len() {
                        // we are in the last row so we submit
                        self.submit_new_row(shared)?;
                    } else {
                        // one of the existing rows, so just save that
                        self.staff_state.submit(shared, row_idx)?;
                    }
                }
            }
            // a.d. completely hacked together tab order since iced does not seem to provide it
            ManagementMessage::HandleEvent(Event::Keyboard(keyboard::Event::KeyPressed {
                key_code: keyboard::KeyCode::Tab,
                modifiers,
            })) => {
                let (focus_idx, mut inputs) = self.collect_inputs();

                if let Some(focus_idx) = focus_idx {
                    let new_focus_idx = if modifiers.shift() {
                        (focus_idx + inputs.len() - 1) % inputs.len()
                    } else {
                        (focus_idx + 1) % inputs.len()
                    };
                    inputs.get_mut(focus_idx).unwrap().unfocus();
                    inputs.get_mut(new_focus_idx).unwrap().focus();
                }
            }
            // fallthrough to ignore events
            ManagementMessage::HandleEvent(_) => {}
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
