use proc_macro::TokenStream;
use quote::quote;
use syn::{self, DeriveInput};
use proc_macro2::{Ident, Span};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let builder_name = Ident::new(&format!("{}Builder", name), Span::call_site());

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
                    let executable = self.executable.clone().ok_or("executable is required")?;
                    let args = self.args.clone().ok_or("args is required")?;
                    let env = self.env.clone().ok_or("env is required")?;
                    let current_dir = self.current_dir.clone().ok_or("current_dir is required")?;

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
