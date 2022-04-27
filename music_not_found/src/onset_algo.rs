use std::cmp::max;
use std::iter::repeat;

use conv::*;
use dsp::window;
use rustfft::num_traits::abs;
use rustfft::{num_complex::Complex, FftPlanner};

use crate::statistics::normalize;
use crate::track::Track;

const WINDOW_SIZE: usize = 2048;

pub struct OnsetInput {
    pub samples: Vec<f32>,
    pub stft: Vec<Vec<Complex<f32>>>,
}

impl OnsetInput {
    pub fn from_track(track: &Track) -> OnsetInput {
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

            let mut fft_buffer_comp: Vec<Complex<f32>> = fft_buffer_real
                .iter()
                .map(|&value| Complex::new(value, 0f32))
                .collect();
            fft.process(&mut fft_buffer_comp);
            cur_pos += WINDOW_SIZE; // TODO: evtl. nicht um /2 sonden um ganzen N_ONSET verschieben
            stft.push(fft_buffer_comp);
        }

        let mut fft_in: Vec<f32> = track.samples[cur_pos..track.samples.len() - 1].to_owned();
        fft_in.extend(repeat(0f32).take(WINDOW_SIZE - fft_in.len()));
        let mut fft_buffer_real = vec![0f32; WINDOW_SIZE];
        hamming.apply(&fft_in, &mut fft_buffer_real);

        let mut fft_buffer_comp: Vec<Complex<f32>> = fft_buffer_real
            .iter()
            .map(|&value| Complex::new(value, 0f32))
            .collect();
        fft.process(&mut fft_buffer_comp);
        stft.push(fft_buffer_comp);

        OnsetInput {
            samples: track.samples.to_owned(),
            stft,
        }
    }
}

pub struct OnsetOutput {
    pub result: Vec<f32>,
    pub mean: f32,
    pub fft_window_size: usize,
}

impl OnsetOutput {
    fn make_output(result: Vec<f32>) -> OnsetOutput {
        OnsetOutput {
            result: normalize(&result),
            mean: result.iter().sum::<f32>() / result.len() as f32,
            fft_window_size: WINDOW_SIZE,
        }
    }
}

pub trait OnsetAlgorithm {
    fn find_onsets(input: &OnsetInput) -> OnsetOutput;
}

pub struct DummyAlgorithm;

impl OnsetAlgorithm for DummyAlgorithm {
    fn find_onsets(input: &OnsetInput) -> OnsetOutput {
        OnsetOutput::make_output(input.samples.to_owned())
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
        OnsetOutput::make_output(d)
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

        OnsetOutput::make_output(d)
    }
}

pub struct Peaks {
    pub peaks: Vec<bool>,
}

pub struct OnsetTimes {
    pub onset_times: Vec<f64>,
}

impl Peaks {
    pub fn pick(output: &[f32]) -> Peaks {
        // Compute times of peaks
        let peaks: Vec<bool> = (0..output.len())
            .into_iter()
            .map(|i| {
                return if (i > 0 && i < output.len() - 1) /* checks if index is at border */
                        && (output[i - 1] <output[i] && output[i] > output[i + 1] )
                /* checks if a peak */
                {
                    true
                } else {
                    false
                };
            })
            .collect::<Vec<bool>>();
        Peaks { peaks }
    }

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
