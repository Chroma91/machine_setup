use ansi_term::Color::{Red, White};
use core::fmt;
use ergo_fs::PathDir;
use std::sync::{Arc, Mutex};

use crate::{command::CommandConfig, config::base_config::TaskList, utils::threads::ThreadPool};

#[derive(Debug, Clone, Copy)]
pub enum TaskRunnerMode {
    Install,
    Update,
    Uninstall,
}

impl fmt::Display for TaskRunnerMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mode = match self {
            TaskRunnerMode::Install => "install",
            TaskRunnerMode::Update => "update",
            TaskRunnerMode::Uninstall => "uninstall",
        };

        write!(f, "{}", mode)
    }
}

pub fn run(
    task_list: TaskList,
    mode: TaskRunnerMode,
    task_name: Option<String>,
    config_dir: PathDir,
) -> Result<(), String> {
    match mode {
        TaskRunnerMode::Install => println!("{}", White.bold().paint("\nInstalling...")),
        TaskRunnerMode::Update => println!("{}", White.bold().paint("\nUpdating...")),
        TaskRunnerMode::Uninstall => println!("{}", White.bold().paint("\nUninstalling...")),
    }

    let command_config = CommandConfig {
        config_dir,
        temp_dir: task_list.temp_dir.to_string(),
        default_shell: task_list.default_shell,
    };

    if let Some(task_name) = task_name {
        let task = task_list.tasks.iter().find(|t| t.name == task_name);
        if task.is_none() {
            return Err(format!(
                "\nTask {} {}",
                White.on(Red).paint(format!(" {} ", task_name)),
                Red.paint("not found")
            ));
        }

        let task_result = task.unwrap().run(&mode, &command_config);
        if task_result.is_err() {
            return Err(format!(
                "\nTask {} {}",
                White.on(Red).paint(format!(" {} ", task_name)),
                Red.paint("failed")
            ));
        }

        return Ok(());
    }

    let mut num_threads = if task_list.parallel {
        task_list.num_threads
    } else {
        1
    };

    if num_threads > task_list.tasks.len() {
        num_threads = task_list.tasks.len();
    }

    if task_list.parallel {
        println!(
            "Running tasks in parallel ({} threads)...",
            White.bold().paint(num_threads.to_string())
        );
    }

    let thread_pool = ThreadPool::new(num_threads);
    let errored_tasks = Arc::new(Mutex::new(vec![]));

    for task in task_list.tasks {
        let config = command_config.clone();
        let task_clone = task.clone();
        let errors = Arc::clone(&errored_tasks);

        let execute = move || {
            let task_result = task_clone.run(&mode, &config);
            if task_result.is_err() {
                let mut e = errors.lock().unwrap();
                e.push(task_clone.name.to_string());
                drop(e);
            }
        };

        thread_pool.execute(execute);
    }

    let errors = errored_tasks.lock().unwrap();
    let num_errored = errors.len();
    if num_errored > 0 {
        return Err(format!(
            "\n{} {} {}\n{}",
            Red.paint("Errors occurred in"),
            Red.bold().underline().paint(num_errored.to_string()),
            Red.paint("tasks:"),
            errors
                .clone()
                .into_iter()
                .map(|e| format!("> {}", e))
                .collect::<Vec<String>>()
                .join("\n")
        ));
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, env::temp_dir};

    use crate::{config::base_config::Command, utils::shell::Shell};

    use super::*;

    fn get_temp_path_dir() -> PathDir {
        PathDir::new(temp_dir()).unwrap()
    }

    #[test]
    fn it_runs_single_task_when_argument_is_passed() {
        let task_list = TaskList {
            tasks: vec![
                Task {
                    name: "task_one".to_string(),
                    commands: vec![Command {
                        name: "_TEST_".to_string(),
                        args: ConfigValue::Array(vec![]),
                    }],
                    os: vec![],
                },
                Task {
                    name: "task_two".to_string(),
                    commands: vec![],
                    os: vec![],
                },
            ],
            temp_dir: "".to_string(),
            default_shell: Shell::Bash,
            num_threads: 1,
        };

        let result = run(
            task_list,
            TaskRunnerMode::Install,
            Some("task_one".to_string()),
            get_temp_path_dir(),
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("task_one"));
    }

    #[test]
    fn it_fails_when_the_task_doesnt_exist() {
        let task_list = TaskList {
            tasks: vec![],
            temp_dir: "".to_string(),
            default_shell: Shell::Bash,
            num_threads: 1,
        };

        let result = run(
            task_list,
            TaskRunnerMode::Install,
            Some("test".to_string()),
            get_temp_path_dir(),
        );

        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("test"));
        assert!(error_message.contains("not found"));
    }

    #[test]
    fn it_runs_all_tasks_when_no_argument_is_passed() {
        let task_list = TaskList {
            tasks: vec![
                Task {
                    name: "task_one".to_string(),
                    commands: vec![],
                    os: vec![],
                },
                Task {
                    name: "task_two".to_string(),
                    commands: vec![],
                    os: vec![],
                },
            ],
            temp_dir: "".to_string(),
            default_shell: Shell::Bash,
            num_threads: 1,
        };

        let result = run(
            task_list,
            TaskRunnerMode::Install,
            None,
            get_temp_path_dir(),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn it_prints_failing_tasks() {
        let task_list = TaskList {
            tasks: vec![
                Task {
                    name: "task_one".to_string(),
                    commands: vec![Command {
                        name: "_TEST_".to_string(),
                        args: ConfigValue::Array(vec![]),
                    }],
                    os: vec![],
                },
                Task {
                    name: "task_two".to_string(),
                    commands: vec![Command {
                        name: "_TEST_".to_string(),
                        args: ConfigValue::Array(vec![]),
                    }],
                    os: vec![],
                },
            ],
            temp_dir: "".to_string(),
            default_shell: Shell::Bash,
            num_threads: 1,
        };

        let result = run(
            task_list,
            TaskRunnerMode::Install,
            None,
            get_temp_path_dir(),
        );

        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Errors occurred in"));
        assert!(error_message.contains("task_one"));
        assert!(error_message.contains("task_two"));
    }

    #[test]
    fn it_runs_commands() {
        let mut run_commands = HashMap::new();
        run_commands.insert(
            String::from("commands"),
            ConfigValue::String(String::from("echo \"test\"")),
        );

        let command = Command {
            name: String::from("run"),
            args: ConfigValue::Hash(run_commands),
        };

        let task_list = TaskList {
            tasks: vec![Task {
                name: "task_one".to_string(),
                commands: vec![command],
                os: vec![],
            }],
            temp_dir: temp_dir().to_str().unwrap().to_string(),
            default_shell: Shell::Bash,
            num_threads: 1,
        };

        let result = run(
            task_list,
            TaskRunnerMode::Install,
            None,
            get_temp_path_dir(),
        );

        assert!(result.is_ok());
    }
}
