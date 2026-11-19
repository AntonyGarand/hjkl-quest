// `nsh test`: opens the Rust app headless and walks into the first isle.
import { expect, open, test } from "nsh/test";

test("the map lists the isles and the first one opens", async () => {
  const app = await open(".");
  expect(app.texts()).toContain("HJKL QUEST");
  expect(app.texts()).toContain("First Steps");
  await app.keyboard.press("enter");
  await app.waitFor("NORMAL");
  expect(app.texts()).toContain("1. First Steps");
  // A command from a later isle is locked and costs no keystroke.
  await app.keyboard.press("w");
  await app.waitFor("You haven't learned `w` yet");
});

test("the golf solution stays hidden until hovered", async () => {
  const app = await open(".", { platform: "web", width: 1280, height: 800 });
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
