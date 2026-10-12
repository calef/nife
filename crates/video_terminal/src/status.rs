/// The terminal is up: `send(REPORT, TERM_UP, cols | rows << 32, mode)`. Sent once, after the
/// blank grid has been presented, so a spawner that sees it knows the geometry was negotiated
/// and the first picture reached the screen.
///
/// **One report, ever**, for the reason `compositor::status::COMP_UP` gives: a status `SEND` is a
/// rendezvous, so a component that narrated every frame would block until its spawner listened,
/// and a spawner that stopped listening would wedge everything behind it. What is on the screen
/// is observable where it belongs: in the frames, and at the display endpoint.
pub const TERM_UP: u64 = 0x7E7_0001;

/// The terminal drives a display endpoint directly (`gfx FLUSH`), owning the whole scanout.
pub const MODE_DISPLAY: u64 = 0;
/// The terminal is a compositor client, owning one window (`compose COMMIT`).
pub const MODE_WINDOW: u64 = 1;

/// The keyboard driver is up: `send(REPORT, KEYBOARD_UP, buffers posted, 0)`. The device is
/// enumerated, the event queue is programmed through the confined transport, and every
/// device-writable buffer is posted, so a spawner that sees this knows a key pressed from here
/// on has somewhere to land. Also one report, ever, and for the same reason [`TERM_UP`] is.
///
/// Renamed from `KBD_UP` (calef, 2026-08-27), the same pass that renamed `user/src/kbd.rs` to
/// `keyboard_driver.rs`: the constant embedded the old short name and would have gone stale.
pub const KEYBOARD_UP: u64 = 0x7E7_0002;
