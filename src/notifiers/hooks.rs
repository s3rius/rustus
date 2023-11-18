use crate::from_str;

/// Hooks for notifications.
#[derive(Copy, Clone, Debug, strum::Display, strum::EnumIter, Eq, PartialEq, Hash)]
pub enum Hook {
    #[strum(serialize = "pre-create")]
    PreCreate,
    #[strum(serialize = "post-create")]
    PostCreate,
    #[strum(serialize = "post-receive")]
    PostReceive,
    #[strum(serialize = "pre-terminate")]
    PreTerminate,
    #[strum(serialize = "post-terminate")]
    PostTerminate,
    #[strum(serialize = "post-finish")]
    PostFinish,
}

from_str!(Hook, "hook");
