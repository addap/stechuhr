// add users and change user names/pin/cardid

use chrono::{DateTime, Local, Locale};
use diesel::prelude::*;
use iced::{
    button, executor, text_input, Application, Button, Column, Command, Container, Element,
    HorizontalAlignment, Length, Row, Settings, Subscription, Text, TextInput,
};
use iced_aw::{modal, Card, Modal, TabLabel};
use iced_native::{window, Event};
use stechuhr::models::*;

use crate::{Message, SharedData, Tab};

pub struct ManagementTab {
    /* wether we are logged in */
    authorized: bool,
    admin_password: String,
    admin_password_value: String,
    admin_password_state: text_input::State,
    /* entry of new users */
    new_name_state: text_input::State,
}

#[derive(Debug, Clone)]
pub enum ManagementMessage {
    ChangePasswordInput(String),
    SubmitPassword,
}

impl ManagementTab {
    fn auth(&mut self) {
        self.authorized = true;
    }

    pub fn deauth(&mut self) {
        self.authorized = false;
    }

    pub fn new() -> Self {
        ManagementTab {
            authorized: false,
            admin_password: String::from("blabla"),
            admin_password_value: String::from(""),
            admin_password_state: text_input::State::default(),
            new_name_state: text_input::State::default(),
        }
    }

    pub fn update(&mut self, shared: &mut SharedData, message: ManagementMessage) {
        match message {
            ManagementMessage::ChangePasswordInput(password) => {
                self.admin_password_value = password;
            }
            ManagementMessage::SubmitPassword => {
                if (self.admin_password_value == self.admin_password) {
                    self.auth();
                }
                self.admin_password_value.clear();
            }
        }
    }
}

impl Tab for ManagementTab {
    type Message = Message;

    fn title(&self) -> String {
        String::from("User Management")
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.title())
    }

    fn content(&mut self, shared: &mut SharedData) -> Element<'_, Self::Message> {
        if (self.authorized) {
            Container::new(Text::new("Eingeloggt!")).into()
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

            let content: Element<'_, ManagementMessage> = Container::new(pw_input).into();
            content.map(Message::Management)
        }
    }
}
