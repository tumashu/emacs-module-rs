use darling::{self, FromMeta};
use quote::quote;
use syn::{export::TokenStream2, AttributeArgs, ItemFn};

use crate::util;

#[derive(Debug)]
enum Name {
    /// Use crate's name.
    Crate,
    /// Use initializer function's name.
    Fn,
    /// Explicitly specify a name.
    Str(String),
}

#[derive(Debug, FromMeta)]
struct ModuleOpts {
    /// Name of this module (feature's name).
    #[darling(default)]
    name: Name,
    /// Separator following the feature's name to form the prefix in functions' full Lisp name.
    #[darling(default = "default::separator")]
    separator: String,
    /// Whether module path should be used to construct functions' full Lisp name.
    #[darling(default = "default::mod_in_name")]
    mod_in_name: bool,
}

#[derive(Debug)]
pub struct Module {
    def: ItemFn,
    opts: ModuleOpts,
}

mod default {
    pub fn separator() -> String {
        "-".into()
    }
    pub fn mod_in_name() -> bool {
        true
    }
    impl Default for super::Name {
        fn default() -> Self {
            super::Name::Crate
        }
    }
}

/// We don't use the derived impl provided by darling, since we want a different syntax.
/// See https://github.com/TedDriggs/darling/issues/74.
impl FromMeta for Name {
    fn from_list(outer: &[syn::NestedMeta]) -> darling::Result<Name> {
        match outer.len() {
            0 => Err(darling::Error::too_few_items(1)),
            1 => {
                let elem = &outer[0];
                match elem {
                    syn::NestedMeta::Meta(syn::Meta::Word(ref ident)) => {
                        match ident.to_string().as_ref() {
                            "fn" => Ok(Name::Fn),
                            "crate" => Ok(Name::Crate),
                            _ => Err(darling::Error::custom("Expected crate/fn").with_span(ident)),
                        }
                    }
                    syn::NestedMeta::Literal(syn::Lit::Str(ref lit)) => Ok(Name::Str(lit.value())),
                    _ => {
                        Err(darling::Error::custom("Expected crate/fn or a string").with_span(elem))
                    }
                }
            }
            _ => Err(darling::Error::too_many_items(1)),
        }
    }

    fn from_string(lit: &str) -> darling::Result<Name> {
        Ok(Name::Str(lit.to_owned()))
    }
}

impl Module {
    pub fn parse(attr_args: AttributeArgs, fn_item: ItemFn) -> Result<Self, TokenStream2> {
        let opts: ModuleOpts = match ModuleOpts::from_list(&attr_args) {
            Ok(v) => v,
            Err(e) => return Err(e.write_errors()),
        };
        Ok(Self { opts, def: fn_item })
    }

    pub fn render(&self) -> TokenStream2 {
        let define_init = self.gen_init();
        let register_init = Self::gen_registrator();
        let define_hook = &self.def;
        quote! {
            #define_hook
            #define_init
            #register_init
        }
    }

    pub fn gen_registrator() -> TokenStream2 {
        let init = Self::init_ident();
        quote! {
            ::emacs::module_init!(#init);
        }
    }

    pub fn gen_init(&self) -> TokenStream2 {
        let env = quote!(env);
        let init = Self::init_ident();
        let feature = quote!(feature);
        let separator = &self.opts.separator;
        let hook = &self.def.ident;
        let init_fns = util::init_fns_path();
        let prefix = util::prefix_path();
        let mod_in_name = util::mod_in_name_path();
        let crate_mod_in_name = &self.opts.mod_in_name;
        let set_feature = match &self.opts.name {
            Name::Crate => quote! {
                let #feature = ::emacs::globals::lisp_pkg(module_path!());
            },
            Name::Fn => {
                let name = util::lisp_name(hook);
                quote! {
                    let #feature = #name.to_owned();
                }
            }
            Name::Str(name) => quote! {
                let #feature = #name.to_owned();
            },
        };
        let set_prefix = quote! {
            {
                let mut prefix = #prefix.try_lock()
                    .expect("Failed to acquire write lock on module prefix");
                *prefix = [#feature.clone(), #separator.to_owned()];
            }
        };
        let configure_mod_in_name = quote! {
            #mod_in_name.store(#crate_mod_in_name, ::std::sync::atomic::Ordering::Relaxed);
        };
        let export_lisp_funcs = quote! {
            {
                let funcs = #init_fns.try_lock()
                    .expect("Failed to acquire a read lock on map of initializers");
                for (name, func) in funcs.iter() {
                    func(#env)?
                }
            }
        };
        quote! {
            #[allow(non_snake_case)]
            fn #init(#env: &::emacs::Env) -> ::emacs::Result<::emacs::Value<'_>> {
                #set_feature
                #set_prefix
                #configure_mod_in_name
                #export_lisp_funcs
                #hook(#env)?;
                #env.provide(&#feature)
            }
        }
    }

    fn init_ident() -> TokenStream2 {
        quote!(__emrs_auto_init__)
    }
}
