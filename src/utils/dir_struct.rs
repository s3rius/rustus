use chrono::{Datelike, Timelike};
use lazy_static::lazy_static;
use log::error;
use std::{collections::HashMap, env};

lazy_static! {
    /// Freezing ENVS on startup.
    static ref ENV_MAP: HashMap<String, String> = {
        let mut m = HashMap::new();
        for (key, value) in env::vars() {
            m.insert(format!("env[{}]", key), value);
        }
        m
    };
}

/// Generate directory name with user template.
pub fn dir_struct(dir_structure: &str) -> String {
    let now = chrono::Utc::now();
    let mut vars: HashMap<String, String> = ENV_MAP.clone();
    vars.insert("day".into(), now.day().to_string());
    vars.insert("month".into(), now.month().to_string());
    vars.insert("year".into(), now.year().to_string());
    vars.insert("hour".into(), now.hour().to_string());
    vars.insert("minute".into(), now.minute().to_string());
    strfmt::strfmt(dir_structure, &vars).unwrap_or_else(|err| {
        error!("{}", err);
        "".into()
    })
}

#[cfg(test)]
mod tests {
    use super::dir_struct;
    use chrono::Datelike;

    #[test]
    pub fn test_time() {
        let now = chrono::Utc::now();
        let dir = dir_struct("{day}/{month}");
        assert_eq!(dir, format!("{}/{}", now.day(), now.month()));
    }

    #[test]
    pub fn test_unknown_var() {
        let dir = dir_struct("test/{quake}");
        assert_eq!(dir, String::from(""));
    }
}
