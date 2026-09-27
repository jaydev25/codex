use std::time::Duration;

use codex_local_models::check_openai_compatible_endpoint;
use url::Url;

use super::*;

impl ChatWidget {
    pub(crate) fn open_actor_model_popup(&mut self) {
        let Some(backend_url) = self.config.local_analysis.backend_url.clone() else {
            self.add_error_message(
                "Set local_models.analysis.backend_url before selecting an actor model."
                    .to_string(),
            );
            return;
        };
        let tx = self.app_event_tx.clone();
        tokio::spawn(async move {
            let result = async {
                let endpoint = Url::parse(&backend_url).map_err(|error| error.to_string())?;
                let client = reqwest::Client::builder()
                    .timeout(Duration::from_secs(5))
                    .build()
                    .map_err(|error| error.to_string())?;
                check_openai_compatible_endpoint(&client, &endpoint, "__list_models__")
                    .await
                    .map(|status| status.available_models)
                    .map_err(|error| error.to_string())
            }
            .await;
            tx.send(AppEvent::ActorModelsLoaded(result));
        });
    }

    pub(crate) fn show_actor_model_popup(&mut self, models: Vec<String>) {
        if models.is_empty() {
            self.add_error_message("LM Studio reported no available models.".to_string());
            return;
        }
        let current = self.config.local_analysis.backend_model.as_deref();
        let items = models
            .into_iter()
            .map(|model| {
                let selected = current == Some(model.as_str());
                let action_model = model.clone();
                SelectionItem {
                    name: model,
                    description: Some(
                        "Local evidence analyst; cloud model remains authoritative".to_string(),
                    ),
                    is_current: selected,
                    actions: vec![Box::new(move |tx| {
                        tx.send(AppEvent::UpdateActorModel(action_model.clone()));
                    })],
                    dismiss_on_select: true,
                    ..Default::default()
                }
            })
            .collect();
        let mut header = ColumnRenderable::new();
        header.push(Line::from("Select LM Studio Actor Model".bold()));
        header.push(Line::from(
            "Used for local evidence only; the cloud model remains authoritative.".dim(),
        ));
        self.bottom_pane.show_selection_view(SelectionViewParams {
            view_id: Some("actor-model-selection"),
            header: Box::new(header),
            footer_hint: Some(self.bottom_pane.standard_popup_hint_line()),
            items,
            ..Default::default()
        });
    }

    pub(crate) fn set_actor_model(&mut self, model: String, context_length: u32) {
        self.config.local_analysis.backend_model = Some(model.clone());
        self.config.local_analysis.model_id = Some(format!("lm-studio:{model}"));
        self.config.local_analysis.require_registered_model = false;
        self.add_info_message(
            format!("Local actor model set to {model} with a {context_length}-token context."),
            None,
        );
    }

    pub(crate) fn begin_actor_model_activity(&mut self, header: String, details: String) {
        self.bottom_pane.set_task_running(/*running*/ true);
        self.set_status(
            header,
            Some(details),
            StatusDetailsCapitalization::Preserve,
            /*details_max_lines*/ 2,
        );
        self.bottom_pane.set_composer_input_enabled(
            /*enabled*/ false,
            Some("Waiting for the local actor model…".to_string()),
        );
    }

    pub(crate) fn finish_actor_model_activity(&mut self) {
        self.bottom_pane.set_task_running(/*running*/ false);
        if !self.blocks_direct_input {
            self.bottom_pane
                .set_composer_input_enabled(/*enabled*/ true, /*placeholder*/ None);
        }
    }

    pub(crate) fn show_actor_context_prompt(
        &mut self,
        identifier: String,
        model_key: String,
        max_context_length: Option<u32>,
    ) {
        let maximum = max_context_length
            .map(|maximum| format!(" Maximum: {maximum} tokens."))
            .unwrap_or_default();
        let tx = self.app_event_tx.clone();
        let event_identifier = identifier.clone();
        let view = CustomPromptView::new(
            format!("Context length for {identifier}"),
            "Enter a positive token count and press Enter".to_string(),
            String::new(),
            Some(format!("LM Studio model: {model_key}.{maximum}")),
            Box::new(move |input| {
                tx.send(AppEvent::ActorModelContextSubmitted {
                    identifier: event_identifier.clone(),
                    model_key: model_key.clone(),
                    max_context_length,
                    input,
                });
            }),
        );
        self.bottom_pane.show_text_prompt(view);
    }

    pub(crate) fn show_actor_model_load_failure(
        &mut self,
        identifier: String,
        model_key: String,
        max_context_length: Option<u32>,
        context_length: u32,
        error: String,
    ) {
        self.add_error_message(format!("Could not load local actor model: {error}"));
        let retry_identifier = identifier.clone();
        let retry_model_key = model_key;
        let retry_actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
            tx.send(AppEvent::ActorModelContextSubmitted {
                identifier: retry_identifier.clone(),
                model_key: retry_model_key.clone(),
                max_context_length,
                input: context_length.to_string(),
            });
        })];
        let choose_actions: Vec<SelectionAction> = vec![Box::new(|tx| {
            tx.send(AppEvent::OpenActorModelPicker);
        })];
        self.show_selection_view(SelectionViewParams {
            title: Some("Local actor model failed to load".to_string()),
            subtitle: Some(format!("{identifier} · {context_length} tokens")),
            footer_hint: Some(standard_popup_hint_line()),
            items: vec![
                SelectionItem {
                    name: "Retry".to_string(),
                    description: Some("Try the same model and context length again".to_string()),
                    actions: retry_actions,
                    dismiss_on_select: true,
                    ..Default::default()
                },
                SelectionItem {
                    name: "Choose another model".to_string(),
                    description: Some("Return to the LM Studio actor-model picker".to_string()),
                    actions: choose_actions,
                    dismiss_on_select: true,
                    ..Default::default()
                },
                SelectionItem {
                    name: "Cancel".to_string(),
                    description: Some("Keep the previous actor-model configuration".to_string()),
                    dismiss_on_select: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
    }
}
