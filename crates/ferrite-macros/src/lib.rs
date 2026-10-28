use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Data, DataEnum, DeriveInput, Error, Fields, GenericArgument, PathArguments, Type,
    parse_macro_input,
};

mod settings;

#[proc_macro_derive(Settings, attributes(setting))]
pub fn derive_settings(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    settings::expand_settings(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Choice, attributes(choice))]
pub fn derive_choice(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    settings::expand_choice(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(FromArgs, attributes(rest, default))]
pub fn derive_from_args(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let (usage, body) = match &input.data {
        Data::Struct(data) => {
            let fields = Arguments::parse(&data.fields)?;
            let construct = fields.construct(quote!(Self));
            let body = quote! {
                cursor.set_usage(<Self as ::ferrite_api::FromArgs>::USAGE);
                ::core::result::Result::Ok(#construct)
            };
            (fields.usage, body)
        }
        Data::Enum(data) => expand_enum(input, data)?,
        Data::Union(_) => {
            return Err(Error::new_spanned(
                name,
                "FromArgs can be derived for structs and enums only",
            ));
        }
    };
    Ok(quote! {
        impl #impl_generics ::ferrite_api::FromArgs for #name #ty_generics #where_clause {
            const USAGE: &'static str = #usage;

            fn from_args(
                cursor: &mut ::ferrite_api::ArgCursor<'_>,
            ) -> ::core::result::Result<Self, ::ferrite_api::Error> {
                #body
            }
        }
    })
}

fn expand_enum(input: &DeriveInput, data: &DataEnum) -> syn::Result<(String, TokenStream2)> {
    if data.variants.is_empty() {
        return Err(Error::new_spanned(
            &input.ident,
            "FromArgs needs at least one variant",
        ));
    }
    let mut usages = Vec::new();
    let mut arms = Vec::new();
    let mut default = None;
    for variant in &data.variants {
        let ident = &variant.ident;
        let word = kebab_case(&ident.to_string());
        let fields = Arguments::parse(&variant.fields)?;
        let usage = if fields.usage.is_empty() {
            word.clone()
        } else {
            format!("{word} {}", fields.usage)
        };
        if variant.attrs.iter().any(|a| a.path().is_ident("default")) {
            if !matches!(variant.fields, Fields::Unit) {
                return Err(Error::new_spanned(
                    variant,
                    "#[default] must be on a variant without fields",
                ));
            }
            if default.is_some() {
                return Err(Error::new_spanned(
                    variant,
                    "only one variant can be #[default]",
                ));
            }
            default = Some(quote!(Self::#ident));
        }
        let construct = fields.construct(quote!(Self::#ident));
        arms.push(quote! {
            #word => {
                cursor.set_usage(#usage);
                ::core::result::Result::Ok(#construct)
            }
        });
        usages.push(usage);
    }
    let when_empty = match default {
        Some(variant) => quote!(::core::result::Result::Ok(#variant)),
        None => quote!(::core::result::Result::Err(cursor.missing_option())),
    };
    let body = quote! {
        cursor.set_usage(<Self as ::ferrite_api::FromArgs>::USAGE);
        let ::core::option::Option::Some(word) = cursor.next_word() else {
            return #when_empty;
        };
        match word.to_ascii_lowercase().as_str() {
            #(#arms)*
            _ => ::core::result::Result::Err(cursor.unknown_option(word)),
        }
    };
    Ok((usages.join(" | "), body))
}

struct Arguments<'a> {
    fields: &'a Fields,
    usage: String,
    values: Vec<TokenStream2>,
}

impl<'a> Arguments<'a> {
    fn parse(fields: &'a Fields) -> syn::Result<Self> {
        let count = fields.len();
        let mut usage = Vec::with_capacity(count);
        let mut values = Vec::with_capacity(count);
        let mut optional_seen = false;
        for (index, field) in fields.iter().enumerate() {
            let label = match &field.ident {
                Some(ident) => ident.to_string().trim_start_matches("r#").replace('_', "-"),
                None => format!("arg{}", index + 1),
            };
            let rest = field.attrs.iter().any(|a| a.path().is_ident("rest"));
            let optional = is_option(&field.ty);
            if rest && index + 1 != count {
                return Err(Error::new_spanned(
                    field,
                    "#[rest] must be on the last field",
                ));
            }
            if optional_seen && !optional {
                return Err(Error::new_spanned(
                    field,
                    "required arguments must come before optional ones",
                ));
            }
            optional_seen |= optional;
            let (token, value) = match (rest, optional) {
                (true, true) => (
                    format!("[{label}...]"),
                    quote!(cursor.optional_rest(#label)?),
                ),
                (true, false) => (
                    format!("<{label}...>"),
                    quote!(cursor.required_rest(#label)?),
                ),
                (false, true) => (format!("[{label}]"), quote!(cursor.optional(#label)?)),
                (false, false) => (format!("<{label}>"), quote!(cursor.required(#label)?)),
            };
            usage.push(token);
            values.push(value);
        }
        Ok(Self {
            fields,
            usage: usage.join(" "),
            values,
        })
    }

    fn construct(&self, path: TokenStream2) -> TokenStream2 {
        let values = &self.values;
        match self.fields {
            Fields::Named(named) => {
                let names = named.named.iter().map(|f| &f.ident);
                quote!(#path { #(#names: #values),* })
            }
            Fields::Unnamed(_) => quote!(#path(#(#values),*)),
            Fields::Unit => path,
        }
    }
}

fn is_option(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.qself.is_none()
        && path.path.segments.last().is_some_and(|segment| {
            segment.ident == "Option"
                && matches!(
                    &segment.arguments,
                    PathArguments::AngleBracketed(args)
                        if matches!(args.args.first(), Some(GenericArgument::Type(_)))
                )
        })
}

fn kebab_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (index, ch) in name.char_indices() {
        if ch.is_uppercase() {
            if index > 0 {
                out.push('-');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}
