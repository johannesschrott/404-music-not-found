extern crate core;

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;


use ansi_term::Style;
use clap::{Arg, ArgGroup, Command, crate_authors, crate_description, crate_version};

use onset_algo::{HighFrequencyContent, OnsetAlgorithm, OnsetInput};
use statistics::{convolve1D, normalize, vec_mult};
use track::Track;

use crate::onset_algo::SpectralDifference;
use crate::peak_picking::PeakPicker;



mod peak_picking;
mod onset_algo;
mod plot;
mod statistics;
mod track;

static N_ONSET: usize = 4096;
static M_ONSET_SIGNAL_ENVELOPE: usize = 10;

/// Accuracy in seconds of the estimated onsets
static ONSET_ACCURACY: f64 = 50e-3;
/// Accuracy in seconds of the estimated beats
static BEAT_ACCURACY: f64 = 70e-3;
/// Deviation of which the estimated tempo may be different (+ and -)
static TEMPO_DEVIATION: f64 = 0.08;

fn main() {
    #[cfg(target_os = "windows")]
    {
        ansi_term::enable_ansi_support(); // enable the super fancy output on windows!!
    }
    let fancy_name = [
        Style::new().bold().paint("404 - music ").to_string(),
        Style::new().bold().strikethrough().paint("not").to_string(),
        Style::new().bold().paint(" found").to_string(),
    ]
        .join("");

    let arg_matches = Command::new(fancy_name)
        .about(crate_description!())
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .required_unless_present("dir")
                .help("Process a given .wav file")
                .takes_value(true)
                .value_name("FILE PATH"),
        )
        .arg(
            Arg::new("dir")
                .short('d')
                .long("directory")
                .required_unless_present("file")
                .help("Process all .wav files in the directory")
                .takes_value(true)
                .value_name("DIRECTORY PATH"),
        )
        .group(
            ArgGroup::new("source")
                .required(true)
                .args(&["file", "dir"]),
        )
        .get_matches();

    if arg_matches.is_present("file") && !arg_matches.is_present("dir") {
        process_file(Path::new(arg_matches.value_of("file").expect("required")));
        // currently nothing is done with the track
    } else if arg_matches.is_present("dir") && !arg_matches.is_present("file") {
        // Folder processing not implented yet
    }
}

fn process_file(file_path: &Path) {
    let track = Track::from_path(file_path);
    let sample_rate = track.header.sample_rate;

    let onset_input = OnsetInput::from_track(&track);
    let high_frequency = HighFrequencyContent::find_onsets(&onset_input);
    let spectral_difference = SpectralDifference::find_onsets(&onset_input);

    plot::plot(&high_frequency.result.data, "high freq.png");
    plot::plot(&spectral_difference.result.data, "spectr_diff.png");

    let kernel_function = |k: &[f32]| {
        // let neighborhood: Vec<usize> = (0..28).into_iter().chain((37..65).into_iter()).collect();
        // neighborhood.into_iter().map(|x| k[x] * 0.00815).sum::<f32>() +
        //     (k[28] + k[29] + k[35] + k[36]) * 0.03 + (k[30] + k[31] + k[33] + k[34]) * 0.05 + k[32] * 0.16
        (k[0] + k[4]) * (-0.3) + (k[1] + k[3]) * (-0.5) + k[2] * 2.6
    };

    // let output: Vec<f32> = normalize(
    //     &vec_mult(
    //         &vec![
    //             &convolve1D(&high_frequency.result, 5, kernel_function)[..],
    //             &convolve1D(&spectral_difference.result, 5, kernel_function)[..],
    //         ][..],
    //     )[..],
    // );

    let output = spectral_difference.convolve(5, kernel_function);

    plot::plot(&output.result.data, "output.png");

    let peak_picker = PeakPicker {
        local_window_max: 1,
        local_window_mean: 1,
        minimum_distance: 1,
        delta: 0.
    };

    // Compute f measure for our different results:
    println!(
        "{}",
        Style::new().bold().paint("Convolved Output").to_string()
    );
    f_measure_onsets(
        &peak_picker.pick(&output).onset_times(&track).onset_times,
        file_path,
    );
    println!();

    println!(
        "{}",
        Style::new().bold().paint("High Frequency").to_string()
    );
    f_measure_onsets(
        &peak_picker.pick(&high_frequency)
            .onset_times(&track)
            .onset_times,
        file_path,
    );
    println!();

    println!(
        "{}",
        Style::new()
            .bold()
            .paint("Spectral Difference Output")
            .to_string()
    );
    f_measure_onsets(
        &peak_picker.pick(&spectral_difference)
            .onset_times(&track)
            .onset_times,
        file_path,
    );
}

// fn get_onset_times(output: &Vec<f32>, window_size: usize, sample_rate: u32) ->Vec<f64> {
//     // Compute times of peaks
//     let peaks: Vec<bool> = (0..output.len())
//         .into_iter()
//         .map(|i| {
//             return if (i > 0 && i < output.len() - 1) /* checks if index is at border */
//                 && (output[i - 1] <output[i] && output[i] > output[i + 1] ) /* checks if a peak */ {
//                 true
//             } else {
//                 false
//             };
//         })
//         .collect::<Vec<bool>>();

//     let mut onset_times: Vec<f64> = Vec::new();

//     for i in 0..peaks.len() {
//         if peaks[i] {
//             onset_times.push(i as f64 * ((window_size as f64) / (sample_rate as f64)));
//         }
//     }
//     return onset_times;
// }

fn f_measure_onsets(found_onsets: &Vec<f64>, file_path: &Path) {
    let file_string_onsets_gt = [
        file_path.to_str().unwrap().strip_suffix(".wav").unwrap(),
        ".onsets.gt",
    ]
    .join("");

    if !Path::new(&file_string_onsets_gt).exists() {
        // if a onsets.gt file in the same folder exists, do a validation!
        return;
    }
    println!("Validation of Found onsets");
    let gt_file = File::open(Path::new(&file_string_onsets_gt)).unwrap();
    let reader = BufReader::new(gt_file);

    /// Vector containing the true onset times (in seconds!)
    let gt_onsets: Vec<f64> = reader.lines().map(|line| line.expect("Error on parsing line")).map(|line| line.parse::<f64>().unwrap()).collect();


    /// current index in vector of found onsets
    let mut i_found: usize = 0;
    /// current index in vector of gt onsets
    let mut i_gt: usize = 0;

    let mut t_p: usize = 0;
    let t_n: usize = 0; // There are no true negatives!
    let mut f_p: usize = 0;
    let mut f_n: usize = 0;

    if found_onsets.len() == 0 && gt_onsets.len() != 0 {
        println!("No onsets found :( Something may have gone wrong");
        return;
    }
    while i_found < found_onsets.len() && i_gt < gt_onsets.len() {
        if gt_onsets[i_gt] - ONSET_ACCURACY <= found_onsets[i_found]
            && found_onsets[i_found] <= gt_onsets[i_gt] + ONSET_ACCURACY
        {
            // the found onset is within the accuracy border
            t_p += 1;
            i_found += 1;
            i_gt += 1;
        } else if found_onsets[i_found] < gt_onsets[i_gt] - ONSET_ACCURACY {
            f_p += 1;
            i_found += 1;
        } else if gt_onsets[i_gt] + ONSET_ACCURACY < found_onsets[i_found] {
            f_n += 1;
            i_gt += 1;
        }
    }
    if i_gt < gt_onsets.len() {
        f_n += gt_onsets.len() - i_gt;
    }
    let precision: f64 = t_p as f64 / (t_p as f64 + f_p as f64);
    let recall: f64 = t_p as f64 / (t_p as f64 + f_n as f64);
    let f_measure: f64 = 2f64 * (precision * recall) / (precision + recall);

    println!("Precession: {}", precision);
    println!("Recall:     {}", recall);
    println!("F-Measure:  {}", f_measure);
}
