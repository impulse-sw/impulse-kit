#![allow(missing_docs)]

use leptos::prelude::*;

/// How many items in accordion group can be opened simultaneously.
#[derive(Clone, Copy, PartialEq)]
pub enum AccordionType {
    /// Only one item at a time.
    Single,
    /// Several items.
    Multiple,
}

/// Accordion wrapper.
///
/// Provides context to all accordion items.
#[component]
pub fn Accordion(
    #[prop(into)] accordion_type: AccordionType,
    #[prop(default = false)] collapsible: bool,
    #[prop(optional)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let (open_items, set_open_items) = signal::<Vec<String>>(Vec::new());

    provide_context(AccordionContext {
        accordion_type,
        collapsible,
        open_items,
        set_open_items,
    });

    let class_str = format!(
        "{}{}",
        "w-full",
        class.map(|c| format!(" {}", c)).unwrap_or_default()
    );

    view! {
        <div class=class_str>
            {children()}
        </div>
    }
}

#[component]
pub fn AccordionItem(
    #[prop(into)] value: String,
    #[prop(optional)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    provide_context(AccordionItemContext { value });

    let class_str = format!(
        "{}{}",
        "border-b",
        class.map(|c| format!(" {}", c)).unwrap_or_default()
    );

    view! {
        <div class=class_str>
            {children()}
        </div>
    }
}

#[component]
pub fn AccordionTrigger(
    #[prop(optional)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let accordion_ctx = use_context::<AccordionContext>()
        .expect("AccordionTrigger must be used within an Accordion");
    let item_ctx1 = use_context::<AccordionItemContext>()
        .expect("AccordionTrigger must be used within an AccordionItem");
    let item_ctx2 = use_context::<AccordionItemContext>()
        .expect("AccordionTrigger must be used within an AccordionItem");

    let is_open = Memo::new(move |_| {
        accordion_ctx.open_items.get().contains(&item_ctx1.value)
    });

    let toggle_item = move |_| {
        let current_items = accordion_ctx.open_items.get();
        let item_value = item_ctx2.value.clone();

        match accordion_ctx.accordion_type {
            AccordionType::Single => {
                if current_items.contains(&item_value) {
                    if accordion_ctx.collapsible {
                        accordion_ctx.set_open_items.set(Vec::new());
                    }
                } else {
                    accordion_ctx.set_open_items.set(vec![item_value.clone()]);
                }
            }
            AccordionType::Multiple => {
                if current_items.contains(&item_value) {
                    let filtered: Vec<String> = current_items
                        .into_iter()
                        .filter(|item| item != &item_value)
                        .collect();
                    accordion_ctx.set_open_items.set(filtered);
                } else {
                    let mut new_items = current_items;
                    new_items.push(item_value.clone());
                    accordion_ctx.set_open_items.set(new_items);
                }
            }
        }
    };

    let class_str = format!(
        "{}{}",
        "flex flex-1 items-center justify-between py-4 font-medium transition-all hover:underline cursor-pointer [&[data-state=open]>svg]:rotate-180",
        class.map(|c| format!(" {}", c)).unwrap_or_default()
    );

    view! {
        <h3 class="flex">
            <button
                class=class_str
                data-state=move || if is_open.get() { "open" } else { "closed" }
                aria-expanded=move || is_open.get()
                on:click=toggle_item
                type="button"
            >
                <span class="text-left">
                    {children()}
                </span>
                <svg
                    class="h-4 w-4 shrink-0 transition-transform duration-200"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <path stroke-linecap="round" stroke-linejoin="round" d="M19 9l-7 7-7-7" />
                </svg>
            </button>
        </h3>
    }
}

#[component]
pub fn AccordionContent(
    #[prop(optional)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let accordion_ctx = use_context::<AccordionContext>()
        .expect("AccordionContent must be used within an Accordion");
    let item_ctx = use_context::<AccordionItemContext>()
        .expect("AccordionContent must be used within an AccordionItem");

    let is_open = Memo::new(move |_| {
        accordion_ctx.open_items.get().contains(&item_ctx.value)
    });

    let class_str = format!(
        "{}{}",
        "overflow-hidden text-sm transition-all data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down",
        class.map(|c| format!(" {}", c)).unwrap_or_default()
    );

    view! {
        <div
            class=class_str
            data-state=move || if is_open.get() { "open" } else { "closed" }
            style:display=move || if is_open.get() { "block" } else { "none" }
        >
            <div class="pb-4 pt-0">
                {children()}
            </div>
        </div>
    }
}

// Context types
#[derive(Clone, Copy)]
struct AccordionContext {
    accordion_type: AccordionType,
    collapsible: bool,
    open_items: ReadSignal<Vec<String>>,
    set_open_items: WriteSignal<Vec<String>>,
}

#[derive(Clone)]
struct AccordionItemContext {
    value: String,
}

// #[component]
// pub fn AccordionExample() -> impl IntoView {
//     view! {
//         <Accordion accordion_type=AccordionType::Single collapsible=true>
//             <AccordionItem value="item-1".to_string()>
//                 <AccordionTrigger>
//                     "Is it accessible?"
//                 </AccordionTrigger>
//                 <AccordionContent>
//                     "Yes. It adheres to the WAI-ARIA design pattern."
//                 </AccordionContent>
//             </AccordionItem>
//             <AccordionItem value="item-2".to_string()>
//                 <AccordionTrigger>
//                     "Is it styled?"
//                 </AccordionTrigger>
//                 <AccordionContent>
//                     "Yes. It comes with default styles that matches the other components' aesthetic."
//                 </AccordionContent>
//             </AccordionItem>
//             <AccordionItem value="item-3".to_string()>
//                 <AccordionTrigger>
//                     "Is it animated?"
//                 </AccordionTrigger>
//                 <AccordionContent>
//                     "Yes. It's animated by default, but you can disable it if you prefer."
//                 </AccordionContent>
//             </AccordionItem>
//         </Accordion>
//     }
// }
