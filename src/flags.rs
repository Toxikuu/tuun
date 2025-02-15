// flags.rs
//
// stores flags for global use

use std::sync::{
    LazyLock,
    atomic::{AtomicBool, Ordering}
};

pub static VERBOSE: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(false));

pub fn set_flags(verbose: bool) {
    VERBOSE.store(verbose, Ordering::Relaxed);
}

pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}
