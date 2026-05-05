use gpui::{
    prelude::*, App, Application, Context, EventEmitter, FocusHandle, Focusable, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Render, WindowOptions,
};
use std::{collections::HashMap, path::PathBuf};

pub mod field;
mod note;
pub mod style;
pub mod view;

use crate::ui::field::FieldId;
use crate::ui::style as s;
use note::NoteId;

const DEFAULT_WINDOW_SIZE: f32 = 256.0;
const MIN_WINDOW_SIZE: f32 = 128.0;

pub fn run() {
    Application::new().run(|cx: &mut App| {
        let window_handle = cx
            .open_window(WindowOptions::default(), |window, cx| {
                window.set_window_title("Ocotelolco Notes");
                let focus_handle = cx.focus_handle();

                cx.new(|cx| {
                    cx.subscribe_self(Model::handled_note_event).detach();

                    Model {
                        focus_handle,
                        windows: HashMap::new(),
                        window_order: Vec::new(),
                        active_field: None,
                        next_note_id: NoteId(1),
                        pointer_interaction: None,
                        pressed_button: None,
                    }
                })
            })
            .expect("failed to open notes window");

        window_handle
            .update(cx, |notes, window, cx| {
                window.focus(&notes.focus_handle);
                cx.activate(true);
            })
            .expect("failed to focus notes window");
    });
}

pub(super) struct Model {
    focus_handle: FocusHandle,
    windows: HashMap<WindowId, Window>,
    window_order: Vec<WindowId>,
    active_field: Option<FieldId>,
    next_note_id: NoteId,
    pointer_interaction: Option<PointerInteraction>,
    pressed_button: Option<ButtonId>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum WindowId {
    Note(NoteId),
}

struct Window {
    x: f32,
    y: f32,
    height: f32,
    width: f32,
    content: WindowContent,
}

enum WindowContent {
    Note(note::Model),
}

impl Window {
    fn new_note(window_id: WindowId, ordinal: usize, offset: f32) -> Self {
        Self {
            x: 32.0 + offset,
            y: 32.0 + offset,
            height: DEFAULT_WINDOW_SIZE,
            width: DEFAULT_WINDOW_SIZE,
            content: WindowContent::Note(note::Model::new(window_id.note_id(), ordinal)),
        }
    }

    fn note(&self) -> &note::Model {
        match &self.content {
            WindowContent::Note(note) => note,
        }
    }

    fn note_mut(&mut self) -> &mut note::Model {
        match &mut self.content {
            WindowContent::Note(note) => note,
        }
    }
}

impl WindowId {
    fn note_id(self) -> NoteId {
        match self {
            Self::Note(note_id) => note_id,
        }
    }
}

impl From<NoteId> for WindowId {
    fn from(note_id: NoteId) -> Self {
        Self::Note(note_id)
    }
}

#[derive(Clone, PartialEq, Eq)]
enum ButtonId {
    NewNote,
    NoteButtonId {
        note_id: NoteId,
        button_id: note::ButtonId,
    },
}

enum Event {
    Note(note::IdEvent),
    SavedNote {
        note_id: NoteId,
        generation: u64,
        result: Result<PathBuf, String>,
    },
}

enum Effect {
    SaveNote(note::SaveRequest),
}

enum PointerInteraction {
    Drag(PointerInteractionState),
    Resize(PointerInteractionState),
}

struct PointerInteractionState {
    window_id: WindowId,
    last_x: f32,
    last_y: f32,
}

#[derive(Clone, Copy)]
struct ActiveField {
    note_id: NoteId,
    kind: FieldKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldKind {
    Body,
    Name,
}

impl Model {
    fn handled_event(&mut self, event: Event, cx: &mut Context<Self>) {
        match event {
            Event::Note(note_event) => {
                self.update_note(note_event, cx);
            }
            Event::SavedNote {
                note_id,
                generation,
                result,
            } => {
                let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(note_id))
                    .map(Window::note_mut)
                else {
                    cx.notify();
                    return;
                };

                let save_result = match result {
                    Ok(_) => Ok(()),
                    Err(error) => {
                        eprintln!("failed to save note {}: {error}", note_id.0);
                        Err(error)
                    }
                };
                note.finished_saving(generation, save_result);
                cx.notify();
            }
        }
    }

    fn dispatch_effect(&mut self, effect: Effect, cx: &mut Context<Self>) {
        match effect {
            Effect::SaveNote(save_note) => {
                cx.spawn(async move |model, cx| {
                    let note_id = save_note.note_id;
                    let generation = save_note.generation;
                    let result = note::save_note_file(save_note).map_err(|error| error.to_string());
                    let _ = model.update(cx, |model, cx| {
                        model.handled_event(
                            Event::SavedNote {
                                note_id,
                                generation,
                                result,
                            },
                            cx,
                        );
                    });
                })
                .detach();
            }
        }
    }

    fn pressed_new_note_button(
        &mut self,
        _: &MouseDownEvent,
        _: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        self.pressed_button = Some(ButtonId::NewNote);
        cx.notify();
    }

    fn clicked_new_note_button(
        &mut self,
        _: &MouseUpEvent,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        self.pressed_button = None;
        let offset = self.window_order.len() as f32 * 24.0;
        let note_id = self.next_note_id;
        let window_id = WindowId::Note(note_id);
        self.next_note_id = NoteId(self.next_note_id.0 + 1);
        let body_field_id = FieldId(format!("note-{}/body", note_id.0));
        self.windows.insert(
            window_id,
            Window::new_note(window_id, self.window_order.len() + 1, offset),
        );
        self.window_order.push(window_id);
        self.active_field = Some(body_field_id);
        window.focus(&self.focus_handle);
        cx.notify();
    }

    fn released_new_note_button_outside(
        &mut self,
        _: &MouseUpEvent,
        _: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        self.pressed_button = None;
        cx.notify();
    }

    fn active_field(&self) -> Option<ActiveField> {
        let active_field_id = self.active_field.as_ref()?;
        self.windows.iter().find_map(|(window_id, ui_window)| {
            let note = ui_window.note();
            if &note.body_field_id() == active_field_id {
                Some(ActiveField {
                    note_id: window_id.note_id(),
                    kind: FieldKind::Body,
                })
            } else if &note.name_field_id() == active_field_id {
                Some(ActiveField {
                    note_id: window_id.note_id(),
                    kind: FieldKind::Name,
                })
            } else {
                None
            }
        })
    }

    fn pointer_window_mut(
        &mut self,
        window_id: WindowId,
        cx: &mut Context<Self>,
    ) -> Option<&mut Window> {
        if !self.windows.contains_key(&window_id) {
            self.pointer_interaction = None;
            cx.notify();
            return None;
        }

        self.windows.get_mut(&window_id)
    }

    fn bring_window_to_front(&mut self, window_id: WindowId) -> Option<WindowId> {
        if !self.windows.contains_key(&window_id) {
            return None;
        }

        if let Some(order_index) = self.window_order.iter().position(|id| *id == window_id) {
            self.window_order.remove(order_index);
        }
        self.window_order.push(window_id);
        Some(window_id)
    }

    fn activate_note_body(&mut self, note_id: NoteId) -> Option<NoteId> {
        let front_window_id = self.bring_window_to_front(WindowId::from(note_id))?;
        self.active_field = Some(self.windows.get(&front_window_id)?.note().body_field_id());
        Some(front_window_id.note_id())
    }

    fn activate_note_name(&mut self, note_id: NoteId) -> Option<NoteId> {
        let front_window_id = self.bring_window_to_front(WindowId::from(note_id))?;
        self.active_field = Some(self.windows.get(&front_window_id)?.note().name_field_id());
        Some(front_window_id.note_id())
    }

    fn moved_mouse(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        let Some(mut pointer_interaction) = self.pointer_interaction.take() else {
            return;
        };
        if !event.dragging() {
            cx.notify();
            return;
        }

        let x = f32::from(event.position.x);
        let y = f32::from(event.position.y);
        match &mut pointer_interaction {
            PointerInteraction::Drag(state) => {
                let dx = x - state.last_x;
                let dy = y - state.last_y;
                state.last_x = x;
                state.last_y = y;

                let Some(ui_window) = self.pointer_window_mut(state.window_id, cx) else {
                    return;
                };
                ui_window.x += dx;
                ui_window.y += dy;
            }
            PointerInteraction::Resize(state) => {
                let dx = x - state.last_x;
                let dy = y - state.last_y;
                state.last_x = x;
                state.last_y = y;

                let Some(ui_window) = self.pointer_window_mut(state.window_id, cx) else {
                    return;
                };
                ui_window.width = (ui_window.width + dx).max(MIN_WINDOW_SIZE);
                ui_window.height = (ui_window.height + dy).max(MIN_WINDOW_SIZE);
            }
        }
        self.pointer_interaction = Some(pointer_interaction);
        cx.notify();
    }

    fn released_mouse(&mut self, _: &MouseUpEvent, _: &mut gpui::Window, cx: &mut Context<Self>) {
        self.pointer_interaction = None;
        cx.notify();
    }

    fn pressed_name_key(&mut self, key_press: &note::KeyPress, cx: &mut Context<Self>) {
        let Some(active_field) = self.active_field() else {
            self.active_field = None;
            return;
        };

        match key_press {
            note::KeyPress::Backspace => {
                if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(active_field.note_id))
                    .map(Window::note_mut)
                {
                    note.pressed_name_backspace();
                }
                cx.notify();
            }
            note::KeyPress::OptionBackspace => {
                if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(active_field.note_id))
                    .map(Window::note_mut)
                {
                    note.pressed_name_option_backspace();
                }
                cx.notify();
            }
            note::KeyPress::CommandBackspace => {
                if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(active_field.note_id))
                    .map(Window::note_mut)
                {
                    note.pressed_name_command_backspace();
                }
                cx.notify();
            }
            note::KeyPress::Enter => {
                let body_field_id = if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(active_field.note_id))
                    .map(Window::note_mut)
                {
                    note.clicked_save_name();
                    note.body_field_id()
                } else {
                    self.active_field = None;
                    cx.notify();
                    return;
                };
                self.active_field = Some(body_field_id);
                cx.notify();
            }
            note::KeyPress::Text(key_char) => {
                if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(active_field.note_id))
                    .map(Window::note_mut)
                {
                    note.pressed_name_key(key_char);
                }
                cx.notify();
            }
        }
    }

    fn handled_note_event(&mut self, note_event: &note::IdEvent, cx: &mut Context<Self>) {
        self.handled_event(Event::Note(note_event.clone()), cx);
    }

    fn update_note(&mut self, note_event: note::IdEvent, cx: &mut Context<Self>) {
        let note_id = note_event.note_id;

        match &note_event.event {
            note::Event::PressedHeader { x, y } => {
                let Some(front_note_id) = self.activate_note_body(note_id) else {
                    return;
                };
                self.pointer_interaction =
                    Some(PointerInteraction::Drag(PointerInteractionState {
                        window_id: WindowId::from(front_note_id),
                        last_x: *x,
                        last_y: *y,
                    }));
                cx.notify();
            }
            note::Event::PressedResizeHandle { x, y } => {
                let Some(front_note_id) = self.activate_note_body(note_id) else {
                    return;
                };
                self.pointer_interaction =
                    Some(PointerInteraction::Resize(PointerInteractionState {
                        window_id: WindowId::from(front_note_id),
                        last_x: *x,
                        last_y: *y,
                    }));
                cx.notify();
            }
            note::Event::PressedBodyEditor => {
                self.activate_note_body(note_id);
                cx.notify();
            }
            note::Event::PressedNameEditor => {
                self.activate_note_name(note_id);
                cx.notify();
            }
            note::Event::ClickedRename => {
                let Some(front_note_id) = self.activate_note_name(note_id) else {
                    return;
                };

                if let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(front_note_id))
                    .map(Window::note_mut)
                {
                    note.clicked_rename();
                    self.active_field = Some(note.name_field_id());
                }
                cx.notify();
            }
            note::Event::ClickedSaveName => {
                let Some(front_window_id) = self.bring_window_to_front(WindowId::from(note_id))
                else {
                    return;
                };
                if let Some(note) = self.windows.get_mut(&front_window_id).map(Window::note_mut) {
                    note.clicked_save_name();
                    if self.active_field == Some(note.name_field_id()) {
                        self.active_field = Some(note.body_field_id());
                    }
                }
                cx.notify();
            }
            note::Event::ClickedSaveButton => {
                self.pressed_button = None;
                let Some(front_window_id) = self.bring_window_to_front(WindowId::from(note_id))
                else {
                    return;
                };
                let Some(note) = self.windows.get_mut(&front_window_id).map(Window::note_mut)
                else {
                    return;
                };
                let save_note = note.save_note();
                self.dispatch_effect(Effect::SaveNote(save_note), cx);
                cx.notify();
            }
            note::Event::ClickedCloseButton => {
                let Some(front_window_id) = self.bring_window_to_front(WindowId::from(note_id))
                else {
                    return;
                };

                if let Some(closed_note) = self.windows.get(&front_window_id).map(Window::note) {
                    self.pressed_button = None;
                    if self.active_field == Some(closed_note.name_field_id())
                        || self.active_field == Some(closed_note.body_field_id())
                    {
                        self.active_field = None;
                    }
                    self.windows.remove(&front_window_id);
                    self.window_order
                        .retain(|ordered_window_id| *ordered_window_id != front_window_id);
                    self.pointer_interaction = None;
                    cx.notify();
                }
            }
            note::Event::PressedButton { button_id } => {
                let Some(front_window_id) = self.bring_window_to_front(WindowId::from(note_id))
                else {
                    return;
                };
                self.pressed_button = Some(ButtonId::NoteButtonId {
                    note_id: front_window_id.note_id(),
                    button_id: button_id.clone(),
                });
                cx.notify();
            }
            note::Event::ReleasedButton => {
                self.pressed_button = None;
                cx.notify();
            }
            note::Event::PressedKey(key_press) => {
                let Some(active_field) = self.active_field() else {
                    self.active_field = None;
                    return;
                };

                if active_field.kind == FieldKind::Name {
                    self.pressed_name_key(key_press, cx);
                    return;
                }

                let Some(note) = self
                    .windows
                    .get_mut(&WindowId::from(active_field.note_id))
                    .map(Window::note_mut)
                else {
                    self.active_field = None;
                    return;
                };

                match key_press {
                    note::KeyPress::Backspace => {
                        note.pressed_body_backspace();
                        cx.notify();
                    }
                    note::KeyPress::OptionBackspace => {
                        note.pressed_body_option_backspace();
                        cx.notify();
                    }
                    note::KeyPress::CommandBackspace => {
                        note.pressed_body_command_backspace();
                        cx.notify();
                    }
                    note::KeyPress::Enter => {
                        note.content.push('\n');
                        note.started_editing();
                        cx.notify();
                    }
                    note::KeyPress::Text(key_char) => {
                        note.content.push_str(key_char);
                        note.started_editing();
                        cx.notify();
                    }
                }
            }
        }
    }
}

impl EventEmitter<note::IdEvent> for Model {}

impl Focusable for Model {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Model {
    fn render(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_focused = self.focus_handle.is_focused(window);
        let ui_windows = self
            .window_order
            .iter()
            .filter_map(|window_id| {
                let Some(ui_window) = self.windows.get(window_id) else {
                    eprintln!("window order referenced a missing window");
                    return None;
                };
                let rendered_content = match &ui_window.content {
                    WindowContent::Note(note) => {
                        let pressed_note_button = match self.pressed_button.as_ref() {
                            Some(ButtonId::NoteButtonId {
                                note_id: pressed_note_id,
                                button_id,
                            }) if pressed_note_id == &note.id => Some(button_id),
                            _ => None,
                        };

                        note::render(
                            note,
                            &self.focus_handle,
                            pressed_note_button,
                            self.active_field.as_ref(),
                            is_focused,
                            cx,
                        )
                    }
                };

                Some(
                    rendered_content
                        .absolute()
                        .left(gpui::px(ui_window.x))
                        .top(gpui::px(ui_window.y))
                        .w(gpui::px(ui_window.width))
                        .h(gpui::px(ui_window.height)),
                )
            })
            .collect::<Vec<_>>();

        gpui::div()
            .flex()
            .flex_col()
            .size_full()
            .font_family(s::FONT)
            .bg(s::GREEN3)
            .text_color(s::GRAY6)
            .on_mouse_move(cx.listener(Self::moved_mouse))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::released_mouse))
            .child(
                gpui::div()
                    .relative()
                    .flex_1()
                    .flex()
                    .size_full()
                    .child(
                        gpui::img(std::path::Path::new(concat!(
                            env!("CARGO_MANIFEST_DIR"),
                            "/ocotelolco_bg.png"
                        )))
                        .absolute()
                        .size_full()
                        .object_fit(gpui::ObjectFit::Cover),
                    )
                    .children(ui_windows),
            )
            .child(toolbar(self.pressed_button == Some(ButtonId::NewNote), cx))
    }
}

fn toolbar(new_button_pressed: bool, cx: &mut Context<Model>) -> impl IntoElement {
    gpui::div()
        .flex()
        .items_center()
        .border_t_2()
        .border_color(s::GRAY3)
        .bg(s::GRAY2)
        .p(s::S3)
        .gap_3()
        .child(
            view::button::from_text("new note", new_button_pressed)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(Model::pressed_new_note_button),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(Model::clicked_new_note_button),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(Model::released_new_note_button_outside),
                ),
        )
}
