use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::ext::IdentExt;
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Expr, Ident, LitStr, Pat, Token};

// --- JS-style arrow-closure sugar: `(pat, ..) => expr` / `(pat, ..) => { .. }` ---
// Supports both sync `() => expr` and async `async () => { ... }` forms.
// The `async` keyword gives the Next.js-style async feel for event handlers
// and reactive bodies.
struct ArrowClosure {
    asyncness: Option<Token![async]>,
    inputs: Punctuated<Pat, Token![,]>,
    body: Box<Expr>,
}

impl Parse for ArrowClosure {
    fn parse(input: ParseStream) -> Result<Self> {
        let asyncness = input.parse::<Token![async]>().ok();

        let params;
        syn::parenthesized!(params in input);
        let inputs = params.parse_terminated(Pat::parse_single, Token![,])?;
        input.parse::<Token![=>]>()?;

        let body: Expr = if input.peek(syn::token::Brace) {
            let block: syn::Block = input.parse()?;
            Expr::Block(syn::ExprBlock {
                attrs: vec![],
                label: None,
                block,
            })
        } else {
            input.parse()?
        };

        // Arrow sugar always binds one full expression/block — no trailing
        // tokens should remain in `input` for this attribute/block position.
        Ok(ArrowClosure {
            asyncness,
            inputs,
            body: Box::new(body),
        })
    }
}

/// Speculatively parses `(pat, ..) => ..` sugar. Consumes nothing and
/// returns `None` if the shape doesn't match, so the caller can fall back to
/// normal `Expr` parsing on the untouched stream.
fn try_parse_arrow(input: ParseStream) -> Option<ArrowClosure> {
    let fork = input.fork();
    match fork.parse::<ArrowClosure>() {
        Ok(arrow) => {
            input.advance_to(&fork);
            Some(arrow)
        }
        Err(_) => None,
    }
}

/// Attribute-value position for `on:*` handlers, which must satisfy
/// `FnMut(web_sys::Event) + 'static`. `() => ..` gets an injected, discarded
/// event param; `(e) => ..` binds the event to `e` (untyped — inferred from
/// the `on()` bound, exactly like a hand-written closure would be).
/// Supports `async () => { ... }` for async event handlers (dispatched via
/// wasm-bindgen-futures).
fn parse_event_handler_value(input: ParseStream) -> Result<Expr> {
    if let Some(ArrowClosure { asyncness, inputs, body }) = try_parse_arrow(input) {
        let closure: Expr = if asyncness.is_some() {
            // Async handler: spawn a future via wasm-bindgen-futures
            if inputs.is_empty() {
                syn::parse_quote! { move |_evt: web_sys::Event| {
                    wasm_bindgen_futures::spawn_local(async move { #body })
                } }
            } else {
                syn::parse_quote! { move |#inputs| {
                    wasm_bindgen_futures::spawn_local(async move { #body })
                } }
            }
        } else {
            // Sync handler (existing behavior)
            if inputs.is_empty() {
                syn::parse_quote! { move |_evt: web_sys::Event| #body }
            } else {
                syn::parse_quote! { move |#inputs| #body }
            }
        };
        return Ok(closure);
    }
    input.parse::<Expr>()
}

/// `{ .. }` reactive-expression position, which wants `FnMut() -> R` (see the
/// `Expr::Closure(_)` branch in `ToTokens for VNode` below). Only the 0-arg
/// form makes sense here — reactive expressions don't receive anything — so
/// non-empty params are left unhandled and fall through to a normal parse
/// error rather than silently doing the wrong thing.
/// Supports `async () => { ... }` for async reactive expressions.
fn try_parse_reactive_arrow(input: ParseStream) -> Option<Expr> {
    let fork = input.fork();
    let arrow = fork.parse::<ArrowClosure>().ok()?;
    if !arrow.inputs.is_empty() {
        return None;
    }
    input.advance_to(&fork);
    let body = arrow.body;
    if arrow.asyncness.is_some() {
        // Async reactive expression: return a future
        Some(syn::parse_quote! { move || async move { #body } })
    } else {
        Some(syn::parse_quote! { move || #body })
    }
}

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

impl ToTokens for VAttr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let key = &self.key;
        let value = &self.value;
        tokens.extend(quote! {
            #key: #value
        });
    }
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

            // `{ () => expr }` sugar for a zero-arg reactive closure, e.g.
            // `{ () => count.get() * 2 }` instead of `{ move || count.get() * 2 }`.
            if let Some(closure_expr) = try_parse_reactive_arrow(&content) {
                if content.is_empty() {
                    return Ok(VNode::ReactiveExpression(closure_expr));
                }
            }

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
                if key.starts_with("on:") {
                    // `on:click={() => set_count.update(|c| c + 1)}` sugar,
                    // expanding to `move |_evt: web_sys::Event| ..` / `move |e| ..`.
                    parse_event_handler_value(&content)?
                } else {
                    content.parse()?
                }
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
                    velo::DomNode::text(#txt)
                });
            }
            VNode::ReactiveExpression(expr) => {
                match expr {
                    Expr::Closure(_) => {
                        tokens.extend(quote! {
                            velo::DomNode::render_expression(#expr)
                        });
                    }
                    _ => {
                        tokens.extend(quote! {
                            velo::DomNode::render_expression(move || velo::signal_value!(#expr))
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
                            let loop_fragment = velo::DomNode::element("div");
                            loop_fragment.reactive_attribute("class", || "contents".into());

                            let list = (#expr).clone();
                            let key_fn = #key_expr;
                            loop_fragment.render_signal_vec(
                                &list,
                                key_fn,
                                move |#pat| -> velo::DomNode {
                                    #(#compiled_body_nodes)*
                                }
                            );
                            loop_fragment
                        }
                    });
                } else {
                    tokens.extend(quote! {
                        {
                            let loop_fragment = velo::DomNode::fragment();

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
                        let fragment_node = velo::DomNode::fragment();
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
                    let component_name = component_ident.to_string();

                    // `Show` and `Suspense` are reactive control-flow components.
                    //
                    // Unlike ordinary components (whose children are built once at
                    // view!-construction time), these react to a boolean condition
                    // (an async resource's `loading`) and swap between a fallback
                    // and the content:
                    //   <Show when={ !r.loading() } fallback={...}>{ ... }</Show>
                    //   <Suspense loading={ r.loading() } fallback={...}>{ ... }</Suspense>
                    //
                    // Both branches are built ONCE (children are independently
                    // reactive via their own render_expression effects), then
                    // the condition is wrapped in `signal_value!` so raw signals
                    // auto-unwrap into their current value.
                    // `reactive_switch` moves whichever branch is active into a
                    // fragment, swapping live when the condition flips.
                    if component_name == "Show" || component_name == "Suspense" {
                        let mut when_val = quote! { false };
                        let mut fallback_val = quote! { velo::DomNode::text("") };
                        for attr in attributes {
                            if attr.key == "when" || attr.key == "loading" {
                                let v = &attr.value;
                                // Auto-unwrap via `signal_value!` so a raw signal
                                // (`when={ show_card }`) works as the condition;
                                // plain bool expressions pass through the
                                // `ViewValue` blanket unchanged.
                                when_val = quote! { velo::signal_value!(#v) };
                            } else if attr.key == "fallback" {
                                let v = &attr.value;
                                fallback_val = quote! { #v };
                            }
                        }
                        let content_block = quote! {
                            {
                                let __velo_cf_frag = velo::DomNode::fragment();
                                #( __velo_cf_frag.append(&#children); )*
                                __velo_cf_frag
                            }
                        };
                        // `Show` shows content when `when` is truthy. `Suspense`'s
                        // `loading` predicate is inverted: content is shown once the
                        // resource is DONE loading, fallback while still loading.
                        let predicate = if component_name == "Suspense" {
                            quote! { !#when_val }
                        } else {
                            quote! { #when_val }
                        };
                        tokens.extend(quote! {
                            velo::reactive_switch(
                                move || #predicate,
                                #content_block,
                                #fallback_val,
                            )
                        });
                        return;
                    }

                    // `Link` is a known builtin with an unusual props shape: its
                    // `label` / `active_class` fields are `Option<&str>` and its
                    // `children` is `Option<Vec<DomNode>>`. String-literal attrs
                    // (`label="Home"`, `active_class="is-active"`) are wrapped in
                    // `Some(..)` automatically so the long-standing `label=".."`
                    // API keeps working next to the children form
                    // (`<Link to="..">Text</Link>`); braced exprs pass through
                    // as-is so callers can also hand an `Option` value.
                    if component_name == "Link" {
                        let mut to_val: TokenStream2 = quote! {};
                        let mut label_val: TokenStream2 = quote! { None };
                        let mut class_val: TokenStream2 = quote! { None };
                        for attr in attributes {
                            let v = &attr.value;
                            let v_optional: TokenStream2 = match v {
                                Expr::Lit(_) => quote! { Some(#v) },
                                _ => quote! { #v },
                            };
                            if attr.key == "to" {
                                to_val = quote! { #v };
                            } else if attr.key == "label" {
                                label_val = v_optional;
                            } else if attr.key == "active_class" {
                                class_val = v_optional;
                            }
                        }
                        let children_val: TokenStream2 = if children.is_empty() {
                            quote! { None }
                        } else {
                            quote! { Some(vec![#(#children),*]) }
                        };
                        tokens.extend(quote! {
                            velo::Link(velo::LinkProps {
                                to: #to_val,
                                label: #label_val,
                                active_class: #class_val,
                                children: #children_val,
                            })
                        });
                        return;
                    }

                    // All other components use a generated `<Name>Props` struct.
                    // `#[component]` synthesizes `NameProps` with one field per
                    // parameter, so attributes are matched BY NAME and may appear
                    // in any order. Children (either nested nodes or a `children=`
                    // attribute) map to the `children` field to support
                    // `<Panel>{..}</Panel>` composition.
                    let props_ident =
                        syn::Ident::new(&format!("{}Props", component_name), proc_macro2::Span::call_site());

                    let mut children_field: Option<TokenStream2> = None;
                    let mut struct_fields: Vec<TokenStream2> = Vec::new();
                    for attr in attributes {
                        if attr.key == "children" {
                            children_field = Some(quote! { #attr.value });
                        } else {
                            let key = syn::Ident::new(&attr.key, proc_macro2::Span::call_site());
                            let val = &attr.value;
                            struct_fields.push(quote! { #key: #val });
                        }
                    }
                    let children_present = !children.is_empty();
                    if children_present || children_field.is_some() {
                        let children_expr = match children_field {
                            Some(c) => c,
                            None => quote! { vec![ #(#children),* ] },
                        };
                        struct_fields.push(quote! { children: #children_expr });
                    }

                    tokens.extend(quote! {
                        #component_ident(#props_ident {
                            #(#struct_fields,)*
                        })
                    });
                } else {
                    let mut setup_statements = Vec::new();

                    setup_statements.push(quote! {
                        let parent_node = velo::DomNode::element(#tag_name);
                    });

                    // Collect every class-related attribute so we can coordinate them
                    // through a single `reactive_classes` call. `class:name={ bool }`
                    // bindings and a plain `class="..."` both write the same `class`
                    // attribute; coordinating them in one registry is what stops them
                    // from clobbering each other.
                    let mut base_classes: Vec<&syn::Expr> = Vec::new();
                    let mut toggle_classes: Vec<(String, &syn::Expr)> = Vec::new();

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
                            toggle_classes.push((class_name, val));
                        } else if key == "class" {
                            // Plain class="..." attribute: gathered with any toggles.
                            base_classes.push(val);
                        } else if key.starts_with("style:") {
                            // Reactive inline style: style:color={ color }
                            let prop = key.strip_prefix("style:").unwrap().to_string();
                            setup_statements.push(quote! {
                                parent_node.reactive_style(#prop, move || velo::signal_value!(#val));
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
                                let bind_tmp = (#val).clone();
                                let bind_sig_1 = bind_tmp.clone();
                                let bind_sig_2 = bind_tmp.clone();
                                // Forward signal -> DOM: set live property AND reactive attribute.
                                {
                                    let bind_node = parent_node.clone();
                                    let bind_sig_1 = bind_sig_1.clone();
                                    parent_node.reactive_attribute(#field_name, move || {
                                        use wasm_bindgen::JsCast;
                                        let v = velo::signal_value!(bind_sig_1);
                                        let s = format!("{}", v);
                                        // Also set live DOM property for correct IME/reset behavior
                                        if let Ok(el) = bind_node.raw_node.clone().dyn_into::<web_sys::HtmlInputElement>() {
                                            let _ = el.set_value(&s);
                                        }
                                        s
                                    });
                                }
                                // Forward DOM -> signal (on input, read element and set signal).
                                {
                                    let bind_node = parent_node.clone();
                                    parent_node.on(#event_name_str, move |e: web_sys::Event| {
                                        use wasm_bindgen::JsCast;
                                        let target = e.target().expect("bind:value event has no target");
                                        let el = target.dyn_into::<web_sys::HtmlInputElement>().expect("bind:value requires an input/textarea/select element");
                                        bind_sig_2.set(el.value());
                                    });
                                }
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
                                let bind_tmp = (#val).clone();
                                let bind_sig_1 = bind_tmp.clone();
                                let bind_sig_2 = bind_tmp.clone();
                                // Forward signal -> DOM: set live property AND reactive attribute.
                                {
                                    let bind_node = parent_node.clone();
                                    let bind_sig_1 = bind_sig_1.clone();
                                    parent_node.reactive_attribute("checked", move || {
                                        use wasm_bindgen::JsCast;
                                        let checked = velo::signal_value!(bind_sig_1);
                                        let s = if checked { "checked" } else { "" };
                                        // Also set live DOM property for correct behavior
                                        if let Ok(el) = bind_node.raw_node.clone().dyn_into::<web_sys::HtmlInputElement>() {
                                            let _ = el.set_checked(checked);
                                        }
                                        s
                                    });
                                }
                                // Forward DOM -> signal (on change, read element and set signal).
                                {
                                    let bind_node = parent_node.clone();
                                    parent_node.on(#event_name_str, move |e: web_sys::Event| {
                                        use wasm_bindgen::JsCast;
                                        let target = e.target().expect("bind:checked event has no target");
                                        let el = target.dyn_into::<web_sys::HtmlInputElement>().expect("bind:checked requires a checkbox/radio input");
                                        bind_sig_2.set(el.checked());
                                    });
                                }
                            });
                        } else if key == "disabled"
                            || key == "checked"
                            || key == "selected"
                            || key == "readonly"
                        {
                            setup_statements.push(quote! {
                            let p_node = parent_node.clone();
                            // Use the top-level unified facade pathway
                            velo::create_effect({
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
                                parent_node.reactive_attribute(#key, move || format!("{}", velo::signal_value!(#val)));
                            });
                        }
                    }

                    // Coordinate class attributes gathered above. When the element
                    // carries any `class:` toggle, fold the whole class set into a
                    // single `reactive_classes` call (shared registry) so the base
                    // `class="..."` and the toggled classes stop overwriting one
                    // another. Static string-literal bases join the base; reactive
                    // `class={...}` expressions still use reactive_attribute.
                    if !toggle_classes.is_empty() {
                        let mut static_base: Vec<String> = Vec::new();
                        let mut reactive_base: Vec<&syn::Expr> = Vec::new();
                        for base in &base_classes {
                            if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = base {
                                static_base.push(s.value());
                            } else {
                                reactive_base.push(base);
                            }
                        }
                        let base_static = static_base.join(" ");

                        let toggle_calls: Vec<proc_macro2::TokenStream> = toggle_classes
                            .iter()
                            .map(|(name, val)| {
                                let name_lit = syn::LitStr::new(name, proc_macro2::Span::call_site());
                                quote! {
                                    (#name_lit, Box::new(move || velo::signal_value!(#val)) as Box<dyn FnMut() -> bool + 'static>),
                                }
                            })
                            .collect();

                        setup_statements.push(quote! {
                            parent_node.reactive_classes(#base_static, vec![ #(#toggle_calls)* ]);
                        });

                        for rb in reactive_base {
                            setup_statements.push(quote! {
                                parent_node.reactive_attribute("class", move || format!("{}", velo::signal_value!(#rb)));
                            });
                        }
                    } else {
                        for base in base_classes {
                            setup_statements.push(quote! {
                                parent_node.reactive_attribute("class", move || format!("{}", velo::signal_value!(#base)));
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
    let parsed_root = parse_macro_input!(input as VNode);
    TokenStream::from(parsed_root.to_token_stream())
}

/// `#[component]` turns a plain function into a Velo component and converts it
/// to the named-props form.
///
/// Velo components are callable with **named props** in any order. The macro
/// rewrites the function so that:
///
/// 1. A `<Name>Props` struct is generated with one `pub` field per parameter.
/// 2. The function now takes that struct as a single `props` argument and
///    destructures it into the original parameter names.
/// 3. The return type is rewritten to `velo::DomNode` so the body can end with
///    a `view! { ... }` tail expression (no explicit `-> DomNode` needed).
///
/// The `view!` macro matches JSX attributes to the struct fields by name, so
/// `<UserCard role="Admin" name="Ada" />` resolves correctly regardless of
/// order. A special `children` field receives child nodes, enabling
/// `<Panel>{..}</Panel>` composition.
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
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = match syn::parse::<syn::ItemFn>(item) {
        Ok(f) => f,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    if !attr.is_empty() {
        return TokenStream::from(
            syn::Error::new_spanned(
                &func.sig.ident,
                "Velo #[component] no longer takes a props-type argument: the <Name>Props struct is generated automatically from the parameter list",
            )
            .to_compile_error(),
        );
    }

    let fn_name = &func.sig.ident;
    let props_ident = syn::Ident::new(&format!("{}Props", fn_name), proc_macro2::Span::call_site());

    // Collect `(field_ident, field_type)` from each typed parameter.
    let mut fields: Vec<(syn::Ident, syn::Type)> = Vec::new();
    for input in &mut func.sig.inputs {
        let arg = match input {
            syn::FnArg::Typed(pt) => pt,
            syn::FnArg::Receiver(_) => {
                return TokenStream::from(
                    syn::Error::new_spanned(input, "Velo #[component] cannot take a `self` receiver")
                        .to_compile_error(),
                );
            }
        };
        let field_ident = match &*arg.pat {
            syn::Pat::Ident(pi) => pi.ident.clone(),
            _ => {
                return TokenStream::from(
                    syn::Error::new_spanned(
                        arg,
                        "Velo #[component] parameters must be plain named identifiers",
                    )
                    .to_compile_error(),
                );
            }
        };
        fields.push((field_ident, (*arg.ty).clone()));
    }

    let field_defs = fields.iter().map(|(id, ty)| quote! { pub #id: #ty });
    let props_struct = quote! {
        #[allow(non_snake_case)]
        pub struct #props_ident {
            #(#field_defs,)*
        }
    };

    // Destructure props back into the original parameter names inside the body.
    let field_names: Vec<_> = fields.iter().map(|(id, _)| id).collect();
    let destructure: syn::Stmt = syn::parse_quote! {
        let #props_ident { #(#field_names,)* } = props;
    };

    // Swap the parameter list for a single `props: <Name>Props` argument.
    func.sig.inputs.clear();
    func.sig.inputs.push(syn::parse_quote! { props: #props_ident });

    // Prepend the destructure statement, keeping the existing body as-is.
    let existing_stmts = std::mem::take(&mut func.block.stmts);
    func.block.stmts.push(destructure);
    func.block.stmts.extend(existing_stmts);

    // Force the return type to DomNode.
    func.sig.output = syn::ReturnType::Type(
        Default::default(),
        Box::new(syn::parse_quote!(velo::DomNode)),
    );

    TokenStream::from(quote! {
        #props_struct
        #func
    })
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
                velo::Route {
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

#[proc_macro_attribute]
pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Invoke as `#[route("/path")]`. For a `#[attr(...)]` proc-macro attribute,
    // rustc hands the macro only the tokens *inside* the parens, so `attr` is
    // just the path literal (no parentheses, no `=`).
    struct RouteAttr(syn::LitStr);
    impl syn::parse::Parse for RouteAttr {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            Ok(RouteAttr(input.parse()?))
        }
    }
    let RouteAttr(path_lit) = parse_macro_input!(attr as RouteAttr);
    let mut func = match syn::parse::<syn::ItemFn>(item) {
        Ok(f) => f,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };
    if !func.sig.inputs.is_empty() {
        return TokenStream::from(syn::Error::new_spanned(
            &func.sig.inputs,
            "Velo: #[route] components take no arguments — read params via FRouter::use_param",
        ).to_compile_error());
    }
    func.sig.output = syn::ReturnType::Type(Default::default(), Box::new(syn::parse_quote!(velo::DomNode)));
    let fn_ident = func.sig.ident.clone();
    TokenStream::from(quote! {
        #func
        velo::inventory::submit! {
            velo::RouteRegistration { path: #path_lit, component: #fn_ident }
        }
    })
}