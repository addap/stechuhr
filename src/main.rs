extern crate csv;
extern crate serde;
#[macro_use]
extern crate serde_derive;

mod tabs;

use chrono::{DateTime, Local, Locale};
use diesel::prelude::*;
use dotenv::dotenv;
use iced::{
    button, executor, Align, Application, Button, Color, Column, Command, Container, Element,
    Length, Settings, Subscription, Text,
};
use iced_aw::{modal, Card, Modal, TabBar, TabLabel};
use iced_native::{event::Status, keyboard, window, Event};
use std::{error, fmt, io};
use stechuhr::models::*;

use tabs::management::{ManagementError, ManagementMessage, ManagementTab};
use tabs::statistics::{StatisticsError, StatsMessage, StatsTab};
use tabs::timetrack::{TimetrackMessage, TimetrackTab};

const TEXT_SIZE: u16 = 24;
const TEXT_SIZE_BIG: u16 = 42;
const HEADER_SIZE: u16 = 32;
const TAB_PADDING: u16 = 16;
const LOG_LENGTH: usize = 6;

pub fn main() -> iced::Result {
    // DONE what does this accomplish? any side-effects?
    // the side effect is populating the env module used below. The ok() is to turn a Result into an Option so that the "unused Result" warning is not triggered.
    dotenv().ok();

    env_logger::init();
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
    prompt_modal_state: modal::State<PromptModalState>,
}

impl SharedData {
    fn log_event(&mut self, event: WorkEvent) {
        let new_eventt = NewWorkEventT::new(event);
        let eventt = stechuhr::insert_event(&new_eventt, &self.connection);
        self.events.push(eventt);
    }

    fn log_info(&mut self, msg: String) {
        self.log_event(WorkEvent::Info(msg));
    }

    fn prompt_info(&mut self, msg: String) {
        self.prompt_modal_state.show(true);
        self.prompt_modal_state.inner_mut().msg = msg;
    }

    /// Handle a result of some computation by showing the error message in a prompt.
    /// TODO also log to syslog
    fn handle_result(&mut self, result: Result<(), StechuhrError>) {
        if let Err(e) = result {
            let e = e.to_string();
            log::error!("{}", &e);
            self.prompt_info(e.clone());
            self.log_event(WorkEvent::Error(e));
        }
    }
}

impl SharedData {
    fn get_logview(&self) -> Container<'static, Message> {
        let initial_logview = Column::new().spacing(5);
        let logview = self
            .events
            .iter()
            // TODO a better way to take the last n items from an iterator
            .rev()
            .take(LOG_LENGTH)
            .rev()
            .fold(initial_logview, |column, eventt| {
                let offset = *Local::now().offset();
                let time = DateTime::<Local>::from_utc(eventt.created_at, offset);
                column.push(Text::new(format!(
                    "{}: {}",
                    time.format_localized("%T", Locale::de_DE).to_string(),
                    eventt.event
                )))
            });
        Container::new(logview)
    }
}

#[derive(Debug, PartialEq, Default)]
struct PromptModalState {
    msg: String,
    ok_button_state: button::State,
}

struct Stechuhr {
    shared: SharedData,
    active_tab: StechuhrTab,
    should_exit: bool,
    timetrack: TimetrackTab,
    management: ManagementTab,
    statistics: StatsTab,
}

#[derive(Debug, Clone, Copy)]
enum StechuhrTab {
    Timetrack = 0,
    Management = 1,
    Statistics = 2,
}

// impl StechuhrTab {
//     fn get_tab<'a>(self, state: &'a Stechuhr) -> Box<dyn Tab<Message = Message>> {
//         match self {
//             StechuhrTab::Timetrack => Box::new(state.timetrack),
//             StechuhrTab::Management => &state.management,
//             StechuhrTab::Statistics => &state.statistics,
//         }
//     }
// }

impl From<usize> for StechuhrTab {
    fn from(active_tab: usize) -> Self {
        match active_tab {
            0 => Self::Timetrack,
            1 => Self::Management,
            2 => Self::Statistics,
            _ => panic!("Unknown active_tab: {}", active_tab),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Tick(DateTime<Local>),
    ExitApplication,
    ExitPrompt,
    TabSelected(usize),
    Timetrack(TimetrackMessage),
    Management(ManagementMessage),
    Statistics(StatsMessage),
    // sent in main subscriptions and delegated down to the prompts
    PressedEnter,
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
                    events: Vec::new(),
                    connection: connection,
                    prompt_modal_state: modal::State::default(),
                },
                active_tab: StechuhrTab::Timetrack,
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
                if self
                    .shared
                    .staff
                    .iter()
                    .any(|staff_member| staff_member.status == WorkStatus::Working)
                {
                    self.shared.prompt_info(String::from(
                        "Es sind noch Personen am Arbeiten. Bitte zuerst das Event beenden.",
                    ));
                } else {
                    match stechuhr::update_staff(&self.shared.staff, &self.shared.connection) {
                        Ok(()) => self.should_exit = true,
                        Err(e) => self.shared.handle_result(Err(StechuhrError::Diesel(e))),
                    }
                }
            }
            Message::ExitPrompt => {
                self.shared.prompt_modal_state.show(false);
                self.shared.prompt_modal_state.inner_mut().msg.clear();
            }
            Message::TabSelected(new_tab) => {
                self.management.deauth();
                self.active_tab = StechuhrTab::from(new_tab);
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
            Message::PressedEnter => {
                if self.shared.prompt_modal_state.is_shown() {
                    self.shared.prompt_modal_state.show(false);
                } else {
                    match StechuhrTab::from(self.active_tab) {
                        StechuhrTab::Timetrack => self
                            .timetrack
                            .update(&mut self.shared, TimetrackMessage::CancelSubmitBreakInput),
                        _ => {}
                    }
                }
            }
        };
        Command::none()
    }

    // TODO what is '_?
    fn view<'a>(&'a mut self) -> Element<'_, Self::Message> {
        // let theme = self
        //     .settings_tab
        //     .settings()
        //     .tab_bar_theme
        //     .unwrap_or_default();

        // let mut scrollbar = Scrollable::new(&mut self.scroll_state)
        //     .padding(10)
        //     .spacing(10)
        //     .width(Length::Fill)
        //     .height(Length::Fill);

        // TODO I want to use a scrollbar and snap to the bottom when a new event is added, but snapping is only supported in iced 0.4 which is not published on cargo yet

        let logview = Container::new(self.shared.get_logview())
            .padding(TAB_PADDING)
            .width(Length::Fill)
            .height(Length::FillPortion(20));

        let tab_bar = TabBar::new(self.active_tab as usize, Message::TabSelected)
            .padding(5)
            .text_size(HEADER_SIZE)
            .push(self.timetrack.tab_label())
            .push(self.management.tab_label())
            .push(self.statistics.tab_label());

        let tab_content = match self.active_tab {
            StechuhrTab::Timetrack => self.timetrack.view(&mut self.shared),
            StechuhrTab::Management => self.management.view(&mut self.shared),
            StechuhrTab::Statistics => self.statistics.view(&mut self.shared),
        };
        let tab_content = Container::new(tab_content)
            .padding(TAB_PADDING)
            .width(Length::Fill)
            .height(Length::FillPortion(80))
            .center_x()
            .center_y();

        let content = Column::new().push(tab_bar).push(tab_content).push(logview);

        let modal = Modal::new(&mut self.shared.prompt_modal_state, content, move |state| {
            Card::new(Text::new("Information"), Text::new(&state.msg))
                .foot(
                    Button::new(&mut state.ok_button_state, Text::new("Ok"))
                        .width(Length::Shrink)
                        .on_press(Message::ExitPrompt),
                )
                .max_width(300)
                .width(Length::Fill)
                .on_close(Message::ExitPrompt)
                .into()
        })
        .backdrop(Message::ExitPrompt)
        .on_esc(Message::ExitPrompt);

        let element: Element<'_, Self::Message> = modal.into();
        // element.explain(Color::BLACK)
        element
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
                ) => Some(Message::PressedEnter),
                (_, _) => None,
            }),
        ])
    }
}

trait Tab<'a: 'b, 'b> {
    type Message;

    fn title(&self) -> String;

    fn tab_label(&self) -> TabLabel;

    fn view(&'a mut self, shared: &'b mut SharedData) -> Element<'_, Message> {
        // each tab has its name in the upper right corner
        let title = Text::new(self.title()).size(HEADER_SIZE);

        // center the content of each tab
        let content = Container::new(self.content(shared))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .align_y(Align::Start);

        Column::new().push(title).push(content).into()
    }

    fn content(&'a mut self, shared: &'b mut SharedData) -> Element<'_, Message>;

    fn update(&'a mut self, shared: &'b mut SharedData, message: Self::Message) {
        let result = self.update_result(shared, message);
        shared.handle_result(result);
    }

    fn update_result<'c: 'd, 'd>(
        &'c mut self,
        shared: &'d mut SharedData,
        message: Self::Message,
    ) -> Result<(), StechuhrError>;
}

#[derive(Debug)]
pub enum StechuhrError {
    Management(ManagementError),
    Statistics(StatisticsError),
    Model(ModelError),
    Diesel(diesel::result::Error),
    Opener(opener::OpenError),
    CSV(csv::Error),
    IO(io::Error),
    Str(String),
}

impl From<ManagementError> for StechuhrError {
    fn from(e: ManagementError) -> Self {
        Self::Management(e)
    }
}

impl From<StatisticsError> for StechuhrError {
    fn from(e: StatisticsError) -> Self {
        Self::Statistics(e)
    }
}

impl From<ModelError> for StechuhrError {
    fn from(e: ModelError) -> Self {
        Self::Model(e)
    }
}

impl From<csv::Error> for StechuhrError {
    fn from(e: csv::Error) -> Self {
        Self::CSV(e)
    }
}

impl From<io::Error> for StechuhrError {
    fn from(e: io::Error) -> Self {
        Self::IO(e)
    }
}

impl From<diesel::result::Error> for StechuhrError {
    fn from(e: diesel::result::Error) -> Self {
        Self::Diesel(e)
    }
}

impl From<opener::OpenError> for StechuhrError {
    fn from(e: opener::OpenError) -> Self {
        Self::Opener(e)
    }
}

impl error::Error for StechuhrError {}

impl fmt::Display for StechuhrError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StechuhrError::Management(e) => e.fmt(f),
            StechuhrError::Statistics(e) => e.fmt(f),
            StechuhrError::Model(e) => e.fmt(f),
            StechuhrError::Diesel(e) => e.fmt(f),
            StechuhrError::Opener(e) => e.fmt(f),
            StechuhrError::CSV(e) => e.fmt(f),
            StechuhrError::IO(e) => e.fmt(f),
            StechuhrError::Str(msg) => f.write_str(msg),
        }
    }
}
