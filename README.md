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
    * :file_facing_up: `Cargo.toml`: contains metadata of the Rust project + list of dependencies and a short
      description
      why a dependency is used.
    * :file_folder: `src`: contains all Rust source files
        * :file_facing_up: `constants.rs`: various constants used across the whole project. Each constant features a
          short documentation comment.
        * to be continued

After having installed `rustc` and `cargo`, please run ... TO BE CONTINUED

## Onset Detection

## Tempo Estimation

## Beat Detection

## Known issues

* The Rust project (the folder `music_not_found`) is missing the 404 in its name and is contained in a subdirectory, as
  names starting with digits are
  not allowed :frowning:
* When processing a directory, the displayed total number of files is incorrect. Nevertheless, the number of already
  processed files is correct.