use std::iter::repeat;

use crate::track::Track;
use conv::*;
use rustfft::{num_complex::Complex, FftPlanner};

const WINDOW_SIZE: usize = 2048;

pub struct OnsetInput {
    pub samples: Vec<f32>,
    pub stft: Vec<Vec<Complex<f32>>>,
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
        while cur_pos + WINDOW_SIZE < samples.len() {
            let mut fft_buffer = samples[cur_pos..cur_pos + WINDOW_SIZE].to_owned();
            fft.process(&mut fft_buffer);
            cur_pos += WINDOW_SIZE / 2; // TODO: evtl. nicht um /2 sonden um ganzen N_ONSET verschieben
            stft.push(fft_buffer);
        }

        let mut fft_buffer = samples[cur_pos..samples.len() - 1].to_owned();
        fft_buffer.extend(repeat(Complex::new(0f32, 0f32)).take(WINDOW_SIZE - fft_buffer.len()));
        fft.process(&mut fft_buffer);
        stft.push(fft_buffer);

        OnsetInput {
            samples: track.samples,
            stft,
        }
    }
}

pub struct OnsetOutput {
    pub result: Vec<f32>,
}

pub trait OnsetAlgorithm {
    fn find_onsets(input: &OnsetInput) -> OnsetOutput;
}

pub struct DummyAlgorithm;

impl OnsetAlgorithm for DummyAlgorithm {
    fn find_onsets(input: &OnsetInput) -> OnsetOutput {
        OnsetOutput {
            result: input.samples.to_owned(),
        }
    }
}

// Hier dann die Algos implementieren

pub struct HighFrequencyContent;

impl HighFrequencyContent {
    fn weights(size: usize) -> Vec<f32> {
        let size_f32 = f32::value_from(size).unwrap();
        (0..size)
            .map(|a| f32::value_from(a).unwrap() / size_f32)
            .collect()
    }
}

impl OnsetAlgorithm for HighFrequencyContent {
    fn find_onsets(input: &OnsetInput) -> OnsetOutput {
        let weights = HighFrequencyContent::weights(WINDOW_SIZE);
        let d: Vec<f32> = input
            .stft
            .iter()
            .map(|single_fft| {
                let s: f32 = single_fft
                    .iter()
                    .zip(weights.iter())
                    .map(|(v, w)| v.norm_sqr() * w)
                    .sum();
                s / f32::value_from(WINDOW_SIZE).unwrap()
            })
            .collect();
        OnsetOutput { result: d }
    }
}

pub fn normalize(data: Vec<f32>) -> Vec<f32> {
    todo!()
}