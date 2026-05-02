use gpui::prelude::*;

use crate::ui::style as s;

pub fn from_text(label: &'static str, pressed: bool) -> gpui::Div {
    button(label, pressed, Size::Text)
}

pub fn x(pressed: bool) -> gpui::Div {
    button("X", pressed, Size::Square)
}

enum Size {
    Text,
    Square,
}

fn button(label: &'static str, pressed: bool, size: Size) -> gpui::Div {
    let button = gpui::div()
        .flex()
        .items_center()
        .justify_center()
        .bg(s::GRAY3)
        .text_color(s::GRAY6)
        .child(label);

    let button = match size {
        Size::Text => button.p(s::S2).px(s::S3),
        Size::Square => button.size(s::S5),
    };

    if pressed {
        s::sunken(button)
    } else {
        s::raised(button)
    }
}
