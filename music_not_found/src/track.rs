use std::{path::Path, fs::File};
use wav_io::header::WavHeader;

pub struct Track {
    pub samples: Vec<f32>,
    header: WavHeader
}

impl Track {
    pub fn from_path(file_path: &Path) -> Track {
        let input_file = File::open(&file_path).unwrap();
        let (header, samples) = wav_io::read_from_file(input_file).unwrap();
        Track { samples,header }
    }

}
