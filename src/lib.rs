extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{AngleBracketedGenericArguments, FnArg, GenericArgument, Ident, ItemFn, LitStr, Pat, PatIdent, Path, PathArguments, PathSegment, PatType, ReturnType, Signature, Type, TypePath, TypeReference, parse_macro_input};

#[proc_macro_attribute]
pub fn event_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let input = parse_macro_input!(item as ItemFn);

    // Build the trait implementation
    impl_event_handler(&input)
}

fn impl_event_handler(ast: &ItemFn) -> TokenStream {
    // println!("AST: {:?}", ast);
    let ItemFn{sig, block, ..} = ast;
    let Signature {ident, inputs, ..} = sig;

    let ident_string = ident.to_string();
    let ident_span = ident.span();
    // println!("X: {:?}: {:?}: {:?}", ident, ident_string, ident_span);
    let ident_tmp = Ident::new(&format!("{}_registry_type", ident_string), ident_span);
    let ident_helper = Ident::new(&format!("{}_helper", ident_string), ident_span);

    let mut arg_iter = inputs.iter();
    let (event_arg_name, event_type) = split_argument(arg_iter.next().unwrap(), &ident_string, "event");
    let (query_model_arg_name, query_model_type) = split_argument(arg_iter.next().unwrap(), &ident_string, "query model");

    let event_type_ident = get_type_ident(event_type, &ident_string, "event");
    // println!("Event type ident: {:?}", event_type_ident);
    let event_type_literal = LitStr::new(&event_type_ident.to_string(), event_type_ident.span());

    let gen = quote! {
        use ::dendrite::axon_utils::HandlerRegistry as #ident_tmp;

        #[tonic::async_trait]
        impl AsyncApplicableTo<#query_model_type> for #event_type {
            async fn apply_to(self: &Self, #query_model_arg_name: &mut #query_model_type) -> Result<()> {
                let #event_arg_name = self;
                debug!("Event type: {:?}", #event_type_literal);
                #block
            }

            fn box_clone(self: &Self) -> Box<dyn AsyncApplicableTo<#query_model_type>> {
                Box::from(#event_type::clone(self))
            }
        }

        // register event handler with registry
        fn #ident(registry: &mut ::dendrite::axon_utils::TheHandlerRegistry<#query_model_type,Option<#query_model_type>>) -> Result<()> {
            registry.insert(
                #event_type_literal,
                &#event_type::decode,
                &(|c,p| Box::pin(#ident_helper(Box::from(c), p)))
            )
        }

        async fn #ident_helper<T: AsyncApplicableTo<P>,P: Clone>(event: Box<T>, projection: P) -> Result<()> {
            let mut p = projection.clone();
            event.apply_to(&mut p).await?;
            Ok(())
        }
    };
    gen.into()
}

#[proc_macro_attribute]
pub fn command_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let input = parse_macro_input!(item as ItemFn);

    // Build the trait implementation
    impl_command_handler(&input)
}

fn impl_command_handler(ast: &ItemFn) -> TokenStream {
    // println!("AST: {:?}", ast);
    let ItemFn{sig, block, ..} = ast;
    let Signature {ident, inputs, output, ..} = sig;

    let ident_string = ident.to_string();
    let ident_span = ident.span();
    // println!("X: {:?}: {:?}: {:?}", ident, ident_string, ident_span);
    let ident_tmp = Ident::new(&format!("{}_registry_type", ident_string), ident_span);
    let ident_impl = Ident::new(&format!("{}_impl", ident_string), ident_span);

    let mut arg_iter = inputs.iter();
    let (command_arg_name, command_type) = split_argument(arg_iter.next().unwrap(), &ident_string, "command");
    let (context_arg_name, context_type) = split_argument(arg_iter.next().unwrap(), &ident_string, "context");
    let context_elem_type = get_elem_type_argument(context_type, &ident_string, "context");

    let command_type_ident = get_type_ident(command_type, &ident_string, "command");
    // println!("Event type ident: {:?}", event_type_ident);
    let command_type_literal = LitStr::new(&command_type_ident.to_string(), command_type_ident.span());

    let (output_type, output_type_ident) = match output {
        ReturnType::Type(_, t) => (t, get_return_type_ident(&**t, &ident_string, "result")),
        _ => panic!("Missing output type: {:?}", ident)
    };
    let output_type_literal = LitStr::new(&output_type_ident.to_string(), output_type_ident.span());

    let gen = quote! {
        use ::dendrite::axon_utils::HandlerRegistry as #ident_tmp;

        // register command handler with registry
        fn #ident(registry: &mut ::dendrite::axon_utils::TheHandlerRegistry<std::sync::Arc<async_lock::Mutex<#context_elem_type>>,::dendrite::axon_utils::SerializedObject>) -> Result<()> {
            registry.insert_with_output(
                #command_type_literal,
                &#command_type::decode,
                &(|c,p| Box::pin(#ident_impl(c, p)))
            )
        }

        async fn #ident_impl(#command_arg_name: #command_type, #context_arg_name: std::sync::Arc<async_lock::Mutex<#context_elem_type>>) -> Result<Option<SerializedObject>> {
            let mut #context_arg_name = #context_arg_name.deref().lock().await;
            debug!("Event type: {:?}", #command_type_literal);
            let result : #output_type = #block;
            let result: Option<Result<SerializedObject>> = result?.map(|r| ::dendrite::axon_utils::axon_serialize(#output_type_literal, &r));
            match result {
                Some(Ok(serialized)) => Ok(Some(serialized)),
                Some(Err(e)) => Err(e),
                None => Ok(None),
            }
        }
    };
    gen.into()
}

#[proc_macro_attribute]
pub fn event_sourcing_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let input = parse_macro_input!(item as ItemFn);

    // Build the trait implementation
    impl_event_sourcing_handler(&input)
}

fn impl_event_sourcing_handler(ast: &ItemFn) -> TokenStream {
    // println!("AST: {:?}", ast);
    let ItemFn{sig, block, ..} = ast;
    let Signature {ident, inputs, ..} = sig;

    let ident_string = ident.to_string();
    let ident_span = ident.span();
    // println!("X: {:?}: {:?}: {:?}", ident, ident_string, ident_span);
    let ident_tmp = Ident::new(&format!("{}_registry_type", ident_string), ident_span);
    let ident_applicable = Ident::new(&format!("{}_applicable_to", ident_string), ident_span);
    let ident_helper = Ident::new(&format!("{}_helper", ident_string), ident_span);

    let mut arg_iter = inputs.iter();
    let (event_arg_name, event_type) = split_argument(arg_iter.next().unwrap(), &ident_string, "event");
    let (projection_arg_name, projection_type) = split_argument(arg_iter.next().unwrap(), &ident_string, "projection");

    let event_type_ident = get_type_ident(event_type, &ident_string, "event");
    // println!("Event type ident: {:?}", event_type_ident);
    let event_type_literal = LitStr::new(&event_type_ident.to_string(), event_type_ident.span());

    let gen = quote! {
        use ::dendrite::axon_utils::HandlerRegistry as #ident_tmp;
        use ::dendrite::axon_utils::ApplicableTo as #ident_applicable;

        #[tonic::async_trait]
        impl #ident_applicable<#projection_type> for #event_type {
            fn apply_to(self: &Self, #projection_arg_name: &mut #projection_type) -> Result<()> {
                let #event_arg_name = self;
                debug!("Event type: {:?}", #event_type_literal);
                #block;
                Ok(())
            }

            fn box_clone(self: &Self) -> Box<dyn #ident_applicable<#projection_type>> {
                Box::from(#event_type::clone(self))
            }
        }

        // register event handler with registry
        fn #ident(registry: &mut ::dendrite::axon_utils::TheHandlerRegistry<#projection_type,#projection_type>) -> Result<()> {
            registry.insert_with_output(
                #event_type_literal,
                &#event_type::decode,
                &(|c,p| Box::pin(#ident_helper(Box::from(c), p)))
            )
        }

        async fn #ident_helper<T: ApplicableTo<P>,P: Clone>(event: Box<T>, projection: P) -> Result<Option<P>> {
            let mut p = projection.clone();
            event.apply_to(&mut p)?;
            Ok(Some(p))
        }
    };
    gen.into()
}

fn split_argument<'a>(argument: &'a FnArg, handler_name: &str, qualifier: &str) -> (&'a Ident, &'a Box<Type>) {
    if let FnArg::Typed(PatType {pat, ty, ..}) = argument {
        if let Pat::Ident(PatIdent {ref ident, ..}) = **pat {
            return (ident, ty);
        }
    }
    panic!("Can't parse argument: {:?}: {:?}", handler_name, qualifier)
}

fn get_elem_type_argument<'a>(argument: &'a Type, handler_name: &str, qualifier: &str) -> &'a Box<Type> {
    // println!("Get elem type of: {:?}", argument);
    if let Type::Reference(TypeReference { elem, .. }) = argument {
        return elem;
    }
    panic!("Can't get element type of reference: {:?}: {:?}", handler_name, qualifier)
}

fn get_type_ident<'a>(ty: &'a Type, handler_name: &str, qualifier: &str) -> &'a Ident {
    if let Type::Path(TypePath {path: Path {segments, ..},..}) = ty {
        let last_segment = segments.last().unwrap();
        return &last_segment.ident;
    }
    panic!("Can't get type identifier: {:?}: {:?}", handler_name, qualifier)
}

fn get_return_type_ident<'a>(ty: &'a Type, handler_name: &str, qualifier: &str) -> &'a Ident {
    let ty = get_first_generic_type_argument(ty, handler_name, qualifier);
    let ty = get_first_generic_type_argument(ty, handler_name, qualifier);
    if let Type::Path(TypePath {path:Path {segments:arg_segments,..}, ..}) = ty {
        let last_arg_segment = arg_segments.last().unwrap();
        let PathSegment { ident, ..} = last_arg_segment;
        return ident;
    }
    panic!("Can't get return type identifier: {:?}: {:?}", handler_name, qualifier)
}

fn get_first_generic_type_argument<'a>(ty: &'a Type, handler_name: &str, qualifier: &str) -> &'a Type {
    // println!("Try to get first generic type argument: {:?}", ty);
    if let Type::Path(TypePath { path: Path { segments, .. }, .. }) = ty {
        let last_segment = segments.last().unwrap();
        if let PathSegment { arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }), .. } = last_segment {
            if let Some(GenericArgument::Type(result)) = args.first() {
                return result;
            }
        }
    }
    panic!("Can't get first generic type argument: {:?}: {:?}", handler_name, qualifier)
}