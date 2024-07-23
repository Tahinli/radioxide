use iced::{
    widget::{button, text, Button, Text},
    Length,
};

use crate::gui::Message;

pub fn button_with_centered_text<T: Into<String>>(txt: T) -> Button<'static, Message> {
    button(text(txt.into()).width(Length::Fill).center())
        .height(Length::Fill)
        .width(Length::Fill)
}

pub fn text_centered<T: Into<String>>(txt: T) -> Text<'static> {
    text(txt.into())
        .width(Length::Fill)
        .height(Length::Fill)
        .center()
}
