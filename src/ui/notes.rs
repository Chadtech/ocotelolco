use gpui::{prelude::*, App, Application, Context, Render, Window, WindowOptions};

use crate::ui::style as s;

pub fn run() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |window, cx| {
            window.set_window_title("Ocotelolco Notes");
            cx.new(|_| NotesApp)
        })
        .expect("failed to open notes window");
    });
}

struct NotesApp;

impl Render for NotesApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
            .flex()
            .size_full()
            .bg(s::GREEN1)
            .text_color(s::GRAY6)
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .w(s::S6)
                    .p(s::S5)
                    .bg(s::GREEN2)
                    .text_color(s::GRAY6)
                    .child(gpui::div().text_xl().child("Notes"))
                    .child(
                        gpui::div()
                            .p(s::S4)
                            .border_1()
                            .bg(s::YELLOW2)
                            .border_color(s::YELLOW5)
                            .text_color(s::YELLOW6)
                            .child("Trading plan"),
                    )
                    .child(
                        gpui::div()
                            .p(s::S4)
                            .border_1()
                            .bg(s::GREEN3)
                            .border_color(s::GREEN5)
                            .child("Watchlist ideas"),
                    )
                    .child(
                        gpui::div()
                            .p(s::S4)
                            .border_1()
                            .bg(s::GREEN3)
                            .border_color(s::GREEN5)
                            .child("Post-trade review"),
                    ),
            )
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .size_full()
                    .p(s::S5)
                    .bg(s::GREEN4)
                    .child(
                        gpui::div()
                            .text_xl()
                            .text_color(s::YELLOW6)
                            .child("Trading plan"),
                    )
                    .child(
                        gpui::div()
                            .size_full()
                            .p(s::S5)
                            .bg(s::GRAY2)
                            .border_1()
                            .border_color(s::GRAY5)
                            .text_color(s::GRAY6)
                            .child("Start writing notes for market context, trade ideas, and risk limits."),
                    ),
            )
    }
}
