# HJKL Quest

Learn vim by walking the **Buffer Archipelago**: twenty floating isles, each made of text, each teaching a few
commands. The world *is* the buffer: every character is a tile, the cursor is a small wisp that hops between
them, and every command you learn moves you (or reshapes the land) exactly as it would in vim.

**Play in the browser:** https://antonygarand.github.io/hjkl-quest/

Built in Rust with [Native Shell (nsh)](https://nsh.dev): the same crate runs as a web app (WebAssembly) and as a
native desktop app on Windows, macOS and Linux.

## How it plays

- **Move with vim keys only.** Each isle unlocks new commands; keys you haven't learned yet are refused (and cost
  nothing).
- **Collect every crystal ◆** on the movement isles, or **make the text match the goal** on the editing isles.
  Tiles that still differ from the goal glow red, using a line-then-character diff, so only real mistakes light up.
- **Vim golf.** Every isle has a *par*: the keystrokes of a short known solution. Finish at par or under for
  three stars, and compare your keys with the golf line afterwards.
- A **minimap** in the corner shows the whole isle, the crystals left and the part of it on screen.
- `:q` returns to the map, `:e!` restarts the isle, `:n` skips it, `:mute` toggles sound. These work from the
  first isle and never cost keystrokes; real ex commands (`:s`, `:%s`, `:{n}`) unlock on isle 19.

| Chapter | Commands |
|---|---|
| I · The Meadow | `h j k l`, counts |
| II · The Word Bridges | `w b e`, `W B E`, `0 ^ $` |
| III · The Tower | `gg G {n}G`, `f F t T ; ,`, `{ }` |
| IV · The Machine | `%`, `/ n N * #` |
| V · The Workshop | `x X r`, `d{motion} dd D`, `c{motion} C s S` |
| VI · The Library | `i a I A o O`, `y yy p P`, `. u Ctrl-r` |
| VII · The Observatory | text objects `iw aw i( a( i" …`, visual `v V`, `~ U u J` |
| VIII · The Summit | `:s`, `:%s///g`, `:{n}`, and a final refactor |

## Run it

You need [nsh](https://nsh.dev/docs/installation) (0.5.1) and Rust. The web target also needs
`rustup target add wasm32-unknown-unknown` and `clang` on your `PATH`.

```sh
nsh run                         # desktop window, rebuilds on change
nsh run --target web            # in the browser
cargo run                       # straight on the desktop backend
nsh package --target web        # static site in .nsh/out/
nsh package --target windows    # or macos, linux
nsh test                        # engine, level and headless UI tests
```

For development, `HJKL_LEVEL=12 cargo run` opens an isle directly.

## Layout

| File | What it holds |
|---|---|
| `src/vim.rs` | A self-contained vim emulator: motions, operators, text objects, insert, visual, search, `:s`, undo/redo, `.` |
| `src/levels.rs` | The campaign: maps, crystals, goals, unlocked keys and each isle's golf solution |
| `src/game.rs` | A level being played: key locking, crystals, completion, stars. Its tests replay every golf solution and check it clears the isle at exactly par. |
| `src/audio.rs` | Sound effects and the ambient pad, synthesized into WAV at start-up |
| `src/lib.rs` | The Native Shell UI: the 3/4-isometric board on canvases, animated values for the wisp, camera and sky, and the menus |

CI (`.github/workflows/ci.yml`) runs the tests, deploys the web build to GitHub Pages and packages the desktop apps
for Windows, macOS and Linux as build artifacts.
