#![allow(clippy::arithmetic_side_effects)]
#![allow(dead_code)]

pub mod errors;
pub mod locator;
pub mod remote_keypair;
pub mod remote_wallet;
pub mod wallet;

#[macro_export]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        println!("{}:{:?} {}", file!(), line!(), format!($($arg)*));
    };
}
