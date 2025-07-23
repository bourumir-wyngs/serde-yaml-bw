/// Parse a YAML string slice into a [`Value`](crate::Value) at runtime.
///
/// # Panics
///
/// Panics if the input string is not valid YAML.
#[macro_export]
macro_rules! yaml {
    ($yaml:expr $(,)?) => {{
        match $crate::from_str::<$crate::Value>($yaml) {
            Ok(v) => v,
            Err(e) => panic!("yaml! macro failed to parse YAML: {e}"),
        }
    }};
}

/// Format a YAML string at runtime and parse it into a [`Value`](crate::Value).
///
/// This macro accepts a format string plus optional arguments in the same way
/// as [`format!`]. The resulting string is parsed as YAML.
///
/// # Panics
///
/// Panics if the expanded string is not valid YAML.
#[macro_export]
macro_rules! yaml_template {
    ($fmt:expr $(, $($arg:tt)+ )?) => {{
        let s = format!($fmt $(, $($arg)+ )?);
        match $crate::from_str::<$crate::Value>(&s) {
            Ok(v) => v,
            Err(e) => panic!("yaml_template! macro failed to parse YAML: {e}"),
        }
    }};
}

