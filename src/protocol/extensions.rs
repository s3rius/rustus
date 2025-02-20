use derive_more::{Display, From};
use strum::EnumIter;

use crate::from_str;
use std::str::FromStr;
use strum::IntoEnumIterator;

/// Enum of available Protocol Extensions
#[derive(PartialEq, Debug, PartialOrd, Display, EnumIter, From, Clone, Ord, Eq)]
pub enum Extensions {
    #[display("creation-defer-length")]
    CreationDeferLength,
    #[display("creation-with-upload")]
    CreationWithUpload,
    #[display("creation")]
    Creation,
    #[display("termination")]
    Termination,
    #[display("concatenation")]
    Concatenation,
    #[display("getting")]
    Getting,
    #[display("checksum")]
    Checksum,
}

from_str!(Extensions, "extension");
