use gpui::{
    prelude::*, App, Application, Context, FocusHandle, Focusable, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Render, Window, WindowOptions,
};

use crate::ui::style as s;

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
                    drag: None,
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
}

struct Model {
    focus_handle: FocusHandle,
    notes: Vec<Note>,
    active_note_index: Option<usize>,
    drag: Option<DragState>,
}

struct DragState {
    note_index: usize,
    last_x: f32,
    last_y: f32,
}

impl Model {
    fn new_note(&mut self, _: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        let offset = self.notes.len() as f32 * 24.0;
        self.notes.push(Note {
            content: String::new(),
            x: 32.0 + offset,
            y: 32.0 + offset,
        });
        self.active_note_index = Some(self.notes.len() - 1);
        window.focus(&self.focus_handle);
        cx.notify();
    }

    fn active_note_mut(&mut self) -> Option<&mut Note> {
        self.active_note_index
            .and_then(|index| self.notes.get_mut(index))
    }

    fn begin_drag(
        &mut self,
        note_index: usize,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_note_index = Some(note_index);
        self.drag = Some(DragState {
            note_index,
            last_x: event.position.x.into(),
            last_y: event.position.y.into(),
        });
        window.focus(&self.focus_handle);
        cx.stop_propagation();
        cx.notify();
    }

    fn drag_note(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.as_mut() else {
            return;
        };
        if !event.dragging() {
            self.drag = None;
            cx.notify();
            return;
        }

        let x = f32::from(event.position.x);
        let y = f32::from(event.position.y);
        let dx = x - drag.last_x;
        let dy = y - drag.last_y;
        drag.last_x = x;
        drag.last_y = y;

        if let Some(note) = self.notes.get_mut(drag.note_index) {
            note.x += dx;
            note.y += dy;
            cx.notify();
        }
    }

    fn end_drag(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.drag = None;
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
            .on_mouse_move(cx.listener(Self::drag_note))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::end_drag))
            .child(
                gpui::div()
                    .relative()
                    .flex_1()
                    .flex()
                    .size_full()
                    .children(note_windows),
            )
            .child(toolbar(cx))
    }
}

fn render_note_window(
    note_index: usize,
    note: &Note,
    focus_handle: &FocusHandle,
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
            .child(
                gpui::div()
                    .h(s::S5)
                    .px(s::S3)
                    .flex()
                    .items_center()
                    .bg(s::GRAY5)
                    .text_color(s::GREEN1)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |model, event, window, cx| {
                            model.begin_drag(note_index, event, window, cx);
                        }),
                    )
                    .child(format!("note {}", note_index + 1)),
            )
            .child(
                gpui::div().p(s::S4).size_full().bg(s::GRAY2).child(
                    s::sunken(
                        gpui::div()
                            .flex()
                            .flex_col()
                            .size_full()
                            .p(s::S4)
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
    .absolute()
    .left(gpui::px(note.x))
    .top(gpui::px(note.y))
    .p(s::S2)
    .w(s::S8)
    .h(s::S8)
}

fn toolbar(cx: &mut Context<Model>) -> impl IntoElement {
    gpui::div()
        .flex()
        .items_center()
        .border_b_2()
        .border_color(s::GRAY3)
        .bg(s::GRAY3)
        .p(s::S2)
        .gap_3()
        .child(toolbar_button("new").on_mouse_up(MouseButton::Left, cx.listener(Model::new_note)))
}

fn toolbar_button(label: &'static str) -> gpui::Div {
    s::raised(label)
        .p(s::S2)
        .flex()
        .items_center()
        .px(s::S3)
        .bg(s::GRAY3)
        .text_color(s::GRAY6)
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
