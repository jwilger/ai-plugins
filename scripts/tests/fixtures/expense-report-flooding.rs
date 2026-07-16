use std::io::{self, Write};

fn main() {
    let chunk = [b'x'; 4096];
    let mut stdout = io::stdout().lock();
    loop {
        if stdout.write_all(&chunk).is_err() {
            return;
        }
    }
}
