use super::*;
use codex_local_models::LmStudioModelInfo;
use codex_local_models::load_lm_studio_model;
use codex_local_models::resolve_lm_studio_model;

impl App {
    pub(super) fn begin_actor_model_metadata_lookup(&mut self, identifier: String) {
        self.chat_widget.begin_actor_model_activity(
            "Checking local coordinator model".to_string(),
            format!("Reading LM Studio metadata for {identifier}"),
        );
        let tx = self.app_event_tx.clone();
        tokio::spawn(async move {
            let lookup_identifier = identifier.clone();
            let result = tokio::task::spawn_blocking(move || {
                resolve_lm_studio_model(&lookup_identifier)
                    .map(|model| (model.model_key, model.max_context_length))
                    .map_err(|error| error.to_string())
            })
            .await
            .unwrap_or_else(|error| Err(format!("LM Studio metadata task failed: {error}")));
            tx.send(AppEvent::ActorModelMetadataLoaded { identifier, result });
        });
    }

    pub(super) fn finish_actor_model_metadata_lookup(
        &mut self,
        identifier: String,
        result: Result<(String, Option<u32>), String>,
    ) {
        self.chat_widget.finish_actor_model_activity();
        match result {
            Ok((model_key, max_context_length)) => self.chat_widget.show_actor_context_prompt(
                identifier,
                model_key,
                max_context_length,
            ),
            Err(error) => {
                self.chat_widget
                    .add_error_message(format!("Could not inspect LM Studio model: {error}"));
                self.chat_widget.open_actor_model_popup();
            }
        }
    }

    pub(super) fn submit_actor_model_context(
        &mut self,
        identifier: String,
        model_key: String,
        max_context_length: Option<u32>,
        input: String,
    ) {
        let context_length =
            match crate::chatwidget::parse_actor_context_length(&input, max_context_length) {
                Ok(context_length) => context_length,
                Err(error) => {
                    self.chat_widget.add_error_message(error);
                    self.chat_widget.show_actor_context_prompt(
                        identifier,
                        model_key,
                        max_context_length,
                    );
                    return;
                }
            };
        self.chat_widget.begin_actor_model_activity(
            "Loading local coordinator model".to_string(),
            format!("{identifier} · {context_length} token context"),
        );
        let tx = self.app_event_tx.clone();
        tokio::spawn(async move {
            let load_identifier = identifier.clone();
            let load_model_key = model_key.clone();
            let result = tokio::task::spawn_blocking(move || {
                load_lm_studio_model(
                    &LmStudioModelInfo {
                        identifier: load_identifier,
                        model_key: load_model_key,
                        max_context_length,
                    },
                    context_length,
                )
                .map_err(|error| error.to_string())
            })
            .await
            .unwrap_or_else(|error| Err(format!("LM Studio load task failed: {error}")));
            tx.send(AppEvent::ActorModelLoadFinished {
                identifier,
                model_key,
                max_context_length,
                context_length,
                result,
            });
        });
    }

    pub(super) async fn finish_actor_model_load(
        &mut self,
        app_server: &mut AppServerSession,
        identifier: String,
        model_key: String,
        max_context_length: Option<u32>,
        context_length: u32,
        result: Result<(), String>,
    ) -> Result<()> {
        self.chat_widget.finish_actor_model_activity();
        if let Err(error) = result {
            self.chat_widget.show_actor_model_load_failure(
                identifier,
                model_key,
                max_context_length,
                context_length,
                error,
            );
            return Ok(());
        }

        let apply_result =
            ConfigEditsBuilder::for_config_path(self.local_settings.user_config_path.as_path())
                .set_local_planner_model(&identifier)
                .apply()
                .await;
        match apply_result {
            Ok(()) => {
                self.chat_widget.set_actor_model(identifier, context_length);
                app_server.reload_user_config().await?;
            }
            Err(error) => self
                .chat_widget
                .add_error_message(format!("Failed to save coordinator model: {error}")),
        }
        Ok(())
    }
}
