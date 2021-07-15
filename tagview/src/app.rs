
use crate::save;

use tagtools::Tag;

#[allow(unused_imports)]
use anyhow::{bail, ensure, Result, Context};
use std::collections::{HashMap, HashSet};
use std::path;
use std::sync::Arc;
use std::thread;
use std::time::Instant;


#[allow(unused_imports)]
use tui::{
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans},
};

pub enum Event<I> {
    Input(I),
    Tick,
}

pub struct App<'a> {
    pub title: &'a str,
    pub enhanced_graphics: bool,
    pub should_quit: bool,
    pub tags: Arc<Vec<Tag>>,
    pub save: bool,
    pub filepath: Option<path::PathBuf>,
    pub flags: HashSet<String>,
    pub tag_rate: f64,
    pub data_size: usize,
    pub hist_len: usize,
    pub singles: Vec<HashMap<u8, f64>>,
    pub coincs: Vec<HashMap<(u8, u8), f64>>,
    pub last_read: Instant,
    pub error_text: Option<anyhow::Error>,
    pub tx_io: flume::Sender<save::SaveMessage>,
}

impl<'a> App<'a> {
    pub fn new(
        title: &'a str,
        enhanced_graphics: bool,
        tx_io: flume::Sender<save::SaveMessage>,
        tx_client: flume::Sender<()>,
    ) -> App<'a>
    {
        App {
            title,
            enhanced_graphics,
            should_quit: false,
            tags: Arc::new(Vec::new()),
            save: false,
            filepath: None,
            flags: HashSet::new(),
            tag_rate: 0.0,
            data_size: 0,
            hist_len: 80,
            singles: Vec::new(),
            coincs: Vec::new(),
            last_read: Instant::now(),
            error_text: None,
            tx_io,
        }
    }

    pub fn on_key(&mut self, c: char) {
        match c {
            'q' => self.should_quit = true,
            'c' => self.flags.clear(),
            's' => {
                match self.save {
                    true => {
                        self.save = false;
                        self.tx_io.send(save::SaveMessage::Reset).unwrap();
                    },
                    false => {
                        self.save = true;
                    },
                }
            }
            _   => {},
        }
    }

    pub fn on_tick(&mut self) {
        self.tags = Arc::new(
            vec![Tag { time: 1, channel: 1 }]
        );
        
        let time = self.last_read.elapsed().as_secs_f64();
        self.last_read = Instant::now();

        // Save data to disk
        if self.save == true {
            match self.tx_io.try_send(
                save::SaveMessage::Save(
                    save::SaveTags { tags: self.tags.clone(), path: self.filepath.clone() }
                )
            )
            {
                Ok(()) => {},
                Err(e) => {
                    self.flags.insert(format!{"{:#}", e });
                },
            }
        } else {
            //self.flags.insert(String::from("Not saving right now"));
        }
    }
}