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

    pub(crate) fn set_actor_model(&mut self, model: String) {
        self.config.local_analysis.backend_model = Some(model.clone());
        self.config.local_analysis.model_id = Some(format!("lm-studio:{model}"));
        self.config.local_analysis.require_registered_model = false;
        self.add_info_message(format!("Local actor model set to {model}."), None);
    }
}
