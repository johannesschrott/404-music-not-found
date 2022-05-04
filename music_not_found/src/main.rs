extern crate core;

use std::{env, thread};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use ansi_term::Style;
use clap::{Arg, ArgGroup, Command, crate_authors, crate_description, crate_version};
use glob::{glob};
use json::JsonValue;

use onset_algo::{HighFrequencyContent, OnsetAlgorithm, OnsetInput};
use track::Track;

use crate::onset_algo::{OnsetOutput, SpectralDifference};
use crate::peak_picking::PeakPicker;

mod peak_picking;
mod onset_algo;
mod plot;
mod statistics;
mod track;


/// Accuracy in seconds of the estimated onsets
static ONSET_ACCURACY: f64 = 50e-3;
/// Accuracy in seconds of the estimated beats
static BEAT_ACCURACY: f64 = 70e-3;
/// Deviation of which the estimated tempo may be different (+ and -)
static TEMPO_DEVIATION: f64 = 0.08;

/// Main entrance point for CLI Application
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
    } else if arg_matches.is_present("dir") && !arg_matches.is_present("file") {
        process_folder(Path::new(arg_matches.value_of("dir").expect("required")));
    }
}

fn process_file(file_path: &Path) -> JsonValue {
    let track = Track::from_path(file_path);
    //let sample_rate = track.header.sample_rate;

    let onset_input = OnsetInput::from_track(&track);
    /*let sd_thread = thread::spawn(move || {

    });
    let hf_thread = thread::spawn(move || {

    });*/

    let spectral_difference = SpectralDifference::find_onsets(&onset_input);

    let high_frequency: OnsetOutput = HighFrequencyContent::find_onsets(&onset_input);

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
        delta: 0.,
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

    // Create JSON Part for current file
    let mut file_json = json::JsonValue::new_object();
    file_json["onsets"] = json::JsonValue::new_array();
    file_json["beats"] = json::JsonValue::new_array();
    file_json["tempo"] = json::JsonValue::new_array();

    // Fill JSON with onsets
    for onset_time in &peak_picker.pick(&spectral_difference).onset_times(&track).onset_times {
        file_json["onsets"].push(onset_time.to_owned());
    }


    return file_json;
}

fn process_folder(folder_path: &Path) {
    let glob_pattern = [folder_path.to_str().unwrap(), "/*.wav"].join("");

    // create empty json file for submission
    let mut overall_json_result = json::JsonValue::new_object();

    let mut file_processings = Vec::new();


    // for each track create a thread
    for music_file in glob(&glob_pattern).unwrap() {
        match music_file {
            Ok(file_path) => {
                let file_name = file_path.file_stem().unwrap().to_str().unwrap().to_owned();
                println!("{:?}", file_path.display());
                let file_processing = thread::spawn(move || {
                    (file_name, process_file(file_path.as_path()))
                });
                file_processings.push(file_processing)
            }
            Err(e) => println!("{:?}", e)
        }
    }

    // join the threads and put results into json
    for file_processing in file_processings {
        let (filename, json_res) = file_processing.join().unwrap();
        overall_json_result[filename] = json_res;
    }

    println!("{}", overall_json_result.dump());
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

fn f_measure_onsets(found_onsets: &Vec<f64>, file_path: &Path) -> (f64, f64, f64) {
    let file_string_onsets_gt = [
        file_path.to_str().unwrap().strip_suffix(".wav").unwrap(),
        ".onsets.gt",
    ]
        .join("");

    if !Path::new(&file_string_onsets_gt).exists() {
        // if a onsets.gt file in the same folder exists, do a validation!
        return (0 as f64, 0 as f64, 0 as f64);
    }
    println!("Validation of Found onsets");
    let gt_file = File::open(Path::new(&file_string_onsets_gt)).unwrap();
    let reader = BufReader::new(gt_file);

    // Vector containing the true onset times (in seconds!)
    let gt_onsets: Vec<f64> = reader.lines().map(|line| line.expect("Error on parsing line")).map(|line| line.parse::<f64>().unwrap()).collect();


    // current index in vector of found onsets
    let mut i_found: usize = 0;
    // current index in vector of gt onsets
    let mut i_gt: usize = 0;

    let mut t_p: usize = 0;
    let t_n: usize = 0; // There are no true negatives!
    let mut f_p: usize = 0;
    let mut f_n: usize = 0;

    if found_onsets.len() == 0 && gt_onsets.len() != 0 {
        println!("No onsets found :( Something may have gone wrong");
        return (0 as f64, 0 as f64, 0 as f64);
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

    return (precision, recall, f_measure);
}
