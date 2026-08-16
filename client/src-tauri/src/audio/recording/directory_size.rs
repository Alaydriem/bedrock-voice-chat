use std::fs;
use std::path::PathBuf;


pub struct DirectorySize;

impl DirectorySize {
    pub fn calculate(path: &PathBuf) -> Result<u64, std::io::Error> {
        let mut total_size = 0u64;

        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    total_size += Self::calculate(&path)?;
                } else {
                    total_size += entry.metadata()?.len();
                }
            }
        }

        Ok(total_size)
    }
}
