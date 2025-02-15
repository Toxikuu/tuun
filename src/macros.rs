// macros.rs
//
// defines macros

#[macro_export]
macro_rules! vpr {
    ($($arg:tt)*) => {{
        if $crate::flags::is_verbose() {
            let f = std::path::Path::new(file!())
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown");
            println!("\n\x1b[34;1m[{}] {}\x1b[0m", f, format!($($arg)*))
        }
    }};
}

#[macro_export]
macro_rules! erm {
    ($($arg:tt)*) => {
        eprintln!("\x1b[31;1m  {}\x1b[0m", format!($($arg)*))
    };
}
