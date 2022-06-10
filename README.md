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
        * :page_facing_up: `plot.rs`: provides functions for plotting float vectors into PNG files
        * :page_facing_up: `track.rs`: reads WAV files and provides a data structure for their content (samples as well
          as file header)
        * to be continued

After having installed `rustc` and `cargo`, please run ... TO BE CONTINUED

## Onset Detection

## Tempo Estimation

Our Tempo estimation is based on auto-correlation. We auto-correlate the whole sample of the track then do some peak
picking. TO BE DESCRIBED IN MORE DETAIL

## Beat Detection

The third function, beat detection, is based on the first two functions, onset detection (= feature extraction) and
tempo estimation (= periodicity estimation).

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