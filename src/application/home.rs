use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::spawn,
};

use poppingboba::{
    help::{HelpTable, HelpWidget},
    spinner::{Spinner, SpinnerType},
};
use ratatui::{
    layout::Layout,
    macros::{constraint, constraints, line, text},
    widgets::{Paragraph, Widget},
};

use crate::application::{
    component::RichContext,
    utils::{
        Either::{Left, Right},
        Separator, WidgetList,
    },
};

use super::component::{Component, Message};

pub struct HomeData {
    loading: Arc<Mutex<Option<Spinner>>>,
    scanning: AtomicBool,
    connected: Arc<Mutex<Connected>>,
    available: Arc<Mutex<Available>>,
    selected: Selected,
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
    list: Vec<ConnectedAPDetails>,
}

struct Available {
    list: Vec<AvailableAPDetails>,
}

struct AvailableAPDetails {
    ssid: String,
}

struct ConnectedAPDetails {
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

                connected
                    .lock()
                    .unwrap()
                    .list
                    .extend((0..2).map(|i| ConnectedAPDetails {
                        ssid: format!("Connection {i}"),
                    }));

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

        // free the lock
        drop(spinner);

        let connected = self.connected.lock().unwrap();
        let (connected_widget, connected_constraint) = if !connected.list.is_empty() {
            let con_sel = self.selected.connected_selected();
            let blocks = connected.list.iter().enumerate().map(|(idx, item)| {
                let selected = con_sel.is_some_and(|sel| sel == idx);
                if selected {
                    Paragraph::new(line!["> ", item.ssid.clone()])
                } else {
                    Paragraph::new(line!["  ", item.ssid.clone()])
                }
            });

            (
                Left(WidgetList::new(
                    Layout::vertical(
                        std::iter::repeat_n(1, connected.list.len()).map(|i| constraint!(== i)),
                    ),
                    blocks,
                )),
                constraint!(== connected.list.len() as u16),
            )
        } else {
            let text = "There are no connections";
            let text = text![line![], line![text].centered(),];

            (Right(text), constraint!(== 3))
        };

        drop(connected);

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
                [Right(Separator('/')), Left(text![scanning_text])],
            ),
            constraint!(== 1),
        );

        let help = match self.selected {
            Selected::Connected(_) => Help::Connected,
            Selected::Available(_) => Help::Available,
            Selected::None => Help::Connected,
        };

        let [c_area, s_area, a_area, h_area] = Layout::vertical([
            connected_constraint,
            separator_constraint,
            available_constraint,
            constraint!(== 1),
        ])
        .areas(frame.area());

        frame.render_widget(connected_widget, c_area);
        frame.render_widget(separator, s_area);
        frame.render_widget(available_widget, a_area);
        frame.render_widget(help, h_area);
    }
}
