---
id: system-appliance-front-panel
type: doctrine
status: active
locus: arcadia/system
aliases:
  - appliance-front-panel
  - one-control-per-job
  - diagnostics-drill-in
description: Arcadia System viewport doctrine for preserving full machine capability while presenting a HomeConsole appliance front panel.
---

# System Appliance Front Panel

Arcadia System is the HomeConsole front panel for machine maintenance. It is not a website, a backend dump, a developer console, or a row of buttons because endpoints exist.

The System viewport keeps the capabilities and changes the exposure:

- Power actions stay visible because they are physical machine actions: restart console, shut down, restart interface, restart game session.
- Remote access is one appliance concept, not raw SSH machinery. It shows current state, address, login policy, and one next lawful action.
- Secure web access is one appliance concept, not trust implementation debris. It shows current mode, Home Root CA state, and one next lawful action.
- System health summarizes service truth using appliance names. Raw units and duplicated implementation nouns stay out of the front panel.
- Diagnostics is the drill-in for support evidence, logs, service rows, and receipts. Empty View/Copy/Download buttons are forbidden.

One job has one visible control. If a backend can enable and disable the same capability, the UI renders current state plus the next useful action, never permanent enable/disable pairs. If two rows point at the same service or endpoint, they collapse into the appliance concept the user recognizes.

Forbidden System substrate:

- duplicate restart buttons for the same target;
- Enable/Disable pairs shown side by side;
- HTTP/HTTPS pairs shown side by side;
- empty modal buttons, empty copy buttons, or fake download controls;
- `Arcadia`, `Web GUI`, `GameScope`, `Samba`, and duplicate inference rows as visible front-panel nouns when they are only implementation seams;
- fake green or readiness claims not backed by live state.

Acceptance: the viewport is recognizable before reading prose, every visible control is unique and wired, diagnostics contains real evidence or truthful unavailable state, and tests guard both the desired structure and the forbidden lazy patterns.
