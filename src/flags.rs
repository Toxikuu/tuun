// flags.rs
//
// stores flags for global use

use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    pub static ref VERBOSE: Mutex<bool> = Mutex::new(false);
}

pub fn set_flags(verbose: bool) {
    *VERBOSE.lock().unwrap() = verbose;
}
