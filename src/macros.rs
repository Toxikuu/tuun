// src/macros.rs

#[macro_export]
macro_rules! p {
    ($($arg:tt)*) => {{
        #[cfg(feature = "debug")]
        println!("\x1b[36;1m[DEBUG] {}\x1b[0m", format!($($arg)*))
    }};
}
