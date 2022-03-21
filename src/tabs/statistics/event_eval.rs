use crate::StechuhrError;

use super::{time_eval::WorkDuration, StatisticsError};
use chrono::NaiveDateTime;
use stechuhr::models::{StaffMember, WorkEvent, WorkEventT, WorkStatus};

enum EventSMLabel {
    Working(NaiveDateTime),
    Away,
}

pub struct EventSM<'a> {
    staff_member: &'a StaffMember,
    duration: WorkDuration,
    label: EventSMLabel,
    has_errors: bool,
}

impl<'a> EventSM<'a> {
    pub fn new(staff_member: &'a StaffMember) -> Self {
        Self {
            staff_member,
            duration: WorkDuration::zero(),
            label: EventSMLabel::Away,
            has_errors: false,
        }
    }

    fn log_error(&mut self, msg: String) {
        log::error!("{}", msg);
        self.has_errors = true;
    }

    fn add_time(&mut self, start_time: NaiveDateTime, end_time: NaiveDateTime) {
        let additional_work_time = WorkDuration::from_start_end_time(start_time, end_time);
        self.duration = self.duration.checked_add(&additional_work_time).unwrap();
    }

    pub fn process(&mut self, event: &WorkEventT) {
        match self.label {
            EventSMLabel::Away => match event.event {
                WorkEvent::StatusChange(uuid, _, WorkStatus::Working)
                    if self.staff_member.uuid() == uuid =>
                {
                    self.label = EventSMLabel::Working(event.created_at)
                }
                WorkEvent::StatusChange(uuid, _, WorkStatus::Away)
                    if self.staff_member.uuid() == uuid =>
                {
                    self.log_error(format!("Encountered StatusChange to Away at {} while staff {} was already not working.", event.created_at, self.staff_member.uuid()));
                }
                _ => {}
            },
            EventSMLabel::Working(start_time) => match event.event {
                WorkEvent::StatusChange(uuid, _, WorkStatus::Away)
                    if self.staff_member.uuid() == uuid =>
                {
                    self.add_time(start_time, event.created_at);
                    self.label = EventSMLabel::Away;
                }
                WorkEvent::StatusChange(uuid, _, WorkStatus::Working)
                    if self.staff_member.uuid() == uuid =>
                {
                    self.log_error(format!("Encountered StatusChange to Working at {} while staff {} was already working.", event.created_at, self.staff_member.uuid()));
                }
                WorkEvent::EventOver => {
                    self.log_error(format!(
                        "Encountered EventOver at {} while staff {} was still working. Assume they worked until that time.",
                        event.created_at,
                        self.staff_member.uuid()
                    ));
                    self.add_time(start_time, event.created_at);
                    self.label = EventSMLabel::Away;
                }
                _ => {}
            },
        }
    }

    pub fn finish(self) -> Result<(&'a StaffMember, WorkDuration), StatisticsError> {
        // sanity check that during evaluation, no staff member is still working
        match self.label {
            EventSMLabel::Working(_) => Err(StatisticsError::StaffStillWorking(
                self.staff_member.name.clone(),
            )),
            EventSMLabel::Away => Ok((self.staff_member, self.duration)),
        }
    }
}
