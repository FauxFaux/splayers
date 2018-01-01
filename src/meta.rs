use std::borrow;
use std::fs;
use std::path::Path;

use ar;
use flate2;
use tar;
use zip;

use errors::*;
use simple_time;

#[derive(Clone, Debug)]
pub struct Meta {
    pub mtime: u64,
    pub item_type: ItemType,
    pub ownership: Ownership,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ItemType {
    // TODO: Magic value "Unknown", or an Option, or..?
    Unknown,
    RegularFile,
    Directory,
    Fifo,
    Socket,
    /// A symlink, with its destination.
    SymbolicLink(Box<[u8]>),
    /// A 'c' device.
    CharacterDevice {
        major: u32,
        minor: u32,
    },
    /// A 'b' device.
    BlockDevice {
        major: u32,
        minor: u32,
    },
}

enum RawItemType {
    Sloppy,
    SymbolicLink,
    CharacterDevice,
    BlockDevice,
}

#[derive(Clone, Debug)]
pub enum Ownership {
    Unknown,
    Posix {
        user: Option<PosixEntity>,
        group: Option<PosixEntity>,
        mode: u32,
    },
}

#[derive(Clone, Debug)]
pub struct PosixEntity {
    pub id: u32,
    pub name: String,
}

/// Directory.
const S_IFDIR: u32 = 0b1000;
/// Regular file.
const S_IFREG: u32 = 0b0100;
/// Symbolic link.
const S_IFLNK: u32 = 0b1010;
/// Fifo/pipe.
const S_IFIFO: u32 = 0b0001;
/// Socket
const S_IFSOCK: u32 = 0b1100;
/// Character device.
const S_IFCHR: u32 = 0b0010;
/// Block device.
const S_IFBLK: u32 = 0b0110;

impl ItemType {
    fn from_mode_lossy(mode: u32) -> ItemType {
        let mode_type = (mode >> 12) & 0b1111;
        match mode_type {
            S_IFREG => ItemType::RegularFile,
            S_IFDIR => ItemType::Directory,
            S_IFIFO => ItemType::Fifo,
            S_IFSOCK => ItemType::Socket,
            _ => ItemType::Unknown,
        }
    }
}

impl RawItemType {
    fn from_mode_lossy(mode: u32) -> RawItemType {
        let mode_type = (mode >> 12) & 0b1111;
        match mode_type {
            S_IFLNK => RawItemType::SymbolicLink,
            S_IFCHR => RawItemType::CharacterDevice,
            S_IFBLK => RawItemType::BlockDevice,
            _ => RawItemType::Sloppy,
        }
    }
}

impl PosixEntity {
    fn just_id(id: u32) -> PosixEntity {
        PosixEntity {
            id,
            name: String::new(),
        }
    }
}

pub fn just_stream() -> Meta {
    Meta {
        mtime: 0,
        item_type: ItemType::RegularFile,
        ownership: Ownership::Unknown,
    }
}

pub fn file<P: AsRef<Path>>(path: P) -> Result<Meta> {
    let meta = path.as_ref().metadata()?;

    let item_type = if meta.is_dir() {
        unreachable!()
    } else if meta.file_type().is_symlink() {
        ItemType::SymbolicLink(fs::read_link(path)?.to_str().ok_or("symlink to invalid utf-8")?.as_bytes().to_vec().into_boxed_slice())
    } else if meta.is_file() {
        ItemType::RegularFile
    } else {
        ItemType::Unknown
    };

    Ok(Meta {
        mtime: simple_time::simple_time_sys(meta.modified()?),
        item_type,
        ownership: Ownership::Unknown,
    })
}

pub fn ar(header: &ar::Header) -> Result<Meta> {
    Ok(Meta {
        mtime: simple_time::simple_time_epoch_seconds(header.mtime()),
        item_type: ItemType::from_mode_lossy(header.mode()),
        ownership: Ownership::Posix {
            user: Some(PosixEntity::just_id(header.uid())),
            group: Some(PosixEntity::just_id(header.gid())),
            mode: header.mode(),
        },
    })
}

pub fn gz(header: &flate2::GzHeader) -> Result<Meta> {
    Ok(Meta {
        mtime: simple_time::simple_time_epoch_seconds(u64::from(header.mtime())),
        item_type: ItemType::RegularFile,
        ownership: Ownership::Unknown,
    })
}

pub fn tar(header: &tar::Header, link_name_bytes: Option<borrow::Cow<[u8]>>) -> Result<Meta> {
    let mode = header.mode()?;
    Ok(Meta {
        mtime: simple_time::simple_time_epoch_seconds(header.mtime()?),
        item_type: match RawItemType::from_mode_lossy(mode) {
            RawItemType::SymbolicLink => ItemType::SymbolicLink(
                link_name_bytes
                    .ok_or("symbolic-link style file with no link")?
                    .to_vec()
                    .into_boxed_slice(),
            ),
            RawItemType::CharacterDevice => ItemType::CharacterDevice {
                major: header.device_major()?.ok_or("char device without major")?,
                minor: header.device_minor()?.ok_or("char device without minor")?,
            },
            RawItemType::BlockDevice => ItemType::BlockDevice {
                major: header.device_major()?.ok_or("block device without major")?,
                minor: header.device_minor()?.ok_or("block device without minor")?,
            },
            RawItemType::Sloppy => ItemType::from_mode_lossy(mode),
        },
        ownership: Ownership::Posix {
            user: Some(PosixEntity {
                id: header.uid()?,
                name: header.username()?.unwrap_or("").to_string(),
            }),
            group: Some(PosixEntity {
                id: header.gid()?,
                name: header.groupname()?.unwrap_or("").to_string(),
            }),
            mode: header.mode()?,
        },
    })
}

pub fn zip(header: &zip::read::ZipFile) -> Result<Meta> {
    Ok(Meta {
        mtime: simple_time::simple_time_tm(header.last_modified()),
        item_type: if header.name_raw().ends_with(b"/") {
            ItemType::Directory
        } else {
            ItemType::RegularFile
        },
        ownership: if let Some(mode) = header.unix_mode() {
            Ownership::Posix {
                user: None,
                group: None,
                mode,
            }
        } else {
            Ownership::Unknown
        },
    })
}
