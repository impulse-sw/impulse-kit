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
//! **Свёрнут по умолчанию, разворачивается под указателем.** Рейка шириной со
//! значок — это всё, что нужно, чтобы перейти в раздел, который человек узнаёт
//! по значку; подписи нужны, пока он выбирает. Поэтому панель стоит рейкой и
//! раскрывается, когда на неё навели — или когда внутри неё что-то получило
//! фокус, иначе до подписей было бы не добраться с клавиатуры.
//!
//! Раскрывается она **поверх содержимого**, а не расталкивая его: место под
//! панель занято всегда одно (`w-16`), и наведение не перекладывает страницу.
//! Текст, перетекающий под указателем, читать невозможно, а в редакторе это
//! ещё и переносит строку под курсором.
//!
//! Состояния у этого нет — ни сигнала, ни ключа в хранилище: `:hover` и
//! `:focus-within` отвечают на вопрос «нужны ли сейчас подписи» точнее, чем
//! переключатель, за который надо помнить.
//!
//! **Подписи у своего содержимого.** Панель — `group`, поэтому всё, что
//! приложение в неё кладёт, разворачивается вместе с ней: [`SideNavLabel`]
//! прячет текст ровно по тому же правилу, что и подписи разделов. Приложение,
//! которому нужно своё — показание, поле, кнопка с текстом, — пишет
//! `group-hover:` на своих классах и получает то же поведение.
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
//!     <SideNavFooter>
//!       <ThemeToggle />
//!     </SideNavFooter>
//!   </SideNav>
//! }
//! ```

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

use crate::icon::Icon;

/// Когда у значков видны подписи.
#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub enum SideNavLabels {
  /// Рейка, разворачивающаяся под указателем и по фокусу внутри.
  #[default]
  OnHover,
  /// Подписи всегда: приложение, которому ширины не жалко.
  Always,
  /// Только значки: разделов много, а подписи у них длинные.
  Never,
}

impl SideNavLabels {
  /// Место, которое панель занимает в раскладке.
  ///
  /// У разворачивающейся — ширина рейки: разворот рисуется поверх, и места он
  /// не просит.
  pub fn nav_class(self) -> &'static str {
    match self {
      Self::OnHover | Self::Never => "w-16",
      Self::Always => "w-56",
    }
  }

  /// Сама панель: ширина и то, как она меняется.
  pub fn panel_class(self) -> &'static str {
    match self {
      Self::OnHover => {
        "w-16 transition-[width] duration-150 ease-out group-hover:w-56 group-hover:shadow-xl \
         group-focus-within:w-56 group-focus-within:shadow-xl"
      }
      Self::Always => "w-56",
      Self::Never => "w-16",
    }
  }

  /// Подпись: видна ли она и когда.
  ///
  /// У свёрнутой панели подпись не удаляется, а гаснет: она остаётся в разметке
  /// — её читает экранный читатель, — и ширину значка не двигает, потому что
  /// панель обрезает всё, что шире её.
  pub fn label_class(self) -> Option<&'static str> {
    match self {
      Self::OnHover => Some(
        "truncate whitespace-nowrap text-sm opacity-0 transition-opacity duration-150 \
         group-hover:opacity-100 group-focus-within:opacity-100",
      ),
      Self::Always => Some("truncate whitespace-nowrap text-sm"),
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
/// Высоту берёт от родителя (`h-full`), а не от экрана: над ней может стоять
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
      class=cn(&["group relative h-full shrink-0", labels.nav_class(), class.as_str()])
    >
      <div
        data-slot="side-nav-panel"
        class=cn(
          &[
            "absolute inset-y-0 left-0 z-30 flex flex-col gap-1 overflow-y-auto overflow-x-hidden \
             border-r border-border bg-background p-2",
            labels.panel_class(),
          ],
        )
      >
        {children()}
      </div>
    </nav>
  }
}

/// Подпись внутри панели: видна, пока панель развёрнута.
///
/// Тем же правилом, что и подписи разделов, — потому что это одно и то же
/// правило, а не похожее: приложение, положившее в панель своё, не должно
/// угадывать, по какому событию она раскрывается.
///
/// Снаружи панели — обычная подпись, видимая всегда. Это не снисходительность
/// к ошибке, а то, ради чего компонент и нужен: одно и то же показание
/// приложения стоит и в панели, и в шапке телефона, и прятать его там, где
/// прятать нечего и не за что, было бы просто ошибкой.
#[component]
pub fn SideNavLabel(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  let ctx = use_context::<SideNavContext>().unwrap_or(SideNavContext {
    labels: SideNavLabels::Always,
  });

  view! {
    {ctx
      .labels
      .label_class()
      .map(|c| {
        view! {
          <span data-slot="side-nav-label" class=cn(&[c, class.as_str()])>
            {children()}
          </span>
        }
      })}
  }
}

/// Низ панели: то, что прижато к нижнему краю.
///
/// Показания приложения, переключатель темы, выход — всё, что относится не к
/// разделу, а к тому, кто в приложении. Внизу потому, что разделы читают
/// сверху вниз, а это не раздел.
#[component]
pub fn SideNavFooter(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
  view! {
    <div
      data-slot="side-nav-footer"
      class=cn(&["mt-auto flex flex-col gap-1 border-t border-border pt-2", class.as_str()])
    >
      {children()}
    </div>
  }
}

/// Пункт панели: значок, подпись и то, он ли сейчас открыт.
///
/// Кнопка, а не ссылка: раздел — состояние приложения, а не адрес. Там, где
/// адрес у него есть, ссылку рисует маршрутизатор, а не навигация.
///
/// Подпись при этом остаётся и в `title`: у свёрнутой панели её не видно, а под
/// указателем нужна сразу — и она же отвечает за имя кнопки, когда подписи нет
/// совсем.
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
            "flex w-full shrink-0 cursor-pointer items-center gap-3 rounded-md px-3 py-2 transition-colors",
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
      <SideNavLabel>{text}</SideNavLabel>
    </button>
  }
}
