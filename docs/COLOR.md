# 🎨 Typefinch project color

- **Identity:** soft periwinkle
- **Hex sRGB:** `#8696F5`
- **CSS OKLCH:** `oklch(70.123% 0.14003 274.97)`
- **OKLab:** approximately `L=0.70123, a=0.01213, b=-0.13951`
- **README color swatch:** `assets/typefinch-color.svg`

This color is perceptually specified and sRGB-safe. Accent colors are not substitutes for contrast-tested light/dark text colors. Default application appearance is System, with Light and Dark overrides. Global registry uniqueness has not been checked.

## Accessibility and contrast audit

WCAG 2.x relative-luminance ratios below are tested against the actual SVG swatch and Rust UI color tokens with `lua5.4 tools/color_audit.lua --check`. The audit refuses color drift and checks every listed normal-text pairing meets the 4.5:1 AA threshold.

| Pairing | Text | Background | Contrast |
| --- | --- | --- | ---: |
| README swatch title/caption | `#202645` | `#8696F5` | 5.40:1 |
| Light UI accent ink | `#4559B5` | `#FFFFFF` | 6.27:1 |
| Dark UI accent ink | `#AAB5FF` | `#202645` | 7.55:1 |
| Light selection text | `#FFFFFF` | `#4559B5` | 6.27:1 |
| Dark selection text | `#202645` | `#8696F5` | 5.40:1 |

**Do not use white text directly on the project swatch:** white on `#8696F5` is only 2.73:1. The swatch uses dark ink and includes an SVG accessible image label. This is a scoped brand/semantic-color audit, not acceptance of all keyboard navigation, user-installed fonts, or platform screen-reader behavior; those remain separate tasks.
