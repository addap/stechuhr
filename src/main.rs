use chrono::{DateTime, Local, Locale};
use diesel::prelude::*;
use iced::{
    button, executor, text_input, Align, Application, Button, Column, Command, Container, Element,
    HorizontalAlignment, Length, Row, Settings, Subscription, Text, TextInput,
};
use iced_aw::{modal, Card, Modal, TabLabel, Tabs};
use iced_native::{window, Event};
use stechuhr::models::*;

mod tabs;
use tabs::management::{ManagementMessage, ManagementTab};
use tabs::timetrack::{TimetrackMessage, TimetrackTab};

const HEADER_SIZE: u16 = 32;
const TAB_PADDING: u16 = 16;

pub fn main() -> iced::Result {
    let connection = stechuhr::establish_connection();

    Stechuhr::run(Settings {
        // a.d. set this so that we can handle the close request ourselves to sync data to db
        exit_on_close_request: false,
        ..Settings::with_flags(connection)
    })
}

pub struct SharedData {
    current_time: DateTime<Local>,
    staff: Vec<StaffMember>,
    events: Vec<WorkEventT>,
    connection: SqliteConnection,
}

impl SharedData {
    fn log_event(&mut self, event: WorkEvent) {
        self.events.push(WorkEventT {
            timestamp: Local::now(),
            event: event,
        });
    }
}

struct Stechuhr {
    shared: SharedData,
    active_tab: usize,
    should_exit: bool,
    timetrack: TimetrackTab,
    management: ManagementTab,
    // generate: GenerateTab,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(DateTime<Local>),
    ExitApplication,
    TabSelected(usize),
    Timetrack(TimetrackMessage),
    Management(ManagementMessage),
}

impl Application for Stechuhr {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = SqliteConnection;

    fn should_exit(&self) -> bool {
        self.should_exit
    }

    fn new(connection: SqliteConnection) -> (Self, Command<Message>) {
        (
            Self {
                shared: SharedData {
                    current_time: Local::now(),
                    staff: stechuhr::load_staff(&connection),
                    events: Vec::new(),
                    connection: connection,
                },
                active_tab: 0,
                should_exit: false,
                timetrack: TimetrackTab::new(),
                management: ManagementTab::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Stechuhr")
    }

    fn update(&mut self, message: Message, _clipboard: &mut iced::Clipboard) -> Command<Message> {
        match message {
            Message::Tick(local_time) => {
                if local_time > self.shared.current_time {
                    self.shared.current_time = local_time;
                }
            }
            Message::ExitApplication => {
                stechuhr::save_staff(&self.shared.staff, &self.shared.connection);
                self.should_exit = true;
            }
            Message::TabSelected(new_tab) => {
                self.management.deauth();
                self.active_tab = new_tab;
            }
            Message::Timetrack(timetrack_message) => {
                self.timetrack.update(&mut self.shared, timetrack_message);
            }
            Message::Management(management_message) => {
                self.management.update(&mut self.shared, management_message);
            }
        };
        Command::none()
    }

    // TODO what is '_?
    fn view(&mut self) -> Element<'_, Message> {
        // let theme = self
        //     .settings_tab
        //     .settings()
        //     .tab_bar_theme
        //     .unwrap_or_default();

        Tabs::new(self.active_tab, Message::TabSelected)
            .push(
                self.timetrack.tab_label(),
                self.timetrack.view(&mut self.shared),
            )
            .push(
                self.management.tab_label(),
                self.management.view(&mut self.shared),
            )
            // .push(self.counter_tab.tab_label(), self.counter_tab.view())
            // .push(self.settings_tab.tab_label(), self.settings_tab.view())
            // .tab_bar_style(theme)
            // .icon_font(ICON_FONT)
            .tab_bar_position(iced_aw::TabBarPosition::Top)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::time::every(std::time::Duration::from_secs(1))
                .map(|_| Message::Tick(Local::now())),
            iced_native::subscription::events_with(|event, _status| match event {
                Event::Window(window::Event::CloseRequested) => Some(Message::ExitApplication),
                _ => None,
            }),
        ])
    }
}

trait Tab {
    type Message;

    fn title(&self) -> String;

    fn tab_label(&self) -> TabLabel;

    fn view(&mut self, shared: &mut SharedData) -> Element<'_, Self::Message> {
        let column = Column::new()
            .spacing(20)
            .push(Text::new(self.title()).size(HEADER_SIZE))
            .push(self.content(shared));

        Container::new(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Align::Center)
            .align_y(Align::Start)
            .padding(TAB_PADDING)
            .into()
    }

    fn content(&mut self, shared: &mut SharedData) -> Element<'_, Self::Message>;
}
