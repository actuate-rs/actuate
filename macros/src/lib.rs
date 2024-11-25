use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, token::Comma, Data, DeriveInput,
    GenericParam, ItemTrait, TypeParamBound,
};

#[proc_macro_derive(Data)]
pub fn derive_data(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;

    let generics = &input.generics;

    let generic_params: Punctuated<_, Comma> = generics
        .params
        .iter()
        .map(|param| match param {
            GenericParam::Lifetime(lifetime_param) => lifetime_param.to_token_stream(),
            GenericParam::Type(type_param) => {
                let ident = &type_param.ident;

                let mut bounds = type_param.bounds.clone();
                bounds.push(parse_quote!(Data));

                quote! {
                    #ident: #bounds
                }
            }
            GenericParam::Const(const_param) => const_param.to_token_stream(),
        })
        .collect();

    let generic_ty_params: Punctuated<_, Comma> = generics
        .params
        .iter()
        .map(|param| match param {
            GenericParam::Lifetime(lifetime_param) => lifetime_param.to_token_stream(),
            GenericParam::Type(type_param) => type_param.ident.to_token_stream(),
            GenericParam::Const(const_param) => const_param.to_token_stream(),
        })
        .collect();

    let Data::Struct(input_struct) = input.data else {
        todo!()
    };

    let checks = input_struct.fields.iter().map(|field| {
        let field_ident = field.ident.as_ref().unwrap();
        let check_ident = format_ident!("__check_{}_{}", ident, field_ident);
        quote! {
           #[doc(hidden)]
           #[allow(non_snake_case)]
           fn #check_ident <#generic_params> (t: #ident <#generic_ty_params>) {
                (&&FieldWrap(t.#field_ident)).check()
           }
        }
    });

    let gen = quote! {
        #( #checks )*

        #[doc(hidden)]
        unsafe impl <#generic_params> Data for #ident <#generic_ty_params> {}
    };
    gen.into()
}

#[proc_macro_attribute]
pub fn data(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemTrait);

    let contains_data = item.supertraits.iter().any(|x| {
        if let TypeParamBound::Trait(trait_bound) = x {
            if trait_bound.path.is_ident("Data") {
                return true;
            }
        }

        false
    });

    if !contains_data {
        return quote! {
            compile_error!("\
                Traits used as `Data` must require all implementations to be `Data`. \
                To fix this, add `Data` as a supertrait to your trait (i.e trait MyTrait: Data {}).\
            ");
        }
        .into();
    }

    let ident = &item.ident;

    quote! {
        #item

        unsafe impl Data for Box<dyn #ident + '_> {}
    }
    .into()
}
