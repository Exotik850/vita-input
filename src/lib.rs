//! # vita-input
//!
//! An idiomatic Rust wrapper around the Vita's input polling functions
//! from [`vitasdk-sys`].
//!
//! ## Quick start
//!
//! ### Controller
//!
//! ```ignore
//! use vita_input::controller::{VitaInput, Button, Joystick, Axis};
//!
//! let input = VitaInput::poll();
//!
//! if input.is_pressed(Button::Cross) {
//!     // jump!
//! }
//!
//! let lx = input.joystick(Joystick::Left).axis(Axis::X);
//! if lx > 0.5 {
//!     // move right
//! }
//! ```
//!
//! ### Touch
//!
//! ```ignore
//! use vita_input::touch::{TouchInput, TouchPort};
//!
//! // Start sampling on the front panel
//! let input = TouchInput::start_sampling(TouchPort::Front);
//!
//! // Poll for touch events
//! let touch = input.poll();
//! for point in &touch {
//!     // handle point.x, point.y (0-1919, 0-1087)
//! }
//! ```

#[cfg(feature = "controller")]
pub mod controller;

#[cfg(feature = "touch")]
pub mod touch;

pub mod prelude {
    #[cfg(feature = "controller")]
    pub use crate::controller::{Button, Joystick, VitaInput};

    #[cfg(feature = "touch")]
    pub use crate::touch::{TouchInput, TouchPoint};
}
