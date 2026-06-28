use proc_macro::TokenStream;
#[cfg(feature = "runtime")]
use quote::quote;
#[cfg(feature = "runtime")]
use syn::{ItemFn, parse_macro_input, parse_quote};

#[proc_macro_attribute]
pub fn instrument(_attr: TokenStream, item: TokenStream) -> TokenStream {
    instrument_impl(item)
}

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    main_impl(item)
}

#[cfg(feature = "runtime")]
fn instrument_impl(item: TokenStream) -> TokenStream {
    let mut function = parse_macro_input!(item as ItemFn);
    let name = function.sig.ident.to_string();
    let body = function.block;
    function.block = parse_quote!({
        let _linkscope_span = ::linkscope::phase(#name);
        let _linkscope_trace = ::linkscope::trace(#name);
        #body
    });
    quote!(#function).into()
}

#[cfg(not(feature = "runtime"))]
fn instrument_impl(item: TokenStream) -> TokenStream {
    item
}

#[cfg(feature = "runtime")]
fn main_impl(item: TokenStream) -> TokenStream {
    let mut function = parse_macro_input!(item as ItemFn);
    let body = function.block;
    function.block = parse_quote!({
        ::linkscope::trace_enable();
        let _linkscope_report = ::linkscope::ReportGuard::new();
        #body
    });
    quote!(#function).into()
}

#[cfg(not(feature = "runtime"))]
fn main_impl(item: TokenStream) -> TokenStream {
    item
}
