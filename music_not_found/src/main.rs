extern crate core;

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::{env, thread};

use ansi_term::Style;
use clap::{crate_authors, crate_description, crate_version, Arg, ArgGroup, ArgMatches, Command};
use glob::glob;
use json::JsonValue;

use f_meausure::{combine_onsets, FMeasure};
use onset_algo::{HighFrequencyContent, OnsetAlgorithm, OnsetInput};
use track::Track;
use crate::beat_tracking::get_beats;

use crate::f_meausure::{f_measure_beats, f_measure_onsets};
use crate::onset_algo::{OnsetOutput, SpectralDifference, LFSF};
use crate::peak_picking::PeakPicker;
use crate::statistics::WinVec;

mod f_meausure;
mod onset_algo;
mod optimize;
mod peak_picking;
mod plot;
mod statistics;
mod track;
mod beat_tracking;

/// Accuracy in seconds of the estimated beats
static BEAT_ACCURACY: f64 = 70e-3;
/// Deviation of which the estimated tempo may be different (+ and -)
static TEMPO_DEVIATION: f64 = 0.08;

const ENSEMBLE_NEEDED_SCORE: f64 = 1.;

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

    let onset_input_big = OnsetInput::from_track(&track, 2048, 1024);
    let onset_input_small = OnsetInput::from_track(&track, 1024, 441);

    //let spectral_small = SpectralDifference.find_onsets(&track, &onset_input_small);
    //let spectral_big = SpectralDifference.find_onsets(&track, &onset_input_big);

    let lfsf_small = LFSF { log_lambda: 0.7 }.find_onsets(&track, &onset_input_small);
    let lfsf_big = LFSF { log_lambda: 0.7 }.find_onsets(&track, &onset_input_big);

    // let high_frequency: OnsetOutput = HighFrequencyContent::find_onsets(&onset_input);

    let peak_picker_small = PeakPicker {
        local_window_max: 7,
        local_window_mean: 7, // the higher, the lower the recall but precission slightly increases
        minimum_distance: 3,
        delta: 0.1, // must be relatively tiny
    };

    let peak_picker_big = PeakPicker {
        local_window_max: 3,
        local_window_mean: 3, // the higher, the lower the recall but precission slightly increases
        minimum_distance: 1,
        delta: 0.1, // must be relatively tiny
    };

    // let f_measure_spectral_small = f_measure_onsets(
    //     &peak_picker_small.pick(&spectral_small).onset_times(&track).onset_times,
    //     file_path,
    // );
    let f_score_spectral_small = 0.6785112079439675;

    // let f_measure_spectral_big = f_measure_onsets(
    //     &peak_picker_big.pick(&spectral_big).onset_times(&track).onset_times,
    //     file_path,
    // );
    let f_score_spectral_big = 0.6935366986327169;

    // let f_measure_lfsf_small = f_measure_onsets(
    //     &peak_picker_small.pick(&lfsf_small).onset_times(&track).onset_times,
    //     file_path,
    // );
    let f_score_lfsf_small = 0.7216659749653946;

    // let f_measure_lfsf_big = f_measure_onsets(
    //     &peak_picker_big.pick(&lfsf_big).onset_times(&track).onset_times,
    //     file_path,
    // );
    let f_score_lfsf_big = 0.757551539129664;

    // Create JSON Part for current file
    let mut file_json = json::JsonValue::new_object();
    file_json["onsets"] = json::JsonValue::new_array();
    file_json["beats"] = json::JsonValue::new_array();
    file_json["tempo"] = json::JsonValue::new_array();

    let combined_onset = combine_onsets(ENSEMBLE_NEEDED_SCORE, vec![
        (
            f_score_lfsf_small,
            peak_picker_small
                .pick(&lfsf_small)
                .onset_times(&track)
                .onset_times,
        ),
        (
            f_score_lfsf_big,
            peak_picker_big
                .pick(&lfsf_big)
                .onset_times(&track)
                .onset_times,
        ),
       /* (
            f_score_spectral_small,
            peak_picker_small
                .pick(&spectral_small)
                .onset_times(&track)
                .onset_times,
        ),
        (
            f_score_spectral_big,
            peak_picker_big
                .pick(&spectral_big)
                .onset_times(&track)
                .onset_times,
        ),*/
    ]);

    // try to compute beat tracking
    let raw_beats = get_beats(&lfsf_small.result.data);

    let beats = raw_beats.iter().to_owned().map(|&f| f as f32).collect::<Vec<f32>>();

    let peak_picker_beats = PeakPicker {
        local_window_max: 0,
        local_window_mean: 0, // the higher, the lower the recall but precission slightly increases
        minimum_distance: 0,
        delta: 0.00, // must be relatively tiny
    };
    let beats = OnsetOutput {
        result: WinVec {
            data: beats,
            window_size: lfsf_small.result.window_size,
            hop_size: lfsf_small.result.hop_size
        }
    };

    let combined_beat = combine_onsets(ENSEMBLE_NEEDED_SCORE, vec![
        (
            f_score_lfsf_small,
            peak_picker_beats
                .pick(&beats)
                .onset_times(&track)
                .onset_times,
        )
        ]);
    // Fill JSON with onsets
    for onset_time in combined_onset.iter() {
        file_json["onsets"].push(onset_time.to_owned());
    }
    for beat_time in combined_beat.iter() {
        file_json["beats"].push(beat_time.to_owned());
    }
   // return (f_measure_onsets(&combined_onset, file_path), file_json);
    return (f_measure_beats(&combined_beat, file_path), file_json);
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
