import Image from "next/image";
import Link from "next/link";

type Size = "sm" | "md" | "lg";

const sizes: Record<
  Size,
  { mark: string; word: string; gap: string; img: number }
> = {
  // Dashboard sidebar / mobile nav: compact lockup that fits a 56px row.
  sm: {
    mark: "h-7 w-7",
    word: "text-[20px]",
    gap: "gap-1.5",
    img: 32,
  },
  // Docs chrome: a touch larger than nav, sits above prose.
  md: {
    mark: "h-8 w-8 sm:h-9 sm:w-9",
    word: "text-[22px] sm:text-[24px]",
    gap: "gap-1.5 sm:gap-2",
    img: 36,
  },
  // Marketing hero: big editorial wordmark.
  lg: {
    mark: "h-9 w-9 sm:h-10 sm:w-10 md:h-11 md:w-11",
    word: "text-[26px] sm:text-[28px] md:text-[32px]",
    gap: "gap-1.5 sm:gap-2",
    img: 44,
  },
};

/**
 * Single source of truth for the Nyxbid wordmark lockup. Renders the
 * `/logo.png` mark + serif "Nyxbid" link → `/`. Used by the landing
 * hero, the docs chrome, and the dashboard sidebar / mobile nav so
 * the brand reads identically everywhere.
 */
export function Brand({
  size = "sm",
  className = "",
  href = "/",
}: {
  size?: Size;
  className?: string;
  href?: string;
}) {
  const s = sizes[size];
  return (
    <Link
      href={href}
      className={`flex items-center ${s.gap} tracking-tight text-foreground/95 hover:text-foreground ${className}`}
      style={{ fontFamily: "var(--font-serif)" }}
    >
      <Image
        src="/logo.png"
        alt=""
        width={s.img}
        height={s.img}
        priority={size === "lg"}
        className={`${s.mark} shrink-0 object-contain`}
      />
      <span className={s.word}>Nyxbid</span>
    </Link>
  );
}
