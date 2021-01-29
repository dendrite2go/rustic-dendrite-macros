extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, Ident, ItemFn, LitStr, Pat, PatIdent, Path, PatType, Signature, Type, TypePath, parse_macro_input};

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
    let ItemFn{sig, block, ..} = ast;
    let Signature {ident, inputs, output, ..} = sig;

    let ident_string = ident.to_string();
    let ident_span = ident.span();
    println!("X: {:?}: {:?}: {:?}", ident, ident_string, ident_span);
    let ident_helper = Ident::new(&format!("{}_helper", ident_string), ident_span);

    let mut arg_iter = inputs.iter();
    let (event_arg_name, event_type) = split_argument(arg_iter.next().unwrap());
    let (query_model_arg_name, query_model_type) = split_argument(arg_iter.next().unwrap());

    let event_type_ident = get_type_ident(event_type);
    println!("Event type ident: {:?}", event_type_ident);
    let event_type_literal = LitStr::new(&event_type_ident.to_string(), event_type_ident.span());

    let gen = quote! {
        #[tonic::async_trait]
        impl AsyncApplicableTo<#query_model_type> for #event_type {
            async fn apply_to(self: &Self, #query_model_arg_name: &mut #query_model_type) -> Result<()> {
                let #event_arg_name = self;
                debug!("Event type: {:?}", #event_type_literal);
                #block
            }
        }

        async fn #ident(#inputs) #output #block

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

fn get_type_ident(ty: &Type) -> &Ident {
    if let Type::Path(TypePath {path: Path {segments, ..},..}) = ty {
        let last_segment = segments.last().unwrap();
        return &last_segment.ident;
    }
    panic!()
}
