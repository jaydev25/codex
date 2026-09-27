use serde::Deserialize;
use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

const MAX_ERROR_CHARS: usize = 512;

/// A loaded LM Studio model resolved to the key accepted by `lms load`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LmStudioModelInfo {
    pub identifier: String,
    pub model_key: String,
    pub max_context_length: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoadedModel {
    identifier: String,
    model_key: String,
    max_context_length: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstalledModel {
    model_key: String,
    max_context_length: Option<u32>,
}

/// Resolves an API-visible model identifier to LM Studio load metadata.
pub fn resolve_lm_studio_model(identifier: &str) -> io::Result<LmStudioModelInfo> {
    let loaded_output = run_lms(["ps", "--json"])?;
    let installed_output = run_lms(["ls", "--json"])?;
    let loaded: Vec<LoadedModel> =
        serde_json::from_slice(&loaded_output.stdout).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("could not parse `lms ps --json`: {error}"),
            )
        })?;
    let installed: Vec<InstalledModel> =
        serde_json::from_slice(&installed_output.stdout).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("could not parse `lms ls --json`: {error}"),
            )
        })?;
    resolve_model(identifier, &loaded, &installed)
}

/// Private helper to resolve model identifier.
fn resolve_model(
    identifier: &str,
    loaded: &[LoadedModel],
    installed: &[InstalledModel],
) -> Result<LmStudioModelInfo, io::Error> {
    // Check if the identifier matches a loaded model
    if let Some(selected) = loaded.iter().find(|model| model.identifier == identifier) {
        // If loaded, return it with matching installed max_context_length when present
        let installed_max = installed
            .iter()
            .find(|model| model.model_key == selected.model_key)
            .and_then(|model| model.max_context_length);
        return Ok(LmStudioModelInfo {
            identifier: selected.identifier.clone(),
            model_key: selected.model_key.clone(),
            max_context_length: installed_max.or(selected.max_context_length),
        });
    }

    // If no loaded model matches, check if an installed model has the same key as requested identifier
    if let Some(installed_model) = installed.iter().find(|model| model.model_key == identifier) {
        return Ok(LmStudioModelInfo {
            identifier: identifier.to_string(),
            model_key: installed_model.model_key.clone(),
            max_context_length: installed_model.max_context_length,
        });
    }

    // If no loaded or installed model matches, return NotFound
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("LM Studio model identifier {identifier:?} is not loaded or installed"),
    ))
}

/// Returns identifiers of all LoadedModel instances with the given model key.
fn loaded_model_identifiers_for_key(loaded: &[LoadedModel], target_key: &str) -> Vec<String> {
    loaded
        .iter()
        .filter(|model| model.model_key == *target_key)
        .map(|model| model.identifier.clone())
        .collect()
}

/// Loads the selected model with the requested context length.
pub fn load_lm_studio_model(model: &LmStudioModelInfo, context_length: u32) -> io::Result<()> {
    let loaded_output = run_lms(["ps", "--json"])?;
    let loaded: Vec<LoadedModel> =
        serde_json::from_slice(&loaded_output.stdout).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("could not parse `lms ps --json`: {error}"),
            )
        })?;
    for identifier in loaded_model_identifiers_for_key(&loaded, &model.model_key) {
        run_lms(["unload", identifier.as_str()])?;
    }

    let context_length = context_length.to_string();
    run_lms([
        "load",
        model.model_key.as_str(),
        "--identifier",
        model.identifier.as_str(),
        "--gpu",
        "max",
        "--parallel",
        "1",
        "--context-length",
        context_length.as_str(),
        "--yes",
    ])?;
    Ok(())
}

fn run_lms<const N: usize>(args: [&str; N]) -> io::Result<Output> {
    match run_lms_program(OsStr::new("lms"), &args) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let fallback = lms_fallback_path().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "LM Studio CLI was not found on PATH or in ~/.lmstudio/bin",
                )
            })?;
            run_lms_program(fallback.as_os_str(), &args)
        }
        result => result,
    }
}

fn run_lms_program(program: &OsStr, args: &[&str]) -> io::Result<Output> {
    let output = Command::new(program).args(args).output()?;
    if output.status.success() {
        return Ok(output);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let reason = stderr
        .trim()
        .chars()
        .take(MAX_ERROR_CHARS)
        .collect::<String>();
    let reason = if reason.is_empty() {
        format!("exit status {}", output.status)
    } else {
        reason
    };
    Err(io::Error::other(format!("LM Studio CLI failed: {reason}")))
}

fn lms_fallback_path() -> Option<PathBuf> {
    let home = if cfg!(windows) {
        std::env::var_os("USERPROFILE")
    } else {
        std::env::var_os("HOME")
    }?;
    let executable = if cfg!(windows) { "lms.exe" } else { "lms" };
    Some(PathBuf::from(home).join(".lmstudio/bin").join(executable))
}

#[cfg(test)]
#[path = "lm_studio_tests.rs"]
mod tests;
