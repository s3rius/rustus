use chrono::{Datelike, Timelike};

pub fn substr_time(dir_structure: &str, time: chrono::DateTime<chrono::Utc>) -> String {
    dir_structure
        .replace("{day}", time.day().to_string().as_str())
        .replace("{month}", time.month().to_string().as_str())
        .replace("{year}", time.year().to_string().as_str())
        .replace("{hour}", time.hour().to_string().as_str())
        .replace("{minute}", time.minute().to_string().as_str())
}

#[cfg(test)]
mod tests {
    use chrono::Datelike;

    use crate::utils::dir_struct::substr_time;

    #[test]
    pub fn test_time() {
        let now = chrono::Utc::now();
        let dir = substr_time("{day}/{month}", now);
        assert_eq!(dir, format!("{}/{}", now.day(), now.month()));
    }

    #[test]
    pub fn test_unknown_var() {
        let now = chrono::Utc::now();
        let dir = substr_time("test/{quake}", now);
        assert_eq!(dir, String::from("test/{quake}"));
    }
}
