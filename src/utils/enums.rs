/// Implement `FromStr` for enums with `EnumIterator` trait from strum.
#[macro_export]
macro_rules! from_str {
    ($enum_name:ty, $name:literal) => {
        impl std::str::FromStr for $enum_name {
            type Err = String;

            fn from_str(input: &str) -> Result<Self, Self::Err> {
                // We iterate over all enum values.
                for store in <Self as strum::IntoEnumIterator>::iter() {
                    if input == store.to_string() {
                        return Ok(store);
                    }
                }
                let available_stores = <Self as strum::IntoEnumIterator>::iter()
                    .map(|info_store| format!("\t* {}", info_store.to_string()))
                    .collect::<Vec<String>>()
                    .join("\n");
                Err(format!(
                    "Unknown {} '{}'.\n Available {}s:\n{}",
                    $name, input, $name, available_stores
                ))
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::from_str;
    use strum::{Display, EnumIter};

    #[derive(PartialEq, Debug, Display, EnumIter, Clone, Eq)]
    pub enum TestEnum {
        #[strum(serialize = "test-val-1")]
        TestVal1,
        #[strum(serialize = "test-val-2")]
        TestVal2,
    }

    from_str!(TestEnum, "test-vals");

    #[test]
    fn test_from_str_unknown_val() {
        let result = TestEnum::from_str("unknown");
        assert!(result.is_err())
    }

    #[test]
    fn test_from_str() {
        let result = TestEnum::from_str("test-val-1");
        assert_eq!(result.unwrap(), TestEnum::TestVal1)
    }
}
