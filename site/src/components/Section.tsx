import type { ReactNode } from "react";
import { Reveal } from "./Reveal";

export function Section({
  id,
  index,
  eyebrow,
  title,
  children,
}: {
  id?: string;
  index: string;
  eyebrow: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <section id={id} className="scroll-mt-16 border-t border-line px-4 py-20 sm:px-6">
      <div className="mx-auto max-w-6xl">
        <Reveal>
          <p className="label text-muted">
            <span className="text-accent-text">{index}</span> / {eyebrow}
          </p>
          <h2 className="mt-4 max-w-2xl text-2xl font-semibold leading-[1.15] tracking-tight sm:text-[32px]">{title}</h2>
        </Reveal>
        <div className="mt-12">{children}</div>
      </div>
    </section>
  );
}
