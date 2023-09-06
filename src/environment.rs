use std::env;

pub fn env_var_else(key: &str, default: &str) -> String {
    if let Ok(value) = env::var(key) {
        return value;
    }

    default.to_owned()
}
