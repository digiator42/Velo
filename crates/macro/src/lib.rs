use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_macro_input, Expr, Ident, LitStr, Token};

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
        key: Option<Expr>,
        body: Vec<VNode>,
    },
    Fragment(Vec<VNode>), // hold sibling nodes without a wrapper tag!
}

struct VAttr {
    key: String,
    value: Expr,
}

impl Parse for VAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key_ident = input.call(syn::Ident::parse_any)?;

        input.parse::<syn::Token![=]>()?;
        let value = input.parse::<syn::Expr>()?;

        let key = key_ident.to_string().trim_start_matches("r#").to_string();

        Ok(VAttr { key, value })
    }
}

// --- Parsing Implementation ---

impl Parse for VNode {
    fn parse(input: ParseStream) -> Result<Self> {
        // Handle expression blocks e.g., { count } or complex conditional blocks
        if input.peek(syn::token::Brace) {
            let fork = input.fork(); // Create a lookahead fork to test loop vs expression blocks safely
            let content;
            syn::braced!(content in fork);

            // Check for inline loop syntax
            if content.peek(Token![for]) {
                let content;
                syn::braced!(content in input); // Advance actual stream pointer
                content.parse::<Token![for]>()?;
                let pat = syn::Pat::parse_single(&content)?;
                content.parse::<Token![in]>()?;
                let expr = content.parse::<Expr>()?;

                // Optional key expression: `key = |item| item.id` or `key = item.id`
                let key = if content.peek(Ident) && content.peek2(Token![=]) {
                    let ident: Ident = content.parse()?;
                    if ident == "key" {
                        content.parse::<Token![=]>()?;
                        Some(content.parse::<Expr>()?)
                    } else {
                        return Err(syn::Error::new(
                            ident.span(),
                            "Velo: unexpected token after `in <expr>` in for-loop (expected `key =` or `{`)",
                        ));
                    }
                } else {
                    None
                };

                let loop_body_content;
                syn::braced!(loop_body_content in content);

                let mut body = Vec::new();
                while !loop_body_content.is_empty() {
                    body.push(loop_body_content.parse::<VNode>()?);
                }

                return Ok(VNode::ForLoop {
                    pat,
                    expr,
                    key,
                    body,
                });
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

        // Parse HTML Tags or Fragments e.g., <div ...> ... </div> or <> ... </>
        input.parse::<Token![<]>()?;

        // Detect Fragment Syntax: Check if the token immediately following `<` is `>`
        if input.peek(Token![>]) {
            input.parse::<Token![>]>()?; // Consume the opening `>`

            let mut children = Vec::new();
            // Parse internal children until we see the start of the closing tag `</`
            while !input.peek(Token![<]) || !input.peek2(Token![/]) {
                children.push(input.parse::<VNode>()?);
            }

            // Consume the fragment closing syntax `</>`
            input.parse::<Token![<]>()?;
            input.parse::<Token![/]>()?;
            input.parse::<Token![>]>()?;

            return Ok(VNode::Fragment(children));
        }

        // Otherwise, proceed to parse standard HTML tag elements
        let tag_ident = input.call(syn::Ident::parse_any)?; // 🚀 Use parse_any for custom component keywords too
        let tag_name = tag_ident.to_string().trim_start_matches("r#").to_string();

        let mut attributes = Vec::new();
        while !input.peek(Token![>]) && !input.peek(Token![/]) {
            // Use parse_any to allow parsing keywords if they show up as attribute parts
            let mut key = input.call(syn::Ident::parse_any)?.to_string();

            // Loop to catch hyphens (e.g., stroke-linecap or aria-describedby)
            while input.peek(Token![-]) {
                input.parse::<Token![-]>()?;
                let next_part = input.call(syn::Ident::parse_any)?.to_string();
                key = format!("{}-{}", key, next_part);
            }

            // Keep your existing namespace colon tracking (e.g., on:click)
            if input.peek(Token![:]) {
                input.parse::<Token![:]>()?;
                let sub_key = input.call(syn::Ident::parse_any)?.to_string();
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
        let end_tag = input.call(syn::Ident::parse_any)?;
        let end_tag_name = end_tag.to_string().trim_start_matches("r#").to_string();
        if end_tag_name != tag_name {
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


impl ToTokens for VNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            VNode::StaticText(txt) => {
                tokens.extend(quote! {
                    velo_dom::DomNode::text(#txt)
                });
            }
            VNode::ReactiveExpression(expr) => {
                match expr {
                    Expr::Closure(_) => {
                        tokens.extend(quote! {
                            velo_dom::DomNode::render_expression(#expr)
                        });
                    }
                    _ => {
                        tokens.extend(quote! {
                            velo_dom::DomNode::render_expression(move || velo_dom::signal_value!(#expr))
                        });
                    }
                }
            }
            VNode::ForLoop {
                pat,
                expr,
                key,
                body,
            } => {
                let compiled_body_nodes = body.iter().map(|child| {
                    quote! { #child }
                });

                if let Some(key_expr) = key {
                    // Keyed, fine-grained list: uses SignalVec + the DOM reconciler.
                    tokens.extend(quote! {
                        {
                            let loop_fragment = velo_dom::DomNode::element("div");
                            loop_fragment.reactive_attribute("class", || "contents".into());

                            let list = (#expr).clone();
                            let key_fn = #key_expr;
                            loop_fragment.render_signal_vec(
                                &list,
                                key_fn,
                                move |#pat| -> velo_dom::DomNode {
                                    #(#compiled_body_nodes)*
                                }
                            );
                            loop_fragment
                        }
                    });
                } else {
                    tokens.extend(quote! {
                        {
                            let loop_fragment = velo_dom::DomNode::fragment();

                            for #pat in #expr {
                                #(
                                    loop_fragment.append(&#compiled_body_nodes);
                                )*
                            }
                            loop_fragment
                        }
                    });
                }
            }
            VNode::Fragment(children) => {
                let compiled_children = children.iter().map(|child| {
                    quote! { #child }
                });

                tokens.extend(quote! {
                    {
                        let fragment_node = velo_dom::DomNode::fragment();
                        #(
                            fragment_node.append(&#compiled_children);
                        )*
                        fragment_node
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
                        } else if key.starts_with("class:") {
                            // Reactive class toggle: class:active={ is_on }
                            let class_name = key.strip_prefix("class:").unwrap().to_string();
                            setup_statements.push(quote! {
                                parent_node.toggle_class(#class_name, move || velo_dom::signal_value!(#val));
                            });
                        } else if key.starts_with("style:") {
                            // Reactive inline style: style:color={ color }
                            let prop = key.strip_prefix("style:").unwrap().to_string();
                            setup_statements.push(quote! {
                                parent_node.reactive_style(#prop, move || velo_dom::signal_value!(#val));
                            });
                        } else if key.starts_with("bind:value") {
                            // Two-way binding for text inputs, textareas, selects.
                            // 1. Hook the "input" event: every keystroke reads the element
                            //    and sets the signal so the signal tracks the DOM.
                            // 2. Reactive forwarding: when the signal is set externally
                            //    (e.g. clearing the field after "Add"), push it back to
                            //    the element's value attribute.
                            //
                            // Evaluate (#val) once into a temporary, then clone that
                            // temporary into each generated closure so there's no shared
                            // variable that both closures try to move.
                            let event_name_str: syn::LitStr = syn::parse_quote! { "input" };
                            let field_name: syn::LitStr = syn::parse_quote! { "value" };
                            setup_statements.push(quote! {
                                let bind_node = parent_node.clone();
                                let bind_tmp = (#val).clone();
                                let bind_sig_1 = bind_tmp.clone();
                                let bind_sig_2 = bind_tmp.clone();
                                // Forward signal -> DOM (reactive value attribute).
                                bind_node.reactive_attribute(#field_name, move || {
                                    let v = velo_dom::signal_value!(bind_sig_1);
                                    format!("{}", v)
                                });
                                // Forward DOM -> signal (on input, read element and set signal).
                                bind_node.on(#event_name_str, move |e: web_sys::Event| {
                                    use wasm_bindgen::JsCast;
                                    let target = e.target().expect("bind:value event has no target");
                                    let el = target.dyn_into::<web_sys::HtmlInputElement>().expect("bind:value requires an input/textarea/select element");
                                    bind_sig_2.set(el.value());
                                });
                            });
                        } else if key.starts_with("bind:checked") {
                            // Two-way binding for checkboxes / radio buttons.
                            // 1. Hook the "change" event: toggled checkbox writes its
                            //    checked state back into the signal.
                            // 2. Reactive forwarding: external signal changes push
                            //    their state into the element's checked attribute.
                            //
                            // Evaluate (#val) once into a temporary, then clone that
                            // temporary into each generated closure so there's no shared
                            // variable that both closures try to move.
                            let event_name_str: syn::LitStr = syn::parse_quote! { "change" };
                            setup_statements.push(quote! {
                                let bind_node = parent_node.clone();
                                let bind_tmp = (#val).clone();
                                let bind_sig_1 = bind_tmp.clone();
                                let bind_sig_2 = bind_tmp.clone();
                                // Forward signal -> DOM (reactive checked attribute).
                                bind_node.reactive_attribute("checked", move || {
                                    if velo_dom::signal_value!(bind_sig_1) { "checked" } else { "" }
                                });
                                // Forward DOM -> signal (on change, read checked and set signal).
                                bind_node.on(#event_name_str, move |e: web_sys::Event| {
                                    use wasm_bindgen::JsCast;
                                    let target = e.target().expect("bind:checked event has no target");
                                    let el = target.dyn_into::<web_sys::HtmlInputElement>().expect("bind:checked requires a checkbox/radio input");
                                    bind_sig_2.set(el.checked());
                                });
                            });
                        } else if key == "disabled"
                            || key == "checked"
                            || key == "selected"
                            || key == "readonly"
                        {
                            setup_statements.push(quote! {
                            let p_node = parent_node.clone();
                            // Use the top-level unified facade pathway
                            velo_core::create_effect({
                                let val_sig = move || #val;
                                move || {
                                    use wasm_bindgen::JsCast;
                                    if let Ok(el) = p_node.raw_node.clone().dyn_into::<web_sys::Element>() {
                                        if val_sig() {
                                            let _ = el.set_attribute(#key, "");
                                        } else {
                                            let _ = el.remove_attribute(#key);
                                        }
                                    }
                                }
                            });
                        });
                        } else {
                            setup_statements.push(quote! {
                                parent_node.reactive_attribute(#key, move || format!("{}", velo_dom::signal_value!(#val)));
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

/// `#[component]` turns a plain function into a Velo component.
///
/// It rewrites the function's return type to `velo_dom::DomNode` so the body can
/// end with a `view! { ... }` tail expression (no explicit `return` / `-> DomNode`
/// needed). Component arguments are passed by the `view!` macro as usual.
///
/// ```ignore
/// #[velo::component]
/// fn UserCard(name: String, active: bool) {
///     view! {
///         <div class:active={ active }>
///             <p>"Hello, " { name }</p>
///         </div>
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = match syn::parse::<syn::ItemFn>(item) {
        Ok(f) => f,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    // Force the return type to DomNode (accept `-> impl ...`, `-> DomNode`, or none).
    func.sig.output = syn::ReturnType::Type(
        Default::default(),
        Box::new(syn::parse_quote!(velo_dom::DomNode)),
    );

    TokenStream::from(quote! { #func })
}

/// Declarative macro for defining route tables.
#[proc_macro]
pub fn routes(input: TokenStream) -> TokenStream {
    let routes = parse_macro_input!(input as RouteList);

    let route_entries = routes
        .0
        .iter()
        .map(|(path, component)| {
            let path_lit = path;
            let comp = component;
            quote! {
                velo_router::Route {
                    path: #path_lit,
                    component: #comp,
                }
            }
        });

    let expanded = quote! {
        vec![#(#route_entries),*]
    };

    TokenStream::from(expanded)
}

/// Parse the full route list.
struct RouteList(Vec<(LitStr, Ident)>);

impl syn::parse::Parse for RouteList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut entries = Vec::new();
        while !input.is_empty() {
            let path: LitStr = input.parse()?;
            input.parse::<syn::Token![=>]>()?;
            let comp: Ident = input.parse()?;
            if !input.is_empty() {
                input.parse::<syn::Token![,]>()?;
            }
            entries.push((path, comp));
        }
        Ok(RouteList(entries))
    }
}
