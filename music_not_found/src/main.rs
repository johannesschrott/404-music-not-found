extern crate core;

use std::{env, thread};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::{Arc, Mutex};

use ansi_term::Style;
use clap::{Arg, ArgGroup, ArgMatches, Command, crate_authors, crate_description, crate_version};
use glob::glob;
use json::JsonValue;

use f_meausure::FMeasure;
use onset_algo::{HighFrequencyContent, OnsetAlgorithm, OnsetInput};
use track::Track;

use crate::f_meausure::f_measure_onsets;
use crate::onset_algo::{LFSF, OnsetOutput, SpectralDifference};
use crate::peak_picking::PeakPicker;

mod f_meausure;
mod onset_algo;
mod peak_picking;
mod plot;
mod statistics;
mod track;

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
        .arg(
            Arg::new("competition")
                .short('c')
                .long("competition")
                .help("Writes Competition JSON")
                .takes_value(true)
                .value_name("JSON OUTPUT PATH"),
        )
        .group(
            ArgGroup::new("source")
                .required(true)
                .args(&["file", "dir"]),
        )
        .get_matches();

    if arg_matches.is_present("file") && !arg_matches.is_present("dir") {
        let output = process_file(Path::new(arg_matches.value_of("file").expect("required")));
        handle_output(arg_matches, output);
    } else if arg_matches.is_present("dir") && !arg_matches.is_present("file") {
        let output = process_folder(Path::new(arg_matches.value_of("dir").expect("required")));
        handle_output(arg_matches, output);
    }
}

fn handle_output(arg_matches: ArgMatches, output: (Option<FMeasure>, JsonValue)) {
    println!("{}", Style::new().bold().paint("F Measure").to_string());

    if let Some(f_measure) = output.0 {
        println!("Precession: {}", f_measure.precision);
        println!("Recall:     {}", f_measure.recall);
        println!("F-Measure:  {}", f_measure.score);
    } else {
        println!("No F_Measure Data");
    }
    if arg_matches.is_present("competition") {
        let path = arg_matches
            .value_of("competition")
            .expect("path for competition flag is needed");
        match fs::write(path, output.1.dump()) {
            Ok(()) => (),
            Err(error) => println!("{}", error),
        }
    }
}

fn process_file(file_path: &Path) -> (Option<FMeasure>, JsonValue) {
    let track = Track::from_path(file_path);

    let onset_input = OnsetInput::from_track(&track);

    let spectral_difference = SpectralDifference::find_onsets(&onset_input);

    let lfsf = LFSF::find_onsets(&onset_input);
    // let high_frequency: OnsetOutput = HighFrequencyContent::find_onsets(&onset_input);

    let peak_picker = PeakPicker {
        local_window_max: 1,
        local_window_mean: 5, // the higher, the lower the recall but precission slightly increases
        minimum_distance: 2,
        delta: 0.0, // must be relatively tiny
    };

    // println!(
    //     "{}",
    //     Style::new().bold().paint("High Frequency").to_string()
    // );
    // f_measure_onsets(
    //     &peak_picker.pick(&high_frequency)
    //         .onset_times(&track)
    //         .onset_times,
    //     file_path,
    // );
    // println!();

    // println!(
    //     "{}",
    //     Style::new()
    //         .bold()
    //         .paint("Spectral Difference Output")
    //         .to_string()
    // );
    let f_measure = f_measure_onsets(
        &peak_picker
            .pick(&lfsf)
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
    for onset_time in &peak_picker
        .pick(&spectral_difference)
        .onset_times(&track)
        .onset_times
    {
        file_json["onsets"].push(onset_time.to_owned());
    }

    return (f_measure, file_json);
}

fn process_folder(folder_path: &Path) -> (Option<FMeasure>, json::JsonValue) {
    let glob_pattern = [folder_path.to_str().unwrap(), "/*.wav"].join("");

    // create empty json file for submission
    let mut overall_json_result = json::JsonValue::new_object();

    let mut file_processings = Vec::new();

    let files = glob(&glob_pattern).unwrap();
    let file_count_ref = Arc::new(Mutex::new(0));
    let done_count_ref = Arc::new(Mutex::new(0));
    // for each track create a thread
    for music_file in files {
        let file_count_ref_cloned = file_count_ref.clone();
        let mut file_count = file_count_ref_cloned.lock().unwrap();
        *file_count += 1;
        match music_file {
            Ok(file_path) => {
                let file_name = file_path.file_stem().unwrap().to_str().unwrap().to_owned();
                let local_state = (file_count_ref.clone(), done_count_ref.clone());
                let file_processing = thread::spawn(move || {
                    let output = (file_name, process_file(file_path.as_path()));

                    let mut done_count = local_state.1.lock().unwrap();
                    *done_count += 1;

                    let file_count = local_state.0.lock().unwrap();

                    println!("{} of {} done", done_count, *file_count);
                    output
                });
                file_processings.push(file_processing);
            }
            Err(e) => println!("{:?}", e),
        }
    }

    let mut f_measures = Vec::new();

    // join the threads and put results into json
    for file_processing in file_processings {
        let (filename, (measure, json_res)) = file_processing.join().unwrap();
        overall_json_result[filename] = json_res;
        f_measures.push(measure);
    }
    let mut precision = 0.;
    let mut recall = 0.;
    let mut score = 0.;
    let mut count = 0;

    for f_measure_ in f_measures {
        if let Some(f_measure) = f_measure_ {
            precision += f_measure.precision;
            recall += f_measure.recall;
            score += f_measure.score;
            count += 1;
        }
    }

    let count_f = count as f64;

    (
        if count > 0 {
            Some(FMeasure {
                precision: precision / count_f,
                recall: recall / count_f,
                score: score / count_f,
            })
        } else {
            None
        },
        overall_json_result,
    )
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
