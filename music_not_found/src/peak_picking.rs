use std::{
    cmp::{max, min},
    iter::repeat,
};

use crate::{helpers::WinVec, onset_algorithms::OnsetOutput, track::Track};

pub struct OnsetTimes {
    pub onset_times: Vec<f64>,
    pub highest_first_beat: usize,
}

pub struct Peaks {
    /// Vector of all of the length of the input signal; values at the indices of onsets have value true; all others false
    pub peaks: WinVec<bool>,
    /// Index of the first local maxima among the onsets
    pub highest_first_beat_index: usize,
}

/// Structure for the PeakPicking parameters, according to LFSF Peak Picking (Slide L04 62)
pub struct PeakPicker {
    /// == w1 == w2
    pub local_window_max: usize,

    /// == w3 == w4
    pub local_window_mean: usize,

    pub delta: f32,

    /// == w5
    pub minimum_distance: usize,
}

/// PeakPicking implementation (Slide L04 62)
impl PeakPicker {
    pub fn pick(&self, onset_output: &OnsetOutput) -> Peaks {
        // Compute times of peaks
        let output = &onset_output.result.data;

        // Initialize the output vector
        let mut peaks: Vec<bool> = repeat(false).take(output.len()).collect();

        // Function that computes the mean of inside a window
        let mean_window = |mean_left, mean_right| {
            output[mean_left..mean_right].iter().sum::<f32>() / (mean_right - mean_left) as f32
        };
        // Function that computes the maximum of inside a window
        let max_window = |max_left, max_right| {
            output[max_left..max_right]
                .iter()
                .cloned()
                .fold(0. / 0., f32::max)
        };

        // Fucntion that checks if two peaks have a minimum distance
        let minimum_distance = |i, peaks: &[bool]| {
            let v = &peaks[max(i, self.minimum_distance) - self.minimum_distance..i]
                .iter()
                .any(|&f| f);
            !v
        };

        // Let the three functions iterate over the output of the onset detection function
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


        // In the following, the index of the onset which is the first local maxima (according to its onset detection function value) of all onsets is determined. This necessary for proper beat detection.

        let peaks_with_values = peaks.clone().into_iter().zip(output.into_iter()).filter(|&val| val.0 == true).collect::<Vec<(bool, &f32)>>();

        let mut highest_first_beat_index: i32 = -1;
        let mut i = 0;

        while highest_first_beat_index == -1 {
            if peaks_with_values[i].1 > peaks_with_values[i + 1].1 {
                highest_first_beat_index = i as i32;
            } else { i += 1 }
        }

        Peaks {
            peaks: onset_output.result.set_data(peaks),
            highest_first_beat_index: highest_first_beat_index as usize,
        }
    }
}

impl Peaks {
    /// Computes times for peaks that are represented in a bool vector
    pub fn onset_times(&self, track: &Track) -> OnsetTimes {
        let mut onset_times: Vec<f64> = Vec::new();

        for i in 0..self.peaks.data.len() {
            if self.peaks.data[i] {
                onset_times.push(
                    (i as f64 + 1.5)
                        * (self.peaks.hop_size as f64 / (track.header.sample_rate as f64)),
                );
            }
        }

        let highes_first_beat = self.highest_first_beat_index; // necessary for beat detection
        OnsetTimes { onset_times, highest_first_beat: highes_first_beat }
    }
}
