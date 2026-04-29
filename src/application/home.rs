use std::{
    sync::{Arc, Mutex},
    thread::spawn,
    time::Duration,
};

use poppingboba::{
    help::{HelpTable, HelpWidget},
    spinner::{Spinner, SpinnerType},
};
use ratatui::{
    layout::Layout,
    macros::{constraint, constraints, line, span, text},
    style::Color,
    widgets::Widget,
};

use crate::application::{
    component::RichContext,
    home::{
        available::{AvailableAPDetails, AvailableList},
        connected::{ConnectedList, ConnectionData, ITEM_HEIGHT},
    },
    theme::PATINA,
    utils::{
        CowStr,
        Either::{Left, Right},
        Separator, WidgetList,
    },
};

mod available;
mod connected;

use super::component::{Component, Message};

pub struct HomeData {
    loading: Arc<Mutex<Option<Spinner>>>,
    connected: Arc<Mutex<Connected>>,
    available: Arc<Mutex<Available>>,
    selected: Selected,
}

struct ConnectionsHeader {
    pub saved_count: usize,
    pub active_count: usize,
    pub selected: Option<(usize, usize)>, // (idx, total)
    pub disabled: bool,
}

struct Header {
    pub hostname: CowStr,
    // TODO: make a status enum based on nm connectivity status, update real-time
    pub online: bool,
    pub active_count: u64,
    pub nm_version: CowStr,
}

impl Widget for &ConnectionsHeader {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let disabled_color = |color: Color| {
            if self.disabled { PATINA.mute } else { color }
        };

        let title = span!(disabled_color(PATINA.accent); "CONNECTIONS  ");
        let status = span!(disabled_color(PATINA.mute); "{} saved · {} active", self.saved_count, self.active_count);

        let line = line![title, status];

        if let Some((idx, total)) = self.selected {
            let tag = span!(disabled_color(PATINA.mute); "selected ");
            let page = span!(disabled_color(PATINA.dim); "{}", idx + 1);
            let total = span!(disabled_color(PATINA.mute); "/{total}");

            let line = line![tag, page, total].right_aligned();
            line.render(area, buf);
        }

        line.render(area, buf);
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
    list: Vec<ConnectionData>,
}

struct Available {
    list: Vec<AvailableAPDetails>,
}

struct AccessPointsHeader {
    pub in_range: u64,
    pub scanned_ago: CowStr,
    pub selected: Option<(usize, usize)>, // (idx, total)
    pub disabled: bool,
}

impl Widget for &AccessPointsHeader {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let disabled_color = |color: Color| {
            if self.disabled { PATINA.mute } else { color }
        };

        let title = span!(disabled_color(PATINA.accent); "ACCESS POINTS  ");
        let status = span!(disabled_color(PATINA.mute); "{} in range · scanned {}", self.in_range, self.scanned_ago);

        let line = line![title, status];

        if let Some((idx, total)) = self.selected {
            let tag = span!(disabled_color(PATINA.mute); "selected ");
            let page = span!(disabled_color(PATINA.dim); "{}", idx + 1);
            let total = span!(disabled_color(PATINA.mute); "/{total}");

            let line = line![tag, page, total].right_aligned();
            line.render(area, buf);
        }

        line.render(area, buf);
    }
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
        let connected = Arc::new(Mutex::new(Connected { list: Vec::new() }));
        let available = Arc::new(Mutex::new(Available { list: Vec::new() }));
        let selected = Selected::None;

        // TODO: right now, when the app starts, no section is selected. change
        // that to select the very first item in the connection list. of course,
        // handling the edge cases
        spawn({
            let loading = loading.clone();
            let connected = connected.clone();
            let available = available.clone();
            move || {
                // mock loading, uncomment to checkout loading state
                // sleep(Duration::from_secs(5));
                available.lock().unwrap().list.extend(
                    [
                        AvailableAPDetails {
                            strength: 82.,
                            ssid: "Overcast-5G".into(),
                            security: "WPA2".into(),
                            frequency: "5.22 GHz".into(),
                            channel: 44,
                            link_speed: "650 Mbps".into(),
                        },
                        AvailableAPDetails {
                            strength: 71.,
                            ssid: "Acme-Corp".into(),
                            security: "WPA2-E".into(),
                            frequency: "5.75 GHz".into(),
                            channel: 149,
                            link_speed: "867 Mbps".into(),
                        },
                        AvailableAPDetails {
                            strength: 58.,
                            ssid: "FiberLink_9A82".into(),
                            security: "WPA3".into(),
                            frequency: "6.13 GHz".into(),
                            channel: 37,
                            link_speed: "1201 Mbps".into(),
                        },
                        AvailableAPDetails {
                            strength: 42.,
                            ssid: "xfinitywifi".into(),
                            security: "Open".into(),
                            frequency: "2.41 GHz".into(),
                            channel: 1,
                            link_speed: "150 Mbps".into(),
                        },
                        AvailableAPDetails {
                            strength: 28.,
                            ssid: "TP-Link_5544".into(),
                            security: "WPA2".into(),
                            frequency: "2.44 GHz".into(),
                            channel: 6,
                            link_speed: "300 Mbps".into(),
                        },
                    ]
                    .into_iter()
                    .cycle()
                    .take(24),
                );

                let conns = vec![
                    ConnectionData::WifiActive {
                        strength: 82.,
                        name: "Overcast-5G".into(),
                        interface: "wlan0".into(),
                        ip: "10.0.0.147/24".into(),
                        frequency: "5 GHz".into(),
                        link_speed: "650 Mbps".into(),
                        versions: "v4+v6".into(),
                    },
                    ConnectionData::WiredActive {
                        interface: "eth0".into(),
                        ip: "192.168.4.22/24".into(),
                        name: "eth0".into(),
                        versions: "v4+v6".into(),
                    },
                    ConnectionData::WifiInactive {
                        name: "Acme-Corp".into(),
                        last_used: Duration::from_secs(60 * 60 * 24),
                        autoconnect: true,
                        metered: false,
                        tag: "WPA2-Enterprise".into(),
                    },
                    ConnectionData::WifiInactive {
                        name: "Pixel-Tether".into(),
                        last_used: Duration::from_secs(60 * 60 * 24 * 14),
                        autoconnect: false,
                        metered: true,
                        tag: "WPA3".into(),
                    },
                ];
                connected
                    .lock()
                    .unwrap()
                    .list
                    .extend(conns.into_iter().cycle().take(9));

                *loading.lock().unwrap() = None;
            }
        });

        Self {
            loading,
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

        let connected = self.connected.lock().unwrap();
        let total_connected = connected.list.len();

        let (connections_header_widget, connections_header_constraint) = (
            ConnectionsHeader {
                saved_count: 5,
                active_count: 2,
                disabled: self.selected.connected_selected().is_none(),
                selected: self
                    .selected
                    .connected_selected()
                    .map(|idx| (idx, total_connected)),
            },
            constraint!(== 1),
        );

        // TODO: when scrolling crosses the pane boundary, the connected list snaps back to rendering from 0
        // keep additional data to avoid that snapping
        let (connected_widget, connected_constraint) = if !connected.list.is_empty() {
            (
                Left(ConnectedList {
                    items: &connected.list,
                    selected: self.selected.connected_selected(),
                    // TODO: load from app context
                    max_items: 5,
                }),
                // TODO: load from AppContext
                constraint!(== connected.list.len().min(5) as u16 * ITEM_HEIGHT),
            )
        } else {
            let text = "There are no connections";
            let text = text![line![], line![text].centered(),];

            (Right(text), constraint!(== 3))
        };

        let available = self.available.lock().unwrap();
        let (access_points_header_widget, access_points_header_constraint) = (
            AccessPointsHeader {
                in_range: available.list.len() as u64,
                scanned_ago: "just now".into(),
                disabled: self.selected.available_selected().is_none(),
                selected: self
                    .selected
                    .available_selected()
                    .map(|idx| (idx, available.list.len())),
            },
            constraint!(== 1),
        );

        let (available_widget, available_constraint) = if !available.list.is_empty() {
            (
                Left(AvailableList {
                    items: &available.list,
                    selected: self.selected.available_selected(),
                }),
                constraint!(*= 1),
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
            ah_area,
            a_area,
            h_area,
        ] = Layout::vertical([
            header_constraint,
            header_sep_constraint,
            connections_header_constraint,
            connected_constraint,
            access_points_header_constraint,
            available_constraint,
            constraint!(== 1),
        ])
        .areas(frame.area());

        frame.render_widget(&header_widget, header_area);
        frame.render_widget(header_sep_widget, hs_area);
        frame.render_widget(&connections_header_widget, ch_area);
        frame.render_widget(connected_widget, c_area);
        frame.render_widget(&access_points_header_widget, ah_area);
        frame.render_widget(available_widget, a_area);
        frame.render_widget(help, h_area);
    }
}
