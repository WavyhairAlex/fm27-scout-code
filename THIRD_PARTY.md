# Third-party notices

FM27 Scout is MIT-licensed. It depends on and/or references the following projects.
Each project remains under its own license.

| Project | Use | License |
|---|---|---|
| [fmsave](https://github.com/rhiever/fmsave) | FM26/FM27 layout reference, development oracle; portions of the parser design are ported | MIT |
| [Tauri](https://tauri.app/) | macOS desktop shell, updater and process plugins | Apache-2.0 / MIT |
| [React](https://react.dev/) | UI | MIT |
| [TanStack Query / Table / Virtual](https://tanstack.com/) | data fetching, table and virtualization | MIT |
| [Tailwind CSS](https://tailwindcss.com/) | styling | MIT |
| [lucide-react](https://lucide.dev/) | icons | ISC |
| [Zustand](https://zustand.docs.pmnd.rs/) | UI state | MIT |
| [rusqlite](https://github.com/rusqlite/rusqlite) | SQLite bindings | MIT |
| [zstd-rs](https://github.com/gyscos/zstd-rs) | zstd bindings | MIT |
| [chrono](https://github.com/chronotope/chrono) | dates | MIT / Apache-2.0 |
| [clap](https://github.com/clap-rs/clap) | CLI | MIT / Apache-2.0 |
| [serde](https://serde.rs/) | JSON | MIT / Apache-2.0 |
| [FM Scout CA guide / CAE attribute weights](https://www.fmscout.com/a-guide-to-current-ability-in-football-manager.html) | Community reference for the approximate CA estimator; the weight values originate from the FM pre-game editor | Factual reference data; no code copied |

`shared/nation-names.csv` is a factual country/region name list keyed by the public editor
database ids published by the FM community (FM Base, sortitoutsi). It contains no player,
club, competition or other SI/SEGA database content.

The CA estimator uses the public FM editor attribute-weight table as reference data. The
values are factual weights published in the FM pre-game editor; no FM Scout code is bundled.

The application does not bundle Sports Interactive or SEGA databases, images, logos,
or other game assets. It reads only files selected by the user.
