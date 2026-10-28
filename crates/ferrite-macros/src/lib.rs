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
            let server = fields.server_impl();
            let body = quote! {
                cursor.set_usage(<Self as ::ferrite_api::FromArgs>::USAGE);
                ::core::result::Result::Ok(#construct)
            };
            let usage = fields.usage.clone();
            return Ok(quote! {
                impl #impl_generics ::ferrite_api::FromArgs for #name #ty_generics #where_clause {
                    const USAGE: &'static str = #usage;

                    fn from_args(
                        cursor: &mut ::ferrite_api::ArgCursor<'_>,
                    ) -> ::core::result::Result<Self, ::ferrite_api::Error> {
                        #body
                    }

                    #server
                }
            });
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
    server: Option<(Vec<TokenStream2>, Vec<TokenStream2>)>,
}

const SERVER_TYPES: [&str; 13] = [
    "i8",
    "i16",
    "i32",
    "u8",
    "u16",
    "u32",
    "f32",
    "f64",
    "bool",
    "String",
    "BlockState",
    "BlockPos",
    "Player",
];

fn server_argument(
    ty: &Type,
    label: &str,
    rest: bool,
    optional: bool,
) -> Option<(TokenStream2, TokenStream2)> {
    let inner = if optional { option_inner(ty)? } else { ty };
    let name = last_ident(inner)?;
    if !SERVER_TYPES.contains(&name.as_str()) || (rest && name != "String" && name != "BlockState")
    {
        return None;
    }
    let kind = if rest {
        quote!(::ferrite_api::server::ArgumentKind::Text)
    } else {
        quote!(<#inner as ::ferrite_api::server::ServerArgument>::kind())
    };
    let spec = quote!(::ferrite_api::server::ArgumentSpec::new(#label, #kind, #optional));
    let value = if optional {
        quote!(values.optional::<#inner>(#label)?)
    } else {
        quote!(values.required::<#inner>(#label)?)
    };
    Some((spec, value))
}

fn last_ident(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else {
        return None;
    };
    if path.qself.is_some() {
        return None;
    }
    let segment = path.path.segments.last()?;
    matches!(segment.arguments, PathArguments::None).then(|| segment.ident.to_string())
}

fn option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let PathArguments::AngleBracketed(args) = &path.path.segments.last()?.arguments else {
        return None;
    };
    match args.args.first()? {
        GenericArgument::Type(inner) => Some(inner),
        _ => None,
    }
}

impl<'a> Arguments<'a> {
    fn parse(fields: &'a Fields) -> syn::Result<Self> {
        let count = fields.len();
        let mut usage = Vec::with_capacity(count);
        let mut values = Vec::with_capacity(count);
        let mut server = Some((Vec::with_capacity(count), Vec::with_capacity(count)));
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
            server = server.and_then(|(mut specs, mut server_values)| {
                let (spec, value) = server_argument(&field.ty, &label, rest, optional)?;
                specs.push(spec);
                server_values.push(value);
                Some((specs, server_values))
            });
            let block_pos = !rest && {
                let inner = if optional {
                    option_inner(&field.ty)
                } else {
                    Some(&field.ty)
                };
                inner.and_then(last_ident).as_deref() == Some("BlockPos")
            };
            if block_pos {
                let (token, value) = if optional {
                    (
                        format!("[{label}]"),
                        quote!(cursor.optional_block_pos(#label)?),
                    )
                } else {
                    (
                        format!("<{label}>"),
                        quote!(cursor.required_block_pos(#label)?),
                    )
                };
                usage.push(token);
                values.push(value);
                continue;
            }
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
            server,
        })
    }

    fn server_impl(&self) -> TokenStream2 {
        let Some((specs, values)) = &self.server else {
            return TokenStream2::new();
        };
        let construct = match self.fields {
            Fields::Named(named) => {
                let names = named.named.iter().map(|f| &f.ident);
                quote!(Self { #(#names: #values),* })
            }
            Fields::Unnamed(_) => quote!(Self(#(#values),*)),
            Fields::Unit => quote!(Self),
        };
        quote! {
            fn server_arguments() -> ::core::option::Option<
                ::std::vec::Vec<::ferrite_api::server::ArgumentSpec>,
            > {
                ::core::option::Option::Some(::std::vec![#(#specs),*])
            }

            fn from_server(
                values: &::ferrite_api::server::Values<'_>,
            ) -> ::core::result::Result<Self, ::ferrite_api::Error> {
                ::core::result::Result::Ok(#construct)
            }
        }
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
