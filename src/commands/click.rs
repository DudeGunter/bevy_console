use bevy::prelude::*;

#[derive(Component, Reflect)]
#[require(Name::new("Debug Click Enabled"))]
pub struct DebugClick;

pub fn debug_click(_: In<String>, mut commands: Commands, query: Query<Entity, With<DebugClick>>) {
    if let Ok(entity) = query.single() {
        commands.entity(entity).despawn();
    } else {
        commands.spawn(DebugClick);
    }
}

pub fn debug_click_observer(trigger: On<Pointer<Click>>, query: Query<Entity, With<DebugClick>>) {
    if let Ok(_entity) = query.single() {
        info!("Clicked on entity {}", trigger.entity);
    }
}
