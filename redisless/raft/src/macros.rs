macro_rules! assert_true {
    ($($arg:tt)*) => ({
        assert!($($arg)+);
        true
    });
}

macro_rules! assert_match {
    ($pat:pat = $expr:expr) => ({
        if let $pat = $expr {
            true
        } else {
            panic!("assertion failed: `$pat = $expr`")
        }
    });
    ($pat:pat = $expr:expr, $($arg:tt)*) => ({
        if let $pat = $expr {
            true
        } else {
            panic!("assertion failed: `$pat = $expr`: {}", format_args!($($arg)+))
        }
    });
}

macro_rules! verbose {
    ($($arg:tt)*) => (
        log::debug!($($arg)*)
    );
}
