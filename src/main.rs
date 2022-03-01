extern crate csv;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use chrono::{DateTime, Local};
use diesel::prelude::*;
use iced::{
    executor, Align, Application, Column, Command, Container, Element, Length, Settings,
    Subscription, Text,
};
use iced_aw::{TabLabel, Tabs};
use iced_native::{event::Status, keyboard, window, Event};
use stechuhr::models::*;

mod tabs;
use tabs::management::{ManagementMessage, ManagementTab};
use tabs::statistics::{StatsMessage, StatsTab};
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
    connection: SqliteConnection,
}

impl SharedData {
    fn log_event(&mut self, event: WorkEvent) {
        stechuhr::save_event(
            WorkEventT {
                created_at: Local::now().naive_local(),
                event: event,
            },
            &self.connection,
        );
    }
}

struct Stechuhr {
    shared: SharedData,
    active_tab: usize,
    should_exit: bool,
    timetrack: TimetrackTab,
    management: ManagementTab,
    statistics: StatsTab,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(DateTime<Local>),
    ExitApplication,
    TabSelected(usize),
    Timetrack(TimetrackMessage),
    Management(ManagementMessage),
    Statistics(StatsMessage),
}

impl Application for Stechuhr {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = SqliteConnection;

    fn should_exit(&self) -> bool {
        self.should_exit
    }

    fn new(connection: SqliteConnection) -> (Self, Command<Message>) {
        let staff = stechuhr::load_staff(&connection);
        let management = ManagementTab::new(&staff);

        (
            Self {
                shared: SharedData {
                    current_time: Local::now(),
                    staff,
                    connection: connection,
                },
                active_tab: 0,
                should_exit: false,
                timetrack: TimetrackTab::new(),
                management,
                statistics: StatsTab::new(),
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
                println!("exit");
                stechuhr::update_staff(&self.shared.staff, &self.shared.connection);
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
            Message::Statistics(stats_message) => {
                self.statistics.update(&mut self.shared, stats_message);
            }
        };
        Command::none()
    }

    // TODO what is '_?
    fn view<'a>(&'a mut self) -> Element<'_, Message> {
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
            .push(
                self.statistics.tab_label(),
                self.statistics.view(&mut self.shared),
            )
            // .tab_bar_style(theme)
            // .icon_font(ICON_FONT)
            .tab_bar_position(iced_aw::TabBarPosition::Top)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::time::every(std::time::Duration::from_secs(1))
                .map(|_| Message::Tick(Local::now())),
            iced_native::subscription::events_with(|event, status| match (status, event) {
                /* event when closing the window e.g. mod+Shift+q in i3 */
                (_, Event::Window(window::Event::CloseRequested)) => Some(Message::ExitApplication),
                /* event when pressing enter key. At the moment we only send it to the timetrack tab to confirm the submission modal.
                 * we need to be careful to only handle events that have not been caputed elsewhere, otherwise we use the enter again which originally opened the submission modal */
                (
                    Status::Ignored,
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::Enter,
                        ..
                    }),
                ) => Some(Message::Timetrack(
                    TimetrackMessage::ConfirmSubmitBreakInput,
                )),
                (_, _) => None,
            }),
        ])
    }
}

trait Tab<'a: 'b, 'b> {
    // type Message;

    fn title(&self) -> String;

    fn tab_label(&self) -> TabLabel;

    fn view(&'a mut self, shared: &'b mut SharedData) -> Element<'_, Message> {
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

    fn content(&'a mut self, shared: &'b mut SharedData) -> Element<'_, Message>;
}
