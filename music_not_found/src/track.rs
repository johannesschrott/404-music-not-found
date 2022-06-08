use std::{path::Path, fs::File};
use wav_io::header::WavHeader;

/// Structure holding the content of WAV files
pub struct Track {
    pub samples: Vec<f32>,
    pub header: WavHeader
}

impl Track {
    /// Create a new Track from a WAV file at the given path
    pub fn from_path(file_path: &Path) -> Track {
        let input_file = File::open(&file_path).unwrap();
        let (header, samples) = wav_io::read_from_file(input_file).unwrap();
        Track { samples,header }
    }

}
