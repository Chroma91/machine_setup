use ansi_term::Color::{Green, Red, White, Yellow};
use ergo_fs::{Path, PathArc};
use std::fs::remove_file;
use symlink::{remove_symlink_file, symlink_file};

use crate::{
    command::{CommandConfig, CommandInterface},
    config::config_value::ConfigValue,
    utils::directory::{expand_path, get_source_and_target, walk_files},
};

pub struct SymlinkCommand {}

fn should_force(args: ConfigValue) -> bool {
    if !args.is_hash() {
        return false;
    }

    let arg_values = args.as_hash().unwrap();

    if let Some(force) = arg_values.get("force") {
        return force.as_bool().unwrap_or(false);
    }

    false
}

impl CommandInterface for SymlinkCommand {
    fn install(&self, args: ConfigValue, config: &CommandConfig) -> Result<(), String> {
        let dirs = get_source_and_target(args.clone(), &config.config_dir);
        if let Err(err_dirs) = dirs {
            return Err(err_dirs);
        }
        let dirs = dirs.unwrap();

        let result = create_symlink(&dirs.src, &dirs.target, dirs.ignore, should_force(args));

        if let Err(err_result) = result {
            return Err(err_result);
        }

        Ok(())
    }

    fn uninstall(&self, args: ConfigValue, config: &CommandConfig) -> Result<(), String> {
        let dirs = get_source_and_target(args, &config.config_dir);
        if dirs.is_err() {
            return Err(dirs.err().unwrap());
        }
        let dirs = dirs.unwrap();

        let result = remove_symlink(&dirs.src, &dirs.target);

        if let Err(err_result) = result {
            return Err(err_result);
        }

        Ok(())
    }

    fn update(&self, args: ConfigValue, config: &CommandConfig) -> Result<(), String> {
        self.install(args, config)
    }
}

fn link_files(
    source_dir: &PathArc,
    destination_dir: &Path,
    ignore: Vec<ConfigValue>,
    force: bool,
) -> Result<(), String> {
    println!(
        "Creating symlinks: {} {} {} ...\n",
        White.bold().paint(source_dir.to_string()),
        Green.bold().paint("->"),
        White.bold().paint(destination_dir.to_str().unwrap())
    );

    let result = walk_files(source_dir, destination_dir, ignore, |src, target| {
        println!(
            "Linking {} to {} ...",
            White.bold().paint(src.to_str().unwrap()),
            White.bold().paint(target.to_str().unwrap())
        );

        if force && target.is_file() {
            println!(
                "{}",
                Yellow.paint("Replacing exisiting file with symlink (force) ...")
            );

            remove_file(target).ok();
        }

        symlink_file(src, target)
            .map_err(|e| format!("Failed to link file: {}", Red.paint(e.to_string())))
            .ok();
    });

    if let Err(err_result) = result {
        return Err(err_result);
    }

    Ok(())
}

fn unlink_files(source_dir: &PathArc, destination_dir: &Path) -> Result<(), String> {
    println!(
        "Unlinking files in {} ...",
        White.bold().paint(destination_dir.to_str().unwrap())
    );

    let result = walk_files(source_dir, destination_dir, vec![], |_src, target| {
        println!(
            "Unlinking {} ...",
            White.bold().paint(target.to_str().unwrap())
        );
        remove_symlink_file(target)
            .map_err(|e| format!("Failed to unlink file: {}", Red.paint(e.to_string())))
            .ok();
    });

    if let Err(err_result) = result {
        return Err(err_result);
    }

    Ok(())
}

pub fn create_symlink(
    source: &str,
    destination: &str,
    ignore: Vec<ConfigValue>,
    force: bool,
) -> Result<(), String> {
    let expanded_source = expand_path(source, false);
    if let Err(err_expand_src) = expanded_source {
        return Err(err_expand_src);
    }
    let source_dir = expanded_source.to_owned().unwrap();

    if !source_dir.exists() {
        return Err(format!("Source directory does not exist: {}", source));
    }

    let expanded_destination = expand_path(destination, true);
    if let Err(err_expand_dest) = expanded_destination {
        return Err(err_expand_dest);
    }
    let destination_dir = expanded_destination.unwrap();

    if source_dir.to_string() == destination_dir.to_string() {
        return Err(format!(
            "Source and destination directories are the same: {}",
            source
        ));
    }

    let result = link_files(&source_dir, &destination_dir, ignore, force);

    if let Err(err_result) = result {
        return Err(err_result);
    }

    Ok(())
}

pub fn remove_symlink(source: &str, destination: &str) -> Result<(), String> {
    let expanded_source = expand_path(source, false);
    if let Err(err_expand_src) = expanded_source {
        return Err(err_expand_src);
    }
    let source_dir = expanded_source.to_owned().unwrap();

    let expanded_destination = expand_path(destination, false);
    if let Err(err_expand_dest) = expanded_destination {
        return Err(err_expand_dest);
    }
    let destination_dir = expanded_destination.unwrap();

    let result = unlink_files(&source_dir, &destination_dir);

    if let Err(err_result) = result {
        return Err(err_result);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{fs::File, vec};
    use tempfile::tempdir;

    #[test]
    fn it_fails_when_dirs_are_the_same() {
        let dir = tempdir().unwrap();
        let src_path = dir.path();
        File::create(&src_path.join("example.txt")).unwrap();

        let src = src_path.to_str().unwrap();

        println!("{:?}", create_symlink(src, src, vec![], false));

        assert!(create_symlink(src, src, vec![], false)
            .unwrap_err()
            .contains("Source and destination directories are the same"));
    }

    #[test]
    fn it_symlinks_files() {
        let src_dir = tempdir().unwrap();
        let src = src_dir.path().to_str().unwrap();
        let src_path = src_dir.path().join("example.txt");
        File::create(&src_path).unwrap();

        let dest_dir = tempdir().unwrap();
        let dest = dest_dir.path().to_str().unwrap();

        assert!(create_symlink(src, dest, vec![], false).is_ok());

        let dest_path = dest_dir.path().join("example.txt");
        assert!(dest_path.is_symlink())
    }

    #[test]
    fn it_overrides_file_with_symlink() {
        let src_dir = tempdir().unwrap();
        let src = src_dir.path().to_str().unwrap();
        let src_path = src_dir.path().join("example.txt");
        File::create(&src_path).unwrap();

        let dest_dir = tempdir().unwrap();
        let dest = dest_dir.path().to_str().unwrap();
        let dest_path = dest_dir.path().join("example.txt");

        File::create(&dest_path).unwrap();

        assert!(create_symlink(src, dest, vec![], true).is_ok());

        assert!(dest_path.is_symlink());
    }

    #[test]
    fn it_removes_symlink() {
        let src_dir = tempdir().unwrap();
        let src = src_dir.path().to_str().unwrap();
        let src_path = src_dir.path().join("example.txt");
        File::create(&src_path).unwrap();

        let dest_dir = tempdir().unwrap();
        let dest = dest_dir.path().to_str().unwrap();

        assert!(create_symlink(src, dest, vec![], false).is_ok());

        let dest_path = dest_dir.path().join("example.txt");
        assert!(dest_path.exists());

        assert!(remove_symlink(src, dest).is_ok());

        assert!(!dest_path.exists());
    }
}
