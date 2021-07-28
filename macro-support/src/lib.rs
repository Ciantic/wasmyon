use proc_macro::*;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{parse_macro_input, parse_quote, token::RArrow, Attribute, ItemFn, ReturnType, Type};

/// Runs the function in a WASM worker, returning a Promise
///
/// This also retains the original function, and creates a new function for the
/// wasm named `__wasm_ORIGINAL_FUNCTION`.
#[proc_macro_attribute]
pub fn promise(attr: TokenStream, item: TokenStream) -> TokenStream {
    // println!("attr: \"{}\"", attr.to_string());
    // println!("item: \"{}\"", item.to_string());
    let original_fn: TokenStream2 = item.clone().into();

    // Parse the input function, get body and name
    let mut wasm_fn: ItemFn = parse_macro_input!(item as ItemFn);
    let wasm_fn_body: TokenStream2 = wasm_fn.block.to_token_stream();
    let wasm_fn_name = format!("{}", wasm_fn.sig.ident);

    // Set new name and body, return type is js_sys::Promise
    wasm_fn.sig.ident = Ident::new(&("__wasm_".to_owned() + &wasm_fn_name), Span::call_site());
    wasm_fn.block = Box::new(parse_quote!({
        wasmyon::run_in_worker(move || #wasm_fn_body)
    }));
    wasm_fn.sig.output = ReturnType::Type(
        RArrow::default(),
        Box::new(Type::Verbatim(quote!(js_sys::Promise))),
    );

    // TODO: How do we change TypeScript output type to Promise<ORIGINALTYPE>?
    let attrs: TokenStream2 = parse_quote!(
        #[wasm_bindgen(js_name = #wasm_fn_name)]
    );

    TokenStream::from(quote! (
        #original_fn

        #attrs
        #wasm_fn
    ))
}
