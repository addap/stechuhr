use super::{time_eval::WorkDuration, SoftStatisticsError, StatisticsError};
use chrono::NaiveDateTime;
use stechuhr::models::{StaffMember, WorkEvent, WorkEventT, WorkStatus};

/// The result of the computation done by EventSM.
pub struct PersonHoursRaw<'a> {
    pub staff_member: &'a StaffMember,
    pub duration: WorkDuration,
}

impl<'a> PersonHoursRaw<'a> {
    fn new(staff_member: &'a StaffMember) -> Self {
        Self {
            staff_member,
            duration: WorkDuration::zero(),
        }
    }
}

enum EventSMLabel {
    Working(NaiveDateTime),
    Away,
}

/// State machine to compute the WorkDuration of a StaffMember based on a collection of events.
pub struct EventSM<'a> {
    hours_raw: PersonHoursRaw<'a>,
    soft_errors: Vec<SoftStatisticsError>,
    label: EventSMLabel,
}

impl<'a> EventSM<'a> {
    pub fn new(staff_member: &'a StaffMember) -> Self {
        Self {
            hours_raw: PersonHoursRaw::new(staff_member),
            soft_errors: Vec::new(),
            label: EventSMLabel::Away,
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
                WorkEvent::EventOver => {
                    self.append_soft_error(SoftStatisticsError::OverWhileWorking(
                        event.created_at,
                        self.hours_raw.staff_member.name.clone(),
                    ));
                    Ok(())
                }
                _ => Ok(()),
            },
        }
    }

    pub fn finish(mut self) -> (PersonHoursRaw<'a>, Vec<SoftStatisticsError>) {
        // sanity check that during evaluation, no staff member is still working
        match self.label {
            EventSMLabel::Working(_) => {
                self.append_soft_error(SoftStatisticsError::StaffStillWorking(
                    self.hours_raw.staff_member.name.clone(),
                ));
            }
            _ => {}
        }

        (self.hours_raw, self.soft_errors)
    }
}
