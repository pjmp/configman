use clap::Clap;
use ignore::WalkBuilder;

use std::{
    env, fs, os,
    path::{Path, PathBuf},
    process,
};

use crate::{utils, Result};

#[derive(Debug, Clap)]
#[clap(setting = clap::AppSettings::ArgRequiredElseHelp, about, version)]
pub struct Configman {
    #[clap(short, long, conflicts_with = "dry-run")]
    pub(crate) verbose: bool,
    #[clap(
        long,
        conflicts_with_all = &["remove"],
        about = "Caution! This will turn files in source to symlinks. Run --help to read full description",
        long_about = r"Caution! This will turn files in source to symlinks and may alter the contents of your destination directory.
Any plain text files in the source that does not exist in destination will be moved to source and a symlink will be created in the destination.

Example:
$ tree src
├── .foo

$ tree target
├──

$ configman src=/path dest=/path --import

$ tree src
├── .foo (symlinked)

$ tree target
├── .foo


"
    )]
    import: bool,
    #[clap(
        short,
        long,
        conflicts_with = "dry-run",
        about = "Prompts user every time it tries to modify filesystem."
    )]
    interactive: bool,
    #[clap(
        long = "dry-run",
        conflicts_with = "interactive",
        about = "Do not do anything; just show what would happen."
    )]
    dry_run: bool,
    #[clap(
        short = 's',
        long = "src",
        alias = "from",
        validator = utils::validate_path,
        parse(from_str = utils::expand),
        about = "Source directory (default is current dir)",
    )]
    source: Option<PathBuf>,
    #[clap(
        short = 'd',
        long = "dest",
        alias = "to",
        validator = utils::validate_path,
        parse(from_str = utils::expand),
        about = "Destination directory (default is $HOME dir)",
    )]
    destination: Option<PathBuf>,
    #[clap(
        long,
        conflicts_with_all = &["import"],
        about = "Unlink the symlinks in destination path linked from the source directory."
    )]
    remove: bool,
}

#[derive(Debug)]
enum Mode {
    DryRun,
    Remove,
    Import,
    Normal,
}

impl Configman {
    /// Simply calls [clap's parse](clap::Clap::parse).
    pub fn new() -> Self {
        Self::parse()
    }

    /// Errors occurring here can't be ignored & nothing can proceed without it,
    /// will exit with exitcode 1 if either src or dest is not found.
    fn src_dest(&self) -> (PathBuf, PathBuf) {
        let Self {
            source,
            destination,
            ..
        } = self;

        let src = match source {
            None => match env::current_dir() {
                Ok(src) => src,
                Err(err) => {
                    eprintln!("Error: {}", err);
                    process::exit(1)
                }
            },
            Some(src) => src.to_path_buf(),
        };

        let dest = match destination {
            #[allow(deprecated)]
            None => match env::home_dir() {
                Some(src) => src,
                None => {
                    eprintln!("Error: Unable to get your home dir");
                    process::exit(1)
                }
            },
            Some(dest) => dest.to_path_buf(),
        };

        (src, dest)
    }

    fn ask_and_run<F>(&self, message: String, cb: F) -> Result<()>
    where
        F: Fn() -> Result<()>,
    {
        if self.interactive {
            if utils::prompt(message) {
                cb()?;
            }
        } else {
            cb()?;
        }

        Ok(())
    }

    pub(crate) fn run(&self) -> Result<()> {
        let mode = if self.dry_run {
            Mode::DryRun
        } else if self.import {
            Mode::Import
        } else if self.remove {
            Mode::Remove
        } else {
            Mode::Normal
        };

        match mode {
            // Add dry-run for import, remove etc
            Mode::DryRun => {
                if !log::log_enabled!(log::Level::Info) {
                    utils::enable_log()?;
                }

                log::warn!("`--dry-run` mode, no changes will be made.");

                self.dir_walk(|path, target| -> Result<()> {
                    if path.is_dir() {
                        if !target.exists() {
                            log::info!("[CREATE] {}", &target.display());
                        }
                    } else if path.is_file() {
                        if target.exists() {
                            log::info!("[SKIP] {} (exist)", target.display());
                        } else {
                            log::info!("[LINK] {} => {}", &path.display(), target.display());
                        }
                    } else {
                        return Err(format!(
                            "{} should either be file or directory",
                            target.display()
                        )
                        .into());
                    }

                    Ok(())
                })?;
            }
            Mode::Remove => {
                self.dir_walk(|path, target| -> Result<()> {
                    if target.is_file() && fs::symlink_metadata(&target)?.file_type().is_symlink() {
                        let real_file = fs::read_link(&target)?;

                        if real_file == path {
                            self.ask_and_run(format!("Remove {}?", &target.display()), || {
                                fs::remove_file(&target)?;
                                log::info!("[UNLINKED] {}", &target.display());
                                Ok(())
                            })?;
                        }
                    }

                    Ok(())
                })?;
            }
            Mode::Import => {
                self.dir_walk(|src, target| {
                    if !target.exists() {
                        if src.is_dir() {
                            self.create_dir(&target)?;
                        }

                        if src.is_file() {
                            fs::copy(&src, &target)?;
                            fs::remove_file(&src)?;
                            self.create_file(&target, &src)?;
                        }
                    } else {
                        log::warn!("{} exist.", &target.display());
                    }

                    Ok(())
                })?;
            }
            Mode::Normal => {
                self.dir_walk(|path, target| -> Result<()> {
                    if path.is_dir() {
                        if !target.exists() {
                            self.create_dir(&target)?;
                        }
                    } else if path.is_file() {
                        self.create_file(&path, &target)?;
                    }

                    Ok(())
                })?;
            }
        }

        Ok(())
    }

    fn dir_walk<Fun>(&self, callback: Fun) -> Result<()>
    where
        Fun: Fn(&Path, &PathBuf) -> Result<()>,
    {
        let (source, target) = self.src_dest();

        let mut it = WalkBuilder::new(&source);

        it.standard_filters(false)
            .hidden(false) // don't ignore hidden files/directory
            .parents(false) // ignore parent directory's `.gitignore`
            .ignore(true) // read `.ignore` file
            .git_ignore(true) // read `.gitignore` file
            .git_global(false) // don't read global `.gitignore`
            .git_exclude(true); // read git exclude file

        for result in it.build() {
            match result {
                Ok(entry) => {
                    let path = entry.path();
                    let dest = target.join(&path.strip_prefix(&source)?);

                    callback(path, &dest)?;
                }
                Err(err) => log::warn!("Error: {}", &err),
            }
        }

        Ok(())
    }

    /// Creates a new directory instead of symlinking the target directory as we
    /// don't own the target directory and we wanna keep the source directory clean.
    fn create_dir(&self, target: &Path) -> Result<()> {
        self.ask_and_run(format!("Create dir {}?", target.display()), || {
            fs::create_dir_all(&target)?;
            log::info!("[CREATE] {}", &target.display());

            Ok(())
        })?;

        Ok(())
    }

    fn create_file(&self, src: &Path, target: &Path) -> Result<()> {
        let exist = target.exists();

        let message = if exist {
            format!("Overwrite {} by {}?", target.display(), src.display())
        } else {
            format!(
                "Create symlink to {} from {}?",
                &target.display(),
                &src.display()
            )
        };

        self.ask_and_run(message, || {
            if exist {
                fs::remove_file(&target)?;
            }

            #[cfg(target_family = "unix")]
            os::unix::fs::symlink(src, &target)?;

            #[cfg(target_family = "windows")]
            os::windows::fs::symlink_file(src, &target)?;

            log::info!("[LINK] {}", &target.display());
            Ok(())
        })?;

        Ok(())
    }
}
