export const REPO = "meojaswi/Zap";
export const REPO_URL = `https://github.com/${REPO}`;
export const RELEASES_URL = `${REPO_URL}/releases/latest`;

// Shown in the Contact section. Replace these with your own details.
export interface Person {
  name: string;
  /** Photo in site/public/, e.g. "ojaswi.jpg". Leave empty to show initials instead. */
  avatar: string;
  handle?: string;
  /** Profile links; the first one is also where the avatar coin points. */
  links: { label: string; url: string }[];
}

export const CONTACT: { email: string; people: Person[] } = {
  email: "you@example.com",
  people: [
    {
      name: "Ojaswi",
      avatar: "ojaswi.png",
      links: [{ label: "LinkedIn", url: "https://www.linkedin.com/in/kumar-ojaswi/" }],
    },
    {
      name: "Harsh",
      avatar: "harsh.png",
      links: [{ label: "LinkedIn", url: "https://www.linkedin.com/in/harsh-pandey-cakewalk/" }],
    },
  ],
};
