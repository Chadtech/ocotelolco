use gpui::{
    prelude::*, App, Application, Context, FocusHandle, Focusable, KeyDownEvent, MouseButton,
    MouseDownEvent, Render, Window, WindowOptions,
};

use crate::ui::style as s;

pub fn run() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |window, cx| {
            window.set_window_title("Ocotelolco Notes");
            cx.new(|cx| NotesApp {
                focus_handle: cx.focus_handle(),
                note: String::new(),
            })
        })
        .expect("failed to open notes window");
    });
}

struct NotesApp {
    focus_handle: FocusHandle,
    note: String,
}

impl NotesApp {
    fn focus_editor(&mut self, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle);
        cx.notify();
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.platform || event.keystroke.modifiers.control {
            return;
        }

        match event.keystroke.key.as_str() {
            "backspace" => {
                self.note.pop();
                cx.stop_propagation();
                cx.notify();
            }
            "enter" => {
                self.note.push('\n');
                cx.stop_propagation();
                cx.notify();
            }
            _ => {
                if let Some(key_char) = event.keystroke.key_char.as_ref() {
                    self.note.push_str(key_char);
                    cx.stop_propagation();
                    cx.notify();
                }
            }
        }
    }
}

impl Focusable for NotesApp {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for NotesApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_focused = self.focus_handle.is_focused(window);
        let mut lines = self.note.split('\n').collect::<Vec<_>>();
        if lines.is_empty() {
            lines.push("");
        }

        gpui::div()
            .flex()
            .size_full()
            .font_family(s::FONT)
            .bg(s::GREEN1)
            .text_color(s::GRAY6)
            .child(s::raised(
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
                            .child("notes"),
                    )
                    .child(s::sunken(
                        gpui::div()
                            .flex()
                            .flex_col()
                            .size_full()
                            .p(s::S4)
                            .bg(s::GREEN1)
                            .track_focus(&self.focus_handle)
                            .key_context("NoteEditor")
                            .on_mouse_down(MouseButton::Left, cx.listener(Self::focus_editor))
                            .on_key_down(cx.listener(Self::handle_key_down))
                            .children(render_note_lines(lines, is_focused)),
                    )),
            ))
    }
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
