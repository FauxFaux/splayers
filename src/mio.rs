use std::io;
use std::io::BufRead;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use errors::*;

/// Same as `io::DEFAULT_BUF_SIZE`.
const CAP: usize = 8 * 1024;
const HEADER_CAP: usize = 1024;

pub struct Mio {
    inner: io::BufReader<fs::File>,
    path: PathBuf,
}

impl Mio {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Mio> {
        Ok(Mio {
            path: path.as_ref().to_path_buf(),
            inner: io::BufReader::with_capacity(CAP, fs::File::open(path)?),
        })
    }

    // should return a slice but the BORROW CHECKER is actually dumb (I'm 99% sure)
    pub fn header(&mut self) -> Vec<u8> {
        fill_buf(&mut self.inner)
    }
}

impl Clone for Mio {
    fn clone(&self) -> Self {
        Mio::from_path(&self.path).expect("mio: clone")
    }
}

pub fn fill_buf<R: BufRead>(mut of: R) -> Vec<u8> {
    let mut last_attempt = 0;
    loop {
        let buf = of.fill_buf().expect("mio: filling");
        debug_assert_lt!(HEADER_CAP, CAP);
        if buf.len() > HEADER_CAP || buf.len() == last_attempt {
            return buf.to_vec();
        }
        last_attempt = buf.len();
    }
}

impl io::Read for Mio {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.inner.read(buf) {
            Ok(len) => Ok(len),
            Err(e) => if e.kind() == io::ErrorKind::UnexpectedEof {
                Err(e)
            } else {
                panic!("unexpected io error from filesystem: {:?}", e)
            },
        }
    }
}

/// I don't think seek really fails because the filesystem is broken
impl io::Seek for Mio {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}
