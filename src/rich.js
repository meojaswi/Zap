// Formatted-snippet helpers shared by the main window and the search box.

// Formatting a snippet can carry; everything else is unwrapped to its text, and links keep only
// web or mail addresses.
const RICH_TAGS = new Set(["B", "STRONG", "I", "EM", "U", "A", "BR", "P", "DIV", "UL", "OL", "LI", "IMG"]);
const IMAGE_ID = /^[0-9a-f-]{36}$/i;
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
      if (child.tagName === "IMG") {
        // Only Zap's own pictures: an outside address or stored source never gets kept.
        const id = child.getAttribute("data-zap-id") || "";
        const width = child.getAttribute("width") || "";
        if (!IMAGE_ID.test(id)) {
          child.remove();
          continue;
        }
        for (const attr of [...child.attributes]) child.removeAttribute(attr.name);
        child.setAttribute("data-zap-id", id);
        if (/^\d{1,4}$/.test(width)) child.setAttribute("width", width);
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
    button.classList.add("busy");
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
      button.classList.remove("busy");
    }
  });
  undo.addEventListener("click", () => {
    if (last && getText() === last.text) setText(last.directions);
    reset();
  });
  return { reset };
}

let closeOpenLinkDialog = null;

function closeLinkDialog() {
  if (closeOpenLinkDialog) closeOpenLinkDialog();
}

/** Asks for a link's display text and address, then adds it to `editor`. An empty display text uses the address. */
function openLinkDialog(editor, afterInsert = () => {}) {
  closeLinkDialog();
  const selection = window.getSelection();
  const inEditor = selection.rangeCount > 0 && editor.contains(selection.anchorNode);
  const range = inEditor ? selection.getRangeAt(0).cloneRange() : null;
  const selectedText = range ? range.toString() : "";

  const back = document.createElement("div");
  back.className = "link-dialog-back";
  back.innerHTML = `
    <form class="link-dialog" role="dialog" aria-modal="true" aria-labelledby="link-dialog-title" novalidate>
      <h3 id="link-dialog-title">Link details</h3>
      <div class="field">
        <label for="link-text">Display text</label>
        <input class="input" id="link-text" placeholder="Leave empty to show the link" autocomplete="off" spellcheck="false">
      </div>
      <div class="field">
        <label for="link-url">Link</label>
        <input class="input" id="link-url" autocomplete="off" spellcheck="false">
      </div>
      <div class="form-error" id="link-error" hidden></div>
      <div class="actions">
        <button type="submit" class="btn primary">Add link</button>
        <button type="button" class="btn" id="link-cancel">Cancel</button>
      </div>
    </form>`;
  document.body.appendChild(back);
  const textField = back.querySelector("#link-text");
  const urlField = back.querySelector("#link-url");
  const error = back.querySelector("#link-error");
  textField.value = selectedText;
  urlField.value = "https://";

  const close = () => {
    back.remove();
    closeOpenLinkDialog = null;
  };
  closeOpenLinkDialog = close;

  const backToEditor = () => {
    close();
    editor.focus();
    const current = window.getSelection();
    current.removeAllRanges();
    if (range) {
      current.addRange(range);
    } else {
      current.selectAllChildren(editor);
      current.collapseToEnd();
    }
  };

  back.querySelector("form").addEventListener("submit", (e) => {
    e.preventDefault();
    const url = urlField.value.trim();
    if (!/^(https?:\/\/\S+|mailto:\S+)$/i.test(url)) {
      error.textContent = "Use a full address starting with https://, http:// or mailto:";
      error.hidden = false;
      urlField.focus();
      return;
    }
    const text = textField.value.trim() || url;
    backToEditor();
    if (selectedText && text === selectedText.trim()) {
      // The wording didn't change, so keep any bold or italic inside it.
      document.execCommand("createLink", false, url);
    } else {
      const link = document.createElement("a");
      link.href = url;
      link.textContent = text;
      document.execCommand("insertHTML", false, link.outerHTML);
    }
    afterInsert();
  });
  back.querySelector("#link-cancel").addEventListener("click", backToEditor);
  back.addEventListener("mousedown", (e) => {
    if (e.target === back) backToEditor();
  });
  back.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      backToEditor();
    }
  });
  (selectedText ? urlField : textField).focus();
  if (selectedText) urlField.setSelectionRange(urlField.value.length, urlField.value.length);
}

const MAX_IMAGE_BYTES = 1024 * 1024;
const MAX_IMAGE_SIDE = 1600;
const IMAGE_SIZES = [["Small", 120], ["Medium", 240], ["Large", 400]];

function blobToBase64(blob) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result).split(",")[1]);
    reader.onerror = reject;
    reader.readAsDataURL(blob);
  });
}

function canvasBlob(canvas, type, quality) {
  return new Promise((resolve) => canvas.toBlob(resolve, type, quality));
}

/** Shrinks a picture until it fits the 1 MB limit, keeping PNG (and transparency) while it can. */
async function fitImage(file, bitmap) {
  const longest = Math.max(bitmap.width, bitmap.height);
  if (file.size <= MAX_IMAGE_BYTES && longest <= MAX_IMAGE_SIDE) return file;
  let scale = Math.min(1, MAX_IMAGE_SIDE / longest);
  for (let i = 0; i < 12; i += 1, scale *= 0.8) {
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.round(bitmap.width * scale));
    canvas.height = Math.max(1, Math.round(bitmap.height * scale));
    const context = canvas.getContext("2d");
    const png = await (context.drawImage(bitmap, 0, 0, canvas.width, canvas.height), canvasBlob(canvas, "image/png"));
    if (png && png.size <= MAX_IMAGE_BYTES) return png;
    // JPG has no transparency, so the picture goes on white.
    context.globalCompositeOperation = "destination-over";
    context.fillStyle = "#ffffff";
    context.fillRect(0, 0, canvas.width, canvas.height);
    const jpeg = await canvasBlob(canvas, "image/jpeg", 0.85);
    if (jpeg && jpeg.size <= MAX_IMAGE_BYTES) return jpeg;
  }
  throw new Error("too large");
}

/** Adds an Image button, paste support and size controls to a formatted-text editor. */
function wireImages({ editor, toolbar, showError, keepOpen = () => {} }) {
  const invoke = window.__TAURI__.core.invoke;
  const button = document.createElement("button");
  button.type = "button";
  button.className = "chip";
  button.textContent = "Image";
  button.title = "Add a PNG or JPG picture";
  const picker = document.createElement("input");
  picker.type = "file";
  picker.accept = ".png,.jpg,.jpeg,image/png,image/jpeg";
  picker.hidden = true;
  picker.className = "image-file";
  const sizes = document.createElement("span");
  sizes.className = "image-sizes";
  sizes.hidden = true;
  toolbar.append(button, sizes, picker);

  let selected = null;
  let range = null;

  function select(image) {
    if (selected) selected.classList.remove("image-selected");
    selected = image;
    if (image) image.classList.add("image-selected");
    sizes.hidden = !image;
  }

  const control = (label, action) => {
    const item = document.createElement("button");
    item.type = "button";
    item.className = "chip";
    item.textContent = label;
    item.addEventListener("mousedown", (e) => {
      e.preventDefault();
      if (selected) action(selected);
    });
    sizes.append(item);
  };
  IMAGE_SIZES.forEach(([label, width]) => control(label, (image) => image.setAttribute("width", String(width))));
  control("Remove", (image) => {
    image.remove();
    select(null);
  });

  function remember() {
    const selection = window.getSelection();
    range = selection.rangeCount && editor.contains(selection.anchorNode) ? selection.getRangeAt(0).cloneRange() : null;
  }

  async function add(file) {
    if (!["image/png", "image/jpeg"].includes(file.type)) {
      showError("Only PNG and JPG images are supported.");
      return;
    }
    try {
      const bitmap = await createImageBitmap(file);
      const fitted = await fitImage(file, bitmap);
      const id = await invoke("save_image", { data: await blobToBase64(fitted) });
      const src = await invoke("get_image", { id });
      const image = document.createElement("img");
      image.setAttribute("data-zap-id", id);
      image.setAttribute("src", src);
      image.setAttribute("width", String(Math.min(bitmap.width, 240)));
      editor.focus();
      const selection = window.getSelection();
      selection.removeAllRanges();
      if (range) {
        selection.addRange(range);
      } else {
        selection.selectAllChildren(editor);
        selection.collapseToEnd();
      }
      document.execCommand("insertHTML", false, image.outerHTML);
    } catch (e) {
      showError(typeof e === "string" ? e : "Couldn't add that image. Try a smaller PNG or JPG.");
    }
  }

  button.addEventListener("mousedown", (e) => {
    e.preventDefault();
    remember();
    keepOpen(true);
    picker.click();
  });
  picker.addEventListener("change", () => {
    keepOpen(false);
    const file = picker.files[0];
    picker.value = "";
    if (file) add(file);
  });
  picker.addEventListener("cancel", () => keepOpen(false));

  editor.addEventListener("click", (e) => select(e.target.tagName === "IMG" ? e.target : null));
  editor.addEventListener("keydown", () => select(null));

  return {
    /** Shows the pictures of HTML just placed in the editor. */
    async hydrate() {
      select(null);
      for (const image of editor.querySelectorAll("img[data-zap-id]")) {
        try {
          image.setAttribute("src", await invoke("get_image", { id: image.getAttribute("data-zap-id") }));
        } catch {
          image.remove();
        }
      }
    },
    /** Takes a picture from the clipboard; true when there was one. */
    paste(event) {
      if (event.clipboardData.getData("text/plain")) return false;
      const file = [...event.clipboardData.files].find((f) => f.type.startsWith("image/"));
      if (!file) return false;
      remember();
      add(file);
      return true;
    },
    reset: () => select(null),
  };
}

const WEB_ADDRESS = /^https?:\/\/\S+$/i;

/** Pastes into a formatted editor: pictures, cleaned formatting, or a link for a lone web address. */
function pasteRich(event, images) {
  event.preventDefault();
  if (images.paste(event)) return;
  const html = event.clipboardData.getData("text/html");
  const plain = event.clipboardData.getData("text/plain");
  if (WEB_ADDRESS.test(plain.trim()) && window.getSelection().isCollapsed) {
    const link = document.createElement("a");
    link.href = plain.trim();
    link.textContent = plain.trim();
    document.execCommand("insertHTML", false, link.outerHTML);
    return;
  }
  document.execCommand("insertHTML", false, html ? sanitizeHtml(html) : plainToHtml(plain));
}

/** Turns a web address into a link once a space is typed after it. */
function wireAutoLink(editor, afterLink = () => {}) {
  editor.addEventListener("input", (e) => {
    if (e.inputType !== "insertText" || e.data !== " ") return;
    const selection = window.getSelection();
    const node = selection.anchorNode;
    if (!selection.isCollapsed || !node || node.nodeType !== Node.TEXT_NODE || node.parentElement.closest("a")) return;
    const before = node.textContent.slice(0, selection.anchorOffset);
    const match = before.match(/(?:^|[\s ])(https?:\/\/[^\s ]+)[\s ]$/i);
    if (!match) return;
    const url = match[1].replace(/[.,;:!?)]+$/, "");
    const end = before.length - 1 - (match[1].length - url.length);
    const range = document.createRange();
    range.setStart(node, end - url.length);
    range.setEnd(node, end);
    selection.removeAllRanges();
    selection.addRange(range);
    document.execCommand("createLink", false, url);
    selection.collapseToEnd();
    selection.modify("move", "forward", "character");
    selection.modify("move", "forward", "character");
    afterLink();
  });
}
