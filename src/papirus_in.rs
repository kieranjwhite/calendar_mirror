use crate::err;
use std::fs::File;
use memmap::MmapOptions;

err!(Error {
    File(io::Error)
});

pub struct GPIO {
    pub fn new() -> Result<GPIO, Error> {
        let f=File::open("/dev/gpiomem")?;
        let mmap = unsafe { MmapOptions::new().map(&f)? };
    }
}
