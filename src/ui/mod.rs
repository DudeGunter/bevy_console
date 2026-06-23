use crate::ui::input::*;
use bevy::{
    feathers::{
        containers::{subpane, subpane_body, subpane_header},
        controls::{FeathersTextInput, FeathersTextInputContainer},
        theme::ThemedText,
    },
    input_focus::tab_navigation::TabIndex,
    prelude::*,
    ui_widgets::{ListBox, ScrollArea},
};

pub mod input;
pub mod message;

#[derive(Component, Clone, Default)]
pub struct ConsoleUI;

/// Needed to handle drag events
/// This could be branched off into a separate crate/component
#[derive(Component, Clone, Default)]
pub struct DragData(Vec2);

pub fn open_close_console(
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Visibility, With<crate::ui::ConsoleUI>>,
) {
    if input.just_pressed(KeyCode::KeyT) {
        if let Ok(mut visibility) = query.single_mut() {
            *visibility = match *visibility {
                Visibility::Hidden => Visibility::Visible,
                Visibility::Visible => Visibility::Hidden,
                _ => Visibility::Hidden,
            };
        }
    }
}

// new with feathers!
pub fn spawn_ui(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        #Console
        ConsoleUI
        DragData
        subpane() Children [
            subpane_header() Children [
                (Text("Console") ThemedText),
            ],
            subpane_body() Children [
                (
                    #Container
                    MessageContainer
                    ListBox
                    TabIndex(0)
                    ScrollArea
                    Node {
                        width: px(600),
                        height: px(300),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        justify_content: JustifyContent::FlexEnd,
                        overflow: Overflow::scroll_y(),
                    }
                ),
                #TextInput
                @FeathersTextInputContainer
                Children [
                    @FeathersTextInput
                    TextInput
                ],
            ],
        ]
        on(|on_drag_start: On<Pointer<DragStart>>,
         mut drag_query: Query<&mut DragData>,
         transform_query: Query<&UiTransform>| {
            if let Ok(mut drag_data) = drag_query.get_mut(on_drag_start.event_target()) {
                if let Ok(transform) = transform_query.get(on_drag_start.entity) {
                    let current_x = match transform.translation.x {
                        Val::Px(x) => x,
                        _ => 0.0,
                    };
                    let current_y = match transform.translation.y {
                        Val::Px(y) => y,
                        _ => 0.0,
                    };
                    // Store offset: pointer position minus element position
                    drag_data.0 = on_drag_start.pointer_location.position
                        - Vec2::new(current_x, current_y);
                }
            }
        })
        on(|on_drag: On<Pointer<Drag>>,
         mut query: Query<&mut UiTransform>,
         drag_query: Query<&DragData>| {
            if let Ok(mut transform) = query.get_mut(on_drag.event_target()) {
                let offset = drag_query.get(on_drag.entity).unwrap();
                // Subtract offset from pointer location
                let new_pos = on_drag.pointer_location.position - offset.0;
                transform.translation = Val2::px(new_pos.x, new_pos.y);
            }
        })
    });
}

/// Singleton container node for messages
/// Children are different console messages or custom node blocks
/// Console messages can be accessed by querying the `ConsoleMessage` component.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct MessageContainer;
