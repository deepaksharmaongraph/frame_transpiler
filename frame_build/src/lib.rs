//! This crate defines a configurable process for compiling Frame source files as part of building
//! a larger Rust package.
//!
//! # What does it do?
//!
//! This crate is intended to be used from your package's `build.rs` file.
//!
//! By default, it will traverse your package's `src` directory, searching for Frame (`.frm`) files
//! to compile into Rust using the Frame transpiler, Framec.
//!
//! The generated Rust files will be stored in an output directory at a relative position
//! corresponding to where the files were found in the `src` directory. For example, a Frame
//! file found at `src/a/b/sm.frm` will be translated into a Rust file `$OUT_DIR/a/b/sm.rs`, where
//! `$OUT_DIR` is the default output directory for a `build.rs` script.
//!
//! Many aspect of this process are configurable. For example, you can change the input and output
//! directories, configure aspects of the input directory traversal, change or add to the target
//! languages that Framec compiles to, filter out certain Frame files based on their path, and
//! more!
//!
//!
//! # How to use it?
//!
//! First, add the following to your `Cargo.toml` file:
//!
//! ```toml
//! [build-dependencies]
//! anyhow = "1.0"
//! frame_build = "0.8"
//! ```
//!
//! You can run the default build process by adding the following `build.rs` file to the root of
//! your package.
//!
//! ```no_run
//! use anyhow::Result;
//! use frame_build::FrameBuild;
//!
//! fn main() -> Result<()> {
//!     FrameBuild::new().run()?;
//!     Ok(())
//! }
//! ```
//!
//! The build process can be configured by various methods on the [`FrameBuild`] struct. These
//! methods are designed to be chained together to define a complete configuration before running
//! the build process.
//!
//! For example, the following `build.rs` script would compile all of the Frame files in the
//! package's `tests` directory to both Rust and [smcat](https://state-machine-cat.js.org/),
//! excluding files with the string `"stack"` in their name (since the smcat backend does not
//! support Frame's state stack feature).
//!
//! ```no_run
//! use anyhow::Result;
//! use frame_build::{FrameBuild, TargetLanguage};
//! use std::path::PathBuf;
//!
//! fn main() -> Result<()> {
//!     FrameBuild::new()
//!         .input_dir(PathBuf::from("tests").as_path())
//!         .add_target(TargetLanguage::Smcat)
//!         .include_only_if(|path| !path.to_str().unwrap().contains("stack"))
//!         .run()?;
//!     Ok(())
//! }
//! ```
//!
//! The generated Rust and Smcat files will be stored side-by-side in their relative positions with
//! the default output directory.
//!
//!
//! # Incorporating Frame-generated Rust in your project
//!
//! A Rust file generated by the default Frame build process can be included in your project by
//! including the code from within the module where you want it to live.
//!
//! We recommend creating a `.rs` file adjacent to each `.frm` file within the package's `src`
//! directory that includes the generated file and adds any supplemental definitions that are
//! needed by the state machines, such as implementing the machines actions.
//!
//! For example, given a Frame specification at `src/a/b/sm.frm`, create a file `src/a/b/sm.rs`
//! that includes the following:
//!
//! ```ignore
//! include!(concat!(env!("OUT_DIR"), "/", "a/b/sm.rs"));
//!
//! // ... action implementations and supplemental definitions go here
//! ```

use anyhow::{Error, Result};
use framec::frame_c::compiler::Exe;
use std::path::{Path, PathBuf};
use std::{env, fs};
use walkdir::WalkDir;

// re-export `TargetLanguage` struct here since it's part of the `frame_build` interface
pub use framec::frame_c::compiler::TargetLanguage;

/// Create, configure, and run a Frame build process. The methods associated with this struct are
/// designed to be chained to override the default configuration. After the process has been
/// configured, the [`FrameBuild::run`] method starts the build process.
pub struct FrameBuild {
    frame_config: Option<PathBuf>,
    input_dir: PathBuf,
    output_dir: PathBuf,
    targets: Vec<TargetLanguage>,
    input_filter: Box<dyn Fn(&Path) -> bool>,
    max_depth: usize,
    min_depth: usize,
    follow_links: bool,
    continue_on_error: bool,
}

impl Default for FrameBuild {
    fn default() -> Self {
        FrameBuild::new()
    }
}

impl FrameBuild {
    /// Construct a new default configuration.
    pub fn new() -> Self {
        FrameBuild {
            frame_config: None,
            input_dir: PathBuf::from("src"),
            output_dir: PathBuf::from(env::var("OUT_DIR").unwrap()),
            targets: vec![TargetLanguage::Rust],
            input_filter: Box::new(|_| true),
            max_depth: ::std::usize::MAX,
            min_depth: 0,
            follow_links: false,
            continue_on_error: false,
        }
    }

    /// Add a Frame `config.yaml` file to pass to Framec.
    ///
    /// If unset, Framec will look for the file in the current working directory.
    pub fn frame_config(mut self, path: &Path) -> Self {
        self.frame_config = Some(path.to_path_buf());
        self
    }

    /// Set the root input directory to traverse, searching for `.frm` files.
    ///
    /// If unset, we will search the project's `src` directory.
    pub fn input_dir(mut self, path: &Path) -> Self {
        self.input_dir = path.to_path_buf();
        self
    }

    /// Set the root output directory to store generated files. Each generated file will be stored
    /// at a relative path within this directory corresponding to the file's location in the input
    /// directory.
    ///
    /// If unset, the build process will store generated files in the default output directory for
    /// a `build.rs` script, which is obtained from the `$OUT_DIR` environment variable set by
    /// Cargo.
    ///
    /// If this library is used outside of the context of a `build.rs` script, this value must be
    /// changed.
    pub fn output_dir(mut self, path: &Path) -> Self {
        self.output_dir = path.to_path_buf();
        self
    }

    /// Set the list of target languages to compile each Frame file to using Framec.
    ///
    /// By default, the build process compiles each Frame file to only Rust. Use this method if
    /// Rust output is not desired. Otherwise, additional targets can be added with
    /// [`FrameBuild::add_target`].
    pub fn set_targets(mut self, targets: &[TargetLanguage]) -> Self {
        self.targets = targets.to_vec();
        self
    }

    /// Add an additional target language to compile to.
    ///
    /// By default, the build process compiles each Frame file to Rust. If Rust output is not
    /// desired, use [`FrameBuild::set_targets`] to override the list of target languages rather
    /// than add to it.
    pub fn add_target(mut self, target: TargetLanguage) -> Self {
        self.targets.push(target);
        self
    }

    /// Set a function that filters the Frame files found in the input directory based on their
    /// paths. For the path of each Frame file found, if this function returns `true`, the file is
    /// compiled into all of the target languages. If the function returns `false`, it is skipped.
    pub fn include_only_if(mut self, filter: impl Fn(&Path) -> bool + 'static) -> Self {
        self.input_filter = Box::new(filter);
        self
    }

    /// Set a maximum depth to search for Frame files in the input directory. A max depth of `1`
    /// means to search only the immediate contents of the input directory. For greater depths,
    /// subdirectories are traversed in a depth-first order.
    ///
    /// By default, the max depth is effectively unbounded.
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set a minimum depth to search for Frame files in the input directory. A min depth of `1`
    /// would include the contents of the input directory, whereas a min depth of `2` would begin
    /// with the contents of the input directory's sub-directories.
    ///
    /// By default, the min depth is `0`.
    pub fn min_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// By default, the traversal of the input directory does not follow symbolic links. Calling
    /// this method sets a flag indicating that symbolic links *should* be followed.
    pub fn follow_links(mut self) -> Self {
        self.follow_links = true;
        self
    }

    /// By default, the build process halts if Framec panics or returns an error. Calling this
    /// method sets a flag that changes this behavior to instead print an error message to `stderr`
    /// and continue searching for and translating Frame files. This can be useful, for example,
    /// when some Frame files are known to not translate to all target languages, and the failing
    /// ones are not needed, or in test suites where some translations are expected to fail.
    ///
    /// Rather than setting this flag, consider also invoking [`FrameBuild::run`] multiple times
    /// with different target languages and different configurations of
    /// [`FrameBuild::include_only_if`].
    ///
    /// Non-Frame errors (e.g. file I/O errors) will halt the build regardless of this setting.
    pub fn continue_on_error(mut self) -> Self {
        self.continue_on_error = true;
        self
    }

    /// Run the Frame build process. The build process is highly configurable using the other
    /// methods associated with this struct.
    ///
    /// On success, this function returns a vector of paths to each of the generated files.
    pub fn run(&self) -> Result<Vec<PathBuf>> {
        let mut generated_files = Vec::new();

        let walk_dir = WalkDir::new(&self.input_dir)
            .max_depth(self.max_depth)
            .min_depth(self.min_depth)
            .follow_links(self.follow_links);

        for entry in walk_dir {
            let entry = entry?;
            let input_path = entry.path();
            if input_path.extension().unwrap_or_default() == "frm"
                && (&self.input_filter)(input_path)
            {
                // tell Cargo this is a source file
                println!("cargo:rerun-if-changed={:?}", &input_path);

                let local_path = input_path.strip_prefix(&self.input_dir)?;
                let output_path = self.output_dir.join(local_path);
                fs::create_dir_all(output_path.parent().unwrap())?;

                for target in &self.targets {
                    let mut target_output_path = output_path.clone();
                    target_output_path.set_extension(target.file_extension());

                    let frame_config = &self.frame_config;
                    let framec_result = std::panic::catch_unwind(move || {
                        Exe::new().run_file(frame_config, input_path, Some(*target))
                    });

                    match framec_result {
                        Ok(Ok(output_content)) => {
                            // success, write the file
                            fs::write(&target_output_path, output_content)?;
                            generated_files.push(target_output_path);
                        }
                        Ok(Err(err)) => {
                            // framec returned an error
                            let msg = format!(
                                "Framec errored while generating {:?}: {:?}",
                                target_output_path, err
                            );
                            if self.continue_on_error {
                                eprintln!("{}", msg);
                            } else {
                                return Err(Error::msg(msg));
                            }
                        }
                        Err(err) => {
                            // framec panicked
                            let msg = format!(
                                "Framec panicked while generating {:?}: {:?}",
                                target_output_path, err
                            );
                            if self.continue_on_error {
                                eprintln!("{}", msg);
                            } else {
                                return Err(Error::msg(msg));
                            }
                        }
                    }
                }
            }
        }

        Ok(generated_files)
    }
}
