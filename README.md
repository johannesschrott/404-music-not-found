# 404 - music ~~not~~ found

Onset detection, tempo estimation, and beat detection - developed as part of the Special Topic "Audio and Music
Processing"
at the Johannes Kepler University Linz in summer semester 2022.

Using the [Rust](https://www.rust-lang.org/) programming language, [Simon Reitinger](https://github.com/Simre1)
and [Johannes Schrott](https://github.com/johannesschrott) created a commandline tool that is capabale of onset
detection, tempo estimation, and beat detection.
In the following, an overview on the project structure, a guideline to the compilation, and usage information is given
before the used methods for each detection and estimation function are described. Generally, all algorithms in this
project
do not include machine learning.

## Structure, Compilation, and Usage of the Project

Project Structure:

* :file_folder: `music_not_found`: Folder containing the Rust Project
    * :page_facing_up: `Cargo.toml`: contains metadata of the Rust project + list of dependencies and a short
      description
      why a dependency is used.
    * :file_folder: `src`: contains all Rust source files
        * :page_facing_up: `beat_tracking_and_tempo.rs`: contains the tempo estimation and beat tracking functions
        * :page_facing_up: `constants.rs`: various constants used across the whole project. Each constant features a
          short documentation comment.
        * _page_facing_up: `f_measure.rs`: Contains functions for F-Measure computation for onsets and beats.
        * :page_facing_up: `helpers.rs`: some useful functions and structures that are used trough out the whole
          project. E.g, the STFT.
        * :page_facing_up: `main.rs`:CLI entry point, managing file processing and folder processing, JSON generation
        * :page_facing_up: `onset_algorithms.rs`: Contains the implementation of LFSF, Spectral Difference and High Frequency Content
        * :page_facing_up: `peak_picking.rs`: Realisation of LFSF Peak Picking
        * :page_facing_up: `plot.rs`: provides functions for plotting float vectors into PNG files
        * :page_facing_up: `track.rs`: reads WAV files and provides a data structure for their content (samples as well
          as file header)

After having installed `rustc` and `cargo`, open the `music_not_found` folder. Our project can be compiled with:
`cargo build --release`
Keep in mind not to forget the `--release` flag since it greatly increases the performance of our application.

To print out an overview over all options, run the program with the `-h` argument:
`cargo run --release -- -h`

If you use an already compiled executable, you should use:
`music-not-found -h`

The `-d DIRECTORY_PATH` flag is be used to specify the directory which should be processed and `-c OUTPUT_PATH`
instructs the program to generate a `json`-file ready for submission. Thus, to generate a submission, one must execute:

`cargo run --release -- -d AUDIO_FILES_DIRECTORY -c submission.json`

If an F-Measure, either for onsets xor for beats,  should be computed,
please also uncomment the corresponding return statement at the end of the `process_file` function in `main.rs`.

## Onset Detection

For the onset detection we tried three different algorithms.

The processing of all three of them can be divided into three parts:

1. preprocessing (the STFT is computed).
2. the detection function
3. postprocessing/peak-picking

We created our own STFT function (see `helpers.rs`), which is utilizes an FFT function which we imported. Our STFT
takes windows size and hop size as parameters and is computed accordingly to them.

For the detection function, we implemented three different algorithms:
Spectral Difference, High Frequency Content and LFSF - for each of them we used the given formula from the lecture
slides.

After trying out the different algorithms on the training data set, we found out that HighFrequencyContent delievered
the lowest F-Measure. Spectral Difference and LFSF delivered almost identical results, but LFSF has an F-Measure
approximately 0.03 higher than Spectral Difference.
Consequently, for the onset detection, we use LFSF.

Also trough trying out, we found out combining the results of an LFSF with windows size 2048 and hop size 1024 with an
LFSF with window size 1024 and hop size 512 slightly increases the F-Measure on the train dataset.
The combination happens after the peak picking, which is described later on. The two LFSF are combined in a way that
only onsets that were found through both LFSF are counted as onsets (see constant ENSEMBLE_NEEDED_SCORE).

For post-processing, we implemented the peak-picking algorithm (implementation is based on lecture slides). The results
of the onset function are processed and only points that are a local maximum in a given window, points that are greater
than the mean of a specified window, and points fulfilling a minimum distance to an already found onset are selected as
onsets. After Peak Picking, the found onsets are converted to onset times in second.

## Tempo Estimation

Our Tempo estimation is based on auto-correlation. We auto-correlate the whole sample of the track then do some peak
picking. TO BE DESCRIBED IN MORE DETAIL

## Beat Detection

The third function, beat detection, is based on the first two functions, onset detection (= feature extraction) and
tempo estimation (= periodicity estimation). We would classify it as a histogram-based beat tracker.

The estimated tempo is taken and the and the ideal duration between two beats is computed.

The first beat is determined by looking at the spectral difference values of the onsets. The first local maxima of
the onsets (with respect to the spectral difference value) is taken as the first beat. Further beats are calculated
based on this first beat and the duration between two beats.
For this, after each found beat the two next onsets that
follow are considered as possible next beats.
This results in an iteration over all found onsets. In the following we describe what is happening in each iteration.

If the duration between the last found beat and the directly following next onset is more than 1.3 times the ideal
duration to
the next beat,
an additional next beat is "artificially" computed at the location of the ideal next beat
(= location of the last beat + beat duration). Experimenting showed, that this slightly influences the f-measure in
a positive way (increase of approx. 0.005)

Independently of this, the difference of the two considered next beats to this ideal next beat is computed and the
one which is closer to the ideal beat is taken as the next onset.
After this the next same procedure is starting again with the currently found beat.

When reaching the end, a vector containing the found onset times is returned.

## Known issues

* The Rust project (the folder `music_not_found`) is missing the 404 in its name and is contained in a subdirectory, as
  names starting with digits are
  not allowed :frowning:
* When processing a directory, the displayed total number of files is incorrect. Nevertheless, the number of already
  processed files is correct.