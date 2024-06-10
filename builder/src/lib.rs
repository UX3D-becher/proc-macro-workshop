use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{self, Data, DataStruct, DeriveInput, Fields};

//     Type::Path(
//         TypePath {
//             qself: None,
//             path: Path {
//                 segments: [
//                     PathSegment {
//                         ident: "Option",
//                         arguments: PathArguments::AngleBracketed(
//                             AngleBracketedGenericArguments {
//                                 args: [
//                                     GenericArgument::Type(
//                                         ...
//                                     ),
//                                 ],
//                             },
//                         ),
//                     },
//                 ],
//             },
//         },
//     )
fn is_optional_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(syn::TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(arg) = args.args.first() {
                        if let syn::GenericArgument::Type(_) = arg {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let builder_name = Ident::new(&format!("{}Builder", name), Span::call_site());

    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => {
            unimplemented!();
        }
    };

    // builder value set checks
    let builder_checks: Vec<_> = fields.iter().map(|field| {
        let ident = &field.ident;
        let ty = &field.ty;
        if is_optional_type(ty) {
            quote! {
                let #ident = self.#ident.clone();
            }
        } else {
            quote! {
                let #ident = self.#ident.clone().ok_or(concat!(stringify!(#ident), " is required"))?;
            }
        }
    }).collect();

    let gen = quote! {
        use std::error::{Error};

        pub struct #builder_name {
            executable: Option<String>,
            args: Option<Vec<String>>,
            env: Option<Vec<String>>,
            current_dir: Option<String>,
        }

        impl #builder_name {
            fn executable(&mut self, executable: String) -> &mut Self {
                self.executable = Some(executable);
                self
            }

            fn args(&mut self, args: Vec<String>) -> &mut Self {
                self.args = Some(args);
                self
            }

            fn env(&mut self, env: Vec<String>) -> &mut Self {
                self.env = Some(env);
                self
            }

            fn current_dir(&mut self, current_dir: String) -> &mut Self {
                self.current_dir = Some(current_dir);
                self
            }

            pub fn build(&mut self) -> Result<#name, Box<dyn Error>> {
                #(#builder_checks)*

                Ok(#name {
                    executable,
                    args,
                    env,
                    current_dir,
                })
            }
        }

        impl #name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    executable: None,
                    args: None,
                    env: None,
                    current_dir: None,
                }
            }

        }
    };
    gen.into()
}
