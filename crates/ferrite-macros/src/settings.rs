use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Expr, Fields, LitStr, Type};

enum Widget {
    Toggle,
    Choice,
    Options(Expr),
    Range { range: Expr, step: Option<Expr> },
}

struct Setting {
    label: Option<String>,
    skip: bool,
    options: Option<Expr>,
    range: Option<Expr>,
    step: Option<Expr>,
}

impl Setting {
    fn parse(field: &syn::Field) -> syn::Result<Self> {
        let mut setting = Self {
            label: None,
            skip: false,
            options: None,
            range: None,
            step: None,
        };
        for attr in field.attrs.iter().filter(|a| a.path().is_ident("setting")) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("skip") {
                    setting.skip = true;
                } else if meta.path.is_ident("label") {
                    setting.label = Some(meta.value()?.parse::<LitStr>()?.value());
                } else if meta.path.is_ident("options") {
                    setting.options = Some(meta.value()?.parse()?);
                } else if meta.path.is_ident("range") {
                    setting.range = Some(meta.value()?.parse()?);
                } else if meta.path.is_ident("step") {
                    setting.step = Some(meta.value()?.parse()?);
                } else {
                    return Err(
                        meta.error("expected `label`, `options`, `range`, `step` or `skip`")
                    );
                }
                Ok(())
            })?;
        }
        if setting.options.is_some() && setting.range.is_some() {
            return Err(Error::new_spanned(
                field,
                "use either `options` or `range`, not both",
            ));
        }
        if setting.step.is_some() && setting.range.is_none() {
            return Err(Error::new_spanned(field, "`step` needs `range`"));
        }
        Ok(setting)
    }

    fn widget(self, ty: &Type) -> Widget {
        match (self.options, self.range) {
            (Some(options), _) => Widget::Options(options),
            (None, Some(range)) => Widget::Range {
                range,
                step: self.step,
            },
            (None, None) if is_bool(ty) => Widget::Toggle,
            (None, None) => Widget::Choice,
        }
    }
}

pub fn expand_settings(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &input.generics,
            "Settings cannot be derived for generic types",
        ));
    }
    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            name,
            "Settings can be derived for structs with named fields only",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(Error::new_spanned(
            name,
            "Settings can be derived for structs with named fields only",
        ));
    };
    let mut widgets = Vec::new();
    for field in &fields.named {
        let setting = Setting::parse(field)?;
        if setting.skip {
            continue;
        }
        let ident = field.ident.as_ref().expect("named field");
        let label = setting
            .label
            .clone()
            .unwrap_or_else(|| sentence_case(&ident.to_string()));
        let on_change = quote! {
            {
                let editor = ::core::clone::Clone::clone(editor);
                move |state: &mut M, value| editor.edit(state, |settings: &mut Self| settings.#ident = value)
            }
        };
        widgets.push(match setting.widget(&field.ty) {
            Widget::Toggle => quote!(page.toggle(#label, self.#ident, #on_change);),
            Widget::Choice => {
                let ty = &field.ty;
                quote_spanned! {ty.span()=>
                    page.choice::<#ty>(#label, &self.#ident, #on_change);
                }
            }
            Widget::Options(options) => {
                let options = convert_string_literals(options);
                quote!(page.cycle(#label, &#options, &self.#ident, #on_change);)
            }
            Widget::Range { range, step } => {
                let step = step.map_or_else(|| quote!(1), |s| quote!(#s));
                quote! {
                    page.cycle(
                        #label,
                        &::core::iter::Iterator::collect::<::std::vec::Vec<_>>(
                            ::core::iter::Iterator::step_by(#range, #step),
                        ),
                        &self.#ident,
                        #on_change,
                    );
                }
            }
        });
    }
    Ok(quote! {
        impl ::ferrite_api::ui::Settings for #name {
            fn add_to<M: 'static>(
                &self,
                page: &mut ::ferrite_api::ui::Page<M>,
                editor: &::ferrite_api::ui::Editor<M, Self>,
            ) {
                #(#widgets)*
            }
        }
    })
}

pub fn expand_choice(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &input.generics,
            "Choice cannot be derived for generic types",
        ));
    }
    let Data::Enum(data) = &input.data else {
        return Err(Error::new_spanned(
            name,
            "Choice can be derived for enums only",
        ));
    };
    if data.variants.is_empty() {
        return Err(Error::new_spanned(
            name,
            "Choice needs at least one variant",
        ));
    }
    let mut idents = Vec::new();
    let mut labels = Vec::new();
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(Error::new_spanned(
                variant,
                "Choice variants cannot have fields",
            ));
        }
        let mut label = None;
        for attr in variant.attrs.iter().filter(|a| a.path().is_ident("choice")) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("label") {
                    label = Some(meta.value()?.parse::<LitStr>()?.value());
                    Ok(())
                } else {
                    Err(meta.error("expected `label`"))
                }
            })?;
        }
        labels.push(label.unwrap_or_else(|| words(&variant.ident.to_string())));
        idents.push(&variant.ident);
    }
    Ok(quote! {
        impl ::ferrite_api::ui::Choice for #name {
            fn choices() -> ::std::vec::Vec<Self> {
                ::std::vec![#(Self::#idents),*]
            }

            fn label(&self) -> ::std::string::String {
                ::std::string::String::from(match self {
                    #(Self::#idents => #labels,)*
                })
            }
        }
    })
}

fn convert_string_literals(options: Expr) -> TokenStream2 {
    let Expr::Array(array) = options else {
        return quote!(#options);
    };
    let elems = array.elems.iter().map(|elem| match elem {
        Expr::Lit(lit) if matches!(lit.lit, syn::Lit::Str(_)) => {
            quote!(::core::convert::Into::into(#elem))
        }
        other => quote!(#other),
    });
    quote!([#(#elems),*])
}

fn is_bool(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if path.qself.is_none() && path.path.is_ident("bool"))
}

fn sentence_case(name: &str) -> String {
    let text = name.trim_start_matches("r#").replace('_', " ");
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn words(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (index, ch) in name.char_indices() {
        if ch.is_uppercase() && index > 0 {
            out.push(' ');
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_read_like_sentences() {
        assert_eq!(sentence_case("show_coords"), "Show coords");
        assert_eq!(sentence_case("r#type"), "Type");
        assert_eq!(words("TopLeft"), "Top left");
        assert_eq!(words("Fast"), "Fast");
    }
}
