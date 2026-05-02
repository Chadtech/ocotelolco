use gpui::{
    prelude::*, App, Application, Context, FocusHandle, Focusable, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Render, Window, WindowOptions,
};

use crate::ui::{style as s, view};

pub fn run() {
    Application::new().run(|cx: &mut App| {
        let window_handle = cx
            .open_window(WindowOptions::default(), |window, cx| {
                window.set_window_title("Ocotelolco Notes");
                let focus_handle = cx.focus_handle();

                cx.new(|_| Model {
                    focus_handle,
                    notes: Vec::new(),
                    active_note_index: None,
                    pointer_interaction: None,
                    new_button_pressed: false,
                    pressed_close_note_index: None,
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

struct Note {
    content: String,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

struct Model {
    focus_handle: FocusHandle,
    notes: Vec<Note>,
    active_note_index: Option<usize>,
    pointer_interaction: Option<PointerInteraction>,
    new_button_pressed: bool,
    pressed_close_note_index: Option<usize>,
}

enum PointerInteraction {
    Drag(PointerInteractionState),
    Resize(PointerInteractionState),
}

struct PointerInteractionState {
    note_index: usize,
    last_x: f32,
    last_y: f32,
}

const DEFAULT_NOTE_SIZE: f32 = 256.0;
const MIN_NOTE_SIZE: f32 = 128.0;

impl Model {
    fn press_new_note_button(
        &mut self,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.new_button_pressed = true;
        cx.notify();
    }

    fn new_note(&mut self, _: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.new_button_pressed = false;
        let offset = self.notes.len() as f32 * 24.0;
        self.notes.push(Note {
            content: String::new(),
            x: 32.0 + offset,
            y: 32.0 + offset,
            width: DEFAULT_NOTE_SIZE,
            height: DEFAULT_NOTE_SIZE,
        });
        self.active_note_index = Some(self.notes.len() - 1);
        window.focus(&self.focus_handle);
        cx.notify();
    }

    fn cancel_new_note_button(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.new_button_pressed = false;
        cx.notify();
    }

    fn press_close_note_button(
        &mut self,
        note_index: usize,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pressed_close_note_index = Some(note_index);
        cx.stop_propagation();
        cx.notify();
    }

    fn close_note(
        &mut self,
        note_index: usize,
        _: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if note_index >= self.notes.len() {
            return;
        }

        self.pressed_close_note_index = None;
        self.notes.remove(note_index);
        self.pointer_interaction = None;
        self.active_note_index = match self.active_note_index {
            Some(active_index) if active_index == note_index => None,
            Some(active_index) if active_index > note_index => Some(active_index - 1),
            active_note_index => active_note_index,
        };
        cx.stop_propagation();
        cx.notify();
    }

    fn cancel_close_note_button(
        &mut self,
        _: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pressed_close_note_index = None;
        cx.notify();
    }

    fn active_note_mut(&mut self) -> Option<&mut Note> {
        self.active_note_index
            .and_then(|index| self.notes.get_mut(index))
    }

    fn pointer_note_mut(&mut self, note_index: usize, cx: &mut Context<Self>) -> Option<&mut Note> {
        if note_index >= self.notes.len() {
            self.pointer_interaction = None;
            cx.notify();
            return None;
        }

        self.notes.get_mut(note_index)
    }

    fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
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

                let Some(note) = self.pointer_note_mut(state.note_index, cx) else {
                    return;
                };
                note.x += dx;
                note.y += dy;
            }
            PointerInteraction::Resize(state) => {
                let dx = x - state.last_x;
                let dy = y - state.last_y;
                state.last_x = x;
                state.last_y = y;

                let Some(note) = self.pointer_note_mut(state.note_index, cx) else {
                    return;
                };
                note.width = (note.width + dx).max(MIN_NOTE_SIZE);
                note.height = (note.height + dy).max(MIN_NOTE_SIZE);
            }
        }
        self.pointer_interaction = Some(pointer_interaction);
        cx.notify();
    }

    fn begin_drag(
        &mut self,
        note_index: usize,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_note_index = Some(note_index);
        self.pointer_interaction = Some(PointerInteraction::Drag(PointerInteractionState {
            note_index,
            last_x: event.position.x.into(),
            last_y: event.position.y.into(),
        }));
        window.focus(&self.focus_handle);
        cx.stop_propagation();
        cx.notify();
    }

    fn begin_resize(
        &mut self,
        note_index: usize,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_note_index = Some(note_index);
        self.pointer_interaction = Some(PointerInteraction::Resize(PointerInteractionState {
            note_index,
            last_x: event.position.x.into(),
            last_y: event.position.y.into(),
        }));
        window.focus(&self.focus_handle);
        cx.stop_propagation();
        cx.notify();
    }

    fn end_pointer_interaction(
        &mut self,
        _: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pointer_interaction = None;
        cx.notify();
    }

    fn focus_editor(
        &mut self,
        note_index: usize,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_note_index = Some(note_index);
        window.focus(&self.focus_handle);
        cx.notify();
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.platform || event.keystroke.modifiers.control {
            return;
        }

        let Some(note) = self.active_note_mut() else {
            return;
        };

        match event.keystroke.key.as_str() {
            "backspace" => {
                note.content.pop();
                cx.stop_propagation();
                cx.notify();
            }
            "enter" => {
                note.content.push('\n');
                cx.stop_propagation();
                cx.notify();
            }
            _ => {
                if let Some(key_char) = event.keystroke.key_char.as_ref() {
                    note.content.push_str(key_char);
                    cx.stop_propagation();
                    cx.notify();
                }
            }
        }
    }
}

impl Focusable for Model {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Model {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_focused = self.focus_handle.is_focused(window);
        let note_windows = self
            .notes
            .iter()
            .enumerate()
            .map(|(index, note)| {
                render_note_window(
                    index,
                    note,
                    &self.focus_handle,
                    self.pressed_close_note_index == Some(index),
                    self.active_note_index == Some(index) && is_focused,
                    cx,
                )
            })
            .collect::<Vec<_>>();

        gpui::div()
            .flex()
            .flex_col()
            .size_full()
            .font_family(s::FONT)
            .bg(s::GREEN2)
            .text_color(s::GRAY6)
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(Self::end_pointer_interaction),
            )
            .child(
                gpui::div()
                    .relative()
                    .flex_1()
                    .flex()
                    .size_full()
                    .children(note_windows),
            )
            .child(toolbar(self.new_button_pressed, cx))
    }
}

fn render_note_window(
    note_index: usize,
    note: &Note,
    focus_handle: &FocusHandle,
    close_button_pressed: bool,
    show_cursor: bool,
    cx: &mut Context<Model>,
) -> gpui::Div {
    let mut lines = note.content.split('\n').collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push("");
    }

    s::raised(
        gpui::div()
            .flex()
            .flex_col()
            .size_full()
            .bg(s::GRAY2)
            .p(s::S2)
            .child(
                gpui::div()
                    // .h(s::S5)
                    .px(s::S3)
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(s::GRAY5)
                    .text_color(s::GREEN1)
                    .p(s::S2)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |model, event, window, cx| {
                            model.begin_drag(note_index, event, window, cx);
                        }),
                    )
                    .child(format!("note {}", note_index + 1))
                    .child(close_button(note_index, close_button_pressed, cx)),
            )
            .child(
                gpui::div().p(s::S2).size_full().bg(s::GRAY2).child(
                    s::sunken(
                        gpui::div()
                            .flex()
                            .flex_col()
                            .size_full()
                            .p(s::S2)
                            .bg(s::GREEN1)
                            .track_focus(focus_handle)
                            .key_context("NoteEditor")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |model, event, window, cx| {
                                    model.focus_editor(note_index, event, window, cx);
                                }),
                            )
                            .on_key_down(cx.listener(Model::handle_key_down))
                            .children(render_note_lines(lines, show_cursor)),
                    )
                    .size_full(),
                ),
            ),
    )
    .child(resize_handle(note_index, cx))
    .absolute()
    .left(gpui::px(note.x))
    .top(gpui::px(note.y))
    .w(gpui::px(note.width))
    .h(gpui::px(note.height))
}

fn toolbar(new_button_pressed: bool, cx: &mut Context<Model>) -> impl IntoElement {
    gpui::div()
        .flex()
        .items_center()
        .border_b_2()
        .border_color(s::GRAY3)
        .bg(s::GRAY3)
        .p(s::S2)
        .gap_3()
        .child(
            view::button::from_text("new", new_button_pressed)
                .on_mouse_down(MouseButton::Left, cx.listener(Model::press_new_note_button))
                .on_mouse_up(MouseButton::Left, cx.listener(Model::new_note))
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(Model::cancel_new_note_button),
                ),
        )
}

fn close_button(note_index: usize, pressed: bool, cx: &mut Context<Model>) -> gpui::Div {
    view::button::x(pressed)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |model, event, window, cx| {
                model.press_close_note_button(note_index, event, window, cx);
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |model, event, window, cx| {
                model.close_note(note_index, event, window, cx);
            }),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(Model::cancel_close_note_button),
        )
}

fn resize_handle(note_index: usize, cx: &mut Context<Model>) -> impl IntoElement {
    gpui::div()
        .absolute()
        .right_0()
        .bottom_0()
        .size(s::S4)
        .child(
            gpui::canvas(
                |_, _, _| {},
                |bounds, _, window, _| {
                    let mut builder = gpui::PathBuilder::stroke(s::S1);
                    builder.move_to(gpui::point(bounds.right() - s::S3, bounds.bottom() - s::S1));
                    builder.line_to(gpui::point(bounds.right() - s::S1, bounds.bottom() - s::S3));
                    builder.move_to(gpui::point(bounds.right() - s::S2, bounds.bottom() - s::S1));
                    builder.line_to(gpui::point(bounds.right() - s::S1, bounds.bottom() - s::S2));
                    if let Ok(path) = builder.build() {
                        window.paint_path(path, s::GRAY1);
                    }
                },
            )
            .size_full(),
        )
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |model, event, window, cx| {
                model.begin_resize(note_index, event, window, cx);
            }),
        )
}

fn render_note_lines(lines: Vec<&str>, is_focused: bool) -> Vec<impl IntoElement> {
    let last_index = lines.len().saturating_sub(1);

    lines
        .into_iter()
        .enumerate()
        .map(move |(index, line)| {
            let text = if is_focused && index == last_index {
                format!("{line}|")
            } else if line.is_empty() {
                " ".to_string()
            } else {
                line.to_string()
            };

            gpui::div().min_h(s::S5).text_color(s::GRAY6).child(text)
        })
        .collect()
}
