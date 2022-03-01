// add users and change user names/pin/cardid

use diesel::SqliteConnection;
use iced::{button, text_input, Button, Column, Element, Length, Row, Text, TextInput};
use iced_aw::TabLabel;
use stechuhr::models::*;

use crate::{Message, SharedData, Tab};

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
    fn new_from_staff(staff: &Vec<StaffMember>) -> Vec<Self> {
        let mut v = Vec::with_capacity(staff.capacity());
        for staff_member in staff {
            v.push(
                StaffMemberState::default()
                    .with_pin(&staff_member.pin)
                    .with_cardid(&staff_member.cardid),
            );
        }
        v
    }
}
/* Abstracts over the vector of staff members and the vector of their UI elements. */
struct MemberRow<'a> {
    staff: &'a mut Vec<StaffMember>,
    states: &'a mut Vec<StaffMemberState>,
}

impl<'a> MemberRow<'a> {
    fn from(staff: &'a mut Vec<StaffMember>, states: &'a mut Vec<StaffMemberState>) -> Self {
        MemberRow { staff, states }
    }

    fn change_pin_state(&mut self, idx: usize, new_pin: String) {
        let state = self.states.get_mut(idx).unwrap();
        state.cardid_value = new_pin;
    }

    fn change_cardid_state(&mut self, idx: usize, new_cardid: String) {
        let state = self.states.get_mut(idx).unwrap();
        state.cardid_value = new_cardid;
    }

    fn submit(&mut self, idx: usize, connection: &SqliteConnection) {
        let state = self.states.get_mut(idx).unwrap();
        let staff_member = self.staff.get_mut(idx).unwrap();

        staff_member.pin.clone_from(&state.pin_value);
        staff_member.cardid.clone_from(&state.cardid_value);

        // save in db
        stechuhr::update_staff_member(staff_member, connection);
    }

    fn submit_new_row(
        &mut self,
        new_name: String,
        new_pin: String,
        new_cardid: String,
        connection: &SqliteConnection,
    ) {
        self.states.push(
            StaffMemberState::default()
                .with_pin(&new_pin)
                .with_cardid(&new_cardid),
        );
        // save in DB
        let new_staff_member = stechuhr::insert_staff(
            NewStaffMember::new(new_name, new_pin, new_cardid),
            connection,
        );
        self.staff.push(new_staff_member);
    }

    // fn delete(&mut self, idx: usize) {
    //     self.states.remove(idx);
    //     self.staff.remove(idx);
    // }
}

pub struct ManagementTab {
    /* wether we are logged in */
    authorized: bool,
    admin_password: String,
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
}

#[derive(Debug, Clone)]
pub enum ManagementMessage {
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
}

impl ManagementTab {
    fn auth(&mut self) {
        self.authorized = true;
    }

    pub fn deauth(&mut self) {
        self.authorized = false;
    }

    pub fn new(staff: &Vec<StaffMember>) -> Self {
        ManagementTab {
            authorized: false,
            admin_password: String::from("blabla"),
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
        }
    }

    pub fn update(&mut self, shared: &mut SharedData, message: ManagementMessage) {
        match message {
            ManagementMessage::ChangePasswordInput(password) => {
                self.admin_password_value = password;
            }
            ManagementMessage::SubmitPassword => {
                if self.admin_password_value == self.admin_password {
                    self.auth();
                } else {
                    // TODO mark pw field as red
                }
                self.admin_password_value.clear();
            }
            ManagementMessage::ChangePIN(idx, new_pin) => {
                MemberRow::from(&mut shared.staff, &mut self.staff_states)
                    .change_pin_state(idx, new_pin)
            }
            ManagementMessage::ChangeCardID(idx, new_cardid) => {
                MemberRow::from(&mut shared.staff, &mut self.staff_states)
                    .change_cardid_state(idx, new_cardid)
            }
            ManagementMessage::SubmitRow(idx) => {
                MemberRow::from(&mut shared.staff, &mut self.staff_states)
                    .submit(idx, &shared.connection)
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
                MemberRow::from(&mut shared.staff, &mut self.staff_states).submit_new_row(
                    self.new_name_value.clone(),
                    self.new_pin_value.clone(),
                    self.new_cardid_value.clone(),
                    &shared.connection,
                );

                self.new_name_value.clear();
                self.new_pin_value.clear();
                self.new_cardid_value.clear();
            }
        }
    }
}

impl ManagementTab {
    fn staff_edit_view(&mut self, shared: &mut SharedData) -> Element<'_, ManagementMessage> {
        let mut staff_col = Column::new().padding(20);

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
            staff_col = staff_col.push(staff_row);
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
            staff_col = staff_col.push(new_row);
        }

        staff_col.into()
    }

    fn password_view(&mut self) -> Element<'_, ManagementMessage> {
        let pw_input = Column::new().push(
            TextInput::new(
                &mut self.admin_password_state,
                "Admin Passwort",
                &self.admin_password_value,
                ManagementMessage::ChangePasswordInput,
            )
            .password()
            .on_submit(ManagementMessage::SubmitPassword),
        );

        pw_input.into()
    }
}

impl<'a: 'b, 'b> Tab<'a, 'b> for ManagementTab {
    // type Message = Message;

    fn title(&self) -> String {
        String::from("User Management")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.title())
    }

    fn content(&'a mut self, shared: &'b mut SharedData) -> Element<'_, Message> {
        let content: Element<'_, ManagementMessage> = if self.authorized {
            self.staff_edit_view(shared)
        } else {
            self.password_view()
        };

        content.map(Message::Management)
    }
}
