extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_macro_input, Expr, LitStr, Token};

/// Represents the types of UI components parsed inside our view macro
enum VNode {
    Element {
        tag_name: String,
        attributes: Vec<VAttr>,
        children: Vec<VNode>,
    },
    StaticText(String),
    ReactiveExpression(Expr),
}

/// Represents attributes like class="btn" or reactive event handlers like on:click={...}
struct VAttr {
    key: String,
    value: Expr,
}

// --- Parsing Implementation ---

impl Parse for VNode {
    fn parse(input: ParseStream) -> Result<Self> {
        // Handle expression blocks e.g., { count } or { count.get() }
        if input.peek(syn::token::Brace) {
            let content;
            syn::braced!(content in input);
            let expr: Expr = content.parse()?;
            return Ok(VNode::ReactiveExpression(expr));
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
        // Parse attributes until we hit tag close sequence
        while !input.peek(Token![>]) && !input.peek(Token![/]) {
            let mut key = input.parse::<syn::Ident>()?.to_string();

            // Handle specialized event namespace syntax (e.g., on:click)
            if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;
                let sub_key = input.parse::<syn::Ident>()?.to_string();
                key = format!("{}:{}", key, sub_key);
            }

            input.parse::<Token![=]>()?;

            // Parse attribute value expression wrapped in brackets or string
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

        // Check for self-closing tag context <img />
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

        // Parse internal nested children tags recursively
        let mut children = Vec::new();
        while !input.peek(Token![<]) || !input.peek2(Token![/]) {
            children.push(input.parse::<VNode>()?);
        }

        // Parse ending verification tag matching </div>
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

// --- Code Generation / Deserialization Blueprint ---

impl ToTokens for VNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            VNode::StaticText(txt) => {
                tokens.extend(quote! {
                    dom::DomNode::text(#txt)
                });
            }
            VNode::ReactiveExpression(expr) => {
                // Highly critical for SPA performance:
                // Auto-wrap reactive variables in an isolated, fine-grained thread-safe closure bundle
                tokens.extend(quote! {
                    dom::DomNode::reactive_text({
                        let mut _expr_clone = #expr;
                        move || format!("{}", _expr_clone)
                    })
                });
            }
            VNode::Element {
                tag_name,
                attributes,
                children,
            } => {
                let mut setup_statements = Vec::new();

                // Code block generation steps
                setup_statements.push(quote! {
                    let parent_node = dom::DomNode::element(#tag_name);
                });

                // Attach attributes and reactive click hooks cleanly
                for attr in attributes {
                    let key = &attr.key;
                    let val = &attr.value;

                    if key.starts_with("on:") {
                        let event_type = key.strip_prefix("on:").unwrap();
                        setup_statements.push(quote! {
                            parent_node.on(#event_type, #val);
                        });
                    } else {
                        // Dynamically update the node attribute if specified as raw code block variables
                        setup_statements.push(quote! {
                            parent_node.reactive_attribute(#key, move || format!("{}", #val));
                        });
                    }
                }

                // Recursively append compiled child structures
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

/// The primary compiler entrypoint for the view custom DSL syntax
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    let parsed_root = parse_macro_input!(input as VNode);
    TokenStream::from(parsed_root.to_token_stream())
}
