extern crate csv;
extern crate serde;
#[macro_use]
extern crate serde_derive;

mod tabs;

use chrono::{DateTime, Local, Locale, NaiveTime};
use chrono::{NaiveDateTime, TimeZone};
use diesel::prelude::*;
use dotenv::dotenv;
use iced::alignment::Vertical;
#[allow(unused_imports)]
use iced::Color;
use iced::{
    button, executor, scrollable, window, Application, Button, Column, Command, Container, Element,
    Length, Scrollable, Settings, Subscription, Text,
};
use iced_aw::{modal, Card, Modal, TabBar, TabLabel};
use iced_native::{event::Status, keyboard, Event};
use std::{error, fmt, io};
use stechuhr::db;
use stechuhr::models::*;

use tabs::management::{ManagementError, ManagementMessage, ManagementTab};
use tabs::statistics::{StatisticsError, StatsMessage, StatsTab};
use tabs::timetrack::{TimetrackMessage, TimetrackTab};

const HEADER_SIZE: u16 = 32;
const TAB_PADDING: u16 = 16;

pub fn main() -> iced::Result {
    // DONE what does this accomplish? any side-effects?
    // the side effect is populating the env module used below. The ok() is to turn a Result into an Option so that the "unused Result" warning is not triggered.
    dotenv().ok();

    env_logger::init();
    let connection = db::establish_connection();

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
    /// Log a WorkEvent in the scrollbar area at the bottom and also persist it to the DB.
    fn create_event(&mut self, event: WorkEvent) {
        let new_eventt = NewWorkEventT::now(event);
        self.log_eventt(new_eventt);
    }

    fn log_eventt(&mut self, new_eventt: NewWorkEventT) {
        let eventt = db::insert_event(&new_eventt, &mut self.connection);
        // This breaks the ordering of events (since we have the pregenerated 6am boundaries in the future)
        self.events.push(eventt);
    }

    /// Log an information event.
    /// TODO remove when logging to journal
    fn log_info(&mut self, msg: String) {
        self.create_event(WorkEvent::Info(msg));
    }

    /// Log an error event.
    /// TODO remove when logging to journal
    fn log_error(&mut self, e: String) {
        self.create_event(WorkEvent::Error(e));
    }

    /// Open a modal to more prominently show some piece of information.
    fn prompt_message(&mut self, msg: String) {
        self.prompt_modal_state.show(true);
        self.prompt_modal_state.inner_mut().msg = msg;
    }

    /// Handle a result of some computation by showing the error message in a prompt.
    /// TODO also log to journal
    fn handle_result(&mut self, result: Result<(), StechuhrError>) {
        if let Err(e) = result {
            let e = e.to_string();
            log::error!("{}", &e);
            self.prompt_message(e.clone());
            self.log_error(e);
        }
    }

    /// Set every staff member that is working to "Away" and corresponding StatusChange events.
    fn sign_off_all_staff(&mut self, sign_off_time: NaiveDateTime) -> Vec<NewWorkEventT> {
        self.staff
            .iter_mut()
            .filter(|staff_member| staff_member.status == WorkStatus::Working)
            .map(|staff_member| {
                let uuid = staff_member.uuid();
                let name = staff_member.name.clone();
                let new_status = WorkStatus::Away;
                staff_member.status = new_status;
                NewWorkEventT::new(
                    sign_off_time,
                    WorkEvent::StatusChange(uuid, name, new_status),
                )
            })
            .collect()
    }
}

#[derive(Debug, PartialEq, Default)]
struct PromptModalState {
    msg: String,
    ok_button_state: button::State,
}

struct Stechuhr {
    shared: SharedData,
    log_scroll: scrollable::State,
    window_mode: window::Mode,
    active_tab: StechuhrTab,
    should_exit: bool,
    timetrack: TimetrackTab,
    management: ManagementTab,
    statistics: StatsTab,
}

impl Stechuhr {
    /// Generate a container containing a scrollable with all WorkEvents.
    fn get_logview<'a>(
        log_scroll: &'a mut scrollable::State,
        shared: &SharedData,
    ) -> Element<'a, Message> {
        let log_initial = Scrollable::new(log_scroll)
            .on_scroll(|d| {
                if d == 1.0 {
                    Message::ScrollSnap
                } else {
                    Message::Nop
                }
            })
            .width(Length::Fill)
            .spacing(5)
            .padding(5);

        let log_view = shared.events.iter().fold(log_initial, |log_view, eventt| {
            let time = Local.from_local_datetime(&eventt.created_at).unwrap();

            log_view.push(Text::new(format!(
                "{}: {}",
                time.format_localized("%T", Locale::de_DE).to_string(),
                eventt.event
            )))
        });

        log_view.into()
    }
}

#[derive(Debug, Clone, Copy)]
enum StechuhrTab {
    Timetrack = 0,
    Management = 1,
    Statistics = 2,
}

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
    HandleEvent(Event),
    ScrollSnap,
    Nop,
    ToggleFullscreen,
}

impl Application for Stechuhr {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = SqliteConnection;

    fn should_exit(&self) -> bool {
        self.should_exit
    }

    /// Always run Stechuhr in fullscreen mode.
    fn mode(&self) -> window::Mode {
        self.window_mode
    }

    fn new(mut connection: SqliteConnection) -> (Self, Command<Message>) {
        let staff = db::load_state(&mut connection);
        let management = ManagementTab::new(&staff);
        // Log should follow new events by default.
        let mut log_scroll = scrollable::State::default();
        log_scroll.snap_to(1.0);

        (
            Self {
                shared: SharedData {
                    current_time: Local::now(),
                    staff,
                    events: Vec::new(),
                    connection: connection,
                    prompt_modal_state: modal::State::default(),
                },
                log_scroll,
                window_mode: window::Mode::Fullscreen,
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

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Tick(local_time) => {
                self.shared.current_time = local_time;

                // If it's just before 6am, sign off all staff. The 6am barrier event will already exist so we don't have to create it again.
                if local_time.time() == NaiveTime::from_hms(5, 59, 59) {
                    let _ = self.shared.sign_off_all_staff(local_time.naive_local());
                }
            }
            Message::ExitApplication => {
                if self
                    .shared
                    .staff
                    .iter()
                    .any(|staff_member| staff_member.status == WorkStatus::Working)
                {
                    self.shared.prompt_message(String::from(
                        "Es sind noch Personen am Arbeiten. Bitte zuerst alle auf \"Pause\" stellen oder das Event beenden.",
                    ));
                } else {
                    match db::save_staff(&self.shared.staff, &mut self.shared.connection) {
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
            Message::HandleEvent(Event::Keyboard(keyboard::Event::KeyPressed {
                key_code: keyboard::KeyCode::Enter,
                ..
            })) if self.shared.prompt_modal_state.is_shown() => {
                self.shared.prompt_modal_state.show(false)
            }
            Message::HandleEvent(e) => match StechuhrTab::from(self.active_tab) {
                StechuhrTab::Timetrack => self
                    .timetrack
                    .update(&mut self.shared, TimetrackMessage::HandleEvent(e)),
                StechuhrTab::Management => self
                    .management
                    .update(&mut self.shared, ManagementMessage::HandleEvent(e)),
                StechuhrTab::Statistics => self
                    .statistics
                    .update(&mut self.shared, StatsMessage::HandleEvent(e)),
            },
            Message::ScrollSnap => {
                self.log_scroll.snap_to(1.0);
            }
            Message::ToggleFullscreen => {
                self.window_mode = match self.window_mode {
                    window::Mode::Fullscreen => window::Mode::Windowed,
                    _ => window::Mode::Fullscreen,
                }
            }
            Message::Nop => {}
        };
        Command::none()
    }

    // DONE what is '_ in Element<'_, ...>?
    // explicitly elided lifetime. can also be set to 'a
    fn view(&mut self) -> Element<'_, Self::Message> {
        // log area at the bottom
        let logview = Container::new(Stechuhr::get_logview(&mut self.log_scroll, &self.shared))
            .padding(TAB_PADDING)
            .width(Length::Fill)
            .height(Length::FillPortion(20))
            .style(stechuhr::style::LogviewStyle);

        // tab area at the top
        let tab_bar = TabBar::new(self.active_tab as usize, Message::TabSelected)
            .padding(5)
            .text_size(HEADER_SIZE)
            .push(self.timetrack.tab_label())
            .push(self.management.tab_label())
            .push(self.statistics.tab_label());

        // content of the currently active tab
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

        // complete window content
        let content = Column::new().push(tab_bar).push(tab_content).push(logview);

        // content has to be embedded into global modal
        let modal = Modal::new(&mut self.shared.prompt_modal_state, content, move |state| {
            Card::new(Text::new("Information"), Text::new(&state.msg))
                .foot(
                    Button::new(&mut state.ok_button_state, Text::new("Ok"))
                        .width(Length::Shrink)
                        .on_press(Message::ExitPrompt),
                )
                .width(Length::Shrink)
                .on_close(Message::ExitPrompt)
                .into()
        })
        .backdrop(Message::ExitPrompt)
        .on_esc(Message::ExitPrompt);

        let element: Element<'_, Self::Message> = modal.into();
        // uncomment to enable debug mode that shows black outlines of containers
        // element.explain(Color::BLACK)
        element
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            // count every second
            iced::time::every(std::time::Duration::from_secs(1))
                .map(|_| Message::Tick(Local::now())),
            // subscribe to keyboard events
            iced_native::subscription::events_with(|event, status| match (status, event) {
                /* event when closing the window e.g. mod+Shift+q in i3 */
                (_, Event::Window(iced_native::window::Event::CloseRequested)) => {
                    Some(Message::ExitApplication)
                }
                (
                    Status::Ignored,
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        key_code: keyboard::KeyCode::F11,
                        ..
                    }),
                ) => Some(Message::ToggleFullscreen),
                /* We need to be careful to only handle events that have not been captured elsewhere.
                 * Otherwise it can happen that we handle the "enter" again which originally opened the submission modal. */
                (Status::Ignored, e) => Some(Message::HandleEvent(e)),
                (_, _) => None,
            }),
        ])
    }
}

trait Tab {
    type Message;

    fn title(&self) -> String;

    fn tab_label(&self) -> TabLabel;

    /// Displays a tab with common features.
    fn view(&mut self, shared: &mut SharedData) -> Element<'_, Message> {
        // each tab has its name in the upper right corner
        let title = Text::new(self.title()).size(HEADER_SIZE);

        // center the content of each tab
        let content = Container::new(self.content(shared))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .align_y(Vertical::Top)
            .style(stechuhr::style::TabContentStyle);

        Column::new().push(title).push(content).into()
    }

    fn content(&mut self, shared: &mut SharedData) -> Element<'_, Message>;

    fn update(&mut self, shared: &mut SharedData, message: Self::Message) {
        let result = self.update_result(shared, message);
        shared.handle_result(result);
    }

    fn update_result(
        &mut self,
        shared: &mut SharedData,
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

#[cfg(test)]
mod tests {

    use diesel::Connection;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

    fn setup_db() -> diesel::SqliteConnection {
        let connection_url = ":memory:";
        let mut connection = diesel::SqliteConnection::establish(&connection_url).unwrap();
        connection.begin_test_transaction().unwrap();

        connection.run_pending_migrations(MIGRATIONS).unwrap();
        connection
    }

    /// Sanity check that test DB was not changed.
    #[test]
    fn check_db_hash() {
        let mut connection = setup_db();
    }

    /// Create Stechuhr application and simulate passing the 6am barrier.
    #[test]
    fn simulate_6am() {}

    /// Create Stechuhr application and load staff that is already working.
    #[test]
    fn load_working() {}

    /// Create Stechuhr application and load staff that forgot to sign off before 6am.
    #[test]
    fn load_6am() {}
}

// mod dsj {
//     use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
//     use std::error::Error;
//     include!("../../../diesel/src/doctest_setup.rs");

//     #[cfg(feature = "postgres")]
//     fn migration_connection() -> diesel::PgConnection {
//         let connection_url = database_url_from_env("PG_DATABASE_URL");
//         let mut conn = diesel::PgConnection::establish(&connection_url).unwrap();
//         conn.begin_test_transaction().unwrap();
//         conn
//     }

//     #[cfg(feature = "sqlite")]
//     fn migration_connection() -> diesel::SqliteConnection {
//         let connection_url = database_url_from_env("SQLITE_DATABASE_URL");
//         let mut conn = diesel::SqliteConnection::establish(&connection_url).unwrap();
//         conn.begin_test_transaction().unwrap();
//         conn
//     }

//     #[cfg(feature = "mysql")]
//     fn migration_connection() -> diesel::MysqlConnection {
//         let connection_url = database_url_from_env("MYSQL_DATABASE_URL");
//         let mut conn = diesel::MysqlConnection::establish(&connection_url).unwrap();
//         conn
//     }

//     #[cfg(feature = "postgres")]
//     pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../migrations/postgresql");
//     #[cfg(all(feature = "mysql", not(feature = "postgres")))]
//     pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../migrations/mysql");
//     #[cfg(all(feature = "sqlite", not(any(feature = "postgres", feature = "mysql"))))]
//     pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../migrations/sqlite");

//     fn main() {
//         let connection = &mut migration_connection();
//         run_migrations(connection).unwrap();
//     }

//     fn run_migrations(
//         connection: &mut impl MigrationHarness<DB>,
//     ) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
//         #[cfg(feature = "mysql")]
//         connection.revert_all_migrations(MIGRATIONS)?;

//         // This will run the necessary migrations.
//         //
//         // See the documentation for `MigrationHarness` for
//         // all available methods.
//         connection.run_pending_migrations(MIGRATIONS)?;

//         Ok(())
//     }
// }
