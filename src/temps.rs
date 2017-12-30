use std::fs;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use tempdir::TempDir;

#[derive(Debug)]
pub struct Temps {
    dir: TempDir,
    count: usize,
}

impl Temps {
    pub fn new_in<P: AsRef<Path>>(inside: P) -> io::Result<Self> {
        Ok(Temps {
            dir: TempDir::new_in(inside, ".splayers")?,
            count: 0,
        })
    }

    pub fn insert<R: Read>(&mut self, mut from: R) -> io::Result<PathBuf> {
        let mut dest = self.dir.as_ref().to_path_buf();
        dest.push(format!(".{}.tmp", self.count));

        self.count += 1;

        let mut tmp = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&dest)?;
        loop {
            let mut buf = [0u8; 8 * 1024];
            let found = from.read(&mut buf)?;
            if 0 == found {
                break;
            }
            tmp.write_all(&buf[..found]).expect("writing to temp file");
        }

        Ok(dest)
    }
}
