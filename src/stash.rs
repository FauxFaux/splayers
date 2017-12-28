use std::fmt;
use std::fs;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use tempdir::TempDir;

#[derive(Debug, Clone, Copy)]
pub struct Stashed {
    idx: usize,
}

#[derive(Debug)]
pub struct Stash {
    dir: TempDir,
    len: usize,
}

impl Stash {
    pub fn new() -> io::Result<Self> {
        Ok(Stash {
            dir: TempDir::new(".splayed")?,
            len: 0,
        })
    }

    pub fn insert<R: Read>(&mut self, mut from: R) -> io::Result<Stashed> {
        let item = Stashed { idx: self.len };
        self.len += 1;

        let mut tmp = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(self.path_of(item))?;
        loop {
            let mut buf = [0u8; 8 * 1024];
            let found = from.read(&mut buf)?;
            if 0 == found {
                break;
            }
            tmp.write_all(&buf[..found]).expect("writing to temp file");
        }

        Ok(item)
    }

    pub fn push_take<R: Read>(&mut self, from: R, size: u64) -> io::Result<Option<Stashed>> {
        Ok(if 0 == size {
            None
        } else {
            Some(self.insert(from.take(size))?)
        })
    }

    pub fn path_of(&self, item: Stashed) -> PathBuf {
        let mut dest = self.dir.as_ref().to_path_buf();
        dest.push(format!(".{}.tmp", item.idx));
        dest
    }
}
