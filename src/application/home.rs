use std::sync::{Arc, Mutex, atomic::AtomicBool};

use poppingboba::spinner::{Spinner, SpinnerType};
use ratatui::{
    layout::Layout,
    macros::{constraint, constraints, line},
};

use crate::application::{Context, component::RichContext};

use super::component::{Component, Message};

pub struct HomeData {
    loading: Arc<Mutex<Option<Spinner>>>,
    scanning: AtomicBool,
    connected: Arc<Mutex<Connected>>,
    available: Arc<Mutex<Available>>,
    selected: Selected,
}

struct Connected {}
struct Available {}

enum Selected {
    Connected(usize),
    Available(usize),
    None,
}

impl HomeData {
    pub fn new(ctx: &RichContext) -> Self {
        // run async task to load connections and update state accordingly
        let loading = Spinner::new(SpinnerType::dot(), ctx.fps);
        let loading = Arc::new(Mutex::new(Some(loading)));
        let scanning = AtomicBool::new(false);
        let connected = Arc::new(Mutex::new(Connected {}));
        let available = Arc::new(Mutex::new(Available {}));
        let selected = Selected::None;

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
    }
}
