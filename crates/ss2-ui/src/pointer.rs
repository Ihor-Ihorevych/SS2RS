//! Pointer (mouse/touch-like) handling for the menu.
//!
//! Rules: hovering a focusable item focuses it; pressing on a slider track sets the value;
//! otherwise an item activates only when the pointer is released over the same item it was
//! pressed on.

use crate::geometry::Vec2;
use crate::layout::{SLIDER_TRACK_W, SLIDER_TRACK_X};
use crate::menu::{
    Item, Menu, MenuCommand, SliderSetting, UiInput, focusable, items, set_slider, slider_value,
};
use crate::menu::{MAX_STEPS, STEP};

impl Menu {
    pub(crate) fn handle_pointer(&mut self, input: UiInput) -> Vec<MenuCommand> {
        let list = items(self.screen());
        let rects = self.item_rects();
        let hit = |p: Vec2| {
            rects
                .iter()
                .enumerate()
                .find(|(i, r)| r.contains(p) && focusable(&list[*i]))
                .map(|(i, _)| i)
        };
        match input {
            UiInput::PointerMoved(p) => {
                if let Some(i) = hit(p) {
                    self.focus = i;
                }
                Vec::new()
            }
            UiInput::PointerPressed(p) => {
                self.pressed = None;
                let Some(i) = hit(p) else { return Vec::new() };
                self.focus = i;
                if let Item::Slider { setting, .. } = list[i] {
                    let track_x = rects[i].x + SLIDER_TRACK_X;
                    if p.x >= track_x && p.x < track_x + SLIDER_TRACK_W {
                        return self.set_slider_fraction(setting, (p.x - track_x) / SLIDER_TRACK_W);
                    }
                }
                self.pressed = Some(i);
                Vec::new()
            }
            UiInput::PointerReleased(p) => match (self.pressed.take(), hit(p)) {
                (Some(a), Some(b)) if a == b => {
                    self.focus = a;
                    self.activate()
                }
                _ => Vec::new(),
            },
            UiInput::Nav(_) => Vec::new(),
        }
    }

    fn set_slider_fraction(&mut self, setting: SliderSetting, fraction: f32) -> Vec<MenuCommand> {
        let new = ((fraction * MAX_STEPS as f32).round() as i32).clamp(0, MAX_STEPS);
        let old = (slider_value(&self.settings, setting) / STEP).round() as i32;
        if new == old {
            return Vec::new();
        }
        set_slider(&mut self.settings, setting, new as f32 * STEP);
        vec![MenuCommand::SettingsChanged(self.settings.clone())]
    }
}
