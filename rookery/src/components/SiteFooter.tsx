import { SITE } from "@/lib/site";

export function SiteFooter() {
  return (
    <footer className="mt-16 border-t border-line py-8 text-center text-sm text-dim">
      {SITE.name}: {SITE.tagline}
    </footer>
  );
}
