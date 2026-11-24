use crate::ui::input::*;
use bevy::{
    feathers::{
        containers::{subpane, subpane_body, subpane_header},
        controls::{FeathersTextInput, FeathersTextInputContainer},
        theme::ThemedText,
    },
    input_focus::{InputFocus, tab_navigation::TabIndex},
    prelude::*,
    text::EditableText,
    ui_widgets::{ListBox, ScrollArea},
};

pub mod input;
pub mod message;

/// The root of the console. Its [`Visibility`] opens and closes the console.
#[derive(Component, Clone, Default)]
pub struct ConsoleUI;

/// Needed to handle drag events
/// This could be branched off into a separate crate/component
#[derive(Component, Clone, Default)]
pub struct DragData(Vec2);

pub fn open_close_console(
    input_focus: Res<InputFocus>,
    input: Res<ButtonInput<KeyCode>>,
    text_inputs: Query<(), With<EditableText>>,
    mut visibility: Single<&mut Visibility, With<ConsoleUI>>,
) {
    if !input.just_pressed(KeyCode::KeyT) {
        return;
    }

    // Only typing matters. Something else usually has the focus, like the primary window.
    if input_focus
        .get()
        .is_some_and(|focused| text_inputs.contains(focused))
    {
        return;
    }

    **visibility = match **visibility {
        Visibility::Hidden => Visibility::Visible,
        _ => Visibility::Hidden,
    };
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
                        justify_content: JustifyContent::FlexStart,
                        overflow: Overflow::scroll(),
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
            if let Ok(mut drag_data) = drag_query.get_mut(on_drag_start.event_target())
                && let Ok(transform) = transform_query.get(on_drag_start.entity)
            {
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
