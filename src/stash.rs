use std::fmt;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use tempfile::NamedTempFile;

pub struct Stashed {
    inner: NamedTempFile,
}

#[derive(Debug)]
pub struct Stash {
    dir: PathBuf,
}

impl Stashed {
    pub fn path(&self) -> &Path {
        self.inner.path()
    }
}

impl fmt::Debug for Stashed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Stashed {{ {:?} }}", self.path())
    }
}

impl Stash {
    pub fn new() -> io::Result<Self> {
        Ok(Stash { dir: "/tmp".into() })
    }

    pub fn stash<R: Read>(&mut self, mut from: R) -> io::Result<Stashed> {
        let mut tmp = NamedTempFile::new_in(&self.dir).expect("creating temp file");
        loop {
            let mut buf = [0u8; 8 * 1024];
            let found = from.read(&mut buf)?;
            if 0 == found {
                break;
            }
            tmp.write_all(&buf[..found]).expect("writing to temp file");
        }

        Ok(Stashed { inner: tmp })
    }

    pub fn stash_take<R: Read>(&mut self, from: R, size: u64) -> io::Result<Option<Stashed>> {
        Ok(if 0 == size {
            None
        } else {
            Some(self.stash(from.take(size))?)
        })
    }
}
