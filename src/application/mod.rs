use std::{sync::Arc, time::Duration};

use enum_dispatch::enum_dispatch;
use ratatui::{
    Frame,
    crossterm::{self},
};

use crate::application::{
    component::{Component, Message, RichContext},
    home::HomeData,
};

pub mod component;
mod home;
pub mod utils;

pub use component::Context;

pub struct Application {
    context: Arc<RichContext>,
    #[allow(unused)]
    pane: Pane,
    message_tx: Arc<std::sync::mpsc::Sender<Message>>,
    message_rx: Option<std::sync::mpsc::Receiver<Message>>,
}

impl Application {
    pub fn new(Context { log_file, fps }: Context) -> Self {
        let (message_tx, message_rx) = std::sync::mpsc::channel();
        let message_tx = Arc::new(message_tx);

        let context = RichContext {
            log_file,
            fps,
            message: message_tx.clone(),
        };

        Self {
            pane: Pane::Home(HomeData::new(&context)),
            context: context.into(),
            message_tx,
            message_rx: Some(message_rx),
        }
    }

    pub fn setup_event_polling(&self) -> anyhow::Result<()> {
        let crossterm_tx = self.message_tx.clone();
        std::thread::spawn(move || {
            loop {
                // TODO: catch this error and only panic if N consecutive calls return an error
                // also write an error! log on error
                let ev = crossterm::event::read().unwrap();

                crossterm_tx.send(Message::Crossterm(ev)).unwrap();
            }
        });

        let timer_tx = self.message_tx.clone();
        let interval = Duration::from_secs(1).div_f64(self.context.fps as _);
        std::thread::spawn(move || {
            loop {
                timer_tx.send(Message::GlobalTick).unwrap();
                std::thread::sleep(interval);
            }
        });

        Ok(())
    }

    /// runs the main loop for the application
    pub fn run(mut self) -> anyhow::Result<()> {
        self.setup_event_polling()?;
        let rx = self.message_rx.take().unwrap();

        ratatui::run(|term| {
            for msg in rx {
                term.draw(|frame| {
                    self.draw(frame);
                })?;

                if let Message::Crossterm(crossterm::event::Event::Key(key)) = msg
                    && key.code.is_char('q')
                {
                    break;
                }

                self.update(msg);
            }

            anyhow::Ok(())
        })?;

        Ok(())
    }

    pub fn update(&mut self, ev: Message) {
        self.pane.update(&self.context, ev);
    }

    pub fn draw(&self, frame: &mut Frame<'_>) {
        self.pane.draw(&self.context, frame);
    }
}

#[enum_dispatch(Component)]
pub enum Pane {
    Home(HomeData),
}
