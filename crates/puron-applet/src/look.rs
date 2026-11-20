//! The look of the popup's About and Settings pages: the color roles that
//! their plans name, the one function that turns a role into a color of
//! the desktop theme, the text styles, and the rounded card that holds a
//! group of items on both pages.
//!
//! A plan never holds a color, only a role, so that the same plan is
//! drawn in light colors on a dark theme and in dark colors on a light
//! one. Only `color` reads colors from the theme; every widget of the two
//! pages that is not in the theme's default text color gets its color
//! from it.

use cosmic::iced::widget::svg;
use cosmic::iced::{Background, Border, Color, Length, Padding};
use cosmic::widget::{container, icon, text};

use crate::popup::PAGE_SIDE_PADDING;

/// The padding of everything below the separator of the About and
/// Settings pages: `PAGE_SIDE_PADDING` at both sides, as on the
/// temperatures, and some room above the first block and below the last,
/// so that neither touches the separator or the popup's lower edge.
pub fn content_padding() -> Padding {
    let side = f32::from(PAGE_SIDE_PADDING);
    Padding {
        top: 12.0,
        bottom: 24.0,
        left: side,
        right: side,
    }
}

/// The color role of a text, an icon or a tile on the About and Settings
/// pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The text color of whatever holds the widget: no color is set.
    Default,
    /// The grey of secondary text: field labels, small headings, and a
    /// source that is off or not available.
    Muted,
    /// A source that is available.
    Success,
    /// liquidctl in the Flatpak, which cannot run the host's program.
    Warning,
    /// The user's accent color: the About page's links and the Settings
    /// section Display Options.
    Accent,
    /// The Settings section Generic Sensor Providers.
    Yellow,
    /// The Settings section GPU Sensor Providers.
    Purple,
    /// The Settings section Drive Sensor Providers. Not the palette's
    /// blue, because that is the default accent and so the color of
    /// Display Options.
    Indigo,
    /// The Settings section Show Sensors.
    Green,
}

/// Every role, in the order of the enum.
pub const ROLES: [Role; 9] = [
    Role::Default,
    Role::Muted,
    Role::Success,
    Role::Warning,
    Role::Accent,
    Role::Yellow,
    Role::Purple,
    Role::Indigo,
    Role::Green,
];

/// The color of `role` in `theme`, or `None` for `Role::Default`, which
/// leaves the color to the widget that holds the text.
///
/// The grey is the palette's `neutral_9` at alpha 0.7, the color that
/// libcosmic itself gives the placeholder text of a text input. The
/// palette fields are defined for each theme mode, so the same role is
/// light on a dark theme and dark on a light one; the success, warning
/// and accent colors follow the user's own choices when the theme has
/// them.
pub fn color(role: Role, theme: &cosmic::Theme) -> Option<Color> {
    let t = theme.cosmic();
    let p = &t.palette;
    let c = match role {
        Role::Default => return None,
        Role::Muted => {
            let mut grey = p.neutral_9;
            grey.alpha = 0.7;
            grey
        }
        Role::Success => t.success_color(),
        Role::Warning => t.warning_color(),
        Role::Accent => t.accent_text_color(),
        Role::Yellow => p.accent_yellow,
        Role::Purple => p.accent_purple,
        Role::Indigo => p.accent_indigo,
        Role::Green => p.accent_green,
    };
    Some(c.into())
}

/// The color of `role` in the theme that the desktop uses now.
pub fn active(role: Role) -> Option<Color> {
    color(role, &cosmic::theme::active())
}

/// A text style of libcosmic that a plan names for a text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextStyle {
    /// `text::title3`: 24 px, bold.
    Title3,
    /// `text::body`: the normal text.
    Body,
    /// `text::heading`: bold body text.
    Heading,
    /// `text::caption_heading`: 12 px, semibold.
    CaptionHeading,
    /// `text::caption`: 12 px.
    Caption,
}

/// The text `content` in `style`, colored by `role`.
pub fn styled<'a, M: 'a>(content: String, style: TextStyle, role: Role) -> cosmic::Element<'a, M> {
    let t = match style {
        TextStyle::Title3 => text::title3(content),
        TextStyle::Body => text::body(content),
        TextStyle::Heading => text::heading(content),
        TextStyle::CaptionHeading => text::caption_heading(content),
        TextStyle::Caption => text::caption(content),
    };
    match active(role) {
        Some(c) => t.class(cosmic::theme::Text::Color(c)).into(),
        None => t.into(),
    }
}

/// The symbolic icon `name` from the icon theme, `size` px square, drawn
/// in the color of `role`.
pub fn tinted_icon<'a, M: 'a>(name: &'static str, size: u16, role: Role) -> cosmic::Element<'a, M> {
    let i = icon::from_name(name).size(size).symbolic(true).icon();
    match active(role) {
        Some(c) => i
            .class(cosmic::theme::Svg::custom(move |_| svg::Style {
                color: Some(c),
            }))
            .into(),
        None => i.into(),
    }
}

/// The style of a Settings section's icon tile in `theme`: the color of
/// `role`, faint enough for the icon on it to stand out (alpha 0.18 on a
/// dark theme, 0.12 on a light one, where the same color looks stronger),
/// with the theme's small corner radius.
pub fn tile(role: Role, theme: &cosmic::Theme) -> container::Style {
    let t = theme.cosmic();
    let alpha = if t.is_dark { 0.18 } else { 0.12 };
    container::Style {
        background: color(role, theme).map(|mut c| {
            c.a = alpha;
            Background::Color(c)
        }),
        border: Border {
            radius: t.radius_s().into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// The style of the round dot in front of a state line: a disc in the
/// color of `role`.
pub fn dot(role: Role, theme: &cosmic::Theme) -> container::Style {
    container::Style {
        background: color(role, theme).map(Background::Color),
        border: Border {
            radius: 3.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// A card: `content` on a rounded background in the component color of
/// what holds it, the look of libcosmic's settings sections. Its corner
/// radius is the theme's small radius, so the cards follow the user's
/// roundness setting as the rest of the desktop does.
pub fn card<'a, M: 'a>(
    content: impl Into<cosmic::Element<'a, M>>,
    padding: impl Into<Padding>,
) -> container::Container<'a, M, cosmic::Theme, cosmic::Renderer> {
    container(content)
        .padding(padding)
        .width(Length::Fill)
        .class(cosmic::theme::Container::List)
}
