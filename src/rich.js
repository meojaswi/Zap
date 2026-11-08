// Formatted-snippet helpers shared by the main window and the search box.

// Formatting a snippet can carry; everything else is unwrapped to its text, and links keep only
// web or mail addresses.
const RICH_TAGS = new Set(["B", "STRONG", "I", "EM", "U", "A", "BR", "P", "DIV", "UL", "OL", "LI"]);
const DROPPED_TAGS = new Set(["SCRIPT", "STYLE", "IFRAME", "OBJECT", "EMBED", "TEMPLATE", "HEAD", "TITLE", "META", "LINK"]);

function sanitizeHtml(html) {
  const template = document.createElement("template");
  template.innerHTML = html;
  const clean = (node) => {
    for (const child of [...node.childNodes]) {
      if (child.nodeType === Node.TEXT_NODE) continue;
      if (child.nodeType !== Node.ELEMENT_NODE || DROPPED_TAGS.has(child.tagName)) {
        child.remove();
        continue;
      }
      clean(child);
      if (!RICH_TAGS.has(child.tagName)) {
        child.replaceWith(...child.childNodes);
        continue;
      }
      const href = child.tagName === "A" ? (child.getAttribute("href") || "").trim() : "";
      for (const attr of [...child.attributes]) child.removeAttribute(attr.name);
      if (/^(https?:|mailto:)/i.test(href)) child.setAttribute("href", href);
    }
  };
  clean(template.content);
  return template.innerHTML;
}

function plainToHtml(text) {
  const box = document.createElement("div");
  box.textContent = text;
  return box.innerHTML.replace(/\n/g, "<br>");
}

/** Advice for a trigger starting with a symbol, as HTML, or "" when there's nothing to say. */
function triggerAdvice(trigger, prefix) {
  const chars = [...trigger];
  if (chars.length < 2 || /^[\p{L}\p{N}]/u.test(trigger)) return "";
  const symbol = plainToHtml(chars[0]);
  const rest = plainToHtml(chars.slice(1).join(""));
  return prefix
    ? `Some websites block <code>${symbol}</code>. There, type <code>${plainToHtml(prefix)}${rest}</code> instead.`
    : `Some websites block <code>${symbol}</code>. Set a secondary trigger prefix in Settings to also type this as letters, like <code>zz${rest}</code>.`;
}

/** Connects a Generate button: the text box's directions go to the AI and its answer replaces them. */
function wireGenerate({ button, undo, fields, getText, setText, blocked, showError }) {
  let last = null;
  const label = button.textContent;
  const reset = () => {
    last = null;
    undo.hidden = true;
  };
  fields.forEach((field) => field.addEventListener("input", reset));
  button.addEventListener("click", async () => {
    const stop = blocked();
    if (stop) return showError(stop);
    const directions = getText();
    button.disabled = true;
    button.textContent = "Generating\u2026";
    try {
      const text = await window.__TAURI__.core.invoke("generate_text", { directions });
      // The user may have moved on while it was writing; don't overwrite what's there now.
      if (getText() === directions) {
        setText(text);
        last = { directions, text: getText() };
        undo.hidden = false;
      }
    } catch (e) {
      showError(e);
    } finally {
      button.disabled = false;
      button.textContent = label;
    }
  });
  undo.addEventListener("click", () => {
    if (last && getText() === last.text) setText(last.directions);
    reset();
  });
  return { reset };
}
