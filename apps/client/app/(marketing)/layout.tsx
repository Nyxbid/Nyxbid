import Link from "next/link";

const navLinks = [
  { href: "/docs", label: "Docs" },
  { href: "/dashboard", label: "App" },
  { href: "https://github.com/Nyxbid/Nyxbid", label: "GitHub" },
];

export default function MarketingLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="flex min-h-screen flex-col">
      <header className="border-b border-border">
        <div className="mx-auto flex h-14 max-w-5xl items-center justify-between px-6">
          <Link href="/" className="text-lg font-semibold tracking-tight">
            nyxbid
          </Link>
          <nav className="flex items-center gap-6">
            {navLinks.map(({ href, label }) => (
              <Link
                key={href}
                href={href}
                className="text-sm text-muted hover:text-foreground"
                {...(href.startsWith("http")
                  ? { target: "_blank", rel: "noopener noreferrer" }
                  : {})}
              >
                {label}
              </Link>
            ))}
          </nav>
        </div>
      </header>
      <main className="flex-1">{children}</main>
      <footer className="border-t border-border py-8">
        <div className="mx-auto max-w-5xl px-6">
          <div className="flex flex-col items-center justify-between gap-4 sm:flex-row">
            <p className="text-xs text-muted">Built on Solana. Open source.</p>
            <div className="flex items-center gap-6">
              <Link
                href="/docs"
                className="text-xs text-muted hover:text-foreground"
              >
                Docs
              </Link>
              <Link
                href="https://github.com/Nyxbid/Nyxbid"
                className="text-xs text-muted hover:text-foreground"
                target="_blank"
                rel="noopener noreferrer"
              >
                GitHub
              </Link>
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
}
