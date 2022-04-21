use std::iter::repeat;

use conv::*;
use dsp::window;
use rustfft::{FftPlanner, num_complex::Complex};

use crate::track::Track;

const WINDOW_SIZE: usize = 2048;

pub struct OnsetInput {
    pub samples: Vec<f32>,
    pub stft: Vec<Vec<Complex<f32>>>,
}

impl OnsetInput {
    pub fn from_track(track: Track) -> OnsetInput {
        let mut planner = FftPlanner::new();
        let hamming = window::hamming(WINDOW_SIZE);

        let fft = planner.plan_fft_forward(WINDOW_SIZE);
        //   let samples: Vec<Complex<f32>> = track
        //       .samples
        //      .iter()
        //     .map(|&value| Complex::new(value, 0f32))
        //      .collect();

        let mut stft = Vec::new();

        let mut cur_pos: usize = 0;
        while cur_pos + WINDOW_SIZE < track.samples.len() {
            let mut fft_buffer_real = vec![0f32; WINDOW_SIZE];
            let fft_in = &track.samples[cur_pos..cur_pos + WINDOW_SIZE];

            hamming.apply(fft_in, &mut fft_buffer_real);

            let mut fft_buffer_comp: Vec<Complex<f32>> = fft_buffer_real.iter()
                .map(|&value| Complex::new(value, 0f32))
                .collect();
            fft.process(&mut fft_buffer_comp);
            cur_pos += WINDOW_SIZE/2 ; // TODO: evtl. nicht um /2 sonden um ganzen N_ONSET verschieben
            stft.push(fft_buffer_comp);
        }

        let mut fft_in: Vec<f32> = track.samples[cur_pos..track.samples.len() - 1].to_owned();
        fft_in.extend(repeat(0f32).take(WINDOW_SIZE - fft_in.len()));
        let mut fft_buffer_real = vec![0f32; WINDOW_SIZE];
        hamming.apply(&fft_in, &mut fft_buffer_real);


        let mut fft_buffer_comp: Vec<Complex<f32>> = fft_buffer_real.iter()
            .map(|&value| Complex::new(value, 0f32))
            .collect();
        fft.process(&mut fft_buffer_comp);
        stft.push(fft_buffer_comp);
        
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