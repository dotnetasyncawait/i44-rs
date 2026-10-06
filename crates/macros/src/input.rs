use syn::{Token, braced, parse::{Parse, ParseStream}};
use quote::{quote, ToTokens};
use proc_macro2::TokenStream as TokenStream2;

pub struct Inputs(Vec<Input>, usize, usize);

impl Parse for Inputs {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let (inputs, count1, count2) = parse_inputs_common(input)?;
		Ok(Self(inputs, count1, count2))
	}
}

impl ToTokens for Inputs {
	fn to_tokens(&self, tokens: &mut TokenStream2) {
		let inputs = &self.0;
		let count1 = self.1;
		let count2 = self.2;
		
		tokens.extend(quote! {{
			static INPUTS: ::std::sync::OnceLock<::rhk::input::InputKeys> = ::std::sync::OnceLock::new();
			INPUTS.get_or_init(|| {
				let mut vec = ::std::vec::Vec::with_capacity(#count2);
				#(#inputs)*
				::rhk::input::InputKeys(vec.into_boxed_slice(), #count1)
			})
		}});
	}
}

pub enum Input {
	Key(Key),
	Char(Char),
	Text(Text)
}

impl Parse for Input {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let input = if Text::peek(input) {
			Self::Text(input.parse()?)
		} else if Char::peek(input) {
			Self::Char(input.parse()?)
		} else {
			Self::Key(input.parse()?)
		};
		
		Ok(input)
	}
}

impl ToTokens for Input {
	fn to_tokens(&self, tokens: &mut TokenStream2) {
		match self {
			Self::Key(key) => tokens.extend(quote! { #key }),
			Self::Char(char) => tokens.extend(quote! { #char }),
			Self::Text(text) => tokens.extend(quote! { #text }),
		}
	}
}

pub struct Key(u16, Option<KeyOpt>);

impl Parse for Key {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		// Once we got here, the next token is neigher Char nor Text.
		// Therefore, we fail here if the token is not a Key.
		
		let mut s = input.parse::<syn::Ident>()?.to_string();
		s.make_ascii_uppercase();
		
		let sc = match s.as_str() {
			"ESC"   => 0x0001, "ENTER" => 0x001C, "TAB"   => 0x000F, "SPACE" => 0x0039,
			"BS"    => 0x000E, "DEL"   => 0xE053, "INS"   => 0xE052, "HOME"  => 0xE047,
			"END"   => 0xE04F, "PG_UP" => 0xE049, "PG_DN" => 0xE051,
			"UP"    => 0xE048, "DOWN"  => 0xE050, "LEFT"  => 0xE04B, "RIGHT" => 0xE04D,
			
			"LC" => 0x001D, "LS" => 0x002A, "LA" => 0x0038, "LW" => 0xE05B,
			"RC" => 0xE01D, "RS" => 0x0036, "RA" => 0xE038, "RW" => 0xE05C,
			
			// TODO: add keys
			
			_ => panic!("invalid key")
		};
		
		let opt = if Times::peek(input) {
			Some(KeyOpt::Times(input.parse()?))
		} else if Group::peek(input) {
			Some(KeyOpt::Group(input.parse()?))
		} else {
			None
		};
		
		Ok(Self(sc, opt))
	}
}

impl ToTokens for Key {
	fn to_tokens(&self, tokens: &mut TokenStream2) {
		let key = self.0;
		
		match &self.1 {
			Some(opt) => match &opt {
				KeyOpt::Times(times) => {
					let n = times.0;
					tokens.extend(quote! { vec.push(::rhk::input::InputKey::Tap(#key, #n)); });
				}
				KeyOpt::Group(group) => {
					tokens.extend(quote! { vec.push(::rhk::input::InputKey::Down(#key)); });
					group.to_tokens(tokens);
					tokens.extend(quote! { vec.push(::rhk::input::InputKey::Up(#key)); });
				}
			}
			None => tokens.extend(quote! { vec.push(::rhk::input::InputKey::Tap(#key, 1)); })
		}
	}
}

pub struct Char(u16, Option<Times>);

impl Char {
	fn peek(input: ParseStream) -> bool {
		input.peek(syn::LitChar)
	}
}

impl Parse for Char {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let sc = match input.parse::<syn::LitChar>()?.value() {
			'a' => 0x001E, 'b' => 0x0030, 'c' => 0x002E, 'd' => 0x0020,
			'e' => 0x0012, 'f' => 0x0021, 'g' => 0x0022, 'h' => 0x0023,
			'i' => 0x0017, 'j' => 0x0024, 'k' => 0x0025, 'l' => 0x0026,
			'm' => 0x0032, 'n' => 0x0031, 'o' => 0x0018, 'p' => 0x0019,
			'q' => 0x0010, 'r' => 0x0013, 's' => 0x001F, 't' => 0x0014,
			'u' => 0x0016, 'v' => 0x002F, 'w' => 0x0011, 'x' => 0x002D,
			'y' => 0x0015, 'z' => 0x002C, '1' => 0x0002, '2' => 0x0003,
			'3' => 0x0004, '4' => 0x0005, '5' => 0x0006, '6' => 0x0007,
			'7' => 0x0008, '8' => 0x0009, '9' => 0x000A, '0' => 0x000B,
			'-' => 0x000C, '=' => 0x000D, '[' => 0x001A, ']' => 0x001B,
			'\\'=> 0x002B, ';' => 0x0027, '\''=> 0x0028, '`' => 0x0029,
			',' => 0x0033, '.' => 0x0034, '/' => 0x0035,
			
			_ => panic!("invalid char")
		};
		
		let times = if Times::peek(input) {
			Some(input.parse()?)
		} else {
			None
		};
		
		Ok(Self(sc, times))
	}
}

impl ToTokens for Char {
	fn to_tokens(&self, tokens: &mut TokenStream2) {
		let key = self.0;
		let times = self.1.as_ref().unwrap_or(&Times(1)).0;
		
		tokens.extend(quote! { vec.push(::rhk::input::InputKey::Tap(#key, #times)); });
	}
}

pub struct Text(Vec<u16>, usize);

impl Text {
	fn peek(input: ParseStream) -> bool {
		input.peek(syn::LitStr)
	}
}

impl Parse for Text {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let s = input.parse::<syn::LitStr>()?.value();
		assert!(s.len() != 0, "empty string literal");
		
		let mut count = 0usize;
		
		let mut iter = s.encode_utf16();
		let mut res = Vec::<u16>::new(); // TODO: use size_hint
		
		while let Some(high) = iter.next() {
			res.push(high);
			if high >= 0xD800 {
				let low = iter.next().expect("should be valid surrogate pair");
				res.push(low);
			}
			// We only increment once because surrogate pairs are combined into a single InputKey.
			count += 1; 
		}
		
		Ok(Self(res, count))
	}
}

impl ToTokens for Text {
	fn to_tokens(&self, tokens: &mut TokenStream2) {
		let mut iter = self.0.iter(); // UTF-16 codepoints
		
		while let Some(&high) = iter.next() {
			if high < 0xD800 {
				tokens.extend(quote! { vec.push(::rhk::input::InputKey::Unicode(#high, 0)); });
			} else {
				let low = *iter.next().expect("should be valid surrogate pair");
				tokens.extend(quote! { vec.push(::rhk::input::InputKey::Unicode(#high, #low)); });
			}
		}
	}
}

enum KeyOpt {
	Times(Times),
	Group(Group),
}

struct Group(Vec<Input>, usize, usize);

impl Group {
	fn peek(input: ParseStream) -> bool {
		input.peek(syn::token::Brace)
	}
}

impl Parse for Group {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let content;
		braced!(content in input);
		
		let (inputs, count1, count2) = parse_inputs_common(&content)?;
		
		// count1 + 2 for the outer key (ie, <KEY>{...})
		// count2 + 2 for DOWN at the beginning of the group and UP afterwards
		Ok(Self(inputs, count1 + 2, count2 + 2))
	}
}

impl ToTokens for Group {
	fn to_tokens(&self, tokens: &mut TokenStream2) {
		for input in &self.0 {
			input.to_tokens(tokens);
		}
	}
}

struct Times(u16);

impl Times {
	fn peek(input: ParseStream) -> bool {
		input.peek(Token![:])
	}
}

impl Parse for Times {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		input.parse::<Token![:]>()?;
		let num = input.parse::<syn::LitInt>()?.base10_parse()?;
		assert!(num > 0, "Times must be > 0");
		Ok(Self(num))
	}
}

fn parse_inputs_common(input: ParseStream) -> syn::Result<(Vec<Input>, usize, usize)> {
	let mut inputs = Vec::new();
	let mut count1 = 0usize;
	let mut count2 = 0usize;
	
	while !input.is_empty() {
		let inp = input.parse::<Input>()?;
		
		count1 += match &inp {
			Input::Key(key) => match &key.1 {
				Some(opt) => match &opt {
					KeyOpt::Times(times) => times.0 as usize * 2,
					KeyOpt::Group(group) => {
						// -1 because of the `count2 += 1;` at the end of the iteration
						count2 += group.2 - 1;
						group.1
					}
				}
				None => 2
			}
			Input::Char(char) => match &char.1 {
				Some(times) => times.0 as usize * 2,
				None => 2
			}
			Input::Text(text) => {
				// -1 because of the `count2 += 1;` at the end of the iteration
				count2 += text.1 - 1;
				text.0.len() * 2
			}
		};
		
		count2 += 1;
		inputs.push(inp);
	}
	
	Ok((inputs, count1, count2))
}
