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

fn strip_optional_type(ty: &syn::Type) -> syn::Type {
    if let syn::Type::Path(syn::TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(arg) = args.args.first() {
                        if let syn::GenericArgument::Type(ty) = arg {
                            return ty.clone();
                        }
                    }
                }
            }
        }
    }
    ty.clone()
}

struct BuilderAttributeOpts {
    each: Option<String>,
}

impl syn::parse::Parse for BuilderAttributeOpts {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut opts = BuilderAttributeOpts {
            each: None,
        };
        while !input.is_empty() {
            let name: Ident = input.parse()?;
            input.parse::<syn::Token![=]>()?;
            match name.to_string().as_str() {
                "each" => {
                    let value: syn::LitStr = input.parse()?;
                    opts.each = Some(value.value());
                }
                _ => {
                    return Err(syn::Error::new(name.span(), "expected `builder(each = \"...\")`"));
                }
            }
            if input.is_empty() {
                break;
            }
            input.parse::<syn::Token![,]>()?;
        }
        Ok(opts)
    }
}

#[proc_macro_derive(Builder, attributes(builder))]
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

    // builder functions
    let each_functions: Vec<_> = fields.iter().map(|field| {
        // Get the builder attribute
        let field_name = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;
        let field_ty_inner = strip_optional_type(field_ty);

        let builder_opts: Option<syn::Result<BuilderAttributeOpts>> = field.attrs.iter().filter(|attr| {
            attr.path().is_ident("builder")
        }).map(|attr| {
            attr.parse_args()
        }).next();

        if builder_opts.is_none() {
            return quote! {
                fn #field_name(&mut self, #field_name: #field_ty_inner) -> &mut Self {
                    self.#field_name = Some(#field_name);
                    self
                }
            };
        } else {
            let opts = builder_opts.unwrap();

            if opts.is_err() {
                let err = opts.err().unwrap();
                return err.to_compile_error();
            }

            let ident = field.ident.as_ref().unwrap();
            let each_fn_name = Ident::new(&opts.unwrap().each.unwrap(), Span::call_site());
            return quote! {
                fn #each_fn_name(&mut self, #ident: std::string::String) -> &mut Self {
                    if let Some(ref mut vec) = self.#ident {
                        vec.push(#ident);
                    } else {
                        self.#ident = Some(vec![#ident]);
                    }
                    self
                }
            };
        }
    }).collect();

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

    let struct_fields: Vec<_> = fields.iter().into_iter().map(|field| {
        let ident = &field.ident;
        quote! {
            #ident
        }
    }).collect();

    let gen = quote! {
        pub struct #builder_name {
            executable: std::option::Option<std::string::String>,
            args: std::option::Option<std::vec::Vec<std::string::String>>,
            env: std::option::Option<std::vec::Vec<std::string::String>>,
            current_dir: std::option::Option<std::string::String>,
        }

        impl #builder_name {

            #(#each_functions)*

            pub fn build(&mut self) -> std::result::Result<#name, std::boxed::Box<dyn std::error::Error>> {
                #(#builder_checks)*

                Ok(#name {
                    #(#struct_fields),*
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
