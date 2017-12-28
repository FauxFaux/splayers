use std;
use std::error;
use std::io;

#[cfg(never)]
use ext4;
use zip;

error_chain! {
    links {
        Ext4(::ext4::Error, ::ext4::ErrorKind) #[cfg(never)];
    }

    foreign_links {
        Io(io::Error);
        Zip(zip::result::ZipError);
    }

}

#[cfg(intellij_type_hinting)]
pub use error_chain_for_dumb_ides::stubs::*;
