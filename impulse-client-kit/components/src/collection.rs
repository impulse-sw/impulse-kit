#![allow(missing_docs, dead_code)]

//! Набор однородных сущностей на экране: доски, документы, папки, теги.
//!
//! # Почему раскладка — не вёрстка, а два разных вида
//!
//! Плитка и список отличаются не числом колонок. В плитке у карточки есть
//! площадь, и правильно потратить её на подробности: выдержку, счётчики, теги,
//! дату — всё, что помогает узнать нужное среди двух десятков похожих. В строке
//! площади нет ни пикселя лишнего, и та же карточка, пережатая до одной
//! колонки, — это карточка с дырой справа и обрезанным текстом слева.
//!
//! Поэтому элемент набора рисуется **двумя** видами, а не одним с разными
//! классами. Какой из них сейчас — говорит [`use_collection_layout`], и
//! приложение пишет на него `match`:
//!
//! ```ignore
//! #[component]
//! fn DocumentItem(doc: DocumentDto) -> impl IntoView {
//!   match use_collection_layout().get() {
//!     CollectionLayout::Grid => view! { <DocumentCard doc=doc /> }.into_any(),
//!     CollectionLayout::List => view! { <DocumentRow doc=doc /> }.into_any(),
//!   }
//! }
//! ```
//!
//! Снаружи набора вид один — тот, что назван умолчанием: показание, стоящее и
//! в наборе, и в шапке, не должно гадать, где оно.
//!
//! # Почему выбор помнится, а не живёт в сигнале приложения
//!
//! «Списком или плиткой» — это не состояние экрана, а привычка человека, и
//! пережить она обязана и уход на другую страницу, и перезапуск. Поэтому выбор
//! лежит в `localStorage` под ключом набора, а приложению об этом знать не
//! нужно: оно объявляет набор, а не заводит ему переключатель.
//!
//! Ключ — имя набора, а не адрес страницы: один и тот же набор на двух экранах
//! — это один набор, и переключив его в одном месте, человек переключил его.

use impulse_client_kit::utils::cn;
use leptos::prelude::*;

use crate::empty::{Empty, EmptyDescription, EmptyHeader, EmptyTitle};
use crate::layout::{Container, ContainerSize, Gap, Grid, GridCols};
use crate::skeleton::Skeleton;

/// Как набор разложен.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CollectionLayout {
  /// Плитка: у карточки есть площадь, и её тратят на подробности.
  #[default]
  Grid,
  /// Список: строка во всю полосу, подробности — только те, что в неё влезают.
  List,
}

impl CollectionLayout {
  /// Как это записано в хранилище. Пустая строка — «ничего не выбирали», то
  /// есть умолчание набора, а не плитка: набор, объявленный списком, иначе
  /// становился бы плиткой у каждого, кто переключателя не касался.
  pub fn as_storage(self) -> &'static str {
    match self {
      CollectionLayout::Grid => "grid",
      CollectionLayout::List => "list",
    }
  }

  pub fn from_storage(value: &str) -> Option<Self> {
    match value {
      "grid" => Some(CollectionLayout::Grid),
      "list" => Some(CollectionLayout::List),
      _ => None,
    }
  }

  /// Другая из двух.
  pub fn other(self) -> Self {
    match self {
      CollectionLayout::Grid => CollectionLayout::List,
      CollectionLayout::List => CollectionLayout::Grid,
    }
  }
}

/// Ключ, под которым помнится выбор раскладки набора `name`.
pub fn collection_layout_key(name: &str) -> String {
  format!("collection-layout:{name}")
}

#[derive(Clone, Copy)]
struct CollectionContext {
  layout: Signal<CollectionLayout>,
}

/// Раскладка набора, **внутри которого** это нарисовано.
///
/// Снаружи любого набора — умолчание ([`CollectionLayout::Grid`]): карточка,
/// нарисованная вне набора, рисуется как карточка.
///
/// Отсюда правило, на котором уже спотыкались: спрашивать надо **там, где
/// рисуют элемент**, а не в теле страницы, которая набор содержит. Контекст
/// поднимает сам [`CollectionView`], и для страницы он ещё не существует —
/// функция отдаст ей умолчание, молча и навсегда, а переключатель будет
/// исправно переключать то, чего элементы не читают.
///
/// ```ignore
/// // Так — да: замыкание зовёт набор, когда рисует детей.
/// <DocumentCollection len=len>
///   {move || {
///     let layout = use_collection_layout();
///     …
///   }}
/// </DocumentCollection>
///
/// // Так — нет: здесь набора ещё нет, и раскладка всегда умолчание.
/// let layout = use_collection_layout();
/// view! { <DocumentCollection len=len>…</DocumentCollection> }
/// ```
pub fn use_collection_layout() -> Signal<CollectionLayout> {
  use_context::<CollectionContext>()
    .map(|ctx| ctx.layout)
    .unwrap_or_else(|| Signal::stored(CollectionLayout::default()))
}

/// Сколько скелетов показать, пока идёт первая загрузка.
///
/// Разное число для плитки и списка — не кокетство: скелет обещает, сколько
/// примерно приедет, и шесть полосок во всю ширину обещают экран списка, а
/// шесть плиток — две строки сетки.
/// Промежуток между элементами при этой раскладке.
///
/// Объявленный промежуток — про **плитку**: у карточки есть площадь, и воздух
/// между карточками отделяет одну от другой. У строки площади нет, и отделяет
/// её не воздух, а линия снизу — поэтому список берёт волосок, что бы набор ни
/// объявил. Шестнадцать пикселей между строками рассыпали перечень на карточки,
/// и строки переставали читаться строками (IUX-24).
///
/// Не настройка: промежуток списка — свойство списка, и объявлять его каждому
/// набору значило бы разрешить им разойтись. Кому нужен волосок у плитки, тот
/// называет его обычным путём.
fn gap_for(layout: CollectionLayout, declared: Gap) -> Gap {
  match layout {
    CollectionLayout::List => Gap::Hair,
    CollectionLayout::Grid => declared,
  }
}

fn skeletons(layout: CollectionLayout) -> usize {
  match layout {
    CollectionLayout::Grid => 6,
    CollectionLayout::List => 4,
  }
}

/// Подписи переключателя, для приложения не на английском.
#[derive(Clone)]
pub struct CollectionLabels {
  pub grid: String,
  pub list: String,
}

impl Default for CollectionLabels {
  fn default() -> Self {
    Self {
      grid: "Tiles".into(),
      list: "List".into(),
    }
  }
}

/// Набор: заголовок с переключателем, сетка или список, скелет и пустое
/// состояние.
///
/// Дети — элементы набора, их пишет приложение (обычно `For` по своим данным).
/// Сколько их — набор не считает сам: у `ChildrenFn` этого не спросить, а
/// пустой набор рисуется не сеткой, а приглашением.
#[component]
pub fn CollectionView(
  /// Имя набора: под ним помнится выбранная раскладка.
  #[prop(into)]
  name: String,
  /// Сколько элементов в наборе.
  #[prop(into)]
  len: Signal<usize>,
  /// Идёт ли первая загрузка: до ответа показывается скелет.
  #[prop(optional, into)]
  loading: Signal<bool>,
  /// Раскладка, с которой набор начинает, пока человек не выбрал другую.
  #[prop(optional)]
  default_layout: CollectionLayout,
  /// Показывать ли переключатель. Без него набор рисуется умолчанием и не
  /// спрашивает: у набора из трёх строк выбирать нечего.
  #[prop(optional)]
  switchable: bool,
  /// До скольких колонок разворачивается плитка. Точки останова — решение
  /// системы: приложение называет предел.
  #[prop(optional)]
  columns: CollectionColumns,
  #[prop(optional)] width: ContainerSize,
  #[prop(optional)] gap: Gap,
  /// Что показать, когда показывать нечего.
  #[prop(optional, into)]
  empty_title: String,
  #[prop(optional, into)] empty_hint: String,
  #[prop(optional)] labels: CollectionLabels,
  /// Своё в строке заголовка: кнопка «создать», фильтр, что угодно. Встаёт
  /// слева от переключателя.
  #[prop(into, optional)]
  toolbar: Option<ViewFn>,
  children: ChildrenFn,
) -> impl IntoView {
  let chosen = RwSignal::new(None::<CollectionLayout>);
  let key = StoredValue::new(collection_layout_key(&name));

  // Прочитанное — эффектом, а не при сборке вида. На гидратации разметка уже
  // пришла с сервера, и вид, собранный из хранилища сразу, разошёлся бы с ней:
  // гидратация принимает узлы, а не перерисовывает их, и расхождение осталось
  // бы на экране до первого изменения сигнала.
  Effect::new(move |previous: Option<()>| {
    if previous.is_some() {
      return;
    }
    if let Some(stored) = read_layout(&key.get_value()) {
      chosen.set(Some(stored));
    }
  });

  let layout = Signal::derive(move || chosen.get().unwrap_or(default_layout));
  provide_context(CollectionContext { layout });

  let pick = move |next: CollectionLayout| {
    chosen.set(Some(next));
    write_layout(&key.get_value(), next);
  };

  let labels = StoredValue::new(labels);
  let children = StoredValue::new(children);
  // Через `StoredValue`, чтобы замыкание вида осталось `FnMut`: вид
  // перерисовывается на каждое изменение раскладки, а замыкание, из которого
  // что-то вынесли, зовётся один раз.
  let empty = StoredValue::new((empty_title, empty_hint));
  let has_toolbar = toolbar.is_some();
  let toolbar = StoredValue::new(toolbar);

  view! {
    <Container size=width class="py-4">
      <Show when=move || switchable || has_toolbar>
        <div data-slot="collection-bar" class="mb-3 flex items-center justify-end gap-2">
          {move || toolbar.with_value(|toolbar| toolbar.as_ref().map(ViewFn::run))}
          <Show when=move || switchable>
            <div
              role="group"
              class="inline-flex items-center rounded-md border border-border p-0.5"
            >
              <LayoutButton
                layout=CollectionLayout::Grid
                current=layout
                label=labels.with_value(|l| l.grid.clone())
                on_pick=Callback::new(pick)
              />
              <LayoutButton
                layout=CollectionLayout::List
                current=layout
                label=labels.with_value(|l| l.list.clone())
                on_pick=Callback::new(pick)
              />
            </div>
          </Show>
        </div>
      </Show>
      {move || {
        if loading.get() {
          let layout = layout.get();
          return view! {
            <Grid cols=GridCols::One gap=gap_for(layout, gap) class=columns.class(layout)>
              {(0..skeletons(layout))
                .map(|_| {
                  let height = match layout {
                    CollectionLayout::Grid => "h-24 w-full rounded-lg",
                    CollectionLayout::List => "h-12 w-full rounded-md",
                  };
                  view! { <Skeleton class=height /> }
                })
                .collect_view()}
            </Grid>
          }
            .into_any();
        }
        if len.get() == 0 {
          // Объявленного пустого состояния может и не быть: тогда набор просто
          // пуст, и выдумывать за приложение текст незачем.
          let (title, hint) = empty.get_value();
          if title.is_empty() {
            return ().into_any();
          }
          return view! {
            <Empty>
              <EmptyHeader>
                <EmptyTitle>{title}</EmptyTitle>
                <EmptyDescription>{hint}</EmptyDescription>
              </EmptyHeader>
            </Empty>
          }
            .into_any();
        }
        view! {
          <Grid cols=GridCols::One gap=gap_for(layout.get(), gap) class=columns.class(layout.get())>
            {children.with_value(|c| c())}
          </Grid>
        }
          .into_any()
      }}
    </Container>
  }
}

/// Элемент набора, раскрывающийся **на своём месте**.
///
/// Подробности появляются прямо под своей строкой, а не внизу списка. Это не
/// вкус: открытое внизу человек не находит. Список шаблонов в Представе
/// открывал редактор под собой целиком, и на двух десятках шаблонов нажатие
/// выглядело как «ничего не произошло» — открылось оно экраном ниже, вне поля
/// зрения.
///
/// Раскрытость приходит сигналом, а не живёт внутри, и это главное в подписи.
/// Набор, у которого раскрыт один элемент, держит не «раскрыт ли я» у каждого,
/// а «который раскрыт» у себя: два открытых редактора одного и того же — это
/// два черновика, и второй затирает первый.
///
/// ```ignore
/// <CollectionDisclosure
///   open=Signal::derive(move || editing.get().as_deref() == Some(name))
///   on_toggle=Callback::new(move |()| toggle(name.clone()))
///   label="Показать шаблон"
///   summary=move || view! { <span>{name.clone()}</span> }
/// >
///   <TemplateEditor name=name />
/// </CollectionDisclosure>
/// ```
#[component]
pub fn CollectionDisclosure(
  /// Раскрыт ли элемент сейчас.
  #[prop(into)]
  open: Signal<bool>,
  /// Нажали на значок раскрытия. Зовётся и приложением — строку обычно
  /// открывают ещё и щелчком по названию.
  on_toggle: Callback<()>,
  /// Чем значок называется экранному читателю.
  #[prop(into, optional)]
  label: String,
  #[prop(into, optional)] class: String,
  /// Всегда видимая строка: название, показания, кнопки элемента.
  #[prop(into)]
  summary: ViewFn,
  /// Подробности. Пока закрыто — не рисуются вовсе: не нарисованное не нужно
  /// ни убирать из обхода табуляцией, ни прятать от экранного читателя, а
  /// редактор на пол-экрана в каждой из двадцати строк — это двадцать
  /// редакторов.
  children: ChildrenFn,
) -> impl IntoView {
  let state = move || if open.get() { "open" } else { "closed" };
  let children = StoredValue::new(children);

  view! {
    <div
      data-slot="collection-disclosure"
      data-state=state
      class=cn(&["rounded-md border border-border", class.as_str()])
    >
      <div class="flex flex-wrap items-center gap-2 px-3 py-2">
        <button
          type="button"
          data-slot="collection-disclosure-trigger"
          // Кнопка, а не вся строка: в строке живут свои кнопки элемента —
          // «удалить», «копировать», — и кнопка внутри кнопки это разметка,
          // которую браузеры чинят каждый по-своему.
          aria-expanded=move || open.get().to_string()
          title=label.clone()
          aria-label=label
          class="grid size-6 shrink-0 cursor-pointer place-items-center rounded-sm text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
          on:click=move |_| on_toggle.run(())
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class=move || {
              cn(&["size-4 transition-transform duration-150", if open.get() { "rotate-90" } else { "" }])
            }
            aria-hidden="true"
          >
            <path d="m9 18 6-6-6-6" />
          </svg>
        </button>
        {summary.run()}
      </div>
      <Show when=move || open.get()>
        <div data-slot="collection-disclosure-detail" class="border-t border-border p-3">
          {children.with_value(|c| c())}
        </div>
      </Show>
    </div>
  }
}

/// До скольких колонок разворачивается плитка.
///
/// Предел, а не количество: на телефоне колонка всегда одна, и объявлять это
/// отдельно значило бы объявлять то, что и так известно. В списке колонка одна
/// на любой ширине — это и есть список.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CollectionColumns(pub u8);

impl Default for CollectionColumns {
  fn default() -> Self {
    Self(2)
  }
}

impl CollectionColumns {
  /// Классы колонок для раскладки.
  ///
  /// Статические строки, а не `format!`: Tailwind собирает CSS, читая
  /// исходники, и класса, которого нет в тексте, в сборке не будет —
  /// `grid-cols-4`, составленный в рантайме, не резолвится ни во что.
  pub fn class(self, layout: CollectionLayout) -> &'static str {
    match layout {
      CollectionLayout::List => "",
      CollectionLayout::Grid => match self.0 {
        0 | 1 => "",
        2 => "sm:grid-cols-2",
        3 => "sm:grid-cols-2 lg:grid-cols-3",
        4 => "sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4",
        5 => "sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5",
        _ => "sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6",
      },
    }
  }
}

/// Одна кнопка переключателя.
#[component]
fn LayoutButton(
  layout: CollectionLayout,
  current: Signal<CollectionLayout>,
  label: String,
  on_pick: Callback<CollectionLayout>,
) -> impl IntoView {
  let active = move || current.get() == layout;
  view! {
    <button
      type="button"
      data-slot="collection-layout"
      data-active=move || active().then_some("true")
      aria-pressed=move || active().to_string()
      title=label.clone()
      aria-label=label
      class=move || {
        cn(
          &[
            "grid size-7 cursor-pointer place-items-center rounded-sm transition-colors",
            if active() {
              "bg-secondary text-secondary-foreground"
            } else {
              "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
            },
          ],
        )
      }
      on:click=move |_| on_pick.run(layout)
    >
      <LayoutGlyph layout=layout />
    </button>
  }
}

/// Значок раскладки.
///
/// Свой, а не из набора значков: `icondata` подключается приложением, а кит
/// обходится без него — и две фигуры в 24 пикселя не повод требовать набор.
#[component]
fn LayoutGlyph(layout: CollectionLayout) -> impl IntoView {
  view! {
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      class="size-4"
      aria-hidden="true"
    >
      {match layout {
        CollectionLayout::Grid => {
          view! {
            <>
              <rect width="7" height="7" x="3" y="3" rx="1" />
              <rect width="7" height="7" x="14" y="3" rx="1" />
              <rect width="7" height="7" x="3" y="14" rx="1" />
              <rect width="7" height="7" x="14" y="14" rx="1" />
            </>
          }
            .into_any()
        }
        CollectionLayout::List => {
          view! {
            <>
              <path d="M3 6h.01M8 6h13M3 12h.01M8 12h13M3 18h.01M8 18h13" />
            </>
          }
            .into_any()
        }
      }}
    </svg>
  }
}

// -- Хранилище --------------------------------------------------------------

/// Выбранная раскладка из `localStorage`, если она там есть и понятна.
#[cfg(any(feature = "csr", feature = "hydrate"))]
fn read_layout(key: &str) -> Option<CollectionLayout> {
  let stored = window().local_storage().ok().flatten()?.get_item(key).ok().flatten()?;
  CollectionLayout::from_storage(&stored)
}

#[cfg(any(feature = "csr", feature = "hydrate"))]
fn write_layout(key: &str, layout: CollectionLayout) {
  if let Ok(Some(storage)) = window().local_storage() {
    let _ = storage.set_item(key, layout.as_storage());
  }
}

/// На сервере хранилища нет, и выбора тоже: страница рисуется умолчанием
/// набора, а гидратация подставляет выбранное.
#[cfg(feature = "ssr")]
fn read_layout(_key: &str) -> Option<CollectionLayout> {
  None
}

#[cfg(feature = "ssr")]
fn write_layout(_key: &str, _layout: CollectionLayout) {}

#[cfg(test)]
mod tests {
  use super::*;

  /// Запись и чтение — одна таблица, прочитанная в обе стороны.
  #[test]
  fn storage_round_trips() {
    for layout in [CollectionLayout::Grid, CollectionLayout::List] {
      assert_eq!(CollectionLayout::from_storage(layout.as_storage()), Some(layout));
    }
  }

  /// Непонятное в хранилище — это «ничего не выбирали», а не плитка.
  ///
  /// Разница видна у набора, объявленного списком: прочитай мы мусор как
  /// `Grid`, такой набор становился бы плиткой у каждого, кто переключателя не
  /// касался, — и у всех после любой смены формата ключа.
  #[test]
  fn unknown_storage_is_no_choice_at_all() {
    for junk in ["", "tiles", "GRID", "список", "null"] {
      assert_eq!(CollectionLayout::from_storage(junk), None, "для {junk:?}");
    }
  }

  /// Ключ несёт имя набора: один и тот же набор на двух экранах — один набор.
  #[test]
  fn the_key_is_named_after_the_collection() {
    assert_eq!(collection_layout_key("document"), "collection-layout:document");
    assert_ne!(collection_layout_key("document"), collection_layout_key("board"));
  }

  /// Классы колонок — статические строки, иначе Tailwind их не соберёт.
  ///
  /// Проверяется и предел: приложение, назвавшее восемь колонок, получает
  /// шесть, а не класс `xl:grid-cols-8`, которого в сборке нет.
  #[test]
  fn columns_are_classes_tailwind_can_find() {
    assert_eq!(CollectionColumns(1).class(CollectionLayout::Grid), "");
    assert_eq!(CollectionColumns(3).class(CollectionLayout::Grid), "sm:grid-cols-2 lg:grid-cols-3");
    assert_eq!(
      CollectionColumns(8).class(CollectionLayout::Grid),
      "sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6"
    );
    // Список — одна колонка на любой ширине, сколько бы их ни объявили.
    assert_eq!(CollectionColumns(4).class(CollectionLayout::List), "");
  }

  /// Скелет обещает, сколько примерно приедет, и обещает это по-разному.
  #[test]
  fn the_skeleton_matches_the_shape_it_stands_in_for() {
    assert!(skeletons(CollectionLayout::Grid) > skeletons(CollectionLayout::List));
  }

  #[test]
  fn the_other_layout_is_the_other_one() {
    assert_eq!(CollectionLayout::Grid.other(), CollectionLayout::List);
    assert_eq!(CollectionLayout::List.other(), CollectionLayout::Grid);
  }
}

#[cfg(test)]
mod usage {
  use super::*;

  #[derive(Clone)]
  struct Doc {
    id: i64,
    title: String,
    excerpt: String,
  }

  /// Элемент набора: два вида, выбранные раскладкой, — то, ради чего набор и
  /// отдаёт её контекстом.
  ///
  /// В плитке у карточки есть площадь, и она тратится на выдержку; в строке
  /// площади нет, и выдержки в ней тоже нет. Это два вида, а не один с
  /// другими классами.
  #[component]
  fn DocumentItem(doc: Doc) -> impl IntoView {
    let layout = use_collection_layout();
    move || match layout.get() {
      CollectionLayout::Grid => view! {
        <div class="rounded-lg border border-border p-4">
          <div class="font-medium">{doc.title.clone()}</div>
          <p class="mt-2 text-sm text-muted-foreground">{doc.excerpt.clone()}</p>
        </div>
      }
      .into_any(),
      CollectionLayout::List => view! {
        <div class="flex items-center gap-3 border-b border-border py-2">
          <span class="truncate font-medium">{doc.title.clone()}</span>
        </div>
      }
      .into_any(),
    }
  }

  /// Раскрывающаяся строка — так, как её пишет приложение: раскрытость
  /// считается от «который открыт», а не хранится в строке.
  #[test]
  fn a_disclosure_is_driven_by_which_one_is_open() {
    let editing = RwSignal::new(None::<String>);
    let _unrendered = move || {
      let name = "report.typ".to_string();
      let (for_open, for_toggle, for_summary) = (name.clone(), name.clone(), name.clone());
      view! {
        <CollectionDisclosure
          open=Signal::derive(move || editing.get().as_deref() == Some(for_open.as_str()))
          on_toggle=Callback::new(move |()| {
            editing
              .update(|at| {
                *at = match at.as_deref() == Some(for_toggle.as_str()) {
                  true => None,
                  false => Some(for_toggle.clone()),
                };
              })
          })
          label="Показать шаблон"
          summary=move || view! { <span class="font-medium">{for_summary.clone()}</span> }
        >
          <textarea class="w-full" />
        </CollectionDisclosure>
      }
    };
  }

  /// Собирается ли набор так, как его пишет приложение.
  ///
  /// Проверка именно сборки: вид здесь не разворачивают, а props у
  /// `#[component]` сверяются на этапе компиляции — и расходятся они ровно
  /// там, где такого вызова нет ни в одном собираемом крейте.
  #[test]
  fn an_app_writes_its_own_for_loop_inside_the_collection() {
    let docs = RwSignal::new(Vec::<Doc>::new());
    let _unrendered = move || {
      view! {
        <CollectionView
          name="document"
          len=Signal::derive(move || docs.get().len())
          switchable=true
          columns=CollectionColumns(3)
          default_layout=CollectionLayout::List
          empty_title="Пока пусто"
          empty_hint="Создайте первый документ"
          labels=CollectionLabels { grid: "Плиткой".into(), list: "Списком".into() }
          toolbar=|| view! { <button type="button">"Создать"</button> }
        >
          <For each=move || docs.get() key=|doc| doc.id let:doc>
            <DocumentItem doc=doc />
          </For>
        </CollectionView>
      }
    };
  }

  /// Строка набора, вынесенная приложением в свой `#[component]`: ровно так её
  /// пишет вкладка шаблонов Представы.
  ///
  /// Проверяются здесь два превращения, на которых такая строка и ломается, а
  /// ломается она в чужом крейте, который этой сборке не виден:
  ///
  /// * `#[prop(into)] Callback<String>` из простого замыкания. `From` для
  ///   `Callback` объявлен только для кортежных входов, так что `Callback<String>`
  ///   из `Fn(String)` берётся не им, а `IntoReactiveValue`, — и держится это
  ///   ровно на том, что `#[prop(into)]` разворачивается в него.
  /// * `#[prop(into)] Callback<()>` из `move |_: ()|`. Тип аргумента подписан
  ///   нарочно: вывести его из одного места вызова компилятору нечем.
  #[component]
  fn TemplateRow(
    name: String,
    editing: RwSignal<Option<String>>,
    #[prop(into)] on_open: Callback<String>,
    #[prop(into)] on_save: Callback<()>,
  ) -> impl IntoView {
    let name = StoredValue::new(name);
    let facts = "2 КиБ".to_string();
    let open = Signal::derive(move || editing.get().as_deref() == Some(name.get_value().as_str()));
    // Замыкание из одних `Copy`-захватов — само `Copy`, и его берут дважды:
    // значок раскрытия и щелчок по названию открывают одно и то же.
    let toggle = move || match open.get_untracked() {
      true => editing.set(None),
      false => on_open.run(name.get_value()),
    };

    view! {
      <CollectionDisclosure
        open=open
        on_toggle=Callback::new(move |()| toggle())
        label="Показать шаблон"
        summary=move || {
          view! {
            <>
              <button on:click=move |_| toggle()>{move || name.get_value()}</button>
              <span class="text-xs text-muted-foreground">{facts.clone()}</span>
            </>
          }
        }
      >
        <button on:click=move |_| on_save.run(())>"Сохранить"</button>
      </CollectionDisclosure>
    }
  }

  #[test]
  fn an_app_hangs_its_own_row_component_off_a_disclosure() {
    let editing = RwSignal::new(None::<String>);
    let open = move |name: String| editing.set(Some(name));
    let save = move |_: ()| editing.set(None);
    let _unrendered = move || {
      view! { <TemplateRow name="report.typ".to_string() editing on_open=open on_save=save /> }
    };
  }

  /// Снаружи набора раскладка — умолчание: карточка, нарисованная вне набора,
  /// рисуется как карточка, а не гадает, где она.
  #[test]
  fn outside_a_collection_the_layout_is_the_default() {
    let _ = move || {
      assert_eq!(use_collection_layout().get(), CollectionLayout::Grid);
    };
  }
}

#[cfg(test)]
mod generated_shape {
  use super::*;
  use crate::layout::{ContainerSize, Gap};

  /// Ровно та обёртка, которую печатает вывод `impulse-apps` на объявление
  /// `Collection::of("board", "Доски").as_grid(3).switchable()`.
  ///
  /// Проверка именно сборки, и проверять есть что: дети едут свойством, а не
  /// блоком — блок стал бы замыканием, забирающим их себе, а набор рисует их
  /// в одной ветке из трёх. Это `FnOnce` и отказ сборки клиента (E0525).
  #[component]
  fn BoardCollection(
    #[prop(into)] len: Signal<usize>,
    #[prop(optional, into)] loading: Signal<bool>,
    children: ChildrenFn,
  ) -> impl IntoView {
    view! {
      <CollectionView
        name="board"
        len=len
        loading=loading
        default_layout=CollectionLayout::Grid
        switchable=true
        columns=CollectionColumns(3)
        width=ContainerSize::Sm
        gap=Gap::Md
        empty_title="Пока нет ни одной доски"
        empty_hint="Создайте первую"
        labels=CollectionLabels { grid: "Плиткой".into(), list: "Списком".into() }
        children=children
      />
    }
  }

  #[test]
  fn the_generated_wrapper_forwards_its_children() {
    let _unrendered = move || {
      view! {
        <BoardCollection len=0>
          <div>"карточка"</div>
        </BoardCollection>
      }
    };
  }
}

#[cfg(test)]
mod two_views {
  use super::*;
  use crate::button::{Button, ButtonVariant};

  /// Так приложение пишет элемент набора, у которого два вида.
  ///
  /// Проверяется не вёрстка, а то, чем раскладка доезжает до классов, — и там
  /// две разные формы, которые легко перепутать:
  ///
  /// * у обычного узла `class` — атрибут, и замыкание, отдающее `&'static
  ///   str`, в него ложится как есть;
  /// * у кнопки кита `class` — свойство `Signal<String>`, и то же самое
  ///   замыкание в него не ложится: нужен `Signal::derive` со `String`.
  ///
  /// Перепутанное — отказ сборки клиента, а клиент приложения собирается не
  /// здесь. Поэтому форма закреплена здесь.
  #[component]
  fn BoardFace(#[prop(into)] title: String, owner: bool) -> impl IntoView {
    let layout = use_collection_layout();
    let title = StoredValue::new(title);

    view! {
      <Button
        variant=ButtonVariant::Outline
        class=Signal::derive(move || {
          match layout.get() {
            CollectionLayout::Grid => "flex h-full min-h-20 w-full flex-col items-start text-left",
            CollectionLayout::List => "flex w-full flex-row items-center gap-3 text-left",
          }
            .to_string()
        })
      >
        <div class=move || match layout.get() {
          CollectionLayout::Grid => "line-clamp-2 w-full min-w-0 whitespace-normal font-medium",
          CollectionLayout::List => "min-w-0 flex-1 truncate font-medium",
        }>{move || title.get_value()}</div>
        <div class=move || match layout.get() {
          CollectionLayout::Grid => "mt-auto w-full text-xs text-muted-foreground",
          CollectionLayout::List => "ml-auto shrink-0 text-xs text-muted-foreground",
        }>{if owner { "владелец" } else { "доступ открыт" }}</div>
      </Button>
    }
  }

  #[test]
  fn an_item_picks_its_classes_by_the_layout_it_is_drawn_in() {
    let _unrendered = move || {
      view! { <BoardFace title="Доска" owner=true /> }
    };
  }
}

#[cfg(test)]
mod spacing {
  use super::*;

  /// Промежуток следует раскладке, а не только объявлению.
  ///
  /// Объявленный — про плитку: у карточки есть площадь, и воздух отделяет одну
  /// от другой. У строки площади нет, и отделяет её линия снизу, поэтому
  /// шестнадцать пикселей между строками рассыпали перечень на карточки
  /// (IUX-24).
  #[test]
  fn список_берёт_волосок_а_плитка_объявленное() {
    assert_eq!(gap_for(CollectionLayout::Grid, Gap::Md).class(), "gap-4");
    assert_eq!(gap_for(CollectionLayout::Grid, Gap::Lg).class(), "gap-6");
    // Что бы набор ни объявил: промежуток списка — свойство списка.
    assert_eq!(gap_for(CollectionLayout::List, Gap::Md).class(), "gap-px");
    assert_eq!(gap_for(CollectionLayout::List, Gap::Xxl).class(), "gap-px");
  }

  /// Волосок — это волосок, а не «почти ноль».
  ///
  /// Ступени между нулём и четырьмя пикселями в шкале не было вовсе, и список
  /// приходилось выбирать между «слиплось» и «рассыпалось».
  #[test]
  fn волосок_это_один_пиксель() {
    assert_eq!(Gap::Hair.class(), "gap-px");
    assert_eq!(Gap::None.class(), "gap-0");
    assert_eq!(Gap::Xs.class(), "gap-1");
  }
}
