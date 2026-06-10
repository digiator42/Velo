extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::{Expr, LitStr, Token};

enum VNode {
    Element {
        tag_name: String,
        attributes: Vec<VAttr>,
        children: Vec<VNode>,
    },
    StaticText(String),
    ReactiveExpression(Expr),
    ForLoop {
        pat: syn::Pat,
        expr: Expr,
        body: Vec<VNode>,
    },
}

struct VAttr {
    key: String,
    value: Expr,
}

// --- Parsing Implementation ---

impl Parse for VNode {
    fn parse(input: ParseStream) -> Result<Self> {
        // Handle expression blocks e.g., { count } or complex conditional blocks
        if input.peek(syn::token::Brace) {
            let fork = input.fork(); // Create a lookahead fork to test loop vs expression blocks safely
            let content;
            syn::braced!(content in fork);

            // 🔍 1. Check for inline loop syntax
            if content.peek(Token![for]) {
                let content;
                syn::braced!(content in input); // Advance actual stream pointer
                content.parse::<Token![for]>()?;
                let pat = syn::Pat::parse_single(&content)?;
                content.parse::<Token![in]>()?;
                let expr = content.parse::<Expr>()?;

                let loop_body_content;
                syn::braced!(loop_body_content in content);

                let mut body = Vec::new();
                while !loop_body_content.is_empty() {
                    body.push(loop_body_content.parse::<VNode>()?);
                }

                return Ok(VNode::ForLoop { pat, expr, body });
            }

            // Try to parse as a simple expression first, fallback to a full statement block expression!
            let content;
            let braced_token = syn::braced!(content in input); // Advance actual stream pointer

            if let Ok(expr) = content.parse::<Expr>() {
                if content.is_empty() {
                    return Ok(VNode::ReactiveExpression(expr));
                }
            }

            // If it contains statements or semicolons, parse it as a full block statement expression!
            let block = content.call(syn::Block::parse_within)?;
            let expr_block = Expr::Block(syn::ExprBlock {
                attrs: vec![],
                label: None,
                block: syn::Block {
                    brace_token: braced_token,
                    stmts: block,
                },
            });

            return Ok(VNode::ReactiveExpression(expr_block));
        }

        // Handle text literals e.g., "Click me"
        if input.peek(LitStr) {
            let lit: LitStr = input.parse()?;
            return Ok(VNode::StaticText(lit.value()));
        }

        // Parse HTML Tags e.g., <div ...> ... </div>
        input.parse::<Token![<]>()?;
        let tag_ident: syn::Ident = input.parse()?;
        let tag_name = tag_ident.to_string();

        let mut attributes = Vec::new();
        while !input.peek(Token![>]) && !input.peek(Token![/]) {
            let mut key = input.parse::<syn::Ident>()?.to_string();

            if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;
                let sub_key = input.parse::<syn::Ident>()?.to_string();
                key = format!("{}:{}", key, sub_key);
            }

            input.parse::<Token![=]>()?;

            let value: Expr = if input.peek(syn::token::Brace) {
                let content;
                syn::braced!(content in input);
                content.parse()?
            } else {
                let lit: LitStr = input.parse()?;
                Expr::Lit(syn::ExprLit {
                    attrs: vec![],
                    lit: syn::Lit::Str(lit),
                })
            };

            attributes.push(VAttr { key, value });
        }

        if input.peek(Token![/]) {
            input.parse::<Token![/]>()?;
            input.parse::<Token![>]>()?;
            return Ok(VNode::Element {
                tag_name,
                attributes,
                children: vec![],
            });
        }

        input.parse::<Token![>]>()?;

        let mut children = Vec::new();
        while !input.peek(Token![<]) || !input.peek2(Token![/]) {
            children.push(input.parse::<VNode>()?);
        }

        input.parse::<Token![<]>()?;
        input.parse::<Token![/]>()?;
        let end_tag: syn::Ident = input.parse()?;
        if end_tag.to_string() != tag_name {
            return Err(syn::Error::new(
                end_tag.span(),
                format!(
                    "Velo Macro Error: Expected matching closing tag </{}>",
                    tag_name
                ),
            ));
        }
        input.parse::<Token![>]>()?;

        Ok(VNode::Element {
            tag_name,
            attributes,
            children,
        })
    }
}

// --- Code Generation Engine ---

impl ToTokens for VNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            VNode::StaticText(txt) => {
                tokens.extend(quote! {
                    velo_dom::DomNode::text(#txt)
                });
            }
            VNode::ReactiveExpression(expr) => {
                let expr_string = quote! { #expr }.to_string();

                if expr_string.starts_with("move") || expr_string.starts_with("||") {
                    tokens.extend(quote! {
                        velo_dom::DomNode::render_expression(#expr)
                    });
                } else {
                    tokens.extend(quote! {
                        velo_dom::DomNode::render_expression(move || #expr)
                    });
                }
            }
            VNode::ForLoop { pat, expr, body } => {
                let compiled_body_nodes = body.iter().map(|child| {
                    quote! { #child }
                });

                tokens.extend(quote! {
                    {
                        let loop_fragment = velo_dom::DomNode::element("div");
                        loop_fragment.reactive_attribute("class", || "contents".into());

                        for #pat in #expr {
                            #(
                                loop_fragment.append(&#compiled_body_nodes); // 🚀 Semicolon is safely tucked inside the repetition!
                            )*
                        }
                        loop_fragment
                    }
                });
            }
            VNode::Element {
                tag_name,
                attributes,
                children,
            } => {
                let first_char = tag_name.chars().next().unwrap_or(' ');

                if first_char.is_uppercase() {
                    let component_ident = syn::Ident::new(tag_name, proc_macro2::Span::call_site());

                    let mut args = Vec::new();
                    for attr in attributes {
                        let val = &attr.value;
                        args.push(quote! { #val });
                    }

                    if !children.is_empty() {
                        args.push(quote! {
                            view! { #(#children)* }
                        });
                    }

                    tokens.extend(quote! {
                        #component_ident(#(#args),*)
                    });
                } else {
                    let mut setup_statements = Vec::new();

                    setup_statements.push(quote! {
                        let parent_node = velo_dom::DomNode::element(#tag_name);
                    });

                    for attr in attributes {
                        let key = &attr.key;
                        let val = &attr.value;

                        if key.starts_with("on:") {
                            let event_type = key.strip_prefix("on:").unwrap();
                            setup_statements.push(quote! {
                                parent_node.on(#event_type, #val);
                            });
                        } else {
                            setup_statements.push(quote! {
                                parent_node.reactive_attribute(#key, move || format!("{}", #val));
                            });
                        }
                    }

                    for child in children {
                        setup_statements.push(quote! {
                            parent_node.append(&#child);
                        });
                    }

                    tokens.extend(quote! {
                        {
                            #(#setup_statements)*
                            parent_node
                        }
                    });
                }
            }
        }
    }
}

#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    match syn::parse::<VNode>(input) {
        Ok(parsed_root) => TokenStream::from(parsed_root.to_token_stream()),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}
