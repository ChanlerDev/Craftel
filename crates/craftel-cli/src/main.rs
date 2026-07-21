mod args;
mod output;

use args::{Cli, Command, ProjectCommand, TaskCommand};
use clap::Parser;
use craftel_core::{CraftelService, app_paths, domain::WorkflowAction};
use std::{env, error::Error};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let mut service = CraftelService::open(&app_paths::database_path()?)?;
    match cli.command {
        Command::Project { command } => match command {
            ProjectCommand::Add { path, name } => {
                let p = service.register_project(&name, &path)?;
                println!("{}", output::human_project(&p));
            }
            ProjectCommand::List { json } => {
                let values = service.list_projects()?;
                if json {
                    println!("{}", output::json(&values)?)
                } else {
                    for p in values {
                        println!("{}", output::human_project(&p));
                    }
                }
            }
            ProjectCommand::Remove { project_id } => {
                service.remove_project(&project_id)?;
                println!("removed {project_id}");
            }
        },
        Command::Create(a) => {
            let p = project_id(&service, a.project)?;
            let task = service.create_task(&p, &a.title, &a.content)?;
            println!("{}", output::human_task(&task));
        }
        Command::Task { command } => match command {
            TaskCommand::List { project, json } => {
                let p = project_id(&service, project)?;
                let values = service.list_tasks(&p)?;
                if json {
                    println!("{}", output::json(&values)?)
                } else {
                    for t in values {
                        println!("{}", output::human_task(&t));
                    }
                }
            }
            TaskCommand::Update {
                task_id,
                project,
                title,
                content,
            } => {
                let p = project_id(&service, project)?;
                let t = service.update_task(&p, &task_id, &title, &content)?;
                println!("{}", output::human_task(&t));
            }
        },
        Command::Move {
            task_id,
            stage,
            project,
        } => action(
            &mut service,
            project,
            &task_id,
            WorkflowAction::Move(stage.into()),
        )?,
        Command::Next(a) => action(&mut service, a.project, &a.task_id, WorkflowAction::Next)?,
        Command::Pass(a) => action(&mut service, a.project, &a.task_id, WorkflowAction::Pass)?,
        Command::Fail(a) => action(&mut service, a.project, &a.task_id, WorkflowAction::Fail)?,
    }
    Ok(())
}
fn action(
    service: &mut CraftelService,
    project: Option<String>,
    task: &str,
    action: WorkflowAction,
) -> Result<(), Box<dyn Error>> {
    let p = project_id(service, project)?;
    let t = service.apply(&p, task, action)?;
    println!("{}", output::human_task(&t));
    Ok(())
}
fn project_id(
    service: &CraftelService,
    explicit: Option<String>,
) -> Result<String, Box<dyn Error>> {
    if let Some(id) = explicit {
        return Ok(id);
    }
    let cwd = env::current_dir()?.canonicalize()?;
    service
        .list_projects()?
        .into_iter()
        .filter(|p| p.available && cwd.starts_with(&p.work_dir))
        .max_by_key(|p| p.work_dir.components().count())
        .map(|p| p.id)
        .ok_or_else(|| {
            "no project specified and current directory is not inside a registered project".into()
        })
}
