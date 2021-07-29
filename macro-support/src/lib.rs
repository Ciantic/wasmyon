use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{
    parse::Parse, parse_macro_input, parse_quote, token::RArrow, Attribute, ItemFn, Meta,
    NestedMeta, ReturnType, Type,
};

/// Runs the function in a WASM worker, returning a Promise
///
/// This moves the attribute params to wasm_bindgen. This means it's possible to
/// call this like: `#[wasmyon_promise(js_name = Blah)]` or
/// `#[wasmyon_promise(skip_typescript)]`
///
/// This retains the original function, and creates a new function for the wasm
/// named `__wasm_ORIGINAL_FUNCTION`.
#[proc_macro_attribute]
pub fn wasmyon_promise(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("{}", attr.to_string());
    println!("{}", item.to_string());
    let args: Vec<syn::NestedMeta> = parse_macro_input!(attr as Vec<syn::NestedMeta>);
    // let args_tokens = args.iter().filter_map(|f| {
    //     match f {
    //         NestedMeta::Meta(Meta::Path(v)) => {
    //             todo!()
    //         },
    //         _ => todo!()
    //     }
    //     // if format!("{}", f) == "foo" {
    //     //     None
    //     // } else {
    //     //     Some(f)
    //     // }
    // }
    //     // NestedMeta::Meta(v) if f.to_string() == "serde" => Some(f),
    //     // _ => ,
    // );
    // let foo = quote::quote!(#(#args_tokens),*);

    // let mut wasm_bindgen_params = TokenStream2::from(attr);
    let mut wasm_bindgen_params = TokenStream2::new();
    let item_original = item.clone();
    let original_fn: ItemFn = parse_macro_input!(item_original as ItemFn);

    // TODO: From parameters, parse "serde" or "serde_wasm_bindgen"
    // Attribute::parse_meta(&self)

    // Parse the input function, get body and name
    let mut wasm_fn: ItemFn = parse_macro_input!(item as ItemFn);
    let wasm_fn_body: TokenStream2 = wasm_fn.block.to_token_stream();
    let wasm_fn_name = format!("{}", wasm_fn.sig.ident);
    if !wasm_bindgen_params.to_string().contains("js_name") {
        if wasm_bindgen_params.is_empty() {
            wasm_bindgen_params = parse_quote!(js_name = #wasm_fn_name);
        } else {
            wasm_bindgen_params = parse_quote!(#wasm_bindgen_params, js_name = #wasm_fn_name);
        }
    };

    // Set new name and body, return type is js_sys::Promise
    wasm_fn.sig.ident = Ident::new(&("__wasm_".to_owned() + &wasm_fn_name), Span::call_site());
    wasm_fn.block = Box::new(parse_quote!({
        wasmyon::run_in_worker_as_promise(move || #wasm_fn_body)
    }));
    wasm_fn.sig.output = ReturnType::Type(
        RArrow::default(),
        Box::new(Type::Verbatim(quote!(js_sys::Promise))),
    );

    // TODO: How do we change TypeScript output type to Promise<ORIGINALTYPE>?
    let attrs: TokenStream2 = parse_quote!(
        #[wasm_bindgen::prelude::wasm_bindgen(#wasm_bindgen_params)]
    );

    TokenStream::from(quote! (
        #original_fn

        #attrs
        #wasm_fn
    ))
}
