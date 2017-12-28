use std::fmt;

use meta;
use errors::*;
use filetype::FileType;
use mio::Mio;
use stash::Stash;
use stash::Stashed;

use std::io::Read;

pub struct Entry {
    pub local: LocalEntry,
    pub children: ::std::result::Result<Vec<Entry>, String>,
}

#[derive(Debug)]
pub struct LocalEntry {
    pub temp: Option<Stashed>,
    pub meta: meta::Meta,
    pub path: Box<[u8]>,
}

pub fn unpack_unknown(mut from: Mio, stash: &mut Stash) -> Result<Vec<Entry>> {
    let kids = match FileType::identify(&from.header()?) {
        FileType::Deb => unpack_deb(from, stash)?,
        FileType::Tar => unpack_tar(from, stash)?,
        FileType::Gz => unpack_gz(from, stash)?,
        FileType::Xz => unpack_xz(from, stash)?,
        other => bail!("unrecognised file type: {:?}", other),
    };

    Ok(kids.into_iter()
        .map(|local| Entry {
            children: match local.temp {
                Some(temp) => unpack_unknown(stash.open(temp), stash).map_err(|x| format!("{:?}", x)),
                None => Err("empty file".to_string()),
            },
            local,
        })
        .collect())
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
