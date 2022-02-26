// add users and change user names/pin/cardid

use iced::{text_input, Column, Element, Row, Text, TextInput};
use iced_aw::TabLabel;
use stechuhr::models::*;

use crate::{Message, SharedData, Tab};

struct StaffMemberState {
    cardid_state: text_input::State,
    cardid_value: String,
}

impl StaffMemberState {
    fn new_from_staff(staff: &Vec<StaffMember>) -> Vec<Self> {
        let mut v = Vec::with_capacity(staff.capacity());
        for staff_member in staff {
            v.push(StaffMemberState {
                cardid_state: text_input::State::new(),
                cardid_value: staff_member.cardid.clone(),
            });
        }
        v
    }
}
/* Abstracts over the vector of staff members and the vector of their UI elements. */
struct MemberUnion<'a> {
    staff: &'a mut Vec<StaffMember>,
    states: &'a mut Vec<StaffMemberState>,
}

impl<'a> MemberUnion<'a> {
    fn from(staff: &'a mut Vec<StaffMember>, states: &'a mut Vec<StaffMemberState>) -> Self {
        MemberUnion { staff, states }
    }

    fn change_cardid_state(&mut self, idx: usize, new_cardid: String) {
        let state = self.states.get_mut(idx).unwrap();
        state.cardid_value = new_cardid;
    }

    fn submit_row(&mut self, idx: usize) {
        let state = self.states.get_mut(idx).unwrap();
        let staff_member = self.staff.get_mut(idx).unwrap();
        staff_member.cardid = state.cardid_value.clone();
    }
}

pub struct ManagementTab {
    /* wether we are logged in */
    authorized: bool,
    admin_password: String,
    admin_password_value: String,
    admin_password_state: text_input::State,
    /* entry of new users */
    staff_states: Vec<StaffMemberState>,
}

#[derive(Debug, Clone)]
pub enum ManagementMessage {
    /* Pre Login */
    ChangePasswordInput(String),
    SubmitPassword,
    /* After Login */
    ChangeCardID(usize, String),
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
            ManagementMessage::ChangeCardID(idx, new_cardid) => {
                MemberUnion::from(&mut shared.staff, &mut self.staff_states)
                    .change_cardid_state(idx, new_cardid)
            }
        }
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
            let mut staff_col = Column::new();

            for (staff_member, state) in shared.staff.iter_mut().zip(self.staff_states.iter_mut()) {
                let staff_row =
                    Row::new()
                        .push(Text::new(&staff_member.name))
                        .push(TextInput::new(
                            &mut state.cardid_state,
                            "click & swipe RFID dongle",
                            &state.cardid_value.clone(),
                            |s| ManagementMessage::ChangeCardID(0, s),
                        ));
                staff_col = staff_col.push(staff_row);
            }

            staff_col.into()
        } else {
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
        };

        content.map(Message::Management)
    }
}
