use tagtools::Tag;

#[allow(unused_imports)]
use anyhow::{bail, ensure, Result, Context};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::path;
use std::sync::Arc;

use crate::save;
use crate::client::{ClientHandle, ClientMessage};
use crate::save::SaveHandle;

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
    pub tags: Arc<Mutex<Vec<Tag>>>,
    pub pats: Arc<Mutex<HashMap<u16, u64>>>,
    pub duration: u64,
    pub save: bool,
    pub filepath: Option<path::PathBuf>,
    pub flags: HashSet<String>,
    pub tag_rate: usize,
    pub data_size: usize,
    pub hist_len: usize,
    pub singles: Vec<HashMap<u8, f64>>,
    pub coincs: Vec<HashMap<(u8, u8), f64>>,
    pub error_text: Option<anyhow::Error>,
    pub client_handle: ClientHandle,
    pub save_handle: SaveHandle,
}

impl<'a> App<'a> {
    pub fn new(
        title: &'a str,
        enhanced_graphics: bool,
        client_handle: ClientHandle,
        save_handle: SaveHandle,
    ) -> App<'a>
    {
        App {
            title,
            enhanced_graphics,
            should_quit: false,
            tags: Arc::new(Mutex::new(Vec::new())),
            pats: Arc::new(Mutex::new(HashMap::new())),
            duration: 0,
            save: false,
            filepath: None,
            flags: HashSet::new(),
            tag_rate: 0,
            data_size: 0,
            hist_len: 80,
            singles: Vec::new(),
            coincs: Vec::new(),
            error_text: None,
            client_handle,
            save_handle,
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
                        self.save_handle.sender.send(save::SaveMessage::Reset).unwrap();
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
        let (respond_to, response) = flume::bounded(1);
        let _ = self.client_handle.sender.send(ClientMessage::GetData { respond_to });

        let mut tags = self.tags.lock();
        let mut pats = self.pats.lock();
        let newdata = response.recv().unwrap();
        self.duration = 0;
        (*tags).clear();
        (*pats).clear();
        if let Some(data) = newdata {
            for mut chunk in data {
                self.duration += chunk.tagpat.duration;
                (*tags).append(&mut chunk.tagpat.tags);
                for lpat in chunk.pats {
                    if let None = (*pats).get(&lpat.patmask) {
                        let _ = (*pats).insert(lpat.patmask, 0);
                    }
                    if let Some(v) = (*pats).get_mut(&lpat.patmask) {
                        *v += lpat.count;
                    }
                }
            }
        }
        self.tag_rate = tags.len();

        // Save data to disk
        if self.save == true {
            match self.save_handle.sender.send(
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