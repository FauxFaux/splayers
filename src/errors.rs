error_chain! {
    links {
        Ext4(::ext4::Error, ::ext4::ErrorKind);
    }

    foreign_links {
        Io(::std::io::Error);
        Zip(::zip::result::ZipError);
    }

}

#[cfg(intellij_type_hinting)]
pub use error_chain_for_dumb_ides::stubs::*;
