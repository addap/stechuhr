use dotenv::dotenv;
use pbkdf2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Pbkdf2,
};
use std::error::Error;
use stechuhr::models::PasswordHash;

fn get_input_pw() -> Result<String, Box<dyn Error>> {
    if let Some(password) = std::env::args().nth(1) {
        Ok(password.trim().to_string())
    } else {
        println!("Usage: add_admin_pw <pw>");
        Err("Password missing".into())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    env_logger::init();

    let password = get_input_pw()?;
    let salt = SaltString::generate(&mut OsRng);

    // Hash password to PHC string ($pbkdf2-sha256$...)
    let password_hash = Pbkdf2.hash_password(password.as_ref(), &salt)?.to_string();
    println!("{}", password_hash);

    let connection = stechuhr::establish_connection();
    stechuhr::save_password(PasswordHash::new(password_hash), &connection);

    // Verify password against PHC string
    // let parsed_hash = PasswordHash::new(&password_hash)?;
    // println!("{:?}", parsed_hash);

    // assert!(Pbkdf2.verify_password(password, &parsed_hash).is_ok());
    //     stechuhr::save_password(password.trim())
    // }
    Ok(())
}
