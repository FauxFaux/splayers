#[macro_use]
extern crate failure;

extern crate time as crates_time;

#[macro_use]
extern crate more_asserts;

use std::path::Path;
use std::path::PathBuf;

use failure::Error;

mod file_type;
mod fill_read;
mod meta;
mod mio;
mod simple_time;
mod temps;
mod unpacker;

pub use crate::meta::ItemType;
pub use crate::unpacker::Entry;
pub use crate::unpacker::Status;

pub struct Unpack {
    status: Status,
    dir: tempfile::TempDir,
}

impl Unpack {
    pub fn unpack_into<P: AsRef<Path>, F: AsRef<Path>>(what: F, root: P) -> Result<Unpack, Error> {
        let mut temps = temps::Temps::new_in(root)?;
        Ok(Unpack {
            status: unpacker::unpack_root(what, &mut temps)?,
            dir: temps.into_dir(),
        })
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    /// causes the temporary files to not be deleted
    pub fn into_path(self) -> PathBuf {
        self.dir.into_path()
    }
}

pub fn print(entries: &[Entry], depth: usize) {
    for entry in entries {
        print!(
            "{} - {:?} at {:?}:",
            String::from_utf8_lossy(&vec![b' '; depth]),
            String::from_utf8_lossy(&entry.local.path),
            entry.local.temp
        );

        if let Status::Success(ref children) = entry.children {
            println!();
            print(children, depth + 2);
        } else {
            println!(" {:?}", entry.children);
        }
    }
}
