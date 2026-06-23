/// Normalized layout contract for the programmable gamepad primitive.
/// All interactive controls SHALL occupy disjoint rectangles inside the body box.
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

/// Authoritative slot map for overlap audits and `data-gamepad-slot` wiring.
pub const GAMEPAD_INTERACTIVE_SLOTS: &[GamepadSlot] = &[
    GamepadSlot { id: "shoulder-l2", control: "L2", x: 0.04, y: 0.00, w: 0.085, h: 0.10 },
    GamepadSlot { id: "shoulder-l1", control: "L1", x: 0.155, y: 0.00, w: 0.085, h: 0.10 },
    GamepadSlot { id: "shoulder-r1", control: "R1", x: 0.76, y: 0.00, w: 0.085, h: 0.10 },
    GamepadSlot { id: "shoulder-r2", control: "R2", x: 0.875, y: 0.00, w: 0.085, h: 0.10 },
    GamepadSlot { id: "stick-left", control: "Left Stick X", x: 0.03, y: 0.14, w: 0.22, h: 0.31 },
    GamepadSlot { id: "dpad", control: "D-pad", x: 0.03, y: 0.52, w: 0.22, h: 0.34 },
    GamepadSlot { id: "system-select", control: "Select", x: 0.34, y: 0.40, w: 0.08, h: 0.10 },
    GamepadSlot { id: "system-start", control: "Start", x: 0.44, y: 0.40, w: 0.08, h: 0.10 },
    GamepadSlot { id: "face-y", control: "Y", x: 0.66, y: 0.12, w: 0.11, h: 0.11 },
    GamepadSlot { id: "face-x", control: "X", x: 0.58, y: 0.26, w: 0.11, h: 0.11 },
    GamepadSlot { id: "face-b", control: "B", x: 0.74, y: 0.26, w: 0.11, h: 0.11 },
    GamepadSlot { id: "face-a", control: "A", x: 0.66, y: 0.44, w: 0.11, h: 0.11 },
    GamepadSlot {
        id: "stick-right",
        control: "Right Stick X",
        x: 0.78,
        y: 0.58,
        w: 0.18,
        h: 0.32,
    },
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
    a.x + a.w + gap <= b.x - gap
        || b.x + b.w + gap <= a.x - gap
        || a.y + a.h + gap <= b.y - gap
        || b.y + b.h + gap <= a.y - gap
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
}