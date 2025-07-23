use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

#[proc_macro]
pub fn yaml(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let tokens = quote! {
        serde_yaml_bw::from_str::<serde_yaml_bw::Value>(#lit)
            .expect("failed to parse YAML at runtime")
    };
    tokens.into()
}
