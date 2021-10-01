use anyhow::{bail, Context, Result};
use chrono::Utc;
use parking_lot::Mutex;
use std::env;
use std::fs;
use std::path;
use std::sync::Arc;
use std::thread;
use tagtools::{Tag, ser};

pub struct SaveTags {
    pub tags: Arc<Mutex<Vec<Tag>>>,
    pub path: Option<path::PathBuf>,
}

pub enum SaveMessage {
    Save(SaveTags),
    Reset,
}

pub struct SaveHandle {
    pub sender: flume::Sender<SaveMessage>,
}

impl SaveHandle {
    pub fn new() -> Self {
        let (sender, receiver) = flume::unbounded();
        
    thread::spawn(move || {
        let mut curpath: Option<path::PathBuf> = None;
        let mut file: Option<fs::File> = None;
        loop {
            if let Ok(st) = receiver.recv() {
                match st {
                    SaveMessage::Save(st) => {
                        let tags = st.tags.lock();
                        let t = &*tags;
                        update_and_write_file(st.path, &mut curpath, &mut file, &t)
                            .context("file io error")
                            .unwrap();
                    },
                    SaveMessage::Reset => {
                        drop(file);
                        file = None;
                    },
                }

            }
        }
    });
    SaveHandle { sender }
    }
}

fn update_and_write_file(
    newpath: Option<path::PathBuf>,
    curpath: &mut Option<path::PathBuf>,
    f: &mut Option<fs::File>,
    tags: &[Tag],
) -> Result<()>
{
    match f {
        Some(_) => {
            if newpath != *curpath {
                update_file(f, newpath)?;
            }
        },
        None => {
            update_file(f, newpath)?;
        },
    }
    ser::tags(f.as_mut().unwrap(), &tags)?;
    Ok(())
}

fn update_file(f: &mut Option<fs::File>, newpath: Option<path::PathBuf>) -> Result<()> {
    let mut path: path::PathBuf;
    match newpath {
        Some(p) => {
            path = p;
        },
        None => {
            path = env::current_dir()?;
            path.push(Utc::now().format("%F-%H-%M-%S").to_string());
            path.set_extension("tags");
        }
    }
    if path.exists() {
        bail!("file already exists");
    } else {
        *f = Some(fs::File::create(path)?);
    }
    Ok(())
}