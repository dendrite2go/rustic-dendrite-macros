extern crate proc_macro;

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
    println!("AST: {:?}", ast);
    let ItemFn{vis, sig, block, ..} = ast;
    let Signature {ident, inputs, output, ..} = sig;
    let ident_string = ident.to_string();
    let ident_span = ident.span();
    println!("X: {:?}: {:?}: {:?}", ident, ident_string, ident_span);
    let ident_helper = Ident::new(&format!("{}_helper", ident_string), ident_span);
    let ident_impl = Ident::new(&format!("{}_impl", ident_string), ident_span);
    let mut arg_iter = inputs.iter();
    let (event_arg_name, event_type) = split_argument(arg_iter.next().unwrap());
    let (query_model_arg_name, _query_model_type) = split_argument(arg_iter.next().unwrap());
    let gen = quote! {
        #vis async fn test(#inputs) #output #block

        async fn #ident_impl(#query_model_arg_name: #_query_model_type, #event_arg_name: #event_type) -> () {
            test(#event_arg_name, #query_model_arg_name).await
        }

        async fn #ident_helper<T: AsyncApplicableTo<P>,P: Clone>(event: Box<T>, projection: P) -> Result<()> {
            let mut p = projection.clone();
            event.apply_to(&mut p).await?;
            Ok(())
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
