extern crate core;

use std::{env, thread};
use std::fs::{self};
use std::path::Path;
use std::sync::{Arc, Mutex};

use ansi_term::Style;
use beat_tracking::get_tempo;
use clap::{Arg, ArgGroup, ArgMatches, Command, crate_authors, crate_description, crate_version};
use glob::{glob};
use json::JsonValue;

use crate::beat_tracking::{get_beats, Tempo};
use f_measure::{FMeasure, f_measure_beats, f_measure_onsets};
use onset_algorithms::*;
use peak_picking::{OnsetTimes, PeakPicker};
use track::Track;
use constants::*;

mod beat_tracking;
mod f_measure;
mod onset_algorithms;
mod peak_picking;
mod plot;
mod statistics;
mod track;
mod constants;


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

/// Based on the passed arguments, a JSON File for containing the results is written out to the file system
fn handle_output(arg_matches: ArgMatches, output: (Option<FMeasure>, JsonValue)) {
    if let Some(f_measure) = output.0 {
        println!("{}", Style::new().bold().paint("F-Measure").to_string());

        println!("Precession: {}", f_measure.precision);
        println!("Recall:     {}", f_measure.recall);
        println!("F-Measure:  {}", f_measure.f_measure);
    } else {
        println!("F-Measure was not computed, due to missing ground truth data or an error occurred during computation.");
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


    /***********************
     * SPECTRAL DIFFERENCE *
     ***********************/
    // As the onset results computed by spectral difference were not good enough,
    // spectral difference got excluded from our final submission.

    // let spectral_small = SpectralDifference.find_onsets(&onset_input_small);
    // let spectral_big = SpectralDifference.find_onsets(&onset_input_big);

    /********
     * LFSF *
     ********/
    let lfsf_small = LFSF { log_lambda: 0.7 }.find_onsets( &onset_input_small);
    let lfsf_big = LFSF { log_lambda: 0.7 }.find_onsets( &onset_input_big);

    /******************
     * HIGH FREQUENCY *
     ******************/
    // As the onset results computed by the high frequency method were not good enough,
    // spectral difference got excluded from our final submission.

    // let high_frequency: OnsetOutput = HighFrequencyContent::find_onsets(&onset_input);

    /****************
     * PEAK PICKING *
     ****************/

    let peak_picker_small = PeakPicker {
        local_window_max: 7,
        local_window_mean: 7, // the higher, the lower the recall but precision slightly increases
        minimum_distance: 3,
        delta: 0.1, // must be relatively tiny
    };

    let peak_picker_big = PeakPicker {
        local_window_max: 3,
        local_window_mean: 3, // the higher, the lower the recall but precision slightly increases
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



    let combined_onset = combine_onsets(
        ENSEMBLE_NEEDED_SCORE,
        vec![
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
        ],
    );

    plot::plot32(&lfsf_small.result.data, "lfsf_small.png");

    // try to compute beat tracking
    let tempo = get_tempo(&track, &lfsf_small.result);

    let tempo_for_beats: Tempo  ;
    if tempo.0.bpm < tempo.1.bpm {
        tempo_for_beats = tempo.0;
    } else { tempo_for_beats = tempo.1 }
    let beats = get_beats(tempo_for_beats, &peak_picker_small
        .pick(&lfsf_small)
        .onset_times(&track)
        .onset_times, peak_picker_small.pick(&lfsf_small).highest_first_beat_index);

    //let beats = get_beats(tempo_for_beats, &combined_onset);
    /**************
    ** Fill JSON **
    **************/

    // Create JSON Part for current file
    let mut file_json = json::JsonValue::new_object();
    file_json["onsets"] = json::JsonValue::new_array();
    file_json["beats"] = json::JsonValue::new_array();
    file_json["tempo"] = json::JsonValue::new_array();

    let onsets_json = &mut file_json["onsets"];

    for onset_time in combined_onset.iter() {
        onsets_json.push(onset_time.to_owned()).unwrap();
    }
    let beats_json = &mut file_json["beats"];

    for beat_time in beats.beats.iter() {
        beats_json.push(beat_time.to_owned()).unwrap();
    }

    // Push the found tempos in ascending order to the JSON
    if tempo.0.bpm < tempo.1.bpm {
        let _ = file_json["tempo"].push(tempo.0.bpm);
        let _ = file_json["tempo"].push(tempo.1.bpm);
    } else {
        let _ = file_json["tempo"].push(tempo.1.bpm);
        let _ = file_json["tempo"].push(tempo.0.bpm);
    }

    // return (f_measure_onsets(&combined_onset, file_path), file_json);
    return (f_measure_beats(&beats.beats, file_path), file_json);
}

fn process_folder(folder_path: &Path) -> (Option<FMeasure>, json::JsonValue) {
    let glob_pattern = [folder_path.to_str().unwrap(), "/*.wav"].join("");

    // create empty json file for submission
    let mut overall_json_result = json::JsonValue::new_object();

    let files = glob(&glob_pattern).unwrap();
    let file_count_ref = Arc::new(Mutex::new(0));
    let done_count_ref = Arc::new(Mutex::new(0));

    /*
    Create a list of file names to be processed
     */
    let mut file_names: Vec<String> = Vec::new();

    files.for_each(|file| {
        match file {
            Ok(file_path) => {
                //       file_names.push(file_path.file_stem().unwrap().to_str().unwrap().to_owned());
                file_names.push(file_path.as_path().to_str().unwrap().to_owned())
            }
            Err(e) => println!("{:?}", e),
        }
    });

    /*
    Create threads in chunks through splitting the file names list
     */

    let mut chunks = Vec::new();

    for chunk in file_names.to_owned().chunks(NO_THREADS) {
        chunks.push(chunk.to_owned());
    }

    let mut f_measures = Vec::new();

    for chunk in chunks {
        let mut file_processings = Vec::new();

        // for each track create a thread
        for file_name in chunk {
            let file_count_ref_cloned = file_count_ref.clone();
            let mut file_count = file_count_ref_cloned.lock().unwrap();
            *file_count += 1;
            //     match music_file {
            //  Ok(file_path) => {
            //    let file_name = file_path.file_stem().unwrap().to_str().unwrap().to_owned();
            let local_state = (file_count_ref.clone(), done_count_ref.clone());
            let file_processing = thread::spawn(move || {
                let file_path = Path::new(&file_name);

                let name = file_path.file_stem().unwrap().to_str().unwrap().to_owned();

                let output = (name, process_file(file_path));

                let mut done_count = local_state.1.lock().unwrap();
                *done_count += 1;

                let file_count = local_state.0.lock().unwrap();

                println!("{} of {} done", done_count, *file_count);
                output
            });
            file_processings.push(file_processing);
            //       }
            //      Err(e) => println!("{:?}", e),
            //}
        }
        // join the threads and put results into json
        for file_processing in file_processings {
            let (filename, (measure, json_res)) = file_processing.join().unwrap();
            overall_json_result[filename] = json_res;
            f_measures.push(measure);
        }
    }

    let mut precision = 0.;
    let mut recall = 0.;
    let mut score = 0.;
    let mut count = 0;

    for f_measure_ in f_measures {
        if let Some(f_measure) = f_measure_ {
            precision += f_measure.precision;
            recall += f_measure.recall;
            score += f_measure.f_measure;
            count += 1;
        }
    }

    let count_f = count as f64;

    (
        if count > 0 {
            Some(FMeasure {
                precision: precision / count_f,
                recall: recall / count_f,
                f_measure: score / count_f,
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
