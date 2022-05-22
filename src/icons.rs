use iced::{alignment::Horizontal, Color, Font, Text};

pub const TEXT_SIZE_EMOJI: u16 = crate::TEXT_SIZE;

pub const FONT_SYMBOLA: Font = Font::External {
    name: "Symbola",
    bytes: include_bytes!("../resources/Symbola.ttf"),
};

pub const FONT_EMOJIONE: Font = Font::External {
    name: "EmojiOne",
    bytes: include_bytes!("../resources/font-adobe/EmojiOneColor.otf"),
};

pub struct Emoji {
    pub codepoint: char,
    font: Font,
    color: Option<Color>,
    size: u16,
}

impl Emoji {
    const fn new(codepoint: char) -> Self {
        Self {
            codepoint,
            font: FONT_SYMBOLA,
            color: None,
            size: TEXT_SIZE_EMOJI,
        }
    }

    pub const fn with_font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    pub const fn with_color(mut self, color: Option<Color>) -> Self {
        self.color = color;
        self
    }

    pub const fn with_size(mut self, size: u16) -> Self {
        self.size = size;
        self
    }
}
#[allow(non_upper_case_globals)]
pub mod emoji {
    use super::*;

    pub const eye: Emoji = Emoji::new('\u{1F441}');
    pub const checkmark: Emoji = Emoji::new('\u{2705}');
    pub const crossmark: Emoji = Emoji::new('\u{274E}');
    pub const trashcan: Emoji = Emoji::new('\u{1F5D1}');
    pub const floppydisk: Emoji = Emoji::new('\u{1F4BE}');
}

pub fn icon(emoji: Emoji) -> Text {
    let t = Text::new(emoji.codepoint.to_string())
        .font(emoji.font)
        .size(emoji.size)
        .horizontal_alignment(Horizontal::Center);

    if let Some(color) = emoji.color {
        t.color(color)
    } else {
        t
    }
}
