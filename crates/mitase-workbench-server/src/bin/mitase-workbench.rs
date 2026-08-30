#![forbid(unsafe_code)]

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use mitase_work_model::WorkRequest;
use mitase_workbench_server::{WorkbenchLaunchConfig, WorkbenchServer, project};
use mitase_workspace::SpecWorkspace;
use std::{
    fs,
    net::IpAddr,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Parser)]
#[command(name = "mitase-workbench", about = "Run the transitional Workbench")]
struct Cli {
    #[command(subcommand)]
    command: WorkbenchCommand,
}

#[derive(Debug, Subcommand)]
enum WorkbenchCommand {
    Project(ProjectArgs),
    Serve(ServeArgs),
}

#[derive(Debug, Args)]
struct ProjectArgs {
    #[arg(long, default_value = ".")]
    workspace: PathBuf,
    #[arg(long)]
    request: Option<PathBuf>,
    #[arg(long, value_enum, default_value = "json")]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct ServeArgs {
    #[arg(long, default_value = ".")]
    workspace: PathBuf,
    #[arg(long)]
    request: Option<PathBuf>,
    #[arg(long, default_value = "127.0.0.1")]
    bind: IpAddr,
    #[arg(long, default_value_t = 7737)]
    port: u16,
    #[arg(long)]
    allow_remote_bind: bool,
    #[arg(long)]
    session_token: Option<String>,
    #[arg(long)]
    show_log: bool,
    #[arg(long)]
    no_open: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Json,
    Yaml,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(2);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        WorkbenchCommand::Project(args) => run_project(args),
        WorkbenchCommand::Serve(args) => run_server(args),
    }
}

fn run_project(args: ProjectArgs) -> Result<()> {
    let workspace = SpecWorkspace::load(&args.workspace)?;
    let index = workspace.index()?;
    let request = args
        .request
        .as_ref()
        .map(|path| read_yaml::<WorkRequest>(path))
        .transpose()?;
    if let Some(request) = &request {
        mitase_planner::validate_work_request(&index, request)
            .context("workbench projection request is outside its exact origin")?;
    }
    let projection = project(&workspace, request.as_ref(), &revision(&workspace.root)?)?;
    match args.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&projection)?),
        OutputFormat::Yaml => print!("{}", serde_yaml::to_string(&projection)?),
    }
    Ok(())
}

fn run_server(args: ServeArgs) -> Result<()> {
    if !args.bind.is_loopback() && !args.allow_remote_bind {
        bail!("remote --bind requires --allow-remote-bind");
    }
    if !args.bind.is_loopback() && args.session_token.as_deref().is_none_or(str::is_empty) {
        bail!("remote --bind requires --session-token");
    }
    let workspace = SpecWorkspace::load(&args.workspace)?;
    let request = args
        .request
        .as_ref()
        .map(|path| read_yaml::<WorkRequest>(path))
        .transpose()?;
    if args.show_log {
        println!("Workbench request logging is handled by the server runtime");
    }
    let server = WorkbenchServer::new(workspace.root.clone()).with_launch(WorkbenchLaunchConfig {
        workspace_root: workspace.root,
        bind: args.bind,
        port: args.port,
        session_token: args.session_token,
        no_open: args.no_open,
    });
    if let Some(request) = request {
        server.with_request(request)?.run()?;
    } else {
        server.run()?;
    }
    Ok(())
}

fn read_yaml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    serde_yaml::from_str(
        &fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?,
    )
    .with_context(|| format!("strict parse {}", path.display()))
}

fn revision(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        bail!("git rev-parse HEAD failed");
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}
