/// Implement `FromStr` for enums with `EnumIterator` trait from strum.
#[macro_export]
macro_rules! from_str {
    ($enum_name:ty, $name:literal) => {
        use std::str::FromStr;
        use strum::IntoEnumIterator;

        impl FromStr for $enum_name {
            type Err = String;

            fn from_str(input: &str) -> Result<Self, Self::Err> {
                let available_stores = <$enum_name>::iter()
                    .map(|info_store| format!("\t* {}", info_store.to_string()))
                    .collect::<Vec<String>>()
                    .join("\n");
                let inp_string = String::from(input);
                for store in <$enum_name>::iter() {
                    if inp_string == store.to_string() {
                        return Ok(store);
                    }
                }
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
    use crate::from_str;
    use derive_more::{Display, From};
    use strum::EnumIter;

    #[derive(PartialEq, Debug, Display, EnumIter, From, Clone, Eq)]
    pub enum TestEnum {
        #[display("test-val-1")]
        TestVal1,
        #[display("test-val-2")]
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
