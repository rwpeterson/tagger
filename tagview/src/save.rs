use anyhow::{bail, Context, Result};
use chrono::Utc;
use std::env;
use std::fs;
use std::path;
use std::sync::Arc;
use std::thread;
use tagtools::{Tag, ser};

pub struct SaveTags {
    pub tags: Arc<Vec<Tag>>,
    pub path: Option<path::PathBuf>,
}

pub enum SaveMessage {
    Save(SaveTags),
    Reset,
}

pub fn main(rx: flume::Receiver::<SaveMessage>) {
    thread::spawn(move || {
        let mut curpath: Option<path::PathBuf> = None;
        let mut file: Option<fs::File> = None;
        loop {
            if let Ok(st) = rx.recv() {
                match st {
                    SaveMessage::Save(st) => {
                        update_and_write_file(st.path, &mut curpath, &mut file, &st.tags)
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