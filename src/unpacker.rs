use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use walkdir;

use meta;
use errors::*;
use file_type;
use file_type::FileType;
use mio;
use mio::Mio;
use temps::Temps;

#[derive(Debug)]
pub struct Entry {
    pub local: LocalEntry,
    pub children: Status,
}

#[derive(Debug)]
pub struct LocalEntry {
    pub temp: Option<PathBuf>,
    pub meta: meta::Meta,
    pub path: Box<[u8]>,
}

#[derive(Debug)]
pub enum Status {
    Unnecessary,
    Unrecognised,
    Unsupported(FileType),
    Error(String),
    Success(Vec<Entry>),
}

pub fn unpack_root<P: AsRef<Path>>(from: P, temps: &mut Temps) -> Result<Status> {
    if !from.as_ref().is_dir() {
        return Ok(unpack_unknown(mio::Mio::from_path(from)?, temps));
    }

    let mut entries = Vec::new();
    for entry in walkdir::WalkDir::new(&from) {
        let entry = entry?;
        if entry.file_type().is_dir() {
            continue;
        }

        let relative_path = entry
            .path()
            .strip_prefix(&from)
            .expect("dir walking confusion");

        let temp = if !entry.path().symlink_metadata()?.file_type().is_symlink() {
            Some(temps.insert(fs::File::open(entry.path())
                .chain_err(|| format!("opening input path: {:?}", entry.path()))?)?)
        } else {
            None
        };

        entries.push(
            LocalEntry {
                temp,
                meta: meta::file(entry.path())?,
                path: relative_path
                    .as_os_str()
                    .to_str()
                    .ok_or("unencodable path in local filesystem is unsupported")?
                    .as_bytes()
                    .to_vec()
                    .into_boxed_slice(),
            }.into_entry(temps),
        )
    }

    Ok(Status::Success(entries))
}

pub fn unpack_unknown(mut from: Mio, temps: &mut Temps) -> Status {
    match match FileType::identify(&from.header()) {
        FileType::Deb => unpack_deb(from, temps),
        FileType::Tar => unpack_tar(from, temps),
        FileType::Zip => unpack_zip(from, temps),
        FileType::Bz => unpack_bz(from, temps),
        FileType::Gz => unpack_gz(from, temps),
        FileType::Xz => unpack_xz(from, temps),
        FileType::Empty => return Status::Unnecessary,
        FileType::Other => return Status::Unrecognised,
        other => return Status::Unsupported(other),
    } {
        Ok(kids) => Status::Success(
            kids.into_iter()
                .map(|local| local.into_entry(temps))
                .collect(),
        ),
        Err(e) => Status::Error(format!("{}", e)),
    }
}

fn unpack_deb(from: Mio, temps: &mut Temps) -> Result<Vec<LocalEntry>> {
    use ar;

    let mut entries = Vec::new();

    let mut decoder = ar::Archive::new(from);
    while let Some(entry) = decoder.next_entry() {
        let entry = entry?;
        let size = entry.header().size();
        let path = entry.header().identifier().to_vec().into_boxed_slice();
        let meta = meta::ar(entry.header())?;

        entries.push(LocalEntry {
            meta,
            path,
            temp: insert_if_non_empty(temps, entry, size)?,
        });
    }

    Ok(entries)
}

fn unpack_tar<R: Read>(from: R, temps: &mut Temps) -> Result<Vec<LocalEntry>> {
    use tar;

    let mut entries = Vec::new();

    for tar in tar::Archive::new(from).entries()? {
        let tar = tar?;
        let size = tar.header().size()?;
        let path = tar.path_bytes().to_vec().into_boxed_slice();
        let mut meta = meta::tar(tar.header(), tar.link_name_bytes())?;

        entries.push(LocalEntry {
            meta,
            path,
            temp: insert_if_non_empty(temps, tar, size)?,
        });
    }

    Ok(entries)
}

fn unpack_zip(from: Mio, temps: &mut Temps) -> Result<Vec<LocalEntry>> {
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
            temp: insert_if_non_empty(temps, entry, size)?,
        });
    }

    Ok(entries)
}

enum EmbeddedTar<T> {
    Found(Vec<LocalEntry>),
    Absent(io::BufReader<T>),
}

fn embedded_tar<F, T: Read>(from: Mio, make: F, temps: &mut Temps) -> Result<EmbeddedTar<T>>
where
    F: Fn(Mio) -> T,
{
    let backup = from.clone();
    let mut decoder = io::BufReader::new(make(from));
    if !file_type::is_probably_tar(&mio::fill_buf(&mut decoder)?) {
        return Ok(EmbeddedTar::Absent(decoder));
    }

    Ok(match unpack_tar(decoder, temps) {
        Ok(v) => EmbeddedTar::Found(v),
        Err(_) => EmbeddedTar::Absent(io::BufReader::new(make(backup))),
    })
}

fn unpack_bz(from: Mio, temps: &mut Temps) -> Result<Vec<LocalEntry>> {
    use bzip2;

    let decoder = match embedded_tar(from, bzip2::read::BzDecoder::new, temps)? {
        EmbeddedTar::Found(vec) => return Ok(vec),
        EmbeddedTar::Absent(decoder) => decoder,
    };

    let temp = Some(temps.insert(decoder)?);

    Ok(vec![
        LocalEntry {
            temp,
            meta: meta::just_stream(),
            path: b"..bz2".to_vec().into_boxed_slice(),
        },
    ])
}

fn unpack_gz(from: Mio, temps: &mut Temps) -> Result<Vec<LocalEntry>> {
    use flate2;

    let decoder = match embedded_tar(from, flate2::read::GzDecoder::new, temps)? {
        EmbeddedTar::Found(vec) => return Ok(vec),
        EmbeddedTar::Absent(decoder) => decoder,
    };

    let header = decoder.get_ref().header().ok_or("invalid header")?.clone();
    let temp = Some(temps.insert(decoder)?);

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

fn unpack_xz(from: Mio, temps: &mut Temps) -> Result<Vec<LocalEntry>> {
    use xz2;

    let decoder = match embedded_tar(from, xz2::read::XzDecoder::new, temps)? {
        EmbeddedTar::Found(vec) => return Ok(vec),
        EmbeddedTar::Absent(decoder) => decoder,
    };

    let temp = Some(temps.insert(decoder)?);

    Ok(vec![
        LocalEntry {
            temp,
            meta: meta::just_stream(),
            path: b"..xz".to_vec().into_boxed_slice(),
        },
    ])
}

fn insert_if_non_empty<R: Read>(temps: &mut Temps, from: R, size: u64) -> Result<Option<PathBuf>> {
    Ok(if 0 == size {
        None
    } else {
        Some(temps.insert(from.take(size))?)
    })
}

impl LocalEntry {
    fn into_entry(mut self, temps: &mut Temps) -> Entry {
        let children = if let Some(temp) = self.temp.as_ref() {
            unpack_unknown(Mio::from_path(temp).expect("working with temps"), temps)
        } else {
            Status::Unnecessary
        };

        if children.fully_consumed() {
            self.temp = None;
        }

        Entry {
            children,
            local: self,
        }
    }
}

impl Status {
    fn fully_consumed(&self) -> bool {
        match *self {
            Status::Success(ref v) if !v.is_empty() => true,
            _ => false,
        }
    }
}
