use iced::{
    alignment,
    widget::{button, text, Button, Text},
    Length,
};

use crate::gui::Message;

pub fn button_with_centered_text(txt: &'static str) -> Button<'static, Message> {
    button(
        text(txt)
            .width(Length::Fill)
            .horizontal_alignment(alignment::Horizontal::Center),
    )
    .height(Length::Fill)
    .width(Length::Fill)
}

pub fn text_centered(txt: &'static str) -> Text {
    text(txt)
        .width(Length::Fill)
        .height(Length::Fill)
        .horizontal_alignment(alignment::Horizontal::Center)
}
