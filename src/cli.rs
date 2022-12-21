use std::{
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::bail;
use gumdrop::Options;

use crate::{create_queue, download, push_signers, run_queue, upload, Signer};

async fn queue(command: QueueCommand) -> anyhow::Result<()> {
    match command {
        QueueCommand::Create(QueueCreateCommand {
            secret,
            client_id,
            doc_id,
            email,
            ..
        }) => create_queue(client_id, secret, doc_id, email).await,
        QueueCommand::Push(QueuePushCommand {
            queue_id,
            queue_secret,
            name,
            email,
            ..
        }) => push_signers(&queue_id, queue_secret, vec![Signer { name, email }]).await,
        QueueCommand::Run(QueueRunCommand {
            queue_id,
            queue_secret,
            ..
        }) => run_queue(&queue_id, queue_secret).await,
    }
}

pub async fn run() -> anyhow::Result<()> {
    let args = Args::parse_args_default_or_exit();
    let Some(command) = args.command else {
        eprintln!("No command!");
        return Ok(());
    };

    match command {
        Command::Upload(UploadCommand {
            files,
            client_id,
            secret,
            ..
        }) => {
            if files.is_empty() {
                bail!("No files specified.");
            }

            upload(client_id, secret, files).await?;
        }
        Command::Download(DownloadCommand {
            doc_id,
            client_id,
            secret,
            output_path,
            ..
        }) => {
            download(client_id, secret, doc_id, &output_path).await?;
        }
        Command::Queue(args) => {
            let Some(command) = args.command else {
                bail!("No command!");
            };

            return queue(command).await;
        }
    }

    Ok(())
}

#[derive(Debug, Options)]
struct Args {
    #[options(help = "print help message")]
    help: bool,

    #[options(command)]
    command: Option<Command>,
}

#[derive(Debug, Options)]
struct UploadCommand {
    #[options(help = "print help message")]
    help: bool,

    #[options(free)]
    files: Vec<PathBuf>,

    client_id: String,
    secret: String,
}

#[derive(Debug, Options)]
struct DownloadCommand {
    #[options(help = "print help message")]
    help: bool,

    client_id: String,
    secret: String,
    doc_id: String,
    output_path: PathBuf,
}

#[derive(Debug, Options)]
enum Command {
    Upload(UploadCommand),
    Download(DownloadCommand),
    Queue(QueueArgs),
}

#[derive(Debug, Options)]
struct QueueArgs {
    help: bool,
    #[options(command)]
    command: Option<QueueCommand>,
}

#[derive(Debug, Options)]
enum QueueCommand {
    Create(QueueCreateCommand),
    Push(QueuePushCommand),
    Run(QueueRunCommand),
}

#[derive(Debug, Options)]
struct QueueCreateCommand {
    help: bool,
    doc_id: String,
    client_id: String,
    secret: String,
    email: String,
}

#[derive(Debug, Options)]
struct QueuePushCommand {
    help: bool,
    #[options(short = "i")]
    queue_id: String,
    #[options(short = "s")]
    queue_secret: String,
    name: String,
    email: String,
}

#[derive(Debug, Options)]
struct QueueRunCommand {
    help: bool,
    #[options(short = "i")]
    queue_id: String,
    #[options(short = "s")]
    queue_secret: String,
}
