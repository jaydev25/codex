use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use codex_core::config::ConfigBuilder;
use codex_local_models::HuggingFaceDownloadRequest;
use codex_local_models::check_openai_compatible_endpoint;
use codex_local_models::download_hugging_face_artifact;
use codex_local_models::inspect_hugging_face_model;
use codex_local_models::load_registry;
use codex_local_models::remove_local_model;
use codex_local_models::search_hugging_face_models;
use codex_utils_cli::CliConfigOverrides;
use std::path::PathBuf;
use std::process::Command;
use url::Url;

/// Inspect locally managed model storage and registered artifacts.
#[derive(Debug, Parser)]
pub struct LocalModelCommand {
    #[command(subcommand)]
    subcommand: LocalModelSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum LocalModelSubcommand {
    /// Show the effective model, download, and registry paths.
    Path,

    /// Search Hugging Face model repositories, ordered by downloads.
    Search {
        /// Search text, such as "qwen coder gguf".
        query: String,

        /// Maximum number of results, from 1 through 100.
        #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u8).range(1..=100))]
        limit: u8,

        /// Emit search results as JSON.
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// List model artifacts recorded in the local registry.
    List {
        /// Emit the complete versioned registry as JSON.
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Check the configured local analysis server and model.
    Status {
        /// Emit the backend status as JSON.
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Import and activate a registered GGUF with LM Studio.
    ActivateLmStudio {
        /// Registered local model ID to activate.
        model_id: String,

        /// Print the LM Studio commands without running them.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },

    /// Inspect a Hugging Face model and resolve a revision to its commit SHA.
    Inspect {
        /// Hugging Face model repository in OWNER/NAME form.
        repository: String,

        /// Branch, tag, or commit to inspect.
        #[arg(long, default_value = "main")]
        revision: String,

        /// Emit complete model and file metadata as JSON.
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Download and register one artifact pinned to a Hugging Face commit.
    Download {
        /// Hugging Face model repository in OWNER/NAME form.
        repository: String,

        /// File path inside the repository, for example model-q4_k_m.gguf.
        filename: PathBuf,

        /// Local registry ID for this artifact.
        #[arg(long)]
        model_id: String,

        /// Branch, tag, or commit. It is resolved to an immutable commit before download.
        #[arg(long)]
        revision: String,

        /// Artifact format understood by the intended backend.
        #[arg(long, default_value = "gguf")]
        format: String,

        /// Optional quantization label such as Q4_K_M.
        #[arg(long)]
        quantization: Option<String>,

        /// Optional expected SHA-256 checksum.
        #[arg(long)]
        sha256: Option<String>,
    },

    /// Remove a registered model artifact from local storage.
    Remove {
        /// Local registry ID to remove.
        model_id: String,
    },
}

pub async fn run(
    command: LocalModelCommand,
    root_config_overrides: CliConfigOverrides,
) -> Result<()> {
    let cli_overrides = root_config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    let config = ConfigBuilder::default()
        .cli_overrides(cli_overrides)
        .build()
        .await?;

    match command.subcommand {
        LocalModelSubcommand::Path => {
            println!("models:  {}", config.local_models.models_dir.display());
            println!("staging: {}", config.local_models.temp_dir.display());
            println!("registry: {}", config.local_models.registry_file.display());
        }
        LocalModelSubcommand::Search { query, limit, json } => {
            let token = std::env::var("HF_TOKEN").ok();
            let results = search_hugging_face_models(
                &reqwest::Client::new(),
                &Url::parse("https://huggingface.co/")?,
                &query,
                limit,
                token.as_deref(),
            )
            .await?;
            if json {
                serde_json::to_writer_pretty(std::io::stdout(), &results)?;
                println!();
            } else if results.is_empty() {
                println!("No matching Hugging Face models found.");
            } else {
                for model in results {
                    println!(
                        "{}\tdownloads={}\tlikes={}\t{}",
                        model.id,
                        model.downloads,
                        model.likes,
                        model.pipeline_tag.as_deref().unwrap_or("-")
                    );
                }
            }
        }
        LocalModelSubcommand::List { json } => {
            let registry = load_registry(&config.local_models.registry_file)?;
            if json {
                serde_json::to_writer_pretty(std::io::stdout(), &registry)?;
                println!();
            } else if registry.models.is_empty() {
                println!("No local models registered.");
            } else {
                for model in registry.models {
                    println!(
                        "{}\t{}@{}\t{}\t{}",
                        model.model_id,
                        model.source.repository,
                        model.source.revision,
                        model.artifact.format,
                        model.artifact.local_path.display()
                    );
                }
            }
        }
        LocalModelSubcommand::Status { json } => {
            let backend_url = config
                .local_analysis
                .backend_url
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("local analysis backend_url is not configured"))?;
            let backend_model = config
                .local_analysis
                .backend_model
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("local analysis backend_model is not configured"))?;
            let endpoint = Url::parse(backend_url)?;
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()?;
            let status =
                check_openai_compatible_endpoint(&client, &endpoint, backend_model).await?;
            if json {
                serde_json::to_writer_pretty(std::io::stdout(), &status)?;
                println!();
            } else {
                println!("endpoint: {}", endpoint);
                println!("model:    {}", status.configured_model);
                println!(
                    "available: {}",
                    if status.model_available { "yes" } else { "no" }
                );
                if !status.available_models.is_empty() {
                    println!("models:   {}", status.available_models.join(", "));
                }
            }
            if !status.model_available {
                anyhow::bail!(
                    "configured model {:?} is not exposed by the local analysis server",
                    status.configured_model
                );
            }
        }
        LocalModelSubcommand::ActivateLmStudio { model_id, dry_run } => {
            let registry = load_registry(&config.local_models.registry_file)?;
            let model = registry
                .models
                .iter()
                .find(|model| model.model_id == model_id)
                .ok_or_else(|| anyhow::anyhow!("local model {model_id:?} is not registered"))?;
            if !model.artifact.format.eq_ignore_ascii_case("gguf") {
                anyhow::bail!(
                    "LM Studio activation currently supports GGUF artifacts, not {:?}",
                    model.artifact.format
                );
            }
            if !model.artifact.local_path.exists() {
                anyhow::bail!(
                    "registered artifact is missing: {}",
                    model.artifact.local_path.display()
                );
            }
            let backend_url = config
                .local_analysis
                .backend_url
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("local analysis backend_url is not configured"))?;
            let backend_model = config
                .local_analysis
                .backend_model
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("local analysis backend_model is not configured"))?;
            let endpoint = Url::parse(backend_url)?;
            let is_loopback = endpoint.host_str().is_some_and(|host| {
                host == "localhost"
                    || host
                        .parse::<std::net::IpAddr>()
                        .is_ok_and(|address| address.is_loopback())
            });
            if !is_loopback || endpoint.scheme() != "http" {
                anyhow::bail!("LM Studio activation requires an HTTP loopback backend_url");
            }
            let port = endpoint
                .port_or_known_default()
                .ok_or_else(|| anyhow::anyhow!("local analysis backend_url has no usable port"))?;

            let import_args = vec![
                "import".to_owned(),
                "--yes".to_owned(),
                "--copy".to_owned(),
                "--user-repo".to_owned(),
                model.source.repository.clone(),
                model.artifact.local_path.display().to_string(),
            ];
            let artifact_name = model
                .artifact
                .local_path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow::anyhow!("model artifact has no UTF-8 filename"))?;
            let imported_model_key = format!("{}/{artifact_name}", model.source.repository);
            let load_args = vec![
                "load".to_owned(),
                imported_model_key,
                "--identifier".to_owned(),
                backend_model.to_owned(),
                "--gpu".to_owned(),
                "max".to_owned(),
                "--yes".to_owned(),
            ];
            let server_args = vec![
                "server".to_owned(),
                "start".to_owned(),
                "--port".to_owned(),
                port.to_string(),
                "--bind".to_owned(),
                "127.0.0.1".to_owned(),
            ];
            if dry_run {
                print_command("lms", &import_args);
                print_command("lms", &load_args);
                print_command("lms", &server_args);
            } else {
                run_command("lms", &import_args)?;
                run_command("lms", &load_args)?;
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build()?;
                let status =
                    match check_openai_compatible_endpoint(&client, &endpoint, backend_model).await
                    {
                        Ok(status) => status,
                        Err(_) => {
                            run_command("lms", &server_args)?;
                            check_openai_compatible_endpoint(&client, &endpoint, backend_model)
                                .await
                                .context("LM Studio server started but did not become healthy")?
                        }
                    };
                if !status.model_available {
                    anyhow::bail!(
                        "LM Studio started, but configured model {:?} is unavailable",
                        status.configured_model
                    );
                }
                println!(
                    "Activated {} as {} at {}.",
                    model.model_id, backend_model, endpoint
                );
            }
        }
        LocalModelSubcommand::Inspect {
            repository,
            revision,
            json,
        } => {
            let token = std::env::var("HF_TOKEN").ok();
            let info = inspect_hugging_face_model(
                &reqwest::Client::new(),
                &Url::parse("https://huggingface.co/")?,
                &repository,
                &revision,
                token.as_deref(),
            )
            .await?;
            if json {
                serde_json::to_writer_pretty(std::io::stdout(), &info)?;
                println!();
            } else {
                println!("model:    {}", info.id);
                println!("revision: {}", info.sha);
                println!("files:    {}", info.siblings.len());
                if let Some(pipeline_tag) = info.pipeline_tag {
                    println!("pipeline: {pipeline_tag}");
                }
                println!("private:  {}", info.private);
                println!("gated:    {}", info.gated);
                println!("disabled: {}", info.disabled);
            }
        }
        LocalModelSubcommand::Download {
            repository,
            filename,
            model_id,
            revision,
            format,
            quantization,
            sha256,
        } => {
            let token = std::env::var("HF_TOKEN").ok();
            let client = reqwest::Client::new();
            let endpoint = Url::parse("https://huggingface.co/")?;
            let resolved = inspect_hugging_face_model(
                &client,
                &endpoint,
                &repository,
                &revision,
                token.as_deref(),
            )
            .await?;
            println!("Resolved {revision} to {}.", resolved.sha);
            let downloaded = download_hugging_face_artifact(
                &client,
                &config.local_models,
                HuggingFaceDownloadRequest {
                    endpoint,
                    repository,
                    revision: resolved.sha,
                    filename,
                    model_id,
                    format,
                    quantization,
                    expected_sha256: sha256,
                    token,
                },
            )
            .await?;
            println!("Downloaded {} bytes.", downloaded.downloaded_bytes);
            if downloaded.resumed_from_bytes > 0 {
                println!("Resumed from {} bytes.", downloaded.resumed_from_bytes);
            }
            println!("Registered {}.", downloaded.model.model_id);
            println!(
                "Artifact: {}",
                downloaded.model.artifact.local_path.display()
            );
        }
        LocalModelSubcommand::Remove { model_id } => {
            let Some(removed) = remove_local_model(&config.local_models, &model_id)? else {
                anyhow::bail!("local model {model_id:?} is not registered");
            };
            if removed.artifact_deleted {
                println!(
                    "Removed {} and deleted its artifact.",
                    removed.model.model_id
                );
            } else {
                println!(
                    "Removed {} from the registry; its artifact was already missing.",
                    removed.model.model_id
                );
            }
        }
    }

    Ok(())
}

fn run_command(program: &str, args: &[String]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to start {program}"))?;
    if !status.success() {
        anyhow::bail!(
            "{} exited with status {status}",
            display_command(program, args)
        );
    }
    Ok(())
}

fn print_command(program: &str, args: &[String]) {
    println!("{}", display_command(program, args));
}

fn display_command(program: &str, args: &[String]) -> String {
    std::iter::once(program)
        .chain(args.iter().map(String::as_str))
        .map(|argument| format!("{argument:?}"))
        .collect::<Vec<_>>()
        .join(" ")
}
