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
