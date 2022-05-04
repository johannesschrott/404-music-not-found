use conv::*;
use rustfft::num_traits::abs;
use rustfft::{num_complex::Complex};

use crate::statistics::{convolve_1d, normalize, stft, WinVec};
use crate::track::Track;

#[derive(Clone)]
pub struct OnsetInput {
    pub samples: Vec<f32>,
    pub stft_2048_1024: WinVec<Vec<Complex<f32>>>,
}


impl OnsetInput {
    pub fn from_track(track: &Track) -> OnsetInput {
        OnsetInput {
            samples: track.samples.to_owned(),
            stft_2048_1024: stft(&track.samples, 2048, 1024),
        }
    }
}

pub struct OnsetOutput {
    pub result: WinVec<f32>,
}

impl OnsetOutput {
    pub fn convolve<F>(&self, kernel_size: usize, kernel_function: F) -> OnsetOutput
    where
        F: Fn(&[f32]) -> f32,
    {
        OnsetOutput {
            result: self
                .result
                .map(|d| normalize(&convolve_1d(&d, kernel_size, &kernel_function))),
        }
    }
}

pub trait OnsetAlgorithm {
    fn find_onsets(input: &OnsetInput) -> OnsetOutput;
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
        let weights = HighFrequencyContent::weights(input.stft_2048_1024.window_size);
        let data: WinVec<f32> = input.stft_2048_1024.map(|data| {
            data.iter()
                .map(|single_fft| {
                    let s: f32 = single_fft
                        .iter()
                        .zip(weights.iter())
                        .map(|(v, w)| v.norm_sqr() * w)
                        .sum();
                    s / f32::value_from(input.stft_2048_1024.window_size).unwrap()
                })
                .collect()
        });
        OnsetOutput { result: data }
    }
}

pub struct SpectralDifference;

impl SpectralDifference {}

impl OnsetAlgorithm for SpectralDifference {
    fn find_onsets(input: &OnsetInput) -> OnsetOutput {
        let mut spectral_differences: Vec<Vec<f32>> = Vec::new();
        let empty_diff = vec![0; 1024].iter().map(|&i| (i as f32) * 0.0).collect();
        spectral_differences.push(empty_diff);
        let data = &input.stft_2048_1024.data;

        let stft_len = data[0].len();
        for i in 1..data.len() {
            let mut sd: Vec<f32> = Vec::new();
            // formula for sd see slide 24 in L04.pdf
            for j in 0..stft_len {
                let x: f32 = (data[i][j].re - data[i - 1][j].re).powi(2);
                sd.push((x + abs(x)) / 2 as f32)
            }
            spectral_differences.push(sd);
        }

        let data: Vec<f32> = spectral_differences
            .iter()
            .map(|diffs| diffs.iter().sum::<f32>())
            .collect();

        OnsetOutput {
            result: input.stft_2048_1024.set_data(data),
        }
    }
}
