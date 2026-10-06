use std::{collections::{HashMap, VecDeque}, sync::Arc};
use crate::common::error::Error;
use std::borrow::Cow;
use super::InputKeys;

pub enum Hotstr {
	Default,
	Erase,
	Input { r: &'static InputKeys, clear: bool },
	Clipb { r: Cow<'static, str>, clear: bool, restore: bool },
	Action(fn() -> Result<(), Error>),
}

impl Hotstr {
	pub fn ok(self) -> Result<Self, Error> {
		Ok(self)
	}
}

pub fn input(r: &'static InputKeys) -> Hotstr {
	Hotstr::Input { r, clear: true }
}

pub fn clipb(r: impl Into<Cow<'static, str>>) -> Hotstr {
	Hotstr::Clipb { r: r.into(), clear: true, restore: true }
}

pub fn clipb_no_restore(r: impl Into<Cow<'static, str>>) -> Hotstr {
	Hotstr::Clipb { r: r.into(), clear: true, restore: false }
}

#[derive(Debug)]
pub(super) struct HotstrTrie {
	root: TrieNode,
}

impl HotstrTrie {
	pub fn new() -> Self {
		Self { root: TrieNode::new() }
	}
	
	pub fn add(&mut self, entry: &str, handler: fn() -> Result<Hotstr, Error>, exempt: bool) {
		let mut node = &mut self.root;
		
		for b in entry.bytes().rev() {
			node = node.children.entry(b).or_insert_with(|| TrieNode::new());
		}
		
		if node.is_word {
			panic!("duplicate hotstr: {entry:?}")
		}
		
		node.handler = handler;
		node.is_word = true;
		node.exempt = exempt;
		node.entry = Arc::from(entry);
	}
	
	pub fn find(&self, entry: &VecDeque<u8>) -> Option<(fn() -> Result<Hotstr, Error>, Arc<str>)> {
		let mut node = &self.root;
		let mut deepest: Option<&TrieNode> = None;
		
		for l in entry.iter().rev() {
			let Some(child) = node.children.get(l) else {
				break;
			};
			if child.is_word {
				deepest = Some(child);
			}
			node = child;
		}
		
		deepest.and_then(|node|
			(node.exempt || !super::handler::is_suspended()).then(|| (node.handler, Arc::clone(&node.entry))))
	}
}

#[derive(Debug)]
struct TrieNode {
	is_word: bool,
	exempt: bool,
	handler: fn() -> Result<Hotstr, Error>,
	children: HashMap<u8, TrieNode>,
	entry: Arc<str>,
}

impl TrieNode {
	fn new() -> Self {
		Self {
			is_word: false, children: HashMap::new(), handler: || unreachable!(), exempt: false, entry: Arc::from(""),
		}
	}
}
