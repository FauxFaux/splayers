extern crate ar;

#[macro_use]
extern crate error_chain;

#[cfg(intellij_type_hinting)]
extern crate error_chain_for_dumb_ides;

extern crate flate2;
extern crate tar;
extern crate tempdir;
extern crate tempfile_fast;
extern crate time as crates_time;

#[macro_use]
extern crate more_asserts;

extern crate xz2;
extern crate zip;

use std::env;

mod errors;
mod filetype;
mod meta;
mod mio;
mod simple_time;
mod stash;
mod unpacker;

use errors::*;

quick_main!(run);
fn run() -> Result<()> {
    let mut stash = stash::Stash::new()?;
    print(
        &unpacker::unpack_unknown(
            mio::Mio::from_path(env::args().nth(1).expect("first argument: file"))?,
            &mut stash,
        )?,
        0,
    )?;
    println!("{:?}", stash);
    ::std::process::exit(1);
    Ok(())
}

fn print(entries: &[unpacker::Entry], depth: usize) -> Result<()> {
    for entry in entries {
        print!(
            "{} - {:?} at {:?}:",
            String::from_utf8_lossy(&vec![b' '; depth]),
            String::from_utf8_lossy(&entry.local.path),
            entry.local.temp
        );

        match entry.children.as_ref() {
            Err(msg) => println!(" {}", msg),
            Ok(children) => {
                println!();
                print(children, depth + 2)?;
            }
        }
    }
    Ok(())
}
