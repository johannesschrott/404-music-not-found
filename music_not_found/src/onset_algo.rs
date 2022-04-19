use rustfft::{num_complex::Complex, FftPlanner};

use crate::track::Track;

const WINDOW_SIZE: usize = 2048;

pub struct OnsetInput {
    samples: Vec<f32>,
    spectogramm: Vec<Vec<Complex<f32>>>,
}

impl OnsetInput {
    pub fn from_track(track: Track) -> OnsetInput {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(WINDOW_SIZE);
        let samples: Vec<Complex<f32>> = track
            .samples
            .iter()
            .map(|&value| Complex::new(value, 0f32))
            .collect();

        let mut stft = Vec::new();

        let mut cur_pos: usize = 0;
        while cur_pos < samples.len() {
            let mut fft_buffer = samples[cur_pos..cur_pos + WINDOW_SIZE].to_owned();
            fft.process(&mut fft_buffer);
            cur_pos += WINDOW_SIZE / 2; // TODO: evtl. nicht um /2 sonden um ganzen N_ONSET verschieben
            stft.push(fft_buffer);
        }
        OnsetInput {
            samples: track.samples,
            spectogramm: stft,
        }
    }
}

pub struct OnsetOutput {
    result: Vec<f32>,
}

pub trait OnsetAlgorithm {
    fn process(input: &OnsetInput) -> OnsetOutput;
}

pub struct DummyAlgorithm;

impl OnsetAlgorithm for DummyAlgorithm {
    fn process(input: &OnsetInput) -> OnsetOutput {
        OnsetOutput {
            result: input.samples.to_owned(),
        }
    }
}

// Hier dann die Algos implementieren