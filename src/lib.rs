extern crate ar;
extern crate bzip2;

#[macro_use]
extern crate error_chain;

#[cfg(intellij_type_hinting)]
extern crate error_chain_for_dumb_ides;

extern crate flate2;
extern crate tar;
extern crate tempdir;
extern crate time as crates_time;

#[macro_use]
extern crate more_asserts;

extern crate xz2;
extern crate zip;

use std::path::Path;
use std::path::PathBuf;

mod errors;
mod file_list;
mod file_type;
mod meta;
mod mio;
mod simple_time;
mod unpacker;

pub use errors::*;
pub use file_list::Id;
pub use unpacker::Status;

pub struct Unpack {
    file_list: file_list::FileList,
    status: Status,
}

impl Unpack {
    pub fn unpack<P: AsRef<Path>, F: AsRef<Path>>(root: P, what: F) -> Result<Unpack> {
        let mut file_list = file_list::FileList::new_in(root)?;
        Ok(Unpack {
            status: unpacker::unpack_unknown(mio::Mio::from_path(what)?, &mut file_list),
            file_list,
        })
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn path_of(&self, item: Id) -> PathBuf {
        self.file_list.path_of(item)
    }
}

pub fn print(entries: &[unpacker::Entry], depth: usize) {
    for entry in entries {
        print!(
            "{} - {:?} at {:?}:",
            String::from_utf8_lossy(&vec![b' '; depth]),
            String::from_utf8_lossy(&entry.local.path),
            entry.local.temp
        );

        if let unpacker::Status::Success(ref children) = entry.children {
            println!();
            print(children, depth + 2);
        } else {
            println!(" {:?}", entry.children);
        }
    }
}
