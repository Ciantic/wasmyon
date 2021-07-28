use proc_macro::*;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{parse_macro_input, parse_quote, token::RArrow, Attribute, ItemFn, ReturnType, Type};

#[proc_macro_attribute]
pub fn promise(attr: TokenStream, item: TokenStream) -> TokenStream {
    // println!("attr: \"{}\"", attr.to_string());
    // println!("item: \"{}\"", item.to_string());
    let original_fn: TokenStream2 = item.clone().into();

    // Parse the input function, get body and name
    let mut input_fn: ItemFn = parse_macro_input!(item as ItemFn);
    let input_fn_body: TokenStream2 = input_fn.block.to_token_stream();
    let input_fn_name = format!("{}", input_fn.sig.ident);

    // Set new name and body, return type is js_sys::Promise
    input_fn.sig.ident = Ident::new(&("__wasm_".to_owned() + &input_fn_name), Span::call_site());
    input_fn.block = Box::new(parse_quote!({
        wasmyon::run_in_worker(move || #input_fn_body)
    }));
    input_fn.sig.output = ReturnType::Type(
        RArrow::default(),
        Box::new(Type::Verbatim(quote!(js_sys::Promise))),
    );

    // TODO: How do we change TypeScript output type to Promise<ORIGINALTYPE>?
    let attrs: TokenStream2 = parse_quote!(
        #[wasm_bindgen(js_name = #input_fn_name)]
    );

    TokenStream::from(quote! (
        #original_fn

        #attrs
        #input_fn
    ))
}
