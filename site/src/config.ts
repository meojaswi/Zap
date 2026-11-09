import { HARSH, OJASWI } from "./portraits";

export const REPO = "meojaswi/Zap";
export const REPO_URL = `https://github.com/${REPO}`;
export const RELEASES_URL = `${REPO_URL}/releases/latest`;

// Shown in the Contact section. Replace these with your own details.
export interface Person {
  name: string;
  /** ASCII art from site/scripts/ascii_portraits.py. */
  portrait: string[];
  handle?: string;
  /** Profile links; the first one is also where the portrait points. */
  links: { label: string; url: string }[];
}

export const CONTACT: { email: string; people: Person[] } = {
  email: "you@example.com",
  people: [
    {
      name: "Ojaswi",
      portrait: OJASWI,
      links: [{ label: "LinkedIn", url: "https://www.linkedin.com/in/kumar-ojaswi/" }],
    },
    {
      name: "Harsh",
      portrait: HARSH,
      links: [{ label: "LinkedIn", url: "https://www.linkedin.com/in/harsh-pandey-cakewalk/" }],
    },
  ],
};
