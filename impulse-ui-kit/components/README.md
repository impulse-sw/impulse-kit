# Impulse UI Components

A comprehensive collection of 60 UI components for Leptos, inspired by shadcn/ui design principles. These components are built with accessibility, customization, and developer experience in mind.

## Table of Contents

- [Getting Started](#getting-started)
- [Basic Components](#basic-components)
- [Form Components](#form-components)
- [Layout Components](#layout-components)
- [Navigation Components](#navigation-components)
- [Overlay Components](#overlay-components)
- [Feedback Components](#feedback-components)
- [Data Display Components](#data-display-components)
- [Interactive Components](#interactive-components)
- [Utility Components](#utility-components)

## Getting Started

### Installation

Since Tailwind CSS compiler can't access the library's source code, you need to install components individually by copying them to your project.

#### Option 1: Using Deployer (Recommended)

Use Deployer to sync components with a simple command:

```bash
depl use -o src/components ui-kit-{component-name}@
```

#### Option 2: Manual Installation

1. Copy the desired component files from the library to your `src/components/` directory
2. Add impulse-ui-kit to your `Cargo.toml` for common utilities (cn, theming, etc.):

```toml
[dependencies]
impulse-ui-kit = { path = "../impulse-ui-kit" }
leptos = "0.7"
```

**Note:** Only use `impulse-ui-kit` as a dependency for utilities like `cn()` and theming functions, not for the components themselves.

### Usage

Import components directly from the modules:

```rust
use impulse_ui_kit::components::button::*;
use impulse_ui_kit::components::input::*;
use leptos::prelude::*;

#[component]
fn MyApp() -> impl IntoView {
    view! {
        <Button variant=ButtonVariant::Default>
            "Click me"
        </Button>
    }
}
```

### Common Patterns

All components follow these patterns:

- **Props**: Use `#[prop(optional)]` for optional properties
- **Styling**: Accept a `class` prop for custom CSS classes via Tailwind
- **State**: Use `RwSignal` for reactive state management
- **Callbacks**: Use `Callback<T>` for event handlers
- **Composition**: Nested components share context using Leptos context API

---

## Basic Components

### Button

A versatile button component with multiple variants and sizes.

**Props:**
- `variant`: `ButtonVariant` - Visual style (Default, Destructive, Outline, Secondary, Ghost, Link)
- `size`: `ButtonSize` - Size preset (Default, Sm, Lg, Icon, IconSm, IconLg)
- `class`: `Signal<String>` - Additional CSS classes
- `node_ref`: `NodeRef<Button>` - Reference to the button element
- `children`: Content to render inside the button

**Example:**
```rust
use impulse_ui_kit::components::button::*;

view! {
    <Button variant=ButtonVariant::Default size=ButtonSize::Middle>
        "Click me"
    </Button>

    <Button variant=ButtonVariant::Destructive>
        "Delete"
    </Button>

    <Button variant=ButtonVariant::Ghost size=ButtonSize::Icon>
        <svg>/* icon */</svg>
    </Button>
}
```

### Icon

Icon display component for rendering SVG icons.

**Props:**
- Component-specific props for icon rendering

### Label

Form label component with proper accessibility.

**Props:**
- `for`: `String` - ID of the associated form control
- `class`: `String` - Additional CSS classes
- `children`: Label content

**Example:**
```rust
use impulse_ui_kit::components::label::*;

view! {
    <Label r#for="email">"Email Address"</Label>
}
```

### Spinner

Loading indicator with size variants.

**Props:**
- `size`: `SpinnerSize` - Size variant (Sm, Default, Lg)
- `class`: `String` - Additional CSS classes

**Example:**
```rust
use impulse_ui_kit::components::spinner::*;

view! {
    <Spinner size=SpinnerSize::Default />
    <Spinner size=SpinnerSize::Lg class="text-primary" />
}
```

### Skeleton

Loading placeholder component for content.

**Props:**
- `class`: `String` - Additional CSS classes
- Component renders a placeholder box with loading animation

**Example:**
```rust
use impulse_ui_kit::components::skeleton::*;

view! {
    <Skeleton class="h-4 w-full" />
    <Skeleton class="h-12 w-12 rounded-full" />
}
```

### Badge

Status or label badge with variant styles.

**Props:**
- `variant`: `BadgeVariant` - Style variant (Default, Secondary, Destructive, Outline)
- `class`: `String` - Additional CSS classes
- `as_child`: `bool` - Render as child element instead of span
- `children`: Badge content

**Example:**
```rust
use impulse_ui_kit::components::badge::*;

view! {
    <Badge variant=BadgeVariant::Default>"New"</Badge>
    <Badge variant=BadgeVariant::Destructive>"Error"</Badge>
    <Badge variant=BadgeVariant::Outline>"Draft"</Badge>
}
```

### Kbd

Keyboard key display component.

**Props:**
- Display keyboard shortcuts in UI

---

## Form Components

### Input

Text input field with full styling and state management.

**Props:**
- `class`: `String` - Additional CSS classes
- `type`: `String` - Input type (text, email, password, etc.)
- `value`: `RwSignal<String>` - Reactive value binding

**Example:**
```rust
use impulse_ui_kit::components::input::*;

let email = RwSignal::new(String::new());

view! {
    <Input
        r#type="email".to_string()
        value=email
        class="w-full"
    />
}
```

### Textarea

Multi-line text input.

**Props:**
- `class`: `String` - Additional CSS classes
- `value`: `RwSignal<String>` - Reactive value binding
- Additional textarea-specific props

**Example:**
```rust
use impulse_ui_kit::components::textarea::*;

let description = RwSignal::new(String::new());

view! {
    <Textarea value=description />
}
```

### Checkbox

Checkbox control with checked state.

**Props:**
- `checked`: `Option<RwSignal<bool>>` - Controlled checked state
- `default_checked`: `bool` - Initial checked state
- `disabled`: `bool` - Disable interaction
- `class`: `String` - Additional CSS classes
- `on_change`: `Option<Callback<bool>>` - Change event handler

**Example:**
```rust
use impulse_ui_kit::components::checkbox::*;

let is_checked = RwSignal::new(false);

view! {
    <Checkbox
        checked=Some(is_checked)
        on_change=Some(Callback::new(move |checked| {
            console::log!("Checked: {}", checked);
        }))
    />
}
```

### Switch

Toggle switch component.

**Props:**
- `checked`: `Option<RwSignal<bool>>` - Controlled checked state
- `default_checked`: `bool` - Initial checked state
- `disabled`: `bool` - Disable interaction
- `class`: `String` - Additional CSS classes
- `on_change`: `Option<Callback<bool>>` - Change event handler

**Example:**
```rust
use impulse_ui_kit::components::switch::*;

let enabled = RwSignal::new(false);

view! {
    <Switch
        checked=Some(enabled)
        on_change=Some(Callback::new(move |checked| {
            console::log!("Enabled: {}", checked);
        }))
    />
}
```

### Radio Group

Radio button group for single selection.

**Props:**
- Group-level state management
- Individual radio items

**Example:**
```rust
use impulse_ui_kit::components::radio_group::*;

view! {
    <RadioGroup>
        <RadioGroupItem value="option1" />
        <RadioGroupItem value="option2" />
    </RadioGroup>
}
```

### Select

Dropdown select component with rich features.

**Composite Components:**
- `Select` - Root component
- `SelectTrigger` - Trigger button
- `SelectValue` - Selected value display
- `SelectContent` - Dropdown content
- `SelectGroup` - Option group
- `SelectLabel` - Group label
- `SelectItem` - Individual option
- `SelectSeparator` - Visual separator
- `SelectScrollUpButton` / `SelectScrollDownButton` - Scroll controls

**Props (Select):**
- `value`: `Option<RwSignal<String>>` - Controlled value
- `default_value`: `Option<String>` - Initial value
- `open`: `Option<RwSignal<bool>>` - Controlled open state
- `on_value_change`: `Option<Callback<String>>` - Value change handler

**Props (SelectTrigger):**
- `size`: `SelectTriggerSize` - Size variant (Sm, Default)
- `disabled`: `bool` - Disable interaction
- `class`: `String` - Additional CSS classes

**Props (SelectItem):**
- `value`: `String` - Item value
- `disabled`: `bool` - Disable item
- `class`: `String` - Additional CSS classes

**Example:**
```rust
use impulse_ui_kit::components::select::*;

let selected = RwSignal::new(String::new());

view! {
    <Select value=Some(selected)>
        <SelectTrigger>
            <SelectValue placeholder="Select an option" />
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
            </SelectGroup>
        </SelectContent>
    </Select>
}
```

### Native Select

Native HTML select element with styling.

**Props:**
- Standard select props with custom styling

### Slider

Range slider for numeric input.

**Props:**
- Min/max values
- Step size
- Value binding

### Form

Form wrapper component.

**Props:**
- Form-level validation and submission

### Field

Form field wrapper with label and error handling.

**Props:**
- Field-level validation state
- Label and error display

### Input Group

Grouped input fields.

**Props:**
- Group multiple related inputs

### Input OTP

One-time password input component.

**Props:**
- Specialized OTP entry with auto-focus

---

## Layout Components

### Card

Container component with header, content, and footer sections.

**Composite Components:**
- `Card` - Root container
- `CardHeader` - Header section
- `CardTitle` - Title text
- `CardDescription` - Description text
- `CardAction` - Action buttons
- `CardContent` - Main content
- `CardFooter` - Footer section

**Example:**
```rust
use impulse_ui_kit::components::card::*;

view! {
    <Card>
        <CardHeader>
            <CardTitle>"Card Title"</CardTitle>
            <CardDescription>"Card description text"</CardDescription>
            <CardAction>
                <Button>"Action"</Button>
            </CardAction>
        </CardHeader>
        <CardContent>
            "Main content goes here"
        </CardContent>
        <CardFooter>
            "Footer content"
        </CardFooter>
    </Card>
}
```

### Separator

Divider line component.

**Props:**
- `orientation`: `SeparatorOrientation` - Horizontal or Vertical
- `decorative`: `bool` - Whether separator is decorative (default: true)
- `class`: `String` - Additional CSS classes

**Example:**
```rust
use impulse_ui_kit::components::separator::*;

view! {
    <div class="space-y-4">
        <div>"Section 1"</div>
        <Separator orientation=SeparatorOrientation::Horizontal />
        <div>"Section 2"</div>
    </div>
}
```

### Tabs

Tabbed interface component.

**Composite Components:**
- `Tabs` - Root component
- `TabsList` - Tab button container
- `TabsTrigger` - Individual tab button
- `TabsContent` - Tab panel content

**Props (Tabs):**
- `value`: `Option<RwSignal<String>>` - Controlled active tab
- `default_value`: `String` - Initial active tab
- `on_value_change`: `Option<Callback<String>>` - Tab change handler
- `class`: `String` - Additional CSS classes

**Props (TabsTrigger):**
- `value`: `String` - Tab identifier
- `disabled`: `bool` - Disable tab
- `class`: `String` - Additional CSS classes

**Props (TabsContent):**
- `value`: `String` - Tab identifier (shows when active)
- `class`: `String` - Additional CSS classes

**Example:**
```rust
use impulse_ui_kit::components::tabs::*;

view! {
    <Tabs default_value="account">
        <TabsList>
            <TabsTrigger value="account">"Account"</TabsTrigger>
            <TabsTrigger value="password">"Password"</TabsTrigger>
            <TabsTrigger value="settings">"Settings"</TabsTrigger>
        </TabsList>
        <TabsContent value="account">
            "Account settings content"
        </TabsContent>
        <TabsContent value="password">
            "Password settings content"
        </TabsContent>
        <TabsContent value="settings">
            "General settings content"
        </TabsContent>
    </Tabs>
}
```

### Accordion

Collapsible sections component.

**Composite Components:**
- `Accordion` - Root component
- `AccordionItem` - Individual item
- `AccordionTrigger` - Collapsible trigger
- `AccordionContent` - Collapsible content

**Props (Accordion):**
- `accordion_type`: `AccordionType` - Single or Multiple (allow multiple open)
- `default_value`: `Option<Vec<String>>` - Initially open items
- `value`: `Option<RwSignal<Vec<String>>>` - Controlled open items
- `class`: `String` - Additional CSS classes

**Props (AccordionItem):**
- `value`: `String` - Item identifier
- `class`: `String` - Additional CSS classes

**Example:**
```rust
use impulse_ui_kit::components::accordion::*;

view! {
    <Accordion accordion_type=AccordionType::Single>
        <AccordionItem value="item-1">
            <AccordionTrigger>"Is it accessible?"</AccordionTrigger>
            <AccordionContent>
                "Yes. It adheres to WAI-ARIA design patterns."
            </AccordionContent>
        </AccordionItem>
        <AccordionItem value="item-2">
            <AccordionTrigger>"Is it styled?"</AccordionTrigger>
            <AccordionContent>
                "Yes. It comes with default styles."
            </AccordionContent>
        </AccordionItem>
    </Accordion>
}
```

### Collapsible

Single collapsible section.

**Props:**
- Similar to accordion but for single items
- Controlled open/close state

### Aspect Ratio

Container that maintains aspect ratio.

**Props:**
- Ratio specification
- Responsive sizing

### Scroll Area

Custom styled scrollable area.

**Props:**
- Custom scrollbar styling
- Scroll behavior controls

### Resizable

Resizable panels.

**Props:**
- Drag handles for resizing
- Min/max constraints

### Sidebar

Sidebar navigation component.

**Props:**
- Collapsible sidebar
- Navigation structure

### Sheet

Side panel/drawer component.

**Props:**
- Slide-in panel from edges
- Overlay backdrop

### Drawer

Slide-out panel component.

**Props:**
- Similar to Sheet
- Mobile-optimized

---

## Navigation Components

### Navigation Menu

Main navigation menu component.

**Props:**
- Hierarchical menu structure
- Dropdown submenus

### Menubar

Menu bar component.

**Props:**
- Horizontal menu bar
- Multiple menu groups

### Breadcrumb

Breadcrumb navigation.

**Props:**
- Path navigation
- Current location indicator

### Pagination

Page navigation component.

**Props:**
- Page numbers
- Next/Previous controls
- Jump to page

### Button Group

Grouped buttons.

**Props:**
- Related buttons grouped together
- Shared styling

---

## Overlay Components

### Dialog

Modal dialog component.

**Composite Components:**
- `Dialog` - Root component
- `DialogTrigger` - Opens dialog
- `DialogContent` - Dialog content
- `DialogHeader` - Header section
- `DialogTitle` - Title text
- `DialogDescription` - Description text
- `DialogFooter` - Footer section
- `DialogClose` - Close button
- `DialogOverlay` - Background overlay

**Props (Dialog):**
- `open`: `Option<RwSignal<bool>>` - Controlled open state
- `default_open`: `Option<bool>` - Initial open state
- `on_open_change`: `Option<Callback<bool>>` - Open state change handler

**Example:**
```rust
use impulse_ui_kit::components::dialog::*;

let is_open = RwSignal::new(false);

view! {
    <Dialog open=Some(is_open)>
        <DialogTrigger>
            <Button>"Open Dialog"</Button>
        </DialogTrigger>
        <DialogContent>
            <DialogHeader>
                <DialogTitle>"Dialog Title"</DialogTitle>
                <DialogDescription>"Dialog description"</DialogDescription>
            </DialogHeader>
            <div>"Dialog content goes here"</div>
            <DialogFooter>
                <DialogClose>
                    <Button variant=ButtonVariant::Ghost>"Cancel"</Button>
                </DialogClose>
                <Button>"Confirm"</Button>
            </DialogFooter>
        </DialogContent>
    </Dialog>
}
```

### Alert Dialog

Confirmation dialog.

**Props:**
- Similar to Dialog
- For destructive or important actions
- Requires explicit confirmation

### Popover

Popup content component.

**Props:**
- Positioning relative to trigger
- Click or hover triggers
- Auto-dismiss behavior

### Tooltip

Hover tooltip component.

**Composite Components:**
- `TooltipProvider` - Context provider
- `Tooltip` - Root component
- `TooltipTrigger` - Hover target
- `TooltipContent` - Tooltip content

**Props (Tooltip):**
- `open`: `Option<RwSignal<bool>>` - Controlled open state

**Example:**
```rust
use impulse_ui_kit::components::tooltip::*;

view! {
    <TooltipProvider>
        <Tooltip>
            <TooltipTrigger>
                <Button>"Hover me"</Button>
            </TooltipTrigger>
            <TooltipContent>
                "Tooltip text here"
            </TooltipContent>
        </Tooltip>
    </TooltipProvider>
}
```

### Hover Card

Hover preview card.

**Props:**
- Rich content on hover
- Delayed appearance
- Positioning options

### Dropdown Menu

Dropdown menu component.

**Props:**
- Menu items
- Submenus
- Keyboard navigation

### Context Menu

Right-click context menu.

**Props:**
- Right-click trigger
- Menu items
- Nested menus

---

## Feedback Components

### Alert

Alert message component.

**Composite Components:**
- `Alert` - Root container
- `AlertTitle` - Title text
- `AlertDescription` - Description text

**Props (Alert):**
- `variant`: `AlertVariant` - Style variant (Default, Destructive)
- `class`: `String` - Additional CSS classes

**Example:**
```rust
use impulse_ui_kit::components::alert::*;

view! {
    <Alert variant=AlertVariant::Default>
        <svg>/* icon */</svg>
        <AlertTitle>"Heads up!"</AlertTitle>
        <AlertDescription>
            "You can add components to your app using the cli."
        </AlertDescription>
    </Alert>

    <Alert variant=AlertVariant::Destructive>
        <AlertTitle>"Error"</AlertTitle>
        <AlertDescription>"Your session has expired."</AlertDescription>
    </Alert>
}
```

### Toast

Toast notification component.

**Props:**
- Temporary notification
- Auto-dismiss
- Position variants

### Sonner

Toast library integration (Sonner).

**Props:**
- Advanced toast features
- Queue management
- Promise-based toasts

### Progress

Progress bar component.

**Props:**
- `value`: `Signal<f64>` - Current progress value
- `max`: `Option<f64>` - Maximum value (default: 100)
- `class`: `String` - Additional CSS classes

**Example:**
```rust
use impulse_ui_kit::components::progress::*;

let progress = Signal::derive(move || 75.0);

view! {
    <Progress value=progress max=Some(100.0) />
}
```

---

## Data Display Components

### Table

Basic table component.

**Props:**
- Table structure
- Header and body
- Row styling

### Data Table

Advanced data table with features.

**Props:**
- Sorting
- Filtering
- Pagination
- Column management

### Calendar

Calendar component.

**Props:**
- Date display
- Month navigation
- Date selection

### Date Picker

Date selection component.

**Props:**
- Calendar popup
- Date input
- Range selection

### Avatar

User avatar component.

**Composite Components:**
- `Avatar` - Root container
- `AvatarImage` - Image element
- `AvatarFallback` - Fallback display

**Example:**
```rust
use impulse_ui_kit::components::avatar::*;

view! {
    <Avatar>
        <AvatarImage src="https://github.com/user.png" alt="User" />
        <AvatarFallback>"UN"</AvatarFallback>
    </Avatar>
}
```

---

## Interactive Components

### Command

Command palette component.

**Props:**
- Keyboard shortcuts
- Command search
- Action execution

### Combobox

Searchable select component.

**Props:**
- Autocomplete
- Search filtering
- Custom rendering

### Toggle

Toggle button component.

**Props:**
- On/off state
- Icon support
- Variants

### Toggle Group

Group of toggle buttons.

**Props:**
- Single or multiple selection
- Group behavior

### Carousel

Image/content carousel.

**Props:**
- Slide navigation
- Auto-play
- Indicators

### Empty

Empty state component.

**Props:**
- Empty state display
- Call-to-action
- Illustrations

---

## Utility Components

### Theme

Theme management component.

**Props:**
- Theme switching
- CSS variable management
- Dark/light mode

### Item

Generic list item component.

**Props:**
- Flexible list item
- Icon support
- Action handling

---

## Styling and Customization

All components use Tailwind CSS for styling and accept a `class` prop for customization:

```rust
view! {
    <Button class="bg-custom-color hover:bg-custom-color-dark">
        "Custom Button"
    </Button>
}
```

### Theme Variables

Components use CSS custom properties for theming:
- `--primary`: Primary color
- `--secondary`: Secondary color
- `--destructive`: Destructive action color
- `--muted`: Muted text color
- `--accent`: Accent color
- `--border`: Border color
- And many more...

## Accessibility

All components follow WAI-ARIA best practices:
- Proper ARIA attributes
- Keyboard navigation
- Focus management
- Screen reader support

## Best Practices

1. **State Management**: Use `RwSignal` for reactive state
2. **Callbacks**: Use `Callback` for event handlers
3. **Composition**: Combine components using the context API
4. **Styling**: Use the `class` prop for custom styling
5. **Accessibility**: Always provide labels and ARIA attributes
6. **Performance**: Use `StoredValue` and `Memo` for optimization

## Examples

### Login Form

```rust
use impulse_ui_kit::components::*;

#[component]
fn LoginForm() -> impl IntoView {
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());

    view! {
        <Card class="w-full max-w-md">
            <CardHeader>
                <CardTitle>"Login"</CardTitle>
                <CardDescription>"Enter your credentials"</CardDescription>
            </CardHeader>
            <CardContent class="space-y-4">
                <div class="space-y-2">
                    <Label r#for="email">"Email"</Label>
                    <Input
                        r#type="email".to_string()
                        value=email
                    />
                </div>
                <div class="space-y-2">
                    <Label r#for="password">"Password"</Label>
                    <Input
                        r#type="password".to_string()
                        value=password
                    />
                </div>
            </CardContent>
            <CardFooter>
                <Button class="w-full">"Sign In"</Button>
            </CardFooter>
        </Card>
    }
}
```

### Settings Page

```rust
use impulse_ui_kit::components::*;

#[component]
fn SettingsPage() -> impl IntoView {
    let notifications = RwSignal::new(true);
    let theme = RwSignal::new("light".to_string());

    view! {
        <div class="space-y-6">
            <div>
                <h1 class="text-2xl font-bold">"Settings"</h1>
                <p class="text-muted-foreground">"Manage your account settings"</p>
            </div>

            <Separator />

            <Card>
                <CardHeader>
                    <CardTitle>"Notifications"</CardTitle>
                    <CardDescription>"Configure notification preferences"</CardDescription>
                </CardHeader>
                <CardContent class="space-y-4">
                    <div class="flex items-center justify-between">
                        <Label>"Enable notifications"</Label>
                        <Switch checked=Some(notifications) />
                    </div>
                </CardContent>
            </Card>

            <Card>
                <CardHeader>
                    <CardTitle>"Appearance"</CardTitle>
                    <CardDescription>"Customize the appearance"</CardDescription>
                </CardHeader>
                <CardContent>
                    <Select value=Some(theme)>
                        <SelectTrigger>
                            <SelectValue placeholder="Select theme" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="light">"Light"</SelectItem>
                            <SelectItem value="dark">"Dark"</SelectItem>
                            <SelectItem value="system">"System"</SelectItem>
                        </SelectContent>
                    </Select>
                </CardContent>
            </Card>
        </div>
    }
}
```

## Contributing

When contributing new components:
1. Follow existing patterns and conventions
2. Include proper prop documentation
3. Add accessibility features
4. Write comprehensive examples
5. Test with different variants

## License

[Add your license information here]
