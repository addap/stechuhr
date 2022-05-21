use iced::{alignment::Horizontal, Font, Length, Text};

pub const TEXT_SIZE_EMOJI: u16 = 38;
pub const ICONS: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../resources/NotoSansSymbols2-Regular.ttf"),
};

pub fn icon(unicode: char) -> Text {
    Text::new(unicode.to_string())
        .font(ICONS)
        .width(Length::Units(20))
        .horizontal_alignment(Horizontal::Center)
        .size(TEXT_SIZE_EMOJI)
}

pub fn eye_unicode() -> char {
    '\u{1F441}'
}

pub fn checkmark_unicode() -> char {
    '\u{2705}'
}

pub fn crossmark_unicode() -> char {
    '\u{274E}'
}
