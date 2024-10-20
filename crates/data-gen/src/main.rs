use std::{fs::OpenOptions, io::Write};

use gen::{complex::gen_complex, numbers::gen_numbers};

mod gen;

/// Generate test data.
fn main() {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open("results/data_generated.txt")
        .expect("Failed to open file!");

    let data = gen_numbers();
    let data = gen_complex();

    file.write_all(data.as_bytes())
        .expect("Failed to write to file!");
}
