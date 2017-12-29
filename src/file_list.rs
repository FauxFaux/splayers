use std::fs;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use tempdir::TempDir;

#[derive(Debug, Clone, Copy)]
pub struct Id {
    idx: usize,
}

#[derive(Debug)]
pub struct FileList {
    dir: TempDir,
    len: usize,
}

impl FileList {
    pub fn new() -> io::Result<Self> {
        Ok(FileList {
            dir: TempDir::new(".splayed")?,
            len: 0,
        })
    }

    pub fn insert<R: Read>(&mut self, mut from: R) -> io::Result<Id> {
        let item = Id { idx: self.len };
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

    pub fn path_of(&self, item: Id) -> PathBuf {
        let mut dest = self.dir.as_ref().to_path_buf();
        dest.push(format!(".{}.tmp", item.idx));
        dest
    }
}
