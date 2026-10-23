# ♿ Typefinch accessibility

## Available behavior
- **Appearance:** System theme is the default; explicit Light and Dark settings remain available independently of all profile and card themes.
- **Reduced motion:** An opt-in saved preference disables egui's standard UI transition duration. It does not alter typing-test timing or the displayed statistics. Ordinary interface use has no required decorative animation.
- **Keyboard access:** F1 toggles the on-screen guide, F2 focuses typing, F5 starts/restarts a test, Escape cancels a focused running test, and F6/F7/F8/F9 trigger explicit local exports. Eg UI widgets use egui's standard Tab/Shift+Tab focus navigation, with Enter/Space for focused actions.
- **Contrast:** Theme-specific ink is used for active, error, correct and untyped characters. Automated light/dark screenshots and color-ratio unit tests run in CI.
- **Security:** A running test cancels on window focus loss rather than recording hidden key events.

## Unverified acceptance
A complete keyboard-only run through every control, accessibility-tree output, screen-reader labeling, reduced-motion evaluation on real Hyprland hardware, all contrast states, and broad Unicode font coverage are not yet independently accepted. Do not report those gates as complete on the basis of the guide or style settings alone.

- **Custom text input:** A keyboard-operated local path field and Load local file button complement drag-and-drop. Input size/encoding validation is shared between both paths.

- **Goal transfer:** Profile goals can be transferred using two ordinary keyboard-focusable buttons and a local path field. These controls do not depend on a mouse file picker.
