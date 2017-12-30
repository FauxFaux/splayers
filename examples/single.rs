extern crate splayers;

use std::env;

fn main() {
    match splayers::unpack_into(env::args_os().nth(1).expect("argument: file"), "/tmp")
        .expect("unpacking failed") {
        splayers::Status::Success(ref entries) => splayers::print(entries, 0),
        ref other => println!("top level: {:?}", other),
    }
}
