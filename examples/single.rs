extern crate splayers;

use std::env;

fn main() {
    match splayers::unpack("/tmp", env::args_os().nth(1).expect("argument: file"))
        .expect("unpacking failed")
    {
        splayers::UnpackResult::Success(ref entries) => splayers::print(entries, 0),
        other => println!("top level: {:?}", other),
    }
}
