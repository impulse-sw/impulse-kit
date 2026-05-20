#![allow(missing_docs, dead_code)]

use impulse_ui_kit::utils::cn;
use leptos::prelude::*;

pub const COOKIE_CONSENT_STORAGE_KEY: &str = "cookie_consent";
pub const COOKIE_CONSENT_ACCEPTED: &str = "accepted";
pub const COOKIE_CONSENT_DECLINED: &str = "declined";

/// Всплывающий баннер согласия с использованием cookies.
///
/// На десктопе — справа снизу (фиксированный блок), на мобильных — полоса
/// по всей ширине внизу экрана. Состояние сохраняется в `localStorage`.
///
/// Совместим с GDPR и ФЗ № 152 «О персональных данных».
///
/// Содержимое баннера скрыто до завершения гидратации: `mounted` остаётся
/// `false` на SSR и в начальном клиентском рендере, после чего `Effect` ставит
/// его в `true`. Благодаря этому SSR-разметка и начальный WASM-рендер
/// идентичны — tachys не паникует с «unreachable code».
#[component]
pub fn CookieConsent(
  /// Ключ `localStorage`. По умолчанию `"cookie_consent"`.
  #[prop(into, optional)]
  storage_key: Option<String>,
  /// Заголовок баннера.
  #[prop(into, optional)]
  title: Option<String>,
  /// Описательный текст.
  #[prop(into, optional)]
  description: Option<String>,
  /// Текст кнопки принятия.
  #[prop(into, optional)]
  accept_label: Option<String>,
  /// Текст кнопки отказа.
  #[prop(into, optional)]
  decline_label: Option<String>,
  /// Ссылка на политику конфиденциальности.
  #[prop(into, optional)]
  policy_href: Option<String>,
  /// Текст ссылки на политику. По умолчанию «Подробнее».
  #[prop(into, optional)]
  policy_label: Option<String>,
  /// Дополнительные CSS-классы.
  #[prop(into, optional)]
  class: String,
) -> impl IntoView {
  use codee::string::FromToStringCodec;
  use leptos_use::storage::use_local_storage;

  // false на SSR и при начальном клиентском рендере; становится true после гидратации
  let (mounted, set_mounted) = signal(false);
  Effect::new(move |_| set_mounted.set(true));

  let key = storage_key.unwrap_or_else(|| COOKIE_CONSENT_STORAGE_KEY.to_string());
  let title = StoredValue::new(title.unwrap_or_else(|| "Файлы cookie".to_string()));
  let description = StoredValue::new(
    description.unwrap_or_else(|| {
      "Мы используем файлы cookie для аналитики и улучшения работы сайта.".to_string()
    }),
  );
  let accept_label = StoredValue::new(accept_label.unwrap_or_else(|| "Принять".to_string()));
  let decline_label = StoredValue::new(decline_label.unwrap_or_else(|| "Отказаться".to_string()));
  let policy_href = StoredValue::new(policy_href);
  let policy_label = StoredValue::new(policy_label);

  let (consent, set_consent, _) = use_local_storage::<String, FromToStringCodec>(key.as_str());

  let is_visible = Signal::derive(move || mounted.get() && consent.get().is_empty());

  let accept = move |_| set_consent.set(COOKIE_CONSENT_ACCEPTED.to_string());
  let decline = move |_| set_consent.set(COOKIE_CONSENT_DECLINED.to_string());

  view! {
    <div
      data-slot="cookie-consent"
      data-state=move || if is_visible.get() { "open" } else { "closed" }
      class=move || cn(&[
        "fixed z-50",
        "bottom-0 inset-x-0 sm:bottom-6 sm:inset-x-auto sm:right-6 sm:w-88",
        "bg-background border-t sm:border sm:rounded-xl border-border shadow-lg",
        "p-4 sm:p-5",
        "data-[state=open]:animate-in data-[state=closed]:animate-out",
        "data-[state=open]:slide-in-from-bottom data-[state=closed]:slide-out-to-bottom",
        "data-[state=open]:fade-in-0 data-[state=closed]:fade-out-0",
        "data-[state=closed]:pointer-events-none data-[state=closed]:invisible",
        "duration-300",
        class.as_str(),
      ])
    >
      <Show when=move || mounted.get()>
        <div class="flex flex-col gap-3">
          <div class="space-y-1">
            <p class="text-sm font-semibold">{move || title.get_value()}</p>
            <p class="text-xs text-muted-foreground leading-relaxed">
              {move || {
                let desc = description.get_value();
                let href = policy_href.get_value();
                if let Some(href) = href {
                  let label = policy_label.get_value().unwrap_or_else(|| "Подробнее".to_string());
                  view! {
                    <span>
                      {desc}
                      " "
                      <a href=href class="underline hover:text-foreground transition-colors" target="_blank">
                        {label}
                      </a>
                    </span>
                  }
                  .into_any()
                } else {
                  view! { <span>{desc}</span> }.into_any()
                }
              }}
            </p>
          </div>
          <div class="flex gap-2 justify-end flex-wrap">
            <button
              type="button"
              on:click=decline
              class="inline-flex items-center justify-center rounded-md text-xs font-medium h-8 px-3 border border-input bg-background hover:bg-accent hover:text-accent-foreground transition-colors"
            >
              {move || decline_label.get_value()}
            </button>
            <button
              type="button"
              on:click=accept
              class="inline-flex items-center justify-center rounded-md text-xs font-medium h-8 px-3 bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
            >
              {move || accept_label.get_value()}
            </button>
          </div>
        </div>
      </Show>
    </div>
  }
}
