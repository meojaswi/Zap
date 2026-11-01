import { useId } from "react";

/**
 * Cut-out portrait (transparent PNG) on a dark studio backdrop: high-contrast grayscale, a soft rim
 * light that follows the silhouette, and film grain. The rim light turns Zap yellow when `active`.
 * Falls back to initials without a photo.
 */
export function CoinAvatar({
  src,
  name,
  active = false,
  className = "",
}: {
  src?: string;
  name: string;
  active?: boolean;
  className?: string;
}) {
  // useId output contains characters like ":" that break url(#...) references.
  const grainId = `grain-${useId().replace(/[^a-zA-Z0-9-]/g, "")}`;
  const initials = name
    .split(/\s+/)
    .map((w) => w[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();

  return (
    <div
      className={`overflow-hidden rounded-full bg-[radial-gradient(circle_at_35%_25%,#2e2e33,#050506_75%)] shadow-[inset_0_0_0_1px_rgba(255,255,255,0.08),0_14px_40px_-12px_rgba(0,0,0,0.9)] ${className}`}
    >
      {src ? (
        <>
          <img
            src={src}
            alt={name}
            draggable={false}
            className={`size-full object-cover object-top transition-[filter] duration-500 ${
              active
                ? "[filter:grayscale(1)_contrast(1.3)_brightness(0.95)_drop-shadow(0_0_10px_rgba(250,204,21,0.55))]"
                : "[filter:grayscale(1)_contrast(1.3)_brightness(0.92)_drop-shadow(0_0_10px_rgba(228,228,231,0.45))]"
            }`}
          />
          {/* Film grain */}
          <svg aria-hidden className="pointer-events-none absolute inset-0 size-full opacity-40 mix-blend-overlay">
            <filter id={grainId}>
              <feTurbulence type="fractalNoise" baseFrequency="1.1" numOctaves="2" stitchTiles="stitch" />
              <feColorMatrix type="saturate" values="0" />
            </filter>
            <rect width="100%" height="100%" filter={`url(#${grainId})`} />
          </svg>
          {/* Edge vignette */}
          <div className="pointer-events-none absolute inset-0 rounded-full shadow-[inset_0_0_30px_rgba(0,0,0,0.6)]" />
        </>
      ) : (
        <div className="grid size-full place-items-center font-mono text-2xl font-semibold text-zinc-300">{initials}</div>
      )}
    </div>
  );
}
