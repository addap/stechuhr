use super::StatisticsError;
use chrono::{Duration, NaiveDateTime, Timelike};
use std::cmp::min;

type Secs = i64;
const SECS_PER_HOUR: Secs = 60 * 60;

enum DurationSMLabel {
    L4_20,
    L20_24,
    L24_4,
}

impl DurationSMLabel {
    /* Compute the number of seconds in one time period */
    fn to_duration_seconds(&self) -> Secs {
        match self {
            Self::L4_20 => (20 - 4) * SECS_PER_HOUR,
            Self::L20_24 => (24 - 20) * SECS_PER_HOUR,
            Self::L24_4 => (4 - 0) * SECS_PER_HOUR,
        }
    }

    /* Compute the first second of each time period */
    fn to_start_seconds(&self) -> Secs {
        match self {
            Self::L4_20 => 4 * SECS_PER_HOUR,
            Self::L20_24 => 20 * SECS_PER_HOUR,
            Self::L24_4 => 0 * SECS_PER_HOUR,
        }
    }

    /* Compute a label for a number of seconds between midnight and midnight of the following day */
    fn from_absolute_seconds(s: Secs) -> Self {
        assert!(s < 24 * SECS_PER_HOUR);

        if s < 4 * SECS_PER_HOUR {
            Self::L24_4
        } else if s < 20 * SECS_PER_HOUR {
            Self::L4_20
        } else {
            Self::L20_24
        }
    }
}

/// State machine to distribute seconds between two datetimes into buckets.
struct DurationSM {
    buckets: [Secs; 3],
    label: DurationSMLabel,
    current_seconds: Secs, /* offset within the current time period (only used at start if starting time is not aligned) */
}

impl DurationSM {
    /* Initialize a state machine from an initial seconds value to choose the starting label. */
    fn new(start_seconds: Secs) -> Self {
        assert!(start_seconds < 24 * SECS_PER_HOUR);
        let label = DurationSMLabel::from_absolute_seconds(start_seconds);
        let current_seconds = start_seconds - label.to_start_seconds();

        Self {
            buckets: [0, 0, 0],
            label,
            current_seconds,
        }
    }

    /* Advance to the next time period. */
    fn next_step(&mut self) {
        match self.label {
            DurationSMLabel::L4_20 => self.label = DurationSMLabel::L20_24,
            DurationSMLabel::L20_24 => self.label = DurationSMLabel::L24_4,
            DurationSMLabel::L24_4 => self.label = DurationSMLabel::L4_20,
        }
    }

    /* Returns the number of seconds in the current time period. */
    fn get_current_seconds(&self) -> Secs {
        self.label.to_duration_seconds() - self.current_seconds
    }

    /* Compute the number of time that can be added in the current time period and add it to the current bucket.
     * The time that can be added must be less or equal to the iven total number of seconds left. */
    fn add_time(&mut self, s: Secs) {
        match self.label {
            DurationSMLabel::L4_20 => self.buckets[0] += s,
            DurationSMLabel::L20_24 => self.buckets[1] += s,
            DurationSMLabel::L24_4 => self.buckets[2] += s,
        }
        self.current_seconds = 0;
    }

    /* Convert to a WorkDuration */
    fn to_work_duration(&self) -> WorkDuration {
        let [s1, s2, s3] = self.buckets;
        WorkDuration([
            Duration::seconds(s1),
            Duration::seconds(s2),
            Duration::seconds(s3),
        ])
    }
}

#[derive(Debug)]
pub struct WorkDuration([Duration; 3]);

impl WorkDuration {
    pub fn zero() -> Self {
        WorkDuration([Duration::zero(), Duration::zero(), Duration::zero()])
    }

    pub fn checked_add(&self, rhs: &Self) -> Result<Self, StatisticsError> {
        let WorkDuration([t1, t2, t3]) = self;
        let WorkDuration([s1, s2, s3]) = rhs;

        let r1 = s1
            .checked_add(t1)
            .ok_or(StatisticsError::DurationError(*s1, *t1))?;
        let r2 = s2
            .checked_add(t2)
            .ok_or(StatisticsError::DurationError(*s2, *t2))?;
        let r3 = s3
            .checked_add(t3)
            .ok_or(StatisticsError::DurationError(*s3, *t3))?;
        Ok(WorkDuration([r1, r2, r3]))
    }

    pub fn from_start_end_time(start_time: NaiveDateTime, end_time: NaiveDateTime) -> Self {
        // TODO ensure that naivedatetime is in correct timezone
        // 4 Uhr - 20 Uhr -> bucket 1
        // 20 Uhr - 24 Uhr -> bucket 2
        // 24 Uhr - 4 Uhr -> bucket 3
        //
        // like in os
        // compute total number of seconds in duration
        // get start seconds in day
        // while total_seconds > 0
        //   get seconds until next threshold
        //   put then into respective bucket
        //   subtract from total
        assert!(start_time < end_time);

        let current_seconds = start_time.num_seconds_from_midnight() as i64;
        // add one second since we're including the end.
        let mut seconds_remaining = end_time.signed_duration_since(start_time).num_seconds() + 1;
        let mut sm = DurationSM::new(current_seconds);

        while seconds_remaining > 0 {
            let s = min(seconds_remaining, sm.get_current_seconds());
            seconds_remaining -= s;
            sm.add_time(s);
            sm.next_step();
        }

        sm.to_work_duration()
    }

    pub fn num_minutes(&self) -> [i64; 3] {
        let WorkDuration([t1, t2, t3]) = self;
        let minutes_1 = t1.num_minutes();
        let minutes_2 = t2.num_minutes();
        let minutes_3 = t3.num_minutes();

        [minutes_1, minutes_2, minutes_3]
    }
}
