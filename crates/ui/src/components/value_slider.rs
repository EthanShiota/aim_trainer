use bevy::{
    color::palettes::tailwind,
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    input_focus::{FocusLost, InputFocus},
    prelude::*,
    text::{EditableText, TextCursorStyle, TextEditChange},
    window::{CursorOptions, WindowFocused},
};

#[derive(Component, Default, Clone)]
#[component(on_add)]
pub struct ValueSlider(pub f64);

impl ValueSlider {
    fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        let inital_value = world.get::<ValueSlider>(ctx.entity).unwrap().0.to_string();
        world
            .commands()
            .entity(ctx.entity)
            .apply_scene(bsn! {
                Node {
                    width: px(200.),
                    height: px(30.)
                }
                EditableText::new(inital_value)
                TextCursorStyle
            })
            .observe(validate)
            .observe(on_focus_loss)
            .observe(on_drag_end)
            .observe(on_drag_start)
            .observe(on_drag);
    }
}

fn validate(
    e: On<TextEditChange, ValueSlider>,
    mut q: Query<(&mut EditableText, &mut ValueSlider)>,
) {
    if let Ok((text, mut slider)) = q.get_mut(e.event_target())
        && let Ok(val) = text.value().to_string().parse::<f64>()
    {
        slider.0 = val;
    }
}

fn on_focus_loss(e: On<FocusLost>, mut q: Query<(&mut EditableText, &mut ValueSlider)>) {
    if let Ok((mut text, mut slider)) = q.get_mut(e.event_target()) {
        if let Ok(val) = text.value().to_string().parse::<f64>() {
            slider.0 = val;
        } else {
            text.editor_mut().set_text(&slider.0.to_string());
        }
    }
}
fn on_drag_end(
    e: On<Pointer<DragEnd>>,
    mut q: Query<(&mut EditableText, &mut ValueSlider)>,
    mut window: Query<(&mut CursorOptions, &Window)>,
) {
    if let Ok((mut text, slider)) = q.get_mut(e.event_target()) {
        text.editor_mut().set_text(&slider.0.to_string());
    }

    for (mut options, _) in window.iter_mut().filter(|(_, w)| w.focused) {
        options.visible = true;
    }
}

fn on_drag_start(e: On<Pointer<DragStart>>, mut window: Query<(&mut CursorOptions, &Window)>) {
    for (mut options, _) in window.iter_mut().filter(|(_, w)| w.focused) {
        options.visible = false;
    }
}

fn on_drag(
    e: On<Pointer<Drag>>,
    mut slider: Single<(&mut ValueSlider, &mut EditableText)>,
    mut window: Query<(&mut Window, &mut CursorOptions)>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let modifier = keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let multiplier = if modifier { 0.1 } else { 1. };

    slider.0.0 += e.delta.x as f64 * multiplier;
    let text = slider.0.0.to_string();
    slider.1.editor_mut().set_text(&text);
}
