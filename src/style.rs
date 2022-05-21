use iced::{container, text_input, Color, TextInput};

pub struct LogviewStyle;
pub struct TabContentStyle;
pub struct TextInputStyle;
pub struct ManagementRow1;
pub struct ManagementRow2;

impl container::StyleSheet for LogviewStyle {
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

impl container::StyleSheet for TabContentStyle {
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

pub fn text_input<'a, F, M>(
    state: &'a mut text_input::State,
    placeholder: &str,
    value: &str,
    f: F,
) -> TextInput<'a, M>
where
    F: 'a + Fn(String) -> M,
    M: Clone,
{
    TextInput::new(state, placeholder, value, f).padding(5)
}

impl container::StyleSheet for ManagementRow1 {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Color::from_rgb8(240, 240, 240).into()),
            ..container::Style::default()
        }
    }
}

impl container::StyleSheet for ManagementRow2 {
    fn style(&self) -> container::Style {
        container::Style {
            background: None,
            ..container::Style::default()
        }
    }
}

pub fn management_row(even: &mut bool) -> Box<dyn container::StyleSheet> {
    let result: Box<dyn container::StyleSheet> = if *even {
        Box::new(ManagementRow1)
    } else {
        Box::new(ManagementRow2)
    };

    *even = !*even;
    result
}
