use std::cmp::min;

use chrono::{Duration, NaiveDateTime, Timelike};

type Secs = u32;
const SECS_PER_HOUR: Secs = 60 * 60;

enum DurationSMLabel {
    L9_22,
    L22_24,
    L24_9,
}

impl DurationSMLabel {
    /* Compute the number of seconds in one time period */
    fn to_duration_seconds(&self) -> Secs {
        match self {
            Self::L9_22 => (22 - 9) * SECS_PER_HOUR,
            Self::L22_24 => (24 - 22) * SECS_PER_HOUR,
            Self::L24_9 => 9 * SECS_PER_HOUR,
        }
    }

    /* Compute the first second of each time period */
    fn to_start_seconds(&self) -> Secs {
        match self {
            Self::L9_22 => 9 * SECS_PER_HOUR,
            Self::L22_24 => 22 * SECS_PER_HOUR,
            Self::L24_9 => 0 * SECS_PER_HOUR,
        }
    }

    /* Compute a label for a number of seconds between midnight and midnight of the following day */
    fn from_absolute_seconds(s: Secs) -> Self {
        assert!(s < 24 * SECS_PER_HOUR);

        if s < 9 * SECS_PER_HOUR {
            Self::L24_9
        } else if s < 22 * SECS_PER_HOUR {
            Self::L9_22
        } else {
            Self::L22_24
        }
    }
}

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
            DurationSMLabel::L9_22 => self.label = DurationSMLabel::L22_24,
            DurationSMLabel::L22_24 => self.label = DurationSMLabel::L24_9,
            DurationSMLabel::L24_9 => self.label = DurationSMLabel::L9_22,
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
            DurationSMLabel::L9_22 => self.buckets[0] += s,
            DurationSMLabel::L22_24 => self.buckets[1] += s,
            DurationSMLabel::L24_9 => self.buckets[2] += s,
        }
        self.current_seconds = 0;
    }

    /* Convert to a WorkDuration */
    fn to_work_duration(&self) -> WorkDuration {
        let [s1, s2, s3] = self.buckets;
        WorkDuration([
            Duration::seconds(s1 as i64),
            Duration::seconds(s2 as i64),
            Duration::seconds(s3 as i64),
        ])
    }
}

#[derive(Debug)]
pub struct WorkDuration([Duration; 3]);

impl WorkDuration {
    pub fn zero() -> Self {
        WorkDuration([Duration::zero(), Duration::zero(), Duration::zero()])
    }

    pub fn checked_add(&self, rhs: &Self) -> Option<Self> {
        let WorkDuration([t1, t2, t3]) = self;
        let WorkDuration([s1, s2, s3]) = rhs;

        Some(WorkDuration([
            s1.checked_add(t1).unwrap(),
            s2.checked_add(t2).unwrap(),
            s3.checked_add(t3).unwrap(),
        ]))
    }

    pub fn from_start_end_time(start_time: NaiveDateTime, end_time: NaiveDateTime) -> Self {
        // TODO ensure that naivedatetime is in correct timezone
        // 9 Uhr - 22 Uhr -> bucket 1
        // 22 Uhr - 24 Uhr -> bucket 2
        // 24 Uhr - 9 Uhr -> bucket 3
        //
        // like in os
        // compute total number of seconds in duration
        // get start seconds in day
        // while total_seconds > 0
        //   get seconds until next threshold
        //   put then into respective bucket
        //   subtract from total
        assert!(start_time < end_time);

        let current_seconds = start_time.num_seconds_from_midnight();
        let mut seconds_remaining =
            u32::try_from(end_time.signed_duration_since(start_time).num_seconds()).unwrap();
        let mut sm = DurationSM::new(current_seconds);

        while seconds_remaining > 0 {
            let s = min(seconds_remaining, sm.get_current_seconds());
            seconds_remaining -= s;
            sm.add_time(s);
            sm.next_step();
        }

        sm.to_work_duration()
    }

    pub fn num_minutes(&self) -> [i64; 4] {
        // TODO round up minutes
        let WorkDuration([t1, t2, t3]) = self;
        let minutes_1 = t1.num_minutes();
        let minutes_2 = t2.num_minutes();
        let minutes_3 = t3.num_minutes();
        let minutes_weigthed = (1.0 * (minutes_1 as f64)
            + 1.25 * (minutes_2 as f64)
            + 1.40 * (minutes_3 as f64)) as i64;

        [minutes_1, minutes_2, minutes_3, minutes_weigthed]
    }
}
