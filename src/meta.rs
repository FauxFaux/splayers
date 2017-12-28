use std::collections::HashMap;

use ar;
use flate2;
use tar;
use zip;

use errors::*;
use simple_time;

#[derive(Clone, Debug)]
pub struct Meta {
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub btime: u64,
    pub item_type: ItemType,
    pub ownership: Ownership,
    pub xattrs: HashMap<String, Box<[u8]>>,
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
    SymbolicLink(String),
    /// A hardlink, with its destination.
    HardLink(String),
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

pub fn just_stream() -> Meta {
    Meta {
        atime: 0,
        mtime: 0,
        ctime: 0,
        btime: 0,
        item_type: ItemType::RegularFile,
        ownership: Ownership::Unknown,
        xattrs: HashMap::new(),
    }
}

pub fn ar(header: &ar::Header) -> Result<Meta> {
    Ok(Meta {
        atime: 0,
        mtime: simple_time::simple_time_epoch_seconds(header.mtime()),
        ctime: 0,
        btime: 0,
        item_type: ItemType::Unknown,
        ownership: Ownership::Unknown,
        xattrs: HashMap::new(),
    })
}

pub fn gz(header: &flate2::GzHeader) -> Result<Meta> {
    Ok(Meta {
        atime: 0,
        mtime: simple_time::simple_time_epoch_seconds(header.mtime() as u64),
        ctime: 0,
        btime: 0,
        item_type: ItemType::Unknown,
        ownership: Ownership::Unknown,
        xattrs: HashMap::new(),
    })
}

pub fn tar(header: &tar::Header) -> Result<Meta> {
    Ok(Meta {
        atime: 0,
        mtime: simple_time::simple_time_epoch_seconds(header.mtime()?),
        ctime: 0,
        btime: 0,
        item_type: ItemType::Unknown,
        ownership: Ownership::Unknown,
        xattrs: HashMap::new(),
    })
}

pub fn zip(header: &zip::read::ZipFile) -> Result<Meta> {
    Ok(Meta {
        atime: 0,
        mtime: simple_time::simple_time_tm(header.last_modified()),
        ctime: 0,
        btime: 0,
        item_type: ItemType::Unknown,
        ownership: Ownership::Unknown,
        xattrs: HashMap::new(),
    })
}
