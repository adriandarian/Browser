#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        eprintln!("[INFO] {}", format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        eprintln!("[WARN] {}", format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {{
        eprintln!("[TRACE] {}", format_args!($($arg)*));
    }};
}
