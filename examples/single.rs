extern crate splayers;

use std::env;

fn main() {
    let unpack =
        splayers::Unpack::unpack_into(env::args_os().nth(1).expect("argument: file"), "/tmp")
            .expect("unpacking failed");
    match *unpack.status() {
        splayers::Status::Success(ref entries) => splayers::print(entries, 0),
        ref other => println!("top level: {:?}", other),
    }

    println!("root: {:?}", unpack.into_path());
}
