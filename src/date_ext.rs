use chrono::{Datelike, NaiveDate};

pub trait NaiveDateExt
where
    Self: Sized,
{
    fn first_dom(self) -> Self;
    fn last_dom(self) -> Self;
}

impl NaiveDateExt for NaiveDate {
    fn first_dom(self) -> Self {
        self.with_day(1).unwrap()
    }

    fn last_dom(self) -> Self {
        let month = self.month();

        let last_day = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => panic!("Month out of range"),
        };
        self.with_day(last_day).unwrap()
    }
}
