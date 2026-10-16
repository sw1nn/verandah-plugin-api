use abi_stable::sabi_trait::TD_Opaque;
use verandah_plugin_api::prelude::*;

struct PassiveWidget;

impl WidgetPlugin for PassiveWidget {
    fn widget_type(&self) -> RStr<'_> {
        "passive".into()
    }

    fn default_interval(&self) -> PluginDuration {
        PluginDuration::from_millis(1000)
    }

    fn init(&mut self, _config: PluginConfig) -> PluginResult<()> {
        PluginResult::ROk(())
    }

    fn config(&self) -> PluginConfig {
        PluginConfig::new()
    }

    fn poll_state(&mut self) -> PluginResult<PluginPollResponse> {
        PluginResult::ROk(PluginPollResponse::state(PluginWidgetState::Boolean(false)))
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
        RVec::new()
    }

    fn handle_action(&mut self, _verb: RStr<'_>) -> PluginResult<()> {
        PluginResult::ROk(())
    }
}

#[test]
fn default_on_gesture_proceeds_with_default_feedback() -> Result<(), Box<dyn std::error::Error>> {
    let mut widget = WidgetPlugin_TO::from_value(PassiveWidget, TD_Opaque);

    let reply = widget.on_gesture(PluginGesture::Long).into_result()?;

    assert!(matches!(reply.action, PluginActionDecision::Proceed));
    assert_eq!(reply.feedback, PluginFeedback::Default);
    Ok(())
}

#[test]
fn proceed_carries_the_host_defaults() {
    let reply = PluginGestureReply::proceed();

    assert!(matches!(reply.action, PluginActionDecision::Proceed));
    assert_eq!(reply.feedback, PluginFeedback::Default);
}

#[test]
fn replace_carries_a_host_action() {
    let reply = PluginGestureReply {
        action: PluginActionDecision::Replace(PluginHostAction::PushDeck("desks@NW".into())),
        feedback: PluginFeedback::Flash,
    };

    match reply.action {
        PluginActionDecision::Replace(PluginHostAction::PushDeck(deck)) => {
            assert_eq!(deck.as_str(), "desks@NW")
        }
        other => panic!("expected a deck replacement, got {other:?}"),
    }
    assert_eq!(reply.feedback, PluginFeedback::Flash);
}
