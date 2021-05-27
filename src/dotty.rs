use ignore::DirEntry;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

use crate::utils;

type DottyReturn = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, StructOpt)]
#[structopt(setting = structopt::clap::AppSettings::ArgRequiredElseHelp)]
pub struct Dotty {
    #[structopt(short, long)]
    verbose: bool,
    #[structopt(
        long,
        conflicts_with_all = &["remove", "relink"],
        help = "Caution! This will turn files in source to symlinks. Run --help to read full description",
        long_help = r"Caution! This will turn files in source to symlinks and may alter the contents of your destination directory.
Any plain text files in the source that does not exist in destination will be moved to source and a symlink will be created in the destination.
Example:
$ tree src
├── .foo

$ tree target
├──

$ dotty src=/path dest=/path --import

$ tree src
├── .foo (symlinked)

$ tree target
├── .foo


"
    )]
    import: bool,
    #[structopt(
        short,
        long,
        conflicts_with = "DryRun",
        help = "Prompts user everytime it tries to modify filesystem."
    )]
    interactive: bool,
    #[structopt(
        long = "dryRun",
        conflicts_with = "interactive",
        help = "Do not do anything; just show what would happen."
    )]
    dry_run: bool,
    #[structopt(
        short = "s",
        long = "src",
        alias = "from",
        parse(from_str = utils::expand_tilde),
        validator = utils::dir_exist,
        help = "Source directory (default is current dir)",
    )]
    source: Option<PathBuf>,
    #[structopt(
        short = "d",
        long = "dest",
        alias = "to",
        parse(from_str = utils::expand_tilde),
        validator = utils::dir_exist,
        help = "Destination directory (default is $HOME dir)",
    )]
    destination: Option<PathBuf>,
    #[structopt(
        long,
        conflicts_with_all = &["import", "relink"],
        help = "Unlink the symlinks in destination path linked from the source directory."
    )]
    remove: bool,
    #[structopt(
        long,
        conflicts_with_all = &["remove", "import"],
        help = "Recreate (unlink target and relink) link to destination from source. This is useful for pruning obsolete symlinks from the destination."
    )]
    relink: bool,
}

#[derive(Debug)]
enum Mode {
    DryRun,
    Remove,
    Relink,
    Import,
    Normal,
}

impl Dotty {
    fn src_dest(&self) -> (PathBuf, PathBuf) {
        let src = match &self.source {
            None => {
                if let Ok(src) = std::env::current_dir() {
                    src
                } else {
                    PathBuf::from(env!("PWD"))
                }
            }
            Some(src) => src.to_path_buf(),
        };

        let dest = match &self.destination {
            None => PathBuf::from(env!("HOME")),
            Some(dest) => dest.to_path_buf(),
        };

        (src, dest)
    }

    fn log(&self, message: String) {
        if self.verbose {
            println!("{}", message);
        }
    }

    fn mode(&self) -> Mode {
        if self.dry_run {
            Mode::DryRun
        } else if self.import {
            Mode::Import
        } else if self.relink {
            Mode::Relink
        } else if self.remove {
            Mode::Remove
        } else {
            Mode::Normal
        }
    }

    fn ask_and_run<F>(&self, message: String, cb: F) -> DottyReturn
    where
        F: Fn() -> DottyReturn,
    {
        if self.interactive {
            if let Ok(yes) = utils::prompt(message) {
                if yes {
                    cb()?;
                }
            }
        } else {
            cb()?;
        }

        Ok(())
    }

    pub(crate) fn run(&self) -> DottyReturn {
        dbg!(self);

        match self.mode() {
            // DryRun import, remove, relink etc
            Mode::DryRun => {
                println!("[WARN] `dryRun` mode, no changes will be made.");
                self.dir_walk(|entry, target| -> DottyReturn {
                    if entry.path().is_dir() {
                        println!("CREATE: {}", &target.display());
                    } else if entry.path().is_file() {
                        println!("LINK: {} => {}", &entry.path().display(), target.display());
                    } else {
                        return Err("Unknown error".into());
                    }

                    Ok(())
                })?;
            }
            Mode::Remove => {
                self.dir_walk(|entry, target| -> DottyReturn {
                    if entry.path().is_file() && target.is_file() {
                        if let Ok(yes) = same_file::is_same_file(&entry.path(), &target) {
                            if yes {
                                self.ask_and_run("Remove?".to_string(), || {
                                    std::fs::remove_file(&target)?;
                                    self.log(format!("[INFO] {} unlinked", &target.display()));
                                    Ok(())
                                })?;
                            }
                        }
                    }

                    Ok(())
                })?;
            }
            Mode::Relink => {
                self.dir_walk(|src, target| -> DottyReturn {
                    let src = src.path();

                    if src.is_file() && target.is_file() {
                        if let Ok(yes) = same_file::is_same_file(&src, &target) {
                            if yes {
                                if utils::is_plain_text(&src) {
                                    // std::fs::copy(&src, &target)?;
                                    // std::fs::remove_file(&target)?;
                                    // self.symlink(&target, &src.to_path_buf())?;
                                    // self.log(format!("[RELINK] {}", &target.display()))
                                    dbg!((&src, &target));
                                }
                                // std::fs::remove_file(&target)?;
                            }
                        }
                    }

                    Ok(())
                })?;
            }
            Mode::Import => {
                self.dir_walk(|src, target| {
                    if !target.exists() {
                        if src.path().is_dir() {
                            self.process_dir(&target)?;
                        }

                        if src.path().is_file() {
                            std::fs::copy(&src.path(), &target)?;
                            std::fs::remove_file(&src.path())?;
                            self.symlink(&target, &src.path().to_path_buf())?;
                        }
                    } else {
                        self.log(format!("[SKIP] {} exist.", &target.display()))
                    }

                    Ok(())
                })?;
            }
            Mode::Normal => {
                self.dir_walk(|entry, target| -> DottyReturn {
                    if entry.path().is_dir() {
                        if !&target.is_dir() {
                            self.process_dir(&target)?;
                        }
                    } else if entry.path().is_file() {
                        // do prompts and stuffs here
                        if !&target.is_file() {
                            self.symlink(entry.path(), &target)?;
                        }
                    }

                    Ok(())
                })?;
            }
        }

        Ok(())
    }

    fn dir_walk<F>(&self, cb: F) -> DottyReturn
    where
        F: Fn(DirEntry, &PathBuf) -> DottyReturn,
    {
        let (src, dest) = self.src_dest();

        // dbg!(&src);

        let walker = ignore::WalkBuilder::new(&src)
            .standard_filters(false)
            .hidden(false)
            .parents(true)
            .ignore(true)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .filter_entry(|d| {
                !(d.path().to_str().unwrap_or_default().contains(".git/")
                    && d.path().ends_with(".git"))
            })
            .build();

        // println!("=========0=========");
        for path in walker.skip(1) {
            // println!("=========1=========");
            match path {
                Ok(path) => {
                    let target = &dest.join(&path.path().strip_prefix(&src)?);

                    cb(path, target)?;
                }
                Err(e) => {
                    dbg!(e);
                }
            }
        }
        Ok(())
    }

    /// Creates a new directory instead of symlinking the target directory as we
    /// don't own the target directory and we wanna keep the source directory clean.
    fn process_dir(&self, target: &PathBuf) -> DottyReturn {
        if self.interactive {
            if let Ok(yes) = utils::prompt("Create dir?".to_string()) {
                if yes {
                    std::fs::create_dir_all(&target)?;
                    self.log(format!("[INFO] {} linked", &target.display()));
                }
            }
        } else {
            std::fs::create_dir_all(&target)?;
            self.log(format!("[INFO] {} linked", &target.display()));
        }

        Ok(())
    }

    fn symlink(&self, src: &Path, target: &PathBuf) -> DottyReturn {
        if utils::is_plain_text(&src) {
            let inner = || -> DottyReturn {
                #[cfg(target_family = "unix")]
                std::os::unix::fs::symlink(src, &target)?;

                #[cfg(target_family = "windows")]
                std::os::windows::fs::symlink_file(src, &target)?;

                Ok(())
            };

            if self.interactive {
                if let Ok(yes) = utils::prompt(format!(
                    "Create symlink to {} from {}?",
                    &target.display(),
                    &src.display()
                )) {
                    if yes {
                        inner()?;
                        self.log(format!("[LINK] {}", &target.display()));
                    }
                }
            } else {
                inner()?;
                self.log(format!("[LINK] {}", &target.display()));
            }
        } else {
            self.log(format!("[SKIP] {} is not plain text", &target.display()));
        }

        Ok(())
    }
}
