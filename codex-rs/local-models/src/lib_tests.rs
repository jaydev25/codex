use super::*;
use codex_utils_absolute_path::test_support::PathBufExt;
use codex_utils_absolute_path::test_support::test_path_buf;
use pretty_assertions::assert_eq;
use std::ffi::OsStr;
use tempfile::tempdir;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::body_partial_json;
use wiremock::matchers::method;
use wiremock::matchers::path;

fn abs(path: &str) -> AbsolutePathBuf {
    test_path_buf(path).abs()
}

#[test]
fn defaults_to_codex_home_models_directory() {
    let codex_home = abs("/codex-home");

    let resolved = resolve_local_model_storage(
        &codex_home,
        /*config*/ None,
        &LocalModelStorageOverrides::default(),
        /*environment_models_dir*/ None,
    )
    .expect("default storage should resolve");

    assert_eq!(
        resolved,
        LocalModelStorageConfig {
            models_dir: abs("/codex-home/models"),
            temp_dir: abs("/codex-home/models/.downloads"),
            registry_file: abs("/codex-home/models/registry.json"),
            max_disk_gb: None,
            models_dir_source: ModelsDirectorySource::Default,
        }
    );
}

#[test]
fn config_sets_storage_paths_and_quota() {
    let codex_home = abs("/codex-home");
    let config = LocalModelsToml {
        storage: Some(LocalModelStorageToml {
            models_dir: Some(abs("/configured-models")),
            temp_dir: Some(abs("/configured-temp")),
            max_disk_gb: Some(500),
        }),
        analysis: None,
    };

    let resolved = resolve_local_model_storage(
        &codex_home,
        Some(&config),
        &LocalModelStorageOverrides::default(),
        /*environment_models_dir*/ None,
    )
    .expect("configured storage should resolve");

    assert_eq!(
        resolved,
        LocalModelStorageConfig {
            models_dir: abs("/configured-models"),
            temp_dir: abs("/configured-temp"),
            registry_file: abs("/configured-models/registry.json"),
            max_disk_gb: Some(500),
            models_dir_source: ModelsDirectorySource::Config,
        }
    );
}

#[test]
fn command_override_takes_precedence_over_environment_and_config() {
    let codex_home = abs("/codex-home");
    let config = LocalModelsToml {
        storage: Some(LocalModelStorageToml {
            models_dir: Some(abs("/configured-models")),
            ..Default::default()
        }),
        analysis: None,
    };
    let overrides = LocalModelStorageOverrides {
        models_dir: Some(abs("/command-models")),
        ..Default::default()
    };
    let environment_models_dir = abs("/environment-models");

    let resolved = resolve_local_model_storage(
        &codex_home,
        Some(&config),
        &overrides,
        Some(environment_models_dir.as_os_str()),
    )
    .expect("command override should resolve");

    assert_eq!(
        resolved,
        LocalModelStorageConfig {
            models_dir: abs("/command-models"),
            temp_dir: abs("/command-models/.downloads"),
            registry_file: abs("/command-models/registry.json"),
            max_disk_gb: None,
            models_dir_source: ModelsDirectorySource::CommandOverride,
        }
    );
}

#[test]
fn environment_takes_precedence_over_config() {
    let codex_home = abs("/codex-home");
    let config = LocalModelsToml {
        storage: Some(LocalModelStorageToml {
            models_dir: Some(abs("/configured-models")),
            ..Default::default()
        }),
        analysis: None,
    };
    let environment_models_dir = abs("/environment-models");

    let resolved = resolve_local_model_storage(
        &codex_home,
        Some(&config),
        &LocalModelStorageOverrides::default(),
        Some(environment_models_dir.as_os_str()),
    )
    .expect("environment override should resolve");

    assert_eq!(resolved.models_dir, abs("/environment-models"));
    assert_eq!(
        resolved.models_dir_source,
        ModelsDirectorySource::Environment
    );
}

#[test]
fn rejects_relative_environment_path() {
    let error = resolve_local_model_storage(
        &abs("/codex-home"),
        /*config*/ None,
        &LocalModelStorageOverrides::default(),
        Some(OsStr::new("relative/models")),
    )
    .expect_err("relative environment path should be rejected");

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}

fn registered_model(model_id: &str, revision: &str) -> RegisteredLocalModel {
    RegisteredLocalModel {
        model_id: model_id.to_owned(),
        source: HuggingFaceModelSource {
            repository: "acme/code-model".to_owned(),
            revision: revision.to_owned(),
        },
        artifact: LocalModelArtifact {
            format: "gguf".to_owned(),
            quantization: Some("Q4_K_M".to_owned()),
            local_path: abs(&format!("/models/{model_id}")),
        },
    }
}

#[test]
fn missing_registry_is_empty() {
    let directory = tempdir().expect("temporary directory");

    let registry = load_registry(&directory.path().join("registry.json"))
        .expect("missing registry should load");

    assert_eq!(registry, LocalModelRegistry::default());
}

#[test]
fn registry_round_trips_in_model_id_order() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("nested").join("registry.json");
    let registry = LocalModelRegistry {
        schema_version: REGISTRY_SCHEMA_VERSION,
        models: vec![
            registered_model("zeta", "rev-z"),
            registered_model("alpha", "rev-a"),
        ],
    };

    save_registry(&path, &registry).expect("registry should save");
    let loaded = load_registry(&path).expect("registry should load");

    assert_eq!(
        loaded,
        LocalModelRegistry {
            schema_version: REGISTRY_SCHEMA_VERSION,
            models: vec![
                registered_model("alpha", "rev-a"),
                registered_model("zeta", "rev-z")
            ],
        }
    );
}

#[test]
fn upsert_replaces_matching_model_id() {
    let mut registry = LocalModelRegistry {
        schema_version: REGISTRY_SCHEMA_VERSION,
        models: vec![registered_model("coder", "old")],
    };

    upsert_registered_model(&mut registry, registered_model("coder", "new"));

    assert_eq!(
        registry,
        LocalModelRegistry {
            schema_version: REGISTRY_SCHEMA_VERSION,
            models: vec![registered_model("coder", "new")],
        }
    );
}

#[test]
fn remove_reports_whether_model_existed() {
    let mut registry = LocalModelRegistry {
        schema_version: REGISTRY_SCHEMA_VERSION,
        models: vec![registered_model("coder", "rev")],
    };

    assert!(remove_registered_model(&mut registry, "coder"));
    assert!(!remove_registered_model(&mut registry, "missing"));
    assert_eq!(registry, LocalModelRegistry::default());
}

#[tokio::test]
async fn inspects_model_and_resolves_revision() {
    let server = MockServer::start().await;
    let sha = "c".repeat(40);
    Mock::given(method("GET"))
        .and(path("/api/models/acme/code-model/revision/main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "acme/code-model",
            "sha": sha,
            "pipeline_tag": "text-generation",
            "private": false,
            "gated": false,
            "disabled": false,
            "siblings": [{"rfilename": "weights.gguf", "size": 1234}]
        })))
        .mount(&server)
        .await;

    let info = inspect_hugging_face_model(
        &reqwest::Client::new(),
        &Url::parse(&format!("{}/", server.uri())).expect("mock URL"),
        "acme/code-model",
        "main",
        None,
    )
    .await
    .expect("inspection should succeed");

    assert_eq!(info.id, "acme/code-model");
    assert_eq!(info.sha, "c".repeat(40));
    assert_eq!(info.siblings[0].rfilename, "weights.gguf");
    assert_eq!(info.siblings[0].size, Some(1234));
}

#[tokio::test]
async fn rejects_invalid_sha_from_model_inspection() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "acme/code-model",
            "sha": "main",
            "siblings": []
        })))
        .mount(&server)
        .await;

    let error = inspect_hugging_face_model(
        &reqwest::Client::new(),
        &Url::parse(&format!("{}/", server.uri())).expect("mock URL"),
        "acme/code-model",
        "main",
        None,
    )
    .await
    .expect_err("non-commit SHA must fail");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

#[tokio::test]
async fn downloads_pinned_artifact_and_registers_it() {
    let server = MockServer::start().await;
    let revision = "a".repeat(40);
    Mock::given(method("GET"))
        .and(path(format!(
            "/acme/code-model/resolve/{revision}/weights.gguf"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"model bytes"))
        .mount(&server)
        .await;
    let directory = tempdir().expect("temporary directory");
    let root = AbsolutePathBuf::from_absolute_path_checked(directory.path())
        .expect("temporary path should be absolute");
    let storage = LocalModelStorageConfig {
        models_dir: root.join("models"),
        temp_dir: root.join("downloads"),
        registry_file: root.join("models/registry.json"),
        max_disk_gb: None,
        models_dir_source: ModelsDirectorySource::Config,
    };
    let expected_sha256 = format!("{:x}", Sha256::digest(b"model bytes"));

    let downloaded = download_hugging_face_artifact(
        &reqwest::Client::new(),
        &storage,
        HuggingFaceDownloadRequest {
            endpoint: Url::parse(&format!("{}/", server.uri())).expect("mock URL"),
            repository: "acme/code-model".to_owned(),
            revision: revision.clone(),
            filename: PathBuf::from("weights.gguf"),
            model_id: "coder".to_owned(),
            format: "gguf".to_owned(),
            quantization: Some("Q4_K_M".to_owned()),
            expected_sha256: Some(expected_sha256),
            token: None,
        },
    )
    .await
    .expect("download should succeed");

    assert_eq!(downloaded.downloaded_bytes, 11);
    assert_eq!(downloaded.resumed_from_bytes, 0);
    assert_eq!(
        load_registry(&storage.registry_file)
            .expect("registry should load")
            .models,
        vec![downloaded.model.clone()]
    );
    assert_eq!(
        fs::read(&downloaded.model.artifact.local_path).expect("artifact should exist"),
        b"model bytes"
    );
}

#[tokio::test]
async fn rejects_download_that_exceeds_storage_quota() {
    let server = MockServer::start().await;
    let revision = "b".repeat(40);
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"too large"))
        .mount(&server)
        .await;
    let directory = tempdir().expect("temporary directory");
    let root = AbsolutePathBuf::from_absolute_path_checked(directory.path())
        .expect("temporary path should be absolute");
    let storage = LocalModelStorageConfig {
        models_dir: root.join("models"),
        temp_dir: root.join("downloads"),
        registry_file: root.join("models/registry.json"),
        max_disk_gb: Some(0),
        models_dir_source: ModelsDirectorySource::Config,
    };

    let error = download_hugging_face_artifact(
        &reqwest::Client::new(),
        &storage,
        HuggingFaceDownloadRequest {
            endpoint: Url::parse(&format!("{}/", server.uri())).expect("mock URL"),
            repository: "acme/code-model".to_owned(),
            revision,
            filename: PathBuf::from("weights.gguf"),
            model_id: "coder".to_owned(),
            format: "gguf".to_owned(),
            quantization: None,
            expected_sha256: None,
            token: None,
        },
    )
    .await
    .expect_err("quota should reject the download");

    assert!(error.to_string().contains("quota exceeded"));
    assert_eq!(
        load_registry(&storage.registry_file).unwrap().models,
        vec![]
    );
}

#[test]
fn removes_registered_artifact_inside_model_root() {
    let directory = tempdir().expect("temporary directory");
    let root = AbsolutePathBuf::from_absolute_path_checked(directory.path())
        .expect("temporary path should be absolute");
    let storage = LocalModelStorageConfig {
        models_dir: root.join("models"),
        temp_dir: root.join("models/.downloads"),
        registry_file: root.join("models/registry.json"),
        max_disk_gb: None,
        models_dir_source: ModelsDirectorySource::Config,
    };
    let artifact = storage.models_dir.join("coder/weights.gguf");
    fs::create_dir_all(artifact.parent().unwrap()).unwrap();
    fs::write(&artifact, b"weights").unwrap();
    let mut model = registered_model("coder", "revision");
    model.artifact.local_path = artifact.clone();
    save_registry(
        &storage.registry_file,
        &LocalModelRegistry {
            schema_version: REGISTRY_SCHEMA_VERSION,
            models: vec![model],
        },
    )
    .unwrap();

    let removed = remove_local_model(&storage, "coder")
        .expect("removal should succeed")
        .expect("model should exist");

    assert!(removed.artifact_deleted);
    assert!(!artifact.exists());
    assert!(
        load_registry(&storage.registry_file)
            .unwrap()
            .models
            .is_empty()
    );
}

#[test]
fn refuses_to_remove_registered_artifact_outside_model_root() {
    let directory = tempdir().expect("temporary directory");
    let root = AbsolutePathBuf::from_absolute_path_checked(directory.path())
        .expect("temporary path should be absolute");
    let storage = LocalModelStorageConfig {
        models_dir: root.join("models"),
        temp_dir: root.join("models/.downloads"),
        registry_file: root.join("models/registry.json"),
        max_disk_gb: None,
        models_dir_source: ModelsDirectorySource::Config,
    };
    let outside = root.join("outside.gguf");
    fs::write(&outside, b"keep me").unwrap();
    let mut model = registered_model("coder", "revision");
    model.artifact.local_path = outside.clone();
    save_registry(
        &storage.registry_file,
        &LocalModelRegistry {
            schema_version: REGISTRY_SCHEMA_VERSION,
            models: vec![model],
        },
    )
    .unwrap();

    let error = remove_local_model(&storage, "coder").expect_err("unsafe path must fail");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(outside.exists());
    assert_eq!(
        load_registry(&storage.registry_file).unwrap().models.len(),
        1
    );
}

#[test]
fn routes_large_artifact_to_registered_local_model() {
    let policy = LocalAnalysisPolicy {
        enabled: true,
        model_id: Some("coder".to_owned()),
        min_output_bytes: 4,
        max_input_bytes: 8,
        backend_url: Some("http://127.0.0.1:1234/v1".to_owned()),
        backend_model: Some("local-analyst".to_owned()),
    };
    let registry = LocalModelRegistry {
        schema_version: REGISTRY_SCHEMA_VERSION,
        models: vec![registered_model("coder", "revision")],
    };
    let artifact = AnalysisArtifact::from_bytes(
        AnalysisWorkloadKind::Test,
        b"twelve bytes",
        policy.max_input_bytes,
        true,
    );

    assert_eq!(
        route_local_analysis(&policy, &registry, &artifact),
        LocalAnalysisRoute::Local {
            model_id: "coder".to_owned(),
            input_bytes: 8,
            truncated: true,
        }
    );
}

#[test]
fn bypasses_local_analysis_when_raw_fallback_is_unavailable() {
    let policy = LocalAnalysisPolicy {
        enabled: true,
        model_id: Some("coder".to_owned()),
        min_output_bytes: 1,
        max_input_bytes: 8,
        backend_url: Some("http://127.0.0.1:1234/v1".to_owned()),
        backend_model: Some("local-analyst".to_owned()),
    };
    let registry = LocalModelRegistry {
        schema_version: REGISTRY_SCHEMA_VERSION,
        models: vec![registered_model("coder", "revision")],
    };
    let artifact = AnalysisArtifact::from_bytes(
        AnalysisWorkloadKind::Log,
        b"large log",
        policy.max_input_bytes,
        false,
    );

    assert_eq!(
        route_local_analysis(&policy, &registry, &artifact),
        LocalAnalysisRoute::CloudRaw {
            reason: LocalAnalysisBypassReason::RawArtifactUnavailable,
        }
    );
}

#[test]
fn validates_cited_local_evidence() {
    let artifact =
        AnalysisArtifact::from_bytes(AnalysisWorkloadKind::Lint, b"error on line", 1024, true);
    let digest = LocalEvidenceDigest {
        schema_version: EVIDENCE_SCHEMA_VERSION,
        artifact_id: artifact.artifact_id.clone(),
        findings: vec![LocalEvidenceFinding {
            summary: "lint error".to_owned(),
            byte_start: 0,
            byte_end: 5,
            confidence_bps: 9_000,
        }],
        unknowns: vec![],
    };

    validate_evidence_digest(&artifact, &digest).expect("valid evidence should pass");
}

#[test]
fn rejects_local_evidence_with_out_of_bounds_citation() {
    let artifact =
        AnalysisArtifact::from_bytes(AnalysisWorkloadKind::Scanner, b"warning", 1024, true);
    let digest = LocalEvidenceDigest {
        schema_version: EVIDENCE_SCHEMA_VERSION,
        artifact_id: artifact.artifact_id.clone(),
        findings: vec![LocalEvidenceFinding {
            summary: "scanner warning".to_owned(),
            byte_start: 0,
            byte_end: 100,
            confidence_bps: 8_000,
        }],
        unknowns: vec![],
    };

    assert_eq!(
        validate_evidence_digest(&artifact, &digest)
            .expect_err("out-of-bounds citation must fail")
            .kind(),
        io::ErrorKind::InvalidData
    );
}

#[test]
fn captures_content_addressed_artifact_and_reads_bounded_input() {
    let directory = tempdir().expect("temporary directory");
    let artifact_dir = AbsolutePathBuf::from_absolute_path_checked(directory.path())
        .expect("temporary path should be absolute")
        .join("analysis-artifacts");

    let first =
        capture_analysis_artifact(&artifact_dir, AnalysisWorkloadKind::Test, "0123456789", 4)
            .expect("artifact should be captured");
    let second =
        capture_analysis_artifact(&artifact_dir, AnalysisWorkloadKind::Test, "0123456789", 4)
            .expect("identical artifact capture should be idempotent");

    assert_eq!(first.artifact_id, second.artifact_id);
    assert_eq!(first.raw_artifact_path, second.raw_artifact_path);
    assert!(first.truncated_for_analysis);
    assert_eq!(
        read_bounded_analysis_input(&first, 4).expect("bounded read should succeed"),
        b"0123"
    );
    assert_eq!(
        fs::read(first.raw_artifact_path.unwrap()).expect("raw artifact should remain complete"),
        b"0123456789"
    );
}

#[tokio::test]
async fn local_analysis_adapter_returns_validated_digest() {
    let server = MockServer::start().await;
    let artifact =
        AnalysisArtifact::from_bytes(AnalysisWorkloadKind::Log, b"fatal error", 1024, true);
    let digest = LocalEvidenceDigest {
        schema_version: EVIDENCE_SCHEMA_VERSION,
        artifact_id: artifact.artifact_id.clone(),
        findings: vec![LocalEvidenceFinding {
            summary: "fatal error found".to_owned(),
            byte_start: 0,
            byte_end: 5,
            confidence_bps: 9_500,
        }],
        unknowns: vec![],
    };
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(body_partial_json(serde_json::json!({
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "local_evidence_digest",
                    "strict": true
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": serde_json::to_string(&digest).unwrap()}}]
        })))
        .mount(&server)
        .await;

    let actual = analyze_with_openai_compatible_endpoint(
        &reqwest::Client::new(),
        &Url::parse(&format!("{}/v1", server.uri())).expect("mock URL"),
        "local-analyst",
        &artifact,
        "Find the failure",
        b"fatal error",
    )
    .await
    .expect("local analysis should succeed");

    assert_eq!(actual, digest);
}

#[tokio::test]
async fn local_analysis_adapter_rejects_non_loopback_endpoint() {
    let artifact =
        AnalysisArtifact::from_bytes(AnalysisWorkloadKind::Log, b"fatal error", 1024, true);

    let error = analyze_with_openai_compatible_endpoint(
        &reqwest::Client::new(),
        &Url::parse("https://example.com/v1").unwrap(),
        "local-analyst",
        &artifact,
        "Find the failure",
        b"fatal error",
    )
    .await
    .expect_err("remote endpoint must be rejected");

    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
}

#[tokio::test]
async fn local_analysis_backend_status_lists_models_and_matches_configuration() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"id": "model-b"},
                {"id": "local-analyst"}
            ]
        })))
        .mount(&server)
        .await;

    let status = check_openai_compatible_endpoint(
        &reqwest::Client::new(),
        &Url::parse(&format!("{}/v1", server.uri())).expect("mock URL"),
        "local-analyst",
    )
    .await
    .expect("backend status should succeed");

    assert_eq!(status.configured_model, "local-analyst");
    assert_eq!(status.available_models, ["local-analyst", "model-b"]);
    assert!(status.model_available);
}

#[tokio::test]
async fn local_analysis_backend_status_rejects_non_loopback_endpoint() {
    let error = check_openai_compatible_endpoint(
        &reqwest::Client::new(),
        &Url::parse("https://example.com/v1").unwrap(),
        "local-analyst",
    )
    .await
    .expect_err("remote endpoint must be rejected");

    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn classifies_supported_validation_commands() {
    assert_eq!(
        classify_analysis_command("cargo nextest run -p core"),
        Some(AnalysisWorkloadKind::Test)
    );
    assert_eq!(
        classify_analysis_command("npm run lint"),
        Some(AnalysisWorkloadKind::Lint)
    );
    assert_eq!(
        classify_analysis_command("trivy fs ."),
        Some(AnalysisWorkloadKind::Scanner)
    );
    assert_eq!(
        classify_analysis_command("cargo check --workspace"),
        Some(AnalysisWorkloadKind::Build)
    );
    assert_eq!(classify_analysis_command("git status"), None);
}

#[tokio::test]
async fn completed_command_pipeline_returns_evidence() {
    let server = MockServer::start().await;
    let output = "test failed";
    let expected_artifact =
        AnalysisArtifact::from_bytes(AnalysisWorkloadKind::Test, output.as_bytes(), 1024, true);
    let digest = LocalEvidenceDigest {
        schema_version: EVIDENCE_SCHEMA_VERSION,
        artifact_id: expected_artifact.artifact_id,
        findings: vec![LocalEvidenceFinding {
            summary: "test failure".to_owned(),
            byte_start: 0,
            byte_end: 4,
            confidence_bps: 9_000,
        }],
        unknowns: vec![],
    };
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": serde_json::to_string(&digest).unwrap()}}]
        })))
        .mount(&server)
        .await;
    let directory = tempdir().unwrap();
    let artifact_dir = AbsolutePathBuf::from_absolute_path_checked(directory.path()).unwrap();
    let policy = LocalAnalysisPolicy {
        enabled: true,
        model_id: Some("coder".to_owned()),
        min_output_bytes: 1,
        max_input_bytes: 1024,
        backend_url: Some(format!("{}/v1", server.uri())),
        backend_model: Some("local-analyst".to_owned()),
    };
    let registry = LocalModelRegistry {
        schema_version: REGISTRY_SCHEMA_VERSION,
        models: vec![registered_model("coder", "revision")],
    };

    let outcome = analyze_completed_command(
        &reqwest::Client::new(),
        &policy,
        &registry,
        &artifact_dir,
        "cargo test",
        output,
        "Find the failing test",
    )
    .await;

    assert!(matches!(outcome, LocalAnalysisOutcome::Evidence { .. }));
}

#[tokio::test]
async fn completed_command_pipeline_falls_back_when_backend_fails() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let directory = tempdir().unwrap();
    let artifact_dir = AbsolutePathBuf::from_absolute_path_checked(directory.path()).unwrap();
    let policy = LocalAnalysisPolicy {
        enabled: true,
        model_id: Some("coder".to_owned()),
        min_output_bytes: 1,
        max_input_bytes: 1024,
        backend_url: Some(format!("{}/v1", server.uri())),
        backend_model: Some("local-analyst".to_owned()),
    };
    let registry = LocalModelRegistry {
        schema_version: REGISTRY_SCHEMA_VERSION,
        models: vec![registered_model("coder", "revision")],
    };

    let outcome = analyze_completed_command(
        &reqwest::Client::new(),
        &policy,
        &registry,
        &artifact_dir,
        "cargo test",
        "test failed",
        "Find the failing test",
    )
    .await;

    assert!(matches!(
        outcome,
        LocalAnalysisOutcome::FailedCloudRaw { .. }
    ));
}
