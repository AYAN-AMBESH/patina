use std::time::Duration;

use ratatui::{
    layout::{Layout, Rect},
    macros::{constraint, constraints, line, span, text},
    style::Style,
    widgets::{Block, Widget},
};

use crate::application::{
    theme::PATINA,
    utils::{CowStr, Separator, WidgetList, humanize_duration, strength_bars},
};

pub const ITEM_HEIGHT: u16 = 4;

// TODO: remove this clone, only needed for testing
#[derive(Clone)]
pub enum ConnectionData {
    WifiActive {
        strength: f32,
        name: CowStr,
        interface: CowStr,
        ip: CowStr,
        frequency: CowStr,
        link_speed: CowStr,
        versions: CowStr,
    },
    WiredActive {
        interface: CowStr,
        ip: CowStr,
        name: CowStr,
        versions: CowStr,
    },
    WifiInactive {
        name: CowStr,
        last_used: Duration,
        autoconnect: bool,
        metered: bool,
        tag: CowStr,
    },
}

pub struct ConnectedList<'a> {
    pub items: &'a [ConnectionData],
    pub selected: Option<usize>,
}

impl Widget for ConnectedList<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let items = self.items.iter().enumerate().map(|(idx, item)| ConnectionItem {
            data: item,
            selected: self.selected.is_some_and(|sel| sel == idx),
        });

        let list = WidgetList::new(
            Layout::vertical(
                std::iter::repeat_n(ITEM_HEIGHT, self.items.len()).map(|i| constraint!(== i)),
            ),
            items,
        );

        list.render(area, buf);
    }
}

pub struct ConnectionItem<'a> {
    pub data: &'a ConnectionData,
    pub selected: bool,
}

impl Widget for ConnectionItem<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let [top_edge, text_top, text_bottom, bot_edge] =
            Layout::vertical(constraints![== 1, == 1, == 1, == 1]).areas(area);

        let text_band = Rect {
            x: area.x,
            y: text_top.y,
            width: area.width,
            height: text_top.height + text_bottom.height,
        };

        if self.selected {
            let edge_style = Style::new().fg(PATINA.bg_alt).bg(PATINA.bg);
            Separator::new('\u{2584}')
                .styled(edge_style)
                .render(top_edge, buf);
            Separator::new('\u{2580}')
                .styled(edge_style)
                .render(bot_edge, buf);

            Block::new()
                .style(Style::new().bg(PATINA.bg_alt))
                .render(text_band, buf);

            if let Some(cell) = buf.cell_mut((area.x + 1, text_top.y)) {
                cell.set_char('\u{3009}')
                    .set_style(Style::new().fg(PATINA.accent).bg(PATINA.bg_alt));
            }
        }

        // essentially a left-only margin of 2 points
        let margin = 4;
        let area = Rect {
            x: text_band.x + margin,
            y: text_band.y,
            width: text_band.width - margin,
            height: text_band.height,
        };

        match self.data {
            ConnectionData::WifiActive {
                strength,
                name,
                interface,
                ip,
                frequency,
                link_speed,
                versions,
            } => {
                let bars = span!(PATINA.live; "{}  ", strength_bars(*strength));
                let strength = span!(PATINA.dim;"{}%  ", strength.round() as u32);
                let name = span!("{}", name);
                let top = line![bars, strength, name];

                let bottom = line![
                    span!(PATINA.mute; "{ip}  "),
                    span!(PATINA.dim; "on "),
                    span!(PATINA.soft; "{interface} "),
                    span!(PATINA.mute; "· {frequency} · {link_speed} · "),
                    span!(PATINA.live; versions)
                ];

                let text = text![top, bottom];

                text.render(area, buf);
            }
            ConnectionData::WiredActive {
                interface,
                ip,
                name,
                versions,
            } => {
                let bars = span!(PATINA.live; "═══  ");
                let name = span!("{} ", name);
                let kind = span!(PATINA.dim; "· Wired");
                let top = line![bars, name, kind];

                let bottom = line![
                    span!(PATINA.mute; "{ip}  "),
                    span!(PATINA.dim; "on "),
                    span!(PATINA.soft; "{interface} "),
                    span!(PATINA.dim; "· "),
                    span!(PATINA.live; versions)
                ];

                let text = text![top, bottom];
                text.render(area, buf);
            }
            ConnectionData::WifiInactive {
                name,
                last_used,
                autoconnect,
                metered,
                tag,
            } => {
                let kind = span!(PATINA.fg_alt; "wifi  ");
                let name = span!("{}", name);
                let top_left = line![kind, name].left_aligned();

                let tag = span!(PATINA.dim; tag);
                let top_right = line![tag].right_aligned();

                let used_label = span!(PATINA.mute; "used ");
                let used_val = span!(PATINA.dim; "{}  ", humanize_duration(*last_used));
                let ac_label = span!(PATINA.mute; "autoconnect ");
                let ac_val = if *autoconnect {
                    span!(PATINA.live; "on")
                } else {
                    span!(PATINA.dim; "off")
                };

                let mut bottom_spans = vec![used_label, used_val, ac_label, ac_val];
                if *metered {
                    bottom_spans.push(span!("  "));
                    bottom_spans.push(span!(PATINA.warn; "metered"));
                }
                let bottom = ratatui::text::Line::from(bottom_spans);

                let [top_area, bottom_area] =
                    Layout::vertical(constraints![== 1, == 1]).areas(area);

                top_right.render(top_area, buf);
                top_left.render(top_area, buf);
                bottom.render(bottom_area, buf);
            }
        }
    }
}
