use arima::acf::acf;

use crate::{statistics::WinVec, track::Track};
use crate::constants::*;


#[derive(Copy, Clone)]
pub struct Tempo {
    pub lag: usize,
    pub bpm: f64,
}

pub fn get_tempo(track: &Track, detection_output: &WinVec<f32>) -> (Tempo, Tempo) {
    let mut times: Vec<f64> = Vec::new();
    let mut a_corr: Vec<f64> = Vec::new();

    for &onset_time in detection_output.data.iter() {
        times.push(onset_time as f64);
    }

    let times_slice: &[f64] = times.as_slice();
    match acf(times_slice, None, false) {
        Ok(auto_corr) => a_corr = auto_corr,
        Err(e) => println!("{:?}", e),
    };

    let high = bpm_to_lag(track, detection_output.hop_size, SLOWEST_BPM);
    let low = bpm_to_lag(track, detection_output.hop_size, HIGHEST_BPM);

    let tempo_area = &a_corr[low..high];

    let mut max = 0;
    let mut max2 = 0;

    for (i, x) in tempo_area.iter().enumerate() {
        if tempo_area[max] <= *x {
            max2 = max;
            max = i;
        } else if tempo_area[max2] <= *x {
            max2 = i;
        }
    }

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

pub struct Beats {
    pub beats: Vec<f64>,
}

pub fn get_beats(tempo: Tempo, onset_times: &Vec<f64>, first_beat_index: usize) -> Beats {
    let mut beats: Vec<f64> = Vec::new();

    let beat_period = 1. / tempo.bpm * 60.;

    // println!("{}", beat_period);

    beats.push(onset_times[first_beat_index]);

    let mut last_beat = onset_times[first_beat_index];
    let mut i = first_beat_index + 1;

    while i < onset_times.len() - 2 {
        let next1: f64 = onset_times[i];
        let next2 = onset_times[i + 1];


        // println!("{}, {}, {}", last_beat, next1, next2);

        if (next1 - last_beat) > 1.3 * beat_period {
            last_beat = last_beat + beat_period;
            beats.push(last_beat);
        }

        // Computes the differences of the next beat vs the ideal next beat (--> lower value means closer to ideal next beat)
        let diff1 = (last_beat + beat_period - next1).abs();
        let diff2 = (last_beat + beat_period - next2).abs();
      //  let diff3 = (last_beat + beat_period - next3).abs();


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

fn bpm_to_lag(track: &Track, hop_size: usize, bpm: f64) -> usize {
    let sample_rate = 1. / (track.header.sample_rate as f64);
    let delta = sample_rate * (hop_size as f64);
    let bps = bpm / 60.;
    let beat_periode = 1. / bps;
    (beat_periode / delta) as usize
}

fn lag_to_bpm(track: &Track, hop_size: usize, lag: usize) -> f64 {
    let sample_rate = 1. / (track.header.sample_rate as f64);
    let delta = sample_rate * (hop_size as f64);
    let beat_periode = lag as f64 * delta;
    let bps = 1. / beat_periode;
    bps * 60.
}

// delta = hop_size * sample_rate
// lag = n * delta
// bpm = beats / (60 * seconds)
// bps = bpm / 60
// beat_periode = 1 / bps

// lag = beat_periode / delta ?
