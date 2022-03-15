extern crate yaml_rust;

use yaml_rust::{Yaml, YamlLoader};

use crate::config::base_config::*;
use std::{io::Read, path::Path};

#[derive(Debug)]
pub struct YamlConfig {}

static ALLOWED_EXTENSIONS: [&str; 2] = ["yml", "yaml"];

fn parse_yaml(path: &Path) -> Result<YamlConfig, String> {
    let mut file = std::fs::File::open(path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let config = YamlLoader::load_from_str(&contents).unwrap();

    let entries = &config[0];

    if entries["tasks"] == Yaml::BadValue {
        return Err(String::from("no tasks found"));
    }

    //println!("{}", entries["tasks"]);

    return Ok(YamlConfig {});
}

impl BaseConfig for YamlConfig {
    fn read(&self, path: &str) -> Result<TaskList, String> {
        let yaml_path = Path::new(path);

        if !yaml_path.exists() {
            return Err(format!("File {} does not exist", path));
        }

        if !ALLOWED_EXTENSIONS.contains(&yaml_path.extension().unwrap().to_str().unwrap()) {
            return Err(format!("File {} is not a yaml file", path));
        }

        println!("Reading yaml config from {}", path);

        let test = parse_yaml(yaml_path);

        // if test has error return error
        if test.is_err() {
            return Err(test.unwrap_err());
        }

        return Ok(TaskList { tasks: vec![] });
    }

    fn next_task(&self) -> Option<Task> {
        println!("Getting next task from yaml config");

        Some(Task {
            name: "Test task".to_string(),
            args: vec!["test".to_string()],
        })
    }
}

// -- tests --

#[cfg(test)]
mod test {
    use std::{fs::File, io::Write};
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn it_fails_when_config_file_is_missing() {
        let config = YamlConfig {};
        let result = config.read("/tmp/missing.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn it_fails_when_config_file_is_not_yaml() {
        let config = YamlConfig {};
        let result = config.read("/tmp/test.txt");
        assert!(result.is_err());
    }

    #[test]
    fn it_fails_when_tasks_are_not_defined() {
        let dir = tempdir().unwrap();
        let src_path = dir.path().join("example.yaml");
        let mut src_file = File::create(&src_path).unwrap();
        // write string to src_file
        src_file.write_all(b"text: hello world").unwrap();

        let config = YamlConfig {};
        let result = config.read(src_path.to_str().unwrap());

        assert!(result.unwrap_err().contains("no tasks found"));
    }
}
