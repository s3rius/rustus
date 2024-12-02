use crate::from_str;
use derive_more::{Display, From};
use strum::EnumIter;

/// Hooks for notifications.
#[derive(Copy, Clone, Debug, Display, From, EnumIter, Eq, PartialEq)]
pub enum Hook {
    #[display("pre-create")]
    PreCreate,
    #[display("post-create")]
    PostCreate,
    #[display("post-receive")]
    PostReceive,
    #[display("pre-terminate")]
    PreTerminate,
    #[display("post-terminate")]
    PostTerminate,
    #[display("post-finish")]
    PostFinish,
}

from_str!(Hook, "hook");
