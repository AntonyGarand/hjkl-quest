// HJKL Quest on a phone or tablet: a button that opens the on-screen
// keyboard, and a bar of the keys it lacks (Esc, Ctrl, Tab).
//
// The game hears keys as key presses (keydown). On-screen keyboards often
// send none that say which key it was (Android sends "Unidentified"), so
// the keys come from what lands in a hidden textarea instead, and are sent
// on as the key presses a desktop keyboard would make.
(() => {
  const touch = matchMedia("(pointer: coarse)").matches || navigator.maxTouchPoints > 0;
  if (!touch) return;

  // US layout: the shifted characters and the key each is on.
  const SHIFTED = {
    "~": "Backquote", _: "Minus", "+": "Equal", "{": "BracketLeft", "}": "BracketRight", "|": "Backslash",
    ":": "Semicolon", '"': "Quote", "<": "Comma", ">": "Period", "?": "Slash",
    "!": "Digit1", "@": "Digit2", "#": "Digit3", $: "Digit4", "%": "Digit5", "^": "Digit6", "&": "Digit7",
    "*": "Digit8", "(": "Digit9", ")": "Digit0",
  };
  const PLAIN = {
    "`": "Backquote", "-": "Minus", "=": "Equal", "[": "BracketLeft", "]": "BracketRight", "\\": "Backslash",
    ";": "Semicolon", "'": "Quote", ",": "Comma", ".": "Period", "/": "Slash", " ": "Space",
  };
  // What phones type for the plain ASCII ones (smart quotes, dashes).
  const FOLD = { "‘": "'", "’": "'", "“": '"', "”": '"', "–": "-", "—": "-", " ": " ", "…": "..." };
  // Kept in the textarea so a backspace always has something to delete.
  const PAD = "​​";

  let ta, bar, button, ctrlKey, ctrl = false, last = PAD, composing = false;

  const keyInit = (ch) => {
    if (ch === "\n") return { key: "Enter", code: "Enter" };
    if (SHIFTED[ch]) return { key: ch, code: SHIFTED[ch], shiftKey: true };
    if (/^[A-Z]$/.test(ch)) return { key: ch, code: `Key${ch}`, shiftKey: true };
    if (/^[a-z]$/.test(ch)) return { key: ch, code: `Key${ch.toUpperCase()}` };
    if (/^[0-9]$/.test(ch)) return { key: ch, code: `Digit${ch}` };
    if (PLAIN[ch]) return { key: ch, code: PLAIN[ch] };
    return null;
  };

  const press = (init) => {
    if (ctrl && init.key.length === 1) {
      init = { key: init.key.toLowerCase(), code: init.code, ctrlKey: true };
      setCtrl(false);
    }
    const opts = { bubbles: true, cancelable: true, composed: true, ...init };
    (ta ?? window).dispatchEvent(new KeyboardEvent("keydown", opts));
    (ta ?? window).dispatchEvent(new KeyboardEvent("keyup", opts));
  };

  const type = (text) => {
    for (const raw of text) {
      for (const ch of FOLD[raw] ?? raw) {
        const init = keyInit(ch);
        if (init) press(init);
      }
    }
  };

  const reset = () => {
    ta.value = PAD;
    ta.setSelectionRange(PAD.length, PAD.length);
    last = PAD;
  };

  // What changed since the last look: deleted characters are backspaces,
  // new ones are typed. Mid-word (a keyboard composing a word) the field
  // is left alone until the word is done.
  const changed = () => {
    const now = ta.value;
    let same = 0;
    while (same < last.length && same < now.length && last[same] === now[same]) same++;
    for (let i = same; i < last.length; i++) press({ key: "Backspace", code: "Backspace" });
    type(now.slice(same).replaceAll("​", ""));
    last = now;
    if (!composing) reset();
  };

  const setCtrl = (on) => {
    ctrl = on;
    ctrlKey?.classList.toggle("on", on);
  };

  const open = () => {
    ta.focus({ preventScroll: true });
    reset();
  };

  // Keeps the bar just above the on-screen keyboard.
  const place = () => {
    const vv = window.visualViewport;
    if (!bar || !vv) return;
    bar.style.top = `${vv.offsetTop + vv.height - bar.offsetHeight}px`;
    bar.style.left = `${vv.offsetLeft}px`;
    bar.style.width = `${vv.width}px`;
  };

  const style = document.createElement("style");
  style.textContent = `
    #hq-kb-field { position: fixed; top: 0; left: 0; width: 2px; height: 2px; opacity: 0; border: 0; padding: 0;
      font-size: 16px; resize: none; caret-color: transparent; z-index: -1; }
    #hq-kb-open { position: fixed; right: 14px; bottom: calc(14px + env(safe-area-inset-bottom)); z-index: 1000;
      width: 54px; height: 54px; border-radius: 27px; border: 1px solid rgba(255, 215, 107, 0.6);
      background: rgba(11, 8, 32, 0.88); color: #ffd76b; font-size: 26px; line-height: 1;
      box-shadow: 0 4px 18px rgba(0, 0, 0, 0.45); touch-action: manipulation; -webkit-tap-highlight-color: transparent; }
    #hq-kb-bar { position: fixed; left: 0; top: 0; z-index: 1001; display: none; gap: 6px; padding: 6px 8px;
      box-sizing: border-box; background: rgba(11, 8, 32, 0.96); border-top: 1px solid rgba(201, 184, 255, 0.25); }
    #hq-kb-bar.shown { display: flex; }
    #hq-kb-bar button { flex: 1; min-width: 0; height: 40px; border-radius: 8px; border: 1px solid rgba(201, 184, 255, 0.3);
      background: rgba(255, 255, 255, 0.07); color: #ece6ff; font: 600 15px ui-monospace, Menlo, Consolas, monospace;
      touch-action: manipulation; -webkit-tap-highlight-color: transparent; user-select: none; -webkit-user-select: none; }
    #hq-kb-bar button:active, #hq-kb-bar button.on { background: rgba(255, 215, 107, 0.3); border-color: #ffd76b; color: #fff3b0; }
    #hq-kb-bar button.done { flex: 0 0 52px; color: #ffd76b; }
  `;
  document.head.append(style);

  // A tap on the bar keeps the keyboard up: no focus change, no click delay.
  const tap = (el, act) => {
    el.addEventListener("pointerdown", (e) => {
      e.preventDefault();
      e.stopPropagation();
      act();
    });
    el.addEventListener("mousedown", (e) => e.preventDefault());
  };

  const key = (label, act, cls) => {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = label;
    if (cls) b.className = cls;
    tap(b, act);
    bar.append(b);
    return b;
  };

  const install = (root) => {
    ta = document.createElement("textarea");
    ta.id = "hq-kb-field";
    ta.setAttribute("autocapitalize", "off");
    ta.setAttribute("autocomplete", "off");
    ta.setAttribute("autocorrect", "off");
    ta.setAttribute("spellcheck", "false");
    ta.setAttribute("enterkeyhint", "enter");
    ta.setAttribute("aria-label", "Keyboard for the game");
    // Inside the app's root: the app takes keys only while the focus is in it.
    root.append(ta);
    ta.addEventListener("compositionstart", () => (composing = true));
    ta.addEventListener("compositionend", () => {
      composing = false;
      changed();
    });
    ta.addEventListener("input", changed);
    ta.addEventListener("focus", () => {
      bar.classList.add("shown");
      button.style.display = "none";
      place();
    });
    // A tap on the board keeps the keyboard up (its ⌄ puts it away); a
    // tap on something of the app's that takes the focus takes it.
    let boardTap = 0;
    root.addEventListener(
      "pointerdown",
      (e) => {
        const takes = e.target instanceof Element && e.target.closest("[tabindex], a, button, input, textarea, .nsh-field");
        boardTap = takes && e.target !== ta ? 0 : performance.now();
      },
      true,
    );
    ta.addEventListener("blur", () => {
      if (performance.now() - boardTap < 1000) {
        boardTap = 0;
        ta.focus({ preventScroll: true });
        return;
      }
      bar.classList.remove("shown");
      button.style.display = "";
      setCtrl(false);
    });

    bar = document.createElement("div");
    bar.id = "hq-kb-bar";
    key("Esc", () => press({ key: "Escape", code: "Escape" }));
    ctrlKey = key("Ctrl", () => setCtrl(!ctrl));
    key("Tab", () => press({ key: "Tab", code: "Tab" }));
    key(":", () => type(":"));
    key("⌫", () => press({ key: "Backspace", code: "Backspace" }));
    key("⏎", () => press({ key: "Enter", code: "Enter" }));
    key("⌄", () => ta.blur(), "done");
    document.body.append(bar);

    button = document.createElement("button");
    button.id = "hq-kb-open";
    button.type = "button";
    button.textContent = "⌨";
    button.setAttribute("aria-label", "Open the keyboard");
    tap(button, open);
    document.body.append(button);

    window.visualViewport?.addEventListener("resize", place);
    window.visualViewport?.addEventListener("scroll", place);
  };

  // A key press from the on-screen keyboard that names a character is left
  // to the textarea (it comes back through `changed`, with the right
  // shift): phones often report "A" or ":" without shift, which the app
  // would read as "a" or ";". Keys with Ctrl, Alt or Meta, and named keys
  // (Enter, Backspace), go to the app as they are.
  window.addEventListener(
    "keydown",
    (e) => {
      if (!e.isTrusted || e.target !== ta || e.ctrlKey || e.altKey || e.metaKey) return;
      if (e.key.length === 1 || e.key === "Unidentified" || e.key === "Process") e.stopImmediatePropagation();
    },
    true,
  );

  const wait = () => {
    const root = document.querySelector(".nsh-root");
    if (root) install(root);
    else setTimeout(wait, 200);
  };
  wait();
})();
