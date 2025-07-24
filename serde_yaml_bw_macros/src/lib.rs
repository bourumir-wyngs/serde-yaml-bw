use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

#[proc_macro]
pub fn yaml(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    match serde_yaml_bw::from_str::<serde_yaml_bw::Value>(&lit.value()) {
        Ok(_) => {
            let tokens = quote! {
                serde_yaml_bw::from_str::<serde_yaml_bw::Value>(#lit).unwrap()
            };
            tokens.into()
        }
        Err(err) => syn::Error::new_spanned(lit, err).to_compile_error().into(),
    }
}
