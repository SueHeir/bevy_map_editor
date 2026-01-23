use bevy::prelude::*;
use bevy_map_runtime::prelude::*;
#[derive(Component, Debug, Clone, Default)]
#[map_entity(type_name = "Player")]
pub struct Player {}
