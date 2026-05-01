use std::{
    path::PathBuf,
    sync::{Arc, mpsc::Sender},
};

use enum_dispatch::enum_dispatch;
use ratatui::{Frame, crossterm};

use crate::application::home::{available::AvailableAPDetails, connected::ConnectionData};

/// Read only context, shared across the application
pub struct Context {
    #[allow(unused)]
    pub log_file: PathBuf,
    pub fps: u32,
    pub connection_maxitem: usize,
}

pub struct RichContext {
    #[allow(unused)]
    pub log_file: PathBuf,
    pub fps: u32,
    #[allow(unused)]
    pub message: Arc<Sender<Message>>,
    pub connection_maxitem: usize,
}

pub struct Connected {
    pub list: Vec<ConnectionData>,
}

pub struct Available {
    pub list: Vec<AvailableAPDetails>,
}

pub enum Message {
    Crossterm(crossterm::event::Event),
    GlobalTick,

    // Home pane events
    LoadConnected(Connected),
    LoadAvailable(Available),
    FinishLoading,
}

#[enum_dispatch]
pub trait Component {
    fn update(&mut self, ctx: &RichContext, ev: Message);
    fn draw(&self, ctx: &RichContext, frame: &mut Frame<'_>);
}
