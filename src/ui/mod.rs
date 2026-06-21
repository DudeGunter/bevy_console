use crate::ui::input::*;
use bevy::{
    feathers::{
        containers::{subpane, subpane_body, subpane_header},
        controls::{FeathersListRow, FeathersListView},
        theme::ThemedText,
    },
    prelude::*,
    ui::{InteractionDisabled, Selected},
};

pub mod input;
pub mod message;

pub const CONSOLE_FONT_SIZE_RAW: f32 = 12.0;
pub const CONSOLE_FONT_SIZE: FontSize = FontSize::Px(CONSOLE_FONT_SIZE_RAW);

#[derive(Component, Clone, Default)]
pub struct ConsoleUI;

/// Needed to handle drag events
/// This could be branched off into a separate crate/component
#[derive(Component, Clone, Default)]
pub struct DragData(Vec2);

pub fn open_close_console(
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Visibility, With<crate::ui::ConsoleUI>>,
    select_check: SelectedBoxCheck,
) {
    if !select_check.any_selected() {
        // If a box is selected, don't open/close the console (key protection)
        return;
    }

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
        Node {
            max_height: px(130),
        }
        subpane() Children [
            subpane_header() Children [
                (Text("List") ThemedText),
            ],
            subpane_body() Children [
                MessageContainer
                @FeathersListView {
                    @rows: {bsn_list![
                        @FeathersListRow Children [(Text("First Worldddddddddddddddddddddddddddddddddddd") ThemedText)],
                        @FeathersListRow Children [(Text("Second Nature") ThemedText)],
                        @FeathersListRow Children [(Text("Third Degree") ThemedText)],
                        @FeathersListRow Children [(Text("Fourth Wall") ThemedText)],
                        @FeathersListRow Children [(Text("Fifth Column") ThemedText)],
                        @FeathersListRow Children [(Text("Sixth Sense") ThemedText)],
                        @FeathersListRow Children [(Text("Seventh Heaven") ThemedText)],
                        @FeathersListRow Children [(Text("Eighth Wonder") ThemedText)],
                        @FeathersListRow Children [(Text("Ninth Inning") ThemedText)],
                        @FeathersListRow Children [(Text("Tenth Amendment") ThemedText)],
                        @FeathersListRow Children [(Text("Eleventh Hour") ThemedText)],
                        @FeathersListRow Children [(Text("Twelfth Night") ThemedText)],
                    ]}
                }
                Node {
                    width: percent(100),
                    flex_grow: 1.0, // Expand to fill space
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::clip_y(),
                }
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

pub fn create_ui(mut commands: Commands) {
    let background_color = Color::BLACK.with_alpha(0.38);
    commands.spawn_scene(bsn! {
        #Console
        ConsoleUI
        Node {
            width: px(750),
            height: px(300),
            border: UiRect::all(px(2)),
            flex_direction: FlexDirection::ColumnReverse,
            overflow: Overflow::clip_y(),
            border_radius: BorderRadius::all(px(5)),
        }
        BorderColor::all(Color::BLACK.with_alpha(0.5))
        DragData
        GlobalZIndex(i32::MAX)
        BackgroundColor({background_color})
        Children [
            TextInputBox
            TextFont {
                font_size: CONSOLE_FONT_SIZE,
            }
            Node {
                width: percent(100),
                flex_grow: 0.0,   // Don't expand
                flex_shrink: 0.0, // Don't shrink
                flex_basis: px(CONSOLE_FONT_SIZE_RAW + 2.0),
            }
            BackgroundColor({background_color}),
            MessageContainer
            Node {
                width: percent(100),
                flex_grow: 1.0, // Expand to fill space
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip_y(),
            }
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
#[derive(Component, Reflect, Debug, Clone, Default)]
pub struct MessageContainer;
