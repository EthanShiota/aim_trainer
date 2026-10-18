mod layout;
mod value_slider;

pub use layout::*;
pub use value_slider::ValueSlider;

pub mod spacing {
    #![allow(unused)]

    pub const fn space(scale: f32) -> Val {
        bevy::ui::Val::Rem(0.25 * scale)
    }
    use bevy::ui::Val;
    pub const XS: Val = space(1.);
    pub const SM: Val = space(2.);
    pub const MD: Val = space(3.);
    pub const LG: Val = space(4.);
    pub const XL: Val = space(6.);
    pub const XXL: Val = space(8.);
    pub const XXXL: Val = space(12.);
    pub const XXXXL: Val = space(16.);
}

pub use spacing::*;

use bevy::{
    feathers::{theme::ThemeBackgroundColor, tokens},
    prelude::*,
};

pub fn card(border: bool) -> impl Scene {
    bsn! {
        Node {
            padding: spacing::SM,
            border: {if border {space(1.)} else {space(0.)}},
        }
    }
}
