use abi_stable::sabi_trait::TD_Opaque;
use verandah_plugin_api::prelude::*;

struct TestWidget {
    config: PluginConfig,
    last_verb: Option<String>,
}

impl WidgetPlugin for TestWidget {
    fn widget_type(&self) -> RStr<'_> {
        "test".into()
    }

    fn default_interval(&self) -> PluginDuration {
        PluginDuration::from_millis(1000)
    }

    fn init(&mut self, config: PluginConfig) -> PluginResult<()> {
        self.config = config;
        PluginResult::ROk(())
    }

    fn config(&self) -> PluginConfig {
        self.config.clone()
    }

    fn poll_state(&mut self) -> PluginResult<PluginPollResponse> {
        PluginResult::ROk(PluginPollResponse::state(PluginWidgetState::Boolean(
            self.last_verb.is_some(),
        )))
    }

    fn render(
        &self,
        _images: RHashMap<RString, PluginImage>,
        _state: &PluginWidgetState,
        image_size: PluginImageSize,
    ) -> PluginResult<PluginImage> {
        PluginResult::ROk(PluginImage::new(image_size.width, image_size.height))
    }

    fn shutdown(&mut self) {}

    fn supported_actions(&self) -> RVec<PluginActionSpec> {
        vec![
            PluginActionSpec::new("toggle", "Toggle the thing").with_default(),
            PluginActionSpec::new("cycle", "Cycle the thing"),
        ]
        .into()
    }

    fn handle_action(&mut self, verb: RStr<'_>) -> PluginResult<()> {
        if verb.as_str() == "toggle" || verb.as_str() == "cycle" {
            self.last_verb = Some(verb.as_str().to_owned());
            PluginResult::ROk(())
        } else {
            PluginResult::RErr(PluginError::new(format!("Unsupported action: {verb}")))
        }
    }
}

#[test]
fn action_methods_work_through_trait_object() -> Result<(), Box<dyn std::error::Error>> {
    let widget = TestWidget {
        config: PluginConfig::new(),
        last_verb: None,
    };
    let mut to = WidgetPlugin_TO::from_value(widget, TD_Opaque);

    let actions = to.supported_actions();
    assert_eq!(actions.len(), 2);
    assert_eq!(actions[0].name.as_str(), "toggle");
    assert!(actions[0].is_default);
    assert!(!actions[1].is_default);

    assert!(to.handle_action("toggle".into()).is_ok());
    assert!(to.handle_action("bogus".into()).is_err());
    Ok(())
}
