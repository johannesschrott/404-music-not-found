use std::cmp::Ordering;

use rustfft::num_complex::Complex;
use rustfft::num_traits::abs;

use crate::constants::*;
use crate::helpers::{stft, WinVec, zeroes};
use crate::track::Track;

/// Data structure holding the samples of a track and its STFT
pub struct OnsetInput {
    pub samples: Vec<f32>,
    pub sampling_rate: u32,
    pub stft: WinVec<Vec<Complex<f32>>>,
}

impl OnsetInput {
    /// Extract samples from a track and compute its stft using the given window- and hop-size.
    pub fn from_track(track: &Track, window_size: usize, hop_size: usize) -> OnsetInput {
        OnsetInput {
            samples: track.samples.to_owned(),
            sampling_rate: track.header.sample_rate.to_owned(),
            stft: stft(&track.samples, window_size, hop_size),
        }
    }
}

/// Data structure holding the values determined by the onset detection function (not holding onsets!)
pub struct OnsetOutput {
    pub result: WinVec<f32>,
}

/// Defines an interface for the onset algorithms
pub trait OnsetAlgorithm {
    fn find_onsets(&self, input: &OnsetInput) -> OnsetOutput;
}


/******************
 * HIGH FREQUENCY *
 ******************/
pub struct HighFrequencyContent;

impl HighFrequencyContent {
    fn weights(size: usize) -> Vec<f32> {
        let size_f32: f32 = size as f32;
        (0..size)
            .map(|a| a as f32 / size_f32)
            .collect()
    }
}

impl OnsetAlgorithm for HighFrequencyContent {
    fn find_onsets(&self, input: &OnsetInput) -> OnsetOutput {
        let weights = HighFrequencyContent::weights(input.stft.window_size);
        let data: WinVec<f32> = input.stft.map(|data| {
            data.iter()
                .map(|single_fft| {
                    // The following is the formula from L04 slide 23
                    let s: f32 = single_fft
                        .iter()
                        .zip(weights.iter())
                        .map(|(v, w)| v.norm_sqr() * w)
                        .sum();
                    s / (input.stft.window_size as f32)
                })
                .collect()
        });
        OnsetOutput { result: data }
    }
}

/***********************
 * SPECTRAL DIFFERENCE *
 ***********************/

pub struct SpectralDifference;

impl SpectralDifference {}

impl OnsetAlgorithm for SpectralDifference {
    fn find_onsets(&self, input: &OnsetInput) -> OnsetOutput {
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

/********
 * LFSF *
 ********/

pub struct LFSF {
    pub log_lambda: f32,
}

impl LFSF {
    fn apply_filterbank(sampling_rate: u32, data: &[Vec<Complex<f32>>]) -> Vec<Vec<f32>> {
        let filterbank: Vec<Vec<f32>> = mel_filter::mel(
            sampling_rate as usize,
            data[0].len(),
            Some(MEL_BANDS),
            None,
            None,
            false,
            mel_filter::NormalizationFactor::One,
        );

        data.iter()
            .map(|frame| {
                filterbank
                    .iter()
                    .map(|mel_frame| {
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
    fn find_onsets(&self, input: &OnsetInput) -> OnsetOutput {
        let raw_data = &input.stft.data;

        let data = &self.apply_log(LFSF::apply_filterbank(input.sampling_rate, &raw_data[..]));

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
            // This compoutes the LFSF detection function (see slide 61 L04.pdf)
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


/***********
 * HELPERS *
 ***********/

/// Combines onset times (after peak picking) from different algorithms. Using the needed_score,
/// someone can determine how much of the passed onset results need an onset to have found in
/// order to count it as one.
pub fn combine_onsets(needed_score: f64, onsets: Vec<(f64, Vec<f64>)>) -> Vec<f64> {
    let mut combined_values = Vec::new();

    for (score, vec) in onsets {
        for x in vec {
            combined_values.push((x, score));
        }
    }

    combined_values.sort_by(|(a, _), (b, _)| {
        if b > a {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    });

    let mut combined = Vec::new();

    let mut time = 0.;
    let mut i = 0;

    while i < combined_values.len() {
        let (t, _) = combined_values[i];
        if t < time {
            i += 1;
            continue;
        }

        time = t;

        let mut scores = Vec::new();
        while i < combined_values.len() && combined_values[i].0 - time <= ONSET_ACCURACY {
            scores.push(combined_values[i].1);
            i += 1;
        }

        if scores.into_iter().sum::<f64>() > needed_score {
            combined.push(time);
        }
    }

    combined
}
