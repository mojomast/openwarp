pub mod root;

pub mod chat_thread;
pub mod draft_buffer;
pub mod input_bar;
pub mod pty_panel;
pub mod question_prompt;
pub mod session_list;
pub mod status_bar;
pub mod tool_approval;

use crate::api::schema::{PermissionId, SessionId};

pub use root::RootView;

#[derive(Debug, Clone)]
pub enum UiAction {
    NewSession,
    SelectSession(SessionId),
    AllowPermission(PermissionId),
    AlwaysAllowPermission(PermissionId),
    DenyPermission(PermissionId),
    TogglePty,
}
