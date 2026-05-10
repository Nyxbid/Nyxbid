import Link from "next/link";
import { notFound } from "next/navigation";

import { fetchJson, ApiError } from "@/lib/api";
import type { Intent, Quote } from "@/lib/data";
import { IntentDetail } from "@/components/intent-detail";

export const dynamic = "force-dynamic";

interface Props {
  params: Promise<{ id: string }>;
}

export default async function IntentDetailPage({ params }: Props) {
  const { id } = await params;

  const [intent, quotes] = await Promise.all([
    fetchJson<Intent>(`/api/intents/${id}`).catch((e) => {
      if (e instanceof ApiError && e.status === 404) return null;
      throw e;
    }),
    fetchJson<Quote[]>(`/api/intents/${id}/quotes`).catch(
      () => [] as Quote[],
    ),
  ]);

  if (!intent) notFound();

  return (
    <>
      <Link
        href="/intents"
        className="inline-flex items-center gap-1.5 text-xs text-muted hover:text-foreground"
      >
        <svg
          width="14"
          height="14"
          viewBox="0 0 14 14"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
        >
          <path d="M8 3L4 7l4 4" />
        </svg>
        All intents
      </Link>

      <div className="mt-6">
        <IntentDetail initialIntent={intent} initialQuotes={quotes} />
      </div>
    </>
  );
}
