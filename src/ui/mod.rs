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

/// Needed to handle drag events, stores the translation from before the drag
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
        @subpane() Children [
            @subpane_header() Children [
                Text("Console") ThemedText
            ]
            --
            @subpane_body() Children [
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
                --
                #TextInput
                @FeathersTextInputContainer
                Children [
                    @FeathersTextInput
                    TextInput
                ]
            ]
        ]
        on(|on_drag_start: On<PointerDragStart>,
         mut query: Query<(&mut DragData, &UiTransform)>| {
            if let Ok((mut drag_data, transform)) = query.get_mut(on_drag_start.entity) {
                let Val2 { x, y } = transform.translation;
                let px = |val| match val {
                    Val::Px(px) => px,
                    _ => 0.0,
                };
                drag_data.0 = Vec2::new(px(x), px(y));
            }
        })
        on(|on_drag: On<PointerDrag>,
         mut query: Query<(&DragData, &mut UiTransform)>| {
            if let Ok((drag_data, mut transform)) = query.get_mut(on_drag.entity) {
                // The distance is measured from where the pointer was pressed. The position of
                // PointerDragStart isn't, as the pointer has already moved by then.
                let new_pos = drag_data.0 + on_drag.distance;
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
