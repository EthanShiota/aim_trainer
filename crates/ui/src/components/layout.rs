use bevy::prelude::*;

pub fn spaced_flex(width: Val) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            justify_content: JustifyContent::SpaceBetween,
            width
        }
    }
}
