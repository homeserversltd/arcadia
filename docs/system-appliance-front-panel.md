# System appliance front panel

Arcadia System is the HomeServer console view for machine maintenance. It is an appliance front panel, not a backend dump, developer console, or a row of buttons added solely because an endpoint exists.

## Product behavior

The System view preserves capability while presenting it in customer-recognizable concepts:

- Power actions remain visible because they are physical appliance actions: restart the console, shut down, restart the interface, or restart a game session.
- Remote access is one appliance concept rather than raw remote-shell machinery. It presents current state, address, login policy, and one appropriate next action.
- Secure web access is one appliance concept rather than implementation debris. It presents current mode, certificate trust state, and one appropriate next action.
- System health summarizes service state using appliance names. Raw units and duplicate implementation nouns remain out of the front panel.
- Diagnostics is the drill-in for support evidence, logs, service rows, and saved reports. Empty View, Copy, or Download buttons do not belong in the UI.

One job has one visible control. When a backend can enable and disable the same capability, the UI renders current state plus the next useful action, never a permanent enable/disable pair. When two rows address the same service or endpoint, they collapse into the appliance concept a customer recognizes.

## Avoided patterns

- Duplicate restart buttons for the same target.
- Enable/Disable pairs shown side by side.
- HTTP/HTTPS pairs shown side by side.
- Empty modal buttons, copy buttons, or fake download controls.
- Implementation names as front-panel nouns when they do not describe a customer task.
- Green or ready claims not backed by live state.

## Acceptance

The view is recognizable before the user reads supporting prose. Every visible control is unique and wired. Diagnostics contains real evidence or a truthful unavailable state. Tests protect both the intended structure and the avoided patterns.
