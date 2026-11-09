import { useState } from "react";
import { CONTACT, type Person } from "../config";
import { CoinAvatar } from "./CoinAvatar";
import { PixelWordmark } from "./PixelWordmark";
import { Reveal } from "./Reveal";
import { StaticNoise } from "./StaticNoise";

const RING = "CLICK · CLICK · CLICK · ";

// The two coins sit on a diagonal: first top-left, second bottom-right. Each person's details
// appear in the corner the *other* coin occupies, since that coin hides while one is active.
const COIN_SLOTS = ["left-0 top-0", "right-0 bottom-0"];
const DETAIL_SLOTS = ["right-2 bottom-12 text-right", "left-2 top-10 text-left"];

function Coin({
  person,
  index,
  state,
  onActivate,
  onDeactivate,
}: {
  person: Person;
  index: number;
  state: "idle" | "active" | "hidden";
  onActivate: () => void;
  onDeactivate: () => void;
}) {
  const avatar = person.avatar ? `${import.meta.env.BASE_URL}${person.avatar}` : undefined;
  const ringId = `ring-path-${index}`;
  const link = person.links[0];
  const active = state === "active";

  return (
    <a
      href={link?.url}
      aria-label={`${person.name} on ${link?.label}`}
      onMouseEnter={onActivate}
      onMouseLeave={onDeactivate}
      onFocus={onActivate}
      onBlur={onDeactivate}
      className={`absolute size-36 outline-none transition-[opacity,scale] duration-500 ease-out-soft sm:size-40 ${COIN_SLOTS[index]} ${
        state === "hidden" ? "pointer-events-none scale-90 opacity-0" : "opacity-100"
      }`}
    >
      {/* The ring only appears around the active coin. */}
      <svg
        viewBox="0 0 100 100"
        aria-hidden
        className={`ring-spin absolute inset-0 size-full transition-opacity duration-500 ${active ? "opacity-100" : "opacity-0"}`}
      >
        <defs>
          <path id={ringId} d="M50,50 m-44,0 a44,44 0 1,1 88,0 a44,44 0 1,1 -88,0" />
        </defs>
        <text className="fill-zinc-300 font-mono text-[7.4px] font-semibold tracking-[0.2em]">
          <textPath href={`#${ringId}`}>{RING}</textPath>
        </text>
      </svg>
      <CoinAvatar
        src={avatar}
        name={person.name}
        active={active}
        className={`absolute inset-0 transition-[scale] duration-500 ease-out-soft ${active ? "scale-[0.66]" : "scale-100"}`}
      />
    </a>
  );
}

/**
 * "ZAP by <makers>": pixel wordmark, then two profile coins. Hovering one hides the other and shows its details.
 * Drawn like a dark screen in both themes, so its colours are fixed rather than design tokens.
 */
export function ContactBanner() {
  const [active, setActive] = useState<number | null>(null);
  const people = CONTACT.people.slice(0, 2);

  return (
    <Reveal className="relative overflow-hidden rounded-card border border-line bg-[#0b0b0c] px-6 py-14 sm:px-12">
      {/* TV static, softened at the edges so the content in the middle stays readable */}
      <StaticNoise className="absolute inset-0 size-full opacity-90" />
      <div aria-hidden className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,transparent_30%,rgba(0,0,0,0.55)_100%)]" />
      {/* Dashed baseline through the wordmark and the lower coin */}
      <div aria-hidden className="absolute inset-x-0 top-1/2 border-t border-dashed border-white/15" />

      <div className="relative flex flex-col items-center gap-10 md:flex-row md:justify-center md:gap-10">
        <PixelWordmark text="ZAP" className="h-20 w-auto sm:h-28 lg:h-32" />

        <span className="font-mono text-2xl text-zinc-200 sm:text-3xl [text-shadow:0_0_18px_#000,0_0_6px_#000]">by</span>

        <div className="relative h-[17rem] w-[18rem] shrink-0 sm:h-[18.5rem] sm:w-[19.5rem]">
          {people.map((person, i) => (
            <div
              key={person.name}
              aria-hidden
              className={`pointer-events-none absolute font-mono transition-opacity duration-300 ${DETAIL_SLOTS[i]} ${
                active === i ? "opacity-100 delay-150" : "opacity-0"
              }`}
            >
              <div className="text-sm font-semibold text-white">{person.name}</div>
              <div className="mt-2 text-xs text-zinc-500">{person.links[0]?.label} ↗</div>
            </div>
          ))}
          {people.map((person, i) => (
            <Coin
              key={person.name}
              person={person}
              index={i}
              state={active === null ? "idle" : active === i ? "active" : "hidden"}
              onActivate={() => setActive(i)}
              onDeactivate={() => setActive((a) => (a === i ? null : a))}
            />
          ))}
        </div>
      </div>
    </Reveal>
  );
}
