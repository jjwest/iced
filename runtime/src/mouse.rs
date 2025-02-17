//! Mouse utilities

use crate::futures::futures::channel::oneshot;
use crate::task::{self, Task};
use iced_core::Point;

/// An operation to be performed for the mouse
#[allow(missing_debug_implementations)]
pub enum Action {
    /// Gets the current mouse position in global screen coordinates
    GetPosition(oneshot::Sender<Option<Point>>),
}

/// Gets the current mouse position in global screen coordinates
pub fn get_position() -> Task<Option<Point>> {
    task::oneshot(move |channel| {
        crate::Action::Mouse(Action::GetPosition(channel))
    })
}
