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
