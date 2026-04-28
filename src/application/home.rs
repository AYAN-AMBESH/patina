use std::{
    borrow::Cow,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::spawn,
    time::Duration,
};

use poppingboba::{
    help::{HelpTable, HelpWidget},
    spinner::{Spinner, SpinnerType},
};
use ratatui::{
    layout::{Layout, Rect},
    macros::{constraint, constraints, line, span, text},
    style::Style,
    widgets::{Block, Paragraph, Widget},
};

use crate::application::{
    component::RichContext,
    theme::PATINA,
    utils::{
        Either::{Left, Right},
        Separator, WidgetList,
    },
};

const ITEM_HEIGHT: u16 = 4;

fn strength_bars(s: f32) -> &'static str {
    if s >= 75.0 {
        "▮▮▮▮"
    } else if s >= 55.0 {
        "▮▮▮▯"
    } else if s >= 35.0 {
        "▮▮▯▯"
    } else if s >= 15.0 {
        "▮▯▯▯"
    } else {
        "▯▯▯▯"
    }
}

fn humanize_duration(d: Duration) -> String {
    timeago::Formatter::new().convert(d)
}

use super::component::{Component, Message};

pub struct HomeData {
    loading: Arc<Mutex<Option<Spinner>>>,
    scanning: AtomicBool,
    connected: Arc<Mutex<Connected>>,
    available: Arc<Mutex<Available>>,
    selected: Selected,
}

type CowStr = Cow<'static, str>;

enum ConnectionData {
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

struct ConnectionItem<'a> {
    pub data: &'a ConnectionData,
    pub selected: bool,
}

struct ConnectionsHeader {
    pub saved_count: u64,
    pub active_count: u64,
}

struct Header {
    pub hostname: Cow<'static, str>,
    // TODO: make a status enum based on nm connectivity status, update real-time
    pub online: bool,
    pub active_count: u64,
    pub nm_version: Cow<'static, str>,
}

impl Widget for &ConnectionsHeader {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let title = span!(PATINA.accent; "CONNECTIONS  ");
        let status =
            span!(PATINA.mute; "{} saved · {} active", self.saved_count, self.active_count);

        let line = line![title, status];

        line.render(area, buf);
    }
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
                    span!(PATINA.dim; "on  "),
                    span!(PATINA.soft; "{interface} "),
                    span!(PATINA.dim; "· {frequency} · {link_speed} · "),
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
                    span!(PATINA.dim; "on  "),
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
                let kind = span!(PATINA.accent; "wifi  ");
                let name = span!("{}", name);
                let top_left = line![kind, name].left_aligned();

                let tag = span!(PATINA.dim; tag);
                let top_right = line![tag].right_aligned();

                let used_label = span!(PATINA.dim; "used ");
                let used_val = span!("{}   ", humanize_duration(*last_used));
                let ac_label = span!(PATINA.dim; "autoconnect ");
                let ac_val = span!("{}", if *autoconnect { "on" } else { "off" });

                let mut bottom_spans = vec![used_label, used_val, ac_label, ac_val];
                if *metered {
                    bottom_spans.push(span!("   "));
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

impl Widget for &Header {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let patina = span!("patina");
        let diamond = span!(PATINA.accent; "  ◆  ");
        let hostname = span!(PATINA.soft; self.hostname);
        let dot = span!(PATINA.dim; "  ·  ");

        let status = span!(PATINA.live; "{} · {} active", if self.online { "online" }  else {"offline"}, self.active_count);

        let left = line![patina, diamond, hostname, dot, status].left_aligned();

        let version = span!(PATINA.dim; self.nm_version);
        let right = line![version].right_aligned();

        // TODO: handle the case where there's not enough room, and truncation is needed
        right.render(area, buf);
        left.render(area, buf);
    }
}

enum Help {
    Connected,
    Available,
}

impl Widget for Help {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        match self {
            Help::Connected => {
                let w = HelpWidget::new(HelpTable::new(
                    [
                        ("quit", ("q", "quit").into()),
                        ("nav", ("j/k", "navigate").into()),
                        ("con", ("enter", "disconnect").into()),
                        ("tab", ("tab", "move to access points").into()),
                        ("scan", ("s", "force scan").into()),
                    ],
                    &["nav", "con", "scan", "quit"],
                    (&[], 0),
                ));

                w.render(area, buf);
            }
            Help::Available => {
                let w = HelpWidget::new(HelpTable::new(
                    [
                        ("quit", ("q", "quit").into()),
                        ("nav", ("j/k", "navigate").into()),
                        ("con", ("enter", "connect").into()),
                        ("tab", ("tab", "move to connections").into()),
                        ("scan", ("s", "force scan").into()),
                    ],
                    &["nav", "con", "scan", "quit"],
                    (&[], 0),
                ));

                w.render(area, buf);
            }
        }
    }
}

struct Connected {
    list: Vec<Arc<ConnectionData>>,
}

struct Available {
    list: Vec<AvailableAPDetails>,
}

struct AvailableAPDetails {
    ssid: String,
}

enum Selected {
    Connected(usize),
    Available(usize),
    None,
}

impl Selected {
    fn down(&self, con_len: usize, avail_len: usize) -> Self {
        match self {
            Selected::Connected(idx) => {
                let next = idx + 1;
                if next == con_len {
                    Self::Available(0)
                } else {
                    Self::Connected(next)
                }
            }
            Selected::Available(idx) => {
                let next = idx + 1;
                if next == avail_len {
                    Self::Available(*idx)
                } else {
                    Self::Available(next)
                }
            }
            Selected::None => {
                if con_len > 0 {
                    Self::Connected(0)
                } else if avail_len > 0 {
                    Self::Available(0)
                } else {
                    Self::None
                }
            }
        }
    }

    fn up(&self, con_len: usize, _avail_len: usize) -> Self {
        match self {
            Selected::Connected(idx) => {
                if *idx == 0 {
                    Self::None
                } else {
                    Self::Connected(idx - 1)
                }
            }
            Selected::Available(idx) => {
                let idx = *idx;
                if idx == 0 && con_len != 0 {
                    Self::Connected(con_len - 1)
                } else if idx == 0 && con_len == 0 {
                    Self::None
                } else {
                    Self::Available(idx - 1)
                }
            }
            Selected::None => Self::None,
        }
    }

    fn tab(&self, con_len: usize, avail_len: usize) -> Self {
        match self {
            Selected::Connected(idx) => {
                if avail_len == 0 {
                    Self::Connected(*idx)
                } else {
                    Self::Available(0)
                }
            }
            Selected::Available(idx) => {
                if con_len == 0 {
                    Self::Available(*idx)
                } else {
                    Self::Connected(0)
                }
            }
            Selected::None => {
                if con_len != 0 {
                    Self::Connected(0)
                } else if avail_len != 0 {
                    Self::Available(0)
                } else {
                    Self::None
                }
            }
        }
    }

    fn connected_selected(&self) -> Option<usize> {
        match self {
            Selected::Connected(idx) => Some(*idx),
            _ => None,
        }
    }
    fn available_selected(&self) -> Option<usize> {
        match self {
            Selected::Available(idx) => Some(*idx),
            _ => None,
        }
    }
}

impl HomeData {
    pub fn new(ctx: &RichContext) -> Self {
        let loading = Spinner::new(SpinnerType::dot(), ctx.fps);
        let loading = Arc::new(Mutex::new(Some(loading)));
        let scanning = AtomicBool::new(false);
        let connected = Arc::new(Mutex::new(Connected { list: Vec::new() }));
        let available = Arc::new(Mutex::new(Available { list: Vec::new() }));
        let selected = Selected::None;

        // run async task to load connections and update state accordingly
        spawn({
            let loading = loading.clone();
            let connected = connected.clone();
            let available = available.clone();
            move || {
                // mock loading, uncomment to checkout loading state
                // sleep(Duration::from_secs(5));
                available
                    .lock()
                    .unwrap()
                    .list
                    .extend((0..=4).map(|i| AvailableAPDetails {
                        ssid: format!("Access Point {i}"),
                    }));

                let mut conns: Vec<Arc<ConnectionData>> = Vec::new();
                conns.push(Arc::new(ConnectionData::WifiActive {
                    strength: 82.,
                    name: "Overcast-5G".into(),
                    interface: "wlan0".into(),
                    ip: "10.0.0.147/24".into(),
                    frequency: "5 GHz".into(),
                    link_speed: "650 Mbps".into(),
                    versions: "v4+v6".into(),
                }));
                conns.push(Arc::new(ConnectionData::WiredActive {
                    interface: "eth0".into(),
                    ip: "192.168.4.22/24".into(),
                    name: "eth0".into(),
                    versions: "v4+v6".into(),
                }));
                conns.push(Arc::new(ConnectionData::WifiInactive {
                    name: "Acme-Corp".into(),
                    last_used: Duration::from_secs(60 * 60 * 24),
                    autoconnect: true,
                    metered: false,
                    tag: "WPA2-Enterprise".into(),
                }));
                conns.push(Arc::new(ConnectionData::WifiInactive {
                    name: "Pixel-Tether".into(),
                    last_used: Duration::from_secs(60 * 60 * 24 * 14),
                    autoconnect: false,
                    metered: true,
                    tag: "WPA3".into(),
                }));
                connected.lock().unwrap().list.extend(conns);

                *loading.lock().unwrap() = None;
            }
        });

        Self {
            loading,
            scanning,
            connected,
            available,
            selected,
        }
    }
}

impl Component for HomeData {
    fn update(&mut self, _ctx: &RichContext, ev: Message) {
        match ev {
            Message::Crossterm(ev) => {
                match ev {
                    ratatui::crossterm::event::Event::Key(key_event)
                        if key_event.code.is_char('j') =>
                    {
                        let con_len = self.connected.lock().unwrap().list.len();
                        let avail_len = self.available.lock().unwrap().list.len();
                        self.selected = self.selected.down(con_len, avail_len);
                    }
                    ratatui::crossterm::event::Event::Key(key_event)
                        if key_event.code.is_char('k') =>
                    {
                        let con_len = self.connected.lock().unwrap().list.len();
                        let avail_len = self.available.lock().unwrap().list.len();
                        self.selected = self.selected.up(con_len, avail_len);
                    }
                    ratatui::crossterm::event::Event::Key(key_event) if key_event.code.is_tab() => {
                        let con_len = self.connected.lock().unwrap().list.len();
                        let avail_len = self.available.lock().unwrap().list.len();
                        self.selected = self.selected.tab(con_len, avail_len);
                    }
                    _ => {
                        // noop
                    }
                }
            }
            Message::GlobalTick => {
                let mut spinner = self.loading.lock().unwrap();
                if let Some(spinner) = spinner.as_mut() {
                    spinner.tick();
                }
            }
        }
    }

    fn draw(&self, _ctx: &RichContext, frame: &mut ratatui::Frame<'_>) {
        let spinner = self.loading.lock().unwrap();
        if let Some(spinner) = spinner.as_ref() {
            let text = " Loading patina";
            let center = frame
                .area()
                .centered(constraint!(== text.len() as u16 + 1), constraint!(== 1));

            let [spinner_area, text_area] =
                Layout::horizontal(constraints![== 1, *=  text.len() as u16]).areas(center);

            frame.render_widget(spinner, spinner_area);
            frame.render_widget(line![text], text_area);
            return;
        }

        drop(spinner);

        let (header_widget, header_constraint) = (
            Header {
                hostname: "stacyweiss".into(),
                online: true,
                active_count: 2,
                nm_version: "NM 2.23".into(),
            },
            constraint!(== 1),
        );

        let (header_sep_widget, header_sep_constraint) = (
            Separator::new('\x5F').styled(PATINA.mute.into()),
            constraint!(== 2),
        );

        let (connections_header_widget, connections_header_constraint) = (
            ConnectionsHeader {
                saved_count: 5,
                active_count: 2,
            },
            constraint!(== 1),
        );

        let connected = self.connected.lock().unwrap();
        let (connected_widget, connected_constraint) = if !connected.list.is_empty() {
            let con_sel = self.selected.connected_selected();
            let blocks = connected.list.iter().enumerate().map(|(idx, item)| {
                let selected = con_sel.is_some_and(|sel| sel == idx);
                ConnectionItem {
                    data: item,
                    selected,
                }
            });

            (
                Left(WidgetList::new(
                    Layout::vertical(
                        std::iter::repeat_n(ITEM_HEIGHT, connected.list.len())
                            .map(|i| constraint!(== i)),
                    ),
                    blocks,
                )),
                constraint!(== connected.list.len() as u16 * ITEM_HEIGHT),
            )
        } else {
            let text = "There are no connections";
            let text = text![line![], line![text].centered(),];

            (Right(text), constraint!(== 3))
        };

        let available = self.available.lock().unwrap();
        let (available_widget, available_constraint) = if !available.list.is_empty() {
            let avail_sel = self.selected.available_selected();
            let blocks = available.list.iter().enumerate().map(|(idx, item)| {
                let selected = avail_sel.is_some_and(|sel| sel == idx);
                if selected {
                    Paragraph::new(line!["> ", item.ssid.clone()])
                } else {
                    Paragraph::new(line!["  ", item.ssid.clone()])
                }
            });

            (
                Left(WidgetList::new(
                    Layout::vertical(
                        std::iter::repeat_n(1, available.list.len()).map(|i| constraint!(== i)),
                    ),
                    blocks,
                )),
                constraint!(*= available.list.len() as u16),
            )
        } else {
            let text = "There are no access points available";
            let text = text![line![text].centered()];

            (
                Right(WidgetList::new(
                    Layout::vertical(constraints![== 1]).flex(ratatui::layout::Flex::SpaceAround),
                    text,
                )),
                constraint!(*= 1),
            )
        };

        drop(available);

        let scanning_text = if self.scanning.load(Ordering::Relaxed) {
            " [+] Scanning ..."
        } else {
            " [-] Scanned     "
        };

        let (separator, separator_constraint) = (
            WidgetList::new(
                Layout::horizontal(constraints![*= 0, == scanning_text.len() as u16]),
                [Right(Separator::new('/')), Left(text![scanning_text])],
            ),
            constraint!(== 1),
        );

        let help = match self.selected {
            Selected::Connected(_) => Help::Connected,
            Selected::Available(_) => Help::Available,
            Selected::None => Help::Connected,
        };

        let [
            header_area,
            hs_area,
            ch_area,
            c_area,
            s_area,
            a_area,
            h_area,
        ] = Layout::vertical([
            header_constraint,
            header_sep_constraint,
            connections_header_constraint,
            connected_constraint,
            separator_constraint,
            available_constraint,
            constraint!(== 1),
        ])
        .areas(frame.area());

        frame.render_widget(&header_widget, header_area);
        frame.render_widget(header_sep_widget, hs_area);
        frame.render_widget(&connections_header_widget, ch_area);
        frame.render_widget(connected_widget, c_area);
        frame.render_widget(separator, s_area);
        frame.render_widget(available_widget, a_area);
        frame.render_widget(help, h_area);
    }
}
