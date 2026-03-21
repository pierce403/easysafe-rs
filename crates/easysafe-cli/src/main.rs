mod node_client;

use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use easysafe_core::{
    HeadRef, KdfProfile, UnlockedWorkspace, Workspace,
};
use node_client::NodeClient;

#[derive(Debug, Parser)]
#[command(name = "easysafe")]
#[command(about = "EasySafe CLI", long_about = None)]
struct Cli {
    #[arg(long, global = true, default_value = ".")]
    dir: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init {
        #[arg(long)]
        server: String,
        #[arg(long, default_value = "balanced")]
        kdf_profile: KdfProfile,
        #[command(flatten)]
        secrets: SecretArgs,
    },
    Info,
    Put {
        path: PathBuf,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        id: Option<String>,
        #[command(flatten)]
        secrets: SecretArgs,
    },
    Commit {
        #[arg(long)]
        title: String,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        content: Option<String>,
        #[arg(long)]
        stdin: bool,
        #[command(flatten)]
        secrets: SecretArgs,
    },
    Ls {
        #[command(flatten)]
        secrets: ReadSecretArgs,
    },
    Cat {
        id: String,
        #[command(flatten)]
        secrets: ReadSecretArgs,
    },
    History {
        #[command(flatten)]
        secrets: ReadSecretArgs,
    },
    Push,
    Pull {
        #[command(flatten)]
        secrets: ReadSecretArgs,
    },
    ShareRead {
        #[command(flatten)]
        secrets: ReadSecretArgs,
    },
    ShareWrite {
        #[command(flatten)]
        secrets: SecretArgs,
    },
}

#[derive(Debug, Clone, Args)]
struct ReadSecretArgs {
    #[arg(long)]
    read_passphrase: Option<String>,
}

#[derive(Debug, Clone, Args)]
struct SecretArgs {
    #[arg(long)]
    read_passphrase: Option<String>,
    #[arg(long)]
    write_passphrase: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init {
            server,
            kdf_profile,
            secrets,
        } => {
            let read = read_secret("Read passphrase", secrets.read_passphrase)?;
            let write = read_optional_secret("Write passphrase (leave empty to reuse read)", secrets.write_passphrase)?;
            let workspace =
                Workspace::init(&cli.dir, &server, &read, write.as_deref(), kdf_profile.parameters())?;
            println!("Initialized workspace {}", workspace.config().workspace_id);
            println!("Server: {}", workspace.config().server_url);
            println!("KDF profile: {:?}", workspace.config().crypto.kdf.profile);
        }
        Command::Info => {
            let workspace = Workspace::open(&cli.dir)?;
            println!("{}", serde_json::to_string_pretty(workspace.config())?);
            println!("{}", serde_json::to_string_pretty(workspace.state())?);
        }
        Command::Put {
            path,
            title,
            id,
            secrets,
        } => {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let title = title.unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("Untitled")
                    .to_string()
            });
            let mut unlocked = unlock_write_workspace(&cli.dir, secrets)?;
            let result = unlocked.upsert_document(title, content, id)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Commit {
            title,
            id,
            content,
            stdin,
            secrets,
        } => {
            let body = if stdin {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer)?;
                buffer
            } else if let Some(content) = content {
                content
            } else {
                bail!("provide --content or --stdin");
            };
            let mut unlocked = unlock_write_workspace(&cli.dir, secrets)?;
            let result = unlocked.upsert_document(title, body, id)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Ls { secrets } => {
            let unlocked = unlock_read_workspace(&cli.dir, secrets)?;
            for doc in unlocked.list_documents()? {
                println!("{}\t{}\t{}", doc.document_id, doc.updated_at, doc.title);
            }
        }
        Command::Cat { id, secrets } => {
            let unlocked = unlock_read_workspace(&cli.dir, secrets)?;
            let snapshot = unlocked.read_document(&id)?;
            println!("{}", snapshot.body);
        }
        Command::History { secrets } => {
            let unlocked = unlock_read_workspace(&cli.dir, secrets)?;
            for (head_id, head) in unlocked.head_history()? {
                println!(
                    "{}\tseq={}\tmanifest={}\tcreated={}",
                    head_id, head.sequence, head.manifest_object_id, head.created_at
                );
            }
        }
        Command::Push => {
            let mut workspace = Workspace::open(&cli.dir)?;
            let client = NodeClient::new(workspace.config().server_url.clone());
            let health = client.health().await?;
            if health.status != "ok" {
                bail!("node did not report healthy status");
            }
            client.put_config(workspace.config()).await?;
            for object_id in workspace.object_ids()? {
                let bytes = workspace.object_bytes(&object_id)?;
                client.put_object(&object_id, &bytes).await?;
            }
            if let Some(head_id) = workspace.current_head_id() {
                let head_ref = HeadRef {
                    workspace_id: workspace.config().workspace_id.clone(),
                    head_id: head_id.to_string(),
                    updated_at: Utc::now(),
                };
                client.put_head(&head_ref).await?;
            }
            workspace.touch_push()?;
            println!("Pushed workspace {}", workspace.config().workspace_id);
        }
        Command::Pull { secrets } => {
            let workspace = Workspace::open(&cli.dir)?;
            let client = NodeClient::new(workspace.config().server_url.clone());
            if let Some(remote_config) = client.get_config(&workspace.config().workspace_id).await? {
                if remote_config.workspace_id != workspace.config().workspace_id {
                    bail!("remote config workspace id mismatch");
                }
            }
            let mut unlocked = unlock_read_workspace(&cli.dir, secrets)?;
            let head_ref = client
                .get_head(&unlocked.config().workspace_id)
                .await?
                .ok_or_else(|| anyhow!("remote workspace has no published head"))?;
            pull_head_chain(&mut unlocked, &client, &head_ref.head_id).await?;
            let remote_head = unlocked.load_head(&head_ref.head_id)?;
            fetch_if_missing(
                unlocked.workspace(),
                &client,
                &remote_head.manifest_object_id,
            )
            .await?;
            let manifest = unlocked.load_manifest(&remote_head.manifest_object_id)?;
            for doc in manifest.documents {
                fetch_if_missing(unlocked.workspace(), &client, &doc.latest_revision_id).await?;
            }
            unlocked
                .workspace_mut()
                .update_current_head(Some(head_ref.head_id.clone()))?;
            unlocked.workspace_mut().touch_pull()?;
            println!("Pulled head {}", head_ref.head_id);
        }
        Command::ShareRead { secrets } => {
            let workspace = Workspace::open(&cli.dir)?;
            let read_passphrase = read_secret("Read passphrase", secrets.read_passphrase)?;
            let bundle = serde_json::json!({
                "serverUrl": workspace.config().server_url,
                "workspaceId": workspace.config().workspace_id,
                "readPassphrase": read_passphrase,
            });
            println!("{}", serde_json::to_string_pretty(&bundle)?);
        }
        Command::ShareWrite { secrets } => {
            let workspace = Workspace::open(&cli.dir)?;
            let read_passphrase = read_secret("Read passphrase", secrets.read_passphrase)?;
            let write_passphrase = read_secret("Write passphrase", secrets.write_passphrase)?;
            let bundle = serde_json::json!({
                "serverUrl": workspace.config().server_url,
                "workspaceId": workspace.config().workspace_id,
                "readPassphrase": read_passphrase,
                "writePassphrase": write_passphrase,
            });
            println!("{}", serde_json::to_string_pretty(&bundle)?);
        }
    }

    Ok(())
}

async fn fetch_if_missing(
    workspace: &Workspace,
    client: &NodeClient,
    object_id: &str,
) -> Result<()> {
    if workspace.object_store().exists(object_id) {
        return Ok(());
    }
    let bytes = client.get_object(object_id).await?;
    workspace.ingest_object(object_id, &bytes)?;
    Ok(())
}

async fn pull_head_chain(
    unlocked: &mut UnlockedWorkspace,
    client: &NodeClient,
    start_head_id: &str,
) -> Result<()> {
    let mut current = Some(start_head_id.to_string());
    while let Some(head_id) = current {
        let already_present = unlocked.workspace().object_store().exists(&head_id);
        if !already_present {
            let bytes = client.get_object(&head_id).await?;
            unlocked.workspace().ingest_object(&head_id, &bytes)?;
        }
        let head = unlocked.load_head(&head_id)?;
        current = head.previous_head_id.clone();
    }
    Ok(())
}

fn unlock_read_workspace(dir: &PathBuf, secrets: ReadSecretArgs) -> Result<UnlockedWorkspace> {
    let workspace = Workspace::open(dir)?;
    let read = read_secret("Read passphrase", secrets.read_passphrase)?;
    Ok(workspace.unlock(&read, None)?)
}

fn unlock_write_workspace(dir: &PathBuf, secrets: SecretArgs) -> Result<UnlockedWorkspace> {
    let workspace = Workspace::open(dir)?;
    let read = read_secret("Read passphrase", secrets.read_passphrase)?;
    let write = read_optional_secret("Write passphrase (leave empty to reuse read)", secrets.write_passphrase)?;
    Ok(workspace.unlock(&read, write.as_deref())?)
}

fn read_secret(prompt: &str, provided: Option<String>) -> Result<String> {
    if let Some(value) = provided {
        return Ok(value);
    }
    Ok(rpassword::prompt_password(format!("{prompt}: "))?)
}

fn read_optional_secret(prompt: &str, provided: Option<String>) -> Result<Option<String>> {
    if let Some(value) = provided {
        return Ok(Some(value));
    }
    let value = rpassword::prompt_password(format!("{prompt}: "))?;
    if value.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}
