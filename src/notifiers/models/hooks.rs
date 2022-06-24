use crate::from_str;
use derive_more::{Display, From};
use strum::EnumIter;

/// Hooks for notifications.
#[derive(Copy, Clone, Debug, Display, From, EnumIter, Eq, PartialEq)]
pub enum Hook {
    #[display(fmt = "pre-create")]
    PreCreate,
    #[display(fmt = "post-create")]
    PostCreate,
    #[display(fmt = "post-receive")]
    PostReceive,
    #[display(fmt = "pre-terminate")]
    PreTerminate,
    #[display(fmt = "post-terminate")]
    PostTerminate,
    #[display(fmt = "post-finish")]
    PostFinish,
}

from_str!(Hook, "hook");
