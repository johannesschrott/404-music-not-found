use std::cmp::{max, min};
use std::iter::repeat;


use conv::*;
use dsp::window;
use rustfft::num_traits::abs;
use rustfft::{num_complex::Complex, FftPlanner};

use crate::statistics::{convolve1D, normalize, stft};
use crate::track::Track;

const WINDOW_SIZE: usize = 2048;

pub struct OnsetInput {
    pub samples: Vec<f32>,
    pub stft: Vec<Vec<Complex<f32>>>,
}

impl OnsetInput {
    pub fn from_track(track: &Track) -> OnsetInput {
        OnsetInput {
            samples: track.samples.to_owned(),
            stft: stft(&track.samples, WINDOW_SIZE, WINDOW_SIZE / 2),
        }
    }
}

pub struct OnsetOutput {
    pub result: Vec<f32>,
    pub means: Vec<f32>,
    pub fft_window_size: usize,
}

impl OnsetOutput {
    fn make_output(onset_input: &OnsetInput, result: Vec<f32>) -> OnsetOutput {
        OnsetOutput {
            result: normalize(&result),
            means: normalize(
                &onset_input
                    .stft
                    .iter()
                    .map(|stft_window| stft_window.iter().map(|x| x.norm()).sum())
                    .collect::<Vec<f32>>()[..],
            ),
            fft_window_size: WINDOW_SIZE,
        }
    }
    pub fn convolve<F>(&self, kernel_size: usize, kernel_function: F) -> OnsetOutput
    where
        F: Fn(&[f32]) -> f32,
    {
        OnsetOutput {
            fft_window_size: self.fft_window_size,
            means: normalize(&convolve1D(&self.means, kernel_size, &kernel_function)),
            result: normalize(&convolve1D(&self.result, kernel_size, &kernel_function)),
        }
    }
}

pub trait OnsetAlgorithm {
    fn find_onsets(input: &OnsetInput) -> OnsetOutput;
}

pub struct DummyAlgorithm;

impl OnsetAlgorithm for DummyAlgorithm {
    fn find_onsets(input: &OnsetInput) -> OnsetOutput {
        OnsetOutput::make_output(input, input.samples.to_owned())
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
        OnsetOutput::make_output(input, d)
    }
}

pub struct SpectralDifference;

impl SpectralDifference {}

impl OnsetAlgorithm for SpectralDifference {
    fn find_onsets(input: &OnsetInput) -> OnsetOutput {
        /*let d: Vec<f32> = input
        .stft
        .iter()
        .map(|stft|  stft.iter().map(|&comp| comp.re as f32).collect::<Vec<f32>>())
        .into_iter()
        .tuple_windows()
        .map(|spectrums| {
            let s = spectrums;
            return 0f32;
        })
        .collect();
        */
        let mut spectral_differences: Vec<Vec<f32>> = Vec::new();
        let empty_diff = vec![0; 1024].iter().map(|&i| (i as f32) * 0.0).collect();
        spectral_differences.push(empty_diff);

        let stft_len = input.stft[0].len();
        for i in 1..input.stft.len() {
            let mut sd: Vec<f32> = Vec::new();
            // formula for sd see slide 24 in L04.pdf
            for j in 0..stft_len {
                let x: f32 = (input.stft[i][j].re - input.stft[i - 1][j].re).powi(2);
                sd.push((x + abs(x)) / 2 as f32)
            }
            spectral_differences.push(sd);
        }

        let d: Vec<f32> = spectral_differences
            .iter()
            .map(|diffs| diffs.iter().sum::<f32>())
            .collect();

        OnsetOutput::make_output(input, d)
    }
}

pub struct Peaks {
    pub peaks: Vec<bool>,
}

pub struct PeakPicker {
    pub local_window_max: usize,  // == w1 == w2
    pub local_window_mean: usize, // == w3 == w4
    pub delta: f32,
    pub minimum_distance: usize,
}

pub struct OnsetTimes {
    pub onset_times: Vec<f64>,
}

impl PeakPicker {
    pub fn pick(&self, onset_output: &OnsetOutput) -> Peaks {
        // Compute times of peaks
        let output = &onset_output.result;
        let means = &onset_output.means;

        let mut peaks: Vec<bool> = repeat(false).take(output.len()).collect();

        let mean_window = |mean_left, mean_right| {
            output[mean_left..mean_right].iter().sum::<f32>() / (mean_right - mean_left) as f32
        };
        let max_window = |max_left, max_right| {
            output[max_left..max_right].iter().cloned().fold(0. / 0., f32::max)
        };

        let minimum_distance = |i, peaks: &[bool]| {
            let v = &peaks[max(i, self.minimum_distance) - self.minimum_distance..i]
                .iter()
                .any(|&f| f);
            !v
        };

        for i in 1..output.len() - 1 {
            let mean_left = max(i, self.local_window_mean) - self.local_window_mean;
            let mean_right = min(output.len(), i + self.local_window_mean + 1);

            let max_left = max(i, self.local_window_max) - self.local_window_max;
            let max_right = min(output.len(), i + self.local_window_max + 1);


            peaks[i] = (output[i - 1] < output[i] && output[i] > output[i + 1] ) // checks if a peak
            // implement adaptive peak picking
                && minimum_distance(i, &peaks)
                && output[i] >= mean_window(mean_left, mean_right)
                && output[i] >= max_window(max_left, max_right);
        }
        Peaks { peaks }
    }
}

impl Peaks {
    pub fn onset_times(&self, track: &Track) -> OnsetTimes {
        let mut onset_times: Vec<f64> = Vec::new();

        for i in 0..self.peaks.len() {
            if self.peaks[i] {
                onset_times
                    .push(i as f64 * ((WINDOW_SIZE as f64) / (track.header.sample_rate as f64)));
            }
        }
        OnsetTimes { onset_times }
    }
}
