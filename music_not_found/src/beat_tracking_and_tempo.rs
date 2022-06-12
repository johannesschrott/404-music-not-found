use arima::acf::acf;

use crate::{helpers::WinVec, track::Track};
use crate::constants::*;

// this is the auto-correlation-function

#[derive(Copy, Clone)]
pub struct Tempo {
    pub lag: usize,
    pub bpm: f64,
}

/// This is the tempo estimation function
pub fn get_tempo(track: &Track, detection_output: &WinVec<f32>) -> (Tempo, Tempo) {
    let mut times: Vec<f64> = Vec::new();
    let mut a_corr: Vec<f64> = Vec::new();

    // Convert the found onset times from f32 to 64
    for &onset_time in detection_output.data.iter() {
        times.push(onset_time as f64);
    }

    let times_slice: &[f64] = times.as_slice(); // The auto-correlation function does not like vectors
    match acf(times_slice, None, false) {
        Ok(auto_corr) => a_corr = auto_corr,
        Err(e) => println!("{:?}", e),
    };

    // For the lowest and highest possible BPM compute its lag (= nr of STFT vectors between two beats)
    let high = bpm_to_lag(track, detection_output.hop_size, SLOWEST_BPM);
    let low = bpm_to_lag(track, detection_output.hop_size, HIGHEST_BPM);


    // Crop the autocorrelated signal to the area between lowest lag (-> BPM 200) and highest lag (-> BPM 60)
    let tempo_area = &a_corr[low..high];

    let mut max = 0;
    let mut max2 = 0;

    // Find the indices of the two highest values in the cropped auto-correlation
    for (i, x) in tempo_area.iter().enumerate() {
        if tempo_area[max] <= *x {
            max2 = max;
            max = i;
        } else if tempo_area[max2] <= *x {
            max2 = i;
        }
    }

    // As the lag has been cropped, re-add the cropped part in order to convert the found maxima
    // correctly to BPM
    (
        Tempo {
            lag: low + max,
            bpm: lag_to_bpm(track, detection_output.hop_size, low + max),
        },
        Tempo {
            lag: low + max2,
            bpm: lag_to_bpm(track, detection_output.hop_size, low + max2),
        },
    )
}

/// Data structure containing found beats
pub struct Beats {
    /// A list of beat times (in seconds)
    pub beats: Vec<f64>,
}

/// The beat detection function
pub fn get_beats(tempo: Tempo, onset_times: &Vec<f64>, first_beat_index: usize) -> Beats {
    let mut beats: Vec<f64> = Vec::new();

    let beat_period = 1. / tempo.bpm * 60.; // Compute the average time duration between two beats

    beats.push(onset_times[first_beat_index]); // The first local maxima of the onsets is set as the first beat.

    let mut last_beat = onset_times[first_beat_index];
    let mut i = first_beat_index + 1; // set the index of the onset of the first beat (starting point for iteration over all onset times)

    // Iterate over the onset times
    while i < onset_times.len() - 2 {
        // take the next two onsets following the last identified beat and treat them as "next beats"
        let next1: f64 = onset_times[i];
        let next2 = onset_times[i + 1];
        // The ideal next beat would be the last beat + beat periodicity

        // Through experimenting we found out the following: If the distance to the next ideal beat
        // is more than 1.3*beat_periodiciy, simply compute an "artificial" beat at the ideal next beat
        if (next1 - last_beat) > 1.3 * beat_period {
            last_beat = last_beat + beat_period;
            beats.push(last_beat);
        }

        // Computes the differences of the next beat vs the ideal next beat (--> lower value means closer to ideal next beat)
        // Consqeuently, the next beat closer to the ideal next beat is taken
        let diff1 = (last_beat + beat_period - next1).abs();
        let diff2 = (last_beat + beat_period - next2).abs();

        if diff1 < diff2 {
            beats.push(next1);
            last_beat = next1;
        } else {
            beats.push(next2);
            last_beat = next2;
            i += 1;
        }
        i += 1;
    }

    Beats { beats }
}

/// Convert BPM into a number of frequency vectors that lay between two beats
fn bpm_to_lag(track: &Track, hop_size: usize, bpm: f64) -> usize {
    let sample_rate = 1. / (track.header.sample_rate as f64);
    let delta = sample_rate * (hop_size as f64);
    let bps = bpm / 60.;
    let beat_periode = 1. / bps;
    (beat_periode / delta) as usize
}

/// the inverse of bpm_to_lag
fn lag_to_bpm(track: &Track, hop_size: usize, lag: usize) -> f64 {
    let sample_rate = 1. / (track.header.sample_rate as f64);
    let delta = sample_rate * (hop_size as f64);
    let beat_periode = lag as f64 * delta;
    let bps = 1. / beat_periode;
    bps * 60.
}