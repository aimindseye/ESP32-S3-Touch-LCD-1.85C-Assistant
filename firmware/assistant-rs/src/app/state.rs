//! App state boundary.
//!
//! `AppState` is the single UI model snapshot consumed by the renderer.
//! Future provider tasks should update this state through app actions/intents,
//! not by reaching directly into rendering code.

use crate::app::{model::OnboardModel, pages::AssistantPage};

pub type AppState = OnboardModel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderSnapshot {
    pub page: AssistantPage,
    pub quiet_render_enabled: bool,
}

impl RenderSnapshot {
    pub fn from_state(state: &AppState) -> Self {
        Self {
            page: state.current_page,
            quiet_render_enabled: state.settings.quiet_render_enabled,
        }
    }
}

pub const APP_STATE_BOUNDARY_MARKER: &str =
    "v0.1.15 app state boundary: renderer consumes AppState snapshot";
