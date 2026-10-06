use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

mod input;

#[proc_macro]
pub fn make_input(input: TokenStream) -> TokenStream {
	parse_macro_input!(input as input::Inputs).to_token_stream().into()
}
