//! Touch panel input for the PlayStation Vita.
//!
//! Wraps the `SceTouch` library from the Vita SDK, providing an ergonomic
//! interface for reading from both the front touchscreen and rear touch pad.
//!
//! ## Quick start
//!
//! ```ignore
//! use vita_input::touch::{TouchInput, TouchPort};
//!
//! // Enable sampling on the front panel (once at startup)
//! let input = TouchInput::start_sampling(TouchPort::Front);
//!
//! // Later, in your game loop — poll is non-blocking
//! let touch = input.poll();
//!
//! if touch.is_touching() {
//!     for point in &touch {
//!         // point.x, point.y are raw panel coordinates
//!         // point.force is 0-255 (enable with TouchInput::enable_force)
//!     }
//! }
//!
//! // Blocking read — waits for new sample
//! let touch = input.read();
//! if let Some(point) = touch.first() {
//!     // handle single touch
//! }
//! ```
//!
//! ## Coordinate normalisation
//!
//! Raw touch coordinates are panel-specific.  Use [`TouchPanelInfo`] to
//! convert them to `[0.0, 1.0]`:
//!
//! ```ignore
//! let info = TouchInput::panel_info(TouchPort::Front);
//! let touch = TouchInput::start_sampling(TouchPort::Front).poll();
//!
//! for point in &touch {
//!     let (nx, ny) = point.normalized_display(&info);
//! }
//! ```

use core::fmt;
use vitasdk_sys::{
    sceTouchDisableTouchForce, sceTouchEnableTouchForce, sceTouchGetPanelInfo, sceTouchPeek,
    sceTouchRead, sceTouchSetSamplingState, SceTouchData, SceTouchPanelInfo, SceTouchReport,
    SCE_TOUCH_PORT_BACK, SCE_TOUCH_PORT_FRONT, SCE_TOUCH_PORT_MAX_NUM,
    SCE_TOUCH_SAMPLING_STATE_START, SCE_TOUCH_SAMPLING_STATE_STOP,
};

// ── TouchPort ─────────────────────────────────────────────────────────────

/// Which touch panel to read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum TouchPort {
    /// Front touchscreen.
    Front = SCE_TOUCH_PORT_FRONT,
    /// Rear touch pad.
    Back = SCE_TOUCH_PORT_BACK,
}

impl TouchPort {
    /// Total number of touch ports available.
    pub const MAX: u32 = SCE_TOUCH_PORT_MAX_NUM;

    #[inline]
    fn to_raw(self) -> u32 {
        self as u32
    }
}

impl TryFrom<u32> for TouchPort {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            SCE_TOUCH_PORT_FRONT => Ok(Self::Front),
            SCE_TOUCH_PORT_BACK => Ok(Self::Back),
            _ => Err(()),
        }
    }
}

impl fmt::Display for TouchPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TouchPort::Front => "Front",
            TouchPort::Back => "Back",
        })
    }
}

// ── TouchPoint ────────────────────────────────────────────────────────────

/// A single touch report from the panel.
///
/// Each point represents one finger or capacitive contact on the screen.
/// The touch `id` stays consistent while the same finger is held down,
/// allowing you to track individual fingers across frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TouchPoint {
    /// Touch identifier — remains consistent while the finger is held down.
    pub id: u8,
    /// Raw X coordinate in panel-native coordinates.
    pub x: i16,
    /// Raw Y coordinate in panel-native coordinates.
    pub y: i16,
    /// Touch force / pressure (0-255).
    ///
    /// Always 0 unless [`TouchInput::enable_force`] has been called.
    pub force: u8,
}

impl TouchPoint {
    /// Create a `TouchPoint` from a raw `SceTouchReport`.
    #[inline]
    fn from_raw(report: &SceTouchReport) -> Self {
        Self {
            id: report.id,
            x: report.x,
            y: report.y,
            force: report.force,
        }
    }

    /// Normalise this point's coordinates to `[0.0, 1.0]` using the panel's
    /// anti-aliased (high-resolution) coordinate range.
    ///
    /// Pass the [`TouchPanelInfo`] obtained from
    /// [`TouchInput::panel_info`].
    pub fn normalized_aa(&self, info: &TouchPanelInfo) -> (f32, f32) {
        let dx = (info.max_aa_x - info.min_aa_x) as f32;
        let dy = (info.max_aa_y - info.min_aa_y) as f32;
        let x = (self.x - info.min_aa_x) as f32 / dx;
        let y = (self.y - info.min_aa_y) as f32 / dy;
        (clamp01(x), clamp01(y))
    }

    #[cfg(feature = "glam")]
    /// Normalise this point's coordinates to `[0.0, 1.0]` using the panel's anti-aliased (high-resolution) coordinate range, returning a `glam::Vec2`.
    pub fn norm_aa_vec2(&self, info: &TouchPanelInfo) -> glam::Vec2 {
        let (x, y) = self.normalized_aa(info);
        glam::Vec2::new(x, y)
    }

    /// Normalise this point's coordinates to `[0.0, 1.0]` using the panel's
    /// display coordinate range.
    ///
    /// This is typically what you want for mapping touch input to your
    /// rendering viewport.
    pub fn normalized_display(&self, info: &TouchPanelInfo) -> (f32, f32) {
        let dx = (info.max_disp_x - info.min_disp_x) as f32;
        let dy = (info.max_disp_y - info.min_disp_y) as f32;
        let x = (self.x - info.min_disp_x) as f32 / dx;
        let y = (self.y - info.min_disp_y) as f32 / dy;
        (clamp01(x), clamp01(y))
    }

    #[cfg(feature = "glam")]
    /// Normalise this point's coordinates to `[0.0, 1.0]` using the panel's display coordinate range, returning a `glam::Vec2`.
    pub fn norm_display_vec2(&self, info: &TouchPanelInfo) -> glam::Vec2 {
        let (x, y) = self.normalized_display(info);
        glam::Vec2::new(x, y)
    }

    /// Normalise the force value to `[0.0, 1.0]` using the panel's reported
    /// force range.
    pub fn normalized_force(&self, info: &TouchPanelInfo) -> f32 {
        let d = (info.max_force - info.min_force) as f32;
        if d <= 0.0 {
            return 0.0;
        }
        clamp01((self.force - info.min_force) as f32 / d)
    }
}

// ── TouchPanelInfo ────────────────────────────────────────────────────────

/// Calibration information for a touch panel.
///
/// Obtained via [`TouchInput::panel_info`].  Use this to normalise raw
/// touch coordinates into `[0.0, 1.0]` space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TouchPanelInfo {
    /// Minimum X in anti-aliased coordinate space.
    pub min_aa_x: i16,
    /// Minimum Y in anti-aliased coordinate space.
    pub min_aa_y: i16,
    /// Maximum X in anti-aliased coordinate space.
    pub max_aa_x: i16,
    /// Maximum Y in anti-aliased coordinate space.
    pub max_aa_y: i16,
    /// Minimum X in display coordinate space.
    pub min_disp_x: i16,
    /// Minimum Y in display coordinate space.
    pub min_disp_y: i16,
    /// Maximum X in display coordinate space.
    pub max_disp_x: i16,
    /// Maximum Y in display coordinate space.
    pub max_disp_y: i16,
    /// Minimum detectable touch force (0-255).
    pub min_force: u8,
    /// Maximum detectable touch force (0-255).
    pub max_force: u8,
}

impl TouchPanelInfo {
    fn from_raw(raw: &SceTouchPanelInfo) -> Self {
        Self {
            min_aa_x: raw.minAaX,
            min_aa_y: raw.minAaY,
            max_aa_x: raw.maxAaX,
            max_aa_y: raw.maxAaY,
            min_disp_x: raw.minDispX,
            min_disp_y: raw.minDispY,
            max_disp_x: raw.maxDispX,
            max_disp_y: raw.maxDispY,
            min_force: raw.minForce,
            max_force: raw.maxForce,
        }
    }

    /// Width of the panel in anti-aliased coordinates.
    #[inline]
    pub fn aa_width(self) -> i16 {
        self.max_aa_x - self.min_aa_x
    }

    /// Height of the panel in anti-aliased coordinates.
    #[inline]
    pub fn aa_height(self) -> i16 {
        self.max_aa_y - self.min_aa_y
    }

    /// Width of the panel in display coordinates.
    #[inline]
    pub fn disp_width(self) -> i16 {
        self.max_disp_x - self.min_disp_x
    }

    /// Height of the panel in display coordinates.
    #[inline]
    pub fn disp_height(self) -> i16 {
        self.max_disp_y - self.min_disp_y
    }
}

// ── TouchInput ────────────────────────────────────────────────────────────

/// A snapshot of one touch panel's state at a point in time.
///
/// Obtain one by calling [`TouchInput::peek`] (non-blocking) or
/// [`TouchInput::read`] (blocking).  The hardware reports up to 8
/// simultaneous touch points.
///
/// # Iteration
///
/// You can iterate over active touch points with `for point in &input`:
///
/// ```ignore
/// for point in &touch {
///     println!("finger {} at ({}, {})", point.id, point.x, point.y);
/// }
/// ```
#[derive(Clone, Copy)]
pub struct TouchInput {
    /// Hardware timestamp (microseconds).
    pub timestamp: u64,
    /// Number of active touch points (0-8).
    pub point_count: u32,
    /// Raw touch reports.  Only indices `0..point_count` are valid.
    points: [TouchPoint; 8],
}

pub struct TouchSampleGaurd {
    port: TouchPort,
}

impl TouchSampleGaurd {
    /// Peek (non-blocking) the current touch state for this guard's port.
    ///
    /// Panics if the underlying syscall returns an error.
    /// In practice this should never happen on a healthy device.
    pub fn poll(&self) -> TouchInput {
        let port = self.port;
        let mut raw = unsafe { core::mem::zeroed::<SceTouchData>() };
        let ret = unsafe { sceTouchPeek(port.to_raw(), &mut raw, 1) };
        assert!(ret >= 0, "sceTouchPeek({port}) failed: {ret}");
        TouchInput::from_raw(&raw)
    }

    /// Read (blocking) the current touch state for this guard's port.
    ///
    /// Panics if the underlying syscall returns an error.
    /// In practice this should never happen on a healthy device.
    pub fn read(&self) -> TouchInput {
        let port = self.port;
        let mut raw = unsafe { core::mem::zeroed::<SceTouchData>() };
        let ret = unsafe { sceTouchRead(port.to_raw(), &mut raw, 1) };
        assert!(ret >= 0, "sceTouchRead({port}) failed: {ret}");
        TouchInput::from_raw(&raw)
    }
}

impl Drop for TouchSampleGaurd {
    fn drop(&mut self) {
        let port = self.port;
        let ret = unsafe { sceTouchSetSamplingState(port.to_raw(), SCE_TOUCH_SAMPLING_STATE_STOP) };
        assert!(
            ret >= 0,
            "sceTouchSetSamplingState(STOP, {port}) failed: {ret}"
        );
    }
}

impl TouchInput {
    /// The maximum number of simultaneous touch points the hardware can
    /// report.
    pub const MAX_POINTS: usize = 8;

    // ── lifecycle ──────────────────────────────────────────────────────

    /// Start touch sampling on the given port, returns a guard that will stop sampling when dropped.
    ///
    /// Must be called once before [`peek`](Self::peek) or
    /// [`read`](Self::read) will return any data.
    ///
    /// # Panics
    ///
    /// Panics if the underlying syscall returns an error (e.g. invalid
    /// port).  In practice this should never happen on a healthy device.
    #[must_use]
    pub fn start_sampling(port: TouchPort) -> TouchSampleGaurd {
        let ret =
            unsafe { sceTouchSetSamplingState(port.to_raw(), SCE_TOUCH_SAMPLING_STATE_START) };
        assert!(
            ret >= 0,
            "sceTouchSetSamplingState(START, {port}) failed: {ret}"
        );
        TouchSampleGaurd { port }
    }

    // ── force / pressure ───────────────────────────────────────────────

    /// Enable touch-force (pressure) reporting for a port.
    ///
    /// Without calling this, [`TouchPoint::force`] will always be `0`.
    ///
    /// # Panics
    ///
    /// Panics if `sceTouchEnableTouchForce` returns an error.
    pub fn enable_force(port: TouchPort) {
        let ret = unsafe { sceTouchEnableTouchForce(port.to_raw()) };
        assert!(ret >= 0, "sceTouchEnableTouchForce({port}) failed: {ret}");
    }

    /// Disable touch-force reporting for a port.
    ///
    /// # Panics
    ///
    /// Panics if `sceTouchDisableTouchForce` returns an error.
    pub fn disable_force(port: TouchPort) {
        let ret = unsafe { sceTouchDisableTouchForce(port.to_raw()) };
        assert!(ret >= 0, "sceTouchDisableTouchForce({port}) failed: {ret}");
    }

    // ── panel info ─────────────────────────────────────────────────────

    /// Retrieve calibration information for a touch panel.
    ///
    /// Use the returned [`TouchPanelInfo`] to convert raw coordinates to
    /// normalised `[0.0, 1.0]` space via
    /// [`TouchPoint::normalized_display`] or
    /// [`TouchPoint::normalized_aa`].
    ///
    /// # Panics
    ///
    /// Panics if `sceTouchGetPanelInfo` returns an error.
    pub fn panel_info(port: TouchPort) -> TouchPanelInfo {
        let mut raw = unsafe { core::mem::zeroed::<SceTouchPanelInfo>() };
        let ret = unsafe { sceTouchGetPanelInfo(port.to_raw(), &mut raw) };
        assert!(ret >= 0, "sceTouchGetPanelInfo({port}) failed: {ret}");
        TouchPanelInfo::from_raw(&raw)
    }

    // ── queries ────────────────────────────────────────────────────────

    /// Returns `true` when at least one finger is touching the panel.
    #[inline]
    pub fn is_touching(&self) -> bool {
        self.point_count > 0
    }

    /// Returns the first touch point, or `None` if nothing is touching
    /// the panel.
    #[inline]
    pub fn first(&self) -> Option<TouchPoint> {
        if self.point_count > 0 {
            Some(self.points[0])
        } else {
            None
        }
    }

    #[cfg(feature = "glam")]
    #[inline]
    /// Returns the average position of all active touch points, normalised to `[0.0, 1.0]` using the panel's display coordinate range, or `None` if nothing is touching the panel.
    pub fn average(&self) -> Option<glam::Vec2> {
        if self.point_count == 0 {
            None
        } else {
            let mut sum = glam::Vec2::ZERO;
            for point in &self.points[..self.point_count as usize] {
                sum += point.norm_display_vec2(&TouchInput::panel_info(self.port));
            }
            Some(sum / self.point_count as f32)
        }
    }

    /// Returns the touch point at the given index, or `None` if the index
    /// is out of range.
    #[inline]
    pub fn get(&self, index: usize) -> Option<TouchPoint> {
        if index < self.point_count as usize {
            Some(self.points[index])
        } else {
            None
        }
    }

    /// Iterate over all active touch points.
    #[inline]
    pub fn iter(&self) -> TouchIter<'_> {
        TouchIter {
            touch: self,
            front: 0,
            back: self.point_count as usize,
        }
    }

    /// Return all active touch points as a slice.
    #[inline]
    pub fn as_slice(&self) -> &[TouchPoint] {
        &self.points[..self.point_count as usize]
    }

    // ── internals ──────────────────────────────────────────────────────

    fn from_raw(raw: &SceTouchData) -> Self {
        let report_num = (raw.reportNum as usize).min(8);
        let mut points = [TouchPoint {
            id: 0,
            x: 0,
            y: 0,
            force: 0,
        }; 8];
        for i in 0..report_num {
            points[i] = TouchPoint::from_raw(&raw.report[i]);
        }
        Self {
            timestamp: raw.timeStamp,
            point_count: report_num as u32,
            points,
        }
    }
}

impl fmt::Debug for TouchInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TouchInput")
            .field("timestamp", &self.timestamp)
            .field("point_count", &self.point_count)
            .field("points", &self.as_slice())
            .finish()
    }
}

// ── IntoIterator ──────────────────────────────────────────────────────────

impl<'a> IntoIterator for &'a TouchInput {
    type Item = TouchPoint;
    type IntoIter = TouchIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ── TouchIter ─────────────────────────────────────────────────────────────

/// Iterator over the active touch points in a [`TouchInput`] snapshot.
///
/// Created by [`TouchInput::iter`] or by iterating over a reference:
///
/// ```ignore
/// for point in &touch_input { ... }
/// ```
#[derive(Clone)]
pub struct TouchIter<'a> {
    touch: &'a TouchInput,
    front: usize,
    back: usize,
}

impl<'a> Iterator for TouchIter<'a> {
    type Item = TouchPoint;

    fn next(&mut self) -> Option<Self::Item> {
        if self.front < self.back {
            let point = self.touch.points[self.front];
            self.front += 1;
            Some(point)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.back - self.front;
        (remaining, Some(remaining))
    }

    fn count(self) -> usize {
        self.back - self.front
    }

    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }
}

impl<'a> DoubleEndedIterator for TouchIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front < self.back {
            self.back -= 1;
            Some(self.touch.points[self.back])
        } else {
            None
        }
    }
}

impl ExactSizeIterator for TouchIter<'_> {}

impl<'a> fmt::Debug for TouchIter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

#[inline]
fn clamp01(v: f32) -> f32 {
    if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

// ── Formatting ────────────────────────────────────────────────────────────

impl fmt::Display for TouchPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Touch #{}: ({}, {}), f={}",
            self.id, self.x, self.y, self.force
        )
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Verify that TouchInput is reasonably sized for stack copies.
    #[test]
    fn touch_input_size_is_reasonable() {
        // 8 touch points × ~8 bytes each + timestamp + count = ~80 bytes
        assert!(core::mem::size_of::<TouchInput>() <= 128);
    }

    #[test]
    fn empty_touch_is_not_touching() {
        let raw = unsafe { core::mem::zeroed::<SceTouchData>() };
        let touch = TouchInput::from_raw(&raw);
        assert!(!touch.is_touching());
        assert_eq!(touch.point_count, 0);
        assert!(touch.first().is_none());
        assert_eq!(touch.iter().count(), 0);
    }

    #[test]
    fn clamp_values() {
        assert!((clamp01(-0.5) - 0.0).abs() < f32::EPSILON);
        assert!((clamp01(0.5) - 0.5).abs() < f32::EPSILON);
        assert!((clamp01(1.5) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn panel_info_dimensions() {
        let info = TouchPanelInfo {
            min_aa_x: 0,
            min_aa_y: 0,
            max_aa_x: 1920,
            max_aa_y: 1088,
            min_disp_x: 0,
            min_disp_y: 0,
            max_disp_x: 960,
            max_disp_y: 544,
            min_force: 0,
            max_force: 255,
        };
        assert_eq!(info.aa_width(), 1920);
        assert_eq!(info.aa_height(), 1088);
        assert_eq!(info.disp_width(), 960);
        assert_eq!(info.disp_height(), 544);
    }

    #[test]
    fn touch_point_normalization() {
        let info = TouchPanelInfo {
            min_aa_x: 0,
            min_aa_y: 0,
            max_aa_x: 1920,
            max_aa_y: 1088,
            min_disp_x: 0,
            min_disp_y: 0,
            max_disp_x: 960,
            max_disp_y: 544,
            min_force: 0,
            max_force: 255,
        };
        let point = TouchPoint {
            id: 0,
            x: 480,
            y: 272,
            force: 128,
        };

        let (nx, ny) = point.normalized_display(&info);
        assert!((nx - 0.5).abs() < 0.01);
        assert!((ny - 0.5).abs() < 0.01);

        let nf = point.normalized_force(&info);
        assert!((nf - 0.5).abs() < 0.02);
    }

    #[test]
    fn touch_iter() {
        let mut raw = unsafe { core::mem::zeroed::<SceTouchData>() };
        raw.reportNum = 3;
        raw.report[0].id = 1;
        raw.report[0].x = 100;
        raw.report[1].id = 2;
        raw.report[1].x = 200;
        raw.report[2].id = 3;
        raw.report[2].x = 300;

        let touch = TouchInput::from_raw(&raw);
        assert_eq!(touch.point_count, 3);

        let points: Vec<_> = touch.iter().collect();
        assert_eq!(points.len(), 3);
        assert_eq!(points[0].id, 1);

        // Double-ended
        let last = touch.iter().rev().next().unwrap();
        assert_eq!(last.id, 3);

        // IntoIterator on reference
        let count = (&touch).into_iter().count();
        assert_eq!(count, 3);
    }
}
