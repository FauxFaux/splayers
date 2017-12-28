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

mod errors;
mod filetype;
mod meta;
mod mio;
mod simple_time;
mod stash;
mod unpacker;

pub use unpacker::UnpackResult;
use errors::*;

pub fn unpack<P: AsRef<Path>, F: AsRef<Path>>(root: P, what: F) -> Result<UnpackResult> {
    let mut stash = stash::Stash::new()?;
    Ok(unpacker::unpack_unknown(
        mio::Mio::from_path(what)?,
        &mut stash,
    ))
}

pub fn print(entries: &[unpacker::Entry], depth: usize) {
    for entry in entries {
        print!(
            "{} - {:?} at {:?}:",
            String::from_utf8_lossy(&vec![b' '; depth]),
            String::from_utf8_lossy(&entry.local.path),
            entry.local.temp
        );

        if let unpacker::UnpackResult::Success(ref children) = entry.children {
            println!();
            print(children, depth + 2);
        } else {
            println!(" {:?}", entry.children);
        }
    }
}
