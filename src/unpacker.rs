use std::io;
use std::io::Read;
use std::path::PathBuf;

use meta;
use errors::*;
use file_type;
use file_type::FileType;
use mio;
use mio::Mio;
use file_list::Temps;

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

pub fn unpack_unknown(mut from: Mio, file_list: &mut Temps) -> Status {
    match match FileType::identify(&from.header()) {
        FileType::Deb => unpack_deb(from, file_list),
        FileType::Tar => unpack_tar(from, file_list),
        FileType::Zip => unpack_zip(from, file_list),
        FileType::Bz => unpack_bz(from, file_list),
        FileType::Gz => unpack_gz(from, file_list),
        FileType::Xz => unpack_xz(from, file_list),
        FileType::Empty => return Status::Unnecessary,
        FileType::Other => return Status::Unrecognised,
        other => return Status::Unsupported(other),
    } {
        Ok(kids) => Status::Success(
            kids.into_iter()
                .map(|local| local.into_entry(file_list))
                .collect(),
        ),
        Err(e) => Status::Error(format!("{}", e)),
    }
}

fn unpack_deb(from: Mio, file_list: &mut Temps) -> Result<Vec<LocalEntry>> {
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
            temp: insert_if_non_empty(file_list, entry, size)?,
        });
    }

    Ok(entries)
}

fn unpack_tar<R: Read>(from: R, file_list: &mut Temps) -> Result<Vec<LocalEntry>> {
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
            temp: insert_if_non_empty(file_list, tar, size)?,
        });
    }

    Ok(entries)
}

fn unpack_zip(from: Mio, file_list: &mut Temps) -> Result<Vec<LocalEntry>> {
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
            temp: insert_if_non_empty(file_list, entry, size)?,
        });
    }

    Ok(entries)
}

enum EmbeddedTar<T> {
    Found(Vec<LocalEntry>),
    Absent(io::BufReader<T>),
}

fn embedded_tar<F, T: Read>(from: Mio, make: F, file_list: &mut Temps) -> Result<EmbeddedTar<T>>
where
    F: Fn(Mio) -> T,
{
    let backup = from.clone();
    let mut decoder = io::BufReader::new(make(from));
    if !file_type::is_probably_tar(&mio::fill_buf(&mut decoder)?) {
        return Ok(EmbeddedTar::Absent(decoder));
    }

    Ok(match unpack_tar(decoder, file_list) {
        Ok(v) => EmbeddedTar::Found(v),
        Err(_) => EmbeddedTar::Absent(io::BufReader::new(make(backup))),
    })
}

fn unpack_bz(from: Mio, file_list: &mut Temps) -> Result<Vec<LocalEntry>> {
    use bzip2;

    let decoder = match embedded_tar(from, |mio| bzip2::read::BzDecoder::new(mio), file_list)? {
        EmbeddedTar::Found(vec) => return Ok(vec),
        EmbeddedTar::Absent(decoder) => decoder,
    };

    let temp = Some(file_list.insert(decoder)?);

    Ok(vec![
        LocalEntry {
            temp,
            meta: meta::just_stream(),
            path: b"..bz2".to_vec().into_boxed_slice(),
        },
    ])
}

fn unpack_gz(from: Mio, file_list: &mut Temps) -> Result<Vec<LocalEntry>> {
    use flate2;

    let decoder = match embedded_tar(from, |mio| flate2::read::GzDecoder::new(mio), file_list)? {
        EmbeddedTar::Found(vec) => return Ok(vec),
        EmbeddedTar::Absent(decoder) => decoder,
    };

    let header = decoder.get_ref().header().ok_or("invalid header")?.clone();
    let temp = Some(file_list.insert(decoder)?);

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

fn unpack_xz(from: Mio, file_list: &mut Temps) -> Result<Vec<LocalEntry>> {
    use xz2;

    let decoder = match embedded_tar(from, |mio| xz2::read::XzDecoder::new(mio), file_list)? {
        EmbeddedTar::Found(vec) => return Ok(vec),
        EmbeddedTar::Absent(decoder) => decoder,
    };

    let temp = Some(file_list.insert(decoder)?);

    Ok(vec![
        LocalEntry {
            temp,
            meta: meta::just_stream(),
            path: b"..xz".to_vec().into_boxed_slice(),
        },
    ])
}

fn insert_if_non_empty<R: Read>(
    file_list: &mut Temps,
    from: R,
    size: u64,
) -> io::Result<Option<PathBuf>> {
    Ok(if 0 == size {
        None
    } else {
        Some(file_list.insert(from.take(size))?)
    })
}

impl LocalEntry {
    fn into_entry(mut self, file_list: &mut Temps) -> Entry {
        let children = if let Some(temp) = self.temp.as_ref() {
            unpack_unknown(
                Mio::from_path(temp).expect("working with file_list"),
                file_list,
            )
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
