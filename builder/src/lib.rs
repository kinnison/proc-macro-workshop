extern crate proc_macro;

use proc_macro::TokenStream;

use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Fields, GenericArgument, Ident, PathArguments, Type,
};

fn optional_field(ty: &Type) -> Option<&Type> {
    let path = match ty {
        Type::Path(path) => path,
        _ => return None,
    };
    if path.qself.is_some() {
        return None;
    }
    if path.path.segments.len() != 1 {
        return None;
    }
    let seg = &path.path.segments[0];
    if &seg.ident.to_string() != "Option" {
        return None;
    }
    let angles = match &seg.arguments {
        PathArguments::AngleBracketed(e) => e,
        _ => return None,
    };
    if angles.args.len() != 1 {
        return None;
    }
    let arg = &angles.args[0];
    let ty = match arg {
        GenericArgument::Type(ty) => ty,
        _ => return None,
    };
    Some(ty)
}

fn field_is_optional(ty: &Type) -> bool {
    optional_field(ty).is_some()
}

fn optional_type(ty: &Type) -> &Type {
    optional_field(ty).expect("Expected optional field")
}

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;

    let builder_struct_name = Ident::new(&format!("{}Builder", struct_name), struct_name.span());

    let ds = match input.data {
        Data::Struct(ds) => ds,
        _ => panic!("Unable to derive Builder unless it's a struct!"),
    };

    let fields = match ds.fields {
        Fields::Named(nf) => nf,
        _ => panic!("Unable to derive Builder unless it's a named field struct"),
    };

    let bits = fields.named.iter().map(|f| {
        let id = &f.ident;
        let ty = &f.ty;
        if field_is_optional(&ty) {
            quote! {
                #id: #ty
            }
        } else {
            quote! {
                #id : Option<#ty>
            }
        }
    });

    let builder_struct = quote! {
        pub struct #builder_struct_name {
            #(#bits),*
        }
    };

    let inits = fields.named.iter().map(|f| {
        let id = &f.ident;
        quote! {
            #id : None
        }
    });

    let builder_impl = quote! {
        impl #struct_name {
            pub fn builder() -> #builder_struct_name {
                #builder_struct_name {
                    #(#inits),*
                }
            }
        }
    };

    let builder_methods = fields.named.iter().map(|f| {
        let id = &f.ident;
        let ty = &f.ty;
        if field_is_optional(&ty) {
            let ty = optional_type(&ty);
            quote! {
                fn #id ( &mut self, #id : #ty ) -> &mut Self {
                    self.#id = Some(#id);
                    self
                }
            }
        } else {
            quote! {
                    fn #id ( &mut self, #id : #ty ) -> &mut Self {
                        self.#id = Some(#id);
                        self
                    }
            }
        }
    });

    let build_method_fields = fields.named.iter().map(|f| {
        let id = f.ident.as_ref().unwrap();
        let id_str = id.to_string();
        let ty = &f.ty;
        if field_is_optional(&ty) {
            quote! {
                #id : self.#id.as_ref().map(|f| f.clone())
            }
        } else {
            quote! {
                #id : self.#id
                          .as_ref()
                          .map(|f| f.clone())
                          .ok_or_else(|| concat!("Missing field ", #id_str).to_owned())?
            }
        }
    });

    let build_method = quote! {
        fn build(&mut self) -> Result<#struct_name, Box<dyn std::error::Error>> {
            Ok(#struct_name {
                #(#build_method_fields),*
            })
        }
    };

    let builder_struct_impl = quote! {
        impl #builder_struct_name {
            #(#builder_methods)*
            #build_method
        }
    };

    (quote! {
        #builder_struct
        #builder_impl
        #builder_struct_impl
    })
    .into()
}
