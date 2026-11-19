// HJKL Quest's loading screen: a vim tip, and how far the game's download
// has got. It counts the bytes of the .wasm as they arrive, against its
// size unpacked (WASM_SIZE, filled in by web/splash.py: the server sends
// it compressed, so the response's own length would not match).
(() => {
  const WASM_SIZE = __WASM_SIZE__;
  const tips = [
    "<kbd>5j</kbd> moves five lines down: counts work with almost every command.",
    "<kbd>w</kbd> and <kbd>b</kbd> hop whole words; <kbd>e</kbd> lands on a word's end.",
    "<kbd>f</kbd> then a letter jumps to it on the line. <kbd>;</kbd> jumps to the next one.",
    "<kbd>ciw</kbd> changes the word under the cursor, wherever in it you are.",
    "<kbd>.</kbd> repeats your last change. Make each change one you can repeat.",
    "<kbd>u</kbd> undoes and <kbd>Ctrl-r</kbd> redoes, on every isle.",
    "<kbd>%</kbd> jumps between matching brackets.",
    "<kbd>:%s/old/new/g</kbd> replaces everywhere at once.",
    "<kbd>qa</kbd> records a macro, <kbd>@a</kbd> plays it back.",
    "<kbd>A</kbd> appends at the end of the line: move and type in one key.",
    "<kbd>Ctrl-o</kbd> jumps back to where you were before the last jump.",
  ];
  const mb = (n) => (n / 1048576).toFixed(1);
  let got = 0;
  const show = () => {
    const fill = document.getElementById("hq-fill");
    const step = document.getElementById("hq-step");
    if (!fill || !step) return;
    if (got >= WASM_SIZE) {
      document.getElementById("hq-bar").classList.add("hq-busy");
      step.textContent = "Building the isles…";
    } else {
      fill.style.width = `${Math.max(4, (got / WASM_SIZE) * 100)}%`;
      step.textContent = got ? `Raising the isles… ${mb(got)} / ${mb(WASM_SIZE)} MB` : "Raising the isles…";
    }
  };
  document.addEventListener("DOMContentLoaded", () => {
    const tip = document.getElementById("hq-tip");
    if (tip) tip.innerHTML = "Tip: " + tips[Math.floor(Math.random() * tips.length)];
    show();
  });
  // The page asks for the .wasm right after this script: count its bytes
  // as they stream in, and hand the same bytes on unchanged.
  const fetch0 = window.fetch;
  window.fetch = function (input, init) {
    const request = fetch0.call(this, input, init);
    const url = typeof input === "string" ? input : input?.url ?? "";
    if (!/\.wasm(\?|$)/.test(url) || typeof ReadableStream === "undefined") return request;
    return request.then((response) => {
      if (!response.ok || !response.body) return response;
      const reader = response.body.getReader();
      // Read ahead of the page: the bar follows the network, not the compiler.
      const counted = new ReadableStream({
        async pull(controller) {
          const { done, value } = await reader.read();
          if (done) {
            got = WASM_SIZE;
            show();
            controller.close();
            return;
          }
          got += value.byteLength;
          show();
          controller.enqueue(value);
        },
        cancel(reason) { return reader.cancel(reason); },
      }, { highWaterMark: 1 << 30, size: (chunk) => chunk.byteLength });
      const headers = new Headers(response.headers);
      // The bytes are already unpacked: no length or encoding to trust.
      headers.delete("content-length");
      headers.delete("content-encoding");
      return new Response(counted, { status: response.status, statusText: response.statusText, headers });
    });
  };
})();
