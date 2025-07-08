use std::sync::OnceLock;

static VERBOSE: OnceLock<bool> = OnceLock::new();

pub fn init_logging(verbose: bool) {
    VERBOSE
        .set(verbose)
        .expect("init_logging called multiple times");
}

pub fn is_verbose() -> bool {
    *VERBOSE.get().unwrap_or(&false)
}

#[macro_export]
macro_rules! log_verbose {
    ($($arg:tt)*) => {
        if $crate::logging::is_verbose() {
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        eprintln!($($arg)*);
    };
}
