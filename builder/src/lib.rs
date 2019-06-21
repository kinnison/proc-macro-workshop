extern crate proc_macro;

use proc_macro::TokenStream;

use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, Data, DeriveInput, Field, Fields, GenericArgument, Ident, Lit, Meta,
    NestedMeta, PathArguments, Type,
};

fn simple_inner_type<'a>(ty: &'a Type, wrapper: &'static str) -> Option<&'a Type> {
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
    if &seg.ident.to_string() != wrapper {
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
    simple_inner_type(ty, "Option").is_some()
}

fn optional_type(ty: &Type) -> &Type {
    simple_inner_type(ty, "Option").expect("Expected optional field")
}

fn builder_error<T: ToTokens>(att: T) -> TokenStream {
    syn::Error::new_spanned(att, "expected `builder(each = \"...\")`")
        .to_compile_error()
        .into()
}

fn get_builder_name(f: &Field) -> Result<Option<(Ident, &Type)>, TokenStream> {
    for att in f.attrs.iter() {
        if att.path.is_ident("builder") {
            let ml = match att.parse_meta().unwrap() {
                Meta::List(l) => l,
                _ => Err(builder_error(att))?,
            };
            if ml.nested.len() != 1 {
                Err(builder_error(att))?;
            }
            let iatt = &ml.nested[0];
            let nv = match iatt {
                NestedMeta::Meta(Meta::NameValue(nv)) => nv,
                _ => Err(builder_error(&ml))?,
            };
            if nv.ident != "each" {
                Err(builder_error(&ml))?;
            }
            let s = match &nv.lit {
                Lit::Str(s) => s,
                _ => Err(builder_error(&ml))?,
            };
            let ident = Ident::new(&s.value(), s.span());
            let inner = simple_inner_type(&f.ty, "Vec").expect("Expected Vec<T> in each=\"name\"");
            return Ok(Some((ident, inner)));
        }
    }
    Ok(None)
}

fn field_is_builder_vec(f: &Field) -> bool {
    match get_builder_name(f) {
        Ok(Some(_)) => true,
        _ => false,
    }
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive_builder(input) {
        Ok(s) => s,
        Err(s) => s,
    }
}

fn derive_builder(input: DeriveInput) -> Result<TokenStream, TokenStream> {

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
                #id : std::option::Option<#ty>
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
        if field_is_builder_vec(&f) {
            quote! {
                #id : std::option::Option::Some(std::vec::Vec::new())
            }
        } else {
            quote! {
                #id : std::option::Option::None
            }
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

    let builder_methods: Result<Vec<_>, TokenStream> = fields
        .named
        .iter()
        .map(|f| {
            let id = f.ident.as_ref().unwrap();
            let ty = &f.ty;
            if field_is_optional(&ty) {
                let ty = optional_type(&ty);
                Ok(quote! {
                    fn #id ( &mut self, #id : #ty ) -> &mut Self {
                        self.#id = std::option::Option::Some(#id);
                        self
                    }

                })
            } else {
                // Non-optional field, so process attribute
                let mut main = quote! {
                        fn #id ( &mut self, #id : #ty ) -> &mut Self {
                            self.#id = std::option::Option::Some(#id);
                            self
                        }
                };

                if let Some((bname, btype)) = get_builder_name(f)? {
                    if id == &bname {
                        main = quote! {};
                    }
                    Ok(quote! {
                        fn #bname ( &mut self, #bname: #btype) -> &mut Self {
                            let mref = self.#id.as_mut().unwrap();
                            mref.push(#bname);
                            self
                        }
                        #main
                    })
                } else {
                    Ok(main)
                }
            }
        })
        .collect();
    let builder_methods = builder_methods?;

    let build_method_fields = fields.named.iter().map(|f| {
        let id = f.ident.as_ref().unwrap();
        let id_str = id.to_string();
        let ty = &f.ty;
        if field_is_optional(&ty) {
            quote! {
                #id : self.#id.as_ref().map(|f| f.clone())
            }
        } else if field_is_builder_vec(&f) {
            quote! {
                #id : self.#id.as_ref().unwrap().clone()
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
        fn build(&mut self) -> std::result::Result<#struct_name, std::boxed::Box<dyn std::error::Error>> {
            std::result::Result::Ok(#struct_name {
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

    Ok((quote! {
        #builder_struct
        #builder_impl
        #builder_struct_impl
    })
    .into())
}
