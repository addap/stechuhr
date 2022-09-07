use chrono::{NaiveDate, NaiveTime};
use dotenv::dotenv;
use std::error::Error;
use stechuhr::{
    db,
    models::{NewWorkEventT, WorkEvent},
};

fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    env_logger::init();

    let mut connection = db::establish_connection();

    let _55959am = NaiveTime::from_hms(5, 59, 59);
    let mut current_date = NaiveDate::from_yo(2020, 1);

    for _ in 0..365 * 30 {
        db::insert_event(
            &NewWorkEventT::new(current_date.and_time(_55959am), WorkEvent::_6am),
            &mut connection,
        );
        current_date = current_date.succ();
    }

    Ok(())
}
