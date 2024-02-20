use std::error::Error;
use std::fs::OpenOptions;
use std::path::PathBuf;

use memmap::MmapMut;

fn main() -> Result<(), Box<dyn Error>> {
    let path: PathBuf = "./test_file.txt".into();
    let file = OpenOptions::new()
                           .read(true)
                           .write(true)
                           .create(true)
                           .open(&path)?;
    file.set_len(5)?;
    
    let mut mmap = unsafe { MmapMut::map_mut(&file)? };
    
    mmap.copy_from_slice(b"12345");

    println!("{:?}", &mmap[0..5]);

    
    Ok(())
}
