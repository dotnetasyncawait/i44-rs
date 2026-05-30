use std::ops::BitOr;
use std::fmt::{self, Debug, Formatter};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mods(u8);

impl Mods {
	#[allow(dead_code)]
	const NONE: Mods = Mods(0);
	
	pub const LC: Mods = Mods(0x01); // LCtrl
	pub const LS: Mods = Mods(0x02); // LShift
	pub const LA: Mods = Mods(0x04); // LAlt
	pub const LW: Mods = Mods(0x08); // LWin
	pub const RC: Mods = Mods(0x10); // RCtrl
	pub const RS: Mods = Mods(0x20); // RShift
	pub const RA: Mods = Mods(0x40); // RAlt
	pub const RW: Mods = Mods(0x80); // RWin
	
	pub const LCS: Mods = Mods::LC.or(Mods::LS);
	pub const LCA: Mods = Mods::LC.or(Mods::LA);
	pub const LCW: Mods = Mods::LC.or(Mods::LW);
	pub const LSA: Mods = Mods::LS.or(Mods::LA);
	pub const LSW: Mods = Mods::LS.or(Mods::LW);
	pub const LAW: Mods = Mods::LA.or(Mods::LW);
	pub const RCS: Mods = Mods::RC.or(Mods::RS);
	pub const RCA: Mods = Mods::RC.or(Mods::RA);
	pub const RCW: Mods = Mods::RC.or(Mods::RW);
	pub const RSA: Mods = Mods::RS.or(Mods::RA);
	pub const RSW: Mods = Mods::RS.or(Mods::RW);
	pub const RAW: Mods = Mods::RA.or(Mods::RW);
	
	pub const LCSA: Mods = Mods::LCS.or(Mods::LA);
	pub const LCSW: Mods = Mods::LCS.or(Mods::LW);
	pub const LCAW: Mods = Mods::LCA.or(Mods::LW);
	pub const LSAW: Mods = Mods::LSA.or(Mods::LW);
	pub const RCSA: Mods = Mods::RCS.or(Mods::RA);
	pub const RCSW: Mods = Mods::RCS.or(Mods::RW);
	pub const RCAW: Mods = Mods::RCA.or(Mods::RW);
	pub const RSAW: Mods = Mods::RSA.or(Mods::RW);
	
	pub const LCSAW: Mods = Mods::LCSA.or(Mods::LW);
	pub const RCSAW: Mods = Mods::RCSA.or(Mods::RW);
	
	const fn or(self, other: Self) -> Self {
		Self(self.0 | other.0)
	}
}

impl BitOr for Mods {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		Self(self.0 | rhs.0)
	}
}

impl Debug for Mods {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "Mods({:04b}_{:04b})", self.0 >> 4, self.0 & 0xF)
	}
}