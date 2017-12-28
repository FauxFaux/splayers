use std::fmt;

use meta;
use errors::*;
use filetype::FileType;
use mio::Mio;
use stash::Stash;
use stash::Stashed;

use std::io::Read;

#[derive(Debug)]
pub struct Entry {
    pub local: LocalEntry,
    pub children: UnpackResult,
}

#[derive(Debug)]
pub struct LocalEntry {
    pub temp: Option<Stashed>,
    pub meta: meta::Meta,
    pub path: Box<[u8]>,
}

#[derive(Debug)]
pub enum UnpackResult {
    Unnecessary,
    Unrecognised,
    Unsupported(FileType),
    Error(String),
    Success(Vec<Entry>),
}

pub fn unpack_unknown(mut from: Mio, stash: &mut Stash) -> UnpackResult {
    match match FileType::identify(&from.header()) {
        FileType::Deb => unpack_deb(from, stash),
        FileType::Tar => unpack_tar(from, stash),
        FileType::Zip => unpack_zip(from, stash),
        FileType::Bz => unpack_bz(from, stash),
        FileType::Gz => unpack_gz(from, stash),
        FileType::Xz => unpack_xz(from, stash),
        FileType::Empty => return UnpackResult::Unnecessary,
        FileType::Other => return UnpackResult::Unrecognised,
        other => return UnpackResult::Unsupported(other),
    } {
        Ok(kids) => UnpackResult::Success(
            kids.into_iter()
                .map(|local| local.into_entry(stash))
                .collect(),
        ),
        Err(e) => UnpackResult::Error(format!("{}", e)),
    }
}

fn unpack_deb(from: Mio, stash: &mut Stash) -> Result<Vec<LocalEntry>> {
    use ar;

    let mut entries = Vec::new();

    let mut decoder = ar::Archive::new(from);
    while let Some(entry) = decoder.next_entry() {
        let entry = entry?;
        let size = entry.header().size();
        let path = entry
            .header()
            .identifier()
            .as_bytes()
            .to_vec()
            .into_boxed_slice();
        let meta = meta::ar(entry.header())?;

        entries.push(LocalEntry {
            meta,
            path,
            temp: stash.stash_take(entry, size)?,
        });
    }

    Ok(entries)
}

fn unpack_tar(from: Mio, stash: &mut Stash) -> Result<Vec<LocalEntry>> {
    use tar;

    let mut entries = Vec::new();

    for tar in tar::Archive::new(from).entries()? {
        let tar = tar?;
        let size = tar.header().size()?;
        let path = tar.header().path_bytes().to_vec().into_boxed_slice();
        let meta = meta::tar(tar.header())?;

        entries.push(LocalEntry {
            meta,
            path,
            temp: stash.stash_take(tar, size)?,
        });
    }

    Ok(entries)
}

fn unpack_zip(from: Mio, stash: &mut Stash) -> Result<Vec<LocalEntry>> {
    use zip;

    let mut entries = Vec::new();

    let mut archive = zip::read::ZipArchive::new(from)?;
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;

        let size = entry.size();
        let path = entry.name_raw().to_vec().into_boxed_slice();
        let meta = meta::zip(&entry)?;

        entries.push(LocalEntry {
            meta,
            path,
            temp: stash.stash_take(entry, size)?,
        });
    }

    Ok(entries)
}

fn unpack_bz(from: Mio, stash: &mut Stash) -> Result<Vec<LocalEntry>> {
    use bzip2;

    let decoder = bzip2::read::BzDecoder::new(from);
    let temp = Some(stash.stash(decoder)?);

    Ok(vec![
        LocalEntry {
            temp,
            meta: meta::just_stream(),
            path: b"..bz2".to_vec().into_boxed_slice(),
        },
    ])
}

fn unpack_gz(from: Mio, stash: &mut Stash) -> Result<Vec<LocalEntry>> {
    use flate2;
    let decoder = flate2::read::GzDecoder::new(from);
    let header = decoder.header().ok_or("invalid header")?.clone();
    let temp = Some(stash.stash(decoder)?);

    Ok(vec![
        LocalEntry {
            temp,
            meta: meta::gz(&header)?,
            path: header
                .filename()
                .unwrap_or(b"..gz")
                .to_vec()
                .into_boxed_slice(),
        },
    ])
}

fn unpack_xz(from: Mio, stash: &mut Stash) -> Result<Vec<LocalEntry>> {
    use xz2;

    let decoder = xz2::read::XzDecoder::new(from);
    let temp = Some(stash.stash(decoder)?);

    Ok(vec![
        LocalEntry {
            temp,
            meta: meta::just_stream(),
            path: b"..xz".to_vec().into_boxed_slice(),
        },
    ])
}

impl LocalEntry {
    fn into_entry(self, stash: &mut Stash) -> Entry {
        let children = if let Some(temp) = self.temp {
            let val = unpack_unknown(stash.open(temp), stash);
            if val.fully_consumed() {
                stash.release(temp);
            }
            val
        } else {
            UnpackResult::Unnecessary
        };

        Entry {
            children,
            local: self,
        }
    }
}

impl UnpackResult {
    fn fully_consumed(&self) -> bool {
        match *self {
            UnpackResult::Success(ref v) if !v.is_empty() => true,
            _ => false,
        }
    }
}
