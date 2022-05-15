use chrono::{Datelike, Timelike};

/// Generate directory name with user template.
pub fn dir_struct(dir_structure: &str) -> String {
    let now = chrono::Utc::now();
    dir_structure
        .replace("{day}", now.day().to_string().as_str())
        .replace("{month}", now.month().to_string().as_str())
        .replace("{year}", now.year().to_string().as_str())
        .replace("{hour}", now.hour().to_string().as_str())
        .replace("{minute}", now.minute().to_string().as_str())
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
        assert_eq!(dir, String::from("test/{quake}"));
    }
}
