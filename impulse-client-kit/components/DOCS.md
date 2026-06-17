# Impulse UI Kit - Component API Documentation

A comprehensive component library for building modern web applications with [Leptos](https://leptos.dev/). Inspired by shadcn/ui design patterns.

## Table of Contents

- [Getting Started](#getting-started)
- [Basic Components](#basic-components)
  - [Button](#button)
  - [Badge](#badge)
  - [Label](#label)
  - [Spinner](#spinner)
  - [Skeleton](#skeleton)
- [Form Components](#form-components)
  - [Input](#input)
  - [Textarea](#textarea)
  - [Checkbox](#checkbox)
  - [Switch](#switch)
  - [Radio Group](#radio-group)
  - [Select](#select)
  - [Slider](#slider)
  - [Input OTP](#input-otp)
- [Layout Components](#layout-components)
  - [Card](#card)
  - [Separator](#separator)
  - [Accordion](#accordion)
  - [Collapsible](#collapsible)
  - [Tabs](#tabs)
  - [Scroll Area](#scroll-area)
  - [Aspect Ratio](#aspect-ratio)
  - [Resizable](#resizable)
- [Navigation Components](#navigation-components)
  - [Breadcrumb](#breadcrumb)
  - [Button Group](#button-group)
- [Overlay Components](#overlay-components)
  - [Dialog](#dialog)
  - [Alert Dialog](#alert-dialog)
  - [Popover](#popover)
  - [Tooltip](#tooltip)
  - [Dropdown Menu](#dropdown-menu)
  - [Context Menu](#context-menu)
  - [Drawer](#drawer)
  - [Sheet](#sheet)
- [Feedback Components](#feedback-components)
  - [Alert](#alert)
  - [Toast](#toast)
  - [Progress](#progress)
- [Data Display Components](#data-display-components)
  - [Table](#table)
  - [Avatar](#avatar)
  - [Calendar](#calendar)
- [Interactive Components](#interactive-components)
  - [Toggle](#toggle)
  - [Toggle Group](#toggle-group)
  - [Command](#command)
  - [Combobox](#combobox)
  - [Carousel](#carousel)
- [Utility Components](#utility-components)
  - [Theme](#theme)
  - [Sidebar](#sidebar)

---

## Getting Started

### Installation

Add the components crate to your `Cargo.toml`:

```toml
[dependencies]
impulse-client-kit-components = { path = "../impulse-client-kit/components" }
```

### Basic Usage

```rust
use impulse_client_kit_components as components;
use components::button::{Button, ButtonVariant, ButtonSize};
use leptos::prelude::*;

#[component]
fn App() -> impl IntoView {
    view! {
        <Button variant=ButtonVariant::Default>"Click me"</Button>
    }
}
```

---

## Basic Components

### Button

A versatile button component with multiple variants and sizes.

#### Import

```rust
use components::button::{Button, ButtonVariant, ButtonSize};
```

#### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `variant` | `ButtonVariant` | `Default` | Visual style variant |
| `size` | `ButtonSize` | `Default` | Size of the button |
| `class` | `Signal<String>` | `""` | Additional CSS classes |
| `node_ref` | `NodeRef<Button>` | - | Reference to the button element |
| `children` | `Children` | required | Button content |

#### Variants

- `ButtonVariant::Default` - Primary button with background
- `ButtonVariant::Destructive` - Red destructive action
- `ButtonVariant::Outline` - Bordered button
- `ButtonVariant::Secondary` - Secondary style
- `ButtonVariant::Ghost` - Transparent with hover effect
- `ButtonVariant::Link` - Styled as a link

#### Sizes

- `ButtonSize::Sm` - Small size (h-8, default)
- `ButtonSize::Middle` - Standard size (h-9)
- `ButtonSize::Lg` - Large size (h-10)
- `ButtonSize::Icon` - Square icon button (size-9)
- `ButtonSize::IconSm` - Small icon button (size-8)
- `ButtonSize::IconLg` - Large icon button (size-10)

#### Examples

```rust
// Basic button variants
<Button variant=ButtonVariant::Default>"Default"</Button>
<Button variant=ButtonVariant::Destructive>"Delete"</Button>
<Button variant=ButtonVariant::Outline>"Outline"</Button>
<Button variant=ButtonVariant::Secondary>"Secondary"</Button>
<Button variant=ButtonVariant::Ghost>"Ghost"</Button>
<Button variant=ButtonVariant::Link>"Link"</Button>

// Button sizes
<Button size=ButtonSize::Sm>"Small"</Button>
<Button size=ButtonSize::Middle>"Middle"</Button>
<Button size=ButtonSize::Lg>"Large"</Button>
<Button size=ButtonSize::Icon>"+"</Button>

// With click handler
<Button on:click=move |_| { log::info!("Clicked!") }>"Click me"</Button>

// Disabled button
<Button attr:disabled=true>"Disabled"</Button>
```

---

### Badge

Status or label badge with variant styles.

#### Import

```rust
use components::badge::{Badge, BadgeVariant};
```

#### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `variant` | `BadgeVariant` | `Default` | Visual style variant |
| `class` | `String` | `""` | Additional CSS classes |
| `as_child` | `bool` | `false` | Render as child element |
| `children` | `ChildrenFragmentFn` | required | Badge content |

#### Variants

- `BadgeVariant::Default` - Primary colored badge
- `BadgeVariant::Secondary` - Secondary colored badge
- `BadgeVariant::Destructive` - Red badge for errors/warnings
- `BadgeVariant::Outline` - Outlined badge

#### Examples

```rust
// Basic badges
<Badge variant=BadgeVariant::Default>"New"</Badge>
<Badge variant=BadgeVariant::Secondary>"Beta"</Badge>
<Badge variant=BadgeVariant::Destructive>"Error"</Badge>
<Badge variant=BadgeVariant::Outline>"Draft"</Badge>

// As child (renders on first child element)
<Badge as_child=true>
    <a href="/new">"New Feature"</a>
</Badge>
```

---

### Label

Form label with proper accessibility support.

#### Import

```rust
use components::label::Label;
```

#### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `r#for` | `String` | `""` | ID of the associated form element |
| `class` | `String` | `""` | Additional CSS classes |
| `children` | `Children` | required | Label content |

#### Examples

```rust
// Basic label with input
<div class="space-y-2">
    <Label r#for="email">"Email Address"</Label>
    <Input r#type="email" attr:id="email" />
</div>

// Label with checkbox
<div class="flex items-center space-x-2">
    <Checkbox attr:id="terms" />
    <Label r#for="terms">"Accept terms and conditions"</Label>
</div>
```

---

### Spinner

Loading indicator with size variants.

#### Import

```rust
use components::spinner::{Spinner, SpinnerSize};
```

#### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `size` | `SpinnerSize` | `Default` | Size of the spinner |
| `class` | `String` | `""` | Additional CSS classes |

#### Sizes

- `SpinnerSize::Sm` - Small (h-4 w-4)
- `SpinnerSize::Default` - Medium (h-8 w-8)
- `SpinnerSize::Lg` - Large (h-12 w-12)

#### Examples

```rust
// Different sizes
<Spinner size=SpinnerSize::Sm />
<Spinner size=SpinnerSize::Default />
<Spinner size=SpinnerSize::Lg />

// With loading text
<div class="flex items-center gap-2">
    <Spinner size=SpinnerSize::Default />
    <span>"Loading..."</span>
</div>
```

---

### Skeleton

Loading placeholder for content.

#### Import

```rust
use components::skeleton::Skeleton;
```

#### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `class` | `String` | `""` | CSS classes to define size/shape |

#### Examples

```rust
// Card skeleton
<div class="space-y-4">
    <Skeleton class="h-12 w-12 rounded-full" />
    <Skeleton class="h-4 w-full" />
    <Skeleton class="h-4 w-3/4" />
    <Skeleton class="h-4 w-1/2" />
</div>

// Text skeleton
<div class="space-y-2">
    <Skeleton class="h-4 w-[250px]" />
    <Skeleton class="h-4 w-[200px]" />
</div>
```

---

## Form Components

### Input

Text input field with full styling support.

#### Import

```rust
use components::input::Input;
```

#### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `r#type` | `String` | `""` | Input type (text, email, password, etc.) |
| `value` | `RwSignal<String>` | - | Controlled value signal |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
// Basic input
let email = RwSignal::new(String::new());
<Input
    r#type="email".to_string()
    attr:placeholder="Enter your email"
    value=email
/>

// Password input
let password = RwSignal::new(String::new());
<Input
    r#type="password".to_string()
    attr:placeholder="Enter password"
    value=password
/>

// With label
<div class="space-y-2">
    <Label r#for="username">"Username"</Label>
    <Input
        r#type="text".to_string()
        attr:id="username"
        attr:placeholder="Enter username"
        value=username
    />
</div>

// Disabled input
<Input
    r#type="text".to_string()
    attr:disabled=true
    attr:placeholder="Disabled"
/>
```

---

### Textarea

Multi-line text input.

#### Import

```rust
use components::textarea::Textarea;
```

#### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `RwSignal<String>` | - | Controlled value signal |
| `placeholder` | `String` | `""` | Placeholder text |
| `disabled` | `bool` | `false` | Disabled state |
| `rows` | `Option<i32>` | `4` | Number of visible rows |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
let description = RwSignal::new(String::new());

// Basic textarea
<Textarea
    value=description
    placeholder="Enter description..."
/>

// With custom rows
<Textarea
    value=description
    placeholder="Write your message..."
    rows=Some(6)
    class="min-h-[150px]"
/>
```

---

### Checkbox

Checkbox control with checked state management.

#### Import

```rust
use components::checkbox::Checkbox;
```

#### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `checked` | `Option<RwSignal<bool>>` | - | Controlled checked state |
| `default_checked` | `bool` | `false` | Initial checked state |
| `disabled` | `bool` | `false` | Disabled state |
| `on_change` | `Option<Callback<bool>>` | - | Change callback |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
let is_checked = RwSignal::new(false);

// Basic checkbox
<div class="flex items-center space-x-2">
    <Checkbox checked=is_checked attr:id="terms" />
    <Label r#for="terms">"Accept terms and conditions"</Label>
</div>

// With callback
<Checkbox
    checked=is_checked
    on_change=Callback::new(move |checked| {
        log::info!("Checked: {}", checked);
    })
/>

// Disabled checkbox
<Checkbox disabled=true default_checked=true />
```

---

### Switch

Toggle switch component.

#### Import

```rust
use components::switch::Switch;
```

#### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `checked` | `Option<RwSignal<bool>>` | - | Controlled checked state |
| `default_checked` | `bool` | `false` | Initial checked state |
| `disabled` | `bool` | `false` | Disabled state |
| `on_change` | `Option<Callback<bool>>` | - | Change callback |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
let is_enabled = RwSignal::new(true);

// Basic switch
<div class="flex items-center space-x-2">
    <Switch checked=is_enabled attr:id="notifications" />
    <Label r#for="notifications">"Enable notifications"</Label>
</div>

// With callback
<Switch
    checked=is_enabled
    on_change=Callback::new(move |enabled| {
        log::info!("Enabled: {}", enabled);
    })
/>
```

---

### Radio Group

Radio button group for single selection.

#### Import

```rust
use components::radio_group::*;
```

#### Props (RadioGroup)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `Option<RwSignal<String>>` | - | Controlled value |
| `default_value` | `String` | `""` | Initial selected value |
| `on_value_change` | `Option<Callback<String>>` | - | Change callback |
| `disabled` | `bool` | `false` | Disable entire group |
| `class` | `String` | `""` | Additional CSS classes |

#### Props (RadioGroupItem)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `String` | required | Value of this option |
| `disabled` | `bool` | `false` | Disabled state |
| `id` | `String` | `""` | Element ID |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
// Basic radio group
<RadioGroup default_value="option1">
    <div class="flex items-center space-x-2">
        <RadioGroupItem value="option1" id="r1" />
        <Label r#for="r1">"Option 1"</Label>
    </div>
    <div class="flex items-center space-x-2">
        <RadioGroupItem value="option2" id="r2" />
        <Label r#for="r2">"Option 2"</Label>
    </div>
    <div class="flex items-center space-x-2">
        <RadioGroupItem value="option3" id="r3" />
        <Label r#for="r3">"Option 3"</Label>
    </div>
</RadioGroup>

// Controlled radio group
let selected = RwSignal::new("option1".to_string());
<RadioGroup
    value=selected
    on_value_change=Callback::new(move |v| log::info!("Selected: {}", v))
>
    // ... RadioGroupItem components
</RadioGroup>
```

---

### Select

Dropdown select with rich features.

#### Import

```rust
use components::select::*;
```

#### Components

- `Select` - Root container
- `SelectTrigger` - Trigger button
- `SelectValue` - Display selected value
- `SelectContent` - Dropdown content
- `SelectGroup` - Group items
- `SelectLabel` - Group label
- `SelectItem` - Selectable item
- `SelectSeparator` - Visual separator

#### Props (Select)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `Option<RwSignal<String>>` | - | Controlled value |
| `default_value` | `Option<String>` | - | Initial value |
| `open` | `Option<RwSignal<bool>>` | - | Controlled open state |
| `on_value_change` | `Option<Callback<String>>` | - | Change callback |

#### Props (SelectTrigger)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `size` | `SelectTriggerSize` | `Default` | Trigger size |
| `disabled` | `bool` | `false` | Disabled state |
| `class` | `String` | `""` | Additional CSS classes |

#### Props (SelectContent)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `side` | `Option<OverlaySide>` | `Bottom` | Placement side |
| `align` | `Option<OverlayAlign>` | `Start` | Alignment |
| `side_offset` | `Option<i32>` | `4` | Offset from trigger |
| `position` | `Option<SelectContentPosition>` | `ItemAligned` | Position mode |
| `class` | `String` | `""` | Additional CSS classes |

#### Props (SelectItem)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `String` | required | Item value |
| `disabled` | `bool` | `false` | Disabled state |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
let selected = RwSignal::new(String::new());

// Basic select
<Select value=selected>
    <SelectTrigger>
        <SelectValue placeholder="Select an option" />
    </SelectTrigger>
    <SelectContent>
        <SelectItem value="option1">"Option 1"</SelectItem>
        <SelectItem value="option2">"Option 2"</SelectItem>
        <SelectItem value="option3">"Option 3"</SelectItem>
    </SelectContent>
</Select>

// Grouped select
<Select value=selected>
    <SelectTrigger>
        <SelectValue placeholder="Select a fruit" />
    </SelectTrigger>
    <SelectContent>
        <SelectGroup>
            <SelectLabel>"Fruits"</SelectLabel>
            <SelectItem value="apple">"Apple"</SelectItem>
            <SelectItem value="banana">"Banana"</SelectItem>
            <SelectItem value="orange">"Orange"</SelectItem>
        </SelectGroup>
        <SelectSeparator />
        <SelectGroup>
            <SelectLabel>"Vegetables"</SelectLabel>
            <SelectItem value="carrot">"Carrot"</SelectItem>
            <SelectItem value="potato">"Potato"</SelectItem>
        </SelectGroup>
    </SelectContent>
</Select>
```

---

### Slider

Range slider for numeric input.

#### Import

```rust
use components::slider::Slider;
```

#### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `Option<RwSignal<f64>>` | - | Controlled value |
| `default_value` | `Option<f64>` | `50.0` | Initial value |
| `min` | `Option<f64>` | `0.0` | Minimum value |
| `max` | `Option<f64>` | `100.0` | Maximum value |
| `step` | `Option<f64>` | `1.0` | Step increment |
| `disabled` | `bool` | `false` | Disabled state |
| `on_value_change` | `Option<Callback<f64>>` | - | Change callback |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
let slider_value = RwSignal::new(50.0);

// Basic slider
<div class="space-y-4">
    <Slider value=slider_value min=0.0 max=100.0 step=1.0 />
    <p class="text-sm text-muted-foreground">
        "Value: " {move || format!("{:.0}", slider_value.get())}
    </p>
</div>

// With callback
<Slider
    value=slider_value
    on_value_change=Callback::new(move |v| {
        log::info!("Value changed: {}", v);
    })
/>
```

---

### Input OTP

One-time password input.

#### Import

```rust
use components::input_otp::{InputOTP, InputOTPWithSeparator};
```

#### Props (InputOTP)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `length` | `usize` | required | Number of digits |
| `on_complete` | `Callback<String>` | required | Called when all digits entered |

#### Props (InputOTPWithSeparator)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `length` | `usize` | required | Number of digits |
| `separator_at` | `usize` | required | Position of separator |

#### Examples

```rust
let otp_code = RwSignal::new(String::new());

// Basic OTP input
<InputOTP
    length=6usize
    on_complete=Callback::new(move |code| {
        otp_code.set(code);
        log::info!("OTP entered: {}", otp_code.get());
    })
/>

// With separator (e.g., 123-456)
<InputOTPWithSeparator length=6usize separator_at=3usize />
```

---

## Layout Components

### Card

Container with header, content, and footer sections.

#### Import

```rust
use components::card::*;
```

#### Components

- `Card` - Main container
- `CardHeader` - Header section
- `CardTitle` - Title text
- `CardDescription` - Description text
- `CardAction` - Action button area
- `CardContent` - Main content
- `CardFooter` - Footer section

#### Examples

```rust
// Basic card
<Card>
    <CardHeader>
        <CardTitle>"Card Title"</CardTitle>
        <CardDescription>"This is a card description"</CardDescription>
    </CardHeader>
    <CardContent>
        <p>"Main content goes here."</p>
    </CardContent>
    <CardFooter>
        <Button variant=ButtonVariant::Outline>"Cancel"</Button>
        <Button>"Save"</Button>
    </CardFooter>
</Card>

// Card with action
<Card>
    <CardHeader>
        <CardTitle>"Settings"</CardTitle>
        <CardDescription>"Manage your preferences"</CardDescription>
        <CardAction>
            <Button size=ButtonSize::Icon variant=ButtonVariant::Ghost>"..."</Button>
        </CardAction>
    </CardHeader>
    <CardContent>
        // Content
    </CardContent>
</Card>

// Stats card
<Card>
    <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle class="text-sm font-medium">"Total Revenue"</CardTitle>
        <span>"$"</span>
    </CardHeader>
    <CardContent>
        <div class="text-2xl font-bold">"$45,231.89"</div>
        <p class="text-xs text-muted-foreground">"+20.1% from last month"</p>
    </CardContent>
</Card>
```

---

### Separator

Visual divider line.

#### Import

```rust
use components::separator::{Separator, SeparatorOrientation};
```

#### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `orientation` | `SeparatorOrientation` | `Horizontal` | Line direction |
| `decorative` | `bool` | `true` | If true, purely decorative |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
// Horizontal separator
<div class="space-y-4">
    <div>"Content above"</div>
    <Separator orientation=SeparatorOrientation::Horizontal />
    <div>"Content below"</div>
</div>

// Vertical separator
<div class="flex h-5 items-center space-x-4">
    <span>"Left"</span>
    <Separator orientation=SeparatorOrientation::Vertical />
    <span>"Right"</span>
</div>
```

---

### Accordion

Collapsible sections for organizing content.

#### Import

```rust
use components::accordion::*;
```

#### Props (Accordion)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `accordion_type` | `AccordionType` | `Single` | Single or multiple open |
| `default_value` | `Option<Vec<String>>` | - | Initially open items |
| `value` | `Option<RwSignal<Vec<String>>>` | - | Controlled open items |
| `class` | `String` | `""` | Additional CSS classes |

#### Props (AccordionItem)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `String` | required | Unique identifier |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
// Single accordion (one open at a time)
<Accordion
    accordion_type=AccordionType::Single
    default_value=vec!["item-1".to_string()]
>
    <AccordionItem value="item-1">
        <AccordionTrigger>"Is it accessible?"</AccordionTrigger>
        <AccordionContent>
            "Yes. It adheres to WAI-ARIA design patterns."
        </AccordionContent>
    </AccordionItem>
    <AccordionItem value="item-2">
        <AccordionTrigger>"Is it styled?"</AccordionTrigger>
        <AccordionContent>
            "Yes. It comes with default styles that match your theme."
        </AccordionContent>
    </AccordionItem>
    <AccordionItem value="item-3">
        <AccordionTrigger>"Is it animated?"</AccordionTrigger>
        <AccordionContent>
            "Yes. Smooth animations are included by default."
        </AccordionContent>
    </AccordionItem>
</Accordion>

// Multiple accordion (multiple can be open)
<Accordion accordion_type=AccordionType::Multiple>
    // ... AccordionItem components
</Accordion>
```

---

### Collapsible

Single collapsible section.

#### Import

```rust
use components::collapsible::*;
```

#### Props (Collapsible)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `open` | `Option<RwSignal<bool>>` | - | Controlled open state |
| `default_open` | `bool` | `false` | Initially open |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
// Basic collapsible
<Collapsible default_open=false>
    <CollapsibleTrigger>
        <Button variant=ButtonVariant::Ghost>"Toggle Details"</Button>
    </CollapsibleTrigger>
    <CollapsibleContent>
        <div class="mt-4 p-4 border rounded-md">
            <p>"This content can be collapsed and expanded."</p>
        </div>
    </CollapsibleContent>
</Collapsible>

// Controlled collapsible
let is_open = RwSignal::new(false);
<Collapsible open=is_open>
    <CollapsibleTrigger>
        <Button>"Toggle"</Button>
    </CollapsibleTrigger>
    <CollapsibleContent>
        <p>"Controlled content"</p>
    </CollapsibleContent>
</Collapsible>
```

---

### Tabs

Tabbed interface for organizing content.

#### Import

```rust
use components::tabs::*;
```

#### Props (Tabs)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `Option<RwSignal<String>>` | - | Controlled active tab |
| `default_value` | `String` | `""` | Initially active tab |
| `on_value_change` | `Option<Callback<String>>` | - | Tab change callback |
| `class` | `String` | `""` | Additional CSS classes |

#### Props (TabsTrigger)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `String` | required | Tab identifier |
| `disabled` | `bool` | `false` | Disabled state |
| `class` | `String` | `""` | Additional CSS classes |

#### Props (TabsContent)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `String` | required | Tab identifier |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
// Basic tabs
<Tabs default_value="overview">
    <TabsList>
        <TabsTrigger value="overview">"Overview"</TabsTrigger>
        <TabsTrigger value="analytics">"Analytics"</TabsTrigger>
        <TabsTrigger value="reports">"Reports"</TabsTrigger>
    </TabsList>
    <TabsContent value="overview">
        <p class="mt-4">"Overview content goes here."</p>
    </TabsContent>
    <TabsContent value="analytics">
        <p class="mt-4">"Analytics content goes here."</p>
    </TabsContent>
    <TabsContent value="reports">
        <p class="mt-4">"Reports content goes here."</p>
    </TabsContent>
</Tabs>

// Controlled tabs
let active_tab = RwSignal::new("tab1".to_string());
<Tabs
    value=active_tab
    on_value_change=Callback::new(move |v| log::info!("Tab: {}", v))
>
    // ... TabsList and TabsContent
</Tabs>
```

---

### Scroll Area

Custom styled scrollable area.

#### Import

```rust
use components::scroll_area::{ScrollArea, ScrollAreaOrientation};
```

#### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `orientation` | `Option<ScrollAreaOrientation>` | `Vertical` | Scroll direction |
| `class` | `String` | `""` | Additional CSS classes |

#### Orientations

- `ScrollAreaOrientation::Vertical`
- `ScrollAreaOrientation::Horizontal`
- `ScrollAreaOrientation::Both`

#### Examples

```rust
// Vertical scroll area
<ScrollArea class="h-[200px] w-full rounded-md border p-4">
    {(0..20)
        .map(|i| {
            view! { <div class="mb-4">{format!("Item {}", i + 1)}</div> }
        })
        .collect_view()}
</ScrollArea>

// Horizontal scroll area
<ScrollArea
    class="w-[400px] whitespace-nowrap rounded-md border"
    orientation=Some(ScrollAreaOrientation::Horizontal)
>
    <div class="flex w-max space-x-4 p-4">
        // Horizontal items
    </div>
</ScrollArea>
```

---

### Aspect Ratio

Container that maintains a specific aspect ratio.

#### Import

```rust
use components::aspect_ratio::AspectRatio;
```

#### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `ratio` | `f64` | required | Width/height ratio |

#### Examples

```rust
// 16:9 aspect ratio
<AspectRatio ratio=16.0 / 9.0>
    <div class="w-full h-full bg-muted rounded-md flex items-center justify-center">
        <span class="text-muted-foreground">"16:9 Aspect Ratio"</span>
    </div>
</AspectRatio>

// Square (1:1)
<AspectRatio ratio=1.0>
    <img src="image.jpg" class="w-full h-full object-cover rounded-md" />
</AspectRatio>

// 4:3 for photos
<AspectRatio ratio=4.0 / 3.0>
    // Content
</AspectRatio>
```

---

### Resizable

Resizable panels with drag handles.

#### Import

```rust
use components::resizable::*;
```

#### Props (ResizablePanelGroup)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `direction` | `ResizableDirection` | required | Layout direction |
| `class` | `String` | `""` | Additional CSS classes |

#### Props (ResizablePanel)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `default_size` | `f64` | required | Initial size percentage |

#### Examples

```rust
// Horizontal panels
<ResizablePanelGroup
    direction=ResizableDirection::Horizontal
    class="min-h-[200px] rounded-md border"
>
    <ResizablePanel default_size=50.0>
        <div class="flex h-full items-center justify-center p-6">
            <span class="font-semibold">"Panel 1"</span>
        </div>
    </ResizablePanel>
    <ResizableHandle />
    <ResizablePanel default_size=50.0>
        <div class="flex h-full items-center justify-center p-6">
            <span class="font-semibold">"Panel 2"</span>
        </div>
    </ResizablePanel>
</ResizablePanelGroup>

// Vertical panels
<ResizablePanelGroup direction=ResizableDirection::Vertical>
    // ... panels
</ResizablePanelGroup>
```

---

## Navigation Components

### Breadcrumb

Hierarchical navigation trail.

#### Import

```rust
use components::breadcrumb::*;
```

#### Components

- `Breadcrumb` - Container with nav element
- `BreadcrumbList` - Ordered list
- `BreadcrumbItem` - Individual item
- `BreadcrumbLink` - Clickable link
- `BreadcrumbPage` - Current page (non-clickable)
- `BreadcrumbSeparator` - Separator (default: chevron)
- `BreadcrumbEllipsis` - Ellipsis for collapsed items

#### Examples

```rust
// Basic breadcrumb
<Breadcrumb>
    <BreadcrumbList>
        <BreadcrumbItem>
            <BreadcrumbLink attr:href="/">"Home"</BreadcrumbLink>
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem>
            <BreadcrumbLink attr:href="/components">"Components"</BreadcrumbLink>
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem>
            <BreadcrumbPage>"Breadcrumb"</BreadcrumbPage>
        </BreadcrumbItem>
    </BreadcrumbList>
</Breadcrumb>

// With ellipsis
<Breadcrumb>
    <BreadcrumbList>
        <BreadcrumbItem>
            <BreadcrumbLink attr:href="/">"Home"</BreadcrumbLink>
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem>
            <BreadcrumbEllipsis />
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem>
            <BreadcrumbPage>"Current"</BreadcrumbPage>
        </BreadcrumbItem>
    </BreadcrumbList>
</Breadcrumb>
```

---

### Button Group

Related buttons grouped together.

#### Import

```rust
use components::button_group::ButtonGroup;
```

#### Examples

```rust
<ButtonGroup>
    <Button variant=ButtonVariant::Outline>"Left"</Button>
    <Button variant=ButtonVariant::Outline>"Center"</Button>
    <Button variant=ButtonVariant::Outline>"Right"</Button>
</ButtonGroup>
```

---

## Overlay Components

### Dialog

Modal dialog component.

#### Import

```rust
use components::dialog::*;
```

#### Components

- `Dialog` - Root container
- `DialogTrigger` - Opens the dialog
- `DialogContent` - Modal content
- `DialogHeader` - Header section
- `DialogTitle` - Title text
- `DialogDescription` - Description text
- `DialogFooter` - Footer with actions
- `DialogClose` - Close button wrapper
- `DialogOverlay` - Background overlay

#### Props (Dialog)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `open` | `Option<RwSignal<bool>>` | - | Controlled open state |
| `default_open` | `Option<bool>` | `false` | Initially open |
| `on_open_change` | `Option<Callback<bool>>` | - | Open state callback |

#### Examples

```rust
let dialog_open = RwSignal::new(false);

// Basic dialog
<Dialog open=dialog_open>
    <DialogTrigger>
        <Button>"Open Dialog"</Button>
    </DialogTrigger>
    <DialogContent>
        <DialogHeader>
            <DialogTitle>"Dialog Title"</DialogTitle>
            <DialogDescription>
                "This is a dialog description explaining what this dialog is for."
            </DialogDescription>
        </DialogHeader>
        <div class="py-4">
            <p>"Dialog content goes here."</p>
        </div>
        <DialogFooter>
            <DialogClose>
                <Button variant=ButtonVariant::Outline>"Cancel"</Button>
            </DialogClose>
            <Button on:click=move |_| dialog_open.set(false)>"Confirm"</Button>
        </DialogFooter>
    </DialogContent>
</Dialog>
```

---

### Alert Dialog

Confirmation dialog for important actions.

#### Import

```rust
use components::alert_dialog::*;
```

#### Components

Similar structure to Dialog:
- `AlertDialog`, `AlertDialogTrigger`, `AlertDialogContent`
- `AlertDialogHeader`, `AlertDialogTitle`, `AlertDialogDescription`
- `AlertDialogFooter`, `AlertDialogCancel`, `AlertDialogAction`

#### Examples

```rust
let alert_open = RwSignal::new(false);

<AlertDialog open=alert_open>
    <AlertDialogTrigger>
        <Button variant=ButtonVariant::Destructive>"Delete Account"</Button>
    </AlertDialogTrigger>
    <AlertDialogContent>
        <AlertDialogHeader>
            <AlertDialogTitle>"Are you absolutely sure?"</AlertDialogTitle>
            <AlertDialogDescription>
                "This action cannot be undone. This will permanently delete your account."
            </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
            <AlertDialogCancel>"Cancel"</AlertDialogCancel>
            <AlertDialogAction>"Continue"</AlertDialogAction>
        </AlertDialogFooter>
    </AlertDialogContent>
</AlertDialog>
```

---

### Popover

Popup content component.

#### Import

```rust
use components::popover::{Popover, PopoverTrigger, PopoverContent};
```

#### Props (Popover)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `open` | `Option<RwSignal<bool>>` | - | Controlled open state |

#### Props (PopoverContent)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `side` | `Option<OverlaySide>` | `Bottom` | Placement side |
| `align` | `Option<OverlayAlign>` | `Center` | Alignment |
| `side_offset` | `Option<i32>` | `4` | Offset from trigger |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
<Popover>
    <PopoverTrigger>
        <Button variant=ButtonVariant::Outline>"Open Popover"</Button>
    </PopoverTrigger>
    <PopoverContent>
        <div class="space-y-2">
            <h4 class="font-medium leading-none">"Popover Title"</h4>
            <p class="text-sm text-muted-foreground">"This is the popover content."</p>
        </div>
    </PopoverContent>
</Popover>
```

---

### Tooltip

Hover tooltip component.

#### Import

```rust
use components::tooltip::*;
```

#### Components

- `TooltipProvider` - Context provider
- `Tooltip` - Root container
- `TooltipTrigger` - Hover target
- `TooltipContent` - Tooltip content

#### Props (TooltipContent)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `side` | `Option<OverlaySide>` | `Top` | Placement side |
| `align` | `Option<OverlayAlign>` | `Center` | Alignment |
| `side_offset` | `Option<i32>` | `4` | Offset from trigger |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
<TooltipProvider>
    <Tooltip>
        <TooltipTrigger>
            <Button variant=ButtonVariant::Outline>"Hover me"</Button>
        </TooltipTrigger>
        <TooltipContent>
            <p>"This is a tooltip"</p>
        </TooltipContent>
    </Tooltip>
</TooltipProvider>
```

---

### Dropdown Menu

Dropdown menu with items and submenus.

#### Import

```rust
use components::dropdown_menu::*;
```

#### Components

- `DropdownMenu` - Root container
- `DropdownMenuTrigger` - Opens menu
- `DropdownMenuContent` - Menu content
- `DropdownMenuItem` - Clickable item
- `DropdownMenuLabel` - Section label
- `DropdownMenuSeparator` - Visual divider
- `DropdownMenuGroup` - Item group
- `DropdownMenuCheckboxItem` - Checkbox item
- `DropdownMenuRadioGroup` / `DropdownMenuRadioItem` - Radio items
- `DropdownMenuShortcut` - Keyboard shortcut text
- `DropdownMenuSub` / `DropdownMenuSubTrigger` / `DropdownMenuSubContent` - Submenu

#### Props (DropdownMenuItem)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `inset` | `bool` | `false` | Add left padding |
| `variant` | `Option<DropdownItemVariant>` | `Default` | Item variant |
| `on_select` | `Option<Callback<()>>` | - | Selection callback |
| `disabled` | `bool` | `false` | Disabled state |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
<DropdownMenu>
    <DropdownMenuTrigger>
        <Button variant=ButtonVariant::Outline>"Open Menu"</Button>
    </DropdownMenuTrigger>
    <DropdownMenuContent>
        <DropdownMenuLabel>"My Account"</DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem>"Profile"</DropdownMenuItem>
        <DropdownMenuItem>"Settings"</DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem variant=Some(DropdownItemVariant::Destructive)>
            "Logout"
        </DropdownMenuItem>
    </DropdownMenuContent>
</DropdownMenu>

// With checkbox items
let show_panel = RwSignal::new(true);
<DropdownMenuCheckboxItem
    checked=show_panel.into()
    on_checked_change=Callback::new(move |v| show_panel.set(v))
>
    "Show Panel"
</DropdownMenuCheckboxItem>
```

---

### Context Menu

Right-click context menu.

#### Import

```rust
use components::context_menu::*;
```

#### Components

Similar to DropdownMenu:
- `ContextMenu`, `ContextMenuTrigger`, `ContextMenuContent`
- `ContextMenuItem`, `ContextMenuLabel`, `ContextMenuSeparator`

#### Examples

```rust
<ContextMenu>
    <ContextMenuTrigger>
        <div class="border-2 border-dashed rounded-lg p-12 text-center">
            <p>"Right click here"</p>
        </div>
    </ContextMenuTrigger>
    <ContextMenuContent>
        <ContextMenuItem>"Cut"</ContextMenuItem>
        <ContextMenuItem>"Copy"</ContextMenuItem>
        <ContextMenuItem>"Paste"</ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem>"Delete"</ContextMenuItem>
    </ContextMenuContent>
</ContextMenu>
```

---

### Drawer

Slide-out drawer panel (mobile-friendly).

#### Import

```rust
use components::drawer::*;
```

#### Components

- `Drawer`, `DrawerTrigger`, `DrawerContent`
- `DrawerHeader`, `DrawerTitle`, `DrawerDescription`
- `DrawerFooter`, `DrawerClose`

#### Examples

```rust
let drawer_open = RwSignal::new(false);

<Drawer open=drawer_open>
    <DrawerTrigger>
        <Button>"Open Drawer"</Button>
    </DrawerTrigger>
    <DrawerContent>
        <DrawerHeader>
            <DrawerTitle>"Drawer Title"</DrawerTitle>
            <DrawerDescription>"This is a drawer description."</DrawerDescription>
        </DrawerHeader>
        <div class="p-4">
            <p>"Drawer content goes here."</p>
        </div>
        <DrawerFooter>
            <Button on:click=move |_| drawer_open.set(false)>"Close"</Button>
        </DrawerFooter>
    </DrawerContent>
</Drawer>
```

---

### Sheet

Side panel component.

#### Import

```rust
use components::sheet::*;
```

#### Props (Sheet)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `open` | `RwSignal<bool>` | - | Controlled open state |
| `side` | `SheetSide` | `Right` | Panel side |

#### Sides

- `SheetSide::Top`
- `SheetSide::Right`
- `SheetSide::Bottom`
- `SheetSide::Left`

#### Examples

```rust
let sheet_open = RwSignal::new(false);

<Sheet open=sheet_open side=SheetSide::Right>
    <SheetTrigger>
        <Button>"Open Sheet"</Button>
    </SheetTrigger>
    <SheetContent>
        <SheetHeader>
            <SheetTitle>"Sheet Title"</SheetTitle>
            <SheetDescription>"This is a sheet description."</SheetDescription>
        </SheetHeader>
        <div class="py-4">
            <p>"Sheet content goes here."</p>
        </div>
        <SheetFooter>
            <SheetClose>
                <Button>"Close"</Button>
            </SheetClose>
        </SheetFooter>
    </SheetContent>
</Sheet>
```

---

## Feedback Components

### Alert

Alert message component.

#### Import

```rust
use components::alert::*;
```

#### Props (Alert)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `variant` | `AlertVariant` | `Default` | Visual style |
| `class` | `String` | `""` | Additional CSS classes |

#### Variants

- `AlertVariant::Default` - Standard info alert
- `AlertVariant::Destructive` - Error/warning alert

#### Examples

```rust
// Info alert
<Alert variant=AlertVariant::Default>
    <AlertTitle>"Information"</AlertTitle>
    <AlertDescription>"This is an informational alert message."</AlertDescription>
</Alert>

// Error alert
<Alert variant=AlertVariant::Destructive>
    <AlertTitle>"Error"</AlertTitle>
    <AlertDescription>"This is an error alert message."</AlertDescription>
</Alert>
```

---

### Toast

Toast notification component.

#### Import

```rust
use components::toast::*;
```

#### Components

- `ToastProvider` - Context provider
- `ToastViewport` - Container for toasts
- `Toast` - Individual toast
- `ToastTitle` - Toast title
- `ToastDescription` - Toast description
- `ToastAction` - Action button
- `ToastClose` - Close button

#### Props (Toast)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `variant` | `ToastVariant` | `Default` | Visual style |
| `open` | `Option<RwSignal<bool>>` | - | Visibility state |
| `class` | `String` | `""` | Additional CSS classes |

#### Variants

- `ToastVariant::Default`
- `ToastVariant::Destructive`

#### Examples

```rust
// Basic toast
<Toast>
    <ToastTitle>"Toast Title"</ToastTitle>
    <ToastDescription>"This is a toast notification"</ToastDescription>
    <ToastClose />
</Toast>

// Destructive toast
<Toast variant=ToastVariant::Destructive>
    <ToastTitle>"Error"</ToastTitle>
    <ToastDescription>"Something went wrong!"</ToastDescription>
    <ToastAction>"Retry"</ToastAction>
    <ToastClose />
</Toast>
```

---

### Progress

Progress bar component.

#### Import

```rust
use components::progress::Progress;
```

#### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `Signal<f64>` | - | Current progress value |
| `max` | `Option<f64>` | `100.0` | Maximum value |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
let progress_value = Signal::derive(move || 65.0);

// Basic progress bar
<div class="space-y-2">
    <Progress value=progress_value />
    <p class="text-sm text-muted-foreground">
        {move || format!("Progress: {:.0}%", progress_value.get())}
    </p>
</div>

// Dynamic progress
let loading_progress = RwSignal::new(0.0);
<Progress value=loading_progress.into() />
```

---

## Data Display Components

### Table

Basic table component.

#### Import

```rust
use components::table::*;
```

#### Components

- `Table` - Table container
- `TableHeader` - Header section (`<thead>`)
- `TableBody` - Body section (`<tbody>`)
- `TableFooter` - Footer section (`<tfoot>`)
- `TableRow` - Table row (`<tr>`)
- `TableHead` - Header cell (`<th>`)
- `TableCell` - Data cell (`<td>`)
- `TableCaption` - Table caption

#### Examples

```rust
<Table>
    <TableHeader>
        <TableRow>
            <TableHead>"Name"</TableHead>
            <TableHead>"Status"</TableHead>
            <TableHead>"Role"</TableHead>
        </TableRow>
    </TableHeader>
    <TableBody>
        <TableRow>
            <TableCell>"John Doe"</TableCell>
            <TableCell>
                <Badge variant=BadgeVariant::Default>"Active"</Badge>
            </TableCell>
            <TableCell>"Developer"</TableCell>
        </TableRow>
        <TableRow>
            <TableCell>"Jane Smith"</TableCell>
            <TableCell>
                <Badge variant=BadgeVariant::Secondary>"Inactive"</Badge>
            </TableCell>
            <TableCell>"Designer"</TableCell>
        </TableRow>
    </TableBody>
</Table>
```

---

### Avatar

User avatar component with image fallback.

#### Import

```rust
use components::avatar::*;
```

#### Components

- `Avatar` - Container
- `AvatarImage` - Image element
- `AvatarFallback` - Fallback content (initials)

#### Examples

```rust
// Avatar with image
<Avatar>
    <AvatarImage attr:src="https://github.com/shadcn.png" attr:alt="User" />
    <AvatarFallback>"CN"</AvatarFallback>
</Avatar>

// Avatar with fallback only
<Avatar>
    <AvatarFallback>"JD"</AvatarFallback>
</Avatar>

// Avatar group
<div class="flex -space-x-4">
    <Avatar>
        <AvatarImage attr:src="user1.png" />
        <AvatarFallback>"U1"</AvatarFallback>
    </Avatar>
    <Avatar>
        <AvatarImage attr:src="user2.png" />
        <AvatarFallback>"U2"</AvatarFallback>
    </Avatar>
</div>
```

---

### Calendar

Calendar date picker.

#### Import

```rust
use components::calendar::*;
```

#### Props (Calendar)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `selected` | `RwSignal<CalendarSelection>` | - | Selected date(s) |
| `class` | `String` | `""` | Additional CSS classes |

#### Examples

```rust
let selected_date = RwSignal::new(CalendarSelection::None);

<Calendar selected=selected_date class="rounded-md border" />
```

---

## Interactive Components

### Toggle

Toggle button with pressed state.

#### Import

```rust
use components::toggle::{Toggle, ToggleVariant, ToggleSize};
```

#### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `pressed` | `Option<RwSignal<bool>>` | - | Controlled pressed state |
| `default_pressed` | `bool` | `false` | Initial state |
| `on_pressed_change` | `Option<Callback<bool>>` | - | State change callback |
| `variant` | `ToggleVariant` | `Default` | Visual style |
| `size` | `ToggleSize` | `Default` | Button size |
| `disabled` | `bool` | `false` | Disabled state |
| `class` | `String` | `""` | Additional CSS classes |

#### Variants

- `ToggleVariant::Default`
- `ToggleVariant::Outline`

#### Sizes

- `ToggleSize::Default` (h-10)
- `ToggleSize::Sm` (h-9)
- `ToggleSize::Lg` (h-11)

#### Examples

```rust
let toggle_pressed = RwSignal::new(false);

// Basic toggle
<Toggle pressed=toggle_pressed>"Bold"</Toggle>

// Outline variant
<Toggle variant=ToggleVariant::Outline>"Italic"</Toggle>

// With callback
<Toggle
    pressed=toggle_pressed
    on_pressed_change=Callback::new(move |p| log::info!("Pressed: {}", p))
>
    "Toggle"
</Toggle>
```

---

### Toggle Group

Group of toggle buttons for selection.

#### Import

```rust
use components::toggle_group::*;
```

#### Props (ToggleGroup)

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `r#type` | `ToggleGroupType` | required | Single or multiple |
| `default_value` | `Vec<String>` | `[]` | Initially selected values |
| `value` | `Option<RwSignal<Vec<String>>>` | - | Controlled selection |
| `on_value_change` | `Option<Callback<Vec<String>>>` | - | Change callback |

#### Types

- `ToggleGroupType::Single` - One selection at a time
- `ToggleGroupType::Multiple` - Multiple selections allowed

#### Examples

```rust
// Single selection (like radio)
<ToggleGroup r#type=ToggleGroupType::Single default_value=vec!["center".to_string()]>
    <ToggleGroupItem value="left">"Left"</ToggleGroupItem>
    <ToggleGroupItem value="center">"Center"</ToggleGroupItem>
    <ToggleGroupItem value="right">"Right"</ToggleGroupItem>
</ToggleGroup>

// Multiple selection
<ToggleGroup r#type=ToggleGroupType::Multiple>
    <ToggleGroupItem value="bold">"B"</ToggleGroupItem>
    <ToggleGroupItem value="italic">"I"</ToggleGroupItem>
    <ToggleGroupItem value="underline">"U"</ToggleGroupItem>
</ToggleGroup>
```

---

### Command

Command palette / search interface.

#### Import

```rust
use components::command::*;
```

#### Components

- `Command` - Root container
- `CommandInput` - Search input
- `CommandList` - Results list
- `CommandEmpty` - Empty state
- `CommandGroup` - Item group
- `CommandItem` - Selectable item
- `CommandSeparator` - Visual divider

#### Examples

```rust
<Command class="rounded-lg border">
    <CommandInput placeholder="Type a command..." />
    <CommandList>
        <CommandEmpty>"No results found."</CommandEmpty>
        <CommandGroup heading="Suggestions">
            <CommandItem>"Calendar"</CommandItem>
            <CommandItem>"Search Emoji"</CommandItem>
            <CommandItem>"Calculator"</CommandItem>
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup heading="Settings">
            <CommandItem>"Profile"</CommandItem>
            <CommandItem>"Billing"</CommandItem>
            <CommandItem>"Settings"</CommandItem>
        </CommandGroup>
    </CommandList>
</Command>
```

---

### Combobox

Searchable select component.

#### Import

```rust
use components::combobox::*;
```

#### Components

- `Combobox` - Root container
- `ComboboxTrigger` - Trigger button
- `ComboboxContent` - Dropdown content
- `ComboboxInput` - Search input
- `ComboboxItem` - Selectable item
- `ComboboxEmpty` - Empty state

#### Examples

```rust
<Combobox>
    <ComboboxTrigger>
        <span>"Select framework..."</span>
    </ComboboxTrigger>
    <ComboboxContent>
        <ComboboxInput placeholder="Search frameworks..." />
        <ComboboxItem value="next">"Next.js"</ComboboxItem>
        <ComboboxItem value="remix">"Remix"</ComboboxItem>
        <ComboboxItem value="astro">"Astro"</ComboboxItem>
        <ComboboxEmpty>"No framework found."</ComboboxEmpty>
    </ComboboxContent>
</Combobox>
```

---

### Carousel

Image/content carousel with navigation.

#### Import

```rust
use components::carousel::*;
```

#### Components

- `Carousel` - Root container
- `CarouselContent` - Slides container
- `CarouselItem` - Individual slide
- `CarouselPrevious` - Previous button
- `CarouselNext` - Next button

#### Examples

```rust
<Carousel class="w-full max-w-xs mx-auto">
    <CarouselContent>
        {(1..=5)
            .map(|i| {
                view! {
                    <CarouselItem>
                        <Card>
                            <CardContent class="flex aspect-square items-center justify-center p-6">
                                <span class="text-4xl font-semibold">{i}</span>
                            </CardContent>
                        </Card>
                    </CarouselItem>
                }
            })
            .collect_view()}
    </CarouselContent>
    <CarouselPrevious />
    <CarouselNext />
</Carousel>
```

---

## Utility Components

### Theme

Theme management and toggle functionality.

#### Import

```rust
use components::theme::{ThemeProvider, ThemeToggle};
```

#### Examples

```rust
// Wrap your app with ThemeProvider
<ThemeProvider>
    <div class="min-h-screen bg-background">
        // Your app content
    </div>
</ThemeProvider>

// Theme toggle button
<ThemeToggle size=ButtonSize::Sm variant=ButtonVariant::Outline>
    <span>"Toggle Theme"</span>
</ThemeToggle>
```

---

### Sidebar

Sidebar navigation component.

#### Import

```rust
use components::sidebar::*;
```

#### Components

- `SidebarProvider` - Context provider
- `Sidebar` - Main sidebar
- `SidebarContent` - Content area
- `SidebarHeader` - Header section
- `SidebarFooter` - Footer section
- `SidebarMenu` - Menu container
- `SidebarMenuItem` - Menu item

#### Examples

```rust
<SidebarProvider>
    <Sidebar>
        <SidebarContent>
            <SidebarMenu>
                <SidebarMenuItem>"Dashboard"</SidebarMenuItem>
                <SidebarMenuItem>"Projects"</SidebarMenuItem>
                <SidebarMenuItem>"Settings"</SidebarMenuItem>
            </SidebarMenu>
        </SidebarContent>
    </Sidebar>
    <main class="flex-1">
        // Main content
    </main>
</SidebarProvider>
```

---

## Combined Examples

### Login Form

```rust
let email = RwSignal::new(String::new());
let password = RwSignal::new(String::new());

<Card class="w-full max-w-md mx-auto">
    <CardHeader>
        <CardTitle>"Login"</CardTitle>
        <CardDescription>"Enter your credentials to continue"</CardDescription>
    </CardHeader>
    <CardContent class="space-y-4">
        <div class="space-y-2">
            <Label r#for="email">"Email"</Label>
            <Input
                attr:id="email"
                r#type="email".to_string()
                attr:placeholder="name@example.com"
                value=email
            />
        </div>
        <div class="space-y-2">
            <Label r#for="password">"Password"</Label>
            <Input
                attr:id="password"
                r#type="password".to_string()
                attr:placeholder="Enter your password"
                value=password
            />
        </div>
        <div class="flex items-center space-x-2">
            <Checkbox attr:id="remember" />
            <Label r#for="remember">"Remember me"</Label>
        </div>
    </CardContent>
    <CardFooter class="flex flex-col space-y-2">
        <Button class="w-full">"Sign In"</Button>
        <Button variant=ButtonVariant::Link class="w-full">"Forgot password?"</Button>
    </CardFooter>
</Card>
```

### Settings Panel

```rust
let notifications_enabled = RwSignal::new(true);
let theme = RwSignal::new("light".to_string());

<Card>
    <CardHeader>
        <CardTitle>"Settings"</CardTitle>
        <CardDescription>"Manage your application preferences"</CardDescription>
    </CardHeader>
    <CardContent class="space-y-6">
        <div class="space-y-4">
            <h3 class="text-lg font-semibold">"Notifications"</h3>
            <div class="flex items-center justify-between">
                <div class="space-y-0.5">
                    <Label>"Email Notifications"</Label>
                    <p class="text-sm text-muted-foreground">"Receive email updates"</p>
                </div>
                <Switch checked=notifications_enabled />
            </div>
        </div>

        <Separator />

        <div class="space-y-4">
            <h3 class="text-lg font-semibold">"Appearance"</h3>
            <div class="space-y-2">
                <Label>"Theme"</Label>
                <Select value=theme>
                    <SelectTrigger>
                        <SelectValue placeholder="Select theme" />
                    </SelectTrigger>
                    <SelectContent>
                        <SelectItem value="light">"Light"</SelectItem>
                        <SelectItem value="dark">"Dark"</SelectItem>
                        <SelectItem value="system">"System"</SelectItem>
                    </SelectContent>
                </Select>
            </div>
        </div>
    </CardContent>
    <CardFooter>
        <Button>"Save Changes"</Button>
    </CardFooter>
</Card>
```

---

## Common Props Reference

### OverlaySide (for positioning)

```rust
pub enum OverlaySide {
    Top,
    Right,
    Bottom,
    Left,
}
```

### OverlayAlign (for alignment)

```rust
pub enum OverlayAlign {
    Start,
    Center,
    End,
}
```

---

## Styling

All components use Tailwind CSS classes and support:
- **Dark mode** via `dark:` prefix classes
- **Focus states** via `focus-visible:` classes
- **Hover states** via `hover:` classes
- **Disabled states** via `disabled:` classes
- **Data attributes** for state-based styling (e.g., `data-[state=open]:`)

### Custom Classes

Every component accepts a `class` prop for additional styling:

```rust
<Button class="my-4 w-full">"Full Width Button"</Button>
<Card class="shadow-lg">"Custom Shadow Card"</Card>
```

---

## Accessibility

All components include:
- Proper ARIA attributes (`aria-expanded`, `aria-selected`, etc.)
- Keyboard navigation (Tab, Enter, Space, Escape, Arrow keys)
- Focus management
- Screen reader support (`role`, `aria-label`)
- `data-slot` attributes for styling hooks

---

## License

MIT License - See LICENSE file for details.
