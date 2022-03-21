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
}

impl<'a> EventSM<'a> {
    pub fn new(staff_member: &'a StaffMember) -> Self {
        Self {
            staff_member,
            duration: WorkDuration::zero(),
            label: EventSMLabel::Away,
        }
    }

    fn add_time(
        &mut self,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
    ) -> Result<(), StatisticsError> {
        let additional_work_time = WorkDuration::from_start_end_time(start_time, end_time);
        let new_duration = self.duration.checked_add(&additional_work_time)?;
        self.duration = new_duration;
        Ok(())
    }

    pub fn process(&mut self, event: &WorkEventT) -> Result<(), StatisticsError> {
        match self.label {
            EventSMLabel::Away => match event.event {
                WorkEvent::StatusChange(uuid, _, WorkStatus::Working)
                    if self.staff_member.uuid() == uuid =>
                {
                    self.label = EventSMLabel::Working(event.created_at);
                    Ok(())
                }
                _ => Ok(()),
            },
            EventSMLabel::Working(start_time) => match event.event {
                WorkEvent::StatusChange(uuid, _, WorkStatus::Away)
                    if self.staff_member.uuid() == uuid =>
                {
                    self.add_time(start_time, event.created_at)?;
                    self.label = EventSMLabel::Away;
                    Ok(())
                }
                WorkEvent::StatusChange(uuid, _, WorkStatus::Working)
                    if self.staff_member.uuid() == uuid =>
                {
                    Err(StatisticsError::AlreadyWorking(
                        event.created_at,
                        self.staff_member.name.clone(),
                    ))
                }
                WorkEvent::EventOver => Err(StatisticsError::OverWhileWorking(
                    event.created_at,
                    self.staff_member.name.clone(),
                )),
                _ => Ok(()),
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
