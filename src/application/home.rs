use std::{
    sync::{Arc, Mutex, atomic::AtomicBool},
    thread::{sleep, spawn},
    time::Duration,
};

use poppingboba::spinner::{Spinner, SpinnerType};
use ratatui::{
    layout::Layout,
    macros::{constraint, constraints, line, text},
    widgets::Paragraph,
};

use crate::application::{
    component::RichContext,
    utils::Either::{Left, Right},
    utils::WidgetList,
};

use super::component::{Component, Message};

pub struct HomeData {
    loading: Arc<Mutex<Option<Spinner>>>,
    scanning: AtomicBool,
    connected: Arc<Mutex<Connected>>,
    available: Arc<Mutex<Available>>,
    selected: Selected,
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
            Message::Crossterm(..) => {
                // noop
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
            let blocks = connected
                .list
                .iter()
                .map(|item| Paragraph::new(text![item.ssid.clone()]));

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
            let blocks = available
                .list
                .iter()
                .map(|item| Paragraph::new(text![item.ssid.clone()]));

            (
                Left(WidgetList::new(
                    Layout::vertical(
                        std::iter::repeat_n(1, available.list.len()).map(|i| constraint!(== i)),
                    ),
                    blocks,
                )),
                constraint!(== available.list.len() as u16),
            )
        } else {
            let text = "There are no connections";
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

        let [c_area, a_area] =
            Layout::vertical([connected_constraint, available_constraint]).areas(frame.area());

        frame.render_widget(connected_widget, c_area);
        frame.render_widget(available_widget, a_area);
    }
}
