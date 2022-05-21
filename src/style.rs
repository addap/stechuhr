use iced::{button, container, text_input, Background, Color, Vector};

pub struct Logview;
pub struct TabContent;
pub struct TextInput;

impl container::StyleSheet for Logview {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Color::from_rgb8(240, 240, 240).into()),
            border_radius: 5.0,
            border_width: 2.0,
            border_color: Color::BLACK,
            ..container::Style::default()
        }
    }
}

impl container::StyleSheet for TabContent {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Color::from_rgb8(250, 250, 250).into()),
            border_radius: 10.0,
            border_width: 2.0,
            border_color: Color::BLACK,
            ..container::Style::default()
        }
    }
}

impl text_input::StyleSheet for TextInput {
    fn active(&self) -> text_input::Style {
        todo!()
    }

    fn focused(&self) -> text_input::Style {
        todo!()
    }

    fn placeholder_color(&self) -> Color {
        todo!()
    }

    fn value_color(&self) -> Color {
        todo!()
    }

    fn selection_color(&self) -> Color {
        todo!()
    }
}
