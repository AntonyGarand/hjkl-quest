# HJKL Quest

Learn vim by walking the **Buffer Archipelago**: fifty-seven floating isles, each made of text, each teaching a few
commands. The world *is* the buffer: every character is a tile, the cursor is a small wisp that hops between
them, and every command you learn moves you (or reshapes the land) exactly as it would in vim.

**Play in the browser:** https://vim.garand.dev/ (on a phone or tablet, tap ⌨ for the
keyboard: a bar above it adds Esc, Ctrl, Tab and `:`)

Built in Rust with [Native Shell (nsh)](https://nsh.dev): the same crate runs as a web app (WebAssembly) and as a
native desktop app on Windows, macOS and Linux.

## How it plays

- **Sail a world map.** The isles lie on a route across the sea, their chapters grouped in regions by topic (the
  Shallows for moving, the Searching Straits, the Mending Coast for editing, the Ex Highlands, the Power Isles,
  the Mainland, and Beyond the Map for the challenges). Pip the ferry owl sails it with `h`/`l` (isle by isle),
  `j`/`k` (chapter by chapter) and `J`/`K` (region by region). Cleared isles fly a flag, isles past the next one
  to clear are in fog, and boss lairs mark each chapter's last isle.
- **A toolkit from every boss.** Beating a chapter's boss shows commands like the ones it taught, unlocked from
  then on: `H M L + -` after the Meadow, `{n}|` and `_` after the Word Bridges, `ge g_ ( )` after the Tower,
  `gi gp Ctrl-r` after the Library, `& g&`, pattern groups and `|` after the Summit…
- **Free play** (`f` on the map, on a cleared isle): the isle again with every key you know and no lesson to use,
  with its own best score.
- **A story and a look per chapter.** Pip greets you at each chapter's first isle; each chapter has its own sky,
  ground and weather (fireflies in the Meadow, embers in the Workshop, snow on the Glacier, rain on the Summit…).
- **Bosses.** Each chapter's last isle is guarded by a boss (the Sheep King, the Rust Golem, the Frost Wyrm…)
  with a health bar: every crystal, bug or fixed character is a hit, and it flinches.
- **Typo bugs** crawl one tile per command and hatch twins every 8 commands; land on them to squash them. Two
  isles (Bug Hunt, Infestation) are about predicting where they'll be: `3fb`, `/bug⏎ n n`.
- **Hints for half-typed commands.** Type `d`, `c`, `y`, `g`, `"`, `f`, `ci`… and a card lists what can come
  next, from the keys you've learned: `dd` the line, `w` to the next word, `i{obj}` a text object…
- **Your vim grammar** (`v` on the map): a grid of 8 operators by 35 motions and text objects. Every pair you use
  (`dw`, `ci"`, `gUiw`, `>}`…) lights up, with a toast the first time.
- **Replays** (`p` on the completion card): your run and the golf line side by side, key by key, with the habits
  the coach spotted.
- **Time trials**: a clock runs from your first key, and each isle keeps its best time.
- **The daily tide** (`d` on the map): a new editing puzzle every day, generated from the date, the same for
  everyone; the setter's keystrokes are the par.
- **Review drills** (`r` on the map): five drills from the isles you cleared longest ago, built from their
  examples, to keep what they teach fresh.
- **Move with vim keys only.** Each isle unlocks new commands; keys you haven't learned yet are refused (and cost
  nothing).
- **Collect every crystal ◆** on the movement isles, or **make the text match the goal** on the editing isles.
  A line-then-character diff shows what's left: extra tiles glow red, a glowing slot and a `+text` label mark
  where missing text goes, missing lines appear as ghost rows, and the goal panel colors each character green
  (in place) or amber (missing), with a count of what's left to add and remove.
- **Rulers** number the lines (with each line's distance from the cursor, for counts like `5j`) and the
  columns (every 5), so distances can be judged at a glance.
- **Traps ▲**: spiked blocks on the movement isles. Land on one and it throws the wisp back where it came from
  (the keys still count): stepping `lllll` gets caught, while `5l`, `e` or `f{c}` jump clean over.
- **Lessons first.** The first time you open an isle, a card lists what it teaches and plays each command on a
  small looping example, run on the game's own vim engine. `:h` brings it back.
- **A habit coach** watches your commands and names the faster one you've already learned: `llll` → `4l` / `e`,
  `0i` → `I`, `$a` → `A`, `dwi` → `cw`, `d$` → `D`, `lh` (moves that cancel out)… The completion card lists the
  habits to break.
- **The address follows you.** On the web the current isle is in the URL (`vim.garand.dev/#looking-back`, `#map`
  on the map), so reloading restarts the isle, links open an isle directly, and back/forward move between them.
- **Use the lesson.** Each isle names its main ideas (Word Hopping: `w`, and `b` or `e`; Search Party: `/` and
  `n`…), ticked off as you use them. Reaching the goal without them fails the isle: no stars, and the card says
  what was missing. Variants worth trying (Punctuation Pass's `E`, Demolition's `D`) are marked optional and
  never fail an isle. Every golf solution is checked to use the required ones.
- **Vim golf.** Every isle has a *par*: the keystrokes of a short known solution. Finish at par or under for
  three stars, and compare your keys with the golf line afterwards (hidden until you hover it, so it never
  spoils a retry).
- A **minimap** in the corner shows the whole isle with its letters and line/column numbers, the crystals left, what's missing and the
  part of it on screen.
- `:q` returns to the map, `:e!` restarts the isle, `:n` skips it, `:mute` toggles sound. These work from the
  first isle and never cost keystrokes; real ex commands (`:s`, `:%s`, `:{n}`) unlock on isle 22.

| Chapter | Commands |
|---|---|
| I · The Meadow | `h j k l`, counts, `:q! :wq`, `:help` |
| II · The Word Bridges | `w b e`, `W B E`, `0 ^ $` (on a 10-keystroke budget) |
| III · The Tower | `gg G {n}G`, `f F t T ; ,`, `{ }`, and a bug hunt |
| IV · The Machine | `%`, `/ ? n N * #`, `Ctrl-o Ctrl-i Ctrl-g`, and an infestation |
| V · The Workshop | `x X r ~`, `u U Ctrl-r` (undo from here on), `d{motion} dd D`, counts on operators `2dd d3w`, `c{motion} C s S` |
| VI · The Library | `i a I A o O`, `y yy p P`, `.` |
| VII · The Observatory | text objects `iw aw i( a( i" …`, visual `v V`, `~ U u J` |
| VIII · The Summit | `:s`, `:%s///g`, `:{n}`, `:set ic hls is`, `\c`, `:noh`, `:s///gc`, and a refactor |
| IX · The Scriptorium | `:r FILE`, `:w FILE`, `v` + `:w`, `:r !ls`, `:!cmd`, `Tab`/`Ctrl-d` completion, switching files `:e FILE :e # :w` |
| X · The Vaults | registers `"a "0 "_ "A :reg`, marks `ma `` `a 'a `` |
| XI · The Clockwork | `Ctrl-a Ctrl-x`, macros `qa … q @a @@` |
| XII · The Terraces | `>> << >{motion}`, `gU gu g~`, replace mode `R` |
| XIII · The Glacier | visual block `Ctrl-v` with `I A $A c`, `gn cgn` with `.`, `it at ip ap` |
| XIV · The Mainland | real files: renaming a variable in Rust (`*` `ciw` `n` `.`), fixing JSON, reordering a changelog, sorting imports (`:sort u`) |
| XV · The Archive | `:g :v`, ranges `:4,6m0 :$t0 :'<,'>`, `:norm`, `:sort`, a grand refactor, and the ending |
| XVI · The Rhythm Reef | after the ending, turn-based: crystals that fade after so many commands, sentries that march a row per command, bugs |
| XVII · The Gauntlet | all of vim on hard isles: swapping names with pattern groups, numbering a list with a macro and `Ctrl-a`, stripping tags |
| XVIII · The Trials | each takes something away: `hjkl` banned, frozen lines no edit may touch, `x d` banned, a five-key budget |

Patterns are vim's: `.` `*` `\+` `\=` `\{n,m}` `[a-z]` `\d \w \s` `\< \>` `\|` `\( \)` `\1` `\zs \ze` and very
magic `\v`, with `& \1 \u \U` in replacements; `|` chains ex commands.

### Everything vimtutor teaches

Every command from `vimtutor`'s seven lessons has an isle: movement and `:q!` (1), `x i A` (1), `dw d$ dd`,
counts, `u U Ctrl-r` (2), `p r ce c$` (3), `Ctrl-g G gg / ? n N Ctrl-o Ctrl-i % :s :%s :s///gc` (4),
`:!cmd :w FILE v+:w :r FILE :r !ls` (5), `o O a e R y p :set ic is hls \c` (6) and command-line completion with
`Tab` and `Ctrl-d` (7). Files and the shell are simulated: each isle has a small shelf of files, and `:!` knows
`ls cat echo pwd date`. Two things don't fit a game: the real `:help` (`:h` reopens the isle's lessons instead)
and a `~/.vimrc`.

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
| `src/vim.rs` | A self-contained vim emulator: motions, operators, text objects, insert/replace/visual/block modes, search, registers, marks, macros, ex ranges with `:s :g :norm :m :t :sort`, undo/redo, `.` |
| `src/lessons.rs` | The commands each isle requires, and how a typed command is matched against them |
| `src/coach.rs` | The habit coach: slow command patterns and the faster command for each |
| `src/examples.rs` | The worked examples of each isle's intro, played through the engine |
| `src/intro.rs` | The intro card shown before an isle |
| `src/diff.rs` | How the buffer differs from the goal: extra characters, characters in place, and where missing text goes |
| `src/levels.rs` | The campaign: maps, crystals, goals, unlocked keys and each isle's golf solution |
| `src/game.rs` | A level being played: key locking, crystals (fading ones too), bugs, sentries, frozen lines, banned keys, free play, completion, stars. Its tests replay every golf solution and check it clears the isle at exactly par. |
| `src/audio.rs` | Sound effects and the ambient pad, synthesized into WAV at start-up |
| `src/story.rs` | Pip's lines, the bosses, each chapter's sky, ground, weather and toolkit, and the map's regions |
| `src/regex.rs` | Vim's patterns: a small backtracking matcher, and `:s` replacements with groups |
| `src/hints.rs` | What can follow a half-typed command |
| `src/grammar.rs` | Operators and motions as verbs and nouns, and how a command splits into them |
| `src/daily.rs` | The daily tide: a seeded puzzle generator |
| `src/review.rs` | Review drills built from the isles' examples |
| `src/worldmap.rs` | The world map and Pip's ferry |
| `src/replay.rs` | A run played back beside the golf line |
| `src/screens.rs` | The grammar grid, the boss banner and the hint card |
| `web/splash.*` | The web loading screen (night sky, a wisp hopping on h j k l, download progress, a vim tip), put into the page `nsh package` writes by `web/splash.py` in CI |
| `web/mobile.js` | The on-screen keyboard for phones and tablets: a ⌨ button, a bar of Esc, Ctrl, Tab and `:`, and typed text sent on as key presses (also put in by `web/splash.py`) |
| `src/lib.rs` | The Native Shell UI: the 3/4-isometric board on canvases, animated values for the wisp, camera and sky, and the menus |

CI (`.github/workflows/ci.yml`) runs the tests, deploys the web build to GitHub Pages and packages the desktop apps
for Windows, macOS and Linux as build artifacts.
