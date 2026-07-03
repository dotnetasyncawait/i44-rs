use i44::input::{hotkey::Hotkey::{self, *}, mods::Mods, keys::Key};
use i44::common::error::Error;
use i44::misc::{win, helpers};
use i44::apps::*;
use i44::App;
use super::{mode::{Mode, ModeState}, kb::I44, mic};
use crate::system::paths;

type HotkeyResult = Result<Hotkey, Error>;

pub trait AppExt {
	fn add_hotkeys(self) -> Self;
}

impl AppExt for App {
	fn add_hotkeys(self) -> Self {
		self
			.hotkey(Mods::NONE, Key::B, b)
			.hotkey(Mods::LS,   Key::B, ls_b)
			.hotkey(Mods::NONE, Key::C, c)
			.hotkey(Mods::LS,   Key::C, ls_c)
			.hotkey(Mods::LS,   Key::D, ls_d)
			.hotkey(Mods::LW,   Key::E, lw_e)
			.hotkey(Mods::NONE, Key::G, g)
			.hotkey(Mods::NONE, Key::H, h)
			.hotkey(Mods::LS,   Key::H, ls_h)
			.hotkey(Mods::NONE, Key::K, k)
			.hotkey(Mods::LS,   Key::K, lc_k)
			.hotkey(Mods::NONE, Key::L, l)
			.hotkey(Mods::NONE, Key::P, p)
			.hotkey(Mods::LS,   Key::P, ls_p)
			.hotkey(Mods::LS,   Key::S, ls_s)
			.hotkey(Mods::LCS,  Key::S, lcs_s)
			.hotkey(Mods::NONE, Key::V, v)
			.hotkey(Mods::NONE, Key::W, w)
			.hotkey(Mods::LS,   Key::W, ls_w)
			.hotkey(Mods::NONE, Key::X, x)
			.hotkey(Mods::LS,   Key::X, ls_x)
			.hotkey(Mods::NONE, Key::Y, y)
			
			.hotkey(Mods::NONE,  Key::PERIOD, period)
			.hotkey(Mods::NONE,  Key::APOSTROPHE, apstrph)
			.hotkey(Mods::RS,    Key::LBRACE, rs_lbrace) // {
			.hotkey(Mods::RS,    Key::RBRACE, rs_rbrace) // }
			.hotkey(Mods::LS_RS, Key::LBRACE, ls_rs_lbrace) // LS + {
			.hotkey(Mods::LS_RS, Key::RBRACE, ls_rs_rbrace) // LS + }
			.hotkey(Mods::LS,    Key::DASH, ls_dash)
			.hotkey(Mods::LS_RS, Key::FSLASH, ls_rs_fslash) // LS + ?
			.hotkey(Mods::LS,    Key::BSLASH, ls_bslash)
			
			.hotkey(Mods::LC, Key::NUM1, lc_1)
			.hotkey(Mods::LS, Key::NUM1, ls_1)
			.hotkey(Mods::LC, Key::NUM2, lc_2)
			.hotkey(Mods::RS, Key::NUM2, rs_2) // @
			.hotkey(Mods::LC, Key::NUM3, lc_3)
			.hotkey(Mods::LS, Key::NUM3, ls_3)
			.hotkey(Mods::LC, Key::NUM4, lc_4)
			.hotkey(Mods::LC, Key::NUM5, lc_5)
			.hotkey(Mods::LC, Key::NUM6, lc_6)
			.hotkey(Mods::LS, Key::NUM7, ls_7)
			.hotkey(Mods::LS, Key::NUM8, ls_8)
			.hotkey(Mods::LS, Key::NUM9, ls_9)
			.hotkey(Mods::LC, Key::NUM0, lc_0)
			.hotkey_exempt(Mods::LSW, Key::NUM0, suspend)
			.hotkey_exempt(Mods::LW,  Key::NUM0, exit)
			
			.hotkey(Mods::LS,    Key::UP, ls_up)
			.hotkey(Mods::LS,    Key::DOWN, ls_down)
			.hotkey(Mods::LA,    Key::UP, la_up)
			.hotkey(Mods::LA,    Key::DOWN, la_down)
			.hotkey(Mods::LCS,   Key::UP, lcs_up)
			.hotkey(Mods::LCS,   Key::DOWN, lcs_down)
			.hotkey(Mods::LSA,   Key::UP, lsa_up)
			.hotkey(Mods::LSA,   Key::DOWN, lsa_down)
			.hotkey(Mods::LS_RS, Key::UP, ls_rs_up)
			.hotkey(Mods::LS_RS, Key::DOWN, ls_rs_down)
			.hotkey(Mods::LS,    Key::LEFT, ls_left)
			.hotkey(Mods::LS,    Key::RIGHT, ls_right)
			.hotkey(Mods::LC,    Key::HOME, lc_home)
			.hotkey(Mods::LC,    Key::END, lc_end)
			.hotkey(Mods::LC_RS, Key::HOME, lc_rs_home)
			.hotkey(Mods::LC_RS, Key::END, lc_rs_end)
			.hotkey(Mods::LS_RS, Key::HOME, ls_rs_home)
			.hotkey(Mods::LS_RS, Key::END, ls_rs_end)
			.hotkey(Mods::LC,    Key::PG_UP, lc_pg_up)
			.hotkey(Mods::LC,    Key::PG_DOWN, lc_pg_down)
			.hotkey(Mods::NONE,  Key::INSERT, insert)
			// .hotkey(Mods::LC,    Key::ENTER, lc_enter) // TODO: Paths, Clipboard
			.hotkey(Mods::LS,    Key::ENTER, ls_enter)
			.hotkey(Mods::LC,    Key::SPACE, lc_space)
			
			.hotkey(Mods::LS, Key::XBUTTON1, ls_xbutton1)
			.hotkey(Mods::LS, Key::LBUTTON, ls_lbutton)
			
			.hotkey(Mods::NONE, Key::F3, f3)
			.hotkey(Mods::NONE, Key::F4, f4)
			.hotkey(Mods::NONE, Key::F6, f6)
			.hotkey(Mods::NONE, Key::F7, f7)
			.hotkey(Mods::NONE, Key::F8, f8)
			.hotkey(Mods::NONE, Key::F21, f21)
			.hotkey_exempt(Mods::NONE, Key::F23, f23)
	}
}

fn b() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::go_to_definition(),
			discord::NAME => discord::nav_to_curr_call(),
			_ => Suppress
		},
		_ => Default
	}.ok()
}

fn ls_b() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::go_to_impl(),
			_ => Suppress
		},
		_ => Default
	}.ok()
}

fn c() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			chrome::NAME => chrome::focus_on_addr_bar(),
			explorer::NAME => explorer::focus_on_addr_bar(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn ls_c() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::copy_line_down(),
			wt::NAME => wt::duplicate_tab(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn ls_d() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			discord::NAME => discord::disconnect(),
			_ => Suppress
		},
		_ => Default
		
	}.ok()
}

fn lw_e() -> HotkeyResult {
	explorer::open(paths::DESKTOP)?;
	Ok(Suppress)
}

fn g() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => Ok(Suppress),
		_ => Ok(Default)
	}
}

fn h() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::unfold(),
			obsid::NAME => obsid::fold_less(),
			_ => Suppress
		},
		_ => Default
	}.ok()
}

fn ls_h() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::unfold_all(),
			obsid::NAME => obsid::unfold_all_headings_and_lists(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn k() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::goto_bracket(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn lc_k() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::togg_breakpoint(),
			_ => Default
		}
		_ => Default
	}.ok()
}

fn l() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::show_or_focus_hover(),
			chrome::NAME if chrome::is_youtube(win::title()?.as_str()) => chrome::togg_loop_mode(),
			discord::NAME => discord::togg_member_list_or_vc_chat(),
			_ => Suppress
		},
		_ => Default
	}.ok()
}

fn p() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			obsid::NAME => obsid::togg_reading_view(),
			chrome::NAME => chrome::reload_tab(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn ls_p() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			chrome::NAME => chrome::reload_tab_ignore_cache(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn ls_s() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			discord::NAME => discord::upload_file(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn lcs_s() -> HotkeyResult {
	match win::name()?.as_str() {
		vscode::NAME => vscode::open_settings(),
		obsid::NAME => obsid::open_settings(),
		wt::NAME => wt::open_settings(),
		_ => Suppress
	}.ok()
}

fn v() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => Ok(Suppress),
		_ => Ok(Default)
	}
}

fn w() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::close_editor(),
			chrome::NAME => chrome::close_tab(),
			wt::NAME => wt::close_pane(),
			obsid::NAME => obsid::close_curr_tab(),
			explorer::NAME => explorer::close_tab(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn ls_w() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::reopen_last_closed_tab(),
			chrome::NAME => chrome::reopen_last_closed_tab(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn x() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::fold(),
			obsid::NAME => obsid::fold_more(),
			chrome::NAME => chrome::new_tab(),
			explorer::NAME => explorer::new_tab(),
			wt::NAME => wt::new_tab(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn ls_x() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::fold_all(),
			obsid::NAME => obsid::fold_all_headings_and_lists(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn y() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::param_hints(),
			chrome::NAME => chrome::open_home_page(),
			_ => Suppress
		},
		_ => Default
	}.ok()
}

fn apstrph() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::comment_line(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn period() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::quick_fix(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn rs_lbrace() -> HotkeyResult {
	match Mode::get() {
		ModeState::NSymbol => match win::name()?.as_str() {
			vscode::NAME => vscode::prev_member(),
			chrome::NAME => chrome::decrease_playb_speed(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn rs_rbrace() -> HotkeyResult {
	match Mode::get() {
		ModeState::NSymbol => match win::name()?.as_str() {
			vscode::NAME => vscode::next_member(),
			chrome::NAME => chrome::increase_playb_speed(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn ls_rs_lbrace() -> HotkeyResult {
	match Mode::get() {
		ModeState::ISymbol => Unicode("«"),
		_ => Default
	}.ok()
}

fn ls_rs_rbrace() -> HotkeyResult {
	match Mode::get() {
		ModeState::ISymbol => Unicode("»"),
		_ => Default
	}.ok()
}

fn ls_dash() -> HotkeyResult {
	match Mode::get() {
		ModeState::ISymbol => Unicode(":="),
		_ => Default
	}.ok()
}

fn ls_bslash() -> HotkeyResult {
	match Mode::get() {
		ModeState::ISymbol => Unicode("\\n"),
		_ => Default
	}.ok()
}

fn ls_rs_fslash() -> HotkeyResult {
	match Mode::get() {
		ModeState::ISymbol => Unicode("—"),
		_ => Default
	}.ok()
}

fn lc_1() -> HotkeyResult {
	match win::name()?.as_str() {
		vscode::NAME => vscode::show_explorer(),
		obsid::NAME => obsid::explorer_focus(),
		wt::NAME => wt::switch_to_tab0(),
		_ => Default
	}.ok()
}

fn ls_1() -> HotkeyResult {
	match Mode::get() {
		ModeState::NSymbol => match win::name()?.as_str() {
			vscode::NAME => vscode::copy_cursor_up(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn lc_2() -> HotkeyResult {
	match win::name()?.as_str() {
		vscode::NAME => vscode::terminal(),
		wt::NAME => wt::switch_to_tab1(),
		_ => Default
	}.ok()
}

fn rs_2() -> HotkeyResult {
	match Mode::get() {
		ModeState::NSymbol => match win::name()?.as_str() {
			chrome::NAME => chrome::default_playb_speed(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn lc_3() -> HotkeyResult {
	match win::name()?.as_str() {
		vscode::NAME => vscode::debug(),
		wt::NAME => wt::switch_to_tab2(),
		obsid::NAME => obsid::show_outline(),
		_ => Default
	}.ok()
}

fn ls_3() -> HotkeyResult {
	match Mode::get() {
		ModeState::NSymbol => match win::name()?.as_str() {
			vscode::NAME => vscode::copy_cursor_down(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn lc_4() -> HotkeyResult {
	match win::name()?.as_str() {
		wt::NAME => wt::switch_to_tab3(),
		_ => Default
	}.ok()
}

fn lc_5() -> HotkeyResult {
	match win::name()?.as_str() {
		wt::NAME => wt::switch_to_tab4(),
		_ => Default
	}.ok()
}

fn lc_6() -> HotkeyResult {
	match win::name()?.as_str() {
		vscode::NAME => vscode::togg_source_ctrl(),
		wt::NAME => wt::switch_to_tab5(),
		_ => Default
	}.ok()
}

fn ls_7() -> HotkeyResult {
	match Mode::get() {
		ModeState::ISymbol => Unicode("–"),
		_ => Default
	}.ok()
}

fn ls_8() -> HotkeyResult {
	match Mode::get() {
		ModeState::ISymbol => Unicode("->"),
		_ => Default
	}.ok()
}

fn ls_9() -> HotkeyResult {
	match Mode::get() {
		ModeState::ISymbol => Unicode("=>"),
		_ => Default
	}.ok()
}

fn lc_0() -> HotkeyResult {
	match win::name()?.as_str() {
		chrome::NAME => chrome::jump_to_rightmost_tab(),
		wt::NAME => wt::switch_to_last_tab(),
		_ => Default
	}.ok()
}

fn ls_up() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => { helpers::center_cursor()?; Remap(Mods::NONE, Key::WH_UP_X2) },
		_ => Suppress
	}.ok()
}

fn ls_down() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => { helpers::center_cursor()?; Remap(Mods::NONE, Key::WH_DOWN_X2) },
		_ => Suppress
	}.ok()
}

fn la_up() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::scroll_term_up_by_line(),
			wt::NAME => wt::scroll_up(),
			_ => Default
		}
		_ => Default
	}.ok()
}

fn la_down() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::scroll_term_down_by_line(),
			wt::NAME => wt::scroll_down(),
			_ => Default
		}
		_ => Default
	}.ok()
}

fn lcs_up() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => { helpers::center_cursor()?; vscode::scroll_up_fast() },
			tg::NAME => { helpers::center_cursor()?; tg::scroll_page_up() },
			_ => Suppress
		},
		_ => Default
	}.ok()
}

fn lcs_down() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => { helpers::center_cursor()?; vscode::scroll_down_fast() },
			tg::NAME => { helpers::center_cursor()?; tg::scroll_page_down() },
			_ => Suppress
		},
		_ => Default
	}.ok()
}

fn lsa_up() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::scroll_term_up_by_page(),
			wt::NAME => wt::scroll_page_up(),
			_ => Default
		}
		_ => Default
	}.ok()
}

fn lsa_down() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::scroll_term_down_by_page(),
			wt::NAME => wt::scroll_page_down(),
			_ => Default
		}
		_ => Default
	}.ok()
}

fn ls_rs_up() -> HotkeyResult {
	match Mode::get() {
		ModeState::Select => { helpers::center_cursor()?; Remap(Mods::NONE, Key::WH_UP_X2) },
		_ => Suppress
	}.ok()
}

fn ls_rs_down() -> HotkeyResult {
	match Mode::get() {
		ModeState::Select => { helpers::center_cursor()?; Remap(Mods::NONE, Key::WH_DOWN_X2) },
		_ => Suppress
	}.ok()
}

fn ls_left() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			obsid::NAME => obsid::find_prev(),
			_ => Default
		}
		_ => Default
	}.ok()
}

fn ls_right() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			obsid::NAME => obsid::find_next(),
			_ => Default
		}
		_ => Default
	}.ok()
}

fn lc_home() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::prev_tab(),
			chrome::NAME => chrome::prev_tab(),
			wt::NAME => wt::prev_tab(),
			explorer::NAME => explorer::prev_tab(),
			obsid::NAME => obsid::prev_tab(),
			_ => Default
		}
		_ => Default
	}.ok()
}

fn lc_end() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => match win::name()?.as_str() {
			vscode::NAME => vscode::next_tab(),
			chrome::NAME => chrome::next_tab(),
			wt::NAME => wt::next_tab(),
			explorer::NAME => explorer::next_tab(),
			obsid::NAME => obsid::next_tab(),
			_ => Default
		}
		_ => Default
	}.ok()
}

fn ls_rs_home() -> HotkeyResult {
	match Mode::get() {
		ModeState::Select => match win::name()?.as_str() {
			vscode::NAME => vscode::shrink_selection(),
			_ => Default
		},
		_ => Default
	}.ok()
}

fn ls_rs_end() -> HotkeyResult {
	match Mode::get() {
		ModeState::Select => match win::name()?.as_str() {
			vscode::NAME => vscode::expand_selection(),
			_ => Default
		},
		_ => Default
	}.ok()
}

fn lc_rs_home() -> HotkeyResult {
	match Mode::get() {
		ModeState::Select => Suppress,
		_ => Default
	}.ok()
}

fn lc_rs_end() -> HotkeyResult {
	match Mode::get() {
		ModeState::Select => Suppress,
		_ => Default
	}.ok()
}

fn lc_pg_up() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => Remap(Mods::LC, Key::HOME), // move_cursor_to_file_beginning
		_ => Default
	}.ok()
}

fn lc_pg_down() -> HotkeyResult {
	match Mode::get() {
		ModeState::Normal => Remap(Mods::LC, Key::END), // move_cursor_to_file_end
		_ => Default
	}.ok()
}

fn insert() -> HotkeyResult {
	match Mode::get() {
		ModeState::NSymbol => match win::name()?.as_str() {
			vscode::NAME => vscode::new_file(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn ls_enter() -> HotkeyResult {
	match win::name()?.as_str() {
		vscode::NAME => vscode::insert_line_below(),
		_ => Default
	}.ok()
}

fn lc_space() -> HotkeyResult {
	match win::name()?.as_str() {
		chrome::NAME => chrome::tabs(),
		_ => Default
	}.ok()
}

fn ls_xbutton1() -> HotkeyResult {
	I44::set_mouse_layer()?;
	Ok(Suppress)
}

fn ls_lbutton() -> HotkeyResult {
	Ok(Action(helpers::drag_win))
}

fn f3() -> HotkeyResult {
	match Mode::get() {
		ModeState::Mouse => match win::name()?.as_str() {
			vscode::NAME => vscode::stop_debugger(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn f4() -> HotkeyResult {
	match Mode::get() {
		ModeState::Mouse => match win::name()?.as_str() {
			vscode::NAME => vscode::restart_debugger(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn f6() -> HotkeyResult {
	match Mode::get() {
		ModeState::Mouse => match win::name()?.as_str() {
			vscode::NAME => vscode::step_out(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn f7() -> HotkeyResult {
	match Mode::get() {
		ModeState::Mouse => match win::name()?.as_str() {
			vscode::NAME => vscode::step_over(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn f8() -> HotkeyResult {
	match Mode::get() {
		ModeState::Mouse => match win::name()?.as_str() {
			vscode::NAME => vscode::step_into(),
			_ => Suppress
		}
		_ => Default
	}.ok()
}

fn f21() -> HotkeyResult {
	Ok(Action(helpers::drag_win))
}

fn f23() -> HotkeyResult {
	mic::tgl_mute()?;
	Ok(Suppress)
}

fn suspend() -> HotkeyResult {
	if App::suspend_togg() {
		// Mode::set_none();
		I44::disable()?;
	} else {
		// Mode::set_default();
		I44::enable()?;
	}
	Ok(Suppress)
}

fn exit() -> HotkeyResult {
	App::exit();
	Ok(Suppress)
}
