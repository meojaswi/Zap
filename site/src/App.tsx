import { ContactBanner } from "./components/ContactBanner";
import { DitherField } from "./components/DitherField";
import { DownloadButton } from "./components/DownloadButton";
import { Panel } from "./components/Panel";
import { PopupMock } from "./components/PopupMock";
import { Reveal } from "./components/Reveal";
import { Section } from "./components/Section";
import { TypingDemo } from "./components/TypingDemo";
import { CONTACT, REPO_URL } from "./config";
import { useLatestRelease } from "./useLatestRelease";

const METRICS = [
  ["0", "accounts or ads"],
  ["DPAPI", "encrypted at rest"],
  ["4.5 MB", "installer size"],
  ["6", "live placeholders"],
];

const TRIGGERS = [
  ["`email", "you@example.com"],
  ["`addr", "221B Example Street, London"],
  ["`thx", "Thanks for reaching out! I'll get back to you…"],
];

const PLACEHOLDERS = [
  ["{date}", "2026-11-01"],
  ["{time}", "14:05"],
  ["{datetime}", "2026-11-01 14:05"],
  ["{day}", "Sunday"],
  ["{clipboard}", "what you copied"],
  ["{uuid}", "fresh random id"],
  ["{date:DD/MM/YYYY}", "07/11/2026"],
  ["{time:hh:mm A}", "05:05 PM"],
];

const CONTROLS = [
  ["pause", "Double-click the tray icon to stop all expansion.", "toggle"],
  ["excluded apps", "Inline mode stays off in games or apps you list.", "list"],
  ["start with windows", "Launches quietly into the tray at sign-in.", "toggle"],
  ["import / export", "Back up or move snippets as JSON.", "file"],
];

const SECURITY = [
  ["storage", "Snippets encrypted with Windows DPAPI, readable only by your account"],
  ["clipboard", "Pastes kept out of Win+V history and cloud sync; old clipboard restored"],
  ["sensitive", "Sensitive snippets are typed key by key, never via clipboard"],
  ["keystrokes", "Never logged or saved; only the last few chars live in memory"],
  ["stats", "Snippet use counts stay on this PC, encrypted; turn them off anytime"],
  ["network", "No account; connects only for signed updates and an anonymous daily user count, both can be turned off"],
];

const STEPS = [
  ["install", "Run the installer. No admin rights needed — it installs just for you."],
  ["define", "Add a snippet: a trigger like `addr and the text it becomes."],
  ["zap", "Type the trigger in any app. Zap swaps it for your text, instantly."],
];

export default function App() {
  const release = useLatestRelease();

  return (
    <>
      <header className="sticky top-0 z-20 border-b border-line bg-bg">
        <nav className="mx-auto flex h-14 max-w-6xl items-center gap-6 px-4 sm:px-6">
          <a href="#top" className="flex items-center gap-2 font-semibold tracking-tight">
            <img src={`${import.meta.env.BASE_URL}favicon.svg`} alt="" className="size-6" />
            Zap
          </a>
          <div className="ml-auto flex items-center gap-6">
            <a href="#features" className="label hidden text-muted transition-colors hover:text-fg sm:inline">
              features
            </a>
            <a href="#privacy" className="label hidden text-muted transition-colors hover:text-fg sm:inline">
              privacy
            </a>
            <a href="#contact" className="label text-muted transition-colors hover:text-fg">
              contact
            </a>
            <a href={REPO_URL} className="label text-muted transition-colors hover:text-fg">
              github ↗
            </a>
          </div>
        </nav>
      </header>

      <main id="top">
        {/* Hero */}
        <section className="relative overflow-hidden px-4 pb-20 pt-10 sm:px-6">
          <DitherField className="absolute inset-0 size-full opacity-70" />
          <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_20%_40%,var(--color-bg),transparent_60%)]" />

          <div className="relative mx-auto max-w-6xl">
            <Reveal className="flex flex-wrap items-center gap-x-6 gap-y-2 border-b border-line pb-4">
              <span className="label text-muted">zap / text expander v{release.version ?? "0.1"}</span>
              <span className="label flex items-center gap-2 text-fg">
                <span className="live-dot size-1.5 rounded-full bg-accent" />
                [active]
              </span>
              <span className="label ml-auto hidden text-muted sm:inline">platform: windows 10 / 11 · x64</span>
            </Reveal>

            <div className="grid items-center gap-14 pt-16 lg:grid-cols-[1fr_1.05fr] lg:pt-24">
              <Reveal>
                <p className="label text-accent-text">type, zap, done</p>
                <h1 className="mt-6 text-4xl font-semibold leading-[1.08] tracking-tight sm:text-[56px]">
                  <span className="mask">
                    <span>Type less.</span>
                  </span>
                  <span className="mask">
                    <span style={{ "--delay": "120ms" } as React.CSSProperties} className="text-muted">
                      Zap the rest.
                    </span>
                  </span>
                </h1>
                <p className="mt-7 max-w-md leading-[1.6] text-muted">
                  A private text expander for Windows. Type a short trigger in any app and it becomes your saved
                  email, address, reply or code — encrypted, local, instant.
                </p>
                <div className="mt-10 flex flex-wrap items-center gap-5">
                  <DownloadButton release={release} />
                  <a
                    href={REPO_URL}
                    className="label rounded-card border border-line px-4 py-3 text-muted transition-colors hover:border-line-strong hover:text-fg"
                  >
                    view source
                  </a>
                </div>
              </Reveal>

              <Reveal delay={200}>
                <TypingDemo />
              </Reveal>
            </div>

            <div className="mt-20 grid grid-cols-2 border border-line bg-bg lg:grid-cols-4">
              {METRICS.map(([value, label], i) => (
                <Reveal
                  key={label}
                  delay={i * 90}
                  className={`px-5 py-6 ${i % 2 ? "border-l border-line" : ""} ${i > 1 ? "border-t border-line lg:border-t-0" : ""} ${i === 2 ? "lg:border-l" : ""}`}
                >
                  <div className="text-3xl font-semibold tracking-tight">{value}</div>
                  <div className="label mt-2 text-muted">{label}</div>
                </Reveal>
              ))}
            </div>
          </div>
        </section>

        {/* Features */}
        <Section id="features" index="01" eyebrow="system modules" title="Everything you type twice, typed once.">
          <div className="grid grid-cols-1 gap-4 lg:grid-cols-12">
            <Reveal className="lg:col-span-7">
              <Panel label="module: inline expansion" meta="hook · local" live>
                <div className="p-6">
                  <p className="max-w-md leading-[1.6] text-muted">
                    Type a trigger in your browser, Slack, Word or editor and Zap replaces it with the full text. No
                    window, no shortcut.
                  </p>
                  <div className="mt-6 rounded-card border border-line bg-subtle">
                    {TRIGGERS.map(([t, out]) => (
                      <div key={t} className="flex items-center gap-4 border-b border-line/60 px-4 py-3 last:border-0">
                        <code className="w-20 shrink-0 font-mono text-xs text-accent-text">{t}</code>
                        <span className="text-faint">→</span>
                        <span className="truncate font-mono text-xs text-fg">{out}</span>
                      </div>
                    ))}
                  </div>
                </div>
              </Panel>
            </Reveal>

            <Reveal delay={100} className="lg:col-span-5">
              <Panel label="module: search" meta="ctrl+shift+space">
                <div className="p-6">
                  <p className="leading-[1.6] text-muted">Can't remember a trigger? Search by name or text and hit Enter.</p>
                  <div className="mt-6">
                    <PopupMock />
                  </div>
                </div>
              </Panel>
            </Reveal>

            <Reveal className="lg:col-span-5">
              <Panel label="module: placeholders" meta="resolved at insert">
                <div className="grid grid-cols-2 gap-px bg-line">
                  {PLACEHOLDERS.map(([ph, out]) => (
                    <div key={ph} className="bg-surface px-4 py-4">
                      <code className="font-mono text-xs text-fg">{ph}</code>
                      <div className="mt-1.5 truncate text-xs text-muted">{out}</div>
                    </div>
                  ))}
                </div>
              </Panel>
            </Reveal>

            <Reveal delay={100} className="lg:col-span-7">
              <Panel label="module: control" meta="tray">
                <ul>
                  {CONTROLS.map(([name, body, kind]) => (
                    <li key={name} className="flex items-center gap-4 border-b border-line/60 px-6 py-4 last:border-0">
                      <div className="min-w-0 flex-1">
                        <div className="label text-fg">{name}</div>
                        <div className="mt-1.5 text-sm text-muted">{body}</div>
                      </div>
                      {kind === "toggle" ? (
                        <span className="relative h-5 w-9 shrink-0 rounded-card bg-invert-bg">
                          <span className="absolute right-0.5 top-0.5 size-4 rounded-[1px] bg-accent" />
                        </span>
                      ) : (
                        <span className="label shrink-0 rounded-card border border-line px-3 py-1.5 text-[10px] text-muted">
                          {kind}
                        </span>
                      )}
                    </li>
                  ))}
                </ul>
              </Panel>
            </Reveal>
          </div>
        </Section>

        {/* Privacy */}
        <Section id="privacy" index="02" eyebrow="security telemetry" title="It sees your keystrokes. So it was built to forget them.">
          <div className="grid grid-cols-1 gap-4 lg:grid-cols-12">
            <Reveal className="lg:col-span-4">
              <p className="leading-[1.6] text-muted">
                Inline expansion needs a keyboard hook, so privacy isn't a feature here — it's the architecture.
                Everything stays on your machine.
              </p>
              <p className="mt-6 text-sm leading-[1.6] text-muted">
                Still, don't store passwords in Zap. A password manager is the right place for those.
              </p>
            </Reveal>
            <Reveal delay={120} className="lg:col-span-8">
              <Panel label="privacy audit" meta={`${SECURITY.length}/${SECURITY.length} checks`} live>
                <ul className="font-mono text-sm">
                  {SECURITY.map(([key, text]) => (
                    <li key={key} className="flex gap-4 border-b border-line/60 px-5 py-4 last:border-0">
                      <span className="label shrink-0 pt-0.5 text-accent-text">[ok]</span>
                      <span className="label w-24 shrink-0 pt-0.5 text-muted">{key}</span>
                      <span className="text-xs leading-relaxed text-fg sm:text-sm">{text}</span>
                    </li>
                  ))}
                </ul>
              </Panel>
            </Reveal>
          </div>
        </Section>

        {/* Steps */}
        <Section index="03" eyebrow="boot sequence" title="Up and running in a minute.">
          <ol className="grid gap-4 md:grid-cols-3">
            {STEPS.map(([title, body], i) => (
              <Reveal as="li" key={title} delay={i * 110}>
                <Panel label={`step 0${i + 1}`} meta={title}>
                  <div className="p-6">
                    <div className="text-2xl font-semibold capitalize tracking-tight">{title}</div>
                    <p className="mt-3 text-sm leading-[1.6] text-muted">{body}</p>
                  </div>
                </Panel>
              </Reveal>
            ))}
          </ol>
        </Section>

        {/* CTA */}
        <section className="relative overflow-hidden border-t border-line px-4 py-28 sm:px-6">
          <DitherField className="absolute inset-0 size-full opacity-40" />
          <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,var(--color-bg),transparent_70%)]" />
          <Reveal className="relative mx-auto max-w-2xl text-center">
            <p className="label text-accent-text">ready</p>
            <h2 className="mt-5 text-3xl font-semibold leading-[1.08] tracking-tight sm:text-5xl">Start zapping.</h2>
            <p className="mx-auto mt-6 max-w-md text-sm leading-[1.6] text-muted">
              Free and open source. The installer isn't code-signed yet, so if Edge says it isn't commonly downloaded,
              choose <span className="text-fg">… → Keep → Show more → Keep anyway</span>. If Windows shows a warning,
              click <span className="text-fg">More info → Run anyway</span>.
            </p>
            <div className="mt-10 flex justify-center">
              <DownloadButton release={release} />
            </div>
          </Reveal>
        </section>

        {/* Contact */}
        <Section id="contact" index="04" eyebrow="contact" title="Questions, ideas or bugs? Get in touch.">
          <ContactBanner />
          <div className="mt-4 grid gap-4 md:grid-cols-2">
            <Reveal delay={100}>
              <a href={`mailto:${CONTACT.email}`} className="block h-full">
                <Panel label="email" meta="reply in a few days">
                  <div className="p-6">
                    <div className="truncate font-mono text-sm text-accent-text">{CONTACT.email}</div>
                    <p className="mt-3 text-sm leading-[1.6] text-muted">For questions, feedback or anything else.</p>
                  </div>
                </Panel>
              </a>
            </Reveal>
            <Reveal delay={200}>
              <a href={`${REPO_URL}/issues`} className="block h-full">
                <Panel label="github issues" meta="public">
                  <div className="p-6">
                    <div className="font-mono text-sm text-accent-text">open an issue ↗</div>
                    <p className="mt-3 text-sm leading-[1.6] text-muted">Bug reports and feature requests.</p>
                  </div>
                </Panel>
              </a>
            </Reveal>
          </div>
        </Section>
      </main>

      <footer className="border-t border-line px-4 py-8 sm:px-6">
        <div className="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-4">
          <span className="label text-muted">© 2026 Kumar Ojaswi</span>
          <span className="flex gap-6">
            {[
              ["github", REPO_URL],
              ["report a bug", `${REPO_URL}/issues`],
              ["releases", `${REPO_URL}/releases`],
            ].map(([label, href]) => (
              <a key={label} href={href} className="label text-muted transition-colors hover:text-fg">
                {label}
              </a>
            ))}
          </span>
        </div>
      </footer>
    </>
  );
}
