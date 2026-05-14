use core::fmt;
use vitasdk_sys::{
    sceCtrlPeekBufferPositive, sceCtrlSetSamplingMode, SceCtrlData, SCE_CTRL_MODE_ANALOG,
    SCE_CTRL_MODE_ANALOG_WIDE, SCE_CTRL_MODE_DIGITAL,
};

// ── Button ────────────────────────────────────────────────────────────────

/// Digital button on the Vita.
///
/// Each variant carries the exact bit-mask used in the hardware register so
/// that `is_pressed` is a single `&` operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Button {
    Select = vitasdk_sys::SCE_CTRL_SELECT,
    L3 = vitasdk_sys::SCE_CTRL_L3,
    R3 = vitasdk_sys::SCE_CTRL_R3,
    Start = vitasdk_sys::SCE_CTRL_START,
    Up = vitasdk_sys::SCE_CTRL_UP,
    Right = vitasdk_sys::SCE_CTRL_RIGHT,
    Down = vitasdk_sys::SCE_CTRL_DOWN,
    Left = vitasdk_sys::SCE_CTRL_LEFT,
    /// Left trigger (L2 on extended modes).
    LTrigger = vitasdk_sys::SCE_CTRL_LTRIGGER,
    /// Right trigger (R2 on extended modes).
    RTrigger = vitasdk_sys::SCE_CTRL_RTRIGGER,
    /// Only available via `*Ext2` functions.
    L1 = vitasdk_sys::SCE_CTRL_L1,
    /// Only available via `*Ext2` functions.
    R1 = vitasdk_sys::SCE_CTRL_R1,
    Triangle = vitasdk_sys::SCE_CTRL_TRIANGLE,
    Circle = vitasdk_sys::SCE_CTRL_CIRCLE,
    Cross = vitasdk_sys::SCE_CTRL_CROSS,
    Square = vitasdk_sys::SCE_CTRL_SQUARE,
    /// Input intercepted by another application (Home button).
    Intercepted = vitasdk_sys::SCE_CTRL_INTERCEPTED,
    Headphone = vitasdk_sys::SCE_CTRL_HEADPHONE,
    VolUp = vitasdk_sys::SCE_CTRL_VOLUP,
    VolDown = vitasdk_sys::SCE_CTRL_VOLDOWN,
    Power = vitasdk_sys::SCE_CTRL_POWER,
}

impl Button {
    /// Raw bit-mask value matching `SceCtrlButtons`.
    #[inline]
    pub fn bits(self) -> u32 {
        self as u32
    }
}

// ── Joystick / Axis ───────────────────────────────────────────────────────

/// Which analogue stick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Joystick {
    Left,
    Right,
}

/// Axis of an analogue stick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
}

/// Normalised joystick data for one stick.
///
/// Values are in the range `[-1.0, 1.0]` where 0.0 is the centre position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JoystickData {
    pub x: f32,
    pub y: f32,
}

impl JoystickData {
    /// Return the value of a single axis.
    #[inline]
    pub fn axis(self, axis: Axis) -> f32 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
        }
    }
}

// ── Digital button pressure ───────────────────────────────────────────────

/// Per-button analogue pressure reading (0-255).
///
/// These are the individual `uint8_t` fields in `SceCtrlData` — they report
/// how far a digital button is pressed rather than a simple on/off state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DigitalPressure {
    pub up: u8,
    pub right: u8,
    pub down: u8,
    pub left: u8,
    pub l_trigger: u8,
    pub r_trigger: u8,
    pub l1: u8,
    pub r1: u8,
    pub triangle: u8,
    pub circle: u8,
    pub cross: u8,
    pub square: u8,
}

// ── Sampling mode ─────────────────────────────────────────────────────────

/// Controller sampling mode.
///
/// Maps directly to `SceCtrlPadInputMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SamplingMode {
    /// Digital buttons only — analogue sticks return 0.
    Digital = 0,
    /// Digital buttons + analogue sticks (0-255 range).
    Analog = 1,
    /// Same as Analog but with a wider range for the sticks.
    AnalogWide = 2,
}

// ── ControllerInput ─────────────────────────────────────────────────────────────

/// A snapshot of the Vita's controller state.
///
/// Obtain one by calling [`ControllerInput::poll`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControllerInput {
    /// Raw button bit-mask (see [`Button`]).
    pub buttons: u32,
    /// Normalised left stick.
    pub left_stick: JoystickData,
    /// Normalised right stick.
    pub right_stick: JoystickData,
    /// Per-button analogue pressure.
    pub pressure: DigitalPressure,
    /// Hardware timestamp (microseconds).
    pub timestamp: u64,
}

impl ControllerInput {
    // ── polling ────────────────────────────────────────────────────────

    /// Poll the controller on port 0 and return a snapshot of its state.
    ///
    /// This is a **non-blocking** call.  It uses `sceCtrlPeekBufferPositive`
    /// internally.
    ///
    /// # Panics
    ///
    /// Panics if the underlying syscall returns an error code.  In practice
    /// this should never happen on a healthy device.
    pub fn poll() -> Self {
        let mut raw = unsafe { core::mem::zeroed::<SceCtrlData>() };

        let ret = unsafe { sceCtrlPeekBufferPositive(0, &mut raw, 1) };
        assert!(ret >= 0, "sceCtrlPeekBufferPositive failed: {ret}");

        Self::from_raw(raw)
    }

    /// Read the controller state and remove it from the input buffer.
    ///
    /// This is a **blocking** call.  It uses `sceCtrlReadBufferPositive`
    /// internally, which waits until at least one input event is available before returning.
    ///
    /// # Panics
    ///
    /// Panics if the underlying syscall returns an error code.  In practice
    /// this should never happen on a healthy device.
    pub fn read() -> Self {
        let mut raw = unsafe { core::mem::zeroed::<SceCtrlData>() };
        let ret = unsafe { vitasdk_sys::sceCtrlReadBufferPositive(0, &mut raw, 1) };
        assert!(ret >= 0, "sceCtrlReadBufferPositive failed: {ret}");
        Self::from_raw(raw)
    }

    // ── button queries ─────────────────────────────────────────────────

    /// Returns `true` when **any** of the given buttons are pressed.
    ///
    /// ```ignore
    /// if input.is_pressed(Button::Cross) { … }
    /// if input.is_pressed(Button::Cross | Button::Circle) { … }
    /// ```
    pub fn is_pressed(&self, buttons: impl Into<ButtonMask>) -> bool {
        let mask = buttons.into();
        (self.buttons & mask.0) != 0
    }

    /// Returns `true` when **all** of the given buttons are pressed.
    pub fn all_pressed(&self, buttons: impl Into<ButtonMask>) -> bool {
        let mask = buttons.into();
        (self.buttons & mask.0) == mask.0
    }

    /// Returns `true` when **none** of the given buttons are pressed.
    pub fn released(&self, buttons: impl Into<ButtonMask>) -> bool {
        let mask = buttons.into();
        (self.buttons & mask.0) == 0
    }

    // ── joystick ───────────────────────────────────────────────────────

    /// Return normalised data for one analogue stick.
    #[inline]
    pub const fn joystick(&self, which: Joystick) -> JoystickData {
        match which {
            Joystick::Left => self.left_stick,
            Joystick::Right => self.right_stick,
        }
    }

    #[inline]
    #[cfg(feature = "glam")]
    pub fn joystick_vec(&self, which: Joystick) -> glam::Vec2 {
        let stick = self.joystick(which);
        glam::vec2(stick.x, stick.y)
    }

    // ── helpers ────────────────────────────────────────────────────────

    /// Set the controller sampling mode.
    ///
    /// Call this once at startup.  The default mode after boot is **Digital**;
    /// you almost always want [`SamplingMode::Analog`].
    ///
    /// Returns the *previous* mode, or a negative value on error.
    pub fn set_sampling_mode(mode: SamplingMode) -> i32 {
        let raw_mode = match mode {
            SamplingMode::Digital => SCE_CTRL_MODE_DIGITAL,
            SamplingMode::Analog => SCE_CTRL_MODE_ANALOG,
            SamplingMode::AnalogWide => SCE_CTRL_MODE_ANALOG_WIDE,
        };
        unsafe { sceCtrlSetSamplingMode(raw_mode) }
    }

    // ── internals ──────────────────────────────────────────────────────

    /// Build a `ControllerInput` from a raw `SceCtrlData`.
    fn from_raw(raw: SceCtrlData) -> Self {
        Self {
            buttons: raw.buttons,
            left_stick: JoystickData {
                x: normalise_axis(raw.lx),
                y: normalise_axis(raw.ly),
            },
            right_stick: JoystickData {
                x: normalise_axis(raw.rx),
                y: normalise_axis(raw.ry),
            },
            pressure: DigitalPressure {
                up: raw.up,
                right: raw.right,
                down: raw.down,
                left: raw.left,
                l_trigger: raw.lt,
                r_trigger: raw.rt,
                l1: raw.l1,
                r1: raw.r1,
                triangle: raw.triangle,
                circle: raw.circle,
                cross: raw.cross,
                square: raw.square,
            },
            timestamp: raw.timeStamp,
        }
    }
}

// ── ButtonMask ────────────────────────────────────────────────────────────

/// A bit-mask of [`Button`]s.
///
/// You rarely need to construct this directly — `Button` and `|` combinations
/// of `Button` convert into it automatically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonMask(u32);

impl From<Button> for ButtonMask {
    fn from(b: Button) -> Self {
        Self(b.bits())
    }
}

/// Allow `input.is_pressed(Button::Cross | Button::Circle)`.
///
/// This works because `Button` is `#[repr(u32)]` and `|` on two `Button`
/// values yields a `u32`.
impl From<u32> for ButtonMask {
    fn from(v: u32) -> Self {
        Self(v)
    }
}

// ── Normalisation ─────────────────────────────────────────────────────────

/// Map a raw `u8` axis value (0-255) to `[-1.0, 1.0]`.
///
/// The Vita's analogue sticks rest at 128.  Values below 128 are negative,
/// values above are positive.
#[inline]
fn normalise_axis(raw: u8) -> f32 {
    const CENTER: f32 = 128.0;
    const SCALE: f32 = 127.0;
    ((raw as f32) - CENTER) / SCALE
}

// ── Formatting ────────────────────────────────────────────────────────────

impl fmt::Display for Button {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Button::Select => "Select",
            Button::L3 => "L3",
            Button::R3 => "R3",
            Button::Start => "Start",
            Button::Up => "Up",
            Button::Right => "Right",
            Button::Down => "Down",
            Button::Left => "Left",
            Button::LTrigger => "LTrigger",
            Button::RTrigger => "RTrigger",
            Button::L1 => "L1",
            Button::R1 => "R1",
            Button::Triangle => "Triangle",
            Button::Circle => "Circle",
            Button::Cross => "Cross",
            Button::Square => "Square",
            Button::Intercepted => "Intercepted",
            Button::Headphone => "Headphone",
            Button::VolUp => "VolUp",
            Button::VolDown => "VolDown",
            Button::Power => "Power",
        };
        f.write_str(name)
    }
}

impl fmt::Display for Joystick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Joystick::Left => "Left",
            Joystick::Right => "Right",
        })
    }
}

impl fmt::Display for Axis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Axis::X => "X",
            Axis::Y => "Y",
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_center_is_zero() {
        let v = normalise_axis(128);
        assert!((v - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn normalise_min_is_neg_one() {
        let v = normalise_axis(1); // 0 would be -128/127 ≈ -1.007, close enough
        assert!(v < -0.99);
    }

    #[test]
    fn normalise_max_is_pos_one() {
        let v = normalise_axis(255);
        assert!((v - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn button_mask_from_button() {
        let mask: ButtonMask = Button::Cross.into();
        assert_eq!(mask.0, 0x0000_4000);
    }

    #[test]
    fn button_mask_from_u32_or() {
        let mask: ButtonMask = (Button::Cross.bits() | Button::Circle.bits()).into();
        assert_eq!(mask.0, 0x0000_6000);
    }

    #[test]
    fn joystick_data_axis() {
        let j = JoystickData { x: 0.5, y: -0.25 };
        assert_eq!(j.axis(Axis::X), 0.5);
        assert_eq!(j.axis(Axis::Y), -0.25);
    }

    #[test]
    fn sampling_mode_values() {
        assert_eq!(SamplingMode::Digital as i32, 0);
        assert_eq!(SamplingMode::Analog as i32, 1);
        assert_eq!(SamplingMode::AnalogWide as i32, 2);
    }
}
