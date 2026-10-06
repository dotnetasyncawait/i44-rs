use rhk::{
	App,
	apps::vscode, input::{make, hotstr::{Hotstr, clipb, clipb_no_restore, input}},
	misc::{self, win},
	common::error::Error};

type HotstrResult = Result<Hotstr, Error>;

pub trait AppExt {
	fn add_hotstrs(self) -> Self;
}

impl AppExt for App {
	fn add_hotstrs(self) -> Self {
		self
			.hotstr(":pl", || input(make!("println!(\"\");" LEFT:3)).ok())
			.hotstr(":grsh", || clipb("git reset --soft HEAD~").ok())
			.hotstr(":grhh", || clipb("git reset --hard HEAD~").ok())
			.hotstr(":grs", || clipb("git reset --soft ").ok())
			.hotstr(":grh", || clipb("git reset --hard ").ok())
			.hotstr(":ghr", || clipb("https://github.com/dotnetasyncawait?tab=repositories").ok())
			.hotstr(":gcm", || input(make!("git cm \"\"" LEFT)).ok())
			.hotstr(":ghahk", || clipb("https://github.com/AutoHotkey/AutoHotkey").ok())
			.hotstr(":ahkd", || clipb("https://www.autohotkey.com/docs/v2/").ok())
			.hotstr(":yt", || clipb("https://www.youtube.com/").ok())
			.hotstr(":ytwl", || clipb("https://www.youtube.com/playlist?list=WL").ok())
			.hotstr(":ythis", || clipb("https://www.youtube.com/feed/history").ok())
			.hotstr(":ytsub", || clipb("https://www.youtube.com/feed/subscriptions").ok())
			.hotstr(":qmkd", || clipb("https://docs.qmk.fm/#/").ok())
			.hotstr(":cpi44", || clipb("qmk compile -kb ergohaven/imperial44 -km schmidt-x").ok())
			.hotstr(":qmkkc", || clipb("https://github.com/qmk/qmk_firmware/blob/master/quantum/keycodes.h").ok())
			.hotstr(":qmkqkc", || clipb("https://github.com/qmk/qmk_firmware/blob/master/quantum/quantum_keycodes.h").ok())
			.hotstr(":qmkmods", || clipb("https://github.com/qmk/qmk_firmware/blob/master/quantum/modifiers.h").ok())
			.hotstr(":dll", || input(make!("DllCall(\"\")" LEFT:2)).ok())
			.hotstr(":gxsh", || clipb("opera://settings/keyboardShortcuts").ok())
			.hotstr(":maps", || clipb("https://www.google.com/maps/").ok())
			.hotstr(":gm", || clipb("https://mail.google.com/mail/u/0/#all").ok())
			.hotstr(":aaa", || input(make!("// Arrange" ENTER:3 "// Act" ENTER:3 "// Assert" ENTER UP:6)).ok())
			.hotstr(":tz", || clipb("https://www.timeanddate.com/time/zone/").ok())
			.hotstr(":urb", || clipb("https://www.urbandictionary.com/").ok())
			.hotstr(":winrsd", || clipb("https://microsoft.github.io/windows-docs-rs/doc/windows/").ok())
			.hotstr("`ahk", ahk_block)
			.hotstr("`rs", rs_block)
			.hotstr(":idk", || clipb(r"¯\_(ツ)_/¯").ok())
			.hotstr(":mb", || input(make!("MsgBox()" LEFT)).ok())
			.hotstr(":guid", || clipb_no_restore(misc::make_guid()?).ok())
			
			.hotstr(":gt",  || todo!()) // "hh:mm:ss tt"
			.hotstr(":gd",  || todo!()) // "dd-MMM-yy"
			.hotstr(":gdt", || todo!()) // "dd-MMM-yy hh:mm:ss tt"
			.hotstr(":@", || todo!()) // menu
	}
}

fn ahk_block() -> HotstrResult {
	match win::name()?.as_str() {
		vscode::NAME => input(make!("```ahk" ENTER:2 UP)), 
		_ => input(make!("```ahk" LS{ENTER:2} "```" UP))
	}.ok()
}

fn rs_block() -> HotstrResult {
	match win::name()?.as_str() {
		vscode::NAME => input(make!("```rs" ENTER:2 UP)), 
		_ => input(make!("```rs" LS{ENTER:2} "```" UP))
	}.ok()
}
