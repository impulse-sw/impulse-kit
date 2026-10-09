#![allow(missing_docs, dead_code)]

//! Боковая навигация: корневые разделы приложения слева от содержимого.
//!
//! Зачем отдельно от [`sidebar`](crate::sidebar), который тоже боковой: тот —
//! выдвижная панель с произвольным содержимым (`SidebarProvider` ставит себе
//! `min-h-screen` и открывается триггером). Этот — постоянная навигация по
//! разделам: он не открывается и не закрывается, живёт внутри уже
//! расставленной раскладки и знает про активный раздел.
//!
//! **Почему не вкладки в шапке.** Вкладки — горизонтальный список, и места у
//! него столько, сколько осталось от всего прочего в строке. Четыре подписи
//! туда влезают, шесть — уже нет: они либо переносятся второй строкой, либо
//! отнимают место у того, что в шапке и так тесно (название открытого
//! документа, показания, кнопки). Вертикальный список растёт вниз, где места
//! столько же, сколько у содержимого, — поэтому число разделов перестаёт быть
//! вопросом вёрстки.
//!
//! **Подписи по ширине окна, а не по выбору.** Ниже `lg` панель сжимается в
//! «рейку»: значок и подпись под ним мельче. Это не переключатель, за который
//! надо помнить, а свойство окна — и поэтому состояния, которое надо где-то
//! хранить и восстанавливать между запусками, здесь нет вовсе. Приложению, у
//! которого ответ другой, [`SideNavLabels`] говорит его один раз на всю панель,
//! а не классом на каждый пункт: [`cn`] склеивает классы и не сливает их, так
//! что `w-56`, переданный поверх `w-16`, встал бы рядом с ним, и кто из них
//! победит, решал бы порядок правил в собранной таблице стилей.
//!
//! ```ignore
//! view! {
//!   <SideNav label="Разделы">
//!     <SideNavItem
//!       icon=icondata::LuFileText
//!       label="Документы"
//!       active=Signal::derive(move || at.get() == Destination::Documents)
//!       on:click=move |_| at.set(Destination::Documents)
//!     />
//!   </SideNav>
//! }
//! ```

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

use crate::icon::Icon;

/// Показывать ли подписи рядом со значками.
#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub enum SideNavLabels {
  /// Подписи с `lg`, ниже — рейка: значок и подпись под ним мельче.
  #[default]
  Responsive,
  /// Подписи всегда: приложение, которое не бывает узким.
  Always,
  /// Только значки: разделов много, а подписи у них длинные.
  Never,
}

impl SideNavLabels {
  /// Ширина панели.
  pub fn nav_class(self) -> &'static str {
    match self {
      Self::Responsive => "w-16 lg:w-56",
      Self::Always => "w-56",
      Self::Never => "w-16",
    }
  }

  /// Как пункт раскладывает значок и подпись.
  pub fn item_class(self) -> &'static str {
    match self {
      Self::Responsive => "flex-col justify-center gap-1 px-1 lg:flex-row lg:justify-start lg:gap-3 lg:px-3",
      Self::Always => "flex-row justify-start gap-3 px-3",
      Self::Never => "flex-col justify-center gap-1 px-1",
    }
  }

  /// Какой подпись величины — и есть ли она вообще.
  pub fn label_class(self) -> Option<&'static str> {
    match self {
      Self::Responsive => Some("truncate text-[10px] leading-tight lg:text-sm"),
      Self::Always => Some("truncate text-sm"),
      Self::Never => None,
    }
  }
}

#[derive(Clone, Copy, Default)]
struct SideNavContext {
  labels: SideNavLabels,
}

/// Панель разделов: вертикальный список во всю высоту отведённого ей места.
///
/// Высоту берёт от родителя (`h-full`), а не от экрана: над ней обычно стоит
/// шапка приложения, и панель, отмеренная экраном, уехала бы под нижний край
/// на её высоту.
#[component]
pub fn SideNav(
  /// Чем панель называется для экранного читателя. Пусто — названия нет, и
  /// `aria-label` не выводится: пустая подпись хуже отсутствующей, она
  /// объявляет имя и не даёт его.
  #[prop(into, optional)]
  label: String,
  #[prop(optional)] labels: SideNavLabels,
  #[prop(into, optional)] class: String,
  children: Children,
) -> impl IntoView {
  provide_context(SideNavContext { labels });

  view! {
    <nav
      data-slot="side-nav"
      aria-label=(!label.is_empty()).then_some(label)
      class=cn(
        &[
          "flex h-full shrink-0 flex-col gap-1 overflow-y-auto border-r border-border p-2",
          labels.nav_class(),
          class.as_str(),
        ],
      )
    >
      {children()}
    </nav>
  }
}

/// Пункт панели: значок, подпись и то, он ли сейчас открыт.
///
/// Кнопка, а не ссылка: раздел — состояние приложения, а не адрес. Там, где
/// адрес у него есть, ссылку рисует маршрутизатор, а не навигация.
///
/// Подпись при этом остаётся и в `title`: в рейке её видно мельком, а под
/// указателем нужна целиком — и она же отвечает за имя кнопки, когда подписи
/// нет совсем.
#[component]
pub fn SideNavItem(
  #[prop(into)] icon: icondata::Icon,
  #[prop(into)] label: String,
  /// Открыт ли этот раздел сейчас.
  ///
  /// Обязательно: панель навигации, которая не говорит, где человек стоит, —
  /// это список ссылок, а не навигация.
  #[prop(into)]
  active: Signal<bool>,
  #[prop(into, optional)] class: String,
) -> impl IntoView {
  let ctx = use_context::<SideNavContext>().expect("SideNavItem живёт внутри SideNav");
  let text = label.clone();

  view! {
    <button
      type="button"
      data-slot="side-nav-item"
      data-active=move || active.get().then_some("true")
      aria-current=move || active.get().then_some("page")
      title=label
      class=move || {
        cn(
          &[
            "flex cursor-pointer items-center rounded-md py-2 transition-colors",
            ctx.labels.item_class(),
            if active.get() {
              "bg-secondary text-secondary-foreground"
            } else {
              "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
            },
            class.as_str(),
          ],
        )
      }
    >
      <Icon class="size-5 shrink-0" icon=icon />
      {ctx
        .labels
        .label_class()
        .map(|c| view! { <span class=c>{text}</span> })}
    </button>
  }
}
