use iced::{
    alignment,
    widget::{button, text, Button},
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
