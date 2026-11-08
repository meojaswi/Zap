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
