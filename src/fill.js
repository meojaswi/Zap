const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const form = document.getElementById("fill-form");
const title = document.getElementById("fill-title");
let labels = [];

function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML.replace(/"/g, "&quot;");
}

function inputs() {
  return [...form.querySelectorAll("input")];
}

function show({ name, labels: next }) {
  labels = next;
  title.textContent = name;
  form.innerHTML = labels
    .map(
      (label, i) => `
      <div class="field">
        <label for="f${i}">${escapeHtml(label)}</label>
        <input class="input" id="f${i}" spellcheck="false">
      </div>`
    )
    .join("");
  inputs()[0]?.focus();
}

function submit() {
  const values = Object.fromEntries(labels.map((label, i) => [label, inputs()[i].value]));
  invoke("fill_submit", { values });
}

form.addEventListener("submit", (e) => e.preventDefault());

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    e.preventDefault();
    invoke("fill_cancel");
  } else if (e.key === "Enter") {
    e.preventDefault();
    const fields = inputs();
    const at = fields.indexOf(document.activeElement);
    // Enter moves to the next field; on the last one it inserts the snippet.
    if (at >= 0 && at < fields.length - 1) fields[at + 1].focus();
    else submit();
  }
});

// No browser-style context menu anywhere in the window.
document.addEventListener("contextmenu", (e) => e.preventDefault());

listen("fill-request", (event) => show(event.payload));
