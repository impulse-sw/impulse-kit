//! Impulse UI Kit - Comprehensive Component Showcase
//!
//! This example demonstrates all 50+ components available in the Impulse UI Kit.

#![allow(unused_imports)]
#![warn(missing_docs)]
#![deny(clippy::todo, clippy::unimplemented)]

use impulse_ui_kit_components as components;

use impulse_ui_kit::prelude::*;
use impulse_ui_kit::utils::{OverlayAlign, OverlaySide};
use leptos_use::{BreakpointsTailwind, breakpoints_tailwind, use_breakpoints};

// Basic Components
use components::badge::{Badge, BadgeVariant};
use components::button::{Button, ButtonSize, ButtonVariant};
use components::icon::*;
use components::label::Label;
use components::skeleton::Skeleton;
use components::spinner::{Spinner, SpinnerSize};

// Form Components
use components::checkbox::Checkbox;
use components::form::*;
use components::input::Input;
use components::input_otp::{InputOTP, InputOTPWithSeparator};
use components::radio_group::*;
use components::select::*;
use components::slider::Slider;
use components::switch::Switch;
use components::textarea::Textarea;

// Layout Components
use components::accordion::*;
use components::aspect_ratio::AspectRatio;
use components::card::*;
use components::collapsible::*;
use components::drawer::*;
use components::resizable::*;
use components::scroll_area::*;
use components::separator::{Separator, SeparatorOrientation};
use components::sheet::*;
use components::sidebar::*;
use components::tabs::*;

// Navigation Components
use components::breadcrumb::*;
use components::button_group::*;

// Overlay Components
use components::alert_dialog::*;
use components::context_menu::*;
use components::dialog::*;
use components::dropdown_menu::*;
use components::popover::{Popover, PopoverContent, PopoverTrigger};
use components::tooltip::*;

// Feedback Components
use components::alert::*;
use components::progress::Progress;
use components::sonner::*;
use components::toast::*;

// Data Display Components
use components::avatar::*;
use components::calendar::*;
use components::data_table::*;
use components::date_picker::*;
use components::table::*;

// Interactive Components
use components::carousel::*;
use components::combobox::*;
use components::command::*;
use components::toggle::*;
use components::toggle_group::*;

// Utility Components
use crate::components::theme::{ThemeProvider, ThemeToggle};

// Blocks
use impulse_ui_kit_blocks::charts::{BarChart, BarChartData, BarChartOptions, BarSeries};
use impulse_ui_kit_blocks::markdown::{Markdown, MarkdownClasses, MarkdownSource};

fn main() {
  setup_app(log::Level::Info, Box::new(move || view! { <App /> }.into_any()))
}

#[component]
fn App() -> impl IntoView {
  let active_section = RwSignal::new("basic".to_string());

  view! {
    <ThemeProvider>
      <div class="min-h-screen bg-background">
        // Header
        <header class="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
          <div class="px-4 flex h-14 items-center space-x-2 justify-between">
            <a href="/">
              <span class="font-bold text-xl">"Impulse UI Kit"</span>
            </a>
            <nav class="flex items-center space-x-2">
              <ThemeToggle size=ButtonSize::Sm variant=ButtonVariant::Outline>
                <span>"Change theme"</span>
              </ThemeToggle>
            </nav>
          </div>
        </header>

        // Main Content
        <div class="container mx-auto py-8">
          <div class="flex flex-col space-y-2">
            // Title and Description
            <div class="space-y-2">
              <h1 class="text-4xl font-bold tracking-tight">"Component Showcase"</h1>
              <p class="text-muted-foreground text-lg">"Explore Impulse UI Kit components"</p>
            </div>

            // Navigation Tabs
            <Tabs
              default_value="basic"
              on_value_change=Callback::new(move |v| active_section.set(v))
            >
              <TabsList class="w-full">
                <TabsTrigger value="basic">"Basic"</TabsTrigger>
                <TabsTrigger value="forms">"Forms"</TabsTrigger>
                <TabsTrigger value="layout">"Layout"</TabsTrigger>
                <TabsTrigger value="navigation">"Navigation"</TabsTrigger>
                <TabsTrigger value="overlay">"Overlay"</TabsTrigger>
                <TabsTrigger value="feedback">"Feedback"</TabsTrigger>
                <TabsTrigger value="data">"Data"</TabsTrigger>
                <TabsTrigger value="interactive">"Interactive"</TabsTrigger>
                <TabsTrigger value="utility">"Utility"</TabsTrigger>
                <TabsTrigger value="blocks">"Blocks"</TabsTrigger>
                <TabsTrigger value="combined">"Combined"</TabsTrigger>
              </TabsList>

              // Basic Components Section
              <TabsContent value="basic">
                <BasicComponentsSection />
              </TabsContent>

              // Form Components Section
              <TabsContent value="forms">
                <FormComponentsSection />
              </TabsContent>

              // Layout Components Section
              <TabsContent value="layout">
                <LayoutComponentsSection />
              </TabsContent>

              // Navigation Components Section
              <TabsContent value="navigation">
                <NavigationComponentsSection />
              </TabsContent>

              // Overlay Components Section
              <TabsContent value="overlay">
                <OverlayComponentsSection />
              </TabsContent>

              // Feedback Components Section
              <TabsContent value="feedback">
                <FeedbackComponentsSection />
              </TabsContent>

              // Data Display Components Section
              <TabsContent value="data">
                <DataDisplayComponentsSection />
              </TabsContent>

              // Interactive Components Section
              <TabsContent value="interactive">
                <InteractiveComponentsSection />
              </TabsContent>

              // Utility Components Section
              <TabsContent value="utility">
                <UtilityComponentsSection />
              </TabsContent>

              // Blocks Section
              <TabsContent value="blocks">
                <BlocksSection />
              </TabsContent>

              // Combined Examples Section
              <TabsContent value="combined">
                <CombinedExamplesSection />
              </TabsContent>
            </Tabs>
          </div>
        </div>
      </div>
    </ThemeProvider>
  }
}

#[component]
fn BasicComponentsSection() -> impl IntoView {
  view! {
    <div class="space-y-8">
      <SectionHeader title="Basic Components" description="Fundamental UI building blocks" />

      // Button
      <ComponentCard title="Button" description="Versatile button with multiple variants and sizes">
        <div class="flex flex-wrap gap-4">
          <Button variant=ButtonVariant::Default>"Default"</Button>
          <Button variant=ButtonVariant::Destructive>"Destructive"</Button>
          <Button variant=ButtonVariant::Outline>"Outline"</Button>
          <Button variant=ButtonVariant::Secondary>"Secondary"</Button>
          <Button variant=ButtonVariant::Ghost>"Ghost"</Button>
          <Button variant=ButtonVariant::Link>"Link"</Button>
        </div>
        <Separator orientation=SeparatorOrientation::Horizontal />
        <div class="flex flex-wrap gap-4">
          <Button size=ButtonSize::Sm>"Small"</Button>
          <Button size=ButtonSize::Middle>"Middle"</Button>
          <Button size=ButtonSize::Lg>"Large"</Button>
          <Button size=ButtonSize::Icon>"→"</Button>
        </div>
      </ComponentCard>

      // Badge
      <ComponentCard title="Badge" description="Status or label badge with variant styles">
        <div class="flex flex-wrap gap-4">
          <Badge variant=BadgeVariant::Default>"Default"</Badge>
          <Badge variant=BadgeVariant::Secondary>"Secondary"</Badge>
          <Badge variant=BadgeVariant::Destructive>"Destructive"</Badge>
          <Badge variant=BadgeVariant::Outline>"Outline"</Badge>
        </div>
      </ComponentCard>

      // Label
      <ComponentCard title="Label" description="Form label with proper accessibility">
        <div class="space-y-2">
          <Label r#for="example-input">"Email Address"</Label>
          <Input r#type="email".to_string() attr:id="example-input" />
        </div>
      </ComponentCard>

      // Spinner
      <ComponentCard title="Spinner" description="Loading indicator with size variants">
        <div class="flex flex-wrap gap-8 items-center">
          <Spinner size=SpinnerSize::Sm />
          <Spinner size=SpinnerSize::Default />
          <Spinner size=SpinnerSize::Lg />
        </div>
      </ComponentCard>

      // Skeleton
      <ComponentCard title="Skeleton" description="Loading placeholder for content">
        <div class="space-y-4">
          <Skeleton class="h-12 w-12 rounded-full" />
          <Skeleton class="h-4 w-full" />
          <Skeleton class="h-4 w-3/4" />
          <Skeleton class="h-4 w-1/2" />
        </div>
      </ComponentCard>
    </div>
  }
}

#[component]
fn FormComponentsSection() -> impl IntoView {
  let email = RwSignal::new(String::new());
  let description = RwSignal::new(String::new());
  let is_checked = RwSignal::new(false);
  let is_enabled = RwSignal::new(true);
  let selected_option = RwSignal::new(String::new());
  let slider_value = RwSignal::new(50.0);
  let otp_code = RwSignal::new(String::new());

  view! {
    <div class="space-y-8">
      <SectionHeader title="Form Components" description="Interactive form controls and inputs" />

      // Input
      <ComponentCard title="Input" description="Text input field with full styling">
        <div class="space-y-4">
          <Input r#type="email".to_string() attr:placeholder="Enter your email" value=email />
          <Input r#type="password".to_string() attr:placeholder="Enter your password" />
        </div>
      </ComponentCard>

      // Textarea
      <ComponentCard title="Textarea" description="Multi-line text input">
        <Textarea value=description placeholder="Enter description..." class="min-h-[100px]" />
      </ComponentCard>

      // Checkbox
      <ComponentCard title="Checkbox" description="Checkbox control with checked state">
        <div class="flex items-center space-x-2">
          <Checkbox checked=is_checked attr:id="terms" />
          <Label r#for="terms">"Accept terms and conditions"</Label>
        </div>
      </ComponentCard>

      // Switch
      <ComponentCard title="Switch" description="Toggle switch component">
        <div class="flex items-center space-x-2">
          <Switch checked=is_enabled attr:id="notifications" />
          <Label r#for="notifications">"Enable notifications"</Label>
        </div>
      </ComponentCard>

      // Radio Group
      <ComponentCard title="Radio Group" description="Radio button group for single selection">
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
      </ComponentCard>

      // Select
      <ComponentCard title="Select" description="Dropdown select with rich features">
        <Select value=selected_option>
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
              <SelectItem value="potato">"Potato"</SelectItem>
            </SelectGroup>
          </SelectContent>
        </Select>
      </ComponentCard>

      // Slider
      <ComponentCard title="Slider" description="Range slider for numeric input">
        <div class="space-y-4">
          <Slider value=slider_value min=0.0 max=100.0 step=1.0 />
          <p class="text-sm text-muted-foreground">
            "Value: " {move || format!("{:.0}", slider_value.get())}
          </p>
        </div>
      </ComponentCard>

      // Input OTP
      <ComponentCard title="Input OTP" description="One-time password input">
        <div class="space-y-4">
          <InputOTP length=6usize on_complete=Callback::new(move |code| otp_code.set(code)) />
          <InputOTPWithSeparator length=6usize separator_at=3usize />
        </div>
      </ComponentCard>
    </div>
  }
}

#[component]
fn LayoutComponentsSection() -> impl IntoView {
  view! {
    <div class="space-y-8">
      <SectionHeader title="Layout Components" description="Components for structuring your UI" />

      // Card
      <ComponentCard title="Card" description="Container with header, content, and footer">
        <Card>
          <CardHeader>
            <CardTitle>"Card Title"</CardTitle>
            <CardDescription>"This is a card description"</CardDescription>
          </CardHeader>
          <CardContent>
            <p>"Main content goes here. Cards are great for organizing related information."</p>
          </CardContent>
          <CardFooter>
            <Button variant=ButtonVariant::Outline>"Cancel"</Button>
            <Button>"Save"</Button>
          </CardFooter>
        </Card>
      </ComponentCard>

      // Separator
      <ComponentCard title="Separator" description="Visual divider line">
        <div class="space-y-4">
          <div>"Content above"</div>
          <Separator orientation=SeparatorOrientation::Horizontal />
          <div>"Content below"</div>
        </div>
      </ComponentCard>

      // Accordion
      <ComponentCard title="Accordion" description="Collapsible sections">
        <Accordion accordion_type=AccordionType::Single default_value=vec!["item-1".to_string()]>
          <AccordionItem value="item-1">
            <AccordionTrigger>"Is it accessible?"</AccordionTrigger>
            <AccordionContent>"Yes. It adheres to WAI-ARIA design patterns."</AccordionContent>
          </AccordionItem>
          <AccordionItem value="item-2">
            <AccordionTrigger>"Is it styled?"</AccordionTrigger>
            <AccordionContent>
              "Yes. It comes with default styles that match your theme."
            </AccordionContent>
          </AccordionItem>
          <AccordionItem value="item-3">
            <AccordionTrigger>"Is it animated?"</AccordionTrigger>
            <AccordionContent>"Yes. Smooth animations are included by default."</AccordionContent>
          </AccordionItem>
        </Accordion>
      </ComponentCard>

      // Collapsible
      <ComponentCard title="Collapsible" description="Single collapsible section">
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
      </ComponentCard>

      // Aspect Ratio
      <ComponentCard title="Aspect Ratio" description="Container that maintains aspect ratio">
        <AspectRatio ratio=RwSignal::new(16.0 / 9.0)>
          <div class="w-full h-full bg-muted rounded-md flex items-center justify-center">
            <span class="text-muted-foreground">"16:9 Aspect Ratio"</span>
          </div>
        </AspectRatio>
      </ComponentCard>

      // Scroll Area
      <ComponentCard title="Scroll Area" description="Custom styled scrollable area">
        <ScrollArea class="h-[200px] w-full rounded-md border p-4">
          {(0..20)
            .map(|i| {
              view! { <div class="mb-4">{format!("Item {}", i + 1)}</div> }
            })
            .collect_view()}
        </ScrollArea>
      </ComponentCard>

      // Resizable
      <ComponentCard title="Resizable" description="Resizable panels with drag handles">
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
      </ComponentCard>
    </div>
  }
}

#[component]
fn NavigationComponentsSection() -> impl IntoView {
  view! {
    <div class="space-y-8">
      <SectionHeader
        title="Navigation Components"
        description="Components for navigation and wayfinding"
      />

      // Breadcrumb
      <ComponentCard title="Breadcrumb" description="Hierarchical navigation">
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
      </ComponentCard>

      // Button Group
      <ComponentCard title="Button Group" description="Related buttons grouped together">
        <ButtonGroup>
          <Button variant=ButtonVariant::Outline>"Left"</Button>
          <Button variant=ButtonVariant::Outline>"Center"</Button>
          <Button variant=ButtonVariant::Outline>"Right"</Button>
        </ButtonGroup>
      </ComponentCard>

      // Tabs (nested example)
      <ComponentCard title="Tabs" description="Tabbed interface">
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
      </ComponentCard>
    </div>
  }
}

#[component]
fn OverlayComponentsSection() -> impl IntoView {
  let dialog_open = RwSignal::new(false);
  let alert_dialog_open = RwSignal::new(false);
  let drawer_open = RwSignal::new(false);
  let sheet_open = RwSignal::new(false);

  view! {
    <div class="space-y-8">
      <SectionHeader title="Overlay Components" description="Modal and popup components" />

      // Dialog
      <ComponentCard title="Dialog" description="Modal dialog component">
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
      </ComponentCard>

      // Alert Dialog
      <ComponentCard title="Alert Dialog" description="Confirmation dialog for important actions">
        <AlertDialog open=alert_dialog_open>
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
      </ComponentCard>

      // Popover
      <ComponentCard title="Popover" description="Popup content component">
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
      </ComponentCard>

      // Tooltip
      <ComponentCard title="Tooltip" description="Hover tooltip component">
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
      </ComponentCard>

      // Dropdown Menu
      <ComponentCard title="Dropdown Menu" description="Dropdown menu with items">
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
            <DropdownMenuItem>"Logout"</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </ComponentCard>

      // Context Menu
      <ComponentCard title="Context Menu" description="Right-click context menu">
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
      </ComponentCard>

      // Drawer
      <ComponentCard title="Drawer" description="Slide-out drawer panel">
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
      </ComponentCard>

      // Sheet
      <ComponentCard title="Sheet" description="Side panel component">
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
      </ComponentCard>
    </div>
  }
}

#[component]
fn FeedbackComponentsSection() -> impl IntoView {
  let progress_value = Signal::derive(move || 65.0);

  view! {
    <div class="space-y-8">
      <SectionHeader
        title="Feedback Components"
        description="Components for user feedback and notifications"
      />

      // Alert
      <ComponentCard title="Alert" description="Alert message component">
        <div class="space-y-4">
          <Alert variant=AlertVariant::Default>
            <AlertTitle>"Info"</AlertTitle>
            <AlertDescription>"This is an informational alert message."</AlertDescription>
          </Alert>
          <Alert variant=AlertVariant::Destructive>
            <AlertTitle>"Error"</AlertTitle>
            <AlertDescription>"This is an error alert message."</AlertDescription>
          </Alert>
        </div>
      </ComponentCard>

      // Toast
      <ComponentCard title="Toast" description="Toast notification component">
        <Toast>
          <ToastTitle>"Toast Title"</ToastTitle>
          <ToastDescription>"This is a toast notification"</ToastDescription>
          <ToastClose />
        </Toast>
      </ComponentCard>

      // Progress
      <ComponentCard title="Progress" description="Progress bar component">
        <div class="space-y-2">
          <Progress value=progress_value />
          <p class="text-sm text-muted-foreground">
            {move || format!("Progress: {:.0}%", progress_value.get())}
          </p>
        </div>
      </ComponentCard>

      // Spinner (shown again in context)
      <ComponentCard title="Spinner" description="Loading spinner">
        <div class="flex items-center gap-4">
          <Spinner size=SpinnerSize::Default />
          <span>"Loading..."</span>
        </div>
      </ComponentCard>
    </div>
  }
}

#[component]
fn DataDisplayComponentsSection() -> impl IntoView {
  let selected_date = RwSignal::new(CalendarSelection::None);

  view! {
    <div class="space-y-8">
      <SectionHeader title="Data Display Components" description="Components for displaying data" />

      // Table
      <ComponentCard title="Table" description="Basic table component">
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
      </ComponentCard>

      // Avatar
      <ComponentCard title="Avatar" description="User avatar component">
        <div class="flex gap-4">
          <Avatar>
            <AvatarImage attr:src="https://github.com/shadcn.png" attr:alt="User" />
            <AvatarFallback>"CN"</AvatarFallback>
          </Avatar>
          <Avatar>
            <AvatarFallback>"JD"</AvatarFallback>
          </Avatar>
        </div>
      </ComponentCard>

      // Calendar
      <ComponentCard title="Calendar" description="Calendar date picker">
        <Calendar selected=selected_date class="rounded-md border" />
      </ComponentCard>
    </div>
  }
}

#[component]
fn InteractiveComponentsSection() -> impl IntoView {
  let toggle_pressed = RwSignal::new(false);

  view! {
    <div class="space-y-8">
      <SectionHeader title="Interactive Components" description="Advanced interactive components" />

      // Command
      <ComponentCard title="Command" description="Command palette">
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
      </ComponentCard>

      // Combobox
      <ComponentCard title="Combobox" description="Searchable select component">
        <Combobox>
          <ComboboxTrigger placeholder="Select option..." />
          <ComboboxContent>
            <ComboboxInput placeholder="Search..." />
            <ComboboxItem value="option1" label="Option 1">
              "Option 1"
            </ComboboxItem>
            <ComboboxItem value="option2" label="Option 2">
              "Option 2"
            </ComboboxItem>
            <ComboboxItem value="option3" label="Option 3">
              "Option 3"
            </ComboboxItem>
            <ComboboxEmpty>"No option found."</ComboboxEmpty>
          </ComboboxContent>
        </Combobox>
      </ComponentCard>

      // Toggle
      <ComponentCard title="Toggle" description="Toggle button">
        <div class="flex gap-4">
          <Toggle pressed=toggle_pressed>"Toggle"</Toggle>
          <Toggle variant=ToggleVariant::Outline>"Outline"</Toggle>
        </div>
      </ComponentCard>

      // Toggle Group
      <ComponentCard title="Toggle Group" description="Group of toggle buttons">
        <ToggleGroup r#type=ToggleGroupType::Single default_value=vec!["center".to_string()]>
          <ToggleGroupItem value="left">"Left"</ToggleGroupItem>
          <ToggleGroupItem value="center">"Center"</ToggleGroupItem>
          <ToggleGroupItem value="right">"Right"</ToggleGroupItem>
        </ToggleGroup>
      </ComponentCard>

      // Carousel
      <ComponentCard title="Carousel" description="Image/content carousel">
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
      </ComponentCard>
    </div>
  }
}

#[component]
fn UtilityComponentsSection() -> impl IntoView {
  view! {
    <div class="space-y-8">
      <SectionHeader title="Utility Components" description="Utility and helper components" />

      // Theme
      <ComponentCard title="Theme" description="Theme management and toggle">
        <div class="flex items-center gap-4">
          <span>"Current theme controls:"</span>
          <ThemeToggle>
            <span>"Toggle Theme"</span>
          </ThemeToggle>
        </div>
      </ComponentCard>

      // Sidebar
      <ComponentCard title="Sidebar" description="Sidebar navigation component">
        <div class="h-[300px] border rounded-md overflow-hidden">
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
          </SidebarProvider>
        </div>
      </ComponentCard>
    </div>
  }
}

#[component]
fn CombinedExamplesSection() -> impl IntoView {
  let login_email = RwSignal::new(String::new());
  let login_password = RwSignal::new(String::new());
  let settings_notifications = RwSignal::new(true);
  let settings_theme = RwSignal::new("light".to_string());

  view! {
    <div class="space-y-8">
      <SectionHeader
        title="Combined Examples"
        description="Real-world examples combining multiple components"
      />

      // Login Form Example
      <ComponentCard title="Login Form" description="Complete login form using multiple components">
        <Card class="w-full max-w-md mx-auto">
          <CardHeader>
            <CardTitle>"Login"</CardTitle>
            <CardDescription>"Enter your credentials to continue"</CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="space-y-2">
              <Label r#for="login-email">"Email"</Label>
              <Input
                attr:id="login-email"
                r#type="email".to_string()
                attr:placeholder="name@example.com"
                value=login_email
              />
            </div>
            <div class="space-y-2">
              <Label r#for="login-password">"Password"</Label>
              <Input
                attr:id="login-password"
                r#type="password".to_string()
                attr:placeholder="Enter your password"
                value=login_password
              />
            </div>
            <div class="flex items-center space-x-2">
              <Checkbox attr:id="remember" />
              <Label r#for="remember">"Remember me"</Label>
            </div>
          </CardContent>
          <CardFooter class="flex flex-col space-y-2">
            <Button class="w-full">"Sign In"</Button>
            <Button variant=ButtonVariant::Link class="w-full">
              "Forgot password?"
            </Button>
          </CardFooter>
        </Card>
      </ComponentCard>

      // Settings Panel Example
      <ComponentCard title="Settings Panel" description="Settings interface with various controls">
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
                <Switch checked=settings_notifications />
              </div>
            </div>

            <Separator />

            <div class="space-y-4">
              <h3 class="text-lg font-semibold">"Appearance"</h3>
              <div class="space-y-2">
                <Label>"Theme"</Label>
                <Select value=settings_theme>
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
      </ComponentCard>

      // Dashboard Example
      <ComponentCard title="Dashboard Cards" description="Dashboard layout with stats">
        <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
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

          <Card>
            <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle class="text-sm font-medium">"Active Users"</CardTitle>
              <span>"👥"</span>
            </CardHeader>
            <CardContent>
              <div class="text-2xl font-bold">"+2,350"</div>
              <p class="text-xs text-muted-foreground">"+180.1% from last month"</p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle class="text-sm font-medium">"Sales"</CardTitle>
              <span>"💳"</span>
            </CardHeader>
            <CardContent>
              <div class="text-2xl font-bold">"+12,234"</div>
              <p class="text-xs text-muted-foreground">"+19% from last month"</p>
            </CardContent>
          </Card>
        </div>
      </ComponentCard>
    </div>
  }
}

// Helper Components

/// Sample document used by the live Markdown editor demo.
const MARKDOWN_SAMPLE: &str = r#"# Markdown block

A **block** is a small widget composed of UI Kit components. This one renders
*Markdown* — either inline content like this, or an `.md` file fetched from a URL.

## Features

- GFM tables, ~~strikethrough~~ and task lists
- Per-element Tailwind classes with sensible defaults
- Inline `code` and fenced blocks

```rust
fn main() {
    println!("Hello from a code block!");
}
```

> Blockquotes follow the UI Kit theme tokens out of the box.

### Tasks

- [x] Parse Markdown
- [x] Inject Tailwind classes
- [ ] Conquer the world

### A table

| Element | Default class hint   |
| ------- | -------------------- |
| Heading | `font-semibold`      |
| Link    | `text-primary`       |

Read more at the [Impulse Kit repo](https://github.com/impulse-sw/impulse-kit).

---

That's it!
"#;

#[component]
fn BlocksSection() -> impl IntoView {
  let markdown = RwSignal::new(MARKDOWN_SAMPLE.to_string());

  // A few per-element overrides, leaving everything else at its default.
  let custom_classes = MarkdownClasses {
    h1: "mt-6 mb-4 text-3xl font-black tracking-tight text-primary".into(),
    h2: "mt-6 mb-3 text-2xl font-bold tracking-tight text-primary/90".into(),
    link: "font-medium text-chart-2 underline decoration-dotted underline-offset-4 hover:text-chart-2/80".into(),
    inline_code: "rounded bg-primary/10 px-1.5 py-0.5 font-mono text-[0.85em] text-primary".into(),
    blockquote: "my-4 rounded-md border-l-4 border-primary bg-primary/5 py-2 pl-4 italic text-foreground/80".into(),
    ..Default::default()
  };

  // Single-series column chart with value labels.
  let revenue = BarChartData {
    categories: vec![
      "Jan".into(),
      "Feb".into(),
      "Mar".into(),
      "Apr".into(),
      "May".into(),
      "Jun".into(),
    ],
    series: vec![BarSeries::new("Revenue, k$", vec![18.0, 24.0, 21.0, 33.0, 29.0, 41.0])],
  };

  // Grouped, multi-series column chart.
  let quarters = BarChartData {
    categories: vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()],
    series: vec![
      BarSeries::new("2024", vec![12.0, 19.0, 7.0, 15.0]),
      BarSeries::new("2025", vec![16.0, 11.0, 21.0, 9.0]),
    ],
  };

  view! {
    <div class="space-y-8">
      <SectionHeader
        title="Blocks"
        description="Higher-level widgets composed of UI Kit components"
      />

      // Single-series column chart.
      <ComponentCard
        title="Bar chart"
        description="SVG column chart with axes, grid, value labels and a hover tooltip"
      >
        <BarChart
          data=revenue
          options=BarChartOptions {
            show_values: true,
            ..Default::default()
          }
        />
      </ComponentCard>

      // Grouped multi-series column chart.
      <ComponentCard
        title="Grouped bar chart"
        description="Multiple series are drawn as grouped columns, colored from the theme --chart-* palette"
      >
        <BarChart data=quarters />
      </ComponentCard>

      // Live Markdown editor.
      <ComponentCard
        title="Markdown"
        description="Render inline Markdown (or an .md file from a URL) into themed HTML"
      >
        <div class="grid gap-4 md:grid-cols-2">
          <div class="space-y-2">
            <Label>"Markdown source"</Label>
            <Textarea
              value=markdown
              class="min-h-[28rem] w-full font-mono text-sm"
            />
          </div>
          <div class="space-y-2">
            <Label>"Rendered output"</Label>
            <div class="min-h-[28rem] overflow-auto rounded-md border bg-card p-4">
              {move || view! { <Markdown source=MarkdownSource::inline(markdown.get()) /> }}
            </div>
          </div>
        </div>
      </ComponentCard>

      // Per-element style overrides.
      <ComponentCard
        title="Custom element styles"
        description="Override the Tailwind classes of individual Markdown elements via MarkdownClasses"
      >
        <div class="rounded-md border bg-card p-4">
          <Markdown
            source=MarkdownSource::inline(MARKDOWN_SAMPLE)
            classes=custom_classes
          />
        </div>
      </ComponentCard>

      // Loading from a URL.
      <ComponentCard
        title="From a URL"
        description="Pass MarkdownSource::url(..) to fetch and render an .md file at runtime"
      >
        <div class="rounded-md border bg-card p-4">
          <Markdown source=MarkdownSource::url("/sample.md") />
        </div>
      </ComponentCard>
    </div>
  }
}

#[component]
fn SectionHeader(title: &'static str, description: &'static str) -> impl IntoView {
  view! {
    <div class="space-y-2">
      <h2 class="text-3xl font-bold tracking-tight">{title}</h2>
      <p class="text-muted-foreground">{description}</p>
    </div>
  }
}

#[component]
fn ComponentCard(title: &'static str, description: &'static str, children: Children) -> impl IntoView {
  view! {
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">{children()}</CardContent>
    </Card>
  }
}
