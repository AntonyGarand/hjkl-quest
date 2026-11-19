// `nsh test`: opens the Rust app headless and walks into the first isle.
import { expect, open, test } from "nsh/test";

test("the map lists the isles and the first one opens", async () => {
  const app = await open(".");
  expect(app.texts()).toContain("HJKL QUEST");
  expect(app.texts()).toContain("First Steps");
  await app.keyboard.press("enter");
  // The first time, the isle's lessons come first, with examples.
  await app.waitFor("YOU WILL LEARN");
  expect(app.texts()).toContain("SEE IT WORK");
  await app.keyboard.press("enter");
  await app.waitFor("NORMAL");
  expect(app.texts()).not.toContain("YOU WILL LEARN");
  expect(app.texts()).toContain("1. First Steps");
  // A command from a later isle is locked and costs no keystroke.
  await app.keyboard.press("w");
  await app.waitFor("You haven't learned `w` yet");
});

test("the golf solution stays hidden until hovered", async () => {
  const app = await open(".", { platform: "web", width: 1280, height: 800 });
  await app.keyboard.press("enter");
  await app.waitFor("YOU WILL LEARN");
  await app.keyboard.press("enter");
  await app.waitFor("NORMAL");
  // A route of par length that differs from the golf line.
  for (const key of "ljjllljjllll") await app.keyboard.press(key);
  await app.waitFor("hover to reveal");
  expect(app.texts()).not.toContain("jjlllljjllll");
  await app.hover("hover to reveal");
  await app.waitFor("jjlllljjllll");
  expect(app.texts()).not.toContain("hover to reveal");
});

test("the goal panel counts what is missing", async () => {
  const app = await open(".", { platform: "web", width: 1280, height: 800 });
  // Isle 15, Insert Coin: words to add, nothing to remove.
  for (let i = 0; i < 14; i++) await app.keyboard.press("j");
  await app.keyboard.press("enter");
  await app.waitFor("YOU WILL LEARN");
  await app.keyboard.press("enter");
  await app.waitFor("15. Insert Coin");
  await app.waitFor("+ 16 to add");
  expect(app.texts()).toContain("✕ 0 to remove");
  // Appending ":" to the first line leaves one less to add.
  await app.keyboard.press("shift+a");
  await app.keyboard.press("shift+;");
  await app.keyboard.press("escape");
  await app.waitFor("+ 15 to add");
});

test("control keys reach the editor", async () => {
  const app = await open(".", { platform: "web", width: 1280, height: 800 });
  // Isle 28, Tally Marks: Ctrl-a winds a number up.
  for (let i = 0; i < 27; i++) await app.keyboard.press("j");
  await app.keyboard.press("enter");
  await app.waitFor("YOU WILL LEARN");
  await app.keyboard.press("escape");
  await app.waitFor("28. Tally Marks");
  await app.keyboard.press("2");
  await app.keyboard.press("ctrl+a");
  await app.waitFor("2<C-a>");
});

test(":h brings the lessons back", async () => {
  const app = await open(".", { platform: "web", width: 1280, height: 800 });
  await app.keyboard.press("enter");
  await app.waitFor("YOU WILL LEARN");
  await app.keyboard.press("enter");
  await app.keyboard.press("shift+;");
  await app.keyboard.press("h");
  await app.keyboard.press("enter");
  await app.waitFor("YOU WILL LEARN");
});

async function openIsle(app, index: number) {
  for (let i = 0; i < index; i++) await app.keyboard.press("j");
  await app.keyboard.press("enter");
  await app.waitFor("YOU WILL LEARN");
  await app.keyboard.press("enter");
  await app.waitFor("NORMAL");
}

async function type(app, keys: string) {
  for (const k of keys) {
    const shifted: Record<string, string> = { ":": "shift+;", "%": "shift+5", "?": "shift+/" };
    await app.keyboard.press(shifted[k] ?? (k === " " ? "space" : k));
  }
}

test("backward search, Ctrl-o and Ctrl-g", async () => {
  const app = await open(".", { platform: "web", width: 1280, height: 800 });
  await openIsle(app, 10); // Looking Back
  await type(app, "?go");
  await app.keyboard.press("enter");
  await app.waitFor("1:3");
  await app.keyboard.press("ctrl+o");
  await app.waitFor("4:1");
  await app.keyboard.press("ctrl+g");
  await app.waitFor('"isle" 8 lines --50%-- line 4 of 8, col 1');
});

test("a confirmed substitute asks about each match", async () => {
  const app = await open(".", { platform: "web", width: 1280, height: 800 });
  await openIsle(app, 21); // Ask First
  await type(app, ":%s/cat/dog/gc");
  await app.keyboard.press("enter");
  await app.waitFor("replace with dog (y/n/a/q/l)?");
  await type(app, "yynyy");
  await app.waitFor("Exactly par. A true vim golfer.");
});

test("Tab completes a file name on the command line", async () => {
  const app = await open(".", { platform: "web", width: 1280, height: 800 });
  await openIsle(app, 23); // Scrolls
  await type(app, ":r i");
  await app.keyboard.press("tab");
  await app.waitFor(":r ingredients.txt▏");
  await app.keyboard.press("enter");
  await app.waitFor("Exactly par. A true vim golfer.");
});
