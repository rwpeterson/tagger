#[allow(unused_imports)]
use anyhow::{bail, ensure, Context, Result};
use flume::RecvTimeoutError;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::path;
use std::sync::Arc;
use std::time::Duration;
use tagtools::cfg::Single;

use tagtools::{bit, cfg, Tag, THRESHOLD_MAX, THRESHOLD_MIN};

use crate::client::{ClientHandle, ClientMessage};
use crate::save;
use crate::save::SaveHandle;
use crate::settings_client::{
    RawChannelSetting, RawSingleChannelState, SettingsClientHandle, SettingsMessage,
};

#[allow(unused_imports)]
use tui::{
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans},
    widgets::ListState,
};

pub enum Event<I> {
    Input(I),
    Tick,
}

pub struct TabsState<'a> {
    pub titles: Vec<&'a str>,
    pub index: usize,
}

impl<'a> TabsState<'a> {
    pub fn new(titles: Vec<&'a str>) -> TabsState {
        TabsState { titles, index: 0 }
    }
    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.titles.len();
    }
    pub fn prev(&mut self) {
        self.index = self.index.checked_sub(1).unwrap_or(self.titles.len() - 1);
    }
}

#[derive(PartialEq)]
pub enum SettingsMode {
    ChannelSelect,
    SettingSelect,
    Invert(Option<bool>),
    Delay(Option<u32>),
    Threshold(Option<f64>),
}

pub enum Grain {
    Coarse,
    Medium,
    Fine,
}

pub struct SettingsState {
    pub index: usize,
    pub st_index: usize,
    pub channel_settings: Vec<RawSingleChannelState>,
    pub mode: SettingsMode,
    pub grain: Grain,
    pub ch_state: ListState,
    pub st_state: ListState,
}

impl SettingsState {
    pub fn next(&mut self) {
        let len = self.channel_settings.len();
        self.index = (self.index + 1) % len;
    }
    pub fn prev(&mut self) {
        let len = self.channel_settings.len();
        self.index = self.index.checked_sub(1).unwrap_or(len - 1);
    }
    pub fn next_set(&mut self) {
        let len = 3;
        self.st_index = (self.st_index + 1) % len;
    }
    pub fn prev_set(&mut self) {
        let len = 3;
        self.st_index = self.st_index.checked_sub(1).unwrap_or(len - 1);
    }
    /*pub fn next_set(&mut self) {
        match self.mode {
            SettingsMode::Delay(_) => {
                self.mode = SettingsMode::Threshold(None);
            }
            SettingsMode::Threshold(_) => {
                self.mode = SettingsMode::Invert(None);
            }
            SettingsMode::Invert(_) => {
                self.mode = SettingsMode::Delay(None);
            }
            _ => {}
        }
    }
    pub fn prev_set(&mut self) {
        match self.mode {
            SettingsMode::Delay(_) => {
                self.mode = SettingsMode::Invert(None);
            }
            SettingsMode::Threshold(_) => {
                self.mode = SettingsMode::Delay(None);
            }
            SettingsMode::Invert(_) => {
                self.mode = SettingsMode::Threshold(None);
            }
            _ => {}
        }
    }*/
    pub fn finer(&mut self) {
        match self.grain {
            Grain::Coarse => self.grain = Grain::Medium,
            Grain::Medium => self.grain = Grain::Fine,
            Grain::Fine => {}
        }
    }
    pub fn coarser(&mut self) {
        match self.grain {
            Grain::Coarse => {}
            Grain::Medium => self.grain = Grain::Coarse,
            Grain::Fine => self.grain = Grain::Medium,
        }
    }
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
    pub hist_len: usize,
    pub singles: Vec<HashMap<u8, f64>>,
    pub coincs: Vec<HashMap<(u8, u8), f64>>,
    pub error_text: Option<anyhow::Error>,
    pub client_handle: ClientHandle,
    pub settings_handle: SettingsClientHandle,
    pub save_handle: SaveHandle,
    pub tabs: TabsState<'a>,
    pub live_settings: bool,
    pub settings_state: Option<SettingsState>,
    pub config: cfg::Run,
}

impl<'a> App<'a> {
    pub fn new(
        title: &'a str,
        enhanced_graphics: bool,
        client_handle: ClientHandle,
        settings_handle: SettingsClientHandle,
        save_handle: SaveHandle,
        config: cfg::Run,
    ) -> App<'a> {
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
            hist_len: 80,
            singles: Vec::new(),
            coincs: Vec::new(),
            error_text: None,
            client_handle,
            settings_handle,
            save_handle,
            tabs: TabsState::new(vec!["Count Monitor", "Input Settings"]),
            live_settings: false,
            settings_state: None,
            config,
        }
    }

    pub fn on_key(&mut self, c: char) {
        match c {
            'c' => self.flags.clear(),
            'w' => {
                if self.tabs.index == 1 && self.live_settings {
                    let state = self.settings_state.as_mut().unwrap();
                    match state.mode {
                        SettingsMode::ChannelSelect => {
                            state.prev();
                            state.ch_state.select(Some(state.index));
                        }
                        SettingsMode::SettingSelect => {
                            state.prev_set();
                            state.st_state.select(Some(state.st_index));
                        }
                        SettingsMode::Delay(None) => {
                            state.mode =
                                SettingsMode::Delay(Some(state.channel_settings[state.index].del));
                        }
                        SettingsMode::Delay(Some(ref mut del)) => {
                            *del += match state.grain {
                                Grain::Coarse => 100,
                                Grain::Medium => 10,
                                Grain::Fine => 1,
                            };
                        }
                        SettingsMode::Threshold(None) => {
                            state.mode = SettingsMode::Threshold(Some(
                                state.channel_settings[state.index].thr,
                            ));
                        }
                        SettingsMode::Threshold(Some(ref mut thr)) => {
                            *thr += match state.grain {
                                Grain::Coarse => 0.1,
                                Grain::Medium => 0.01,
                                Grain::Fine => 0.001,
                            };
                            if *thr > THRESHOLD_MAX {
                                *thr = THRESHOLD_MAX;
                            }
                        }
                        SettingsMode::Invert(None) => {
                            state.mode =
                                SettingsMode::Invert(Some(state.channel_settings[state.index].inv));
                        }
                        SettingsMode::Invert(Some(ref mut inv)) => {
                            *inv = !*inv;
                        }
                    }
                }
            }
            's' => {
                if self.tabs.index == 1 && self.live_settings {
                    let state = self.settings_state.as_mut().unwrap();
                    match state.mode {
                        SettingsMode::ChannelSelect => {
                            state.next();
                            state.ch_state.select(Some(state.index));
                        }
                        SettingsMode::SettingSelect => {
                            state.next_set();
                            state.st_state.select(Some(state.st_index));
                        }
                        SettingsMode::Delay(None) => {
                            state.mode =
                                SettingsMode::Delay(Some(state.channel_settings[state.index].del));
                        }
                        SettingsMode::Delay(Some(ref mut del)) => {
                            *del = del
                                .checked_sub(match state.grain {
                                    Grain::Coarse => 100,
                                    Grain::Medium => 10,
                                    Grain::Fine => 1,
                                })
                                .unwrap_or_default();
                        }
                        SettingsMode::Threshold(None) => {
                            state.mode = SettingsMode::Threshold(Some(
                                state.channel_settings[state.index].thr,
                            ));
                        }
                        SettingsMode::Threshold(Some(ref mut thr)) => {
                            *thr -= match state.grain {
                                Grain::Coarse => 0.1,
                                Grain::Medium => 0.01,
                                Grain::Fine => 0.001,
                            };
                            if *thr < THRESHOLD_MIN {
                                *thr = THRESHOLD_MIN;
                            }
                        }
                        SettingsMode::Invert(None) => {
                            state.mode =
                                SettingsMode::Invert(Some(state.channel_settings[state.index].inv));
                        }
                        SettingsMode::Invert(Some(ref mut inv)) => {
                            *inv = !*inv;
                        }
                    }
                }
            }
            'a' => {
                if self.tabs.index == 1 && self.live_settings {
                    let state = self.settings_state.as_mut().unwrap();
                    match state.mode {
                        SettingsMode::Delay(_)
                        | SettingsMode::Threshold(_)
                        | SettingsMode::Invert(_) => {
                            state.mode = SettingsMode::SettingSelect;
                        }
                        SettingsMode::SettingSelect => {
                            state.mode = SettingsMode::ChannelSelect;
                        }
                        _ => {}
                    }
                }
            }
            'd' => {
                if self.tabs.index == 1 && self.live_settings {
                    let state = self.settings_state.as_mut().unwrap();
                    match state.mode {
                        SettingsMode::ChannelSelect => {
                            state.mode = SettingsMode::SettingSelect;
                        }
                        SettingsMode::SettingSelect => {
                            state.mode = match state.st_index {
                                0 => SettingsMode::Delay(None),
                                1 => SettingsMode::Threshold(None),
                                2 => SettingsMode::Invert(None),
                                _ => SettingsMode::SettingSelect,
                            };
                        }
                        _ => {}
                    }
                }
            }
            'e' => {
                if self.tabs.index == 1 && self.live_settings {
                    let state = self.settings_state.as_mut().unwrap();
                    match state.mode {
                        SettingsMode::Delay(_) | SettingsMode::Threshold(_) => {
                            state.finer();
                        }
                        _ => {}
                    }
                }
            }
            'q' => {
                if self.tabs.index == 1 && self.live_settings {
                    let state = self.settings_state.as_mut().unwrap();
                    match state.mode {
                        SettingsMode::Delay(_) | SettingsMode::Threshold(_) => {
                            state.coarser();
                        }
                        _ => {}
                    }
                }
            }
            ' ' => {
                if self.tabs.index == 1 && self.live_settings {
                    let state = self.settings_state.as_mut().unwrap();
                    let index = state.index;
                    let ch = state.channel_settings[index].ch;
                    match state.mode {
                        SettingsMode::Delay(Some(del)) => {
                            let (respond_to, response) = flume::bounded(1);
                            let _ = self.settings_handle.sender.send(SettingsMessage::Set {
                                setting: RawChannelSetting::Delay((ch, del)),
                                respond_to,
                            });
                            match response.recv_timeout(Duration::from_millis(1000)) {
                                Ok(()) => {
                                    state.mode = SettingsMode::Delay(None);
                                    state.channel_settings[index].del = del;
                                }
                                Err(RecvTimeoutError::Timeout) => {
                                    self.flags.insert(String::from("Set delay timeout"));
                                }
                                Err(RecvTimeoutError::Disconnected) => {
                                    self.should_quit = true;
                                }
                            }
                        }
                        SettingsMode::Threshold(Some(thr)) => {
                            let (respond_to, response) = flume::bounded(1);
                            let _ = self.settings_handle.sender.send(SettingsMessage::Set {
                                setting: RawChannelSetting::Threshold((ch, thr)),
                                respond_to,
                            });
                            match response.recv_timeout(Duration::from_millis(1000)) {
                                Ok(()) => {
                                    state.mode = SettingsMode::Threshold(None);
                                    state.channel_settings[index].thr = thr;
                                }
                                Err(RecvTimeoutError::Timeout) => {
                                    self.flags.insert(String::from("Set threshold timeout"));
                                }
                                Err(RecvTimeoutError::Disconnected) => {
                                    self.should_quit = true;
                                }
                            }
                        }
                        SettingsMode::Invert(Some(inv)) => {
                            let (respond_to, response) = flume::bounded(1);
                            let _ = self.settings_handle.sender.send(SettingsMessage::Set {
                                setting: RawChannelSetting::Inversion((ch, inv)),
                                respond_to,
                            });
                            match response.recv_timeout(Duration::from_millis(1000)) {
                                Ok(()) => {
                                    state.mode = SettingsMode::Invert(None);
                                    state.channel_settings[index].inv = inv;
                                }
                                Err(RecvTimeoutError::Timeout) => {
                                    self.flags.insert(String::from("Set inversion timeout"));
                                }
                                Err(RecvTimeoutError::Disconnected) => {
                                    self.should_quit = true;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            'r' => match self.save {
                true => {
                    self.save = false;
                    self.save_handle
                        .sender
                        .send(save::SaveMessage::Reset)
                        .unwrap();
                }
                false => {
                    self.save = true;
                }
            },
            'x' => {
                if self.tabs.index == 1 && self.live_settings == false {
                    self.live_settings = true;
                    let mut channel_settings = Vec::new();
                    let (respond_to, response) = flume::bounded(1);
                    let _ = self
                        .settings_handle
                        .sender
                        .send(SettingsMessage::Get { respond_to });
                    match response.recv_timeout(Duration::from_secs(1)) {
                        Ok(tagger_state) => {
                            for s in &self.config.singles {
                                if let Single::Channel(ch) = s {
                                    channel_settings.push(RawSingleChannelState {
                                        ch: *ch,
                                        inv: bit::checkbit16(tagger_state.invm, ch - 1),
                                        del: tagger_state.dels[(*ch - 1) as usize],
                                        thr: tagger_state.thrs[(*ch - 1) as usize],
                                    });
                                }
                            }
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            self.flags
                                .insert(String::from("Load settings timeout: please restart"));
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            self.should_quit = true;
                        }
                    }
                    self.settings_state = Some(SettingsState {
                        index: 0,
                        st_index: 0,
                        channel_settings,
                        mode: SettingsMode::ChannelSelect,
                        grain: Grain::Fine,
                        ch_state: ListState::default(),
                        st_state: ListState::default(),
                    });
                }
            }
            'm' => {
                if self.tabs.index == 1 && self.live_settings == false {
                    self.live_settings = true;
                    let mut channel_settings = Vec::new();
                    for cs in &self.config.channel_settings {
                        let ch = cs.channel;
                        if let Some(inv) = cs.invert {
                            let (respond_to, response) = flume::bounded(1);
                            let _ = self.settings_handle.sender.send(SettingsMessage::Set {
                                setting: RawChannelSetting::Inversion((ch, inv)),
                                respond_to,
                            });
                            response.recv().unwrap();
                        }
                        if let Some(del) = cs.delay {
                            let (respond_to, response) = flume::bounded(1);
                            let _ = self.settings_handle.sender.send(SettingsMessage::Set {
                                setting: RawChannelSetting::Delay((ch, del)),
                                respond_to,
                            });
                            response.recv().unwrap();
                        }
                        if let Some(thr) = cs.threshold {
                            let (respond_to, response) = flume::bounded(1);
                            let _ = self.settings_handle.sender.send(SettingsMessage::Set {
                                setting: RawChannelSetting::Threshold((ch, thr)),
                                respond_to,
                            });
                            response.recv().unwrap();
                        }
                    }
                    // Now read back everything from the tagger to populate channel_settings
                    let (respond_to, response) = flume::bounded(1);
                    let _ = self
                        .settings_handle
                        .sender
                        .send(SettingsMessage::Get { respond_to });
                    match response.recv_timeout(Duration::from_secs(1)) {
                        Ok(tagger_state) => {
                            for cs in &self.config.channel_settings {
                                let ch = cs.channel;
                                channel_settings.push(RawSingleChannelState {
                                    ch,
                                    inv: bit::checkbit16(tagger_state.invm, ch - 1),
                                    del: tagger_state.dels[(ch - 1) as usize],
                                    thr: tagger_state.thrs[(ch - 1) as usize],
                                });
                            }
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            self.flags
                                .insert(String::from("Load settings timeout: please restart"));
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            self.should_quit = true;
                        }
                    }
                    self.settings_state = Some(SettingsState {
                        index: 0,
                        st_index: 0,
                        channel_settings,
                        mode: SettingsMode::ChannelSelect,
                        grain: Grain::Fine,
                        ch_state: ListState::default(),
                        st_state: ListState::default(),
                    });
                }
            }
            _ => {}
        }
    }

    pub fn on_left(&mut self) {
        self.tabs.prev();
    }

    pub fn on_right(&mut self) {
        self.tabs.next();
    }

    pub fn on_tick(&mut self) {
        let (respond_to, response) = flume::bounded(1);
        let _ = self
            .client_handle
            .sender
            .send(ClientMessage::GetData { respond_to });

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

        // Save data to disk
        if self.save == true {
            match self
                .save_handle
                .sender
                .send(save::SaveMessage::Save(save::SaveTags {
                    tags: self.tags.clone(),
                    path: self.filepath.clone(),
                })) {
                Ok(()) => {}
                Err(e) => {
                    self.flags.insert(format! {"{:#}", e });
                }
            }
        } else {
            //self.flags.insert(String::from("Not saving right now"));
        }
    }
}
