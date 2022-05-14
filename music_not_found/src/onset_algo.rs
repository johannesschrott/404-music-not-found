use std::cmp::max;
use std::iter::repeat;
use serde::*;

use conv::*;
use rustfft::num_complex::Complex;
use rustfft::num_traits::{abs, Float};

use crate::statistics::{convolve_1d, mel, normalize, stft, zeroes, WinVec};
use crate::track::Track;

#[derive(Clone)]
pub struct OnsetInput {
    pub samples: Vec<f32>,
    pub stft: WinVec<Vec<Complex<f32>>>,
}

fn preprocess_mel(track: &Track, data: WinVec<Vec<Complex<f32>>>) -> WinVec<Vec<f32>> {
    // TODO: Dont know how to apply mel filterbank

    let mel_data = data
        .data
        .iter()
        .map(|single_fft| {
            single_fft
                .iter()
                .map(|comp| mel(comp.norm()))
                .collect::<Vec<f32>>()
        })
        .collect::<Vec<Vec<f32>>>();

    let sampling_rate = track.header.sample_rate.to_owned();

    let no_mel_buckets = 40;

    WinVec {
        data: mel_data,
        window_size: data.window_size,
        hop_size: data.hop_size,
    }
}

impl OnsetInput {
    pub fn from_track(track: &Track, window_size: usize, hop_size: usize) -> OnsetInput {
        OnsetInput {
            samples: track.samples.to_owned(),
            stft: stft(&track.samples, window_size, hop_size),
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
    fn find_onsets(&self, track: &Track, input: &OnsetInput) -> OnsetOutput;
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
    fn find_onsets(&self, track: &Track, input: &OnsetInput) -> OnsetOutput {
        let weights = HighFrequencyContent::weights(input.stft.window_size);
        let data: WinVec<f32> = input.stft.map(|data| {
            data.iter()
                .map(|single_fft| {
                    let s: f32 = single_fft
                        .iter()
                        .zip(weights.iter())
                        .map(|(v, w)| v.norm_sqr() * w)
                        .sum();
                    s / f32::value_from(input.stft.window_size).unwrap()
                })
                .collect()
        });
        OnsetOutput { result: data }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SpectralDifference;
impl SpectralDifference {}

impl OnsetAlgorithm for SpectralDifference {
    fn find_onsets(&self, track: &Track, input: &OnsetInput) -> OnsetOutput {
        let mut spectral_differences: Vec<Vec<f32>> = Vec::new();
        let empty_diff = vec![0; 1024].iter().map(|&i| (i as f32) * 0.0).collect();
        spectral_differences.push(empty_diff);
        let data = &input.stft.data;

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
            result: input.stft.set_data(data),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct LFSF {
    pub log_lambda: f32,
}

impl LFSF {
    const MEL_BANDS: usize = 128;

    fn apply_filterbank(track: &Track, data: &[Vec<Complex<f32>>]) -> Vec<Vec<f32>> {
        let filterbank: Vec<Vec<f32>> = mel_filter::mel(
            track.header.sample_rate as usize,
            data[0].len(),
            Some(LFSF::MEL_BANDS),
            None,
            None,
            false,
            mel_filter::NormalizationFactor::One,
        );

        data.iter()
            .enumerate()
            .map(|(i, frame)| {
                filterbank
                    .iter()
                    .map(|(mel_frame)| {
                        frame
                            .iter()
                            .enumerate()
                            .map(|(i, x)| x.norm() * mel_frame[i / 2])
                            .sum()
                    })
                    .collect()
            })
            .collect()
    }

    fn apply_log(&self, data: Vec<Vec<f32>>) -> Vec<Vec<f32>> {
        data.into_iter()
            .map(|frame| {
                frame
                    .into_iter()
                    .map(|x| (x * self.log_lambda + 1.).log10())
                    .collect()
            })
            .collect()
    }
}

impl OnsetAlgorithm for LFSF {
    fn find_onsets(&self, track: &Track, input: &OnsetInput) -> OnsetOutput {
        let raw_data = &input.stft.data;

        let data = &self.apply_log(LFSF::apply_filterbank(track, &raw_data[..]));

        let mut detection_vector: Vec<f32> = zeroes(data.len());
        let zero_vector = zeroes(data[0].len());

        let half_wave = |x| f32::max(x, 0.);
        let previous = |i| {
            if i == 0 {
                &zero_vector
            } else {
                &data[i - 1]
            }
        };

        for i in 0..data.len() {
            detection_vector[i] = data[i]
                .iter()
                .enumerate()
                .map(|(j, x)| half_wave(x - previous(i)[j]))
                .sum();
        }
        OnsetOutput {
            result: input.stft.set_data(detection_vector),
        }
    }
}

// impl OnsetAlgorithm for LFSF {
//     fn find_onsets(track: &Track, input: &OnsetInput) -> OnsetOutput {
//         // Phase 1 STFT computation already done (1024 512 Stft!!)

//         // Phase 2 Apply Semitone Scale
//         let a_base: f32 = 27.5;
//         let mut a_semitones: Vec<i32> = Vec::new();
//         let mut max_a: f32 = 0.0;

//         while (44100 / 2) > (max_a as i32) {
//             a_semitones.push(max_a as i32);
//             if a_semitones.len() == 1 {
//                 max_a = a_base;
//             } else {
//                 max_a *= 2.;
//             }
//         }
//         a_semitones.push(max_a as i32);

//         let mut filtered_vecs: Vec<Vec<f32>> = Vec::new();
//         for i in 0..input.stft.data.len() {
//             let mut filtered_vec: Vec<f32> = Vec::new();
//             for j in 2..a_semitones.len() {
//                 let lower = a_semitones[j - 2];
//                 let mid = a_semitones[j - 1];
//                 let higher = a_semitones[j];
//                 let mut sum_norm: f32 = 0.;
//                 let mut sum: f32 = 0.;
//                 if (mid as usize) < input.stft.data.get(i).unwrap().len() {
//                     for k in lower..mid {
//                         sum_norm += (abs(k - lower) as f32) / ((mid - lower) as f32);
//                         let t1 = input
//                             .stft
//                             .data
//                             .get(i)
//                             .unwrap()
//                             .get(k as usize)
//                             .unwrap()
//                             .norm();
//                         let t2 = (abs(k - lower) as f32) / ((mid - lower) as f32);
//                         sum += mel(input
//                             .stft
//                             .data
//                             .get(i)
//                             .unwrap()
//                             .get(k as usize)
//                             .unwrap()
//                             .norm())
//                             * (abs(k - lower) as f32)
//                             / ((mid - lower) as f32);
//                     }
//                 }
//                 if (higher as usize) < input.stft.data.get(i).unwrap().len() {
//                     for k in mid..higher {
//                         sum_norm += 1. - (abs(higher - k) as f32) / ((higher - mid) as f32);
//                         sum += mel(input
//                             .stft
//                             .data
//                             .get(i)
//                             .unwrap()
//                             .get(k as usize)
//                             .unwrap()
//                             .norm())
//                             * (1. - (abs(higher - k) as f32) / ((higher - mid) as f32));
//                     }
//                 }
//                 let val = sum / sum_norm;
//                 if !val.is_nan() {
//                     filtered_vec.push(val);
//                 }
//             }
//             filtered_vecs.push(filtered_vec);
//         }

//         let mut spectral_differences: Vec<Vec<f32>> = Vec::new();
//         let empty_diff = vec![0; filtered_vecs.len() - 1]
//             .iter()
//             .map(|&i| (i as f32) * 0.0)
//             .collect();
//         spectral_differences.push(empty_diff);

//         let stft_len = filtered_vecs.get(0).unwrap().len();
//         for i in 1..filtered_vecs.len() {
//             let mut sd: Vec<f32> = Vec::new();
//             // formula for sd see slide 24 in L04.pdf
//             for j in 0..stft_len {
//                 let x: f32 = (mel(
//                     filtered_vecs[i][j] + 1., /*lambda already applied before (/sum)*/
//                 ) - mel(filtered_vecs[i - 1][j] + 1.))
//                 .powi(2);
//                 sd.push((x + abs(x)) / 2 as f32)
//             }
//             spectral_differences.push(sd);
//         }

//         let data: Vec<f32> = spectral_differences
//             .iter()
//             .map(|diffs| diffs.iter().sum::<f32>())
//             .collect();

//         OnsetOutput {
//             result: input.stft.set_data(data),
//         }
//     }
// }
