use iced::{alignment::Horizontal, Font, Text};

pub const TEXT_SIZE_EMOJI: u16 = crate::TEXT_SIZE;

pub const ICON_FONT: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../resources/Symbola.ttf"),
};

type Emoji = char;

#[allow(non_upper_case_globals)]
pub mod emoji {
    use super::*;

    pub const eye: Emoji = '\u{1F441}';
    pub const checkmark: Emoji = '\u{2705}';
    pub const crossmark: Emoji = '\u{274E}';
    pub const trashcan: Emoji = '\u{1F5D1}';
    pub const floppydisk: Emoji = '\u{1F4BE}';
}

pub fn icon(unicode: char) -> Text {
    Text::new(unicode.to_string())
        .font(ICON_FONT)
        .horizontal_alignment(Horizontal::Center)
        .size(TEXT_SIZE_EMOJI)
}
