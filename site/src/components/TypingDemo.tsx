import { useEffect, useState } from "react";

interface Example {
  app: string;
  before: string;
  trigger: string;
  expansion: string;
  summary: string;
}

const today = new Date().toISOString().slice(0, 10);

const EXAMPLES: Example[] = [
  { app: "gmail", before: "Sure, reach me at ", trigger: "`pemail", expansion: "your.email@example.com", summary: "email" },
  { app: "notes", before: "Standup — ", trigger: "`date", expansion: today, summary: "{date}" },
  { app: "code.exe", before: "", trigger: "`iferr", expansion: "if err != nil {\n\treturn err\n}", summary: "3 lines" },
];

type Phase = "typing" | "expanded";

const prefersReducedMotion = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** A fake app window that types a trigger and swaps it for the snippet, on a loop. */
export function TypingDemo() {
  const [index, setIndex] = useState(0);
  const [typed, setTyped] = useState(0);
  const [phase, setPhase] = useState<Phase>("typing");
  const example = EXAMPLES[index];

  useEffect(() => {
    if (prefersReducedMotion()) {
      setPhase("expanded");
      return;
    }
    let timer: number;
    if (phase === "typing" && typed < example.trigger.length) {
      timer = window.setTimeout(() => setTyped((n) => n + 1), 110);
    } else if (phase === "typing") {
      timer = window.setTimeout(() => setPhase("expanded"), 350);
    } else {
      timer = window.setTimeout(() => {
        setIndex((i) => (i + 1) % EXAMPLES.length);
        setTyped(0);
        setPhase("typing");
      }, 2600);
    }
    return () => window.clearTimeout(timer);
  }, [phase, typed, example.trigger.length]);

  const expanded = phase === "expanded";
  const buffer = expanded ? "" : example.trigger.slice(0, typed);

  return (
    <div className="w-full rounded-card border border-line bg-surface/90 shadow-[0_40px_80px_-30px_rgba(0,0,0,1)] backdrop-blur-sm">
      <div className="flex items-center gap-2 border-b border-line px-4 py-3">
        <span className="live-dot size-1.5 rounded-full bg-accent" />
        <span className="label text-muted">inline engine</span>
        <span className="label ml-auto text-zinc-500">target: {example.app}</span>
      </div>

      <div className="m-3 rounded-card border border-line bg-surface-2">
        <pre
          className="min-h-36 whitespace-pre-wrap px-5 py-5 font-mono text-sm leading-relaxed text-zinc-200"
          aria-live="polite"
        >
          {example.before}
          {expanded ? (
            <span className="bg-accent/15 text-white shadow-[inset_0_-1px_0_0_#facc15]">{example.expansion}</span>
          ) : (
            <span className="text-white">{buffer}</span>
          )}
          <span className="caret ml-px inline-block h-[1.1em] w-[0.5em] translate-y-[0.15em] bg-accent" />
        </pre>
      </div>

      <dl className="grid grid-cols-3 border-t border-line">
        {[
          ["buffer", buffer || "—"],
          ["match", expanded ? example.trigger : "—"],
          ["status", expanded ? "zapped" : "listening"],
        ].map(([k, v], i) => (
          <div key={k} className={`px-4 py-3 ${i ? "border-l border-line" : ""}`}>
            <dt className="label text-[10px] text-zinc-500">{k}</dt>
            <dd className={`mt-1.5 truncate font-mono text-xs ${k === "status" && expanded ? "text-accent" : "text-zinc-300"}`}>
              {v}
            </dd>
          </div>
        ))}
      </dl>
      <p className="sr-only">
        Typing {example.trigger} becomes {example.summary}.
      </p>
    </div>
  );
}
