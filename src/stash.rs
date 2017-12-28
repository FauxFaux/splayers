use std::fs;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use tempdir::TempDir;
use tempfile_fast::persistable_tempfile_in;

use errors::*;
use mio::Mio;

#[derive(Debug, Copy, Clone)]
pub struct Stashed {
    idx: u64,
}

#[derive(Debug)]
pub struct Stash {
    dir: TempDir,
    idx: u64,
}

impl Stash {
    pub fn new() -> io::Result<Self> {
        Ok(Stash {
            dir: TempDir::new("splayers")?,
            idx: 0,
        })
    }

    pub fn stash<R: Read>(&mut self, mut from: R) -> io::Result<Stashed> {
        let mut tmp = persistable_tempfile_in(&self.dir).expect("creating temp file");
        loop {
            let mut buf = [0u8; 8 * 1024];
            let found = from.read(&mut buf)?;
            if 0 == found {
                break;
            }
            tmp.write_all(&buf[..found]).expect("writing to temp file");
        }

        self.idx += 1;
        let stashed = Stashed { idx: self.idx };
        tmp.persist_noclobber(self.path_of(stashed))
            .expect("persisting temp file");
        Ok(stashed)
    }

    pub fn stash_take<R: Read>(&mut self, mut from: R, size: u64) -> io::Result<Option<Stashed>> {
        Ok(if 0 == size {
            None
        } else {
            Some(self.stash(from.take(size))?)
        })
    }

    fn path_of(&self, item: Stashed) -> PathBuf {
        let mut dest = self.dir.as_ref().to_path_buf();
        dest.push(format!("{}.tmp", item.idx));
        dest
    }

    pub fn open(&self, item: Stashed) -> Mio {
        assert!(item.idx <= self.idx, "can't be a valid idx");
        Mio::from_path(self.path_of(item)).expect("working with stash")
    }

    pub fn release(&self, item: Stashed) {
       fs::remove_file(self.path_of(item)).expect("unlinking temp file")
    }
}
