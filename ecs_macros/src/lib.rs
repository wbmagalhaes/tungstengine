use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics ::tungstengine::ecs::Component for #name #ty_generics #where_clause {}
    }
    .into()
}

#[proc_macro_derive(Resource, attributes(resource))]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let mut core = false;
    for attr in input.attrs.iter().filter(|a| a.path().is_ident("resource")) {
        let parsed = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("core") {
                core = true;
                Ok(())
            } else {
                Err(meta.error("unknown option in #[resource(...)]; expected #[resource(core)]"))
            }
        });
        if let Err(e) = parsed {
            return e.to_compile_error().into();
        }
    }
    let core_impl = core.then(|| {
        quote! {
            impl #impl_generics ::tungstengine::ecs::ResourceCore for #name #ty_generics #where_clause {}
        }
    });
    quote! {
        impl #impl_generics ::tungstengine::ecs::Resource for #name #ty_generics #where_clause {}
        #core_impl
    }
    .into()
}

#[proc_macro_derive(Action)]
pub fn derive_action(input: TokenStream) -> TokenStream {
    derive_named(input, quote! { ::tungstengine::input::Action })
}

#[proc_macro_derive(Sound)]
pub fn derive_sound(input: TokenStream) -> TokenStream {
    derive_named(input, quote! { ::tungstengine::audio::Sound })
}

#[proc_macro_derive(SpriteName, attributes(sprite))]
pub fn derive_sprite_name(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let syn::Data::Enum(data) = &input.data else {
        return syn::Error::new_spanned(name, "SpriteName only works on enums")
            .to_compile_error()
            .into();
    };
    let keys = data.variants.iter().map(|v| {
        let ident = &v.ident;
        let key = sprite_key(v);
        match &v.fields {
            syn::Fields::Unit => quote! {
                #name::#ident => ::std::borrow::Cow::Borrowed(#key)
            },
            _ => quote! {
                #name::#ident(value) => {
                    ::std::borrow::Cow::Owned(format!("{}_{}", #key, value))
                }
            },
        }
    });
    let ids = data.variants.iter().map(|v| {
        let ident = &v.ident;
        let key = sprite_key(v);
        match &v.fields {
            syn::Fields::Unit => quote! {
                #name::#ident => ::tungstengine::sprites::sprite_id(#key)
            },
            _ => quote! {
                #name::#ident(value) => ::tungstengine::sprites::sprite_id_with(#key, value)
            },
        }
    });
    quote! {
        impl #impl_generics ::tungstengine::sprites::SpriteKey for #name #ty_generics #where_clause {
            fn key(&self) -> ::std::borrow::Cow<'_, str> {
                match self { #(#keys,)* }
            }

            fn id(&self) -> u64 {
                match self { #(#ids,)* }
            }
        }
    }
    .into()
}

fn sprite_key(variant: &syn::Variant) -> String {
    variant
        .attrs
        .iter()
        .find(|a| a.path().is_ident("sprite"))
        .and_then(|a| a.parse_args::<syn::LitStr>().ok())
        .map_or_else(|| snake_case(&variant.ident.to_string()), |l| l.value())
}

fn derive_named(input: TokenStream, target: proc_macro2::TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let syn::Data::Enum(data) = &input.data else {
        return syn::Error::new_spanned(name, "this derive only works on enums")
            .to_compile_error()
            .into();
    };
    let arms = data.variants.iter().map(|v| {
        let ident = &v.ident;
        let key = snake_case(&ident.to_string());
        quote! { #name::#ident => #key }
    });
    quote! {
        impl #target for #name {
            fn name(self) -> &'static str {
                match self { #(#arms,)* }
            }
        }
    }
    .into()
}

#[proc_macro]
pub fn embed_game(input: TokenStream) -> TokenStream {
    let file = if input.is_empty() {
        "manifest.toml".to_string()
    } else {
        parse_macro_input!(input as syn::LitStr).value()
    };
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let manifest = root.join(file).to_string_lossy().to_string();
    let mut locales: Vec<(String, String)> = std::fs::read_dir(root.join("assets/locale"))
        .map(|dir| {
            dir.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "toml"))
                .filter_map(|p| {
                    let stem = p.file_stem()?.to_str()?.to_string();
                    Some((stem, p.to_string_lossy().to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    locales.sort();
    let entries = locales.iter().map(|(name, path)| {
        quote! { (#name, include_str!(#path)) }
    });
    quote! {
        ::tungstengine::manifest::GameAssets {
            manifest: include_str!(#manifest),
            locales: &[#(#entries),*],
        }
    }
    .into()
}

fn snake_case(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(c.to_lowercase());
    }
    out
}
