extern crate proc_macro2;
extern crate quote;
extern crate syn;

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::parse_macro_input;
use syn::spanned::Spanned;

struct Variant<'a> {
    ident: syn::Ident,
    fields: &'a syn::Fields,
}

impl<'a> Variant<'a> {
    pub fn try_from_ast(variant: &'a syn::Variant) -> syn::Result<Self> {
        if variant
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("serde"))
        {
            return Err(syn::Error::new(
                variant.span(),
                "UntaggedEnumDeserialize: #[serde(..)] attributes on variants are not supported",
            ));
        }

        Ok(Variant {
            ident: variant.ident.clone(),
            fields: &variant.fields,
        })
    }

    fn gen_type_name(&self) -> syn::Result<proc_macro2::TokenStream> {
        match self.fields {
            syn::Fields::Unit => Ok(quote! { <()> }),
            syn::Fields::Unnamed(fields) => {
                if fields.unnamed.len() == 1 {
                    // If there's only one unnamed field, we can use its type directly
                    let ty = &fields.unnamed[0].ty;
                    Ok(quote! { <#ty> })
                } else {
                    // If there are multiple unnamed fields, we create a tuple type
                    let types = fields
                        .unnamed
                        .iter()
                        .map(|f| f.ty.clone())
                        .collect::<Vec<_>>();
                    Ok(quote! { <(#(#types),*)> })
                }
            }
            syn::Fields::Named(_) => Err(syn::Error::new(
                self.ident.span(),
                "UntaggedEnumDeserialize: inlined struct variants are not supported -- use a named struct type instead",
            )),
        }
    }

    fn gen_constructor(&self) -> syn::Result<proc_macro2::TokenStream> {
        let enum_name = &self.ident;
        match self.fields {
            syn::Fields::Unit => Ok(quote! { #enum_name }),
            syn::Fields::Unnamed(fields) => {
                if fields.unnamed.len() == 1 {
                    Ok(quote! { #enum_name(__inner) })
                } else {
                    let elems = (0..fields.unnamed.len())
                        .map(|i| {
                            let i = syn::Index::from(i);
                            quote! { __inner.#i }
                        })
                        .collect::<Vec<proc_macro2::TokenStream>>();
                    Ok(quote! { #enum_name(#(#elems),*) })
                }
            }
            syn::Fields::Named(_) => Err(syn::Error::new(
                self.ident.span(),
                "UntaggedEnumDeserialize: inlined struct variants are not supported -- use a named struct type instead",
            )),
        }
    }

    fn gen_deserialize_block(&self) -> syn::Result<proc_macro2::TokenStream> {
        let type_name = self.gen_type_name()?;

        let block = quote! {
            __unused_keys.clear();
            let __inner = {
                let mut collect_unused_keys: __serde_yaml::value::UnusedKeyCallback  = Box::new(|path: __serde_yaml::Path<'_>, key: &__serde_yaml::Value, value: &__serde_yaml::Value| {
                    __unused_keys.push((path.to_owned_path(), key.clone(), value.clone()));
                });

                #type_name::deserialize(__state.get_deserializer(Some(&mut collect_unused_keys)))
            };
        };

        Ok(block)
    }

    fn gen_constructor_block(
        &self,
        enum_name: &syn::Ident,
    ) -> syn::Result<proc_macro2::TokenStream> {
        let constructor = self.gen_constructor()?;

        let block = quote! {
            if let Ok(__inner) = __inner {
                if let Some(mut __callback) = __unused_key_callback {
                    for (path, key, value) in __unused_keys.iter() {
                        __callback(*path.as_path(), key, value);
                    }
                }
                return Ok(#enum_name::#constructor);
            }
        };

        Ok(block)
    }
}

struct EnumDef<'a> {
    ident: syn::Ident,
    generics: &'a syn::Generics,
    variants: Vec<Variant<'a>>,
}

impl<'a> EnumDef<'a> {
    pub fn try_from_ast(input: &'a DeriveInput) -> syn::Result<Self> {
        // Check if the input is an enum
        let syn::Data::Enum(data_enum) = &input.data else {
            return Err(syn::Error::new(
                input.span(),
                "UntaggedEnumDeserialize: can only be derived for enums",
            ));
        };

        // Check for #[serde(untagged)] attribute
        let has_untagged_attr = input.attrs.iter().any(|attr| {
            if !attr.path().is_ident("serde") {
                return false;
            }
            if let Ok(syn::Expr::Path(expr_path)) = attr.parse_args() {
                return expr_path.path.is_ident("untagged");
            }
            false
        });
        if !has_untagged_attr {
            return Err(syn::Error::new(
                input.span(),
                "UntaggedEnumDeserialize: can only be derived for untagged enums",
            ));
        }

        // Check the enum has no borrowed lifetimes
        for param in &input.generics.params {
            if let syn::GenericParam::Lifetime(lifetime_param) = param {
                return Err(syn::Error::new(
                    lifetime_param.lifetime.span(),
                    "UntaggedEnumDeserialize: borrowed lifetimes are not supported",
                ));
            }
        }

        let ident = input.ident.clone();
        let generics = &input.generics;
        let variants = data_enum
            .variants
            .iter()
            .map(Variant::try_from_ast)
            .collect::<syn::Result<Vec<_>>>()?;
        Ok(EnumDef {
            ident,
            generics,
            variants,
        })
    }

    fn build_impl_generics(&self) -> syn::Generics {
        let mut generics = self.generics.clone();
        // Inject a 'de lifetime bound for deserialization
        generics
            .params
            .push(syn::GenericParam::Lifetime(syn::LifetimeParam {
                attrs: Vec::new(),
                lifetime: syn::Lifetime::new("'de", self.ident.span()),
                colon_token: None,
                bounds: syn::punctuated::Punctuated::new(),
            }));

        // Inject a where clause bound `T: serde::de::Deserialize<'_>` for each
        // non-lifetime type parameter `T`:
        for param in &mut generics.params {
            if let syn::GenericParam::Type(ty_param) = param {
                ty_param
                    .bounds
                    .push(syn::parse_quote!(__serde::de::DeserializeOwned));
            }
        }

        generics
    }

    fn gen_deserialize_impl(&self) -> syn::Result<proc_macro2::TokenStream> {
        let enum_name = &self.ident;
        let generics = self.build_impl_generics();
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, ty_generics, _) = self.generics.split_for_impl();

        let mut variant_blocks = Vec::new();
        for variant in &self.variants {
            let deserialize_block = variant.gen_deserialize_block()?;
            let constructor_block = variant.gen_constructor_block(enum_name)?;
            variant_blocks.push(quote! {
                #deserialize_block
                #constructor_block
            });
        }

        let err_message = format!("data did not match any variant of untagged enum {enum_name}");

        Ok(quote! {
            #[automatically_derived]
            impl #impl_generics __serde::Deserialize<'de> for #enum_name #ty_generics #where_clause {
                fn deserialize<__D>(deserializer: __D) -> Result<Self, __D::Error>
                where
                    __D: __serde::de::Deserializer<'de>,
                {
                    let mut __state = __serde_yaml::value::extract_reusable_deserializer_state(deserializer)?;
                    let __unused_key_callback = __state.take_unused_key_callback();
                    let mut __unused_keys = vec![];

                    #( #variant_blocks )*

                    Err(__serde::de::Error::custom(#err_message))
                }
            }
        })
    }
}

fn expand_derive_deserialize(
    input: &mut syn::DeriveInput,
) -> syn::Result<proc_macro2::TokenStream> {
    let enum_def = EnumDef::try_from_ast(input)?;
    let deserialize_impl = enum_def.gen_deserialize_impl()?;

    let block = quote! {
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate dbt_serde_yaml as __serde_yaml;
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as __serde;
            #deserialize_impl
        };
    };

    Ok(block)
}

#[proc_macro_derive(UntaggedEnumDeserialize, attributes(serde))]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);

    expand_derive_deserialize(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
