use chrono::{DateTime, Local, Locale};
use diesel::prelude::*;
use iced::{
    button, executor, text_input, Application, Button, Column, Command, Container, Element,
    HorizontalAlignment, Length, Row, Settings, Subscription, Text, TextInput,
};
use iced_aw::{modal, Card, Modal};
use iced_native::{window, Event};
use std::collections::HashMap;
use stechuhr::models::*;

pub fn main() -> iced::Result {
    let connection = stechuhr::establish_connection();

    Stechuhr::run(Settings {
        // a.d. set this so that we can handle the close request ourselves to sync data to db
        exit_on_close_request: false,
        ..Settings::with_flags(connection)
    })
}

enum Menu {
    Main,
}

struct Stechuhr {
    current_time: DateTime<Local>,
    staff: HashMap<i32, StaffMember>,
    menu: Menu,
    events: Vec<WorkEventT>,
    break_input_value: String,
    break_input_uuid: Option<i32>,
    // widget states
    end_party_button_state: button::State,
    exit_button_state: button::State,
    break_input_state: text_input::State,
    break_modal_state: modal::State<BreakModalState>,
    connection: SqliteConnection,
    should_exit: bool,
}

#[derive(Default)]
struct BreakModalState {
    confirm_state: button::State,
    cancel_state: button::State,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(DateTime<Local>),
    ChangeBreakInput(String),
    SubmitBreakInput,
    ConfirmSubmitBreakInput,
    CancelSubmitBreakInput,
    EndEvent,
    ExitApplication,
}

impl Stechuhr {
    fn log_event(&mut self, event: WorkEvent) {
        self.events.push(WorkEventT {
            timestamp: Local::now(),
            event: event,
        });
    }
}

impl Application for Stechuhr {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = SqliteConnection;

    fn should_exit(&self) -> bool {
        self.should_exit
    }

    fn new(connection: SqliteConnection) -> (Self, Command<Message>) {
        use stechuhr::schema::staff::dsl::*;

        let staff_db = staff
            .load::<StaffMember>(&connection)
            .expect("Error loading staff from DB");

        (
            Self {
                current_time: Local::now(),
                staff: StaffMember::to_hash_map(staff_db),
                menu: Menu::Main,
                events: Vec::new(),
                break_input_value: String::new(),
                break_input_uuid: None,
                end_party_button_state: button::State::default(),
                exit_button_state: button::State::default(),
                break_input_state: text_input::State::new(),
                // TODO why does State not take the type argument <BreakModalState> here?
                break_modal_state: modal::State::default(),
                connection: connection,
                should_exit: false,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Counter - Iced")
    }

    fn update(&mut self, message: Message, _clipboard: &mut iced::Clipboard) -> Command<Message> {
        use stechuhr::schema::staff::dsl::*;

        // always focus the input
        self.break_input_state.focus();

        match message {
            Message::Tick(local_time) => {
                if local_time > self.current_time {
                    self.current_time = local_time;
                }
            }
            Message::ChangeBreakInput(value) => {
                self.break_input_value = value;
            }
            Message::SubmitBreakInput => {
                let input = self.break_input_value.trim();

                if input.len() == 4 || input.len() == 6 {
                    if let Some(uuid) = StaffMember::get_uuid_by_pin_or_card_id(&self.staff, input)
                    {
                        self.break_modal_state.show(true);
                        self.break_input_uuid = Some(uuid);
                    } else {
                        println!("No matching staff member found for input {}.", input);
                    }
                } else {
                    println!("Malformed input {}.", input);
                }
            }
            Message::ConfirmSubmitBreakInput => {
                if let Some(break_uuid) = self.break_input_uuid {
                    match self.staff.get_mut(&break_uuid) {
                        Some(staff_member) => {
                            let new_status = !staff_member.status;
                            staff_member.status = new_status;
                            self.log_event(WorkEvent::StatusChange(break_uuid, new_status));
                            self.break_modal_state.show(false);
                            self.break_input_uuid = None;
                            self.break_input_value.clear();
                        }
                        None => {
                            println!("No matching staff member found for uuid {}.", break_uuid);
                        }
                    }
                }
            }
            Message::CancelSubmitBreakInput => {
                self.break_modal_state.show(false);
                self.break_input_uuid = None;
                self.break_input_value.clear();
            }
            Message::EndEvent => {
                self.log_event(WorkEvent::EventOver);
            }
            Message::ExitApplication => {
                for staff_member in self.staff.values() {
                    diesel::update(staff_member)
                        .set(status.eq(staff_member.status))
                        .execute(&self.connection)
                        .expect(&format!("Error updating staff {}", staff_member.name));
                }

                self.should_exit = true;
            }
        };
        Command::none()
    }

    // TODO what is '_?
    fn view(&mut self) -> Element<'_, Message> {
        let mut staff_view = Column::new();
        for staff_member in self.staff.values() {
            staff_view = staff_view.push(Text::new(format!(
                "{}: {}",
                staff_member.name,
                staff_member.status.to_string()
            )));
        }

        let content = Container::new(
            Column::new()
                .padding(20)
                .push(Text::new(
                    self.current_time
                        .format_localized("%T, %A, %e. %B %Y", Locale::de_DE)
                        .to_string(),
                ))
                .push(staff_view)
                .push(
                    TextInput::new(
                        &mut self.break_input_state,
                        "Deine PIN",
                        &self.break_input_value,
                        Message::ChangeBreakInput,
                    )
                    .on_submit(Message::SubmitBreakInput),
                )
                .push(
                    Button::new(
                        &mut self.end_party_button_state,
                        Text::new("Event beenden")
                            .horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .on_press(Message::EndEvent),
                )
                .push(
                    Button::new(
                        &mut self.exit_button_state,
                        Text::new("Exit").horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .on_press(Message::ExitApplication),
                ),
        );

        let break_modal_value = if let Some(break_uuid) = self.break_input_uuid {
            match self.staff.get(&break_uuid) {
                Some(staff_member) => format!(
                    "{} wird auf '{}' gesetzt. Korrekt?",
                    staff_member.name,
                    WorkStatus::from_bool(staff_member.status).toggle()
                ),
                None => {
                    String::from("Error: Kein Mitarbeiter gefunden. Bitte Adrian Bescheid sagen.")
                }
            }
        } else {
            String::from("Warnung: kein Mitarbeiter ausgewählt.")
        };

        let modal = Modal::new(&mut self.break_modal_state, content, move |state| {
            Card::new(
                Text::new("Änderung des Arbeitsstatus"),
                Text::new(break_modal_value.clone()),
            )
            .foot(
                Row::new()
                    .spacing(10)
                    .padding(5)
                    .width(Length::Fill)
                    .push(
                        Button::new(
                            &mut state.confirm_state,
                            Text::new("Ok").horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .width(Length::Fill)
                        .on_press(Message::ConfirmSubmitBreakInput),
                    )
                    .push(
                        Button::new(
                            &mut state.cancel_state,
                            Text::new("Zurück").horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .width(Length::Fill)
                        .on_press(Message::CancelSubmitBreakInput),
                    ),
            )
            .max_width(300)
            //.width(Length::Shrink)
            .on_close(Message::CancelSubmitBreakInput)
            .into()
        })
        .backdrop(Message::CancelSubmitBreakInput)
        .on_esc(Message::CancelSubmitBreakInput)
        .into();

        modal
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::time::every(std::time::Duration::from_secs(1))
                .map(|_| Message::Tick(Local::now())),
            iced_native::subscription::events_with(|event, _status| match dbg!(event) {
                Event::Window(window::Event::CloseRequested) => Some(Message::ExitApplication),
                _ => None,
            }),
        ])
    }
}
