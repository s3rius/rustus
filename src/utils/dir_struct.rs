use chrono::{Datelike, Timelike};

/// Generate directory name with user template.
pub fn substr_now(dir_structure: &str) -> String {
    let now = chrono::Utc::now();
    substr_time(dir_structure, now)
}

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
    use super::substr_now;
    use chrono::Datelike;

    #[test]
    pub fn test_time() {
        let now = chrono::Utc::now();
        let dir = substr_now("{day}/{month}");
        assert_eq!(dir, format!("{}/{}", now.day(), now.month()));
    }

    #[test]
    pub fn test_unknown_var() {
        let dir = substr_now("test/{quake}");
        assert_eq!(dir, String::from("test/{quake}"));
    }
}
