use agent_context_doctor::{audit, Host, Report, ResolutionState};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Audit {
        path: PathBuf,
        #[arg(long, value_enum)]
        host: Host,
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

fn main() {
    let result = match Cli::parse().command {
        Command::Audit { path, host, format } => audit(&path, host)
            .and_then(|report| render(&report, format).map(|output| (report, output))),
    };
    match result {
        Ok((report, output)) => {
            println!("{output}");
            let has_findings = report.artifacts.iter().any(|artifact| {
                matches!(
                    artifact.state,
                    ResolutionState::Ignored
                        | ResolutionState::Shadowed
                        | ResolutionState::TrustGated
                        | ResolutionState::OutOfScope
                )
            });
            std::process::exit(if has_findings { 1 } else { 0 });
        }
        Err(_) => {
            eprintln!("agent_context_doctor: audit could not be completed");
            std::process::exit(2);
        }
    }
}

fn render(report: &Report, format: OutputFormat) -> anyhow::Result<String> {
    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(report)?),
        OutputFormat::Text => {
            let mut lines = vec![format!("model: {}", report.model_version)];
            for artifact in &report.artifacts {
                lines.push(format!(
                    "{}\t{}\t{}\t{}",
                    artifact.state, artifact.kind, artifact.path, artifact.rule_id
                ));
            }
            Ok(lines.join("\n"))
        }
    }
}
