use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(Data)]
pub fn data(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;

    let generics = &input.generics;
    let Data::Struct(input_struct) = input.data else {
        todo!()
    };

    let checks = input_struct.fields.iter().map(|field| {
        let field_ident = field.ident.as_ref().unwrap();
        let check_ident = format_ident!("check_{}", field_ident);
        quote! {
           fn #check_ident(t: #ident #generics) {
                use actuate::{DataField, StateField};

                (&t.#field_ident).check()
           }
        }
    });

    let gen = quote! {
        #(#checks)*

        unsafe impl #generics actuate::Data for #ident #generics {}
    };
    gen.into()
}
