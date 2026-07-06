/// Normalized hit-region contract for the programmable gamepad primitive.
/// Paint lives in the inline SVG gamepad body; these rectangles audit the generous
/// interactive regions that surround each anatomical control.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GamepadSlot {
    pub id: &'static str,
    pub control: &'static str,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Minimum normalized gap required between any two interactive hit targets.
pub const GAMEPAD_LAYOUT_MIN_GAP: f32 = 0.01;

/// Authoritative slot map for hit-region overlap audits and `data-gamepad-slot` wiring.
/// Coordinates are normalized to the SVG `viewBox="0 0 1000 620"`.
pub const GAMEPAD_INTERACTIVE_SLOTS: &[GamepadSlot] = &[
    GamepadSlot { id: "shoulder-l2", control: "L2", x: 0.1580, y: 0.0742, w: 0.1260, h: 0.0800 },
    GamepadSlot { id: "shoulder-l1", control: "L1", x: 0.2940, y: 0.1774, w: 0.1080, h: 0.0800 },
    GamepadSlot { id: "shoulder-r2", control: "R2", x: 0.6160, y: 0.0742, w: 0.1260, h: 0.0800 },
    GamepadSlot { id: "shoulder-r1", control: "R1", x: 0.7600, y: 0.1774, w: 0.1080, h: 0.0800 },
    GamepadSlot { id: "stick-left", control: "Left Stick X", x: 0.1800, y: 0.2935, w: 0.1500, h: 0.1700 },
    GamepadSlot { id: "dpad", control: "D-pad", x: 0.2060, y: 0.5655, w: 0.1220, h: 0.1600 },
    GamepadSlot { id: "system-select", control: "Select", x: 0.4120, y: 0.4581, w: 0.0580, h: 0.0700 },
    GamepadSlot { id: "system-start", control: "Start", x: 0.5020, y: 0.4581, w: 0.0580, h: 0.0700 },
    GamepadSlot { id: "face-y", control: "Y", x: 0.7100, y: 0.3232, w: 0.0440, h: 0.0660 },
    GamepadSlot { id: "face-x", control: "X", x: 0.6500, y: 0.4200, w: 0.0440, h: 0.0660 },
    GamepadSlot { id: "face-b", control: "B", x: 0.7700, y: 0.4200, w: 0.0440, h: 0.0660 },
    GamepadSlot { id: "face-a", control: "A", x: 0.7100, y: 0.5168, w: 0.0440, h: 0.0660 },
    GamepadSlot { id: "stick-right", control: "Right Stick X", x: 0.5440, y: 0.6000, w: 0.1500, h: 0.1700 },
];

pub fn gamepad_slot_for_control(control: &str) -> Option<&'static GamepadSlot> {
    GAMEPAD_INTERACTIVE_SLOTS
        .iter()
        .find(|slot| slot.control == control)
}

pub fn gamepad_slots_overlap(a: &GamepadSlot, b: &GamepadSlot, gap: f32) -> bool {
    if a.id == b.id {
        return false;
    }
    !gamepad_slots_separated(a, b, gap)
}

pub fn gamepad_slots_separated(a: &GamepadSlot, b: &GamepadSlot, gap: f32) -> bool {
    a.x + a.w + gap <= b.x
        || b.x + b.w + gap <= a.x
        || a.y + a.h + gap <= b.y
        || b.y + b.h + gap <= a.y
}

pub fn gamepad_layout_overlap_report(gap: f32) -> Vec<String> {
    let mut overlaps = Vec::new();
    for (index, left) in GAMEPAD_INTERACTIVE_SLOTS.iter().enumerate() {
        for right in GAMEPAD_INTERACTIVE_SLOTS.iter().skip(index + 1) {
            if gamepad_slots_overlap(left, right, gap) {
                overlaps.push(format!(
                    "{} ({}) overlaps {} ({}) at gap {gap}",
                    left.id, left.control, right.id, right.control
                ));
            }
        }
    }
    overlaps
}

pub fn gamepad_layout_is_separated(gap: f32) -> bool {
    gamepad_layout_overlap_report(gap).is_empty()
}

#[cfg(test)]
mod gamepad_layout_tests {
    use super::*;

    #[test]
    fn interactive_slots_never_overlap_at_contract_gap() {
        let overlaps = gamepad_layout_overlap_report(GAMEPAD_LAYOUT_MIN_GAP);
        assert!(
            overlaps.is_empty(),
            "gamepad layout overlap violations: {}",
            overlaps.join("; ")
        );
    }

    #[test]
    fn start_and_face_x_hit_regions_are_separated() {
        let start = gamepad_slot_for_control("Start").unwrap();
        let face_x = gamepad_slot_for_control("X").unwrap();
        assert!(
            gamepad_slots_separated(start, face_x, GAMEPAD_LAYOUT_MIN_GAP),
            "Start ({start:?}) must not overlap X ({face_x:?})"
        );
    }

    #[test]
    fn every_interactive_control_has_a_slot() {
        for control in [
            "L2", "L1", "R1", "R2", "Left Stick X", "D-pad", "Select", "Start", "Y", "X", "B", "A",
            "Right Stick X",
        ] {
            assert!(
                gamepad_slot_for_control(control).is_some(),
                "missing gamepad slot for {control}"
            );
        }
    }

    #[test]
    fn svg_hit_region_manifest_keeps_thirteen_slots() {
        assert_eq!(GAMEPAD_INTERACTIVE_SLOTS.len(), 13);
    }
}
