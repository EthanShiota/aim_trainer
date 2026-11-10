use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};

// Sets display to none when not in state S
// Preserves display mode (Flex, Grid, ...)
#[derive(Component, Clone, Debug, PartialEq, Eq, Default)]
#[require(Node)]
#[component(on_add)]
pub struct DisplayInState<S: States>(pub S);

impl<S: States> DisplayInState<S> {
    fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        let Some(state) = world.get_resource::<State<S>>() else {
            error!("State not initialized");
            return;
        };

        let state = state.get().clone();

        if let Ok((mut node, display_state, prior_display)) = world
            .entity_mut(ctx.entity)
            .get_components_mut::<(&mut Node, &DisplayInState<S>, Option<&PriorDisplay>)>()
        {
            if state == display_state.0 {
                if let Some(prior_display) = prior_display {
                    node.display = prior_display.0;
                } else {
                    // Default flex
                    node.display = Display::Flex;
                }
            } else {
                let prior = node.display;
                node.display = Display::None;
                world
                    .commands()
                    .entity(ctx.entity)
                    .insert(PriorDisplay(prior));
            }
        }
    }
}

#[derive(Component, Clone, Debug)]
pub struct PriorDisplay(pub Display);

// Use to register state for updating
pub fn apply_display_in_state<S: States>(
    mut event: MessageReader<StateTransitionEvent<S>>,
    mut commands: Commands,
    mut q_display_in_state: Query<(Entity, &mut Node, &DisplayInState<S>, Option<&PriorDisplay>)>,
) {
    let Some(msg) = event.read().last() else {
        return;
    };

    let StateTransitionEvent {
        entered,
        exited,
        allow_same_state_transitions,
    } = msg;
    if entered == exited && !allow_same_state_transitions {
        return;
    }
    let Some(entered) = entered else {
        return;
    };

    for (entity, mut node, display_in_state, prior_display) in q_display_in_state.iter_mut() {
        if &display_in_state.0 == entered {
            if let Some(display) = prior_display {
                node.display = display.0;
            } else {
                node.display = Display::Flex;
            }
        } else if node.display != Display::None {
            commands.entity(entity).insert(PriorDisplay(node.display));
            node.display = Display::None;
        }
    }
}
