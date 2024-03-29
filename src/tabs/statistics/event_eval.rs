use super::{
    time_eval::WorkDuration, PersonHours, PersonHoursCSV, SoftStatisticsError, StaffHours,
    StatisticsError,
};
use crate::{SharedData, StechuhrError};
use chrono::{Date, Local, Locale, NaiveDateTime, NaiveTime, TimeZone};
use std::borrow::Cow;
use stechuhr::{
    date_ext::NaiveDateExt,
    db,
    models::{DBStaffMember, StaffMember, WorkEvent, WorkEventT, WorkStatus},
};

enum EventSMLabel {
    Working(NaiveDateTime),
    Away,
}

/// State machine to compute the WorkDuration of a StaffMember based on a collection of events.
pub struct EventSM<'a> {
    hours_raw: PersonHours<'a>,
    soft_errors: Vec<SoftStatisticsError>,
    label: EventSMLabel,
}

impl<'a> EventSM<'a> {
    pub fn new(staff_member: &'a StaffMember, initial_start_time: Option<NaiveDateTime>) -> Self {
        let label = if let Some(start_time) = initial_start_time {
            EventSMLabel::Working(start_time)
        } else {
            EventSMLabel::Away
        };

        Self {
            hours_raw: PersonHours::new(staff_member),
            soft_errors: Vec::new(),
            label,
        }
    }

    fn append_soft_error(&mut self, error: SoftStatisticsError) {
        self.soft_errors.push(error);
    }

    fn add_time(
        &mut self,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
    ) -> Result<(), StatisticsError> {
        let additional_work_time = WorkDuration::from_start_end_time(start_time, end_time);
        let new_duration = self.hours_raw.duration.checked_add(&additional_work_time)?;
        self.hours_raw.duration = new_duration;
        Ok(())
    }

    pub fn process(&mut self, event: &WorkEventT) -> Result<(), StatisticsError> {
        match self.label {
            EventSMLabel::Away => match event.event {
                WorkEvent::StatusChange(uuid, _, WorkStatus::Working)
                    if self.hours_raw.staff_member.uuid() == uuid =>
                {
                    self.label = EventSMLabel::Working(event.created_at);
                    Ok(())
                }
                WorkEvent::StatusChange(uuid, _, WorkStatus::Away)
                    if self.hours_raw.staff_member.uuid() == uuid =>
                {
                    self.append_soft_error(SoftStatisticsError::AlreadyAway(
                        event.created_at,
                        self.hours_raw.staff_member.name.clone(),
                    ));
                    Ok(())
                }
                _ => Ok(()),
            },
            EventSMLabel::Working(start_time) => match event.event {
                WorkEvent::StatusChange(uuid, _, WorkStatus::Away)
                    if self.hours_raw.staff_member.uuid() == uuid =>
                {
                    self.add_time(start_time, event.created_at)?;
                    self.label = EventSMLabel::Away;
                    Ok(())
                }
                WorkEvent::StatusChange(uuid, _, WorkStatus::Working)
                    if self.hours_raw.staff_member.uuid() == uuid =>
                {
                    self.append_soft_error(SoftStatisticsError::AlreadyWorking(
                        event.created_at,
                        self.hours_raw.staff_member.name.clone(),
                    ));
                    Ok(())
                }
                WorkEvent::_6am => {
                    self.append_soft_error(SoftStatisticsError::StaffStillWorking(
                        event.created_at,
                        self.hours_raw.staff_member.name.clone(),
                    ));
                    self.add_time(start_time, event.created_at)?;
                    self.label = EventSMLabel::Away;
                    Ok(())
                }
                _ => Ok(()),
            },
        }
    }

    pub fn finish(self) -> (PersonHours<'a>, Vec<SoftStatisticsError>) {
        (self.hours_raw, self.soft_errors)
    }
}

pub fn evaluate_hours_for_month(
    shared: &mut SharedData,
    date: Date<Local>,
) -> Result<StaffHours, StechuhrError> {
    // The start and end time will be first and last day of the selected month, respectively.
    let _6am = NaiveTime::from_hms(6, 0, 0);
    let start_time = date.naive_local().first_dom().and_time(_6am);
    let end_time = date.naive_local().last_dom().succ().and_time(_6am);

    let start_time_local = Local.from_local_datetime(&start_time).unwrap();
    let end_time_local = Local.from_local_datetime(&end_time).unwrap();

    shared.log_info(format!(
        "Starte Auswertung für {}, zwischen {} und {}",
        date.format_localized("%B %Y", Locale::de_DE).to_string(),
        start_time_local
            .format_localized("%d. %B (%R)", Locale::de_DE)
            .to_string(),
        end_time_local
            .format_localized("%d. %B (%R)", Locale::de_DE)
            .to_string()
    ));

    evaluate_hours_for_time(shared, start_time, end_time)
}

fn evaluate_hours_for_time(
    shared: &mut SharedData,
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
) -> Result<StaffHours, StechuhrError> {
    // Load events before the evaluation period in order to set the correct initial status for staff members.
    let previous_events = db::load_events_between(None, Some(start_time), &mut shared.connection);
    let events = db::load_events_between(Some(start_time), Some(end_time), &mut shared.connection);
    let raw_staff = shared
        .staff
        .iter()
        // Only do calculation for visible staff members.
        .filter(|staff_member| staff_member.is_visible)
        // Turn everyone into DBStaffMember to forget the working status.
        .map(|staff_member| DBStaffMember::from(Cow::Borrowed(staff_member)))
        .collect::<Vec<_>>();

    evaluate_hours_for_events(raw_staff, &events, &previous_events, start_time)
}

fn evaluate_hours_for_events(
    raw_staff: Vec<DBStaffMember>,
    events: &[WorkEventT],
    previous_events: &[WorkEventT],
    start_time: NaiveDateTime,
) -> Result<StaffHours, StechuhrError> {
    // Set the initial status for staff members.
    // Atm we only do evaluation starting at 6am on the 1st of the month, so no one will be working as we set everyone to non-working at 6am.
    let staff = raw_staff
        .into_iter()
        // Compute the initial status.
        .map(|staff_member| db::staff_member_compute_status(staff_member, &previous_events))
        .collect::<Vec<_>>();

    let (hours, soft_errors): (Vec<PersonHours>, Vec<Vec<SoftStatisticsError>>) = staff
        .iter()
        // Associate with each staff member a WorkDuration, which counts the minutes of work time
        .map(move |staff_member| evaluate_hours_for_staff_member(staff_member, &events, start_time))
        .collect::<Result<Vec<(PersonHours, Vec<SoftStatisticsError>)>, StatisticsError>>()?
        .into_iter()
        .unzip();

    let hours_csv: Vec<PersonHoursCSV> = hours
        .into_iter()
        // Transform the calculated WorkDuration into a PersonHours struct for serialization.
        .map(PersonHoursCSV::from)
        .collect();

    Ok(StaffHours {
        hours_csv,
        soft_errors: soft_errors.into_iter().flatten().collect(),
    })
}

/// Create a EventSM state machine and feed all WorkEventT events to it to compute the StaffMemberHours.
fn evaluate_hours_for_staff_member<'a>(
    staff_member: &'a StaffMember,
    events: &[WorkEventT],
    start_time: NaiveDateTime,
) -> Result<(PersonHours<'a>, Vec<SoftStatisticsError>), StatisticsError> {
    let initial_start_time = if staff_member.status == WorkStatus::Working {
        Some(start_time)
    } else {
        None
    };

    let mut event_sm = EventSM::new(staff_member, initial_start_time);

    for event in events {
        event_sm.process(event)?;
    }

    Ok(event_sm.finish())
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use stechuhr::models::{DBStaffMember, WorkEvent, WorkEventT, WorkStatus};

    use crate::tabs::statistics::SoftStatisticsError;

    use super::evaluate_hours_for_events;

    /// evaluate_hours_for_events where staff member has no StatusChange events.
    #[test]
    fn zero_worktime() {
        let raw_staff = vec![DBStaffMember::new(
            1,
            String::from("Aaron"),
            String::from("1111"),
            String::from("1111111111"),
            true,
        )];
        let events = vec![];
        let previous_events = vec![];
        let start_time = NaiveDate::from_ymd(2000, 1, 1).and_hms(20, 0, 0);

        let hours =
            evaluate_hours_for_events(raw_staff, &events, &previous_events, start_time).unwrap();

        assert!(hours.errors().is_empty());

        assert_eq!(hours.hours()[0].minutes_1, 0);
        assert_eq!(hours.hours()[0].minutes_2, 0);
        assert_eq!(hours.hours()[0].minutes_3, 0);
    }

    /// evaluate_hours_for_events where staff member has some worktime in all slots.
    #[test]
    fn normal_worktime() {
        let raw_staff = vec![DBStaffMember::new(
            1,
            String::from("Aaron"),
            String::from("1111"),
            String::from("1111111111"),
            true,
        )];
        let events = vec![
            WorkEventT::new(
                1,
                NaiveDate::from_ymd(2000, 1, 1).and_hms(18, 0, 0),
                WorkEvent::StatusChange(1, String::from("Aaron"), WorkStatus::Working),
            ),
            WorkEventT::new(
                2,
                NaiveDate::from_ymd(2000, 1, 1).and_hms(20, 30, 0),
                WorkEvent::StatusChange(1, String::from("Aaron"), WorkStatus::Away),
            ),
            WorkEventT::new(
                3,
                NaiveDate::from_ymd(2000, 1, 1).and_hms(23, 0, 0),
                WorkEvent::StatusChange(1, String::from("Aaron"), WorkStatus::Working),
            ),
            WorkEventT::new(
                4,
                NaiveDate::from_ymd(2000, 1, 2).and_hms(2, 0, 0),
                WorkEvent::StatusChange(1, String::from("Aaron"), WorkStatus::Away),
            ),
            WorkEventT::new(
                5,
                NaiveDate::from_ymd(2000, 1, 2).and_hms(3, 0, 0),
                WorkEvent::StatusChange(1, String::from("Aaron"), WorkStatus::Working),
            ),
            WorkEventT::new(
                6,
                NaiveDate::from_ymd(2000, 1, 2).and_hms(5, 0, 0),
                WorkEvent::StatusChange(1, String::from("Aaron"), WorkStatus::Away),
            ),
        ];
        let previous_events = vec![];
        let start_time = NaiveDate::from_ymd(2000, 1, 1).and_hms(6, 0, 0);

        let hours =
            evaluate_hours_for_events(raw_staff, &events, &previous_events, start_time).unwrap();

        assert!(hours.errors().is_empty());

        assert_eq!(hours.hours()[0].minutes_1, 3 * 60);
        assert_eq!(hours.hours()[0].minutes_2, 1 * 60 + 30);
        assert_eq!(hours.hours()[0].minutes_3, 3 * 60);
    }

    /// evaluate_hours_for_events where staff member has been working before the time starts.
    #[test]
    fn worktime_start() {
        let raw_staff = vec![DBStaffMember::new(
            1,
            String::from("Aaron"),
            String::from("1111"),
            String::from("1111111111"),
            true,
        )];
        let events = vec![WorkEventT::new(
            2,
            NaiveDate::from_ymd(2000, 1, 2).and_hms(1, 0, 0),
            WorkEvent::StatusChange(1, String::from("Aaron"), WorkStatus::Away),
        )];
        let previous_events = vec![WorkEventT::new(
            1,
            NaiveDate::from_ymd(2000, 1, 1).and_hms(18, 0, 0),
            WorkEvent::StatusChange(1, String::from("Aaron"), WorkStatus::Working),
        )];
        let start_time = NaiveDate::from_ymd(2000, 1, 1).and_hms(19, 0, 0);

        let hours =
            evaluate_hours_for_events(raw_staff, &events, &previous_events, start_time).unwrap();

        assert!(hours.errors().is_empty());

        assert_eq!(hours.hours()[0].minutes_1, 1 * 60);
        assert_eq!(hours.hours()[0].minutes_2, 4 * 60);
        assert_eq!(hours.hours()[0].minutes_3, 1 * 60);
    }

    /// evaluate_hours_for_events where staff member works through a 6am barrier.
    #[test]
    fn error_worktime_6am() {
        let raw_staff = vec![DBStaffMember::new(
            1,
            String::from("Aaron"),
            String::from("1111"),
            String::from("1111111111"),
            true,
        )];
        let events = vec![
            WorkEventT::new(
                1,
                NaiveDate::from_ymd(2000, 1, 2).and_hms(5, 0, 0),
                WorkEvent::StatusChange(1, String::from("Aaron"), WorkStatus::Working),
            ),
            WorkEventT::new(
                2,
                NaiveDate::from_ymd(2000, 1, 2).and_hms(5, 59, 59),
                WorkEvent::_6am,
            ),
        ];
        let previous_events = vec![];
        let start_time = NaiveDate::from_ymd(2000, 1, 1).and_hms(6, 0, 0);

        let hours =
            evaluate_hours_for_events(raw_staff, &events, &previous_events, start_time).unwrap();

        assert_eq!(
            hours.errors()[0],
            SoftStatisticsError::StaffStillWorking(
                NaiveDate::from_ymd(2000, 1, 2).and_hms(5, 59, 59),
                String::from("Aaron")
            )
        );

        assert_eq!(hours.hours()[0].minutes_1, 1 * 60);
        assert_eq!(hours.hours()[0].minutes_2, 0);
        assert_eq!(hours.hours()[0].minutes_3, 0);
    }

    /// evaluate_hours_for_events where staff member has two consecutive StatusChange events to Working
    #[test]
    fn error_worktime_already_working() {
        let raw_staff = vec![DBStaffMember::new(
            1,
            String::from("Aaron"),
            String::from("1111"),
            String::from("1111111111"),
            true,
        )];
        let events = vec![
            WorkEventT::new(
                1,
                NaiveDate::from_ymd(2000, 1, 2).and_hms(5, 0, 0),
                WorkEvent::StatusChange(1, String::from("Aaron"), WorkStatus::Working),
            ),
            WorkEventT::new(
                2,
                NaiveDate::from_ymd(2000, 1, 2).and_hms(5, 30, 0),
                WorkEvent::StatusChange(1, String::from("Aaron"), WorkStatus::Working),
            ),
            WorkEventT::new(
                3,
                NaiveDate::from_ymd(2000, 1, 2).and_hms(5, 59, 59),
                WorkEvent::_6am,
            ),
        ];
        let previous_events = vec![];
        let start_time = NaiveDate::from_ymd(2000, 1, 1).and_hms(6, 0, 0);

        let hours =
            evaluate_hours_for_events(raw_staff, &events, &previous_events, start_time).unwrap();

        assert_eq!(
            hours.errors()[0],
            SoftStatisticsError::AlreadyWorking(
                NaiveDate::from_ymd(2000, 1, 2).and_hms(5, 30, 00),
                String::from("Aaron")
            )
        );

        assert_eq!(hours.hours()[0].minutes_1, 1 * 60);
        assert_eq!(hours.hours()[0].minutes_2, 0);
        assert_eq!(hours.hours()[0].minutes_3, 0);
    }

    /// evaluate_hours_for_events where staff member has two consecutive StatusChange events to Away
    #[test]
    fn error_worktime_already_away() {
        let raw_staff = vec![DBStaffMember::new(
            1,
            String::from("Aaron"),
            String::from("1111"),
            String::from("1111111111"),
            true,
        )];
        let events = vec![
            WorkEventT::new(
                1,
                NaiveDate::from_ymd(2000, 1, 2).and_hms(5, 0, 0),
                WorkEvent::StatusChange(1, String::from("Aaron"), WorkStatus::Working),
            ),
            WorkEventT::new(
                2,
                NaiveDate::from_ymd(2000, 1, 2).and_hms(5, 30, 0),
                WorkEvent::StatusChange(1, String::from("Aaron"), WorkStatus::Away),
            ),
            WorkEventT::new(
                3,
                NaiveDate::from_ymd(2000, 1, 2).and_hms(5, 45, 0),
                WorkEvent::StatusChange(1, String::from("Aaron"), WorkStatus::Away),
            ),
        ];
        let previous_events = vec![];
        let start_time = NaiveDate::from_ymd(2000, 1, 1).and_hms(6, 0, 0);

        let hours =
            evaluate_hours_for_events(raw_staff, &events, &previous_events, start_time).unwrap();

        assert_eq!(
            hours.errors()[0],
            SoftStatisticsError::AlreadyAway(
                NaiveDate::from_ymd(2000, 1, 2).and_hms(5, 45, 00),
                String::from("Aaron")
            )
        );

        assert_eq!(hours.hours()[0].minutes_1, 30);
        assert_eq!(hours.hours()[0].minutes_2, 0);
        assert_eq!(hours.hours()[0].minutes_3, 0);
    }
}
