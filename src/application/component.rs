use std::{
    path::PathBuf,
    sync::{Arc, mpsc::Sender},
};

use enum_dispatch::enum_dispatch;
use ratatui::{Frame, crossterm};

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

pub enum Message {
    Crossterm(crossterm::event::Event),
    GlobalTick,
}

#[enum_dispatch]
pub trait Component {
    fn update(&mut self, ctx: &RichContext, ev: Message);
    fn draw(&self, ctx: &RichContext, frame: &mut Frame<'_>);
}
