use std::{
    cmp::{max, min},
    iter::repeat,
};

use crate::{onset_algo::OnsetOutput, statistics::WinVec, track::Track};

pub struct Peaks {
    pub peaks: WinVec<bool>,
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
        let output = &onset_output.result.data;

        let mut peaks: Vec<bool> = repeat(false).take(output.len()).collect();

        let mean_window = |mean_left, mean_right| {
            output[mean_left..mean_right].iter().sum::<f32>() / (mean_right - mean_left) as f32
        };
        let max_window = |max_left, max_right| {
            output[max_left..max_right]
                .iter()
                .cloned()
                .fold(0. / 0., f32::max)
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

            peaks[i] = output[i - 1] < output[i] && output[i] > output[i + 1]  // checks if a peak
            // implement adaptive peak picking
                && minimum_distance(i, &peaks)
                && output[i] >= mean_window(mean_left, mean_right) + self.delta
                && output[i] >= max_window(max_left, max_right);
        }
        Peaks {
            peaks: onset_output.result.set_data(peaks),
        }
    }
}

impl Peaks {
    pub fn onset_times(&self, track: &Track) -> OnsetTimes {
        let mut onset_times: Vec<f64> = Vec::new();

        for i in 0..self.peaks.data.len() {
            if self.peaks.data[i] {
                onset_times.push(
                    i as f64
                        * (self.peaks.hop_size as f64 / (track.header.sample_rate as f64)),
                );
            }
        }
        OnsetTimes { onset_times }
    }
}
