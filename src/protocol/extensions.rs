use derive_more::{Display, From};
use strum::EnumIter;

use crate::from_str;

/// Enum of available Protocol Extensions
#[derive(PartialEq, Debug, PartialOrd, Display, EnumIter, From, Clone, Ord, Eq)]
pub enum Extensions {
    #[display(fmt = "creation-defer-length")]
    CreationDeferLength,
    #[display(fmt = "creation-with-upload")]
    CreationWithUpload,
    #[display(fmt = "creation")]
    Creation,
    #[display(fmt = "termination")]
    Termination,
    #[display(fmt = "concatenation")]
    Concatenation,
    #[display(fmt = "getting")]
    Getting,
}

from_str!(Extensions, "extension");
