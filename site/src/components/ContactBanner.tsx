import { CONTACT } from "../config";
import { AsciiPortrait } from "./AsciiPortrait";
import { PixelWordmark } from "./PixelWordmark";
import { Reveal } from "./Reveal";
import { StaticNoise } from "./StaticNoise";

/**
 * "ZAP by <makers>": pixel wordmark above two large ASCII portraits that link to each maker.
 * Drawn like a dark screen in both themes, so its colours are fixed rather than design tokens.
 */
export function ContactBanner() {
  return (
    <Reveal className="relative overflow-hidden rounded-card border border-line bg-[#0b0b0c] px-4 py-12 sm:px-12">
      {/* TV static, softened at the edges so the content in the middle stays readable */}
      <StaticNoise className="absolute inset-0 size-full opacity-90" />
      <div aria-hidden className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,transparent_30%,rgba(0,0,0,0.55)_100%)]" />

      <div className="relative mx-auto flex max-w-4xl flex-col items-center gap-10">
        <div className="flex items-end gap-5">
          <PixelWordmark text="ZAP" className="h-14 w-auto sm:h-20" />
          <span className="font-mono text-2xl text-zinc-200 sm:text-3xl">by</span>
        </div>

        <div className="grid w-full grid-cols-1 gap-6 md:grid-cols-2">
          {CONTACT.people.slice(0, 2).map((person) => {
            const link = person.links[0];
            return (
              <a
                key={person.name}
                href={link?.url}
                aria-label={`${person.name} on ${link?.label}`}
                className="group block rounded-card border border-white/15 bg-[#050506] outline-none transition-colors duration-300 hover:border-accent focus-visible:border-accent"
              >
                <div className="p-3">
                  <AsciiPortrait art={person.portrait} name={person.name} />
                </div>
                <div className="flex items-center justify-between border-t border-white/15 px-4 py-3">
                  <span className="label text-white">{person.name}</span>
                  <span className="label text-zinc-400">{link?.label} ↗</span>
                </div>
              </a>
            );
          })}
        </div>
      </div>
    </Reveal>
  );
}
