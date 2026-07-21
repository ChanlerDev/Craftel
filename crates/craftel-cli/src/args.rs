use clap::{Args, Parser, Subcommand, ValueEnum};
use craftel_core::domain::Stage;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version = craftel_core::VERSION)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    Create(CreateArgs),
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    Move {
        task_id: String,
        stage: StageArg,
        #[arg(long)]
        project: Option<String>,
    },
    Next(TaskRef),
    Pass(TaskRef),
    Fail(TaskRef),
}

#[derive(Subcommand)]
pub enum ProjectCommand {
    Add {
        path: PathBuf,
        #[arg(long)]
        name: String,
    },
    List {
        #[arg(long)]
        json: bool,
    },
    Remove {
        project_id: String,
    },
}
#[derive(Subcommand)]
pub enum TaskCommand {
    List {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Update {
        task_id: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        title: String,
        #[arg(long)]
        content: String,
    },
}
#[derive(Args)]
pub struct CreateArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub content: String,
}
#[derive(Args)]
pub struct TaskRef {
    pub task_id: String,
    #[arg(long)]
    pub project: Option<String>,
}
#[derive(Clone, Copy, ValueEnum)]
pub enum StageArg {
    Inbox,
    Defining,
    Implementation,
    Reviewing,
    Done,
}
impl From<StageArg> for Stage {
    fn from(value: StageArg) -> Self {
        match value {
            StageArg::Inbox => Self::Inbox,
            StageArg::Defining => Self::Defining,
            StageArg::Implementation => Self::Implementation,
            StageArg::Reviewing => Self::Reviewing,
            StageArg::Done => Self::Done,
        }
    }
}
