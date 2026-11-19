//! The panel button's content: a `Plan` built from the items that
//! `puron_core::panel::panel` chose, and the widgets drawn from it. This
//! module chooses no item, value or icon kind itself and holds no state.
//! It also owns the map from icon kinds to icon names, which the popup
//! uses too.
//!
//! Each value text sits in a *box*: room as wide as the widest of the
//! value and the item's box texts (see `puron_core::panel`), with the
//! value at its right edge. A value that gets shorter then leaves the
//! box, and every element after it, where they were.

use cosmic::applet::{Context, PanelType};
use cosmic::iced::advanced::layout::{Layout, Limits, Node};
use cosmic::iced::advanced::widget::{Tree, Widget};
use cosmic::iced::advanced::{mouse, renderer};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Padding, Rectangle, Size};
use cosmic::widget::{button, column, container, icon, row, text, Id, Space};
use puron_core::menu::{IconKind, Section};
use puron_core::panel::{Icon, Item, Panel};
use puron_core::settings::Settings;

use crate::i18n;

/// The space between neighbouring elements, in px, in both orientations.
/// It is zero: the icon padding and the label padding alone separate them.
pub const SPACING: f32 = 0.0;

/// The padding in px before and after every icon, along a horizontal
/// panel.
pub const ICON_PADDING_HORIZONTAL: f32 = 9.0;

/// The padding in px above and below every icon, along a vertical panel.
pub const ICON_PADDING_VERTICAL: f32 = 5.0;

/// Whether the panel runs across the screen or down its side. The
/// elements are laid out along the panel in this direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

impl Orientation {
    /// The orientation that libcosmic reports for the applet's panel.
    pub fn of(context: &Context) -> Self {
        if context.is_horizontal() {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        }
    }
}

/// The side in px of a square icon, as libcosmic suggests it for the
/// panel's size.
pub fn icon_size(context: &Context) -> u16 {
    context.suggested_size(true).0
}

/// The smallest text size in px that `TextSize::MatchPanel` gives, so that
/// the text stays readable on a very small custom panel.
pub const MATCH_TEXT_SIZE_MIN: u16 = 8;

/// The text size in px that `TextSize::MatchPanel` gives beside icons of
/// `icon_size` px: seven eighths of the icon size, rounded down, and at
/// least `MATCH_TEXT_SIZE_MIN`. At COSMIC's default panel size the icons
/// are 16 px, which gives 14 px, the size the text always had before it
/// could be set.
pub fn match_text_size(icon_size: u16) -> u16 {
    let size = u32::from(icon_size) * 7 / 8;
    (size as u16).max(MATCH_TEXT_SIZE_MIN)
}

/// The text size in px of the panel button's texts. On a dock the dock's
/// setting applies; on a panel, and on a panel of any other type, the
/// panel's setting. The other setting plays no part.
pub fn text_size(settings: &Settings, panel_type: &PanelType, icon_size: u16) -> u16 {
    let setting = match panel_type {
        PanelType::Dock => settings.dock_text_size,
        PanelType::Panel | PanelType::Other(_) => settings.panel_text_size,
    };
    setting
        .fixed_px()
        .unwrap_or_else(|| match_text_size(icon_size))
}

/// Where the applet runs, as libcosmic's applet context describes it:
/// what the panel button's plan and its button depend on besides the
/// items and the settings.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub orientation: Orientation,
    pub icon_size: u16,
    pub panel_type: PanelType,
    /// libcosmic's applet padding in px along the panel, inside the
    /// button.
    pub padding_along: u16,
    /// libcosmic's applet padding in px across the panel, on each side of
    /// the icons. The icon size plus twice this is the panel's thickness.
    pub padding_across: u16,
}

impl Place {
    /// The place that libcosmic's applet context describes.
    pub fn of(context: &Context) -> Place {
        let (along, across) = context.suggested_padding(true);
        Place {
            orientation: Orientation::of(context),
            icon_size: icon_size(context),
            panel_type: context.panel_type.clone(),
            padding_along: along,
            padding_across: across,
        }
    }

    /// The panel's thickness in px: its size across, the icon size plus
    /// the applet padding on both sides.
    pub fn thickness(&self) -> u16 {
        self.icon_size + 2 * self.padding_across
    }
}

/// The icon of the empty-state item: the symbolic form of Puron's app
/// icon, so that a panel with nothing pinned still shows which applet it
/// is. `make install` installs it under hicolor's `symbolic/apps` in every
/// package. Until version 1.6 the GPU temperature icon stood there.
pub const EMPTY_STATE_ICON: &str = "io.github.elalonde.Puron-symbolic";

/// The themed icon name for an icon kind. This is the only map from kinds
/// to names; the popup's section headings use it too. Each kind of device
/// has its own temperature icon, so that two pinned temperatures can be
/// told apart by the icon alone. The drive, Wi-Fi and wired network icons
/// are the COSMIC theme's; the others are Puron's own, installed under
/// hicolor. Voltage and power share one icon.
pub fn icon_name(kind: IconKind) -> &'static str {
    match kind {
        IconKind::GeneralTemperature => "puron-temperature-symbolic",
        IconKind::CpuTemperature => "puron-cpu-temperature-symbolic",
        IconKind::GpuTemperature => "puron-gpu-temperature-symbolic",
        IconKind::DriveTemperature => "drive-harddisk-system-symbolic",
        IconKind::WifiTemperature => "network-wireless-symbolic",
        IconKind::WiredNetworkTemperature => "network-wired-symbolic",
        IconKind::MemoryTemperature => "puron-memory-temperature-symbolic",
        IconKind::Fan => "puron-fan-symbolic",
        IconKind::Voltage | IconKind::Power => "puron-voltage-symbolic",
    }
}

/// The icon name for a popup section heading: the icon of the matching
/// kind, through `icon_name`. Temperature sections use the general
/// temperature icon.
pub fn section_icon_name(section: Section) -> &'static str {
    icon_name(match section {
        Section::Temperature => IconKind::GeneralTemperature,
        Section::Fan => IconKind::Fan,
        Section::Voltage => IconKind::Voltage,
        Section::Power => IconKind::Power,
    })
}

/// One element of the panel button. `item` is the index in
/// `Panel::items` of the item it was built from: a pinned reading gives
/// an optional icon or slot followed by its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Element {
    Icon {
        item: usize,
        name: &'static str,
    },
    /// An empty square of the icon's size, drawn while icons are on but
    /// the pin's icon kind is not known yet, so that its text keeps its
    /// place.
    Slot {
        item: usize,
    },
    Text {
        item: usize,
        text: String,
    },
}

/// Everything the panel button draws, as plain data. `view` builds the
/// widgets from it alone, so tests can check a plan without a display.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub orientation: Orientation,
    /// The box texts of each item, at the item's index in `Panel::items`:
    /// the texts whose width the box of the item's value text takes,
    /// although only the value is drawn. For a pin they are its box text
    /// with its unit (when it has one), then the texts for no value and
    /// for a missing value, so that each of the two fits in the box when
    /// it replaces the value. The empty-state item has none.
    pub boxes: Vec<Vec<String>>,
    pub elements: Vec<Element>,
    pub spacing: f32,
    /// The padding in px before and after each icon or slot along the
    /// panel, outside its square.
    pub icon_padding: f32,
    /// The padding in px left and right of each text, from
    /// `Panel::padding`.
    pub padding: u32,
    pub icon_size: u16,
    /// The size in px of every text, `⚠` included (see [`text_size`]).
    pub text_size: u16,
}

/// The plan for the items of `panel`. `Item::EmptyState`, shown when no
/// pin is of a shown class, becomes `EMPTY_STATE_ICON` alone. A pin
/// becomes its icon (or an empty slot, or nothing when icons are off)
/// followed by its value text. The text size comes from `settings` for
/// the panel type of `place`, at every call, so that a new setting or a
/// new panel size shows at the next drawing.
pub fn plan(panel: &Panel, place: &Place, settings: &Settings) -> Plan {
    let orientation = place.orientation;
    let icon_size = place.icon_size;
    let mut elements = Vec::new();
    for (item, it) in panel.items.iter().enumerate() {
        match it {
            Item::EmptyState => elements.push(Element::Icon {
                item,
                name: EMPTY_STATE_ICON,
            }),
            Item::Pin { icon, value } => {
                match icon {
                    Icon::None => {}
                    Icon::Kind(k) => elements.push(Element::Icon {
                        item,
                        name: icon_name(*k),
                    }),
                    Icon::EmptySlot => elements.push(Element::Slot { item }),
                }
                let text = i18n::panel_value(value.as_ref().map(|v| v.as_ref()));
                elements.push(Element::Text { item, text });
            }
        }
    }
    let boxes = panel
        .items
        .iter()
        .enumerate()
        .map(|(item, it)| match it {
            Item::EmptyState => Vec::new(),
            Item::Pin { .. } => {
                let template = panel.boxes.get(item).and_then(Option::as_ref);
                let mut texts: Vec<String> = template
                    .map(|t| i18n::value_text(Some(t)))
                    .into_iter()
                    .collect();
                texts.push(i18n::value_text(None));
                texts.push(i18n::panel_value(None));
                texts
            }
        })
        .collect();
    Plan {
        orientation,
        boxes,
        elements,
        spacing: SPACING,
        icon_padding: match orientation {
            Orientation::Horizontal => ICON_PADDING_HORIZONTAL,
            Orientation::Vertical => ICON_PADDING_VERTICAL,
        },
        padding: panel.padding,
        icon_size,
        text_size: text_size(settings, &place.panel_type, icon_size),
    }
}

/// The widget ID of the element at `index` in `Plan::elements`, so that
/// tests can find its rectangle. For a text element that rectangle is its
/// box.
pub fn element_id(index: usize) -> Id {
    Id::new(format!("e{index}"))
}

/// The widget ID of the value text inside the box of the text element at
/// `index` in `Plan::elements`, so that tests can find the value's own
/// rectangle.
pub fn value_id(index: usize) -> Id {
    Id::new(format!("v{index}"))
}

/// The panel button's content, built from the plan alone: a row on a
/// horizontal panel, a column on a vertical one, centred across the
/// panel. Each element sits in a container whose widget ID is
/// `element_id(index)`. That container is exactly the text's box or the
/// square's own rectangle; the padding is applied by an outer container,
/// so it is not part of the rectangle.
pub fn view<'a, M: 'a>(plan: &Plan) -> cosmic::Element<'a, M> {
    let size = plan.icon_size;
    let pad = plan.padding as f32;
    let around = icon_padding(plan);
    let children: Vec<cosmic::Element<'a, M>> = plan
        .elements
        .iter()
        .enumerate()
        .map(|(i, e)| match e {
            Element::Icon { name, .. } => container(
                container(
                    icon::from_name(*name)
                        .size(size)
                        .symbolic(true)
                        .fallback(None)
                        .icon(),
                )
                .id(element_id(i)),
            )
            .padding(around)
            .into(),
            Element::Slot { .. } => container(
                container(
                    Space::new()
                        .width(Length::Fixed(f32::from(size)))
                        .height(Length::Fixed(f32::from(size))),
                )
                .id(element_id(i)),
            )
            .padding(around)
            .into(),
            // The box is a column, aligned to the right, of the box texts'
            // room and the value: as wide as the widest of them and as
            // high as the value. The text carries its own vertical
            // centring. A text taller than its room is drawn from the text
            // widget's alignment, not its row's, so without it a large
            // text would start at the top of the button and be cut only at
            // the bottom.
            Element::Text { item, text: t } => container(
                container(
                    column::with_children(vec![
                        Widest::new(
                            plan.boxes
                                .get(*item)
                                .into_iter()
                                .flatten()
                                .map(|b| text(b.clone()).size(plan.text_size).into())
                                .collect(),
                        )
                        .into(),
                        container(
                            text(t.clone())
                                .size(plan.text_size)
                                .align_y(Vertical::Center),
                        )
                        .id(value_id(i))
                        .into(),
                    ])
                    .align_x(Horizontal::Right),
                )
                .id(element_id(i)),
            )
            .padding([0.0, pad])
            .into(),
        })
        .collect();
    match plan.orientation {
        Orientation::Horizontal => row::with_children(children)
            .spacing(plan.spacing)
            .align_y(Alignment::Center)
            .height(Length::Fill)
            .into(),
        Orientation::Vertical => column::with_children(children)
            .spacing(plan.spacing)
            .align_x(Alignment::Center)
            .width(Length::Fill)
            .into(),
    }
}

/// The panel button: the plan's content inside libcosmic's applet button.
/// On a horizontal panel the button is exactly as high as the panel is
/// thick, whatever the text size, so a text taller than the panel is cut
/// by the panel's edge rather than making the applet taller than the
/// panel. Its content then fills that height with no padding above or
/// below: the elements are centred in it, so the icons sit where the
/// applet padding would put them, and a text has the whole thickness of
/// the panel before it is cut. On a vertical panel the button has the
/// applet padding of `place` on all sides and takes the width of its
/// widest element.
pub fn button<'a, M: Clone + 'a>(
    plan: &Plan,
    place: &Place,
    on_press: M,
) -> cosmic::Element<'a, M> {
    let (along, across) = (
        f32::from(place.padding_along),
        f32::from(place.padding_across),
    );
    let b = button::custom(view::<M>(plan))
        .class(cosmic::theme::Button::AppletIcon)
        .on_press(on_press);
    match plan.orientation {
        Orientation::Horizontal => b
            .padding([0.0, along])
            .height(Length::Fixed(f32::from(place.thickness())))
            .into(),
        Orientation::Vertical => b.padding([along, across]).into(),
    }
}

/// The plan's icon padding along the panel only: left and right on a
/// horizontal panel, top and bottom on a vertical one.
fn icon_padding(plan: &Plan) -> Padding {
    let p = plan.icon_padding;
    match plan.orientation {
        Orientation::Horizontal => Padding {
            top: 0.0,
            right: p,
            bottom: 0.0,
            left: p,
        },
        Orientation::Vertical => Padding {
            top: p,
            right: 0.0,
            bottom: p,
            left: 0.0,
        },
    }
}

/// Room as wide as the widest of some texts, with no height, that draws
/// nothing. The texts are laid out in the size and font they are given,
/// so the width is exact for whatever font and language are in use and
/// Puron never measures a text itself. Over a value in a column it makes
/// the value's box as wide as the widest text the box must hold, without
/// making it higher. The texts are not drawn, nor offered to screen
/// readers, as they are never shown.
struct Widest<'a, M> {
    texts: Vec<cosmic::Element<'a, M>>,
}

impl<'a, M> Widest<'a, M> {
    fn new(texts: Vec<cosmic::Element<'a, M>>) -> Self {
        Widest { texts }
    }
}

impl<M> Widget<M, cosmic::Theme, cosmic::Renderer> for Widest<'_, M> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn children(&self) -> Vec<Tree> {
        self.texts.iter().map(Tree::new).collect()
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(&mut self.texts);
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &cosmic::Renderer, limits: &Limits) -> Node {
        let width = self
            .texts
            .iter_mut()
            .zip(&mut tree.children)
            .map(|(t, tree)| {
                t.as_widget_mut()
                    .layout(tree, renderer, limits)
                    .size()
                    .width
            })
            .fold(0.0, f32::max);
        Node::new(limits.resolve(Length::Shrink, Length::Shrink, Size::new(width, 0.0)))
    }

    fn draw(
        &self,
        _tree: &Tree,
        _renderer: &mut cosmic::Renderer,
        _theme: &cosmic::Theme,
        _style: &renderer::Style,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
    }
}

impl<'a, M: 'a> From<Widest<'a, M>> for cosmic::Element<'a, M> {
    fn from(w: Widest<'a, M>) -> Self {
        cosmic::Element::new(w)
    }
}
