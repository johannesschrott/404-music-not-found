use conv::*;
use rustfft::num_complex::Complex;
use rustfft::num_traits::abs;

use crate::statistics::{convolve_1d, mel, normalize, stft, WinVec};
use crate::track::Track;

#[derive(Clone)]
pub struct OnsetInput {
    pub samples: Vec<f32>,
    pub stft_2048_1024: WinVec<Vec<Complex<f32>>>,
    pub stft_1024_512: WinVec<Vec<Complex<f32>>>,
}

fn preprocess_mel(track: &Track, data: WinVec<Vec<Complex<f32>>>) -> WinVec<Vec<f32>> {
    // TODO: Dont know how to apply mel filterbank

    let mel_data = data.data.iter().map(|single_fft| single_fft.iter().map(|comp| mel(comp.norm())).collect::<Vec<f32>>()).collect::<Vec<Vec<f32>>>();

    let sampling_rate = track.header.sample_rate.to_owned();

    let no_mel_buckets = 40;


    WinVec {
        data: mel_data,
        window_size: data.window_size,
        hop_size: data.hop_size,
    }
}

impl OnsetInput {
    pub fn from_track(track: &Track) -> OnsetInput {
        OnsetInput {
            samples: track.samples.to_owned(),
            stft_2048_1024: stft(&track.samples, 2048, 1024),
            stft_1024_512: stft(&track.samples, 2048, 441), // This are roughly the values from slide 60/61
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
                let x: f32 = (data[i][j].norm() - data[i - 1][j].norm()).powi(2);
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


pub struct LFSF;

impl LFSF {}

impl OnsetAlgorithm for LFSF {
    fn find_onsets(input: &OnsetInput) -> OnsetOutput {

        // Phase 1 STFT computation already done (1024 512 Stft!!)

        // Phase 2 Apply Semitone Scale
        let a_base: f32 = 27.5;
        let mut a_semitones: Vec<i32> = Vec::new();
        let mut max_a: f32 = 0.0;

        while (44100 / 2) > (max_a as i32) {
            a_semitones.push(max_a as i32);
            if a_semitones.len() == 1 {
                max_a = a_base;
            } else {
                max_a *= 2.;
            }
        }
        a_semitones.push(max_a as i32);


        let mut filtered_vecs: Vec<Vec<f32>> = Vec::new();
        for i in 0..input.stft_1024_512.data.len() {
            let mut filtered_vec: Vec<f32> = Vec::new();
            for j in 2..a_semitones.len() {
                let lower = a_semitones[j - 2];
                let mid = a_semitones[j - 1];
                let higher = a_semitones[j];
                let mut sum_norm: f32 = 0.;
                let mut sum: f32 = 0.;
                if (mid as usize) < input.stft_1024_512.data.get(i).unwrap().len() {
                    for k in lower..mid {
                        sum_norm += (abs(k - lower) as f32) / ((mid - lower) as f32);
                        let t1 = input.stft_1024_512.data.get(i).unwrap().get(k as usize).unwrap().norm();
                        let t2 = (abs(k - lower) as f32) / ((mid - lower) as f32);
                        sum += mel(input.stft_1024_512.data.get(i).unwrap().get(k as usize).unwrap().norm()) * (abs(k - lower) as f32) / ((mid - lower) as f32);
                    }
                }
                if (higher as usize) < input.stft_1024_512.data.get(i).unwrap().len() {
                    for k in mid..higher {
                        sum_norm += 1. - (abs(higher - k) as f32) / ((higher - mid) as f32);
                        sum += mel(input.stft_1024_512.data.get(i).unwrap().get(k as usize).unwrap().norm()) * (1. - (abs(higher - k) as f32) / ((higher - mid) as f32));
                    }
                }
                let val = sum / sum_norm;
                if !val.is_nan() {
                    filtered_vec.push(val);
                }
            }
            filtered_vecs.push(filtered_vec);
        }

        let mut spectral_differences: Vec<Vec<f32>> = Vec::new();
        let empty_diff = vec![0; filtered_vecs.len() - 1].iter().map(|&i| (i as f32) * 0.0).collect();
        spectral_differences.push(empty_diff);

        let stft_len = filtered_vecs.get(0).unwrap().len();
        for i in 1..filtered_vecs.len() {
            let mut sd: Vec<f32> = Vec::new();
            // formula for sd see slide 24 in L04.pdf
            for j in 0..stft_len {
                let x: f32 = (mel(filtered_vecs[i][j] + 1. /*lambda already applied before (/sum)*/) - mel(filtered_vecs[i - 1][j] + 1.)).powi(2);
                sd.push((x + abs(x)) / 2 as f32)
            }
            spectral_differences.push(sd);
        }

        let data: Vec<f32> = spectral_differences
            .iter()
            .map(|diffs| diffs.iter().sum::<f32>())
            .collect();

        OnsetOutput {
            result: input.stft_1024_512.set_data(data),
        }
    }
}