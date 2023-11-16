#[derive(strum::Display, Debug, strum::EnumIter, Clone, PartialEq, Eq, Hash)]
pub enum TusExtensions {
    #[strum(serialize = "creation-defer-length")]
    CreationDeferLength,
    #[strum(serialize = "creation-with-upload")]
    CreationWithUpload,
    #[strum(serialize = "creation")]
    Creation,
    #[strum(serialize = "termination")]
    Termination,
    #[strum(serialize = "concatenation")]
    Concatenation,
    #[strum(serialize = "getting")]
    Getting,
    #[strum(serialize = "checksum")]
    Checksum,
}

crate::from_str!(TusExtensions, "extensions");
