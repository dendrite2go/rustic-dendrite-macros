extern crate proc_macro;

use log::debug;
use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, Ident, ItemFn, Pat, PatIdent, PatType, Signature, Type, parse_macro_input};

#[proc_macro_attribute]
pub fn event_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let input = parse_macro_input!(item as ItemFn);

    // Build the trait implementation
    impl_event_handler(&input)
}

fn impl_event_handler(ast: &ItemFn) -> TokenStream {
    debug!("AST: {:?}", ast);
    let ItemFn{sig: Signature {ident, inputs, ..}, ..} = ast;
    debug!("X: {:?}", ident);
    let mut arg_iter = inputs.iter();
    let (event_arg_name, event_type) = split_argument(arg_iter.next().unwrap());
    let (query_model_arg_name, _query_model_type) = split_argument(arg_iter.next().unwrap());
    let gen = quote! {
        #ast
        fn test(#query_model_arg_name: #_query_model_type, #event_arg_name: #event_type) -> () {
            #ident(#event_arg_name, #query_model_arg_name)
        }
    };
    gen.into()
}

fn split_argument(argument: &FnArg) -> (&Ident, &Box<Type>) {
    if let FnArg::Typed(PatType {pat, ty, ..}) = argument {
        if let Pat::Ident(PatIdent {ref ident, ..}) = **pat {
            return (ident, ty);
        }
    }
    panic!()
}
