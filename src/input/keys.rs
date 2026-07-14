use std::fmt::{self, Debug, Formatter};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(pub(super) u16);

impl Debug for Key {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "Key(0x{:04X})", self.0)
	}
}

impl Key {
	pub const NONE: Key = Key(0);
	
	pub const A: Key = Key(0x001E); // Letters
	pub const B: Key = Key(0x0030);
	pub const C: Key = Key(0x002E);
	pub const D: Key = Key(0x0020);
	pub const E: Key = Key(0x0012);
	pub const F: Key = Key(0x0021);
	pub const G: Key = Key(0x0022);
	pub const H: Key = Key(0x0023);
	pub const I: Key = Key(0x0017);
	pub const J: Key = Key(0x0024);
	pub const K: Key = Key(0x0025);
	pub const L: Key = Key(0x0026);
	pub const M: Key = Key(0x0032);
	pub const N: Key = Key(0x0031);
	pub const O: Key = Key(0x0018);
	pub const P: Key = Key(0x0019);
	pub const Q: Key = Key(0x0010);
	pub const R: Key = Key(0x0013);
	pub const S: Key = Key(0x001F);
	pub const T: Key = Key(0x0014);
	pub const U: Key = Key(0x0016);
	pub const V: Key = Key(0x002F);
	pub const W: Key = Key(0x0011);
	pub const X: Key = Key(0x002D);
	pub const Y: Key = Key(0x0015);
	pub const Z: Key = Key(0x002C);
	
	pub const NUM1: Key = Key(0x0002); // Numbers
	pub const NUM2: Key = Key(0x0003);
	pub const NUM3: Key = Key(0x0004);
	pub const NUM4: Key = Key(0x0005);
	pub const NUM5: Key = Key(0x0006);
	pub const NUM6: Key = Key(0x0007);
	pub const NUM7: Key = Key(0x0008);
	pub const NUM8: Key = Key(0x0009);
	pub const NUM9: Key = Key(0x000A);
	pub const NUM0: Key = Key(0x000B);
	
	pub const ESCAPE:  Key = Key(0x0001); // Navigation
	pub const ENTER:   Key = Key(0x001C);
	pub const TAB:     Key = Key(0x000F);
	pub const SPACE:   Key = Key(0x0039);
	pub const BS:      Key = Key(0x000E);
	pub const DEL:     Key = Key(0xE053);
	pub const INSERT:  Key = Key(0xE052);
	pub const HOME:    Key = Key(0xE047);
	pub const END:     Key = Key(0xE04F);
	pub const PG_UP:   Key = Key(0xE049);
	pub const PG_DOWN: Key = Key(0xE051);
	pub const UP:      Key = Key(0xE048);
	pub const DOWN:    Key = Key(0xE050);
	pub const LEFT:    Key = Key(0xE04B);
	pub const RIGHT:   Key = Key(0xE04D);
	
	pub const DASH:       Key = Key(0x000C); // Symbols
	pub const EQUALS:     Key = Key(0x000D);
	pub const LBRACE:     Key = Key(0x001A);
	pub const RBRACE:     Key = Key(0x001B);
	pub const BSLASH:     Key = Key(0x002B);
	pub const SEMICOLON:  Key = Key(0x0027);
	pub const APOSTROPHE: Key = Key(0x0028);
	pub const GRAVE:      Key = Key(0x0029);
	pub const COMMA:      Key = Key(0x0033);
	pub const PERIOD:     Key = Key(0x0034);
	pub const FSLASH:     Key = Key(0x0035);
	
	pub const KEYPAD1:       Key = Key(0x004F); // Keypad
	pub const KEYPAD2:       Key = Key(0x0050);
	pub const KEYPAD3:       Key = Key(0x0051);
	pub const KEYPAD4:       Key = Key(0x004B);
	pub const KEYPAD5:       Key = Key(0x004C);
	pub const KEYPAD6:       Key = Key(0x004D);
	pub const KEYPAD7:       Key = Key(0x0047);
	pub const KEYPAD8:       Key = Key(0x0048);
	pub const KEYPAD9:       Key = Key(0x0049);
	pub const KEYPAD0:       Key = Key(0x0052);
	pub const KEYPAD_FSLASH: Key = Key(0xE035);
	pub const KEYPAD_STAR:   Key = Key(0x0037);
	pub const KEYPAD_DASH:   Key = Key(0x004A);
	pub const KEYPAD_PLUS:   Key = Key(0x004E);
	pub const KEYPAD_ENTER:  Key = Key(0xE01C);
	pub const KEYPAD_PERIOD: Key = Key(0x0053);
	
	pub const F1:  Key = Key(0x003B); // Function keys
	pub const F2:  Key = Key(0x003C);
	pub const F3:  Key = Key(0x003D);
	pub const F4:  Key = Key(0x003E);
	pub const F5:  Key = Key(0x003F);
	pub const F6:  Key = Key(0x0040);
	pub const F7:  Key = Key(0x0041);
	pub const F8:  Key = Key(0x0042);
	pub const F9:  Key = Key(0x0043);
	pub const F10: Key = Key(0x0044);
	pub const F11: Key = Key(0x0057);
	pub const F12: Key = Key(0x0058);
	pub const F13: Key = Key(0x0064);
	pub const F14: Key = Key(0x0065);
	pub const F15: Key = Key(0x0066);
	pub const F16: Key = Key(0x0067);
	pub const F17: Key = Key(0x0068);
	pub const F18: Key = Key(0x0069);
	pub const F19: Key = Key(0x006A);
	pub const F20: Key = Key(0x006B);
	pub const F21: Key = Key(0x006C);
	pub const F22: Key = Key(0x006D);
	pub const F23: Key = Key(0x006E);
	pub const F24: Key = Key(0x0076);
	
	pub const LCTRL:  Key = Key(0x001D); // Mods
	pub const LSHIFT: Key = Key(0x002A);
	pub const LALT:   Key = Key(0x0038);
	pub const LWIN:   Key = Key(0xE05B);
	pub const RCTRL:  Key = Key(0xE01D);
	pub const RSHIFT: Key = Key(0x0036);
	pub const RALT:   Key = Key(0xE038);
	pub const RWIN:   Key = Key(0xE05C);
	
	pub const BROWSER_BACK:     Key = Key(0xE06A); // VK_BROWSER_BACK        0xA6 // Consumer
	pub const BROWSER_FORWARD:  Key = Key(0xE069); // VK_BROWSER_FORWARD     0xA7
	pub const BROWSER_REFRESH:  Key = Key(0xE067); // VK_BROWSER_REFRESH     0xA8
	pub const BROWSER_STOP:     Key = Key(0xE068); // VK_BROWSER_STOP        0xA9
	pub const BROWSER_SEARCH:   Key = Key(0xE065); // VK_BROWSER_SEARCH      0xAA
	pub const BROWSER_FAV:      Key = Key(0xE066); // VK_BROWSER_FAVORITES   0xAB
	pub const BROWSER_HOME:     Key = Key(0xE032); // VK_BROWSER_HOME        0xAC
	pub const VOLUME_MUTE:      Key = Key(0xE020); // VK_VOLUME_MUTE         0xAD
	pub const VOLUME_DOWN:      Key = Key(0xE02E); // VK_VOLUME_DOWN         0xAE
	pub const VOLUME_UP:        Key = Key(0xE030); // VK_VOLUME_UP           0xAF
	pub const MEDIA_NEXT_TRACK: Key = Key(0xE019); // VK_MEDIA_NEXT_TRACK    0xB0
	pub const MEDIA_PREV_TRACK: Key = Key(0xE010); // VK_MEDIA_PREV_TRACK    0xB1
	pub const MEDIA_STOP:       Key = Key(0xE024); // VK_MEDIA_STOP          0xB2
	pub const MEDIA_PLAY_PAUSE: Key = Key(0xE022); // VK_MEDIA_PLAY_PAUSE    0xB3
	pub const LAUNCH_MAIL:      Key = Key(0xE06C); // VK_LAUNCH_MAIL         0xB4
	pub const LAUNCH_MEDIA:     Key = Key(0xE06D); // VK_LAUNCH_MEDIA_SELECT 0xB5
	pub const LAUNCH_APP1:      Key = Key(0xE06B); // VK_LAUNCH_APP1         0xB6
	pub const LAUNCH_APP2:      Key = Key(0xE021); // VK_LAUNCH_APP2         0xB7
	
	pub const CAPSLOCK:    Key = Key(0x003A); // Misc
	pub const SCROLL_LOCK: Key = Key(0x0046);
	pub const NUMLOCK:     Key = Key(0xE045);
	
	pub const PRINTSCREEN: Key = Key(0xE037);
	pub const PAUSE:       Key = Key(0x0045);
	pub const APP:         Key = Key(0xE05D);
	
	pub const LBUTTON:  Key = Key(0x0200); // Mouse keys
	pub const RBUTTON:  Key = Key(0x0201);
	pub const MBUTTON:  Key = Key(0x0202);
	pub const XBUTTON1: Key = Key(0x0203);
	pub const XBUTTON2: Key = Key(0x0204);
	pub const WH_UP:    Key = Key(0x0205);
	pub const WH_DOWN:  Key = Key(0x0206);
	pub const WH_LEFT:  Key = Key(0x0207);
	pub const WH_RIGHT: Key = Key(0x0208);
	
	pub const WH_UP_X2:  Key = Key::WH_UP.or(0x1000); // Wheel multipliers
	pub const WH_UP_X3:  Key = Key::WH_UP.or(0x2000);
	pub const WH_UP_X4:  Key = Key::WH_UP.or(0x3000);
	pub const WH_UP_X5:  Key = Key::WH_UP.or(0x4000);
	pub const WH_UP_X6:  Key = Key::WH_UP.or(0x5000);
	pub const WH_UP_X7:  Key = Key::WH_UP.or(0x6000);
	pub const WH_UP_X8:  Key = Key::WH_UP.or(0x7000);
	pub const WH_UP_X9:  Key = Key::WH_UP.or(0x8000);
	pub const WH_UP_X10: Key = Key::WH_UP.or(0x9000);
	pub const WH_UP_X11: Key = Key::WH_UP.or(0xA000);
	pub const WH_UP_X12: Key = Key::WH_UP.or(0xB000);
	pub const WH_UP_X13: Key = Key::WH_UP.or(0xC000);
	pub const WH_UP_X14: Key = Key::WH_UP.or(0xD000);
	pub const WH_UP_X15: Key = Key::WH_UP.or(0xE000);
	pub const WH_UP_X16: Key = Key::WH_UP.or(0xF000);
	
	pub const WH_DOWN_X2:  Key = Key::WH_DOWN.or(0x1000);
	pub const WH_DOWN_X3:  Key = Key::WH_DOWN.or(0x2000);
	pub const WH_DOWN_X4:  Key = Key::WH_DOWN.or(0x3000);
	pub const WH_DOWN_X5:  Key = Key::WH_DOWN.or(0x4000);
	pub const WH_DOWN_X6:  Key = Key::WH_DOWN.or(0x5000);
	pub const WH_DOWN_X7:  Key = Key::WH_DOWN.or(0x6000);
	pub const WH_DOWN_X8:  Key = Key::WH_DOWN.or(0x7000);
	pub const WH_DOWN_X9:  Key = Key::WH_DOWN.or(0x8000);
	pub const WH_DOWN_X10: Key = Key::WH_DOWN.or(0x9000);
	pub const WH_DOWN_X11: Key = Key::WH_DOWN.or(0xA000);
	pub const WH_DOWN_X12: Key = Key::WH_DOWN.or(0xB000);
	pub const WH_DOWN_X13: Key = Key::WH_DOWN.or(0xC000);
	pub const WH_DOWN_X14: Key = Key::WH_DOWN.or(0xD000);
	pub const WH_DOWN_X15: Key = Key::WH_DOWN.or(0xE000);
	pub const WH_DOWN_X16: Key = Key::WH_DOWN.or(0xF000);
	
	pub const WH_LEFT_X2:  Key = Key::WH_LEFT.or(0x1000);
	pub const WH_LEFT_X3:  Key = Key::WH_LEFT.or(0x2000);
	pub const WH_LEFT_X4:  Key = Key::WH_LEFT.or(0x3000);
	pub const WH_LEFT_X5:  Key = Key::WH_LEFT.or(0x4000);
	pub const WH_LEFT_X6:  Key = Key::WH_LEFT.or(0x5000);
	pub const WH_LEFT_X7:  Key = Key::WH_LEFT.or(0x6000);
	pub const WH_LEFT_X8:  Key = Key::WH_LEFT.or(0x7000);
	pub const WH_LEFT_X9:  Key = Key::WH_LEFT.or(0x8000);
	pub const WH_LEFT_X10: Key = Key::WH_LEFT.or(0x9000);
	pub const WH_LEFT_X11: Key = Key::WH_LEFT.or(0xA000);
	pub const WH_LEFT_X12: Key = Key::WH_LEFT.or(0xB000);
	pub const WH_LEFT_X13: Key = Key::WH_LEFT.or(0xC000);
	pub const WH_LEFT_X14: Key = Key::WH_LEFT.or(0xD000);
	pub const WH_LEFT_X15: Key = Key::WH_LEFT.or(0xE000);
	pub const WH_LEFT_X16: Key = Key::WH_LEFT.or(0xF000);
	
	pub const WH_RIGHT_X2:  Key = Key::WH_RIGHT.or(0x1000);
	pub const WH_RIGHT_X3:  Key = Key::WH_RIGHT.or(0x2000);
	pub const WH_RIGHT_X4:  Key = Key::WH_RIGHT.or(0x3000);
	pub const WH_RIGHT_X5:  Key = Key::WH_RIGHT.or(0x4000);
	pub const WH_RIGHT_X6:  Key = Key::WH_RIGHT.or(0x5000);
	pub const WH_RIGHT_X7:  Key = Key::WH_RIGHT.or(0x6000);
	pub const WH_RIGHT_X8:  Key = Key::WH_RIGHT.or(0x7000);
	pub const WH_RIGHT_X9:  Key = Key::WH_RIGHT.or(0x8000);
	pub const WH_RIGHT_X10: Key = Key::WH_RIGHT.or(0x9000);
	pub const WH_RIGHT_X11: Key = Key::WH_RIGHT.or(0xA000);
	pub const WH_RIGHT_X12: Key = Key::WH_RIGHT.or(0xB000);
	pub const WH_RIGHT_X13: Key = Key::WH_RIGHT.or(0xC000);
	pub const WH_RIGHT_X14: Key = Key::WH_RIGHT.or(0xD000);
	pub const WH_RIGHT_X15: Key = Key::WH_RIGHT.or(0xE000);
	pub const WH_RIGHT_X16: Key = Key::WH_RIGHT.or(0xF000);
	
	const fn or(self, other: u16) -> Self {
		Self(self.0 | other)
	}
	
	pub(super) fn is_mouse_key(self) -> bool { self.0 & 0x0200 != 0 }

	pub(super) fn is_mouse_wheel(self) -> bool {
		const L: u16 = Key::WH_UP.0;
		const U: u16 = Key::WH_RIGHT.0;
		matches!(self.0 & 0x02FF, L..=U) // excluding wheel-multiplier
	}

	pub(super) fn is_mouse_button(self) -> bool {
		const L: u16 = Key::LBUTTON.0;
		const U: u16 = Key::XBUTTON2.0;
		matches!(self.0, L..=U)
	}

	pub(super) fn is_extended_key(self) -> bool { self.0 & 0xE000 == 0xE000 }
}