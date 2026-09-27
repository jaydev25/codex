use super::*;
use pretty_assertions::assert_eq;

#[test]
fn test_resolve_model_loaded() {
    let loaded = vec![LoadedModel {
        identifier: "test-model".to_string(),
        model_key: "test-key".to_string(),
        max_context_length: Some(4096),
    }];
    let installed = vec![InstalledModel {
        model_key: "test-key".to_string(),
        max_context_length: Some(8192),
    }];

    let result = resolve_model("test-model", &loaded, &installed).unwrap();
    assert_eq!(
        result,
        LmStudioModelInfo {
            identifier: "test-model".to_string(),
            model_key: "test-key".to_string(),
            max_context_length: Some(8192),
        }
    );
}

#[test]
fn test_resolve_model_installed_only() {
    let loaded = vec![];
    let installed = vec![InstalledModel {
        model_key: "test-key".to_string(),
        max_context_length: Some(4096),
    }];

    let result = resolve_model("test-key", &loaded, &installed).unwrap();
    assert_eq!(
        result,
        LmStudioModelInfo {
            identifier: "test-key".to_string(),
            model_key: "test-key".to_string(),
            max_context_length: Some(4096),
        }
    );
}

#[test]
fn test_resolve_model_not_found() {
    let loaded = vec![];
    let installed = vec![];

    let result = resolve_model("unknown-model", &loaded, &installed);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    assert!(error.to_string().contains("unknown-model"));
}

#[test]
fn test_loaded_model_identifiers_for_key() {
    let loaded = vec![
        LoadedModel {
            identifier: "model-a".to_string(),
            model_key: "test-key".to_string(),
            max_context_length: Some(4096),
        },
        LoadedModel {
            identifier: "model-b".to_string(),
            model_key: "other-key".to_string(),
            max_context_length: Some(8192),
        },
        LoadedModel {
            identifier: "model-c".to_string(),
            model_key: "test-key".to_string(),
            max_context_length: Some(2048),
        },
    ];

    let result = loaded_model_identifiers_for_key(&loaded, "test-key");
    assert_eq!(result, vec!["model-a", "model-c"]);
}
