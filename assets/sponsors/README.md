# Sponsor logos

Sources for the sponsor reel (`c/src/sponsors.c`, `src/sponsors.rs`).
`python3 tools/sim/logo.py` turns them into the greyscale bitmaps in
`c/src/logos.h` and `src/logos.rs`; its `LOGOS` table says how each one is
cut and sized. Needs Inkscape.

| File | Source |
|---|---|
| `badgeteam.svg` | the logo on <https://badge.team/> |
| `nicolai.svg` | the logo on <https://nicolaielectronics.nl/> |
| `allnet.svg` | <https://commons.wikimedia.org/wiki/File:ALLNET_Logo.svg> (public domain, trademarked) |

Saved as plain SVG (Inkscape `--export-plain-svg`, editor metadata
dropped); the drawings are unchanged. The logos are trademarks of their
owners and are here only to thank them on the badge.
