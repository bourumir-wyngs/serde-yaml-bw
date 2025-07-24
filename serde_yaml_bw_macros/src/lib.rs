use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

#[proc_macro]
pub fn yaml(input: TokenStream) -> TokenStream {
    let yaml_text = parse_macro_input!(input as LitStr);
    match serde_yaml_bw::from_str::<serde_yaml_bw::Value>(&yaml_text.value()) {
        Ok(_) => {
            let tokens = quote! {
                serde_yaml_bw::from_str::<serde_yaml_bw::Value>(#yaml_text).unwrap()
            };
            tokens.into()
        }
        Err(err) => syn::Error::new_spanned(yaml_text, err).to_compile_error().into(),
    }
}
