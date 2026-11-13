/** Tell the badge checker something worth counting just happened (a game ended, a lesson was solved...). It looks a few seconds later. */
export const announceProgress = () => {
  if (typeof window !== "undefined") window.dispatchEvent(new Event("rookery:progress"));
};
